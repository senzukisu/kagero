//! Safe installation of a verified update package.
//!
//! Kagero never overwrites its own running executable. Instead it:
//! 1. extracts the verified package into a staging folder (flat, allow-listed names);
//! 2. copies the package's `kagero-updater` helper next to the staging folder;
//! 3. starts the helper with local paths only, then exits;
//! 4. the helper waits until `kagero.exe` can be moved, backs up every file it replaces,
//!    copies the new files in, and rolls everything back if any step fails;
//! 5. the helper starts Kagero again (new version, or the old one after a rollback).
//!
//! Nothing from the network is ever passed to a shell. See `docs/developers/services.md`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Main executable name inside packages and install folders.
pub const APP_EXE: &str = if cfg!(windows) {
    "kagero.exe"
} else {
    "kagero"
};
/// Helper executable name.
pub const UPDATER_EXE: &str = if cfg!(windows) {
    "kagero-updater.exe"
} else {
    "kagero-updater"
};
/// Folder (inside the install folder) that keeps the previous version.
pub const BACKUP_DIR: &str = ".kagero-backup";
const BACKUP_MANIFEST: &str = "backup.json";
const MAX_ENTRIES: usize = 64;
const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
/// How long the helper waits for Kagero to exit.
pub const WAIT_FOR_EXIT: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the update package is damaged: {0}")]
    BadPackage(String),
    #[error("Kagero's folder is not writable ({0}). Download the update manually from the releases page")]
    NotWritable(String),
    #[error("Kagero did not close in time, so the update was not installed")]
    StillRunning,
    #[error("installing is only supported for official Windows builds")]
    Unsupported,
    #[error("the update failed and the previous version was restored: {0}")]
    RolledBack(String),
}

/// A file name allowed in an update package: flat, simple characters only.
fn allowed_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Extracts a package zip into `dest` (created fresh). Returns the extracted names.
///
/// Rejects directories, nested paths, odd names, oversized entries and packages without
/// both executables.
pub fn extract_package(zip_path: &Path, dest: &Path) -> Result<Vec<String>, InstallError> {
    let file = fs::File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| InstallError::BadPackage(e.to_string()))?;
    if zip.len() > MAX_ENTRIES {
        return Err(InstallError::BadPackage("too many files".into()));
    }
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    let mut names = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| InstallError::BadPackage(e.to_string()))?;
        let name = entry.name().to_owned();
        if entry.is_dir() || !allowed_name(&name) {
            return Err(InstallError::BadPackage(format!(
                "unexpected entry \"{name}\""
            )));
        }
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(InstallError::BadPackage(format!("\"{name}\" is too large")));
        }
        let mut out = fs::File::create(dest.join(&name))?;
        let copied = std::io::copy(&mut (&mut entry).take(MAX_ENTRY_BYTES + 1), &mut out)?;
        if copied > MAX_ENTRY_BYTES {
            return Err(InstallError::BadPackage(format!("\"{name}\" is too large")));
        }
        names.push(name);
    }
    for required in [APP_EXE, UPDATER_EXE] {
        if !names.iter().any(|n| n == required) {
            return Err(InstallError::BadPackage(format!(
                "\"{required}\" is missing"
            )));
        }
    }
    Ok(names)
}

/// Checks that files can be created in `dir` without administrator rights.
pub fn check_writable(dir: &Path) -> Result<(), InstallError> {
    let probe = dir.join(format!(".kagero-write-test-{}", std::process::id()));
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => Err(InstallError::NotWritable(e.to_string())),
    }
}

/// What a backup holds, so it can be restored exactly.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    /// Files that existed before and were moved into the backup folder.
    pub replaced: Vec<String>,
    /// Files the update added (removed again on restore).
    pub added: Vec<String>,
}

/// Moves `from` to `to`, retrying while the file is locked (e.g. Kagero still exiting).
fn move_with_retry(from: &Path, to: &Path, timeout: Duration) -> Result<(), InstallError> {
    let start = Instant::now();
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if start.elapsed() < timeout => {
                tracing::debug!("waiting to move {}: {e}", from.display());
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(_) => return Err(InstallError::StillRunning),
        }
    }
}

/// Replaces `names` in `target` with the files from `source`, keeping the previous
/// files in `backup`. On any failure everything is restored and `RolledBack` returned.
pub fn install_files(
    source: &Path,
    target: &Path,
    backup: &Path,
    names: &[String],
    wait: Duration,
) -> Result<(), InstallError> {
    if backup.exists() {
        fs::remove_dir_all(backup)?;
    }
    fs::create_dir_all(backup)?;
    let mut manifest = BackupManifest::default();

    // The main executable first: it stays locked until Kagero has exited.
    let mut ordered: Vec<&String> = names.iter().filter(|n| *n == APP_EXE).collect();
    ordered.extend(names.iter().filter(|n| *n != APP_EXE));

    let result = (|| -> Result<(), InstallError> {
        for name in &ordered {
            if !allowed_name(name) {
                return Err(InstallError::BadPackage(format!(
                    "unexpected name \"{name}\""
                )));
            }
            let current = target.join(name);
            if current.exists() {
                let timeout = if name.as_str() == APP_EXE {
                    wait
                } else {
                    Duration::from_secs(5)
                };
                move_with_retry(&current, &backup.join(name), timeout)?;
                manifest.replaced.push((*name).clone());
            } else {
                manifest.added.push((*name).clone());
            }
            // Save progress so an interrupted install can still be restored.
            write_manifest(backup, &manifest)?;
        }
        for name in &ordered {
            fs::copy(source.join(name), target.join(name))?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => Ok(()),
        Err(InstallError::StillRunning) if manifest.replaced.is_empty() => {
            Err(InstallError::StillRunning)
        }
        Err(e) => {
            let reason = e.to_string();
            restore_backup(target, backup).map_err(|restore| {
                InstallError::RolledBack(format!("{reason}; restoring also failed: {restore}"))
            })?;
            Err(InstallError::RolledBack(reason))
        }
    }
}

fn write_manifest(backup: &Path, manifest: &BackupManifest) -> Result<(), InstallError> {
    let json =
        serde_json::to_vec_pretty(manifest).map_err(|e| InstallError::BadPackage(e.to_string()))?;
    fs::write(backup.join(BACKUP_MANIFEST), json)?;
    Ok(())
}

/// Whether a previous version is available to restore.
pub fn has_backup(target: &Path) -> bool {
    target.join(BACKUP_DIR).join(BACKUP_MANIFEST).is_file()
}

/// Puts the backed-up files back and removes files the update added.
pub fn restore_backup(target: &Path, backup: &Path) -> Result<(), InstallError> {
    let bytes = fs::read(backup.join(BACKUP_MANIFEST))?;
    let manifest: BackupManifest =
        serde_json::from_slice(&bytes).map_err(|e| InstallError::BadPackage(e.to_string()))?;
    for name in manifest.added.iter().chain(&manifest.replaced) {
        if !allowed_name(name) {
            return Err(InstallError::BadPackage(format!(
                "unexpected name \"{name}\""
            )));
        }
    }
    for name in &manifest.added {
        let path = target.join(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    for name in &manifest.replaced {
        let saved = backup.join(name);
        if saved.exists() {
            let dest = target.join(name);
            if dest.exists() {
                fs::remove_file(&dest)?;
            }
            fs::rename(saved, dest)?;
        }
    }
    fs::remove_file(backup.join(BACKUP_MANIFEST))?;
    Ok(())
}

/// Arguments the app passes to the helper (all local paths).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperArgs {
    pub source: PathBuf,
    pub target: PathBuf,
    /// Restore the previous version from the backup instead of installing `source`.
    pub restore: bool,
}

impl HelperArgs {
    pub fn to_args(&self) -> Vec<std::ffi::OsString> {
        let mut args = vec!["--target".into(), self.target.clone().into_os_string()];
        if self.restore {
            args.push("--restore".into());
        } else {
            args.push("--source".into());
            args.push(self.source.clone().into_os_string());
        }
        args
    }

    pub fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Self> {
        let mut source = None;
        let mut target = None;
        let mut restore = false;
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.to_str() {
                Some("--source") => source = it.next().map(PathBuf::from),
                Some("--target") => target = it.next().map(PathBuf::from),
                Some("--restore") => restore = true,
                _ => return None,
            }
        }
        let target = target?;
        if restore {
            Some(Self {
                source: PathBuf::new(),
                target,
                restore,
            })
        } else {
            Some(Self {
                source: source?,
                target,
                restore,
            })
        }
    }
}

/// The folder Kagero runs from.
pub fn install_dir() -> Result<PathBuf, InstallError> {
    let exe = std::env::current_exe()?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| InstallError::NotWritable("unknown install folder".into()))
}

/// Prepares a verified package and starts the helper. On `Ok` the app must exit
/// promptly so the helper can replace the files.
pub fn launch_install(package: &Path, version: &str) -> Result<(), InstallError> {
    if !cfg!(windows) || !crate::release::is_official_build() {
        return Err(InstallError::Unsupported);
    }
    let target = install_dir()?;
    check_writable(&target)?;
    let work = std::env::temp_dir()
        .join("kagero-update")
        .join(format!("install-{version}"));
    let staging = work.join("files");
    extract_package(package, &staging)?;
    // Run the helper from outside the install folder so it can replace itself.
    let helper = work.join(UPDATER_EXE);
    fs::copy(staging.join(UPDATER_EXE), &helper)?;
    let args = HelperArgs {
        source: staging,
        target,
        restore: false,
    };
    std::process::Command::new(&helper)
        .args(args.to_args())
        .spawn()?;
    Ok(())
}

/// Starts the helper in restore mode (previous version). The app must then exit.
pub fn launch_restore() -> Result<(), InstallError> {
    if !cfg!(windows) || !crate::release::is_official_build() {
        return Err(InstallError::Unsupported);
    }
    let target = install_dir()?;
    if !has_backup(&target) {
        return Err(InstallError::BadPackage(
            "there is no previous version to restore".into(),
        ));
    }
    check_writable(&target)?;
    let work = std::env::temp_dir().join("kagero-update").join("restore");
    fs::create_dir_all(&work)?;
    let helper = work.join(UPDATER_EXE);
    fs::copy(target.join(UPDATER_EXE), &helper)?;
    let args = HelperArgs {
        source: PathBuf::new(),
        target,
        restore: true,
    };
    std::process::Command::new(&helper)
        .args(args.to_args())
        .spawn()?;
    Ok(())
}

/// The helper's whole job: install (or restore), then start Kagero again.
pub fn run_helper(args: &HelperArgs) -> Result<(), InstallError> {
    let backup = args.target.join(BACKUP_DIR);
    let outcome = if args.restore {
        // Wait for Kagero to exit by moving its executable aside first.
        let exe = args.target.join(APP_EXE);
        let parked = args.target.join(format!(".{APP_EXE}.old"));
        move_with_retry(&exe, &parked, WAIT_FOR_EXIT)?;
        let result = restore_backup(&args.target, &backup);
        if args.target.join(APP_EXE).exists() {
            let _ = fs::remove_file(&parked);
        } else {
            let _ = fs::rename(&parked, &exe);
        }
        result
    } else {
        let mut names: Vec<String> = fs::read_dir(&args.source)?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        install_files(&args.source, &args.target, &backup, &names, WAIT_FOR_EXIT)
    };
    // Start whichever version is in place now.
    let exe = args.target.join(APP_EXE);
    if exe.exists() {
        std::process::Command::new(&exe)
            .current_dir(&args.target)
            .spawn()?;
    }
    outcome
}

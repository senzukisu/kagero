//! The Windows installer and uninstaller (`kagero-setup.exe`).
//!
//! Kagero installs **per user** into `%LOCALAPPDATA%\Programs\Kagero`, so it needs
//! no administrator rights, and the built-in updater can keep replacing its files
//! there. The installer:
//!
//! * carries the release zip as a payload appended to its own executable (a footer
//!   at the end of the file says where it starts);
//! * checks the package (allow-listed names, and SHA256SUMS.txt when present);
//! * installs the files the same way the updater does: the previous version goes to
//!   `.kagero-backup`, and a failed install is rolled back;
//! * writes `uninstall.exe`, a Start menu shortcut, an optional desktop shortcut, and
//!   an entry in "Apps & features" (HKCU, no admin).
//!
//! Uninstalling removes exactly those files, shortcuts and the entry. It **never**
//! touches the user's data in `%APPDATA%\OnibiLab\Kagero` (settings, palettes,
//! themes, plugins, autosave copies) or files the user put into the install folder.

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::update_installer::{
    check_writable, extract_package, install_files, InstallError, APP_EXE, BACKUP_DIR, UPDATER_EXE,
};

/// Marks the end of an installer that carries a payload.
const MAGIC: &[u8; 16] = b"KAGERO-SETUP-01\0";
const FOOTER: usize = 8 + MAGIC.len();
/// The uninstaller's name in the install folder.
pub const UNINSTALL_EXE: &str = "uninstall.exe";
pub const SUMS_FILE: &str = "SHA256SUMS.txt";
/// "Apps & features" entry under HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall.
pub const UNINSTALL_KEY: &str = "Kagero";
const UNINSTALL_ROOT: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const SHORTCUT: &str = "Kagero.lnk";

/// Appends `payload` (the release zip) to the installer `stub`.
pub fn attach_payload(stub: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stub.len() + payload.len() + FOOTER);
    out.extend_from_slice(stub);
    out.extend_from_slice(payload);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    out
}

/// Where the payload is inside an installer file: `(stub length, payload length)`,
/// or `None` for an installer without one (e.g. `uninstall.exe`).
pub fn find_payload(file: &mut (impl Read + Seek)) -> std::io::Result<Option<(u64, u64)>> {
    let len = file.seek(SeekFrom::End(0))?;
    if len < FOOTER as u64 {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-(FOOTER as i64)))?;
    let mut footer = [0u8; FOOTER];
    file.read_exact(&mut footer)?;
    if &footer[8..] != MAGIC {
        return Ok(None);
    }
    let payload = u64::from_le_bytes(footer[..8].try_into().unwrap_or_default());
    let Some(stub) = payload
        .checked_add(FOOTER as u64)
        .and_then(|tail| len.checked_sub(tail))
    else {
        return Ok(None);
    };
    Ok(Some((stub, payload)))
}

/// Copies `len` bytes at `offset` of `from` into the file `to`.
fn copy_range(from: &Path, offset: u64, len: u64, to: &Path) -> std::io::Result<()> {
    let mut src = fs::File::open(from)?;
    src.seek(SeekFrom::Start(offset))?;
    let mut dst = fs::File::create(to)?;
    let copied = std::io::copy(&mut src.take(len), &mut dst)?;
    if copied != len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "the installer file is incomplete",
        ));
    }
    dst.flush()
}

/// Splits an installer file into its uninstaller part (the program without the
/// payload) and the release zip, written to `stub_out` and `zip_out`.
pub fn unpack_installer(installer: &Path, stub_out: &Path, zip_out: &Path) -> Result<(), String> {
    let mut file = fs::File::open(installer).map_err(|e| e.to_string())?;
    let (stub, payload) = find_payload(&mut file)
        .map_err(|e| e.to_string())?
        .ok_or("this installer contains no Kagero package")?;
    copy_range(installer, 0, stub, stub_out).map_err(|e| e.to_string())?;
    copy_range(installer, stub, payload, zip_out).map_err(|e| e.to_string())?;
    Ok(())
}

/// Checks the files in `dir` against its SHA256SUMS.txt, if there is one.
pub fn verify_sums(dir: &Path) -> Result<(), String> {
    let Ok(text) = fs::read_to_string(dir.join(SUMS_FILE)) else {
        return Ok(());
    };
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let (hash, name) = line
            .split_once("  ")
            .ok_or_else(|| format!("{SUMS_FILE} has an unreadable line"))?;
        let bytes =
            fs::read(dir.join(name.trim())).map_err(|_| format!("\"{name}\" is missing"))?;
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if !actual.eq_ignore_ascii_case(hash.trim()) {
            return Err(format!("\"{name}\" is damaged (checksum mismatch)"));
        }
    }
    Ok(())
}

/// Where shortcuts go (the real folders, or test folders).
#[derive(Debug, Clone)]
pub struct ShellPaths {
    pub start_menu: Option<PathBuf>,
    pub desktop: Option<PathBuf>,
}

impl ShellPaths {
    /// The current user's Start menu "Programs" folder and desktop.
    pub fn current_user() -> Self {
        let start_menu = std::env::var_os("APPDATA")
            .map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs"));
        Self {
            start_menu,
            desktop: desktop_dir(),
        }
    }
}

#[cfg(windows)]
fn desktop_dir() -> Option<PathBuf> {
    // The real desktop can be moved (e.g. into OneDrive); the shell keeps its place here.
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders")
        .ok()?;
    let raw: String = key.get_value("Desktop").ok()?;
    Some(PathBuf::from(expand_env(&raw)))
}

#[cfg(not(windows))]
fn desktop_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Desktop"))
}

/// Expands `%NAME%` references (for registry values of type REG_EXPAND_SZ).
fn expand_env(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The default install folder: `%LOCALAPPDATA%\Programs\Kagero`.
pub fn default_install_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Programs").join("Kagero"))
}

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub dir: PathBuf,
    pub start_menu_shortcut: bool,
    pub desktop_shortcut: bool,
    /// Add the "Apps & features" entry (off in tests).
    pub register: bool,
    pub version: String,
}

/// What an install or uninstall did, for the final page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub steps: Vec<String>,
    /// Things that didn't work but didn't stop the rest (e.g. a shortcut).
    pub warnings: Vec<String>,
}

/// Installs the release zip `package` and the uninstaller `uninstaller` (the setup
/// program without payload).
pub fn install(
    package: &Path,
    uninstaller: &Path,
    options: &InstallOptions,
    shell: &ShellPaths,
) -> Result<Report, String> {
    let dir = &options.dir;
    let mut report = Report::default();
    fs::create_dir_all(dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    check_writable(dir).map_err(|e| e.to_string())?;
    static RUNS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let run = RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging = std::env::temp_dir().join(format!("kagero-setup-{}-{run}", std::process::id()));
    let names = extract_package(package, &staging).map_err(|e| e.to_string())?;
    // A first install has no previous version: no backup must be left behind, or
    // Settings would offer to "restore" one (which would remove Kagero instead).
    let fresh = !dir.join(APP_EXE).exists();
    let result = (|| {
        verify_sums(&staging)?;
        install_files(
            &staging,
            dir,
            &dir.join(BACKUP_DIR),
            &names,
            Duration::from_secs(3),
        )
        .map_err(|e| match e {
            InstallError::StillRunning => {
                "Kagero is running from this folder — close it and try again".to_owned()
            }
            other => other.to_string(),
        })
    })();
    let _ = fs::remove_dir_all(&staging);
    result?;
    if fresh {
        let _ = fs::remove_dir_all(dir.join(BACKUP_DIR));
    }
    report.steps.push(format!(
        "Installed Kagero {} in {}",
        options.version,
        dir.display()
    ));

    fs::copy(uninstaller, dir.join(UNINSTALL_EXE))
        .map_err(|e| format!("can't write the uninstaller: {e}"))?;
    let exe = dir.join(APP_EXE);
    for (wanted, folder, what) in [
        (options.start_menu_shortcut, &shell.start_menu, "Start menu"),
        (options.desktop_shortcut, &shell.desktop, "desktop"),
    ] {
        if !wanted {
            continue;
        }
        match folder {
            Some(folder) => match make_shortcut(&exe, dir, &folder.join(SHORTCUT)) {
                Ok(()) => report.steps.push(format!("Added a {what} shortcut")),
                Err(e) => report.warnings.push(format!("No {what} shortcut: {e}")),
            },
            None => report
                .warnings
                .push(format!("No {what} shortcut: folder not found")),
        }
    }
    if options.register {
        match register(dir, &options.version) {
            Ok(()) => report.steps.push("Added Kagero to Apps & features".into()),
            Err(e) => report
                .warnings
                .push(format!("Not added to Apps & features: {e}")),
        }
    }
    Ok(report)
}

#[cfg(windows)]
fn make_shortcut(target: &Path, workdir: &Path, lnk: &Path) -> Result<(), String> {
    if let Some(parent) = lnk.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut link = mslnk::ShellLink::new(target).map_err(|e| e.to_string())?;
    link.set_working_dir(Some(workdir.to_string_lossy().into_owned()));
    link.set_name(Some("Kagero — pixel-art editor".to_owned()));
    link.create_lnk(lnk).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn make_shortcut(_target: &Path, _workdir: &Path, _lnk: &Path) -> Result<(), String> {
    Err("shortcuts are only made on Windows".into())
}

/// Values of the "Apps & features" entry.
pub fn registry_values(dir: &Path, version: &str) -> Vec<(&'static str, String)> {
    let exe = dir.join(APP_EXE);
    vec![
        ("DisplayName", "Kagero".to_owned()),
        ("DisplayVersion", version.to_owned()),
        ("Publisher", "Onibi Lab".to_owned()),
        ("InstallLocation", dir.display().to_string()),
        ("DisplayIcon", format!("{},0", exe.display())),
        (
            "UninstallString",
            format!("\"{}\"", dir.join(UNINSTALL_EXE).display()),
        ),
        (
            "URLInfoAbout",
            "https://github.com/senzukisu/kagero".to_owned(),
        ),
    ]
}

#[cfg(windows)]
fn register(dir: &Path, version: &str) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    let (key, _) = winreg::RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(format!(r"{UNINSTALL_ROOT}\{UNINSTALL_KEY}"))
        .map_err(|e| e.to_string())?;
    for (name, value) in registry_values(dir, version) {
        key.set_value(name, &value).map_err(|e| e.to_string())?;
    }
    key.set_value("NoModify", &1u32)
        .map_err(|e| e.to_string())?;
    key.set_value("NoRepair", &1u32)
        .map_err(|e| e.to_string())?;
    let size_kb: u64 = [APP_EXE, UPDATER_EXE, UNINSTALL_EXE]
        .iter()
        .filter_map(|n| fs::metadata(dir.join(n)).ok())
        .map(|m| m.len() / 1024)
        .sum();
    key.set_value("EstimatedSize", &(size_kb.min(u64::from(u32::MAX)) as u32))
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn register(_dir: &Path, _version: &str) -> Result<(), String> {
    Ok(())
}

/// Whether a shortcut file points into `dir` (its target path is stored in the
/// file, as ANSI and/or UTF-16 text).
pub fn shortcut_points_into(lnk: &Path, dir: &Path) -> bool {
    let Ok(bytes) = fs::read(lnk) else {
        return false;
    };
    let needle = dir.display().to_string().to_lowercase();
    let ansi: String = bytes
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .to_lowercase();
    if ansi.contains(&needle) {
        return true;
    }
    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let odd: Vec<u16> = bytes
        .get(1..)
        .unwrap_or_default()
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    [wide, odd]
        .iter()
        .any(|w| String::from_utf16_lossy(w).to_lowercase().contains(&needle))
}

/// Whether two paths are the same folder (by their real paths when both exist,
/// otherwise by their text, ignoring case and trailing separators).
pub fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => {
            let norm = |p: &Path| {
                p.display()
                    .to_string()
                    .trim_end_matches(['\\', '/'])
                    .to_lowercase()
            };
            norm(a) == norm(b)
        }
    }
}

#[cfg(windows)]
fn unregister(dir: &Path) -> Result<(), String> {
    // Only the entry of this install folder (not another copy's).
    if !registered_dir().is_some_and(|r| same_dir(&r, dir)) {
        return Ok(());
    }
    let root = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    match root.delete_subkey_all(format!(r"{UNINSTALL_ROOT}\{UNINSTALL_KEY}")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(windows))]
fn unregister(_dir: &Path) -> Result<(), String> {
    Ok(())
}

/// The install folder registered in "Apps & features", if Kagero was installed.
#[cfg(windows)]
pub fn registered_dir() -> Option<PathBuf> {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(format!(r"{UNINSTALL_ROOT}\{UNINSTALL_KEY}"))
        .ok()?;
    key.get_value::<String, _>("InstallLocation")
        .ok()
        .map(PathBuf::from)
}

#[cfg(not(windows))]
pub fn registered_dir() -> Option<PathBuf> {
    None
}

/// After an update: keeps the version in "Apps & features" right when this Kagero
/// is the installed one. Does nothing for portable copies.
#[cfg(windows)]
pub fn refresh_registration(running_dir: &Path, version: &str) {
    let Some(dir) = registered_dir() else {
        return;
    };
    if !same_dir(&dir, running_dir) {
        return;
    }
    if let Ok(key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            format!(r"{UNINSTALL_ROOT}\{UNINSTALL_KEY}"),
            winreg::enums::KEY_READ | winreg::enums::KEY_WRITE,
        )
    {
        let current: String = key.get_value("DisplayVersion").unwrap_or_default();
        if current != version {
            let _ = key.set_value("DisplayVersion", &version.to_owned());
        }
    }
}

#[cfg(not(windows))]
pub fn refresh_registration(_running_dir: &Path, _version: &str) {}

/// Whether `dir` looks like a Kagero install folder (protects against deleting the
/// wrong folder).
pub fn is_install_dir(dir: &Path) -> bool {
    dir.join(APP_EXE).is_file() || dir.join(UNINSTALL_EXE).is_file()
}

/// Removes Kagero from `dir`: its program files, backup, uninstaller, shortcuts and
/// "Apps & features" entry (when `unregister_entry`). Other files in the folder and
/// all user data elsewhere are kept. The folder itself is removed only if empty.
pub fn uninstall(dir: &Path, shell: &ShellPaths, unregister_entry: bool) -> Result<Report, String> {
    if !is_install_dir(dir) {
        return Err(format!("{} is not a Kagero folder", dir.display()));
    }
    let mut report = Report::default();
    // The program first: if Kagero is running, stop before changing anything.
    let exe = dir.join(APP_EXE);
    if exe.exists() {
        remove_with_retry(&exe, Duration::from_secs(5))
            .map_err(|_| "Kagero is running — close it and try again".to_owned())?;
    }
    for name in [UPDATER_EXE, SUMS_FILE] {
        let p = dir.join(name);
        if p.exists() {
            if let Err(e) = remove_with_retry(&p, Duration::from_secs(5)) {
                report.warnings.push(format!("{name} not removed: {e}"));
            }
        }
    }
    let backup = dir.join(BACKUP_DIR);
    if backup.exists() {
        if let Err(e) = fs::remove_dir_all(&backup) {
            report
                .warnings
                .push(format!("old version backup not removed: {e}"));
        }
    }
    // The uninstaller may still be closing (it started this copy): wait for it.
    let uninstaller = dir.join(UNINSTALL_EXE);
    if uninstaller.exists() {
        if let Err(e) = remove_with_retry(&uninstaller, Duration::from_secs(15)) {
            report
                .warnings
                .push(format!("{UNINSTALL_EXE} not removed: {e}"));
        }
    }
    report
        .steps
        .push(format!("Removed Kagero from {}", dir.display()));
    for folder in [&shell.start_menu, &shell.desktop].into_iter().flatten() {
        let lnk = folder.join(SHORTCUT);
        // Only shortcuts to this copy (a portable Kagero may have its own).
        if lnk.exists() && shortcut_points_into(&lnk, dir) {
            match fs::remove_file(&lnk) {
                Ok(()) => report
                    .steps
                    .push(format!("Removed the shortcut in {}", folder.display())),
                Err(e) => report.warnings.push(format!("Shortcut not removed: {e}")),
            }
        }
    }
    if unregister_entry {
        match unregister(dir) {
            Ok(()) => report
                .steps
                .push("Removed Kagero from Apps & features".into()),
            Err(e) => report
                .warnings
                .push(format!("Apps & features entry not removed: {e}")),
        }
    }
    // Only an empty folder is removed: anything the user put there stays.
    if fs::remove_dir(dir).is_err() && dir.exists() {
        report.warnings.push(format!(
            "{} was kept because it contains other files",
            dir.display()
        ));
    }
    Ok(report)
}

fn remove_with_retry(path: &Path, timeout: Duration) -> std::io::Result<()> {
    let start = std::time::Instant::now();
    loop {
        match fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(e) if start.elapsed() >= timeout => return Err(e),
            Err(_) => std::thread::sleep(Duration::from_millis(200)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_round_trip() {
        let stub = b"MZ fake program".to_vec();
        let zip = b"PK fake zip bytes".to_vec();
        let installer = attach_payload(&stub, &zip);
        let mut cur = std::io::Cursor::new(installer.clone());
        assert_eq!(
            find_payload(&mut cur).expect("io"),
            Some((stub.len() as u64, zip.len() as u64))
        );
        let mut plain = std::io::Cursor::new(stub);
        assert_eq!(find_payload(&mut plain).expect("io"), None);
        let mut short = std::io::Cursor::new(b"x".to_vec());
        assert_eq!(find_payload(&mut short).expect("io"), None);
        // A footer claiming more payload than the file has is rejected.
        let mut bad = installer;
        let n = bad.len();
        bad[n - FOOTER..n - MAGIC.len()].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            find_payload(&mut std::io::Cursor::new(bad)).expect("io"),
            None
        );
    }

    #[test]
    fn env_expansion() {
        std::env::set_var("KAGERO_TEST_X", "C:\\Users\\me");
        assert_eq!(
            expand_env("%KAGERO_TEST_X%\\Desktop"),
            "C:\\Users\\me\\Desktop"
        );
        assert_eq!(expand_env("%NOPE_NOT_SET_9%\\a"), "%NOPE_NOT_SET_9%\\a");
        assert_eq!(expand_env("100%"), "100%");
    }
}

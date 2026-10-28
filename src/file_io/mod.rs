//! Loading, saving, exporting and validating files.

pub mod kgr;
pub mod palette;
pub mod png;

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

pub use kgr::{load_kgr, read_kgr, save_kgr, write_kgr, KGR_EXTENSION, KGR_FORMAT_VERSION};
pub use png::{export_frames, export_png, frame_export_paths, import_png};

/// Largest `.kgr` file Kagero will open.
pub const MAX_KGR_FILE_BYTES: u64 = 1024 * 1024 * 1024;
/// Largest `.png` file Kagero will import.
pub const MAX_PNG_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// Errors from file operations. The `Display` text is shown to the user.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the project archive is damaged or not a Kagero project ({0})")]
    Zip(#[from] zip::result::ZipError),
    #[error("could not read or write the image: {0}")]
    Image(#[from] image::ImageError),
    #[error("the project data is damaged: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the file is not valid: {0}")]
    Invalid(String),
    #[error("the file is too large: {0}")]
    TooLarge(String),
    #[error("this project was made with a newer Kagero (format version {found}); this version supports up to {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error(transparent)]
    Core(#[from] crate::core::CoreError),
}

/// Refuses files bigger than `limit` before reading them.
pub(crate) fn check_file_size(path: &Path, limit: u64) -> Result<(), FileError> {
    let len = fs::metadata(path)?.len();
    if len > limit {
        return Err(FileError::TooLarge(format!(
            "{} MiB (limit {} MiB)",
            len / (1024 * 1024),
            limit / (1024 * 1024)
        )));
    }
    Ok(())
}

/// Writes a file through a temporary sibling and renames it over the target, so a crash
/// or error never leaves a half-written file in place of the old one.
pub fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> Result<(), FileError>,
) -> Result<(), FileError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| FileError::Invalid("the target path has no file name".to_owned()))?;
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    let result: Result<(), FileError> = (|| {
        let mut writer = BufWriter::new(File::create(&tmp)?);
        write(&mut writer)?;
        writer.flush()?;
        let file = writer.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
        drop(file);
        // On Windows `rename` replaces an existing target (MoveFileEx with REPLACE_EXISTING).
        fs::rename(&tmp, path)?;
        Ok(())
    })();

    if result.is_err() {
        if let Err(e) = fs::remove_file(&tmp) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("could not remove temporary file {}: {e}", tmp.display());
            }
        }
    }
    result
}

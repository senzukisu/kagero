//! Native file dialogs (blocking, run on the UI thread).

use std::path::{Path, PathBuf};

use crate::file_io::KGR_EXTENSION;

/// Asks for a `.kgr` or `.png` file to open.
pub fn pick_open(start_dir: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Open")
        .add_filter("Kagero project or PNG", &[KGR_EXTENSION, "png"])
        .add_filter("Kagero project", &[KGR_EXTENSION])
        .add_filter("PNG image", &["png"]);
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}

/// Asks where to save a project; always returns a path ending in `.kgr`.
pub fn pick_save_project(start_dir: Option<&Path>, name: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save Project")
        .add_filter("Kagero project", &[KGR_EXTENSION])
        .set_file_name(format!("{name}.{KGR_EXTENSION}"));
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.save_file().map(|p| with_extension(p, KGR_EXTENSION))
}

/// Asks where to export a PNG; always returns a path ending in `.png`.
pub fn pick_export_png(start_dir: Option<&Path>, name: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Export PNG")
        .add_filter("PNG image", &["png"])
        .set_file_name(format!("{name}.png"));
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.save_file().map(|p| with_extension(p, "png"))
}

/// Appends `ext` unless the path already has it (case-insensitive).
pub fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
    let has = path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(ext));
    if has {
        path
    } else {
        let mut s = path.into_os_string();
        s.push(".");
        s.push(ext);
        PathBuf::from(s)
    }
}

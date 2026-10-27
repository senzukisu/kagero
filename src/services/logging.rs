//! Logging setup. Release builds have no console, so logs go to a file next to the settings.

use std::path::Path;
use std::sync::Mutex;

/// Starts logging to `<dir>/kagero.log` (overwritten each run), or to stderr if that fails.
pub fn init(dir: Option<&Path>) {
    let file = dir.and_then(|dir| {
        std::fs::create_dir_all(dir).ok()?;
        std::fs::File::create(dir.join("kagero.log")).ok()
    });
    let builder = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false);
    let result = match file {
        Some(file) => builder
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .try_init(),
        None => builder.with_writer(std::io::stderr).try_init(),
    };
    if let Err(e) = result {
        eprintln!("logging is not available: {e}");
    }
}

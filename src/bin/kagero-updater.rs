// The updater helper runs without a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Installs a verified Kagero update after Kagero has exited, or restores the previous
//! version. Started only by Kagero itself with local paths; see `docs/developers/updater-internals.md`.

use kagero::services::update_installer::{run_helper, HelperArgs};

fn main() {
    let Some(args) = HelperArgs::parse(std::env::args_os().skip(1)) else {
        eprintln!("kagero-updater is started by Kagero; it takes no user arguments.");
        std::process::exit(2);
    };
    let log_dir = args.target.join(".kagero-backup");
    let _ = std::fs::create_dir_all(&log_dir);
    let message = match run_helper(&args) {
        Ok(()) => "update finished successfully".to_owned(),
        Err(e) => format!("update failed: {e}"),
    };
    let _ = std::fs::write(log_dir.join("update.log"), format!("{message}\n"));
}

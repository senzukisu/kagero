// Release builds are GUI-only (no console window on Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kagero::app::KageroApp;
use kagero::config::Config;
use kagero::services::logging;
use kagero::ui::icons::logo_rgba;

fn main() {
    logging::init(Config::default_dir().as_deref());
    tracing::info!("Kagero {} starting", kagero::APP_VERSION);

    let (rgba, width, height) = logo_rgba(4);
    // Open maximized unless the user left Kagero in a window last time; a windowed
    // start is centered on the screen instead of wherever the OS puts it.
    let maximized = Config::default_path()
        .map(|p| Config::load(&p).ui.window_maximized)
        .unwrap_or(true);
    let options = eframe::NativeOptions {
        centered: !maximized,
        viewport: egui::ViewportBuilder::default()
            .with_title("Kagero")
            .with_app_id("kagero")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([820.0, 540.0])
            .with_maximized(maximized)
            .with_icon(egui::IconData {
                rgba,
                width,
                height,
            }),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "Kagero",
        options,
        Box::new(|cc| {
            let mut app = KageroApp::new(cc);
            // `kagero.exe path\to\file.kgr` (e.g. "Open with" in Explorer) opens it.
            if let Some(path) = std::env::args_os().nth(1) {
                app.open_on_start(std::path::PathBuf::from(path));
            }
            Ok(Box::new(app))
        }),
    ) {
        tracing::error!("Kagero could not start: {e}");
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Kagero")
            .set_description(format!(
                "Kagero could not start.\n\n{e}\n\nYour graphics driver may not support OpenGL 2.0 or newer."
            ))
            .show();
        std::process::exit(1);
    }
}

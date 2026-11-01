//! Kagero's installer and uninstaller (see `kagero::services::setup`).
//!
//! * `kagero-<version>-setup.exe`: shows the install window.
//!   Silent: `--silent [--dir <folder>] [--desktop-shortcut] [--no-start-menu]
//!   [--no-register]` (no "Apps & features" entry).
//! * `uninstall.exe` (the same program without its package, in the install folder):
//!   shows the uninstall window. It copies itself to the temp folder and continues
//!   from there, so it can remove the install folder. Silent: `--uninstall --silent`.
//! * Release script: `kagero-setup.exe --build-installer <package.zip> <out.exe>`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use kagero::services::setup::{self, InstallOptions, Report, ShellPaths};
use kagero::theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let code = if let Some(zip) = value("--build-installer") {
        build_installer(&zip, args.last().map(String::as_str).unwrap_or_default())
    } else if let Some(dir) = value("--uninstall-from") {
        // The temp copy does the work and then reports.
        finish_uninstall(Path::new(&dir), has("--silent"))
    } else if has("--uninstall") || is_uninstaller() {
        if has("--silent") {
            start_uninstall(true)
        } else {
            run_window(Mode::Uninstall)
        }
    } else if has("--silent") {
        silent_install(
            value("--dir"),
            has("--desktop-shortcut"),
            !has("--no-start-menu"),
            !has("--no-register"),
        )
    } else {
        run_window(Mode::Install)
    };
    std::process::exit(code);
}

/// `uninstall.exe` has no package attached.
fn is_uninstaller() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let named = exe
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case(setup::UNINSTALL_EXE));
    named
        || std::fs::File::open(&exe)
            .ok()
            .and_then(|mut f| setup::find_payload(&mut f).ok())
            .is_some_and(|p| p.is_none())
}

fn build_installer(zip: &str, out: &str) -> i32 {
    let result = (|| -> Result<(), String> {
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        let stub = std::fs::read(&me).map_err(|e| e.to_string())?;
        let payload = std::fs::read(zip).map_err(|e| format!("{zip}: {e}"))?;
        std::fs::write(out, setup::attach_payload(&stub, &payload))
            .map_err(|e| format!("{out}: {e}"))
    })();
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// Splits this installer into the uninstaller and the package (in a temp folder).
fn unpack() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let work = std::env::temp_dir().join(format!("kagero-setup-{}-unpack", std::process::id()));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let (stub, zip) = (work.join("uninstall.exe"), work.join("package.zip"));
    setup::unpack_installer(&me, &stub, &zip)?;
    Ok((work, stub, zip))
}

fn do_install(options: &InstallOptions) -> Result<Report, String> {
    let (work, stub, zip) = unpack()?;
    let result = setup::install(&zip, &stub, options, &ShellPaths::current_user());
    let _ = std::fs::remove_dir_all(work);
    result
}

fn silent_install(dir: Option<String>, desktop: bool, start_menu: bool, register: bool) -> i32 {
    let Some(dir) = dir.map(PathBuf::from).or_else(setup::default_install_dir) else {
        return 1;
    };
    let options = InstallOptions {
        dir,
        start_menu_shortcut: start_menu,
        desktop_shortcut: desktop,
        register,
        version: VERSION.to_owned(),
    };
    match do_install(&options) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// Continues uninstalling from a temp copy (so the install folder can be removed).
fn start_uninstall(silent: bool) -> i32 {
    let result = (|| -> Result<(), String> {
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        let dir = me.parent().ok_or("unknown folder")?.to_path_buf();
        if !setup::is_install_dir(&dir) {
            return Err(format!("{} is not a Kagero folder", dir.display()));
        }
        let copy =
            std::env::temp_dir().join(format!("kagero-uninstall-{}.exe", std::process::id()));
        std::fs::copy(&me, &copy).map_err(|e| e.to_string())?;
        let mut cmd = std::process::Command::new(&copy);
        cmd.arg("--uninstall-from").arg(&dir);
        if silent {
            cmd.arg("--silent");
        }
        cmd.spawn().map_err(|e| e.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => 0,
        Err(e) => {
            message(true, &format!("Kagero could not be uninstalled.\n\n{e}"));
            1
        }
    }
}

fn finish_uninstall(dir: &Path, silent: bool) -> i32 {
    let result = setup::uninstall(dir, &ShellPaths::current_user(), true);
    delete_self_later();
    if silent {
        return i32::from(result.is_err());
    }
    match result {
        Ok(report) => {
            let mut text = String::from(
                "Kagero was uninstalled.\n\nYour projects, settings, palettes, themes, plugins and autosave copies were kept.",
            );
            for w in &report.warnings {
                text.push_str("\n• ");
                text.push_str(w);
            }
            message(false, &text);
            0
        }
        Err(e) => {
            message(true, &format!("Kagero could not be uninstalled.\n\n{e}"));
            1
        }
    }
}

/// The temp copy that did the uninstall removes itself a moment after it exits.
fn delete_self_later() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let Ok(me) = std::env::current_exe() else {
            return;
        };
        // Only the temp copy (never the installer or an installed file).
        let in_temp = me.starts_with(std::env::temp_dir())
            && me
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("kagero-uninstall-"));
        if !in_temp {
            return;
        }
        // cmd.exe has its own quoting rules, so the line is passed to it unchanged.
        let _ = std::process::Command::new("cmd")
            .raw_arg(format!(
                "/C ping -n 4 127.0.0.1 >nul & del /f /q \"{}\"",
                me.display()
            ))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

fn message(error: bool, text: &str) {
    let _ = rfd::MessageDialog::new()
        .set_level(if error {
            rfd::MessageLevel::Error
        } else {
            rfd::MessageLevel::Info
        })
        .set_title("Kagero Setup")
        .set_description(text)
        .show();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Install,
    Uninstall,
}

enum Page {
    Ask,
    Done(Report),
    Failed(String),
}

struct SetupApp {
    mode: Mode,
    page: Page,
    dir: PathBuf,
    desktop: bool,
    start_menu: bool,
    launch: bool,
    logo: Option<egui::TextureHandle>,
}

fn run_window(mode: Mode) -> i32 {
    let (rgba, width, height) = kagero::ui::icons::logo_rgba(4);
    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_title(match mode {
                Mode::Install => "Install Kagero",
                Mode::Uninstall => "Uninstall Kagero",
            })
            .with_inner_size([520.0, 330.0])
            .with_resizable(false)
            .with_icon(egui::IconData {
                rgba,
                width,
                height,
            }),
        ..Default::default()
    };
    let dir = match mode {
        Mode::Install => setup::registered_dir()
            .or_else(setup::default_install_dir)
            .unwrap_or_else(|| PathBuf::from("Kagero")),
        Mode::Uninstall => std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
            .unwrap_or_default(),
    };
    let app = SetupApp {
        mode,
        page: Page::Ask,
        dir,
        desktop: true,
        start_menu: true,
        launch: true,
        logo: None,
    };
    match eframe::run_native(
        "Kagero Setup",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            theme::sync_fonts(&cc.egui_ctx, false);
            Ok(Box::new(app))
        }),
    ) {
        Ok(()) => 0,
        Err(e) => {
            message(true, &format!("The setup window could not open: {e}"));
            1
        }
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::panel())
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tex = self.logo.get_or_insert_with(|| {
                        let (rgba, w, h) = kagero::ui::icons::logo_rgba(3);
                        let img = egui::ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            &rgba,
                        );
                        ctx.load_texture("setup-logo", img, egui::TextureOptions::NEAREST)
                    });
                    ui.image((tex.id(), egui::vec2(48.0, 48.0)));
                    ui.vertical(|ui| {
                        let title = match self.mode {
                            Mode::Install => format!("Kagero {VERSION}"),
                            Mode::Uninstall => "Uninstall Kagero".to_owned(),
                        };
                        ui.label(
                            egui::RichText::new(title)
                                .font(theme::font(2))
                                .color(theme::text_primary()),
                        );
                        ui.label(
                            egui::RichText::new("Pixel-art and animation editor by Onibi Lab")
                                .color(theme::text_secondary()),
                        );
                    });
                });
                ui.add_space(12.0);
                match &self.page {
                    Page::Ask => self.ask(ui, ctx),
                    Page::Done(report) => {
                        let report = report.clone();
                        self.done(ui, ctx, &report);
                    }
                    Page::Failed(e) => {
                        let e = e.clone();
                        ui.label(egui::RichText::new(&e).color(theme::error()));
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui.button("Try Again").clicked() {
                                self.page = Page::Ask;
                            }
                            if ui.button("Close").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    }
                }
            });
    }
}

impl SetupApp {
    fn ask(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        match self.mode {
            Mode::Install => {
                ui.label("Install for your user account (no administrator rights needed) in:");
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(self.dir.display().to_string())
                            .monospace()
                            .color(theme::text_primary()),
                    );
                    if ui.small_button("Change…").clicked() {
                        if let Some(parent) = rfd::FileDialog::new()
                            .set_title("Install Kagero in")
                            .pick_folder()
                        {
                            self.dir = parent.join("Kagero");
                        }
                    }
                });
                ui.add_space(6.0);
                ui.checkbox(&mut self.start_menu, "Start menu shortcut");
                ui.checkbox(&mut self.desktop, "Desktop shortcut");
                ui.checkbox(&mut self.launch, "Start Kagero when done");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "An older Kagero in this folder is replaced (and kept as the previous version). Updates install here automatically later.",
                    )
                    .small()
                    .color(theme::text_secondary()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(kagero::ui::dialogs::accent_button("Install"))
                        .clicked()
                    {
                        let options = InstallOptions {
                            dir: self.dir.clone(),
                            start_menu_shortcut: self.start_menu,
                            desktop_shortcut: self.desktop,
                            register: true,
                            version: VERSION.to_owned(),
                        };
                        self.page = match do_install(&options) {
                            Ok(report) => Page::Done(report),
                            Err(e) => Page::Failed(format!("Kagero was not installed: {e}")),
                        };
                    }
                    if ui.button("Cancel").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            }
            Mode::Uninstall => {
                ui.label(format!(
                    "This removes Kagero from {}, its shortcuts and its entry in Apps & features.",
                    self.dir.display()
                ));
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Your projects, settings, palettes, themes, plugins and autosave copies are kept.",
                    )
                    .color(theme::success()),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let clicked = ui
                        .add(kagero::ui::dialogs::accent_button("Uninstall"))
                        .clicked();
                    // The work continues in a temp copy; this window closes.
                    if clicked && start_uninstall(false) == 0 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Cancel").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            }
        }
    }

    fn done(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, report: &Report) {
        ui.label(
            egui::RichText::new("Kagero is installed.")
                .strong()
                .color(theme::success()),
        );
        for step in &report.steps {
            ui.label(
                egui::RichText::new(format!("✓ {step}"))
                    .small()
                    .color(theme::text_secondary()),
            );
        }
        for w in &report.warnings {
            ui.label(egui::RichText::new(w).small().color(theme::error()));
        }
        ui.add_space(12.0);
        if ui
            .add(kagero::ui::dialogs::accent_button("Finish"))
            .clicked()
        {
            if self.launch {
                let _ = std::process::Command::new(self.dir.join("kagero.exe"))
                    .current_dir(&self.dir)
                    .spawn();
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

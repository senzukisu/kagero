//! The eframe application: screens, panels, dialogs and command dispatch.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Frame, Margin, ViewportCommand};

use crate::config::{Config, TIMELINE_MAX_HEIGHT, TIMELINE_MIN_HEIGHT};
use crate::core::palette::{self as palettes, Palette};
use crate::core::Document;
use crate::editor::{display_name, Editor};
use crate::export::{ExportKind, FrameSelection};
use crate::file_io::palette as palette_io;
use crate::file_io::{self, FileError};
use crate::integrations::discord_rpc::{build_presence, AppMode, DiscordPresence, Flags};
use crate::release;
use crate::services::dialogs as native;
use crate::services::update_installer;
use crate::services::updater::{self, UpdateEvent, UpdateManager, UpdateStatus, UreqClient};
use crate::theme;
use crate::ui::animation_preview::Player;
use crate::ui::canvas::{CanvasOptions, CanvasView};
use crate::ui::dialogs::{self, ConfirmChoice, NewDocumentResult, NewDocumentState, UnsavedChoice};
use crate::ui::export_dialog::{ExportDialog, ExportDialogAction};
use crate::ui::layers::RenameState;
use crate::ui::menu::MenuState;
use crate::ui::palette::PaletteRename;
use crate::ui::settings::{SettingsAction, SettingsTab, SettingsView, SettingsWindow};
use crate::ui::shortcuts::KeyContext;
use crate::ui::status::{MessageKind, StatusMessage};
use crate::ui::timeline::{Timeline, TimelineSettings, HEADER_HEIGHT};
use crate::ui::{home, layers, menu, palette, shortcuts, status, tool_options, toolbar, Command};

/// How often automatic update checks are considered (the interval itself is hours).
const AUTO_CHECK_POLL: Duration = Duration::from_secs(60);
/// Settings changes are written at most this often.
const CONFIG_SAVE_DELAY: Duration = Duration::from_millis(600);

/// An action that may discard the open document and so must ask about unsaved changes.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    NewDocument,
    OpenPath(PathBuf),
    CloseDocument,
    Exit,
    InstallUpdate,
    RestorePrevious,
}

enum Dialog {
    NewDocument(NewDocumentState),
    Unsaved(Pending),
    Error {
        title: String,
        message: String,
    },
    About,
    /// Kagero's and the third-party licenses; the open tab.
    Licenses(u8),
    CanvasSize(dialogs::CanvasSizeState),
    Recovery(dialogs::RecoveryState),
    Tag(dialogs::TagState),
    ConvertPalette(dialogs::ConvertPaletteState),
    ConfirmDeleteFrame(usize),
}

pub struct KageroApp {
    config: Config,
    config_path: Option<PathBuf>,
    config_save_at: Option<Instant>,
    editor: Option<Editor>,
    canvas: CanvasView,
    timeline: Timeline,
    player: Player,
    settings: SettingsWindow,
    export: ExportDialog,
    palette_rename: PaletteRename,
    /// Last seen maximized state of the window (`None` until the first frame).
    seen_maximized: Option<bool>,
    /// The saved window state was applied on the first frame.
    window_restored: bool,
    updater: UpdateManager,
    discord: DiscordPresence,
    session_start_unix: i64,
    last_auto_check_poll: Option<Instant>,
    notified_version: Option<semver::Version>,
    has_backup: bool,
    dialog: Option<Dialog>,
    /// This session's autosave copy and the recovery folder.
    recovery: crate::services::recovery::Recovery,
    /// When the last autosave copy was started (or the document changed hands).
    last_autosave: Instant,
    /// Built-in and imported color themes.
    themes: crate::themes::ThemeStore,
    /// While a theme is previewed in Settings: the one being shown.
    theme_preview: Option<String>,
    plugins: crate::plugins::PluginStore,
    /// A plugin command running in the background, with the layer it works on.
    plugin_job: Option<(
        crate::plugins::PluginJob,
        crate::core::FrameId,
        crate::core::LayerId,
    )>,
    status: Option<StatusMessage>,
    rename: RenameState,
    allow_close: bool,
    window_title: String,
}

impl KageroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let config_path = Config::default_path();
        let config = config_path.as_deref().map(Config::load).unwrap_or_default();
        let recovery = crate::services::recovery::Recovery::start(
            Config::default_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("recovery"),
        );
        let orphans = recovery.orphans();
        // After a self-update, keep the version in "Apps & features" right.
        if let Ok(dir) = crate::services::update_installer::install_dir() {
            crate::services::setup::refresh_registration(&dir, crate::APP_VERSION);
        }
        let themes =
            crate::themes::ThemeStore::load(Config::default_dir().map(|d| d.join("themes")));
        let colors = themes
            .find(&config.theme)
            .map_or(crate::theme::EMBER, |t| t.colors);
        crate::theme::set_colors(colors);
        theme::apply(&cc.egui_ctx);
        let canvas = CanvasView::new(config.show_grid);
        let repaint_ctx = cc.egui_ctx.clone();
        let updater = UpdateManager::new(
            Arc::new(UreqClient::new()),
            Arc::new(move || repaint_ctx.request_repaint()),
        );
        let mut app = Self {
            config,
            config_path,
            config_save_at: None,
            editor: None,
            canvas,
            timeline: Timeline::default(),
            player: Player::default(),
            settings: SettingsWindow::default(),
            export: ExportDialog::default(),
            palette_rename: None,
            seen_maximized: None,
            window_restored: false,
            updater,
            discord: DiscordPresence::start(release::DISCORD_APPLICATION_ID),
            session_start_unix: updater::unix_now() as i64,
            last_auto_check_poll: None,
            notified_version: None,
            has_backup: false,
            dialog: None,
            recovery,
            last_autosave: Instant::now(),
            themes,
            theme_preview: None,
            plugins: crate::plugins::PluginStore::load(
                Config::default_dir().map(|d| d.join("plugins")),
            ),
            plugin_job: None,
            status: None,
            rename: None,
            allow_close: false,
            window_title: String::new(),
        };
        if !orphans.is_empty() {
            app.dialog = Some(Dialog::Recovery(dialogs::RecoveryState {
                entries: orphans,
                errors: Default::default(),
            }));
        }
        app
    }

    /// Opens a `.kgr` or `.png` given on the command line.
    pub fn open_on_start(&mut self, path: PathBuf) {
        self.open_path(&path);
    }

    // ------------------------------------------------------------ messages

    fn info(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMessage::new(MessageKind::Info, text));
    }

    fn success(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMessage::new(MessageKind::Success, text));
    }

    fn status_error(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMessage::new(MessageKind::Error, text));
    }

    fn message_dialog(&mut self, title: impl Into<String>, message: String) {
        let title = title.into();
        tracing::error!("{title}: {message}");
        self.status_error(title.clone());
        self.dialog = Some(Dialog::Error { title, message });
    }

    fn error_dialog(&mut self, title: impl Into<String>, err: &FileError) {
        self.message_dialog(title, err.to_string());
    }

    /// Writes settings soon (several quick changes become one write).
    fn schedule_config_save(&mut self) {
        self.config_save_at = Some(Instant::now() + CONFIG_SAVE_DELAY);
    }

    fn save_config(&mut self) {
        self.config_save_at = None;
        let Some(path) = self.config_path.clone() else {
            return;
        };
        if let Err(e) = self.config.save(&path) {
            tracing::warn!("could not save settings: {e}");
            self.status_error(format!("Settings were not saved: {e}"));
        }
    }

    /// Folder to start file dialogs in.
    fn start_dir(&self) -> Option<PathBuf> {
        self.editor
            .as_ref()
            .and_then(|e| e.path())
            .or_else(|| self.config.recent_files.first().map(PathBuf::as_path))
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    }

    fn stop_playback(&mut self) {
        if let Some(editor) = &mut self.editor {
            self.player.stop(editor);
        } else {
            self.player.reset();
        }
    }

    // ------------------------------------------------------------ documents

    fn request(&mut self, ctx: &egui::Context, pending: Pending) {
        if self.editor.as_ref().is_some_and(Editor::is_modified) {
            self.dialog = Some(Dialog::Unsaved(pending));
        } else {
            self.perform(ctx, pending);
        }
    }

    fn perform(&mut self, ctx: &egui::Context, pending: Pending) {
        match pending {
            Pending::NewDocument => {
                self.dialog = Some(Dialog::NewDocument(NewDocumentState {
                    width: self.config.new_width,
                    height: self.config.new_height,
                    background: dialogs::Background::Transparent,
                }));
            }
            Pending::OpenPath(path) => self.open_path(&path),
            Pending::CloseDocument => {
                self.player.reset();
                self.editor = None;
                self.rename = None;
                self.canvas.reset();
                self.timeline.clear();
            }
            Pending::Exit => self.exit(ctx),
            Pending::InstallUpdate => {
                let UpdateStatus::Downloaded { release, package } = self.updater.status() else {
                    return;
                };
                if !package.signature_verified {
                    self.message_dialog(
                        "The update was not installed",
                        "Its signature could not be verified.".to_owned(),
                    );
                    return;
                }
                match update_installer::launch_install(&package.path, &release.version.to_string())
                {
                    Ok(()) => {
                        tracing::info!("installing Kagero {}", release.version);
                        self.exit(ctx);
                    }
                    Err(e) => self.message_dialog("The update was not installed", e.to_string()),
                }
            }
            Pending::RestorePrevious => match update_installer::launch_restore() {
                Ok(()) => self.exit(ctx),
                Err(e) => {
                    self.message_dialog("The previous version was not restored", e.to_string())
                }
            },
        }
    }

    fn exit(&mut self, ctx: &egui::Context) {
        self.stop_playback();
        // Leaving on purpose: the work was saved or deliberately discarded.
        self.recovery.discard();
        self.save_config();
        self.allow_close = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    /// Installs a new document, keeping the tool and colors of the previous one.
    fn set_document(&mut self, doc: Document, path: Option<PathBuf>, name: String) {
        self.player.reset();
        // The previous document was saved or deliberately discarded by now.
        self.recovery.discard();
        self.last_autosave = Instant::now();
        let mut editor = Editor::new(doc, path, name);
        if let Some(old) = &self.editor {
            editor.tool = old.tool;
            editor.settings = old.settings;
            editor.primary = old.primary;
            editor.secondary = old.secondary;
            editor.clipboard = old.clipboard.clone();
        }
        self.editor = Some(editor);
        self.rename = None;
        self.canvas.reset();
        self.timeline.clear();
    }

    fn create_document(&mut self, state: &NewDocumentState) {
        match Document::new(state.width, state.height, state.background.color()) {
            Ok(doc) => {
                self.set_document(doc, None, "Untitled".to_owned());
                self.config.new_width = state.width;
                self.config.new_height = state.height;
                self.save_config();
                self.info(format!("New image {}×{}", state.width, state.height));
            }
            Err(e) => self.error_dialog("Could not create the image", &e.into()),
        }
    }

    fn open_path(&mut self, path: &Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let result = match ext.as_str() {
            file_io::KGR_EXTENSION => {
                file_io::load_kgr(path).map(|d| (d, Some(path.to_path_buf())))
            }
            "png" => file_io::import_png(path).map(|d| (d, None)),
            _ => Err(FileError::Invalid(
                "Kagero opens .kgr projects and .png images".to_owned(),
            )),
        };
        let file_name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        match result {
            Ok((doc, save_path)) => {
                let imported = save_path.is_none();
                let frames = doc.frame_count();
                self.set_document(doc, save_path, display_name(path));
                self.config.add_recent(path);
                self.save_config();
                if imported {
                    self.info(format!(
                        "Imported {file_name} — use Save to keep it as a .kgr project"
                    ));
                } else if frames > 1 {
                    self.success(format!("Opened {file_name} · {frames} frames"));
                } else {
                    self.success(format!("Opened {file_name}"));
                }
                tracing::info!("opened {}", path.display());
            }
            Err(e) => self.error_dialog(format!("Could not open “{file_name}”"), &e),
        }
    }

    /// Saves to the current path, or asks for one. Returns `true` on success.
    fn save(&mut self) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        editor.end_stroke();
        match editor.path().map(Path::to_path_buf) {
            Some(path) => self.save_to(path),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        editor.end_stroke();
        let name = editor.name().to_owned();
        match native::pick_save_project(self.start_dir().as_deref(), &name) {
            Some(path) => self.save_to(path),
            None => false,
        }
    }

    fn save_to(&mut self, path: PathBuf) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        match file_io::save_kgr(editor.document(), &path) {
            Ok(()) => {
                editor.mark_saved(path.clone());
                self.recovery.discard();
                self.config.add_recent(&path);
                self.save_config();
                self.success(format!("Saved {}", path.display()));
                tracing::info!("saved {}", path.display());
                true
            }
            Err(e) => {
                self.error_dialog("The project was not saved", &e);
                false
            }
        }
    }

    fn export_png(&mut self) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        editor.end_stroke();
        let name = editor.name().to_owned();
        let Some(path) = native::pick_export_png(self.start_dir().as_deref(), &name) else {
            return;
        };
        let Some(editor) = &self.editor else {
            return;
        };
        match file_io::export_png(editor.document(), &path) {
            Ok(()) => self.success(format!("Exported {}", path.display())),
            Err(e) => self.error_dialog("The PNG was not exported", &e),
        }
    }

    // ------------------------------------------------------------ commands

    fn execute(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::NewDocument => self.request(ctx, Pending::NewDocument),
            Command::Open => {
                if let Some(path) = native::pick_open(self.start_dir().as_deref()) {
                    self.request(ctx, Pending::OpenPath(path));
                }
            }
            Command::OpenPath(path) => self.request(ctx, Pending::OpenPath(path)),
            Command::RemoveRecent(path) => {
                self.config.remove_recent(&path);
                self.save_config();
            }
            Command::Save => {
                self.stop_playback();
                self.save();
            }
            Command::SaveAs => {
                self.stop_playback();
                self.save_as();
            }
            Command::ExportPng => {
                self.stop_playback();
                self.export_png();
            }
            Command::ExportAllFrames => {
                self.config.export.frames = FrameSelection::All;
                self.config.export.tag = None;
                self.execute(ctx, Command::OpenExport(ExportKind::Image));
            }
            Command::OpenExport(kind) => {
                if let Some(editor) = &mut self.editor {
                    self.player.stop(editor);
                    editor.end_stroke();
                    self.export.open_at(kind, &mut self.config.export);
                }
            }
            Command::Copy => {
                if let Some(editor) = &mut self.editor {
                    if editor.copy_selection() {
                        // Ctrl+V only reaches Kagero when the system clipboard has text.
                        let size = editor.selection().map(|r| (r.w, r.h)).unwrap_or_default();
                        ctx.copy_text(format!("Kagero pixels {}×{}", size.0, size.1));
                        self.info("Copied");
                    } else {
                        self.info("Select an area first (M, then drag on the canvas)");
                    }
                }
            }
            Command::Cut => {
                if let Some(editor) = &mut self.editor {
                    self.player.stop(editor);
                    match editor.cut_selection() {
                        Ok(true) => {
                            ctx.copy_text("Kagero pixels".to_owned());
                            self.info("Cut");
                        }
                        Ok(false) => self.info("Select an area first (M, then drag on the canvas)"),
                        Err(e) => self.status_error(capitalize(&e.to_string())),
                    }
                }
            }
            Command::CopyHex(color) => {
                ctx.copy_text(color.to_hex());
                self.info(format!("Copied {}", color.to_hex()));
            }
            Command::UseColor(_) => {}
            Command::CloseDocument => {
                if self.editor.is_some() {
                    self.stop_playback();
                    self.request(ctx, Pending::CloseDocument);
                }
            }
            Command::Exit => self.request(ctx, Pending::Exit),
            Command::About => self.dialog = Some(Dialog::About),
            Command::ManagePlugins => {
                self.plugins.reload();
                self.settings
                    .open_at(crate::ui::settings::SettingsTab::Plugins);
            }
            Command::RunPlugin(plugin, command) => self.run_plugin(&plugin, &command),
            Command::OpenConvertPalette => {
                if let Some(editor) = &mut self.editor {
                    self.player.stop(editor);
                    editor.end_stroke();
                    self.dialog = Some(Dialog::ConvertPalette(
                        dialogs::ConvertPaletteState::default(),
                    ));
                }
            }
            Command::NewTag => {
                if let Some(editor) = &mut self.editor {
                    self.player.stop(editor);
                    let doc = editor.document();
                    let selected = self.timeline.selected_indices(doc);
                    let (start, end) = match (selected.first(), selected.last()) {
                        (Some(&a), Some(&b)) if selected.len() > 1 => (a, b),
                        _ => (doc.active_frame_index(), doc.active_frame_index()),
                    };
                    let tag = editor.suggest_tag(start, end);
                    let frame_count = editor.document().frame_count();
                    self.dialog = Some(Dialog::Tag(dialogs::TagState {
                        index: None,
                        tag,
                        frame_count,
                    }));
                }
            }
            Command::EditTag(i) => {
                if let Some(editor) = &mut self.editor {
                    self.player.stop(editor);
                    let doc = editor.document();
                    if let Some(tag) = doc.tags().get(i) {
                        self.dialog = Some(Dialog::Tag(dialogs::TagState {
                            index: Some(i),
                            tag: tag.clone(),
                            frame_count: doc.frame_count(),
                        }));
                    }
                }
            }
            Command::OpenRecovery => {
                let entries = self.recovery.orphans();
                if entries.is_empty() {
                    self.info("No autosaved copies to recover");
                } else {
                    self.dialog = Some(Dialog::Recovery(dialogs::RecoveryState {
                        entries,
                        errors: Default::default(),
                    }));
                }
            }
            Command::OpenCanvasSize => {
                if let Some(editor) = &self.editor {
                    let d = editor.document();
                    self.dialog = Some(Dialog::CanvasSize(dialogs::CanvasSizeState::new(
                        d.width(),
                        d.height(),
                    )));
                }
            }
            Command::Licenses => self.dialog = Some(Dialog::Licenses(0)),
            Command::ToggleGrid => {
                self.config.show_grid = !self.config.show_grid;
                self.schedule_config_save();
            }
            Command::ToggleTimeline => {
                self.config.ui.timeline_visible = !self.config.ui.timeline_visible;
                self.schedule_config_save();
            }
            Command::ToggleSidePanel => {
                self.config.ui.side_panel_visible = !self.config.ui.side_panel_visible;
                self.schedule_config_save();
            }
            Command::ToggleOnionSkin => {
                self.config.onion.enabled = !self.config.onion.enabled;
                self.schedule_config_save();
                let state = if self.config.onion.enabled {
                    "on"
                } else {
                    "off"
                };
                self.info(format!("Onion skin {state}"));
            }
            Command::SettingsChanged => self.schedule_config_save(),
            Command::OpenSettings(tab) => self.open_settings(tab),
            Command::CheckForUpdates => {
                self.open_settings(SettingsTab::Updates);
                self.updater.check(self.config.updates.include_prereleases);
            }
            Command::AddPaletteColor => {
                if let Some(color) = self.editor.as_ref().map(|e| e.primary) {
                    if self.config.add_palette_color(color) {
                        self.save_config();
                    } else {
                        self.info("The color is already in the palette (or the palette is full)");
                    }
                }
            }
            Command::RemovePaletteColor(index) => {
                if self.config.remove_palette_color(index) {
                    self.schedule_config_save();
                }
            }
            Command::MovePaletteColor(index, delta) => {
                if self.config.move_palette_color(index, delta) {
                    self.schedule_config_save();
                }
            }
            Command::ResetPalette => {
                self.config.restore_builtin_palettes();
                self.schedule_config_save();
                self.info("Built-in palettes restored");
            }
            Command::SelectPalette(index) => {
                if index < self.config.palettes.len() {
                    self.config.active_palette = index;
                    self.schedule_config_save();
                }
            }
            Command::NewPalette => {
                let first = self
                    .editor
                    .as_ref()
                    .map_or(crate::core::Rgba::BLACK, |e| e.primary);
                let palette = Palette {
                    name: "New Palette".to_owned(),
                    colors: vec![first],
                };
                if self.config.add_palette(palette) {
                    self.schedule_config_save();
                }
            }
            Command::DuplicatePalette => {
                let mut copy = self.config.active_palette().clone();
                copy.name = format!("{} copy", copy.name);
                if self.config.add_palette(copy) {
                    self.schedule_config_save();
                }
            }
            Command::DeletePalette => {
                if self.config.delete_active_palette() {
                    self.schedule_config_save();
                }
            }
            Command::RenamePalette(name) => {
                let name = palettes::sanitize_name(&name);
                if name != self.config.active_palette().name {
                    let unique = self.config.unique_palette_name(&name);
                    self.config.active_palette_mut().name = unique;
                    self.schedule_config_save();
                }
            }
            Command::ImportPalette => self.import_palette(),
            Command::ExportPalette => self.export_palette(),
            Command::DeleteFrame(index) => {
                let has_content = self
                    .editor
                    .as_ref()
                    .is_some_and(|e| e.frame_has_content(index));
                if has_content {
                    self.stop_playback();
                    self.dialog = Some(Dialog::ConfirmDeleteFrame(index));
                } else {
                    self.execute_on_document(Command::DeleteFrameConfirmed(index));
                }
            }
            other => self.execute_on_document(other),
        }
    }

    fn import_palette(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import Palette")
            .add_filter("Palettes", &["gpl", "hex", "txt"])
            .pick_file()
        else {
            return;
        };
        match palette_io::load_palette(&path) {
            Ok(palette) => {
                let name = palette.name.clone();
                if self.config.add_palette(palette) {
                    self.schedule_config_save();
                    self.success(format!("Imported palette “{name}”"));
                } else {
                    self.status_error("Too many palettes — delete one first");
                }
            }
            Err(e) => self.error_dialog("The palette was not imported", &e),
        }
    }

    fn export_palette(&mut self) {
        let palette = self.config.active_palette().clone();
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Palette")
            .add_filter("GIMP palette", &["gpl"])
            .add_filter("HEX list", &["hex"])
            .set_file_name(format!(
                "{}.gpl",
                crate::export::sequence::sanitize_stem(&palette.name)
            ))
            .save_file()
        else {
            return;
        };
        match palette_io::save_palette(&palette, &path) {
            Ok(()) => self.success(format!("Saved {}", path.display())),
            Err(e) => self.error_dialog("The palette was not saved", &e),
        }
    }

    fn open_settings(&mut self, tab: SettingsTab) {
        self.has_backup = update_installer::install_dir()
            .map(|d| update_installer::has_backup(&d))
            .unwrap_or(false);
        self.settings.open_at(tab);
    }

    /// Commands that need an open document; ignored when there is none.
    fn execute_on_document(&mut self, command: Command) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        // Any edit or frame navigation ends the preview first.
        let keeps_playing = matches!(
            command,
            Command::TogglePlayback
                | Command::SetTool(_)
                | Command::SwapColors
                | Command::CenterCanvas
                | Command::FitCanvas
                | Command::ZoomIn
                | Command::ZoomOut
                | Command::ZoomActual
        );
        if !keeps_playing {
            self.player.stop(editor);
        }
        let active = editor.document().active_index();
        let result = match command {
            Command::Undo => match editor.undo() {
                Ok(Some(label)) => {
                    self.info(format!("Undo: {label}"));
                    Ok(())
                }
                Ok(None) => Ok(()),
                Err(e) => Err(e),
            },
            Command::Redo => match editor.redo() {
                Ok(Some(label)) => {
                    self.info(format!("Redo: {label}"));
                    Ok(())
                }
                Ok(None) => Ok(()),
                Err(e) => Err(e),
            },
            Command::SetTool(tool) => {
                editor.set_tool(tool);
                Ok(())
            }
            Command::SwapColors => {
                editor.swap_colors();
                Ok(())
            }
            Command::CenterCanvas => {
                self.canvas.center();
                Ok(())
            }
            Command::FitCanvas => {
                self.canvas.request_fit();
                Ok(())
            }
            Command::ZoomIn => {
                self.canvas.zoom_in();
                Ok(())
            }
            Command::ZoomOut => {
                self.canvas.zoom_out();
                Ok(())
            }
            Command::ZoomActual => {
                self.canvas.zoom_actual();
                Ok(())
            }
            Command::AddLayer => editor.add_layer(),
            Command::DeleteLayer => editor.delete_layer(active),
            Command::LayerUp => editor.move_layer_up(active),
            Command::LayerDown => editor.move_layer_down(active),
            Command::SelectLayer(i) => {
                editor.select_layer(i);
                Ok(())
            }
            Command::ToggleLayerVisibility(i) => editor.toggle_layer_visibility(i),
            Command::BeginRenameLayer(i) => {
                if let Some(layer) = editor.document().layers().get(i) {
                    self.rename = Some((i, layer.name.clone()));
                }
                Ok(())
            }
            Command::RenameLayer(i, name) => editor.rename_layer(i, &name),
            Command::TogglePlayback => {
                if self.player.is_playing() {
                    self.player.stop(editor);
                } else if let Some(range) = self.timeline.loop_range(editor.document()) {
                    self.player
                        .start(editor, &self.config.playback, Some(range));
                } else if editor.document().frame_count() > 1 {
                    self.player.start(editor, &self.config.playback, None);
                } else {
                    self.info("Add a second frame to preview an animation");
                }
                Ok(())
            }
            Command::StopPlayback => Ok(()),
            Command::FirstFrame => {
                editor.select_frame(0);
                Ok(())
            }
            Command::PrevFrame => {
                editor.previous_frame();
                Ok(())
            }
            Command::NextFrame => {
                editor.next_frame();
                Ok(())
            }
            Command::SelectFrame(i) => {
                editor.select_frame(i);
                Ok(())
            }
            Command::AddFrame => editor.add_frame(),
            Command::DuplicateFrame => editor.duplicate_frame(),
            Command::DeleteFrameConfirmed(i) => editor.delete_frame(i),
            Command::MoveFrameLeft(i) => editor.move_frame_left(i),
            Command::MoveFrameRight(i) => editor.move_frame_right(i),
            Command::SetFrameDuration(i, ms) => editor.set_frame_duration(i, ms),
            Command::InsertTag(tag) => editor.insert_tag(tag).map(|i| {
                let doc = editor.document();
                if let Some(t) = doc.tags().get(i) {
                    self.timeline.select_range(doc, t.frames(doc.frame_count()));
                }
            }),
            Command::UpdateTag(i, tag) => editor.update_tag(i, tag),
            Command::DeleteTag(i) => editor.remove_tag(i),
            Command::PrevTag | Command::NextTag => {
                let doc = editor.document();
                let active = doc.active_frame_index();
                let mut starts: Vec<usize> = doc.tags().iter().map(|t| t.start).collect();
                starts.sort_unstable();
                starts.dedup();
                let target = if command == Command::NextTag {
                    starts.into_iter().find(|&s| s > active)
                } else {
                    // The start of the tag we're in counts as "previous" only when we're
                    // past it; otherwise go to the one before.
                    starts.into_iter().rev().find(|&s| s < active)
                };
                match target {
                    Some(s) => {
                        editor.select_frame(s);
                        Ok(())
                    }
                    None => {
                        self.info(if command == Command::NextTag {
                            "No tag after this frame"
                        } else {
                            "No tag before this frame"
                        });
                        Ok(())
                    }
                }
            }
            Command::DuplicateLayer => editor.duplicate_layer(editor.document().active_index()),
            Command::MergeDown => editor.merge_down(editor.document().active_index()),
            Command::SetLayerProps(i, props) => editor.set_layer_props(i, props),
            Command::PreviewLayerOpacity(i, v) => {
                editor.preview_layer_opacity(i, v);
                Ok(())
            }
            Command::CommitLayerOpacity(i, before) => editor.commit_layer_opacity(i, before),
            Command::ResizeCanvas(w, h, anchor) => {
                let r = editor.resize_canvas(w, h, anchor);
                if r.is_ok() {
                    self.canvas.request_fit();
                }
                r
            }
            Command::ApplyCrop => match editor.crop_rect.take() {
                Some(c) => {
                    let r = editor.crop(c.x.max(0) as u32, c.y.max(0) as u32, c.w, c.h);
                    if r.is_ok() {
                        self.canvas.request_fit();
                    }
                    r
                }
                None => Ok(()),
            },
            Command::CancelCrop => {
                editor.crop_rect = None;
                Ok(())
            }
            Command::CropToSelection => match editor.selection() {
                Some(s) => {
                    let r = editor.crop(s.x.max(0) as u32, s.y.max(0) as u32, s.w, s.h);
                    if r.is_ok() {
                        editor.crop_rect = None;
                        self.canvas.request_fit();
                    }
                    r
                }
                None => Err(crate::editor::EditError::NoSelection),
            },
            Command::SelectAll => {
                editor.set_tool(crate::editor::ToolKind::Select);
                editor.select_all();
                Ok(())
            }
            Command::Deselect => {
                editor.deselect();
                Ok(())
            }
            Command::FinishPolygon => {
                editor.end_stroke();
                Ok(())
            }
            Command::BrushFromSelection => {
                let r = editor.brush_from_selection();
                if r.is_ok() {
                    if !editor.tool.uses_brush_size() {
                        editor.set_tool(crate::editor::ToolKind::Pencil);
                    }
                    self.info("The selection is now the brush (Brush: Custom)");
                }
                r
            }
            Command::ConvertToPalette(scope) => {
                let colors = self.config.active_palette().colors.clone();
                editor.convert_to_palette(&colors, scope).map(|n| {
                    self.info(if n == 0 {
                        "Every color is already in the palette".to_owned()
                    } else {
                        format!("Converted {n} pixels to the palette")
                    });
                })
            }
            Command::InvertSelection => {
                editor.invert_selection();
                if !editor.tool.selects() {
                    editor.set_tool(crate::editor::ToolKind::Select);
                }
                Ok(())
            }
            Command::SetSelection(rect) => {
                editor.set_selection(rect);
                Ok(())
            }
            Command::MoveSelectionOutline(dx, dy) => {
                editor.move_selection_outline(dx, dy);
                Ok(())
            }
            Command::Paste => {
                if editor.clipboard.is_none() {
                    self.info("Nothing to paste — copy a selection first");
                    return;
                }
                editor.set_tool(crate::editor::ToolKind::Select);
                editor.paste().map(|_| ())
            }
            Command::DeleteSelection => editor.delete_selection().map(|_| ()),
            Command::DuplicateSelection => editor.duplicate_selection().map(|_| ()),
            Command::MoveSelection(dx, dy) => editor.move_selection(dx, dy).map(|_| ()),
            Command::Transform(kind) => editor.transform(kind).map(|_| ()),
            _ => Ok(()),
        };
        if let Err(e) = result {
            self.status_error(capitalize(&e.to_string()));
        }
    }

    fn execute_settings_action(&mut self, ctx: &egui::Context, action: SettingsAction) {
        match action {
            SettingsAction::Save => self.schedule_config_save(),
            SettingsAction::CheckUpdates => {
                self.updater.check(self.config.updates.include_prereleases)
            }
            SettingsAction::DownloadUpdate => self.updater.download(),
            SettingsAction::CancelDownload => self.updater.cancel(),
            SettingsAction::DismissUpdateError => self.updater.dismiss_error(),
            SettingsAction::InstallUpdate => {
                self.stop_playback();
                self.request(ctx, Pending::InstallUpdate);
            }
            SettingsAction::RestorePrevious => {
                self.stop_playback();
                self.request(ctx, Pending::RestorePrevious);
            }
            SettingsAction::DiscordRetry => self.discord.retry(),
            SettingsAction::PreviewTheme(name) => {
                if name == self.config.theme {
                    self.theme_preview = None;
                } else {
                    self.theme_preview = Some(name.clone());
                }
                self.show_theme(ctx, &name);
            }
            SettingsAction::KeepTheme => {
                if let Some(name) = self.theme_preview.take() {
                    self.config.theme = name;
                    self.schedule_config_save();
                }
            }
            SettingsAction::RevertTheme => {
                self.theme_preview = None;
                let saved = self.config.theme.clone();
                self.show_theme(ctx, &saved);
            }
            SettingsAction::ImportTheme => {
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Import Theme")
                    .add_filter("Kagero theme", &["json"])
                    .pick_file()
                else {
                    return;
                };
                match self.themes.import(&path) {
                    Ok((theme, replaced)) => {
                        self.success(if replaced {
                            format!("Updated the theme \"{}\"", theme.name)
                        } else {
                            format!("Imported the theme \"{}\" — previewing it", theme.name)
                        });
                        self.execute_settings_action(ctx, SettingsAction::PreviewTheme(theme.name));
                    }
                    Err(e) => self.status_error(format!("Couldn't import the theme: {e}")),
                }
            }
            SettingsAction::ExportTheme(name) => {
                let Some(theme) = self.themes.find(&name) else {
                    return;
                };
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Export Theme")
                    .add_filter("Kagero theme", &["json"])
                    .set_file_name(format!("{name}.json"))
                    .save_file()
                else {
                    return;
                };
                let path = crate::services::dialogs::with_extension(path, "json");
                let json = crate::themes::to_json(&theme);
                let written = crate::file_io::write_atomic(&path, |w| {
                    std::io::Write::write_all(w, json.as_bytes())?;
                    Ok(())
                });
                match written {
                    Ok(()) => self.success(format!("Exported the theme to {}", path.display())),
                    Err(e) => self.status_error(format!("Couldn't export the theme: {e}")),
                }
            }
            SettingsAction::DeleteTheme(name) => match self.themes.delete(&name) {
                Ok(()) => {
                    if self.config.theme == name {
                        self.config.theme = crate::themes::DEFAULT_THEME.to_owned();
                        self.schedule_config_save();
                    }
                    if self.theme_preview.as_deref() == Some(name.as_str())
                        || self.theme_preview.is_none()
                    {
                        self.theme_preview = None;
                        let saved = self.config.theme.clone();
                        self.show_theme(ctx, &saved);
                    }
                    self.info(format!("Deleted the theme \"{name}\""));
                }
                Err(e) => self.status_error(format!("Couldn't delete the theme: {e}")),
            },
            SettingsAction::OpenThemesFolder => {
                if let Some(dir) = self.themes.dir() {
                    let _ = std::fs::create_dir_all(dir);
                    crate::ui::export_dialog::open_folder(dir);
                }
            }
            SettingsAction::ReloadThemes => {
                self.themes.reload();
                let shown = self
                    .theme_preview
                    .clone()
                    .unwrap_or_else(|| self.config.theme.clone());
                self.show_theme(ctx, &shown);
            }
            SettingsAction::OpenPluginsFolder => {
                if let Some(dir) = self.plugins.dir() {
                    let _ = std::fs::create_dir_all(dir);
                    crate::ui::export_dialog::open_folder(dir);
                }
            }
            SettingsAction::ReloadPlugins => self.plugins.reload(),
            SettingsAction::InstallExamplePlugin => match self.plugins.install_example() {
                Ok(_) => self.success("Installed \"Example Tools\" — turn it on below"),
                Err(e) => self.status_error(format!("Couldn't install the example: {e}")),
            },
            SettingsAction::OpenRecoveryFolder => {
                crate::ui::export_dialog::open_folder(self.recovery.dir());
            }
            SettingsAction::DiscordDisconnect => self
                .discord
                .disconnect(self.config.discord.clear_on_disable),
        }
    }

    // ------------------------------------------------------------ services

    fn poll_updates(&mut self) {
        for event in self.updater.take_events() {
            match event {
                UpdateEvent::CheckFinished { found } => {
                    self.config.updates.last_check_unix = Some(updater::unix_now());
                    self.schedule_config_save();
                    if let Some(version) = found {
                        let new = self.notified_version.as_ref() != Some(&version);
                        if new && self.config.updates.notify && !self.settings.open {
                            self.info(format!(
                                "Kagero {version} is available — Help › Check for Updates"
                            ));
                        }
                        self.notified_version = Some(version);
                    }
                }
                UpdateEvent::CheckFailed(_) => {}
                UpdateEvent::DownloadFinished => {
                    if !self.settings.open {
                        self.success("Update downloaded — see Settings › Updates");
                    }
                }
            }
        }
        // Automatic checks: once at startup, then by interval, never while busy.
        let now = Instant::now();
        let first = self.last_auto_check_poll.is_none();
        if !first
            && self
                .last_auto_check_poll
                .is_some_and(|t| now.duration_since(t) < AUTO_CHECK_POLL)
        {
            return;
        }
        self.last_auto_check_poll = Some(now);
        let u = &self.config.updates;
        let due = (first && u.check_on_startup)
            || (u.periodic_check
                && updater::check_due(u.last_check_unix, u.interval_hours, updater::unix_now()));
        if due
            && !self.updater.is_busy()
            && matches!(
                self.updater.status(),
                UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Failed(_)
            )
        {
            self.updater.check(u.include_prereleases);
        }
    }

    fn update_presence(&mut self) {
        let mode = match &self.editor {
            None => AppMode::Home,
            Some(_) if self.export.open => AppMode::Exporting,
            Some(_) if self.player.is_playing() => AppMode::Previewing,
            Some(e) if e.document().frame_count() > 1 => AppMode::AnimationDocument,
            Some(_) => AppMode::StaticDocument,
        };
        let name = self.editor.as_ref().map(|e| e.name().to_owned());
        let desired = build_presence(
            mode,
            name.as_deref(),
            self.session_start_unix,
            &self.config.discord,
        );
        self.discord
            .update(desired, Flags::from(&self.config.discord));
    }

    // ------------------------------------------------------------ input

    fn collect_shortcuts(&mut self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        let keys = self
            .editor
            .as_ref()
            .map_or(KeyContext::default(), |e| KeyContext {
                active_layer: e.document().active_index(),
                has_selection: e.selection().is_some(),
                cropping: e.crop_rect.is_some(),
                polygon: e.is_drawing_polygon(),
            });
        let keymap = crate::ui::keymap::Keymap::from_overrides(&self.config.shortcuts);
        let (found, escape) = shortcuts::collect(ctx, &keymap, keys);
        commands.extend(found);
        if escape {
            if let Some((job, _, _)) = &self.plugin_job {
                job.cancel();
                self.info("Stopping the plugin…");
            } else if self.player.is_playing() {
                self.stop_playback();
            } else if let Some(editor) = &mut self.editor {
                if editor.is_stroking() {
                    editor.cancel_stroke();
                    self.info("Stroke canceled");
                } else if editor.crop_rect.is_some() {
                    editor.crop_rect = None;
                } else if editor.selection().is_some() {
                    editor.deselect();
                }
            }
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.allow_close {
            return;
        }
        if self.editor.as_ref().is_some_and(Editor::is_modified) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if !matches!(self.dialog, Some(Dialog::Unsaved(_))) {
                self.dialog = Some(Dialog::Unsaved(Pending::Exit));
            }
        } else {
            self.save_config();
        }
    }

    fn show_dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        match dialog {
            Dialog::NewDocument(mut state) => match dialogs::new_document(ctx, &mut state) {
                NewDocumentResult::Open => self.dialog = Some(Dialog::NewDocument(state)),
                NewDocumentResult::Create => self.create_document(&state),
                NewDocumentResult::Cancel => {}
            },
            Dialog::Unsaved(pending) => {
                let name = self
                    .editor
                    .as_ref()
                    .map_or_else(|| "Untitled".to_owned(), |e| e.name().to_owned());
                match dialogs::unsaved_changes(ctx, &name) {
                    UnsavedChoice::Pending => self.dialog = Some(Dialog::Unsaved(pending)),
                    UnsavedChoice::Save => {
                        if self.save() {
                            self.perform(ctx, pending);
                        }
                    }
                    UnsavedChoice::Discard => self.perform(ctx, pending),
                    UnsavedChoice::Cancel => {}
                }
            }
            Dialog::Error { title, message } => {
                if !dialogs::error(ctx, &title, &message) {
                    self.dialog = Some(Dialog::Error { title, message });
                }
            }
            Dialog::ConvertPalette(mut state) => {
                let palette = self.config.active_palette().clone();
                let result = match &self.editor {
                    Some(editor) => dialogs::convert_palette(ctx, &mut state, editor, &palette),
                    None => dialogs::ConvertPaletteResult::Cancel,
                };
                match result {
                    dialogs::ConvertPaletteResult::Open => {
                        self.dialog = Some(Dialog::ConvertPalette(state));
                    }
                    dialogs::ConvertPaletteResult::Cancel => {}
                    dialogs::ConvertPaletteResult::Apply => {
                        self.execute_on_document(Command::ConvertToPalette(state.scope));
                    }
                }
            }
            Dialog::Tag(mut state) => match dialogs::frame_tag(ctx, &mut state) {
                dialogs::TagResult::Open => self.dialog = Some(Dialog::Tag(state)),
                dialogs::TagResult::Cancel => {}
                dialogs::TagResult::Apply => {
                    let command = match state.index {
                        Some(i) => Command::UpdateTag(i, state.tag),
                        None => Command::InsertTag(state.tag),
                    };
                    self.execute_on_document(command);
                }
                dialogs::TagResult::Delete => {
                    if let Some(i) = state.index {
                        self.execute_on_document(Command::DeleteTag(i));
                    }
                }
            },
            Dialog::Recovery(mut state) => match dialogs::recovery(ctx, &state) {
                dialogs::RecoveryChoice::Keep => self.dialog = Some(Dialog::Recovery(state)),
                dialogs::RecoveryChoice::Close => {}
                dialogs::RecoveryChoice::Delete(i) => {
                    let entry = state.entries.remove(i);
                    crate::services::recovery::delete(&entry);
                    if !state.entries.is_empty() {
                        self.dialog = Some(Dialog::Recovery(state));
                    }
                }
                dialogs::RecoveryChoice::OpenOriginal(i) => {
                    if self.editor.as_ref().is_some_and(Editor::is_modified) {
                        self.status_error("Save or close the current image first");
                        self.dialog = Some(Dialog::Recovery(state));
                    } else if let Some(path) = state.entries[i].original.clone() {
                        self.open_path(&path);
                        self.dialog = Some(Dialog::Recovery(state));
                    }
                }
                dialogs::RecoveryChoice::OpenRecovered(i) => {
                    if self.editor.as_ref().is_some_and(Editor::is_modified) {
                        self.status_error("Save or close the current image first");
                        self.dialog = Some(Dialog::Recovery(state));
                    } else {
                        let entry = state.entries[i].clone();
                        match entry.load() {
                            Ok(doc) => {
                                self.set_document(doc, None, format!("{} (recovered)", entry.name));
                                if let Some(editor) = &mut self.editor {
                                    editor.mark_unsaved();
                                }
                                self.recovery.adopt(entry);
                                self.success("Recovered — save it to keep it");
                                state.entries.remove(i);
                                if !state.entries.is_empty() {
                                    self.dialog = Some(Dialog::Recovery(state));
                                }
                            }
                            Err(e) => {
                                tracing::warn!("recovery copy {} unreadable: {e}", entry.id);
                                state.errors.insert(entry.id.clone(), e.to_string());
                                self.dialog = Some(Dialog::Recovery(state));
                            }
                        }
                    }
                }
            },
            Dialog::CanvasSize(mut state) => match dialogs::canvas_size(ctx, &mut state) {
                dialogs::CanvasSizeResult::Open => self.dialog = Some(Dialog::CanvasSize(state)),
                dialogs::CanvasSizeResult::Cancel => {}
                dialogs::CanvasSizeResult::Apply => self.execute_on_document(
                    Command::ResizeCanvas(state.width, state.height, state.anchor),
                ),
            },
            Dialog::About => match dialogs::about(ctx) {
                dialogs::AboutChoice::Keep => self.dialog = Some(Dialog::About),
                dialogs::AboutChoice::Licenses => self.dialog = Some(Dialog::Licenses(0)),
                dialogs::AboutChoice::Close => {}
            },
            Dialog::Licenses(mut tab) => {
                if dialogs::licenses(ctx, &mut tab) {
                    self.dialog = Some(Dialog::Licenses(tab));
                }
            }
            Dialog::ConfirmDeleteFrame(index) => {
                let message = format!(
                    "Frame {} has drawings on it. You can bring it back with Undo.",
                    index + 1
                );
                match dialogs::confirm(ctx, "Delete this frame?", &message, "Delete Frame") {
                    ConfirmChoice::Pending => self.dialog = Some(Dialog::ConfirmDeleteFrame(index)),
                    ConfirmChoice::Confirm => {
                        self.execute_on_document(Command::DeleteFrameConfirmed(index))
                    }
                    ConfirmChoice::Cancel => {}
                }
            }
        }
    }

    /// Starts a plugin command on the active layer in the background.
    fn run_plugin(&mut self, plugin_id: &str, command: &str) {
        if self.plugin_job.is_some() {
            self.info("A plugin is still running — wait or press Esc to stop it");
            return;
        }
        let Some(editor) = &mut self.editor else {
            return;
        };
        self.player.stop(editor);
        editor.end_stroke();
        let enabled = self.config.plugins.enabled.iter().any(|e| e == plugin_id);
        let Some(plugin) = self.plugins.find(plugin_id).filter(|_| enabled) else {
            self.status_error("That plugin is turned off or no longer installed");
            return;
        };
        let doc = editor.document();
        let input = crate::plugins::PluginInput {
            width: doc.width(),
            height: doc.height(),
            pixels: doc.active_cel().as_bytes().to_vec(),
            selection: editor.selection_shape().cloned(),
            primary: editor.primary,
            secondary: editor.secondary,
        };
        let target = (doc.active_frame().id, doc.active_layer().id);
        match crate::plugins::PluginJob::spawn(plugin, command, input, editor.history().state_id())
        {
            Ok(job) => {
                self.info(format!("Running {}… (Esc stops it)", job.title));
                self.plugin_job = Some((job, target.0, target.1));
            }
            Err(e) => self.status_error(format!("Couldn't start the plugin: {e}")),
        }
    }

    /// Applies a finished plugin command (one undo step), if the image didn't change.
    fn poll_plugin(&mut self, ctx: &egui::Context) {
        let Some((job, frame, layer)) = &self.plugin_job else {
            return;
        };
        let Some(result) = job.poll() else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        };
        let (job, frame, layer) = (job.title.clone(), *frame, *layer);
        let state_id = self.plugin_job.as_ref().map(|j| j.0.state_id);
        self.plugin_job = None;
        let output = match result {
            Ok(o) => o,
            Err(e) => {
                self.status_error(format!("{job}: {e}"));
                return;
            }
        };
        let Some(editor) = &mut self.editor else {
            return;
        };
        let doc = editor.document();
        let same = Some(editor.history().state_id()) == state_id
            && doc.active_frame().id == frame
            && doc.active_layer().id == layer;
        if !same {
            self.status_error(format!(
                "{job}: the image changed while it ran, so nothing was applied"
            ));
            return;
        }
        if let Some(c) = output.primary {
            editor.primary = c;
        }
        let applied = match output.pixels {
            Some(px) => editor.replace_active_pixels("Plugin", px),
            None => Ok(false),
        };
        let note = output.messages.join(" · ");
        match applied {
            Ok(true) if note.is_empty() => self.success(format!("{job}: done")),
            Ok(_) if !note.is_empty() => self.success(format!("{job}: {note}")),
            Ok(_) => self.info(format!("{job}: nothing changed")),
            Err(e) => self.status_error(format!("{job}: {e}")),
        }
    }

    /// Shows a theme on the whole window (the default theme if it no longer exists).
    fn show_theme(&mut self, ctx: &egui::Context, name: &str) {
        let colors = self
            .themes
            .find(name)
            .map_or(crate::theme::EMBER, |t| t.colors);
        crate::theme::set_colors(colors);
        theme::apply(ctx);
    }

    /// Writes a recovery copy of a document with unsaved changes every few minutes.
    fn autosave(&mut self, ctx: &egui::Context) {
        if let Some(crate::services::recovery::WriteResult::Failed(e)) = self.recovery.poll() {
            self.status_error(format!("Autosave failed: {e}"));
        }
        let settings = self.config.autosave;
        let Some(editor) = &self.editor else {
            return;
        };
        if !settings.enabled || !editor.is_modified() || editor.is_stroking() {
            return;
        }
        let state = editor.history().state_id();
        if !self.recovery.needs_copy(state) || self.recovery.is_writing() {
            return;
        }
        let interval = Duration::from_secs(u64::from(settings.interval_minutes) * 60);
        let elapsed = self.last_autosave.elapsed();
        if elapsed < interval {
            // Come back when it's due, even if nothing else happens.
            ctx.request_repaint_after(interval - elapsed);
            return;
        }
        let doc = editor.document().clone();
        let name = editor.name().to_owned();
        let original = editor.path().map(Path::to_path_buf);
        if self.recovery.write(doc, state, &name, original.as_deref()) {
            self.last_autosave = Instant::now();
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    /// Remembers whether the user maximized or restored the window, so the next
    /// start looks the same. Only changes after the first frame count: while the
    /// window opens, the OS may report "not maximized" before maximizing it.
    fn remember_window_state(&mut self, ctx: &egui::Context) {
        if !self.window_restored {
            // `with_maximized` at window creation isn't always honored on Windows,
            // so ask again once the window exists.
            self.window_restored = true;
            if self.config.ui.window_maximized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            return;
        }
        let (maximized, minimized) =
            ctx.input(|i| (i.viewport().maximized, i.viewport().minimized));
        let Some(maximized) = maximized else {
            return;
        };
        if minimized == Some(true) {
            return;
        }
        let previous = self.seen_maximized.replace(maximized);
        if previous.is_some_and(|p| p != maximized) && maximized != self.config.ui.window_maximized
        {
            self.config.ui.window_maximized = maximized;
            self.schedule_config_save();
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match &self.editor {
            Some(e) => format!(
                "{}{} — Kagero",
                e.name(),
                if e.is_modified() { " •" } else { "" }
            ),
            None => "Kagero".to_owned(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    fn panels(&mut self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        let bar = |fill| {
            Frame::new()
                .fill(fill)
                .inner_margin(Margin::symmetric(8, 4))
        };
        let playing = self.player.is_playing();
        let keymap = crate::ui::keymap::Keymap::from_overrides(&self.config.shortcuts);

        crate::ui::title_bar::resize_edges(ctx);
        let title = self.window_title.clone();
        egui::TopBottomPanel::top("title-bar")
            .frame(Frame::new().fill(theme::background()))
            .exact_height(crate::ui::title_bar::HEIGHT)
            .show_separator_line(false)
            .show(ctx, |ui| {
                let plugin_commands: Vec<(String, String, String)> = self
                    .plugins
                    .plugins
                    .iter()
                    .filter(|p| p.is_valid() && self.config.plugins.enabled.contains(&p.id()))
                    .filter_map(|p| p.manifest.as_ref())
                    .flat_map(|m| {
                        m.commands.iter().map(move |c| {
                            (
                                m.id.clone(),
                                c.id.clone(),
                                format!("{}: {}", m.name, c.title),
                            )
                        })
                    })
                    .collect();
                let state = MenuState {
                    editor: self.editor.as_ref(),
                    recent: &self.config.recent_files,
                    show_grid: self.config.show_grid,
                    playing,
                    onion: self.config.onion.enabled,
                    timeline_visible: self.config.ui.timeline_visible,
                    side_panel_visible: self.config.ui.side_panel_visible,
                    keymap: &keymap,
                    plugin_commands: &plugin_commands,
                    plugin_running: self.plugin_job.is_some(),
                };
                crate::ui::title_bar::show(ui, &title, |ui| menu::show(ui, &state, commands));
            });

        if let Some(editor) = &mut self.editor {
            egui::TopBottomPanel::top("tool-options")
                .frame(bar(theme::panel()))
                .exact_height(36.0)
                .show(ctx, |ui| {
                    tool_options::show(ui, editor, self.config.show_grid, commands);
                });
        }

        egui::TopBottomPanel::bottom("status-bar")
            .frame(bar(theme::background()))
            .exact_height(26.0)
            .show(ctx, |ui| {
                status::show(
                    ui,
                    self.editor.as_ref(),
                    self.canvas.hover_pixel,
                    self.canvas.zoom,
                    playing,
                    self.status.as_ref(),
                );
            });

        if let Some(editor) = &self.editor {
            let expanded = self.config.ui.timeline_visible;
            let frame = Frame::new()
                .fill(theme::panel())
                .inner_margin(Margin::symmetric(10, 3));
            let panel = if expanded {
                egui::TopBottomPanel::bottom("timeline-expanded")
                    .resizable(true)
                    .default_height(self.config.ui.timeline_height)
                    .height_range(TIMELINE_MIN_HEIGHT..=TIMELINE_MAX_HEIGHT)
            } else {
                egui::TopBottomPanel::bottom("timeline-collapsed")
                    .resizable(false)
                    .exact_height(HEADER_HEIGHT)
            };
            let response = panel.frame(frame).show(ctx, |ui| {
                let settings = TimelineSettings {
                    onion: &mut self.config.onion,
                    playback: &mut self.config.playback,
                    expanded,
                };
                self.timeline.show(ui, editor, playing, settings, commands);
            });
            if expanded {
                let height = response.response.rect.height();
                if (height - self.config.ui.timeline_height).abs() > 0.5 {
                    self.config.ui.timeline_height = height;
                    self.config_save_at
                        .get_or_insert(Instant::now() + CONFIG_SAVE_DELAY);
                }
            }
        }

        if let Some(editor) = &mut self.editor {
            egui::SidePanel::left("toolbar")
                .frame(
                    Frame::new()
                        .fill(theme::panel())
                        .inner_margin(Margin::same(6)),
                )
                .resizable(false)
                .exact_width(toolbar::panel_width(ctx))
                .show(ctx, |ui| toolbar::show(ui, editor.tool, &keymap, commands));

            if self.config.ui.side_panel_visible {
                egui::SidePanel::right("side-panel")
                    .frame(
                        Frame::new()
                            .fill(theme::panel())
                            .inner_margin(Margin::same(10)),
                    )
                    .resizable(true)
                    .default_width(250.0)
                    .width_range(210.0..=420.0)
                    .show(ctx, |ui| {
                        palette::show(ui, editor, &self.config, &mut self.palette_rename, commands);
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        layers::show(ui, editor, &mut self.rename, commands);
                    });
            }
        }

        let mut canvas_error = None;
        let options = CanvasOptions {
            onion: self.config.onion,
            playing,
            reduce_motion: self.config.ui.reduce_motion,
            tiled: self.config.experimental.tiled_view,
        };
        if let Some(editor) = &mut self.editor {
            editor.settings.pixel_perfect = self.config.experimental.pixel_perfect;
        }
        self.canvas.show_grid = self.config.show_grid;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(theme::background()))
            .show(ctx, |ui| match &mut self.editor {
                Some(editor) => canvas_error = self.canvas.show(ui, editor, options),
                None => home::show(ui, &self.config.recent_files, commands),
            });
        if let Some(e) = canvas_error {
            self.status_error(capitalize(&e.to_string()));
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

impl eframe::App for KageroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::sync_fonts(ctx, self.config.experimental.pixel_font);
        self.remember_window_state(ctx);
        self.handle_close_request(ctx);
        self.poll_updates();

        if let Some(editor) = &mut self.editor {
            self.player.tick(ctx, editor, &self.config.playback);
        }

        let mut commands = Vec::new();
        // No shortcuts while a dialog is open or a shortcut waits for its new key.
        if self.dialog.is_none() && !self.settings.is_capturing() {
            self.collect_shortcuts(ctx, &mut commands);
        }
        self.panels(ctx, &mut commands);

        let view = SettingsView {
            update_status: self.updater.status(),
            discord_state: self.discord.state(),
            discord_configured: self.discord.is_configured(),
            can_install: cfg!(windows)
                && release::is_official_build()
                && !release::UPDATE_PUBLIC_KEY.is_empty(),
            has_backup: self.has_backup,
            themes: self.themes.all(),
            current_theme: self.config.theme.clone(),
            theme_preview: self.theme_preview.clone(),
            theme_problems: self.themes.problems.clone(),
            plugins: self.plugins.plugins.clone(),
        };
        for action in self.settings.show(ctx, &mut self.config, &view) {
            self.execute_settings_action(ctx, action);
        }
        if !self.settings.open && self.theme_preview.is_some() {
            // Settings closed without keeping the previewed theme.
            self.execute_settings_action(ctx, SettingsAction::RevertTheme);
        }

        match &self.editor {
            Some(editor) => {
                let selected = self.timeline.selected_indices(editor.document());
                let action = self
                    .export
                    .show(ctx, editor, &selected, &mut self.config.export);
                if action == ExportDialogAction::SettingsChanged {
                    self.schedule_config_save();
                }
            }
            None => self.export.open = false,
        }
        self.show_dialogs(ctx);
        for command in commands {
            self.execute(ctx, command);
        }
        if let Some(color) = self.editor.as_mut().and_then(Editor::take_used_color) {
            if self.config.remember_color(color) {
                self.schedule_config_save();
            }
        }
        self.update_presence();
        self.update_title(ctx);
        self.autosave(ctx);
        self.poll_plugin(ctx);

        if let Some(at) = self.config_save_at {
            let now = Instant::now();
            if now >= at {
                self.save_config();
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
        if self
            .status
            .as_ref()
            .is_some_and(|m| m.kind != MessageKind::Error && m.is_visible())
        {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        // Keep connection states and automatic checks fresh without busy-looping.
        ctx.request_repaint_after(if self.settings.open {
            Duration::from_millis(500)
        } else {
            AUTO_CHECK_POLL
        });
    }
}

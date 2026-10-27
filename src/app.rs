//! The eframe application: screens, panels, dialogs and command dispatch.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Frame, Margin, ViewportCommand};

use crate::config::{default_palette, Config, TIMELINE_MAX_HEIGHT, TIMELINE_MIN_HEIGHT};
use crate::core::Document;
use crate::editor::{display_name, Editor};
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
use crate::ui::layers::RenameState;
use crate::ui::menu::MenuState;
use crate::ui::settings::{SettingsAction, SettingsTab, SettingsView, SettingsWindow};
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
    ConfirmDeleteFrame(usize),
    ConfirmOverwriteFrames {
        dir: PathBuf,
        base: String,
        existing: usize,
    },
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
    updater: UpdateManager,
    discord: DiscordPresence,
    session_start_unix: i64,
    last_auto_check_poll: Option<Instant>,
    notified_version: Option<semver::Version>,
    has_backup: bool,
    dialog: Option<Dialog>,
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
        let canvas = CanvasView::new(config.show_grid);
        let repaint_ctx = cc.egui_ctx.clone();
        let updater = UpdateManager::new(
            Arc::new(UreqClient::new()),
            Arc::new(move || repaint_ctx.request_repaint()),
        );
        Self {
            config,
            config_path,
            config_save_at: None,
            editor: None,
            canvas,
            timeline: Timeline::default(),
            player: Player::default(),
            settings: SettingsWindow::default(),
            updater,
            discord: DiscordPresence::start(release::DISCORD_APPLICATION_ID),
            session_start_unix: updater::unix_now() as i64,
            last_auto_check_poll: None,
            notified_version: None,
            has_backup: false,
            dialog: None,
            status: None,
            rename: None,
            allow_close: false,
            window_title: String::new(),
        }
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
        self.save_config();
        self.allow_close = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    /// Installs a new document, keeping the tool and colors of the previous one.
    fn set_document(&mut self, doc: Document, path: Option<PathBuf>, name: String) {
        self.player.reset();
        let mut editor = Editor::new(doc, path, name);
        if let Some(old) = &self.editor {
            editor.tool = old.tool;
            editor.settings = old.settings;
            editor.primary = old.primary;
            editor.secondary = old.secondary;
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

    /// Asks for a folder, then exports every frame (confirming before overwriting).
    fn export_all_frames(&mut self) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        editor.end_stroke();
        let base = editor.name().to_owned();
        let Some(dir) = native::pick_folder(self.start_dir().as_deref(), "Export All Frames")
        else {
            return;
        };
        let Some(editor) = &self.editor else {
            return;
        };
        let existing = file_io::frame_export_paths(editor.document(), &dir, &base)
            .iter()
            .filter(|p| p.exists())
            .count();
        if existing > 0 {
            self.dialog = Some(Dialog::ConfirmOverwriteFrames {
                dir,
                base,
                existing,
            });
        } else {
            self.write_all_frames(&dir, &base);
        }
    }

    fn write_all_frames(&mut self, dir: &Path, base: &str) {
        let Some(editor) = &self.editor else {
            return;
        };
        match file_io::export_frames(editor.document(), dir, base) {
            Ok(paths) => self.success(format!(
                "Exported {} frames to {}",
                paths.len(),
                dir.display()
            )),
            Err(e) => self.error_dialog("The frames were not exported", &e),
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
                self.stop_playback();
                self.export_all_frames();
            }
            Command::CloseDocument => {
                if self.editor.is_some() {
                    self.stop_playback();
                    self.request(ctx, Pending::CloseDocument);
                }
            }
            Command::Exit => self.request(ctx, Pending::Exit),
            Command::About => self.dialog = Some(Dialog::About),
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
                if index < self.config.palette.len() && self.config.palette.len() > 1 {
                    self.config.palette.remove(index);
                    self.save_config();
                }
            }
            Command::ResetPalette => {
                self.config.palette = default_palette();
                self.save_config();
            }
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
                } else if editor.document().frame_count() > 1 {
                    self.player.start(editor, &self.config.playback);
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
        let active = self
            .editor
            .as_ref()
            .map_or(0, |e| e.document().active_index());
        let (found, escape) = shortcuts::collect(ctx, active);
        commands.extend(found);
        if escape {
            if self.player.is_playing() {
                self.stop_playback();
            } else if let Some(editor) = &mut self.editor {
                if editor.is_stroking() {
                    editor.cancel_stroke();
                    self.info("Stroke canceled");
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
            Dialog::About => {
                if !dialogs::about(ctx) {
                    self.dialog = Some(Dialog::About);
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
            Dialog::ConfirmOverwriteFrames {
                dir,
                base,
                existing,
            } => {
                let message = format!(
                    "{existing} file(s) named “{base}_####.png” already exist in {}. Replace them?",
                    dir.display()
                );
                match dialogs::confirm(ctx, "Replace existing files?", &message, "Replace") {
                    ConfirmChoice::Pending => {
                        self.dialog = Some(Dialog::ConfirmOverwriteFrames {
                            dir,
                            base,
                            existing,
                        })
                    }
                    ConfirmChoice::Confirm => self.write_all_frames(&dir, &base),
                    ConfirmChoice::Cancel => {}
                }
            }
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

        egui::TopBottomPanel::top("menu-bar")
            .frame(bar(theme::BACKGROUND))
            .show(ctx, |ui| {
                let state = MenuState {
                    editor: self.editor.as_ref(),
                    recent: &self.config.recent_files,
                    show_grid: self.config.show_grid,
                    playing,
                    onion: self.config.onion.enabled,
                    timeline_visible: self.config.ui.timeline_visible,
                    side_panel_visible: self.config.ui.side_panel_visible,
                };
                menu::show(ui, &state, commands);
            });

        if let Some(editor) = &mut self.editor {
            egui::TopBottomPanel::top("tool-options")
                .frame(bar(theme::PANEL))
                .exact_height(36.0)
                .show(ctx, |ui| {
                    tool_options::show(ui, editor, self.config.show_grid, commands);
                });
        }

        egui::TopBottomPanel::bottom("status-bar")
            .frame(bar(theme::BACKGROUND))
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
                .fill(theme::PANEL)
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
                        .fill(theme::PANEL)
                        .inner_margin(Margin::same(6)),
                )
                .resizable(false)
                .exact_width(48.0)
                .show(ctx, |ui| toolbar::show(ui, editor.tool, commands));

            if self.config.ui.side_panel_visible {
                egui::SidePanel::right("side-panel")
                    .frame(
                        Frame::new()
                            .fill(theme::PANEL)
                            .inner_margin(Margin::same(10)),
                    )
                    .resizable(true)
                    .default_width(250.0)
                    .width_range(210.0..=420.0)
                    .show(ctx, |ui| {
                        palette::show(ui, editor, &self.config.palette, commands);
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
        };
        self.canvas.show_grid = self.config.show_grid;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(theme::BACKGROUND))
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
        self.handle_close_request(ctx);
        self.poll_updates();

        if let Some(editor) = &mut self.editor {
            self.player.tick(ctx, editor, &self.config.playback);
        }

        let mut commands = Vec::new();
        if self.dialog.is_none() {
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
        };
        for action in self.settings.show(ctx, &mut self.config, &view) {
            self.execute_settings_action(ctx, action);
        }

        self.show_dialogs(ctx);
        for command in commands {
            self.execute(ctx, command);
        }
        self.update_presence();
        self.update_title(ctx);

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

//! The eframe application: screens, panels, dialogs and command dispatch.

mod background;
mod dialog_flow;
mod files;
mod settings_actions;

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

    // ------------------------------------------------------------ services

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
            toolbar::panel(ctx).show(ctx, |ui| toolbar::show(ui, editor.tool, &keymap, commands));

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

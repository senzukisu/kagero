//! The eframe application: screens, panels, dialogs and command dispatch.

use std::path::{Path, PathBuf};

use egui::{Frame, Margin, ViewportCommand};

use crate::config::{default_palette, Config};
use crate::core::Document;
use crate::editor::{display_name, Editor};
use crate::file_io::{self, FileError};
use crate::services::dialogs as native;
use crate::theme;
use crate::ui::canvas::CanvasView;
use crate::ui::dialogs::{self, NewDocumentResult, NewDocumentState, UnsavedChoice};
use crate::ui::layers::RenameState;
use crate::ui::status::{MessageKind, StatusMessage};
use crate::ui::{home, layers, menu, palette, status, tool_options, toolbar, Command};

/// An action that may discard the open document and so must ask about unsaved changes.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    NewDocument,
    OpenPath(PathBuf),
    CloseDocument,
    Exit,
}

enum Dialog {
    NewDocument(NewDocumentState),
    Unsaved(Pending),
    Error { title: String, message: String },
    About,
}

pub struct KageroApp {
    config: Config,
    config_path: Option<PathBuf>,
    editor: Option<Editor>,
    canvas: CanvasView,
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
        Self {
            config,
            config_path,
            editor: None,
            canvas,
            dialog: None,
            status: None,
            rename: None,
            allow_close: false,
            window_title: String::new(),
        }
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

    fn error_dialog(&mut self, title: impl Into<String>, err: &FileError) {
        let title = title.into();
        tracing::error!("{title}: {err}");
        self.status_error(title.clone());
        self.dialog = Some(Dialog::Error {
            title,
            message: err.to_string(),
        });
    }

    fn save_config(&mut self) {
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
                self.editor = None;
                self.rename = None;
                self.canvas.reset();
            }
            Pending::Exit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    /// Installs a new document, keeping the tool and colors of the previous one.
    fn set_document(&mut self, doc: Document, path: Option<PathBuf>, name: String) {
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
                self.set_document(doc, save_path, display_name(path));
                self.config.add_recent(path);
                self.save_config();
                if imported {
                    self.info(format!(
                        "Imported {file_name} — use Save to keep it as a .kgr project"
                    ));
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
                self.save();
            }
            Command::SaveAs => {
                self.save_as();
            }
            Command::ExportPng => self.export_png(),
            Command::CloseDocument => {
                if self.editor.is_some() {
                    self.request(ctx, Pending::CloseDocument);
                }
            }
            Command::Exit => self.request(ctx, Pending::Exit),
            Command::About => self.dialog = Some(Dialog::About),
            Command::ToggleGrid => {
                self.canvas.show_grid = !self.canvas.show_grid;
                self.config.show_grid = self.canvas.show_grid;
                self.save_config();
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
            other => self.execute_on_document(other),
        }
    }

    /// Commands that need an open document; ignored when there is none.
    fn execute_on_document(&mut self, command: Command) {
        let Some(editor) = &mut self.editor else {
            return;
        };
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
            _ => Ok(()),
        };
        if let Err(e) = result {
            self.status_error(capitalize(&e.to_string()));
        }
    }

    // ------------------------------------------------------------ input

    fn collect_shortcuts(&mut self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        use egui::{Key, KeyboardShortcut as Shortcut, Modifiers};
        let typing = ctx.wants_keyboard_input();
        let active = self
            .editor
            .as_ref()
            .map_or(0, |e| e.document().active_index());
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let mut escape = false;
        ctx.input_mut(|i| {
            // Shift variants first: plain Ctrl+S would also match Ctrl+Shift+S.
            let global = [
                (cmd_shift, Key::S, Command::SaveAs),
                (cmd, Key::S, Command::Save),
                (cmd_shift, Key::N, Command::AddLayer),
                (cmd, Key::N, Command::NewDocument),
                (cmd, Key::O, Command::Open),
                (cmd, Key::E, Command::ExportPng),
                (cmd, Key::W, Command::CloseDocument),
                (cmd, Key::G, Command::ToggleGrid),
            ];
            for (mods, key, command) in global {
                if i.consume_shortcut(&Shortcut::new(mods, key)) {
                    commands.push(command);
                }
            }
            if typing {
                // Let text fields keep their own Ctrl+Z, letters and Space.
                return;
            }
            let editing = [
                (cmd_shift, Key::Z, Command::Redo),
                (cmd, Key::Z, Command::Undo),
                (cmd, Key::Y, Command::Redo),
                (cmd, Key::CloseBracket, Command::LayerUp),
                (cmd, Key::OpenBracket, Command::LayerDown),
            ];
            for (mods, key, command) in editing {
                if i.consume_shortcut(&Shortcut::new(mods, key)) {
                    commands.push(command);
                }
            }
            use crate::editor::ToolKind as T;
            if i.modifiers.shift_only() && i.key_pressed(Key::U) {
                commands.push(Command::SetTool(T::FilledRectangle));
            }
            let no_mods = i.modifiers.is_none();
            let plain = [
                (Key::B, Command::SetTool(T::Pencil)),
                (Key::E, Command::SetTool(T::Eraser)),
                (Key::G, Command::SetTool(T::Fill)),
                (Key::I, Command::SetTool(T::Eyedropper)),
                (Key::L, Command::SetTool(T::Line)),
                (Key::U, Command::SetTool(T::Rectangle)),
                (Key::H, Command::SetTool(T::Pan)),
                (Key::X, Command::SwapColors),
                (Key::Home, Command::CenterCanvas),
                (Key::Num0, Command::FitCanvas),
                (Key::Num1, Command::ZoomActual),
                (Key::F2, Command::BeginRenameLayer(active)),
            ];
            if no_mods {
                for (key, command) in plain {
                    if i.key_pressed(key) {
                        commands.push(command);
                    }
                }
            }
            if !i.modifiers.command && !i.modifiers.alt {
                if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                    commands.push(Command::ZoomIn);
                }
                if i.key_pressed(Key::Minus) {
                    commands.push(Command::ZoomOut);
                }
            }
            escape = i.key_pressed(Key::Escape);
        });
        if escape {
            if let Some(editor) = &mut self.editor {
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

        let mut commands = Vec::new();
        if self.dialog.is_none() {
            self.collect_shortcuts(ctx, &mut commands);
        }

        let bar = |fill| {
            Frame::new()
                .fill(fill)
                .inner_margin(Margin::symmetric(8, 4))
        };

        egui::TopBottomPanel::top("menu-bar")
            .frame(bar(theme::BACKGROUND))
            .show(ctx, |ui| {
                menu::show(
                    ui,
                    self.editor.as_ref(),
                    &self.config.recent_files,
                    self.canvas.show_grid,
                    &mut commands,
                );
            });

        if let Some(editor) = &mut self.editor {
            egui::TopBottomPanel::top("tool-options")
                .frame(bar(theme::PANEL))
                .exact_height(36.0)
                .show(ctx, |ui| {
                    tool_options::show(ui, editor, self.canvas.show_grid, &mut commands);
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
                    self.status.as_ref(),
                );
            });

        if let Some(editor) = &mut self.editor {
            egui::SidePanel::left("toolbar")
                .frame(
                    Frame::new()
                        .fill(theme::PANEL)
                        .inner_margin(Margin::same(6)),
                )
                .resizable(false)
                .exact_width(48.0)
                .show(ctx, |ui| toolbar::show(ui, editor.tool, &mut commands));

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
                    palette::show(ui, editor, &self.config.palette, &mut commands);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    layers::show(ui, editor, &mut self.rename, &mut commands);
                });
        }

        let mut canvas_error = None;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(theme::BACKGROUND))
            .show(ctx, |ui| match &mut self.editor {
                Some(editor) => canvas_error = self.canvas.show(ui, editor),
                None => home::show(ui, &self.config.recent_files, &mut commands),
            });
        if let Some(e) = canvas_error {
            self.status_error(capitalize(&e.to_string()));
        }

        self.show_dialogs(ctx);
        for command in commands {
            self.execute(ctx, command);
        }
        self.update_title(ctx);

        if self
            .status
            .as_ref()
            .is_some_and(|m| m.kind != MessageKind::Error && m.is_visible())
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

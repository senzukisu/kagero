//! Main menu bar.

use std::path::PathBuf;

use egui::Button;

use super::settings::SettingsTab;
use super::Command;
use crate::editor::{Editor, ToolKind, Transform};
use crate::export::ExportKind;

/// View state the menu shows as checkmarks and labels.
pub struct MenuState<'a> {
    pub editor: Option<&'a Editor>,
    pub recent: &'a [PathBuf],
    pub show_grid: bool,
    pub playing: bool,
    pub onion: bool,
    pub timeline_visible: bool,
    pub side_panel_visible: bool,
}

pub fn show(ui: &mut egui::Ui, state: &MenuState<'_>, commands: &mut Vec<Command>) {
    let editor = state.editor;
    let has_doc = editor.is_some();
    let editable = has_doc && !state.playing;
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            item(ui, commands, "New…", "Ctrl+N", true, Command::NewDocument);
            item(ui, commands, "Open…", "Ctrl+O", true, Command::Open);
            ui.add_enabled_ui(!state.recent.is_empty(), |ui| {
                ui.menu_button("Open Recent", |ui| {
                    for path in state.recent {
                        let label = path.display().to_string();
                        if ui.button(label).clicked() {
                            commands.push(Command::OpenPath(path.clone()));
                            ui.close_menu();
                        }
                    }
                });
            });
            ui.separator();
            item(ui, commands, "Save", "Ctrl+S", has_doc, Command::Save);
            item(
                ui,
                commands,
                "Save As…",
                "Ctrl+Shift+S",
                has_doc,
                Command::SaveAs,
            );
            item(
                ui,
                commands,
                "Export Current Frame as PNG…",
                "Ctrl+E",
                has_doc,
                Command::ExportPng,
            );
            item(
                ui,
                commands,
                "Export…",
                "Ctrl+Shift+E",
                has_doc,
                Command::OpenExport(ExportKind::Image),
            );
            item(
                ui,
                commands,
                "Export Animation as GIF…",
                "",
                has_doc,
                Command::OpenExport(ExportKind::Animation),
            );
            item(
                ui,
                commands,
                "Export Sprite Sheet…",
                "",
                has_doc,
                Command::OpenExport(ExportKind::SpriteSheet),
            );
            ui.separator();
            item(
                ui,
                commands,
                "Settings…",
                "Ctrl+,",
                true,
                Command::OpenSettings(SettingsTab::General),
            );
            ui.separator();
            item(
                ui,
                commands,
                "Close Document",
                "Ctrl+W",
                has_doc,
                Command::CloseDocument,
            );
            item(ui, commands, "Exit", "Alt+F4", true, Command::Exit);
        });

        ui.menu_button("Edit", |ui| {
            let (undo, redo) = editor.map_or((None, None), |e| {
                (e.history().undo_label(), e.history().redo_label())
            });
            let undo_text = undo.map_or("Undo".to_owned(), |l| format!("Undo {l}"));
            let redo_text = redo.map_or("Redo".to_owned(), |l| format!("Redo {l}"));
            item(
                ui,
                commands,
                &undo_text,
                "Ctrl+Z",
                undo.is_some(),
                Command::Undo,
            );
            item(
                ui,
                commands,
                &redo_text,
                "Ctrl+Y",
                redo.is_some(),
                Command::Redo,
            );
            ui.separator();
            ui.separator();
            let has_selection = editor.is_some_and(|e| e.selection().is_some());
            let has_clipboard = editor.is_some_and(|e| e.clipboard.is_some());
            let sel_edit = editable && has_selection;
            item(ui, commands, "Cut", "Ctrl+X", sel_edit, Command::Cut);
            item(ui, commands, "Copy", "Ctrl+C", has_selection, Command::Copy);
            item(
                ui,
                commands,
                "Paste",
                "Ctrl+V",
                editable && has_clipboard,
                Command::Paste,
            );
            item(
                ui,
                commands,
                "Delete Selection",
                "Delete",
                sel_edit,
                Command::DeleteSelection,
            );
            item(
                ui,
                commands,
                "Duplicate Selection",
                "Ctrl+D",
                sel_edit,
                Command::DuplicateSelection,
            );
            ui.separator();
            item(
                ui,
                commands,
                "Select All",
                "Ctrl+A",
                editable,
                Command::SelectAll,
            );
            item(
                ui,
                commands,
                "Deselect",
                "Esc",
                has_selection,
                Command::Deselect,
            );
            ui.separator();
            item(
                ui,
                commands,
                "Swap Colors",
                "X",
                has_doc,
                Command::SwapColors,
            );
        });

        ui.menu_button("Image", |ui| {
            let target = if editor.is_some_and(|e| e.selection().is_some()) {
                "Selection"
            } else {
                "Layer"
            };
            ui.label(
                egui::RichText::new(format!("Applies to: {target}"))
                    .small()
                    .weak(),
            );
            for (text, shortcut, kind) in [
                ("Flip Horizontal", "Shift+H", Transform::FlipHorizontal),
                ("Flip Vertical", "Shift+V", Transform::FlipVertical),
                ("Rotate 90° Clockwise", "Ctrl+R", Transform::Rotate90),
                ("Rotate 180°", "", Transform::Rotate180),
                (
                    "Rotate 90° Counterclockwise",
                    "Ctrl+Shift+R",
                    Transform::Rotate270,
                ),
            ] {
                item(
                    ui,
                    commands,
                    text,
                    shortcut,
                    editable,
                    Command::Transform(kind),
                );
            }
        });

        ui.menu_button("Tools", |ui| {
            let current = editor.map(|e| e.tool);
            for tool in ToolKind::ALL {
                let mut b = Button::new(tool.name()).shortcut_text(tool.shortcut());
                if current == Some(tool) {
                    b = b.selected(true);
                }
                if ui.add_enabled(has_doc, b).clicked() {
                    commands.push(Command::SetTool(tool));
                    ui.close_menu();
                }
            }
        });

        ui.menu_button("View", |ui| {
            check(
                ui,
                commands,
                "Pixel Grid",
                "Ctrl+G",
                true,
                state.show_grid,
                Command::ToggleGrid,
            );
            check(
                ui,
                commands,
                "Timeline",
                "F7",
                true,
                state.timeline_visible,
                Command::ToggleTimeline,
            );
            check(
                ui,
                commands,
                "Colors & Layers Panel",
                "F8",
                true,
                state.side_panel_visible,
                Command::ToggleSidePanel,
            );
            ui.separator();
            item(ui, commands, "Zoom In", "+", has_doc, Command::ZoomIn);
            item(ui, commands, "Zoom Out", "−", has_doc, Command::ZoomOut);
            item(
                ui,
                commands,
                "Actual Size (100%)",
                "1",
                has_doc,
                Command::ZoomActual,
            );
            item(
                ui,
                commands,
                "Fit to Window",
                "0",
                has_doc,
                Command::FitCanvas,
            );
            item(
                ui,
                commands,
                "Center Canvas",
                "Home",
                has_doc,
                Command::CenterCanvas,
            );
        });

        ui.menu_button("Layer", |ui| {
            let (count, active, can_add) = editor.map_or((0, 0, false), |e| {
                let d = e.document();
                (d.layer_count(), d.active_index(), d.can_add_layer().is_ok())
            });
            item(
                ui,
                commands,
                "New Layer",
                "Ctrl+Shift+N",
                can_add && editable,
                Command::AddLayer,
            );
            item(
                ui,
                commands,
                "Delete Layer",
                "",
                count > 1 && editable,
                Command::DeleteLayer,
            );
            item(
                ui,
                commands,
                "Rename Layer…",
                "F2",
                editable,
                Command::BeginRenameLayer(active),
            );
            item(
                ui,
                commands,
                "Toggle Visibility",
                "",
                editable,
                Command::ToggleLayerVisibility(active),
            );
            ui.separator();
            item(
                ui,
                commands,
                "Move Up",
                "Ctrl+]",
                editable && active + 1 < count,
                Command::LayerUp,
            );
            item(
                ui,
                commands,
                "Move Down",
                "Ctrl+[",
                editable && active > 0,
                Command::LayerDown,
            );
        });

        ui.menu_button("Frame", |ui| {
            let (count, active, can_add) = editor.map_or((0, 0, false), |e| {
                let d = e.document();
                (
                    d.frame_count(),
                    d.active_frame_index(),
                    d.can_add_frame().is_ok(),
                )
            });
            let play_label = if state.playing {
                "Stop Preview"
            } else {
                "Play Preview"
            };
            item(
                ui,
                commands,
                play_label,
                "Enter",
                has_doc && (count > 1 || state.playing),
                Command::TogglePlayback,
            );
            item(
                ui,
                commands,
                "First Frame",
                "Ctrl+Home",
                has_doc,
                Command::FirstFrame,
            );
            item(
                ui,
                commands,
                "Previous Frame",
                ",",
                has_doc,
                Command::PrevFrame,
            );
            item(ui, commands, "Next Frame", ".", has_doc, Command::NextFrame);
            ui.separator();
            item(
                ui,
                commands,
                "New Frame",
                "Alt+N",
                editable && can_add,
                Command::AddFrame,
            );
            item(
                ui,
                commands,
                "Duplicate Frame",
                "Alt+D",
                editable && can_add,
                Command::DuplicateFrame,
            );
            item(
                ui,
                commands,
                "Delete Frame",
                "",
                editable && count > 1,
                Command::DeleteFrame(active),
            );
            item(
                ui,
                commands,
                "Move Frame Left",
                "",
                editable && active > 0,
                Command::MoveFrameLeft(active),
            );
            item(
                ui,
                commands,
                "Move Frame Right",
                "",
                editable && active + 1 < count,
                Command::MoveFrameRight(active),
            );
            ui.separator();
            check(
                ui,
                commands,
                "Onion Skin",
                "O",
                has_doc,
                state.onion,
                Command::ToggleOnionSkin,
            );
        });

        ui.menu_button("Help", |ui| {
            item(
                ui,
                commands,
                "Check for Updates…",
                "",
                true,
                Command::CheckForUpdates,
            );
            item(
                ui,
                commands,
                "Discord Rich Presence…",
                "",
                true,
                Command::OpenSettings(SettingsTab::Discord),
            );
            ui.separator();
            item(ui, commands, "About Kagero", "", true, Command::About);
        });
    });
}

fn item(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    text: &str,
    shortcut: &str,
    enabled: bool,
    command: Command,
) {
    if ui
        .add_enabled(enabled, Button::new(text).shortcut_text(shortcut))
        .clicked()
    {
        commands.push(command);
        ui.close_menu();
    }
}

/// A menu item with a check mark.
fn check(
    ui: &mut egui::Ui,
    commands: &mut Vec<Command>,
    text: &str,
    shortcut: &str,
    enabled: bool,
    checked: bool,
    command: Command,
) {
    let button = Button::new(text).shortcut_text(shortcut).selected(checked);
    if ui.add_enabled(enabled, button).clicked() {
        commands.push(command);
        ui.close_menu();
    }
}

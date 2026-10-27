//! Main menu bar.

use std::path::PathBuf;

use egui::Button;

use super::Command;
use crate::editor::{Editor, ToolKind};

pub fn show(
    ui: &mut egui::Ui,
    editor: Option<&Editor>,
    recent: &[PathBuf],
    show_grid: bool,
    commands: &mut Vec<Command>,
) {
    let has_doc = editor.is_some();
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            item(ui, commands, "New…", "Ctrl+N", true, Command::NewDocument);
            item(ui, commands, "Open…", "Ctrl+O", true, Command::Open);
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Open Recent", |ui| {
                    for path in recent {
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
                "Export PNG…",
                "Ctrl+E",
                has_doc,
                Command::ExportPng,
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
            item(
                ui,
                commands,
                "Swap Colors",
                "X",
                has_doc,
                Command::SwapColors,
            );
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
            let grid_label = if show_grid { "Hide Grid" } else { "Show Grid" };
            item(
                ui,
                commands,
                grid_label,
                "Ctrl+G",
                true,
                Command::ToggleGrid,
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
                can_add,
                Command::AddLayer,
            );
            item(
                ui,
                commands,
                "Delete Layer",
                "",
                count > 1,
                Command::DeleteLayer,
            );
            item(
                ui,
                commands,
                "Rename Layer…",
                "F2",
                has_doc,
                Command::BeginRenameLayer(active),
            );
            item(
                ui,
                commands,
                "Toggle Visibility",
                "",
                has_doc,
                Command::ToggleLayerVisibility(active),
            );
            ui.separator();
            item(
                ui,
                commands,
                "Move Up",
                "Ctrl+]",
                active + 1 < count,
                Command::LayerUp,
            );
            item(
                ui,
                commands,
                "Move Down",
                "Ctrl+[",
                has_doc && active > 0,
                Command::LayerDown,
            );
        });

        ui.menu_button("Help", |ui| {
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

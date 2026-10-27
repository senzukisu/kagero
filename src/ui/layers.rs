//! Layers panel. Rows are listed top layer first, like a stack seen from above.

use egui::{Color32, Rect, RichText, Sense, Vec2};

use super::icons::{icon_button, Icon};
use super::Command;
use crate::core::MAX_LAYER_NAME_CHARS;
use crate::editor::Editor;
use crate::theme;

/// An inline rename in progress: layer index and the edited text.
pub type RenameState = Option<(usize, String)>;

pub fn show(
    ui: &mut egui::Ui,
    editor: &Editor,
    rename: &mut RenameState,
    commands: &mut Vec<Command>,
) {
    let doc = editor.document();
    let active = doc.active_index();
    let count = doc.layer_count();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("LAYERS")
                .small()
                .strong()
                .color(theme::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(
                ui,
                Icon::ArrowDown,
                28.0,
                false,
                active > 0,
                "Move layer down",
            )
            .clicked()
            {
                commands.push(Command::LayerDown);
            }
            if icon_button(
                ui,
                Icon::ArrowUp,
                28.0,
                false,
                active + 1 < count,
                "Move layer up",
            )
            .clicked()
            {
                commands.push(Command::LayerUp);
            }
            if icon_button(ui, Icon::Trash, 28.0, false, count > 1, "Delete layer").clicked() {
                commands.push(Command::DeleteLayer);
            }
            let can_add = doc.can_add_layer().is_ok();
            if icon_button(
                ui,
                Icon::Plus,
                28.0,
                false,
                can_add,
                "New layer (Ctrl+Shift+N)",
            )
            .clicked()
            {
                commands.push(Command::AddLayer);
            }
        });
    });
    ui.add_space(2.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for index in (0..count).rev() {
                let Some(layer) = doc.layers().get(index) else {
                    continue;
                };
                let row_height = 30.0;
                let (row_rect, row_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_height),
                    Sense::click(),
                );
                let selected = index == active;
                let bg = if selected {
                    theme::SURFACE_HOVER
                } else if row_resp.hovered() {
                    theme::SURFACE
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(row_rect, theme::RADIUS as f32, bg);
                if selected {
                    let bar = Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_height));
                    ui.painter().rect_filled(bar, 1.0, theme::ACCENT);
                }

                // Visibility toggle on the left.
                let eye_rect =
                    Rect::from_min_size(row_rect.min + Vec2::new(6.0, 1.0), Vec2::splat(28.0));
                let eye_icon = if layer.visible {
                    Icon::EyeOpen
                } else {
                    Icon::EyeClosed
                };
                let tip = if layer.visible {
                    "Hide layer"
                } else {
                    "Show layer"
                };
                let eye = ui.put(eye_rect, |ui: &mut egui::Ui| {
                    icon_button(ui, eye_icon, 28.0, false, true, tip)
                });
                if eye.clicked() {
                    commands.push(Command::ToggleLayerVisibility(index));
                }

                let name_rect = Rect::from_min_max(
                    row_rect.min + Vec2::new(40.0, 3.0),
                    row_rect.max - Vec2::new(6.0, 3.0),
                );
                let renaming_this = matches!(rename, Some((i, _)) if *i == index);
                if renaming_this {
                    if let Some((_, text)) = rename.as_mut() {
                        let edit = ui.put(
                            name_rect,
                            egui::TextEdit::singleline(text)
                                .char_limit(MAX_LAYER_NAME_CHARS)
                                .id_salt(("layer-rename", index)),
                        );
                        if !edit.has_focus() && !edit.lost_focus() {
                            edit.request_focus();
                        }
                        let (enter, escape) = ui.input(|i| {
                            (
                                i.key_pressed(egui::Key::Enter),
                                i.key_pressed(egui::Key::Escape),
                            )
                        });
                        if escape {
                            *rename = None;
                        } else if edit.lost_focus() || enter {
                            commands.push(Command::RenameLayer(index, text.clone()));
                            *rename = None;
                        }
                    }
                } else {
                    let color = if layer.visible {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    };
                    ui.painter().text(
                        name_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        &layer.name,
                        egui::FontId::proportional(14.0),
                        color,
                    );
                }

                if row_resp.clicked() && !eye.hovered() {
                    commands.push(Command::SelectLayer(index));
                }
                if row_resp.double_clicked() {
                    commands.push(Command::BeginRenameLayer(index));
                }
                row_resp.context_menu(|ui| {
                    if ui.button("Rename…").clicked() {
                        commands.push(Command::BeginRenameLayer(index));
                        ui.close_menu();
                    }
                    let label = if layer.visible { "Hide" } else { "Show" };
                    if ui.button(label).clicked() {
                        commands.push(Command::ToggleLayerVisibility(index));
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(count > 1, egui::Button::new("Delete"))
                        .clicked()
                    {
                        commands.push(Command::SelectLayer(index));
                        commands.push(Command::DeleteLayer);
                        ui.close_menu();
                    }
                });
            }
        });
}

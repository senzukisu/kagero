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
                .color(theme::text_secondary()),
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
            let can_merge = editor.can_merge_down(active).is_ok();
            if icon_button(
                ui,
                Icon::MergeDown,
                28.0,
                false,
                can_merge,
                "Merge down into the layer below",
            )
            .clicked()
            {
                commands.push(Command::MergeDown);
            }
            let can_add = doc.can_add_layer().is_ok();
            if icon_button(ui, Icon::Duplicate, 28.0, false, can_add, "Duplicate layer").clicked() {
                commands.push(Command::DuplicateLayer);
            }
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
    layer_properties(ui, editor, commands);
    ui.add_space(4.0);

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
                    theme::surface_hover()
                } else if row_resp.hovered() {
                    theme::surface()
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(row_rect, theme::RADIUS as f32, bg);
                if selected {
                    let bar = Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_height));
                    ui.painter().rect_filled(bar, 0.0, theme::accent());
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
                        theme::text_primary()
                    } else {
                        theme::text_secondary()
                    };
                    ui.painter().text(
                        name_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        &layer.name,
                        theme::font(1),
                        color,
                    );
                    // Right side: opacity when not full, and lock markers.
                    let mut x = name_rect.right();
                    for (on, icon) in [
                        (layer.locked, Icon::Lock),
                        (layer.alpha_lock, Icon::AlphaLock),
                    ] {
                        if on {
                            let r = Rect::from_center_size(
                                egui::pos2(x - 8.0, name_rect.center().y),
                                Vec2::splat(16.0),
                            );
                            super::icons::paint_icon(
                                ui.painter(),
                                r,
                                icon,
                                super::icons::Tint::Normal,
                            );
                            x -= 20.0;
                        }
                    }
                    if layer.opacity < 255 {
                        ui.painter().text(
                            egui::pos2(x - 2.0, name_rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            format!("{}%", percent(layer.opacity)),
                            egui::TextStyle::Small.resolve(ui.style()),
                            theme::text_secondary(),
                        );
                    }
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
                    if ui
                        .add_enabled(doc.can_add_layer().is_ok(), egui::Button::new("Duplicate"))
                        .clicked()
                    {
                        commands.push(Command::SelectLayer(index));
                        commands.push(Command::DuplicateLayer);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            editor.can_merge_down(index).is_ok(),
                            egui::Button::new("Merge Down"),
                        )
                        .clicked()
                    {
                        commands.push(Command::SelectLayer(index));
                        commands.push(Command::MergeDown);
                        ui.close_menu();
                    }
                    ui.separator();
                    let label = if layer.visible { "Hide" } else { "Show" };
                    if ui.button(label).clicked() {
                        commands.push(Command::ToggleLayerVisibility(index));
                        ui.close_menu();
                    }
                    let mut props = layer.props();
                    if ui
                        .checkbox(&mut props.locked, "Lock")
                        .on_hover_text("Locked layers can't be drawn on or changed")
                        .changed()
                    {
                        commands.push(Command::SetLayerProps(index, props));
                        ui.close_menu();
                    }
                    let mut props = layer.props();
                    if ui
                        .checkbox(&mut props.alpha_lock, "Alpha Lock")
                        .on_hover_text("Only recolor pixels that already have color")
                        .changed()
                    {
                        commands.push(Command::SetLayerProps(index, props));
                        ui.close_menu();
                    }
                    ui.separator();
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

fn percent(opacity: u8) -> u32 {
    (u32::from(opacity) * 100 + 127) / 255
}

/// Opacity slider and lock toggles of the active layer.
fn layer_properties(ui: &mut egui::Ui, editor: &Editor, commands: &mut Vec<Command>) {
    let doc = editor.document();
    let index = doc.active_index();
    let layer = doc.active_layer();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Opacity").color(theme::text_secondary()));
        let mut value = percent(layer.opacity);
        let slider = egui::Slider::new(&mut value, 0..=100)
            .suffix("%")
            .clamping(egui::SliderClamping::Always);
        ui.spacing_mut().slider_width = (ui.available_width() - 120.0).max(60.0);
        let resp = ui
            .add(slider)
            .on_hover_text("How strongly this layer shows (not saved into its pixels)");
        let opacity = ((value * 255 + 50) / 100) as u8;
        // A drag previews live and becomes one undo step when released.
        let start_id = egui::Id::new(("layer-opacity-start", layer.id.0));
        if resp.drag_started() {
            ui.data_mut(|d| d.insert_temp(start_id, layer.opacity));
        }
        if resp.changed() {
            if resp.dragged() {
                commands.push(Command::PreviewLayerOpacity(index, opacity));
            } else {
                let mut props = layer.props();
                props.opacity = opacity;
                commands.push(Command::SetLayerProps(index, props));
            }
        }
        if resp.drag_stopped() {
            let before = ui
                .data_mut(|d| d.remove_temp::<u8>(start_id))
                .unwrap_or(layer.opacity);
            commands.push(Command::PreviewLayerOpacity(index, opacity));
            commands.push(Command::CommitLayerOpacity(index, before));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let mut props = layer.props();
            let tip = if props.alpha_lock {
                "Alpha lock on: only colored pixels change"
            } else {
                "Alpha lock: only recolor pixels that already have color"
            };
            if icon_button(ui, Icon::AlphaLock, 26.0, props.alpha_lock, true, tip).clicked() {
                props.alpha_lock = !props.alpha_lock;
                commands.push(Command::SetLayerProps(index, props));
            }
            let mut props = layer.props();
            let tip = if props.locked {
                "Locked: click to unlock"
            } else {
                "Lock the layer against changes"
            };
            if icon_button(ui, Icon::Lock, 26.0, props.locked, true, tip).clicked() {
                props.locked = !props.locked;
                commands.push(Command::SetLayerProps(index, props));
            }
        });
    });
}

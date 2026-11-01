//! Color panel: primary/secondary colors, palettes and recently used colors.
//!
//! Switching palettes never changes the document — palettes are only for picking.

use egui::{Color32, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

use super::icons::{icon_button, Icon};
use super::Command;
use crate::config::Config;
use crate::core::Rgba;
use crate::editor::{ColorSlot, Editor};
use crate::theme;

const SWATCH: f32 = 20.0;

fn to_color32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), c.a())
}

/// Paints a color swatch with a checkerboard under translucent colors.
pub fn paint_swatch(painter: &egui::Painter, rect: Rect, color: Rgba) {
    if color.a() < 255 {
        let half = rect.width() / 2.0;
        painter.rect_filled(rect, 0.0, theme::checker_light());
        for (dx, dy) in [(0.0, 0.0), (half, half)] {
            let r = Rect::from_min_size(
                rect.min + Vec2::new(dx, dy),
                Vec2::new(half, rect.height() / 2.0),
            );
            painter.rect_filled(r, 0.0, theme::checker_dark());
        }
    }
    painter.rect_filled(rect, 0.0, to_color32(color));
}

/// Rename state for the active palette (text being edited).
pub type PaletteRename = Option<String>;

pub fn show(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    config: &Config,
    rename: &mut PaletteRename,
    commands: &mut Vec<Command>,
) {
    ui.label(
        RichText::new("COLORS")
            .small()
            .strong()
            .color(theme::text_secondary()),
    );
    ui.horizontal(|ui| {
        for (slot, label) in [
            (ColorSlot::Primary, "Primary"),
            (ColorSlot::Secondary, "Secondary"),
        ] {
            ui.vertical(|ui| {
                let mut c = editor.color(slot).0;
                let resp = ui
                    .color_edit_button_srgba_unmultiplied(&mut c)
                    .on_hover_text(format!("{label} color — click to edit"));
                if resp.changed() {
                    editor.set_color(slot, Rgba(c));
                }
                let hex = editor.color(slot).to_hex();
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(&hex)
                                .small()
                                .monospace()
                                .color(theme::text_secondary()),
                        )
                        .sense(Sense::click()),
                    )
                    .on_hover_text("Click to copy")
                    .clicked()
                {
                    commands.push(Command::CopyHex(editor.color(slot)));
                }
            });
        }
        if icon_button(ui, Icon::Swap, 28.0, false, true, "Swap colors (X)").clicked() {
            commands.push(Command::SwapColors);
        }
    });
    color_input(ui, editor);

    // Recently used colors.
    if !config.recent_colors.is_empty() {
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.0);
            ui.label(
                RichText::new("Recent")
                    .small()
                    .color(theme::text_secondary()),
            );
            for &c in &config.recent_colors {
                swatch(ui, editor, c, None, commands);
            }
        });
    }

    // Palette selector and its menu.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if let Some(text) = rename {
            let edit = ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(140.0)
                    .char_limit(48),
            );
            edit.request_focus();
            let (enter, escape) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                )
            });
            if escape {
                *rename = None;
            } else if enter || edit.lost_focus() {
                commands.push(Command::RenamePalette(text.clone()));
                *rename = None;
            }
            return;
        }
        let active = config.active_palette();
        egui::ComboBox::from_id_salt("palette-select")
            .selected_text(&active.name)
            .width(140.0)
            .show_ui(ui, |ui| {
                for (i, p) in config.palettes.iter().enumerate() {
                    if ui
                        .selectable_label(
                            i == config.active_palette,
                            format!("{} ({})", p.name, p.colors.len()),
                        )
                        .clicked()
                    {
                        commands.push(Command::SelectPalette(i));
                    }
                }
            });
        ui.menu_button("...", |ui| {
            let mut item = |ui: &mut egui::Ui, text: &str, enabled: bool, c: Command| {
                if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
                    commands.push(c);
                    ui.close_menu();
                }
            };
            item(ui, "Add Primary Color", true, Command::AddPaletteColor);
            item(
                ui,
                "Convert Image to Palette…",
                true,
                Command::OpenConvertPalette,
            );
            ui.separator();
            item(ui, "New Palette", true, Command::NewPalette);
            item(ui, "Duplicate Palette", true, Command::DuplicatePalette);
            if ui.button("Rename Palette…").clicked() {
                *rename = Some(config.active_palette().name.clone());
                ui.close_menu();
            }
            item(
                ui,
                "Delete Palette",
                config.palettes.len() > 1,
                Command::DeletePalette,
            );
            ui.separator();
            item(
                ui,
                "Import Palette… (.gpl, .hex)",
                true,
                Command::ImportPalette,
            );
            item(ui, "Export Palette…", true, Command::ExportPalette);
            ui.separator();
            item(ui, "Restore Built-in Palettes", true, Command::ResetPalette);
        })
        .response
        .on_hover_text("Palette actions");
    });

    // Swatches.
    let palette = &config.active_palette().colors;
    let width = ui.available_width();
    let per_row = ((width + 2.0) / (SWATCH + 2.0)).floor().max(1.0) as usize;
    for (row_index, row) in palette.chunks(per_row).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.0);
            for (col, &color) in row.iter().enumerate() {
                swatch(ui, editor, color, Some(row_index * per_row + col), commands);
            }
        });
    }
}

/// Exact entry of the primary color: a hex field and R, G, B, A values.
fn color_input(ui: &mut egui::Ui, editor: &mut Editor) {
    let id = ui.id().with("primary-hex");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // While typing, the text stays as typed; it's applied on Enter or leaving.
        let stored: Option<String> = ui.data(|d| d.get_temp(id));
        let mut text = stored.unwrap_or_else(|| editor.primary.to_hex());
        let valid = Rgba::from_hex(&text).is_some();
        let resp = ui
            .add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(84.0)
                    .char_limit(9)
                    .font(egui::TextStyle::Monospace)
                    .text_color(if valid {
                        theme::text_primary()
                    } else {
                        theme::error()
                    }),
            )
            .on_hover_text(
                "Primary color as #RRGGBB or #RRGGBBAA (#RGB works too) — Enter applies",
            );
        if resp.has_focus() {
            ui.data_mut(|d| d.insert_temp(id, text.clone()));
        }
        if resp.lost_focus() {
            if let Some(c) = Rgba::from_hex(&text) {
                editor.set_color(ColorSlot::Primary, c);
            }
            ui.data_mut(|d| d.remove::<String>(id));
        }
        let mut c = editor.primary.0;
        let mut changed = false;
        for (i, name) in ["R", "G", "B", "A"].into_iter().enumerate() {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c[i])
                        .range(0..=255)
                        .prefix(format!("{name} ")),
                )
                .on_hover_text(format!("Primary color: {name} (0–255)"))
                .changed();
        }
        if changed {
            editor.set_color(ColorSlot::Primary, Rgba(c));
        }
    });
}

/// One clickable swatch. `index` is set for palette swatches (enables the menu).
fn swatch(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    color: Rgba,
    index: Option<usize>,
    commands: &mut Vec<Command>,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(SWATCH), Sense::click());
    paint_swatch(ui.painter(), rect, color);
    let selected = color == editor.primary;
    if selected || resp.hovered() {
        let stroke_color = if selected {
            theme::accent()
        } else {
            theme::text_primary()
        };
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.5_f32, stroke_color),
            StrokeKind::Inside,
        );
    }
    if resp.clicked() {
        editor.primary = color;
        commands.push(Command::UseColor(color));
    }
    if resp.secondary_clicked() && index.is_none() {
        editor.secondary = color;
    }
    let resp = resp.on_hover_text(format!(
        "{}  ·  R {} G {} B {} A {}\nLeft click: primary · right click: menu",
        color.to_hex(),
        color.r(),
        color.g(),
        color.b(),
        color.a()
    ));
    if let Some(i) = index {
        resp.context_menu(|ui| {
            if ui.button("Use as Primary").clicked() {
                editor.primary = color;
                ui.close_menu();
            }
            if ui.button("Use as Secondary").clicked() {
                editor.secondary = color;
                ui.close_menu();
            }
            if ui.button(format!("Copy {}", color.to_hex())).clicked() {
                commands.push(Command::CopyHex(color));
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Move Left").clicked() {
                commands.push(Command::MovePaletteColor(i, -1));
                ui.close_menu();
            }
            if ui.button("Move Right").clicked() {
                commands.push(Command::MovePaletteColor(i, 1));
                ui.close_menu();
            }
            if ui.button("Remove from Palette").clicked() {
                commands.push(Command::RemovePaletteColor(i));
                ui.close_menu();
            }
        });
    }
}

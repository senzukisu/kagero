//! Color panel: primary/secondary colors and the palette.

use egui::{Color32, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

use super::icons::{icon_button, Icon};
use super::Command;
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
        painter.rect_filled(rect, 0.0, theme::CHECKER_LIGHT);
        for (dx, dy) in [(0.0, 0.0), (half, half)] {
            let r = Rect::from_min_size(
                rect.min + Vec2::new(dx, dy),
                Vec2::new(half, rect.height() / 2.0),
            );
            painter.rect_filled(r, 0.0, theme::CHECKER_DARK);
        }
    }
    painter.rect_filled(rect, 0.0, to_color32(color));
}

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, palette: &[Rgba], commands: &mut Vec<Command>) {
    ui.label(
        RichText::new("COLORS")
            .small()
            .strong()
            .color(theme::TEXT_SECONDARY),
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
                ui.label(
                    RichText::new(editor.color(slot).to_hex())
                        .small()
                        .monospace()
                        .color(theme::TEXT_SECONDARY),
                );
            });
        }
        if icon_button(ui, Icon::Swap, 28.0, false, true, "Swap colors (X)").clicked() {
            commands.push(Command::SwapColors);
        }
    });

    ui.add_space(4.0);
    let width = ui.available_width();
    let per_row = ((width + 2.0) / (SWATCH + 2.0)).floor().max(1.0) as usize;
    for (row_index, row) in palette.chunks(per_row).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.0);
            for (col, &color) in row.iter().enumerate() {
                let index = row_index * per_row + col;
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(SWATCH), Sense::click());
                paint_swatch(ui.painter(), rect, color);
                let selected = color == editor.primary;
                if selected || resp.hovered() {
                    let stroke_color = if selected {
                        theme::ACCENT
                    } else {
                        theme::TEXT_PRIMARY
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
                }
                if resp.secondary_clicked() {
                    editor.secondary = color;
                }
                let resp = resp.on_hover_text(format!(
                    "{}\nLeft click: primary · Right click: secondary\nMiddle click: remove",
                    color.to_hex()
                ));
                if resp.middle_clicked() {
                    commands.push(Command::RemovePaletteColor(index));
                }
            }
        });
    }
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        if ui
            .small_button("Add primary")
            .on_hover_text("Add the primary color to the palette")
            .clicked()
        {
            commands.push(Command::AddPaletteColor);
        }
        if ui
            .small_button("Reset")
            .on_hover_text("Restore the default Kagero palette")
            .clicked()
        {
            commands.push(Command::ResetPalette);
        }
    });
}

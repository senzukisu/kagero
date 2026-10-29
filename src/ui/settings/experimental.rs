//! Settings → Experimental: features that are switched on only here.

use egui::RichText;

use super::{hint, section, toggle, SettingsAction};
use crate::config::Config;
use crate::theme;

pub fn show(ui: &mut egui::Ui, config: &mut Config, actions: &mut Vec<SettingsAction>) {
    egui::Frame::new()
        .fill(theme::EMBER_DEEP.gamma_multiply(0.35))
        .stroke(egui::Stroke::new(
            1.0_f32,
            theme::ACCENT.gamma_multiply(0.6),
        ))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(
                    "These features are still being tested. They may change, move or be \
                     removed in a later version. Your projects are never affected.",
                )
                .color(theme::TEXT_PRIMARY),
            );
        });

    let x = &mut config.experimental;
    let mut changed = false;

    section(ui, "Interface");
    changed |= toggle(ui, &mut x.pixel_font, "Pixel font");
    hint(
        ui,
        "Kagero Pixel, Kagero's own bitmap font, for the whole interface — always sharp, \
         drawn on whole screen pixels.",
    );

    section(ui, "Drawing");
    changed |= toggle(ui, &mut x.pixel_perfect, "Pixel-perfect pencil");
    hint(
        ui,
        "1 px Pencil and Eraser strokes skip the doubled pixel in “L” corners, so \
         freehand lines stay one pixel thin.",
    );

    section(ui, "Canvas");
    changed |= toggle(ui, &mut x.tiled_view, "Tiled view");
    hint(
        ui,
        "Shows the image repeated around itself, dimmed, to check seamless tiles and \
         patterns. View only — the copies are never drawn on or exported.",
    );

    if changed {
        actions.push(SettingsAction::Save);
    }
}

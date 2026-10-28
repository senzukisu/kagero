use super::{hint, section, toggle, SettingsAction};
use crate::config::Config;

pub fn show(ui: &mut egui::Ui, config: &mut Config, actions: &mut Vec<SettingsAction>) {
    let mut changed = false;
    section(ui, "Workspace");
    changed |= toggle(
        ui,
        &mut config.ui.timeline_visible,
        "Show the timeline (F7)",
    );
    changed |= toggle(
        ui,
        &mut config.ui.side_panel_visible,
        "Show colors and layers panel (F8)",
    );
    changed |= toggle(
        ui,
        &mut config.show_grid,
        "Pixel grid when zoomed in (Ctrl+G)",
    );
    changed |= toggle(ui, &mut config.ui.pixel_font, "Pixel font (experimental)");
    hint(
        ui,
        "Kagero Pixel, Kagero's own bitmap font. Off: the classic font from 0.3.0.",
    );
    changed |= toggle(ui, &mut config.ui.reduce_motion, "Reduce motion");
    hint(
        ui,
        "Turns off the pulsing preview indicator and eased transitions.",
    );

    section(ui, "Onion skin");
    changed |= toggle(ui, &mut config.onion.enabled, "Show neighbor frames (O)");
    changed |= toggle(
        ui,
        &mut config.onion.show_previous,
        "Previous frame (warm tint)",
    );
    changed |= toggle(ui, &mut config.onion.show_next, "Next frame (cool tint)");
    ui.horizontal(|ui| {
        ui.label("Opacity");
        let mut percent = (config.onion.clamped_opacity() * 100.0).round() as u32;
        let resp = ui.add(egui::Slider::new(&mut percent, 5..=90).suffix("%"));
        if resp.changed() {
            config.onion.opacity = percent as f32 / 100.0;
        }
        changed |= resp.drag_stopped() || (resp.changed() && !resp.dragged());
    });
    hint(
        ui,
        "Onion skin is only a drawing aid: it is never saved into frames or exported.",
    );
    if changed {
        actions.push(SettingsAction::Save);
    }
}

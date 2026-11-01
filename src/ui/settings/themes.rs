//! Settings → Themes: built-in and imported color themes, previewed live.

use egui::{Color32, RichText, Sense, Vec2};

use super::{hint, section, SettingsAction, SettingsView};
use crate::theme;
use crate::themes::Theme;

pub fn show(ui: &mut egui::Ui, view: &SettingsView, actions: &mut Vec<SettingsAction>) {
    if let Some(name) = &view.theme_preview {
        egui::Frame::new()
            .fill(theme::ember_deep())
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Previewing {name}"))
                            .strong()
                            .color(theme::text_primary()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Revert").clicked() {
                            actions.push(SettingsAction::RevertTheme);
                        }
                        if ui
                            .add(super::super::dialogs::accent_button("Keep This Theme"))
                            .clicked()
                        {
                            actions.push(SettingsAction::KeepTheme);
                        }
                    });
                });
            });
        hint(
            ui,
            "Closing Settings without keeping it goes back to your theme.",
        );
    }

    section(ui, "Themes");
    let shown = view.theme_preview.as_deref().unwrap_or(&view.current_theme);
    let width = (ui.available_width() - 8.0) / 2.0;
    for pair in view.themes.chunks(2) {
        ui.horizontal_top(|ui| {
            for t in pair {
                card(
                    ui,
                    t,
                    width,
                    t.name == view.current_theme,
                    t.name == shown,
                    actions,
                );
            }
        });
        ui.add_space(4.0);
    }

    section(ui, "Your themes");
    ui.horizontal(|ui| {
        if ui.button("Import Theme…").clicked() {
            actions.push(SettingsAction::ImportTheme);
        }
        if ui.button("Open Themes Folder").clicked() {
            actions.push(SettingsAction::OpenThemesFolder);
        }
        if ui
            .button("Reload")
            .on_hover_text("Read the themes folder again")
            .clicked()
        {
            actions.push(SettingsAction::ReloadThemes);
        }
    });
    hint(
        ui,
        "A theme is a small JSON file (format \"kagero-theme\", version 1). Export a built-in theme to get a starting point. Themes with hard-to-read text are refused.",
    );
    for (file, problem) in &view.theme_problems {
        ui.label(
            RichText::new(format!("{file}: {problem}"))
                .small()
                .color(theme::error()),
        );
    }
}

fn card(
    ui: &mut egui::Ui,
    t: &Theme,
    width: f32,
    current: bool,
    shown: bool,
    actions: &mut Vec<SettingsAction>,
) {
    let c = t.colors;
    let to = |c: Color32| c;
    egui::Frame::new()
        .fill(to(c.panel))
        .stroke(egui::Stroke::new(
            if shown { 2.0_f32 } else { 1.0_f32 },
            if shown {
                theme::accent()
            } else {
                theme::border()
            },
        ))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            // Stacked inside the card (the row of cards is horizontal).
            ui.vertical(|ui| {
                ui.set_width(width - 20.0);
                ui.horizontal(|ui| {
                    // The theme's own colors, so cards show what you'd get.
                    ui.label(RichText::new(&t.name).strong().color(c.text_primary));
                    let badge = match (current, t.is_builtin()) {
                        (true, _) => "In use",
                        (false, true) => "Built-in",
                        (false, false) => "Yours",
                    };
                    ui.label(RichText::new(badge).small().color(c.text_secondary));
                });
                let (strip, _) =
                    ui.allocate_exact_size(Vec2::new(width - 20.0, 18.0), Sense::hover());
                let swatches = [
                    c.background,
                    c.surface,
                    c.surface_hover,
                    c.accent,
                    c.ember_deep,
                    c.success,
                    c.error,
                    c.text_primary,
                ];
                let w = strip.width() / swatches.len() as f32;
                for (i, s) in swatches.into_iter().enumerate() {
                    let r = egui::Rect::from_min_size(
                        strip.min + Vec2::new(i as f32 * w, 0.0),
                        Vec2::new(w, strip.height()),
                    );
                    ui.painter().rect_filled(r, 0.0, s);
                }
                ui.horizontal(|ui| {
                    let label = if shown { "Showing" } else { "Preview" };
                    if ui
                        .add_enabled(!shown, egui::Button::new(label))
                        .on_hover_text("Try it on the whole window")
                        .clicked()
                    {
                        actions.push(SettingsAction::PreviewTheme(t.name.clone()));
                    }
                    if ui
                        .button("Export…")
                        .on_hover_text("Save as a theme file")
                        .clicked()
                    {
                        actions.push(SettingsAction::ExportTheme(t.name.clone()));
                    }
                    if !t.is_builtin() {
                        // Deleting asks once more in place.
                        let id = egui::Id::new(("delete-theme", &t.name));
                        let armed = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
                        let text = if armed { "Really delete?" } else { "Delete" };
                        let resp = ui.button(RichText::new(text).color(if armed {
                            theme::error()
                        } else {
                            theme::text_primary()
                        }));
                        if resp.clicked() {
                            if armed {
                                actions.push(SettingsAction::DeleteTheme(t.name.clone()));
                                ui.data_mut(|d| d.remove::<bool>(id));
                            } else {
                                ui.data_mut(|d| d.insert_temp(id, true));
                            }
                        }
                        if armed && !resp.hovered() && ui.input(|i| i.pointer.any_click()) {
                            ui.data_mut(|d| d.remove::<bool>(id));
                        }
                    }
                });
            });
        });
}

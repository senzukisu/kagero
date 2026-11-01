//! Settings → Plugins: installed plugins, their state and on/off switches.

use egui::RichText;

use super::{hint, section, toggle, SettingsAction, SettingsView};
use crate::config::Config;
use crate::theme;

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    view: &SettingsView,
    actions: &mut Vec<SettingsAction>,
) {
    section(ui, "Safety");
    hint(
        ui,
        "Plugins are Lua scripts. They run in a sandbox: they can read and change only the layer you run them on — no files, no network, no other programs — with limited memory and time. Still, turn on only plugins you trust. New plugins start turned off.",
    );

    section(ui, "Installed");
    if view.plugins.is_empty() {
        hint(
            ui,
            "No plugins yet. Install the example to see how they work, or put plugin folders into the plugins folder.",
        );
    }
    for p in &view.plugins {
        let id = p.id();
        egui::Frame::new()
            .fill(theme::surface())
            .stroke(egui::Stroke::new(1.0_f32, theme::border()))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                match &p.manifest {
                    Some(m) => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&m.name).strong().color(theme::text_primary()));
                            ui.label(
                                RichText::new(format!("{} · {}", m.version, m.id))
                                    .small()
                                    .color(theme::text_secondary()),
                            );
                        });
                        if !m.author.is_empty() {
                            hint(ui, &format!("by {}", m.author));
                        }
                        if !m.description.is_empty() {
                            ui.label(RichText::new(&m.description).color(theme::text_secondary()));
                        }
                        let titles: Vec<&str> =
                            m.commands.iter().map(|c| c.title.as_str()).collect();
                        hint(ui, &format!("Commands: {}", titles.join(" · ")));
                    }
                    None => {
                        ui.label(RichText::new(&id).strong().color(theme::text_primary()));
                    }
                }
                match &p.problem {
                    Some(problem) => {
                        ui.label(
                            RichText::new(format!("Can't be used: {problem}"))
                                .color(theme::error()),
                        );
                    }
                    None => {
                        let mut on = config.plugins.enabled.contains(&id);
                        if toggle(ui, &mut on, "Turned on (shown in the Plugins menu)") {
                            config.plugins.enabled.retain(|e| *e != id);
                            if on {
                                config.plugins.enabled.push(id.clone());
                            }
                            actions.push(SettingsAction::Save);
                        }
                    }
                }
            });
        ui.add_space(4.0);
    }

    section(ui, "Manage");
    ui.horizontal(|ui| {
        if ui.button("Open Plugins Folder").clicked() {
            actions.push(SettingsAction::OpenPluginsFolder);
        }
        if ui
            .button("Reload")
            .on_hover_text("Read the plugins folder again")
            .clicked()
        {
            actions.push(SettingsAction::ReloadPlugins);
        }
        if ui
            .button("Install Example Plugin")
            .on_hover_text(
                "Adds \"Example Tools\" (invert, grayscale, outline) to your plugins folder",
            )
            .clicked()
        {
            actions.push(SettingsAction::InstallExamplePlugin);
        }
    });
    hint(
        ui,
        "Each plugin is a folder with plugin.json and main.lua (plugin API 1). How to write one: docs/plugins/README.md.",
    );
}

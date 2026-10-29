//! Settings → Shortcuts: every rebindable action with its key.
//!
//! Click a key to change it, then press the new combination (Esc cancels). A key
//! already used elsewhere moves to the new action, and the page says where it came
//! from. ↺ restores one default, *Reset all* restores every default.

use egui::{Event, Key, RichText};

use super::{hint, section, SettingsAction};
use crate::config::Config;
use crate::theme;
use crate::ui::keymap::{assign, Action, Binding, Group, Keymap, FIXED};

/// Keys that can't be assigned because they have fixed jobs.
fn reserved(key: Key) -> bool {
    matches!(
        key,
        Key::Escape
            | Key::Space
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Tab
    )
}

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    capturing: &mut Option<Action>,
    filter: &mut String,
    note: &mut Option<String>,
    actions: &mut Vec<SettingsAction>,
) {
    // A shortcut waiting for its key: take the next key press.
    if let Some(action) = *capturing {
        let pressed = ui.ctx().input_mut(|i| {
            let found = i.events.iter().find_map(|e| match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            });
            if found.is_some() {
                i.events.retain(|e| !matches!(e, Event::Key { .. }));
            }
            found
        });
        if let Some((key, mods)) = pressed {
            *capturing = None;
            if key == Key::Escape {
                *note = None;
            } else if reserved(key) {
                *note = Some(format!(
                    "{} has a fixed job and can't be assigned.",
                    key.name()
                ));
            } else {
                let binding = Binding {
                    ctrl: mods.command,
                    shift: mods.shift,
                    alt: mods.alt,
                    key,
                };
                let taken = assign(&mut config.shortcuts, action, Some(binding));
                *note = Some(match taken {
                    Some(other) => format!(
                        "{} now does “{}” — it was taken from “{}”, which has no key now.",
                        binding.label(),
                        action.label(),
                        other.label()
                    ),
                    None => format!("{} now does “{}”.", binding.label(), action.label()),
                });
                actions.push(SettingsAction::Save);
            }
        }
    }

    let keymap = Keymap::from_overrides(&config.shortcuts);
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.add(
            egui::TextEdit::singleline(filter)
                .hint_text("Action or key…")
                .desired_width(220.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!config.shortcuts.is_empty(), egui::Button::new("Reset all"))
                .on_hover_text("Restore every default shortcut")
                .clicked()
            {
                config.shortcuts.clear();
                *capturing = None;
                *note = Some("All shortcuts are back to their defaults.".to_owned());
                actions.push(SettingsAction::Save);
            }
        });
    });
    if let Some(text) = note {
        ui.add_space(2.0);
        ui.label(RichText::new(text.as_str()).color(theme::ACCENT));
    }

    let query = filter.trim().to_lowercase();
    let matches = |a: Action| {
        query.is_empty()
            || a.label().to_lowercase().contains(&query)
            || keymap.label(a).to_lowercase().contains(&query)
    };
    for group in Group::ALL {
        let rows: Vec<Action> = Action::ALL
            .iter()
            .copied()
            .filter(|a| a.group() == group && matches(*a))
            .collect();
        if rows.is_empty() {
            continue;
        }
        section(ui, group.label());
        egui::Grid::new(("shortcuts", group.label()))
            .num_columns(3)
            .spacing([12.0, 4.0])
            .min_col_width(0.0)
            .show(ui, |ui| {
                for action in rows {
                    // A fixed-width first column keeps the keys aligned across groups.
                    ui.horizontal(|ui| {
                        ui.set_min_width(240.0);
                        ui.label(action.label());
                    });
                    let waiting = *capturing == Some(action);
                    let text = if waiting {
                        RichText::new("Press keys…").color(theme::ON_ACCENT)
                    } else {
                        match keymap.binding(action) {
                            Some(b) => RichText::new(b.label()).color(theme::ACCENT),
                            None => RichText::new("—").color(theme::TEXT_SECONDARY),
                        }
                    };
                    let mut button = egui::Button::new(text).min_size(egui::vec2(150.0, 22.0));
                    if waiting {
                        button = button.fill(theme::ACCENT);
                    }
                    if ui
                        .add(button)
                        .on_hover_text("Click, then press the new key combination (Esc cancels)")
                        .clicked()
                    {
                        *capturing = if waiting { None } else { Some(action) };
                        *note = None;
                    }
                    ui.horizontal(|ui| {
                        let changed = keymap.binding(action) != action.default_binding();
                        let default = action
                            .default_binding()
                            .map_or("no key".to_owned(), Binding::label);
                        if ui
                            .add_enabled(changed, egui::Button::new("↺").small())
                            .on_hover_text(format!("Restore the default ({default})"))
                            .clicked()
                        {
                            let taken =
                                assign(&mut config.shortcuts, action, action.default_binding());
                            *note = taken.map(|o| {
                                format!("“{}” lost its key to the restored default.", o.label())
                            });
                            actions.push(SettingsAction::Save);
                        }
                        if ui
                            .add_enabled(
                                keymap.binding(action).is_some(),
                                egui::Button::new("×").small(),
                            )
                            .on_hover_text("Remove the key")
                            .clicked()
                        {
                            assign(&mut config.shortcuts, action, None);
                            actions.push(SettingsAction::Save);
                        }
                    });
                    ui.end_row();
                }
            });
    }

    section(ui, "Fixed keys");
    egui::Grid::new("shortcuts-fixed")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (keys, what) in FIXED {
                ui.label(RichText::new(keys).color(theme::TEXT_PRIMARY));
                ui.label(RichText::new(what).color(theme::TEXT_SECONDARY));
                ui.end_row();
            }
        });
    hint(
        ui,
        "Copy, cut and paste come from Windows itself; Esc, Space and the arrow keys have fixed jobs.",
    );
}

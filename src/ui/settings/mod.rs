//! Settings window: General, Updates and Integrations → Discord.

mod discord;
mod general;
mod updates;

use egui::{Color32, RichText, Sense, Vec2};

use crate::config::Config;
use crate::integrations::discord_rpc::ConnectionState;
use crate::services::updater::UpdateStatus;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Updates,
    Discord,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Updates => "Updates",
            SettingsTab::Discord => "Discord",
        }
    }
}

/// What the user asked for in the settings window; executed by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    /// A setting changed and should be saved.
    Save,
    CheckUpdates,
    DownloadUpdate,
    CancelDownload,
    InstallUpdate,
    RestorePrevious,
    DismissUpdateError,
    DiscordRetry,
    DiscordDisconnect,
}

/// Read-only state the window displays.
pub struct SettingsView {
    pub update_status: UpdateStatus,
    pub discord_state: ConnectionState,
    pub discord_configured: bool,
    pub can_install: bool,
    pub has_backup: bool,
}

pub struct SettingsWindow {
    pub open: bool,
    pub tab: SettingsTab,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self {
            open: false,
            tab: SettingsTab::General,
        }
    }
}

impl SettingsWindow {
    pub fn open_at(&mut self, tab: SettingsTab) {
        self.open = true;
        self.tab = tab;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
        view: &SettingsView,
    ) -> Vec<SettingsAction> {
        let mut actions = Vec::new();
        if !self.open {
            return actions;
        }
        let mut open = true;
        egui::Window::new(RichText::new("Settings").strong())
            .id(egui::Id::new("kagero-settings"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size([600.0, 470.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_size(Vec2::new(600.0, 440.0));
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(124.0);
                        for tab in [
                            SettingsTab::General,
                            SettingsTab::Updates,
                            SettingsTab::Discord,
                        ] {
                            let selected = self.tab == tab;
                            let mut text = RichText::new(tab.label()).color(theme::TEXT_PRIMARY);
                            if selected {
                                text = text.strong();
                            }
                            if ui
                                .add_sized(
                                    [120.0, 30.0],
                                    egui::SelectableLabel::new(selected, text),
                                )
                                .clicked()
                            {
                                self.tab = tab;
                            }
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(450.0);
                        egui::ScrollArea::vertical()
                            .max_height(430.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| match self.tab {
                                SettingsTab::General => general::show(ui, config, &mut actions),
                                SettingsTab::Updates => {
                                    updates::show(ui, config, view, &mut actions)
                                }
                                SettingsTab::Discord => {
                                    discord::show(ui, config, view, &mut actions)
                                }
                            });
                    });
                });
            });
        if !open {
            self.open = false;
        }
        actions
    }
}

/// Section heading used on every settings page.
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(2.0);
}

/// Secondary explanatory text.
fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::TEXT_SECONDARY));
}

/// Kagero's switch: a pill toggle followed by its label. Returns `true` when changed.
pub fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let size = Vec2::new(34.0, 18.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        if resp.clicked() {
            *value = !*value;
            changed = true;
        }
        let t = ui.ctx().animate_bool_with_time(resp.id, *value, 0.12);
        let off = theme::SURFACE_HOVER;
        let track = lerp_color(off, theme::ACCENT, t);
        ui.painter().rect_filled(rect, size.y / 2.0, track);
        let knob_x = egui::lerp((rect.left() + 9.0)..=(rect.right() - 9.0), t);
        ui.painter().circle_filled(
            egui::pos2(knob_x, rect.center().y),
            6.5,
            if *value {
                theme::ON_ACCENT
            } else {
                theme::TEXT_PRIMARY
            },
        );
        let text = ui.add(egui::Label::new(label).sense(Sense::click()));
        if text.clicked() {
            *value = !*value;
            changed = true;
        }
    });
    changed
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// Colored status dot followed by text.
fn status_line(ui: &mut egui::Ui, color: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.5, color);
        ui.label(RichText::new(text).color(theme::TEXT_PRIMARY));
    });
}

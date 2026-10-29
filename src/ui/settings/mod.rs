//! Settings window: a side menu of pages (General, Shortcuts, Updates, Discord,
//! Experimental, Plugins, Themes) with a header for the open page.

mod discord;
mod experimental;
mod general;
mod keys;
mod soon;
mod updates;

use egui::{Color32, Rect, RichText, Sense, Vec2};

use super::icons::{paint_icon, Icon, Tint};
use super::keymap::Action;
use crate::config::Config;
use crate::integrations::discord_rpc::ConnectionState;
use crate::services::updater::UpdateStatus;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Shortcuts,
    Updates,
    Discord,
    Experimental,
    Plugins,
    Themes,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Shortcuts => "Shortcuts",
            SettingsTab::Updates => "Updates",
            SettingsTab::Discord => "Discord",
            SettingsTab::Experimental => "Experimental",
            SettingsTab::Plugins => "Plugins",
            SettingsTab::Themes => "Themes",
        }
    }

    fn icon(self) -> Icon {
        match self {
            SettingsTab::General => Icon::Settings,
            SettingsTab::Shortcuts => Icon::Keyboard,
            SettingsTab::Updates => Icon::Download,
            SettingsTab::Discord => Icon::Chat,
            SettingsTab::Experimental => Icon::Flask,
            SettingsTab::Plugins => Icon::Plug,
            SettingsTab::Themes => Icon::Palette,
        }
    }

    fn description(self) -> &'static str {
        match self {
            SettingsTab::General => "Workspace layout and onion skin.",
            SettingsTab::Shortcuts => {
                "Change any keyboard shortcut. Click a key, then press the new one."
            }
            SettingsTab::Updates => "Check for, download and install new versions of Kagero.",
            SettingsTab::Discord => "Show what you're working on in your Discord profile.",
            SettingsTab::Experimental => {
                "Features that are still being tested. Turn them on to try them."
            }
            SettingsTab::Plugins => "Extend Kagero with new tools, formats and effects.",
            SettingsTab::Themes => "Change how Kagero looks.",
        }
    }

    fn is_soon(self) -> bool {
        matches!(self, SettingsTab::Plugins | SettingsTab::Themes)
    }
}

/// The side menu, in groups.
const MENU: [(&str, &[SettingsTab]); 3] = [
    (
        "App",
        &[
            SettingsTab::General,
            SettingsTab::Shortcuts,
            SettingsTab::Updates,
        ],
    ),
    ("Integrations", &[SettingsTab::Discord]),
    (
        "Labs",
        &[
            SettingsTab::Experimental,
            SettingsTab::Plugins,
            SettingsTab::Themes,
        ],
    ),
];

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
    /// The shortcut waiting for a key press (Shortcuts page).
    capturing: Option<Action>,
    /// Search text on the Shortcuts page.
    filter: String,
    /// Last note on the Shortcuts page ("Ctrl+P was taken from Fill").
    note: Option<String>,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self {
            open: false,
            tab: SettingsTab::General,
            capturing: None,
            filter: String::new(),
            note: None,
        }
    }
}

impl SettingsWindow {
    pub fn open_at(&mut self, tab: SettingsTab) {
        self.open = true;
        self.tab = tab;
    }

    /// While a shortcut waits for its new key, the app must not run shortcuts.
    pub fn is_capturing(&self) -> bool {
        self.open && self.capturing.is_some()
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
        view: &SettingsView,
    ) -> Vec<SettingsAction> {
        let mut actions = Vec::new();
        if !self.open {
            self.capturing = None;
            return actions;
        }
        let mut open = true;
        let height = (ctx.screen_rect().height() - 140.0).clamp(360.0, 560.0);
        egui::Window::new(RichText::new("Settings").strong())
            .id(egui::Id::new("kagero-settings"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(780.0);
                ui.set_height(height);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        self.menu(ui);
                    });
                    ui.add_space(6.0);
                    let x = ui.cursor().min.x;
                    ui.painter().vline(
                        x,
                        ui.max_rect().y_range(),
                        egui::Stroke::new(1.0_f32, theme::BORDER),
                    );
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        page_header(ui, self.tab);
                        egui::ScrollArea::vertical()
                            .id_salt(("settings-page", self.tab.label()))
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.tab {
                                SettingsTab::General => general::show(ui, config, &mut actions),
                                SettingsTab::Shortcuts => keys::show(
                                    ui,
                                    config,
                                    &mut self.capturing,
                                    &mut self.filter,
                                    &mut self.note,
                                    &mut actions,
                                ),
                                SettingsTab::Updates => {
                                    updates::show(ui, config, view, &mut actions)
                                }
                                SettingsTab::Discord => {
                                    discord::show(ui, config, view, &mut actions)
                                }
                                SettingsTab::Experimental => {
                                    experimental::show(ui, config, &mut actions)
                                }
                                SettingsTab::Plugins => soon::plugins(ui),
                                SettingsTab::Themes => soon::themes(ui),
                            });
                    });
                });
            });
        if !open {
            self.open = false;
            self.capturing = None;
        }
        actions
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        for (group, tabs) in MENU {
            ui.add_space(6.0);
            ui.label(
                RichText::new(group.to_uppercase())
                    .small()
                    .color(theme::TEXT_SECONDARY),
            );
            ui.add_space(2.0);
            for &tab in tabs {
                if menu_row(ui, tab, self.tab == tab).clicked() && self.tab != tab {
                    self.tab = tab;
                    self.capturing = None;
                    self.note = None;
                }
            }
        }
    }
}

/// One row of the side menu: icon, label and a "Soon" tag for planned pages.
fn menu_row(ui: &mut egui::Ui, tab: SettingsTab, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(166.0, 30.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, theme::EMBER_DEEP);
        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(bar, 0.0, theme::ACCENT);
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, theme::SURFACE_HOVER);
    }
    let icon = Rect::from_center_size(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        Vec2::splat(16.0),
    );
    paint_icon(painter, icon, tab.icon(), Tint::Normal);
    painter.text(
        egui::pos2(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        tab.label(),
        egui::TextStyle::Body.resolve(ui.style()),
        if selected {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        },
    );
    if tab.is_soon() {
        let tag = egui::pos2(rect.right() - 8.0, rect.center().y);
        painter.text(
            tag,
            egui::Align2::RIGHT_CENTER,
            "SOON",
            egui::TextStyle::Small.resolve(ui.style()),
            theme::ACCENT.gamma_multiply(0.8),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Big icon, title and one-line description of the open page.
fn page_header(ui: &mut egui::Ui, tab: SettingsTab) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
        paint_icon(ui.painter(), rect, tab.icon(), Tint::Normal);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                RichText::new(tab.label())
                    .font(theme::font(2))
                    .color(theme::TEXT_PRIMARY),
            );
            ui.label(RichText::new(tab.description()).color(theme::TEXT_SECONDARY));
        });
    });
    ui.add_space(4.0);
    ui.separator();
}

/// Section heading used on every settings page.
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(theme::ACCENT),
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

//! Settings window: a side menu of pages (General, Shortcuts, Updates, Discord,
//! Experimental, Plugins, Themes) with a header for the open page.

mod discord;
mod experimental;
mod general;
mod keys;
mod plugins;
mod themes;
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
            SettingsTab::Plugins => "Add commands written in Lua, run safely in a sandbox.",
            SettingsTab::Themes => "Change Kagero's colors, or make your own theme.",
        }
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
    OpenRecoveryFolder,
    /// Show a theme on the whole window without saving it yet.
    PreviewTheme(String),
    KeepTheme,
    RevertTheme,
    ImportTheme,
    ExportTheme(String),
    DeleteTheme(String),
    OpenThemesFolder,
    ReloadThemes,
    OpenPluginsFolder,
    ReloadPlugins,
    InstallExamplePlugin,
}

/// Read-only state the window displays.
pub struct SettingsView {
    pub update_status: UpdateStatus,
    pub discord_state: ConnectionState,
    pub discord_configured: bool,
    pub can_install: bool,
    pub has_backup: bool,
    /// Every theme (built-in first), the saved one, and the one being previewed.
    pub themes: Vec<crate::themes::Theme>,
    pub current_theme: String,
    pub theme_preview: Option<String>,
    /// Theme files that couldn't be used, and why.
    pub theme_problems: Vec<(String, String)>,
    /// Plugin folders found (usable or not).
    pub plugins: Vec<crate::plugins::Plugin>,
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
                        egui::Stroke::new(1.0_f32, theme::border()),
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
                                SettingsTab::Plugins => {
                                    plugins::show(ui, config, view, &mut actions)
                                }
                                SettingsTab::Themes => themes::show(ui, view, &mut actions),
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
                    .color(theme::text_secondary()),
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

/// One row of the side menu: icon and label.
fn menu_row(ui: &mut egui::Ui, tab: SettingsTab, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(166.0, 30.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, theme::ember_deep());
        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(bar, 0.0, theme::accent());
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, theme::surface_hover());
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
            theme::text_primary()
        } else {
            theme::text_secondary()
        },
    );
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
                    .color(theme::text_primary()),
            );
            ui.label(RichText::new(tab.description()).color(theme::text_secondary()));
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
            .color(theme::accent()),
    );
    ui.add_space(2.0);
}

/// Secondary explanatory text.
fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::text_secondary()));
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
        let off = theme::surface_hover();
        let track = lerp_color(off, theme::accent(), t);
        ui.painter().rect_filled(rect, size.y / 2.0, track);
        let knob_x = egui::lerp((rect.left() + 9.0)..=(rect.right() - 9.0), t);
        ui.painter().circle_filled(
            egui::pos2(knob_x, rect.center().y),
            6.5,
            if *value {
                theme::on_accent()
            } else {
                theme::text_primary()
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
        ui.label(RichText::new(text).color(theme::text_primary()));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_renders_including_themes_and_plugins() {
        let dir = std::env::temp_dir().join(format!("kagero-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = crate::plugins::PluginStore::load(Some(dir.clone()));
        store.install_example().expect("example");
        let view = SettingsView {
            update_status: UpdateStatus::Idle,
            discord_state: ConnectionState::Disabled,
            discord_configured: false,
            can_install: false,
            has_backup: false,
            themes: crate::themes::builtins(),
            current_theme: crate::themes::DEFAULT_THEME.to_owned(),
            theme_preview: Some("Paper".to_owned()),
            theme_problems: vec![("bad.json".into(), "the color \"panel\" is missing".into())],
            plugins: store.plugins.clone(),
        };
        let mut config = Config::default();
        let mut window = SettingsWindow::default();
        let ctx = egui::Context::default();
        for (_, tabs) in MENU {
            for &tab in tabs {
                window.open_at(tab);
                for _ in 0..4 {
                    let _ = ctx.run(egui::RawInput::default(), |ctx| {
                        let actions = window.show(ctx, &mut config, &view);
                        assert!(actions.is_empty(), "{tab:?}: {actions:?}");
                    });
                }
                // No page may stretch the window beyond its designed width.
                let width = ctx
                    .memory(|m| m.area_rect(egui::Id::new("kagero-settings")))
                    .map_or(0.0, |r| r.width());
                assert!(
                    width > 0.0 && width < 850.0,
                    "{tab:?}: window is {width} wide"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Kagero's visual system: a dark workshop lit by warm amber.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

pub const BACKGROUND: Color32 = Color32::from_rgb(0x17, 0x19, 0x1F);
pub const PANEL: Color32 = Color32::from_rgb(0x20, 0x23, 0x2B);
pub const SURFACE: Color32 = Color32::from_rgb(0x2C, 0x30, 0x3A);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(0x37, 0x3C, 0x48);
pub const BORDER: Color32 = Color32::from_rgb(0x34, 0x38, 0x43);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xA5, 0xA9, 0xB5);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF0, 0xEE, 0xE8);
pub const ACCENT: Color32 = Color32::from_rgb(0xFF, 0x9D, 0x42);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0xF0, 0x6B, 0x35);
pub const SUCCESS: Color32 = Color32::from_rgb(0x75, 0xC7, 0xB5);
/// Deep ember used behind light text for pressed and selected states (readable contrast).
pub const EMBER_DEEP: Color32 = Color32::from_rgb(0x6B, 0x36, 0x1C);
pub const ERROR: Color32 = Color32::from_rgb(0xE8, 0x5D, 0x5D);
/// Text drawn on top of the accent color.
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x1A, 0x14, 0x10);
/// Two shades of the transparency checkerboard.
pub const CHECKER_LIGHT: Color32 = Color32::from_rgb(0x4A, 0x4E, 0x58);
pub const CHECKER_DARK: Color32 = Color32::from_rgb(0x3A, 0x3D, 0x46);

pub const RADIUS: u8 = 4;

/// Applies the Kagero style to an egui context.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut visuals = Visuals::dark();
    visuals.override_text_color = None;
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = SURFACE;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = EMBER_DEEP;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.error_fg_color = ERROR;
    visuals.warn_fg_color = ACCENT;

    let radius = CornerRadius::same(RADIUS);
    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = SURFACE;
    w.inactive.weak_bg_fill = SURFACE;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT.linear_multiply(0.6));
    w.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    w.hovered.corner_radius = radius;

    w.active.bg_fill = EMBER_DEEP;
    w.active.weak_bg_fill = EMBER_DEEP;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    // egui also uses this color for `strong()` text, so it must stay light.
    w.active.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    w.active.corner_radius = radius;

    w.open.bg_fill = SURFACE_HOVER;
    w.open.weak_bg_fill = SURFACE_HOVER;
    w.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.open.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    w.open.corner_radius = radius;

    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.window_margin = Margin::same(14);
        style.spacing.interact_size.y = 24.0;
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(20.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(11.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}

/// Adds a system Japanese font as a fallback so "陽炎" renders on the home screen.
/// The font is read from the local Windows installation and never redistributed.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
        r"C:\Windows\Fonts\meiryo.ttc",
    ];
    for path in candidates {
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        if meta.len() > 64 * 1024 * 1024 {
            continue;
        }
        match std::fs::read(path) {
            Ok(bytes) => {
                fonts.font_data.insert(
                    "jp-fallback".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                for family in [FontFamily::Proportional, FontFamily::Monospace] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push("jp-fallback".to_owned());
                }
                break;
            }
            Err(e) => tracing::debug!("font {path} not loaded: {e}"),
        }
    }
    ctx.set_fonts(fonts);
}

//! Settings → Plugins and Themes: planned pages that say what's coming.

use egui::{RichText, Vec2};

use crate::theme;
use crate::ui::icons::{paint_icon, Icon, Tint};

pub fn plugins(ui: &mut egui::Ui) {
    coming_soon(
        ui,
        Icon::Plug,
        "Plugins are coming soon",
        &[
            "New tools and brushes",
            "Import and export formats",
            "Image effects and batch actions",
        ],
    );
}

pub fn themes(ui: &mut egui::Ui) {
    coming_soon(
        ui,
        Icon::Palette,
        "Themes are coming soon",
        &[
            "Light and dark color schemes",
            "Accent colors and canvas backgrounds",
            "Your own themes, shared as files",
        ],
    );
}

fn coming_soon(ui: &mut egui::Ui, icon: Icon, title: &str, plans: &[&str]) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(72.0), egui::Sense::hover());
        paint_icon(ui.painter(), rect, icon, Tint::Normal);
        ui.add_space(10.0);
        // A badge as wide as its text (a Frame here would fill the centered column).
        let galley = ui.painter().layout_no_wrap(
            "SOON".to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            theme::TEXT_PRIMARY,
        );
        let (badge, _) =
            ui.allocate_exact_size(galley.size() + Vec2::new(16.0, 4.0), egui::Sense::hover());
        ui.painter().rect_filled(badge, 0.0, theme::EMBER_DEEP);
        ui.painter().rect_stroke(
            badge,
            0.0,
            egui::Stroke::new(1.0_f32, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
        ui.painter().galley(
            badge.center() - galley.size() / 2.0,
            galley,
            theme::TEXT_PRIMARY,
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .font(theme::font(2))
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(6.0);
        ui.label(RichText::new("Planned:").color(theme::TEXT_SECONDARY));
        for plan in plans {
            ui.label(RichText::new(format!("· {plan}")).color(theme::TEXT_SECONDARY));
        }
    });
}

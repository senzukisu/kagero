//! Start screen: create, open, and recent projects.

use std::path::PathBuf;

use egui::{Align, Button, Frame, Layout, Margin, RichText, Vec2};

use super::icons::{paint_bitmap, LOGO};
use super::Command;
use crate::theme;

/// Height of the card's content area; the recent list scrolls inside it.
const CONTENT_HEIGHT: f32 = 360.0;

pub fn show(ui: &mut egui::Ui, recent: &[PathBuf], commands: &mut Vec<Command>) {
    let avail = ui.available_size();
    let card_width = 760.0_f32.min(avail.x - 32.0).max(320.0);
    // Card (content + margins) plus the footer line below it.
    let total_height = CONTENT_HEIGHT + 56.0 + 40.0;
    ui.add_space(((avail.y - total_height) / 2.0).max(12.0));
    ui.vertical_centered(|ui| {
        ui.set_max_width(card_width);
        Frame::new()
            .fill(theme::PANEL)
            .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
            .corner_radius(8)
            .inner_margin(Margin::same(28))
            .show(ui, |ui| {
                ui.set_width(card_width - 56.0);
                ui.horizontal_top(|ui| {
                    let left_width = 280.0_f32.min(ui.available_width() / 2.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, CONTENT_HEIGHT),
                        Layout::top_down(Align::Min),
                        |ui| brand_column(ui, commands),
                    );
                    ui.add_space(16.0);
                    // Fixed-height divider: `ui.separator()` would stretch to the window.
                    let (line, _) = ui
                        .allocate_exact_size(Vec2::new(1.0, CONTENT_HEIGHT), egui::Sense::hover());
                    ui.painter().rect_filled(line, 0.0, theme::BORDER);
                    ui.add_space(16.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), CONTENT_HEIGHT),
                        Layout::top_down(Align::Min),
                        |ui| recent_column(ui, recent, commands),
                    );
                });
            });
        ui.add_space(14.0);
        ui.label(
            RichText::new(format!(
                "{} · Kagero {} {} · MIT License",
                crate::STUDIO_NAME,
                crate::APP_VERSION,
                crate::APP_CHANNEL
            ))
            .small()
            .color(theme::TEXT_SECONDARY),
        );
    });
}

fn brand_column(ui: &mut egui::Ui, commands: &mut Vec<Command>) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(96.0), egui::Sense::hover());
    paint_bitmap(
        ui.painter(),
        rect,
        &LOGO,
        theme::TEXT_PRIMARY,
        theme::ACCENT,
    );
    ui.add_space(6.0);
    ui.label(
        RichText::new("Kagero")
            .size(40.0)
            .strong()
            .color(theme::TEXT_PRIMARY),
    );
    ui.label(
        RichText::new("陽炎 · pixel art editor")
            .size(15.0)
            .color(theme::ACCENT),
    );
    ui.label(
        RichText::new(format!("by {}", crate::STUDIO_NAME))
            .small()
            .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(24.0);

    let size = Vec2::new(220.0, 36.0);
    let create = Button::new(RichText::new("Create New").strong().color(theme::ON_ACCENT))
        .fill(theme::ACCENT)
        .min_size(size);
    if ui.add(create).on_hover_text("Ctrl+N").clicked() {
        commands.push(Command::NewDocument);
    }
    ui.add_space(4.0);
    let open = Button::new(RichText::new("Open Project").strong()).min_size(size);
    if ui
        .add(open)
        .on_hover_text("Open a .kgr project or a .png image (Ctrl+O)")
        .clicked()
    {
        commands.push(Command::Open);
    }
}

fn recent_column(ui: &mut egui::Ui, recent: &[PathBuf], commands: &mut Vec<Command>) {
    ui.label(
        RichText::new("RECENT PROJECTS")
            .small()
            .strong()
            .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(6.0);
    if recent.is_empty() {
        ui.label(
            RichText::new("Projects you save or open will appear here.")
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .max_height(330.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for path in recent {
                let exists = path.exists();
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let dir = path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    let title_color = if exists {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    };
                    let text = RichText::new(&name).color(title_color);
                    let resp = ui
                        .add_enabled(exists, Button::new(text).frame(false))
                        .on_hover_text(path.display().to_string())
                        .on_disabled_hover_text("File not found");
                    if resp.clicked() {
                        commands.push(Command::OpenPath(path.clone()));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button("×")
                            .on_hover_text("Remove from the list")
                            .clicked()
                        {
                            commands.push(Command::RemoveRecent(path.clone()));
                        }
                    });
                });
                let sub = if exists {
                    dir
                } else {
                    format!("Missing · {dir}")
                };
                ui.label(RichText::new(sub).small().color(theme::TEXT_SECONDARY));
                ui.add_space(6.0);
            }
        });
}

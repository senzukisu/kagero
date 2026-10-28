//! Modal dialogs: new document, unsaved changes, errors, about.

use egui::{Button, Id, Modal, RichText, Vec2};

use crate::core::{Rgba, MAX_CANVAS_SIZE};
use crate::theme;

/// Background of a new document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    Transparent,
    White,
    Black,
}

impl Background {
    pub fn color(self) -> Rgba {
        match self {
            Background::Transparent => Rgba::TRANSPARENT,
            Background::White => Rgba::WHITE,
            Background::Black => Rgba::BLACK,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Background::Transparent => "Transparent",
            Background::White => "White",
            Background::Black => "Black",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocumentState {
    pub width: u32,
    pub height: u32,
    pub background: Background,
}

pub enum NewDocumentResult {
    Open,
    Create,
    Cancel,
}

const PRESETS: [(u32, u32); 6] = [
    (16, 16),
    (32, 32),
    (64, 64),
    (128, 128),
    (256, 256),
    (320, 180),
];

pub fn new_document(ctx: &egui::Context, state: &mut NewDocumentState) -> NewDocumentResult {
    let mut result = NewDocumentResult::Open;
    let modal = Modal::new(Id::new("new-document")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading("New Image");
        ui.add_space(8.0);
        egui::Grid::new("new-doc-grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Width");
                ui.add(
                    egui::DragValue::new(&mut state.width)
                        .range(1..=MAX_CANVAS_SIZE)
                        .suffix(" px"),
                );
                ui.end_row();
                ui.label("Height");
                ui.add(
                    egui::DragValue::new(&mut state.height)
                        .range(1..=MAX_CANVAS_SIZE)
                        .suffix(" px"),
                );
                ui.end_row();
                ui.label("Background");
                egui::ComboBox::from_id_salt("new-doc-bg")
                    .selected_text(state.background.name())
                    .show_ui(ui, |ui| {
                        for bg in [
                            Background::Transparent,
                            Background::White,
                            Background::Black,
                        ] {
                            ui.selectable_value(&mut state.background, bg, bg.name());
                        }
                    });
                ui.end_row();
            });
        ui.add_space(6.0);
        ui.label(
            RichText::new("Presets")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        ui.horizontal_wrapped(|ui| {
            for (w, h) in PRESETS {
                let selected = state.width == w && state.height == h;
                if ui.selectable_label(selected, format!("{w}×{h}")).clicked() {
                    state.width = w;
                    state.height = h;
                }
            }
        });
        ui.label(
            RichText::new(format!(
                "Any size from 1×1 to {MAX_CANVAS_SIZE}×{MAX_CANVAS_SIZE}."
            ))
            .small()
            .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.add(accent_button("Create")).clicked() {
                result = NewDocumentResult::Create;
            }
            if ui.button("Cancel").clicked() {
                result = NewDocumentResult::Cancel;
            }
        });
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ui.ctx().wants_keyboard_input() {
            result = NewDocumentResult::Create;
        }
    });
    if modal.should_close() && matches!(result, NewDocumentResult::Open) {
        result = NewDocumentResult::Cancel;
    }
    state.width = state.width.clamp(1, MAX_CANVAS_SIZE);
    state.height = state.height.clamp(1, MAX_CANVAS_SIZE);
    result
}

pub enum UnsavedChoice {
    Pending,
    Save,
    Discard,
    Cancel,
}

pub fn unsaved_changes(ctx: &egui::Context, name: &str) -> UnsavedChoice {
    let mut choice = UnsavedChoice::Pending;
    let modal = Modal::new(Id::new("unsaved-changes")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading("Unsaved changes");
        ui.add_space(6.0);
        ui.label(format!("Save changes to “{name}” before continuing?"));
        ui.label(
            RichText::new("Changes that are not saved will be lost.")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.add(accent_button("Save")).clicked() {
                choice = UnsavedChoice::Save;
            }
            if ui.button("Don't Save").clicked() {
                choice = UnsavedChoice::Discard;
            }
            if ui.button("Cancel").clicked() {
                choice = UnsavedChoice::Cancel;
            }
        });
    });
    if modal.should_close() && matches!(choice, UnsavedChoice::Pending) {
        choice = UnsavedChoice::Cancel;
    }
    choice
}

pub enum ConfirmChoice {
    Pending,
    Confirm,
    Cancel,
}

/// A yes/no question with a destructive or important confirm action.
pub fn confirm(
    ctx: &egui::Context,
    title: &str,
    message: &str,
    confirm_label: &str,
) -> ConfirmChoice {
    let mut choice = ConfirmChoice::Pending;
    let modal = Modal::new(Id::new(("confirm-dialog", title))).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading(title);
        ui.add_space(6.0);
        ui.label(message);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.add(accent_button(confirm_label)).clicked() {
                choice = ConfirmChoice::Confirm;
            }
            if ui.button("Cancel").clicked() {
                choice = ConfirmChoice::Cancel;
            }
        });
    });
    if modal.should_close() && matches!(choice, ConfirmChoice::Pending) {
        choice = ConfirmChoice::Cancel;
    }
    choice
}

/// Shows an error; returns `true` when dismissed.
pub fn error(ctx: &egui::Context, title: &str, message: &str) -> bool {
    let mut close = false;
    let modal = Modal::new(Id::new("error-dialog")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(RichText::new(title).heading().color(theme::ERROR));
        ui.add_space(6.0);
        ui.label(message);
        ui.add_space(12.0);
        if ui.add(accent_button("OK")).clicked() {
            close = true;
        }
    });
    close || modal.should_close()
}

/// Shows the About box; returns `true` when dismissed.
pub fn about(ctx: &egui::Context) -> bool {
    let mut close = false;
    let modal = Modal::new(Id::new("about-dialog"))
        .frame(
            egui::Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0_f32, theme::BEVEL_LIGHT))
                .shadow(egui::Shadow {
                    offset: [6, 6],
                    blur: 0,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(170),
                })
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.spacing_mut().item_spacing.y = 4.0;
            about_banner(ui);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ui.label(format!(
                        "An independent pixel-art and sprite-animation editor by {}.",
                        crate::STUDIO_NAME
                    ));
                    ui.add_space(6.0);
                    let repo = format!(
                        "https://github.com/{}/{}",
                        crate::release::GITHUB_OWNER,
                        crate::release::GITHUB_REPO
                    );
                    egui::Grid::new("about-facts")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            fact(ui, "Studio", |ui| {
                                ui.label(crate::STUDIO_NAME);
                            });
                            fact(ui, "License", |ui| {
                                ui.label("MIT (dependencies: THIRD_PARTY_LICENSES.md)");
                            });
                            fact(ui, "Made with", |ui| {
                                ui.label("Rust, egui and the Kagero Pixel font");
                            });
                            fact(ui, "Links", |ui| {
                                ui.hyperlink_to(
                                    "Documentation",
                                    "https://kagero.gitbook.io/kagero-docs",
                                );
                                ui.label(RichText::new("·").color(theme::TEXT_SECONDARY));
                                ui.hyperlink_to(
                                    "Changelog",
                                    format!("{repo}/blob/main/CHANGELOG.md"),
                                );
                                ui.label(RichText::new("·").color(theme::TEXT_SECONDARY));
                                ui.hyperlink_to("GitHub", repo.clone());
                            });
                        });

                    ui.add_space(12.0);
                    ui.label(RichText::new("SHORTCUTS").color(theme::TEXT_SECONDARY));
                    ui.add_space(2.0);
                    egui::Grid::new("about-shortcuts")
                        .num_columns(4)
                        .spacing([10.0, 5.0])
                        .show(ui, |ui| {
                            let rows = [
                                ("B E G I", "Draw, erase, fill, pick", "M", "Select"),
                                ("L U", "Line, rectangle", "Ctrl+C Ctrl+V", "Copy, paste"),
                                ("Space", "Pan (hold)", "Ctrl+Z Ctrl+Y", "Undo, redo"),
                                ("0 1", "Fit, 100%", "Ctrl+Shift+E", "Export"),
                                ("X", "Swap colors", "Enter", "Play / stop"),
                                ("O", "Onion skin", "Alt+N Alt+D", "New, duplicate frame"),
                            ];
                            for (k1, v1, k2, v2) in rows {
                                keycaps(ui, k1);
                                ui.label(RichText::new(v1).color(theme::TEXT_SECONDARY));
                                keycaps(ui, k2);
                                ui.label(RichText::new(v2).color(theme::TEXT_SECONDARY));
                                ui.end_row();
                            }
                        });

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("© 2026 {}", crate::STUDIO_NAME))
                                .color(theme::TEXT_SECONDARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(accent_button("Close")).clicked() {
                                close = true;
                            }
                        });
                    });
                });
        });
    close || modal.should_close()
}

/// The dark banner at the top of About: flame, name and version badge.
fn about_banner(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(theme::BACKGROUND)
        .inner_margin(egui::Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());
                super::icons::paint_bitmap(
                    ui.painter(),
                    rect,
                    &super::icons::LOGO,
                    super::icons::Tint::Normal,
                );
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("Kagero")
                            .font(theme::font(3))
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.label(RichText::new("陽炎 · pixel art editor").color(theme::ACCENT));
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme::EMBER_DEEP)
                        .stroke(egui::Stroke::new(1.0_f32, theme::ACCENT))
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "VERSION {} {}",
                                    crate::APP_VERSION,
                                    crate::APP_CHANNEL.to_uppercase()
                                ))
                                .color(theme::TEXT_PRIMARY),
                            );
                        });
                });
            });
        });
    // A 2-pixel ember rule between the banner and the body.
    let (line, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, theme::ACCENT_PRESSED);
}

fn fact(ui: &mut egui::Ui, name: &str, value: impl FnOnce(&mut egui::Ui)) {
    ui.label(RichText::new(name).color(theme::TEXT_SECONDARY));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        value(ui);
    });
    ui.end_row();
}

/// Space-separated keys drawn as small bevelled keycaps.
fn keycaps(ui: &mut egui::Ui, keys: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for key in keys.split(' ') {
            let resp = egui::Frame::new()
                .fill(theme::SURFACE)
                .stroke(egui::Stroke::new(1.0_f32, theme::BEVEL_DARK))
                .inner_margin(egui::Margin::symmetric(5, 1))
                .show(ui, |ui| {
                    ui.label(RichText::new(key).color(theme::ACCENT));
                });
            super::icons::bevel(ui.painter(), resp.response.rect.shrink(1.0), true);
        }
    });
}

/// The primary call-to-action button style.
pub fn accent_button(text: &str) -> Button<'static> {
    Button::new(
        RichText::new(text.to_owned())
            .strong()
            .color(theme::ON_ACCENT),
    )
    .fill(theme::ACCENT)
    .min_size(Vec2::new(88.0, 28.0))
}

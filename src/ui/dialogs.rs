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
    let modal = Modal::new(Id::new("about-dialog")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), egui::Sense::hover());
            super::icons::paint_bitmap(
                ui.painter(),
                rect,
                &super::icons::LOGO,
                theme::TEXT_PRIMARY,
                theme::ACCENT,
            );
            ui.vertical(|ui| {
                ui.label(RichText::new("Kagero").size(24.0).strong());
                ui.label(
                    RichText::new(format!("Version {} Alpha", crate::APP_VERSION))
                        .color(theme::TEXT_SECONDARY),
                );
            });
        });
        ui.add_space(8.0);
        ui.label(format!(
            "An independent pixel-art editor by {}.",
            crate::STUDIO_NAME
        ));
        ui.label(
            RichText::new(
                "Source code under the MIT License. See THIRD_PARTY_LICENSES.md for dependencies.",
            )
            .small()
            .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(8.0);
        ui.label(RichText::new("Shortcuts").strong());
        egui::Grid::new("about-shortcuts")
            .num_columns(2)
            .show(ui, |ui| {
                for (k, v) in [
                    ("B / E / G / I", "Pencil / Eraser / Fill / Eyedropper"),
                    ("L / U / Shift+U / H", "Line / Rectangle / Filled / Pan"),
                    ("Space + drag", "Pan the canvas"),
                    ("Wheel, + / −", "Zoom"),
                    ("Home / 0 / 1", "Center / Fit / 100%"),
                    ("X", "Swap colors"),
                    ("Ctrl+Z / Ctrl+Y", "Undo / Redo"),
                ] {
                    ui.label(RichText::new(k).monospace().color(theme::ACCENT));
                    ui.label(v);
                    ui.end_row();
                }
            });
        ui.add_space(12.0);
        if ui.add(accent_button("Close")).clicked() {
            close = true;
        }
    });
    close || modal.should_close()
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

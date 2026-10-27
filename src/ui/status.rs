//! Bottom status bar.

use std::time::{Duration, Instant};

use egui::{RichText, Sense, Vec2};

use crate::editor::Editor;
use crate::theme;

/// How long informational messages stay visible.
const MESSAGE_TTL: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub kind: MessageKind,
    pub at: Instant,
}

impl StatusMessage {
    pub fn new(kind: MessageKind, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind,
            at: Instant::now(),
        }
    }

    /// Errors stay until replaced; other messages fade after a few seconds.
    pub fn is_visible(&self) -> bool {
        self.kind == MessageKind::Error || self.at.elapsed() < MESSAGE_TTL
    }
}

pub fn show(
    ui: &mut egui::Ui,
    editor: Option<&Editor>,
    hover: Option<(i32, i32)>,
    zoom: f32,
    message: Option<&StatusMessage>,
) {
    ui.horizontal_centered(|ui| {
        let secondary = |t: String| RichText::new(t).small().color(theme::TEXT_SECONDARY);
        if let Some(editor) = editor {
            let doc = editor.document();
            ui.label(secondary(editor.tool.name().to_owned()));
            ui.separator();
            let pos = hover.map_or("—".to_owned(), |(x, y)| format!("{x}, {y}"));
            ui.label(secondary(format!("Cursor {pos}")));
            ui.separator();
            ui.label(secondary(format!("{} × {} px", doc.width(), doc.height())));
            ui.separator();
            ui.label(secondary(format!("Zoom {:.0}%", zoom * 100.0)));
            ui.separator();
            ui.label(secondary(format!(
                "Layer {}/{}: {}",
                doc.active_index() + 1,
                doc.layer_count(),
                doc.active_layer().name
            )));
        } else {
            ui.label(secondary("No document open".to_owned()));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(editor) = editor {
                let (color, text) = if editor.is_modified() {
                    (theme::ACCENT, "Unsaved changes")
                } else {
                    (theme::SUCCESS, "Saved")
                };
                ui.label(RichText::new(text).small().color(color));
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, color);
                ui.separator();
            }
            if let Some(msg) = message.filter(|m| m.is_visible()) {
                let color = match msg.kind {
                    MessageKind::Info => theme::TEXT_PRIMARY,
                    MessageKind::Success => theme::SUCCESS,
                    MessageKind::Error => theme::ERROR,
                };
                ui.label(RichText::new(&msg.text).small().color(color));
            }
        });
    });
}

//! Timeline: playback controls, frame operations and a strip of frame thumbnails.
//!
//! Thumbnails are cached per frame and rebuilt only when that frame's pixels or the
//! layer structure changed, a few per UI frame at most, and only for visible cards.

use std::collections::{BTreeSet, HashMap};

use egui::{Color32, ColorImage, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, Vec2};

use super::icons::{icon_button, Icon};
use super::Command;
use crate::config::{PlaybackSettings, PlaybackTiming, FPS_RANGE};
use crate::core::animation::{MAX_FRAME_DURATION_MS, MIN_FRAME_DURATION_MS};
use crate::core::onion_skin::{OnionSkin, MAX_OPACITY, MIN_OPACITY};
use crate::core::{Document, FrameId};
use crate::editor::Editor;
use crate::theme;

/// Height of the controls row; also the whole panel height when collapsed.
pub const HEADER_HEIGHT: f32 = 38.0;
const CARD_WIDTH: f32 = 64.0;
const THUMB_BOX: f32 = 50.0;
/// Largest thumbnail texture side in pixels.
const THUMB_PIXELS: u32 = 64;
/// How many thumbnails may be rebuilt per UI frame.
const THUMB_BUILD_BUDGET: usize = 12;

struct Thumb {
    key: (u64, u64),
    texture: TextureHandle,
}

#[derive(Default)]
pub struct Timeline {
    thumbs: HashMap<FrameId, Thumb>,
    /// Duration being edited (frame, value) — committed when editing ends.
    duration_edit: Option<(FrameId, u32)>,
    /// Frames picked with Ctrl/Shift+click (for exporting selected frames).
    selected: BTreeSet<FrameId>,
}

/// Settings the timeline edits in place (saved by the app on `SettingsChanged`).
pub struct TimelineSettings<'a> {
    pub onion: &'a mut OnionSkin,
    pub playback: &'a mut PlaybackSettings,
    pub expanded: bool,
}

impl Timeline {
    /// Document positions of frames selected for export, in order.
    pub fn selected_indices(&self, doc: &crate::core::Document) -> Vec<usize> {
        doc.frames()
            .iter()
            .enumerate()
            .filter(|(_, f)| self.selected.contains(&f.id))
            .map(|(i, _)| i)
            .collect()
    }

    /// Drops cached thumbnails (e.g. when another document opens).
    pub fn clear(&mut self) {
        self.thumbs.clear();
        self.duration_edit = None;
        self.selected.clear();
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        playing: bool,
        settings: TimelineSettings<'_>,
        commands: &mut Vec<Command>,
    ) {
        let doc = editor.document();
        self.thumbs
            .retain(|id, _| doc.frames().iter().any(|f| f.id == *id));
        self.selected
            .retain(|id| doc.frames().iter().any(|f| f.id == *id));

        ui.allocate_ui(Vec2::new(ui.available_width(), HEADER_HEIGHT - 6.0), |ui| {
            ui.horizontal_centered(|ui| {
                self.header(ui, editor, playing, settings, commands);
            });
        });
        if ui.available_height() < 40.0 {
            return;
        }
        ui.separator();
        if doc.frame_count() == 0 {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("This document has no frames.").color(theme::TEXT_SECONDARY),
                );
                if ui.button("Create the first frame").clicked() {
                    commands.push(Command::AddFrame);
                }
            });
            return;
        }
        self.strip(ui, editor, playing, commands);
    }

    fn header(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        playing: bool,
        settings: TimelineSettings<'_>,
        commands: &mut Vec<Command>,
    ) {
        let doc = editor.document();
        let active = doc.active_frame_index();
        let count = doc.frame_count();
        let editable = !playing;
        ui.spacing_mut().item_spacing.x = 3.0;

        ui.label(
            RichText::new("TIMELINE")
                .small()
                .strong()
                .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(6.0);
        if icon_button(
            ui,
            Icon::FirstFrame,
            28.0,
            false,
            true,
            "First frame (Ctrl+Home)",
        )
        .clicked()
        {
            commands.push(Command::FirstFrame);
        }
        if icon_button(
            ui,
            Icon::PrevFrame,
            28.0,
            false,
            true,
            "Previous frame (, or Left)",
        )
        .clicked()
        {
            commands.push(Command::PrevFrame);
        }
        let (icon, tip) = if playing {
            (Icon::Stop, "Stop preview (Enter)")
        } else {
            (Icon::Play, "Play preview (Enter)")
        };
        if icon_button(ui, icon, 28.0, playing, count > 1 || playing, tip).clicked() {
            commands.push(Command::TogglePlayback);
        }
        if icon_button(
            ui,
            Icon::NextFrame,
            28.0,
            false,
            true,
            "Next frame (. or Right)",
        )
        .clicked()
        {
            commands.push(Command::NextFrame);
        }
        ui.label(
            RichText::new(format!("{} / {}", active + 1, count))
                .monospace()
                .color(theme::TEXT_PRIMARY),
        )
        .on_hover_text("Current frame / total frames");

        ui.separator();
        let can_add = doc.can_add_frame().is_ok() && editable;
        if icon_button(
            ui,
            Icon::Plus,
            28.0,
            false,
            can_add,
            "New empty frame (Alt+N)",
        )
        .clicked()
        {
            commands.push(Command::AddFrame);
        }
        if icon_button(
            ui,
            Icon::Duplicate,
            28.0,
            false,
            can_add,
            "Duplicate frame (Alt+D)",
        )
        .clicked()
        {
            commands.push(Command::DuplicateFrame);
        }
        if icon_button(
            ui,
            Icon::Trash,
            28.0,
            false,
            editable && count > 1,
            "Delete frame",
        )
        .clicked()
        {
            commands.push(Command::DeleteFrame(active));
        }
        if icon_button(
            ui,
            Icon::MoveLeft,
            28.0,
            false,
            editable && active > 0,
            "Move frame left",
        )
        .clicked()
        {
            commands.push(Command::MoveFrameLeft(active));
        }
        if icon_button(
            ui,
            Icon::MoveRight,
            28.0,
            false,
            editable && active + 1 < count,
            "Move frame right",
        )
        .clicked()
        {
            commands.push(Command::MoveFrameRight(active));
        }

        ui.separator();
        self.duration_field(ui, doc, editable, commands);

        ui.separator();
        let onion = settings.onion;
        if icon_button(
            ui,
            Icon::Onion,
            28.0,
            onion.enabled,
            true,
            "Onion skin: show neighbor frames (O)",
        )
        .clicked()
        {
            commands.push(Command::ToggleOnionSkin);
        }
        if onion.enabled {
            let mut changed = false;
            changed |= ui
                .checkbox(&mut onion.show_previous, "Prev")
                .on_hover_text("Show the previous frame in a warm tint")
                .changed();
            changed |= ui
                .checkbox(&mut onion.show_next, "Next")
                .on_hover_text("Show the next frame in a cool tint")
                .changed();
            let mut percent = (onion.clamped_opacity() * 100.0).round() as u32;
            let resp = ui
                .add(
                    egui::DragValue::new(&mut percent)
                        .range((MIN_OPACITY * 100.0) as u32..=(MAX_OPACITY * 100.0) as u32)
                        .suffix("%")
                        .speed(1.0),
                )
                .on_hover_text("Onion skin opacity");
            if resp.changed() {
                onion.opacity = percent as f32 / 100.0;
            }
            changed |= resp.drag_stopped() || (resp.changed() && !resp.dragged());
            if changed {
                commands.push(Command::SettingsChanged);
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (icon, tip) = if settings.expanded {
                (Icon::ArrowDown, "Collapse the timeline (F7)")
            } else {
                (Icon::ArrowUp, "Expand the timeline (F7)")
            };
            if icon_button(ui, icon, 28.0, false, true, tip).clicked() {
                commands.push(Command::ToggleTimeline);
            }
            let playback = settings.playback;
            let mut changed = false;
            if playback.timing == PlaybackTiming::FixedFps {
                let resp = ui
                    .add(
                        egui::DragValue::new(&mut playback.fps)
                            .range(FPS_RANGE)
                            .suffix(" fps"),
                    )
                    .on_hover_text("Preview speed in frames per second");
                changed |= resp.drag_stopped() || (resp.changed() && !resp.dragged());
            }
            let label = match playback.timing {
                PlaybackTiming::FrameDurations => "Frame durations",
                PlaybackTiming::FixedFps => "Fixed FPS",
            };
            egui::ComboBox::from_id_salt("playback-timing")
                .selected_text(label)
                .width(124.0)
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(
                            &mut playback.timing,
                            PlaybackTiming::FrameDurations,
                            "Frame durations",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut playback.timing,
                            PlaybackTiming::FixedFps,
                            "Fixed FPS",
                        )
                        .changed();
                })
                .response
                .on_hover_text("How the preview is timed");
            ui.label(
                RichText::new("Preview")
                    .small()
                    .color(theme::TEXT_SECONDARY),
            );
            if changed {
                commands.push(Command::SettingsChanged);
            }
        });
    }

    /// Duration of the active frame; one undo step per finished edit, not per drag tick.
    fn duration_field(
        &mut self,
        ui: &mut egui::Ui,
        doc: &Document,
        editable: bool,
        commands: &mut Vec<Command>,
    ) {
        let active = doc.active_frame_index();
        let frame = doc.active_frame();
        let mut value = match self.duration_edit {
            Some((id, v)) if id == frame.id => v,
            _ => frame.duration_ms(),
        };
        ui.label(
            RichText::new("Duration")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        let resp = ui
            .add_enabled(
                editable,
                egui::DragValue::new(&mut value)
                    .range(MIN_FRAME_DURATION_MS..=MAX_FRAME_DURATION_MS)
                    .suffix(" ms")
                    .speed(2.0),
            )
            .on_hover_text(format!(
                "How long this frame is shown ({MIN_FRAME_DURATION_MS}–{MAX_FRAME_DURATION_MS} ms)"
            ));
        if resp.changed() {
            self.duration_edit = Some((frame.id, value));
        }
        let finished = resp.drag_stopped()
            || resp.lost_focus()
            || (resp.changed() && !resp.dragged() && !resp.has_focus());
        if finished {
            if let Some((id, v)) = self.duration_edit.take() {
                if id == frame.id {
                    commands.push(Command::SetFrameDuration(active, v));
                }
            }
        }
    }

    fn strip(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        playing: bool,
        commands: &mut Vec<Command>,
    ) {
        let doc = editor.document();
        let active = doc.active_frame_index();
        let ctx = ui.ctx().clone();
        let mut budget = THUMB_BUILD_BUDGET;
        let mut missing = false;
        egui::ScrollArea::horizontal()
            .id_salt("timeline-strip")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (index, frame) in doc.frames().iter().enumerate() {
                        let (rect, resp) = ui.allocate_exact_size(
                            Vec2::new(CARD_WIDTH, THUMB_BOX + 34.0),
                            Sense::click(),
                        );
                        if index == active && playing {
                            resp.scroll_to_me(None);
                        }
                        if !ui.is_rect_visible(rect) {
                            continue;
                        }
                        // Keep the active thumbnail stable while a stroke is in progress.
                        let key = (frame.revision(), doc.structure_revision());
                        let stale = self.thumbs.get(&frame.id).is_none_or(|t| t.key != key);
                        let busy = index == active && editor.is_stroking();
                        if stale && (!busy || !self.thumbs.contains_key(&frame.id)) {
                            if budget > 0 {
                                budget -= 1;
                                let image = thumbnail(doc, index);
                                match self.thumbs.get_mut(&frame.id) {
                                    Some(t) => {
                                        t.texture.set(image, egui::TextureOptions::NEAREST);
                                        t.key = key;
                                    }
                                    None => {
                                        let texture = ctx.load_texture(
                                            format!("thumb-{}", frame.id.0),
                                            image,
                                            egui::TextureOptions::NEAREST,
                                        );
                                        self.thumbs.insert(frame.id, Thumb { key, texture });
                                    }
                                }
                            } else {
                                missing = true;
                            }
                        }
                        paint_card(
                            ui,
                            rect,
                            &resp,
                            index,
                            frame.duration_ms(),
                            index == active,
                            self.selected.contains(&frame.id),
                            self.thumbs.get(&frame.id).map(|t| &t.texture),
                        );
                        let resp = resp.on_hover_text(format!(
                            "Frame {} · {} ms\nClick to edit · Ctrl/Shift+click to select for export · right-click for actions",
                            index + 1,
                            frame.duration_ms()
                        ));
                        if resp.clicked() {
                            let mods = ui.input(|i| i.modifiers);
                            if mods.command {
                                // Toggle this frame (the current one joins the first time).
                                if self.selected.is_empty() {
                                    self.selected.insert(doc.active_frame().id);
                                }
                                if !self.selected.remove(&frame.id) {
                                    self.selected.insert(frame.id);
                                }
                            } else if mods.shift {
                                let (a, b) = (active.min(index), active.max(index));
                                for f in &doc.frames()[a..=b] {
                                    self.selected.insert(f.id);
                                }
                            } else {
                                self.selected.clear();
                                commands.push(Command::SelectFrame(index));
                            }
                        }
                        resp.context_menu(|ui| {
                            ui.add_enabled_ui(!playing, |ui| {
                                if ui.button("Duplicate").clicked() {
                                    commands.push(Command::SelectFrame(index));
                                    commands.push(Command::DuplicateFrame);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(index > 0, egui::Button::new("Move Left"))
                                    .clicked()
                                {
                                    commands.push(Command::MoveFrameLeft(index));
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(
                                        index + 1 < doc.frame_count(),
                                        egui::Button::new("Move Right"),
                                    )
                                    .clicked()
                                {
                                    commands.push(Command::MoveFrameRight(index));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .add_enabled(doc.frame_count() > 1, egui::Button::new("Delete"))
                                    .clicked()
                                {
                                    commands.push(Command::DeleteFrame(index));
                                    ui.close_menu();
                                }
                            });
                        });
                    }
                    ui.add_space(4.0);
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(36.0, THUMB_BOX), Sense::click());
                    let enabled = !playing && doc.can_add_frame().is_ok();
                    let color = if resp.hovered() && enabled {
                        theme::ACCENT
                    } else {
                        theme::BORDER
                    };
                    ui.painter().rect_stroke(
                        rect,
                        theme::RADIUS as f32,
                        Stroke::new(1.0_f32, color),
                        StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(20.0),
                        color,
                    );
                    if resp.on_hover_text("New empty frame").clicked() && enabled {
                        commands.push(Command::AddFrame);
                    }
                });
            });
        if missing {
            ctx.request_repaint();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_card(
    ui: &egui::Ui,
    rect: Rect,
    resp: &egui::Response,
    index: usize,
    duration: u32,
    active: bool,
    selected: bool,
    texture: Option<&TextureHandle>,
) {
    let painter = ui.painter();
    let bg = if active || selected {
        theme::SURFACE_HOVER
    } else if resp.hovered() {
        theme::SURFACE
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(rect, theme::RADIUS as f32, bg);
    let box_rect = Rect::from_min_size(
        rect.min + Vec2::new((CARD_WIDTH - THUMB_BOX) / 2.0, 3.0),
        Vec2::splat(THUMB_BOX),
    );
    // A small fixed checkerboard tells transparency apart from dark pixels.
    let cell = THUMB_BOX / 6.0;
    painter.rect_filled(box_rect, 2.0, theme::CHECKER_DARK);
    for cy in 0..6 {
        for cx in 0..6 {
            if (cx + cy) % 2 == 0 {
                let min = box_rect.min + Vec2::new(cx as f32 * cell, cy as f32 * cell);
                painter.rect_filled(
                    Rect::from_min_size(min, Vec2::splat(cell)),
                    0.0,
                    theme::CHECKER_LIGHT,
                );
            }
        }
    }
    if let Some(tex) = texture {
        let [tw, th] = tex.size();
        let scale = (THUMB_BOX / tw.max(1) as f32).min(THUMB_BOX / th.max(1) as f32);
        let size = Vec2::new(tw as f32, th as f32) * scale;
        let img_rect = Rect::from_center_size(box_rect.center(), size);
        painter.image(
            tex.id(),
            img_rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let border = if active {
        theme::ACCENT
    } else if selected {
        theme::SUCCESS
    } else {
        theme::BORDER
    };
    painter.rect_stroke(
        box_rect,
        2.0,
        Stroke::new(if active || selected { 2.0_f32 } else { 1.0_f32 }, border),
        StrokeKind::Outside,
    );
    let number_color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_PRIMARY
    };
    painter.text(
        egui::Pos2::new(rect.center().x, box_rect.max.y + 5.0),
        egui::Align2::CENTER_TOP,
        format!("{}", index + 1),
        egui::FontId::proportional(12.0),
        number_color,
    );
    painter.text(
        egui::Pos2::new(rect.center().x, box_rect.max.y + 17.0),
        egui::Align2::CENTER_TOP,
        format!("{duration} ms"),
        egui::FontId::proportional(9.5),
        theme::TEXT_SECONDARY,
    );
}

/// Nearest-neighbor thumbnail of a frame, sampling only the pixels it needs.
pub fn thumbnail(doc: &Document, frame: usize) -> ColorImage {
    let (w, h) = (doc.width(), doc.height());
    let scale = (THUMB_PIXELS as f32 / w as f32)
        .min(THUMB_PIXELS as f32 / h as f32)
        .min(1.0);
    let tw = ((w as f32 * scale).round() as u32).max(1);
    let th = ((h as f32 * scale).round() as u32).max(1);
    let mut bytes = Vec::with_capacity((tw * th * 4) as usize);
    for ty in 0..th {
        let y = ((ty as f32 + 0.5) * h as f32 / th as f32) as i32;
        for tx in 0..tw {
            let x = ((tx as f32 + 0.5) * w as f32 / tw as f32) as i32;
            let px = doc.composite_frame_pixel(frame, x, y).unwrap_or_default();
            bytes.extend_from_slice(&px.0);
        }
    }
    ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Rgba;

    #[test]
    fn thumbnails_keep_aspect_and_sample_real_pixels() {
        let doc = Document::new(256, 128, Rgba::WHITE).expect("valid");
        let thumb = thumbnail(&doc, 0);
        assert_eq!(thumb.size, [64, 32]);
        assert!(thumb.pixels.iter().all(|p| *p == Color32::WHITE));
        let small = Document::new(8, 4, Rgba::TRANSPARENT).expect("valid");
        assert_eq!(
            thumbnail(&small, 0).size,
            [8, 4],
            "small images are not upscaled"
        );
    }
}

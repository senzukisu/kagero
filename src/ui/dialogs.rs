//! Modal dialogs: new document, unsaved changes, errors, about.

use egui::emath::GuiRounding as _;
use egui::{Button, Id, Modal, RichText, Vec2};

use crate::core::{Anchor, Rgba, MAX_CANVAS_SIZE};
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

/// Named starting sizes.
const PRESETS: [(&str, u32, u32); 8] = [
    ("Icon", 16, 16),
    ("Sprite", 32, 32),
    ("Character", 64, 64),
    ("Portrait", 128, 128),
    ("Large", 256, 256),
    ("Tile strip", 128, 32),
    ("Screen", 320, 180),
    ("Wide screen", 640, 360),
];

pub fn new_document(ctx: &egui::Context, state: &mut NewDocumentState) -> NewDocumentResult {
    let mut result = NewDocumentResult::Open;
    let modal = Modal::new(Id::new("new-document"))
        .frame(
            egui::Frame::new()
                .fill(theme::panel())
                .stroke(egui::Stroke::new(1.0_f32, theme::bevel_light()))
                .shadow(egui::Shadow {
                    offset: [6, 6],
                    blur: 0,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(170),
                })
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ctx, |ui| {
            ui.set_width(600.0);
            ui.label(
                RichText::new("New Image")
                    .font(theme::font(2))
                    .color(theme::text_primary()),
            );
            ui.label(
                RichText::new("Pick a preset or type any size, then choose the background.")
                    .color(theme::text_secondary()),
            );
            ui.add_space(10.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    caption(ui, "Presets");
                    egui::Grid::new("new-doc-presets")
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            for (i, (name, w, h)) in PRESETS.into_iter().enumerate() {
                                let selected = state.width == w && state.height == h;
                                if preset_card(ui, name, w, h, selected).clicked() {
                                    state.width = w;
                                    state.height = h;
                                }
                                if i % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });
                });
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    caption(ui, "Size");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.width)
                                .range(1..=MAX_CANVAS_SIZE)
                                .suffix(" px"),
                        )
                        .on_hover_text("Width");
                        if super::icons::icon_button(
                            ui,
                            super::icons::Icon::Swap,
                            24.0,
                            false,
                            state.width != state.height,
                            "Swap width and height",
                        )
                        .clicked()
                        {
                            std::mem::swap(&mut state.width, &mut state.height);
                        }
                        ui.add(
                            egui::DragValue::new(&mut state.height)
                                .range(1..=MAX_CANVAS_SIZE)
                                .suffix(" px"),
                        )
                        .on_hover_text("Height");
                    });
                    ui.label(
                        RichText::new(format!("1 to {MAX_CANVAS_SIZE} px per side"))
                            .small()
                            .color(theme::text_secondary()),
                    );
                    ui.add_space(8.0);
                    caption(ui, "Background");
                    ui.horizontal(|ui| {
                        for bg in [
                            Background::Transparent,
                            Background::White,
                            Background::Black,
                        ] {
                            if background_swatch(ui, bg, state.background == bg).clicked() {
                                state.background = bg;
                            }
                        }
                    });
                    ui.add_space(8.0);
                    caption(ui, "Preview");
                    preview(ui, state);
                });
            });
            ui.add_space(12.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add(accent_button("Create")).clicked() {
                    result = NewDocumentResult::Create;
                }
                if ui.button("Cancel").clicked() {
                    result = NewDocumentResult::Cancel;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Enter creates · Esc cancels")
                            .small()
                            .color(theme::text_secondary()),
                    );
                });
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

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(theme::accent()),
    );
}

/// A preset button: a tiny outline of its shape, the name and the size.
fn preset_card(ui: &mut egui::Ui, name: &str, w: u32, h: u32, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(146.0, 48.0), egui::Sense::click());
    let painter = ui.painter();
    let fill = if selected {
        theme::ember_deep()
    } else if resp.hovered() {
        theme::surface_hover()
    } else {
        theme::surface()
    };
    painter.rect_filled(rect, 0.0, fill);
    let outline = if selected {
        theme::accent()
    } else {
        theme::bevel_dark()
    };
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0_f32, outline),
        egui::StrokeKind::Inside,
    );
    // Shape thumbnail, fitted in a 28 px box.
    let scale = 28.0 / w.max(h) as f32;
    let shape = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 24.0, rect.center().y),
        Vec2::new((w as f32 * scale).max(3.0), (h as f32 * scale).max(3.0)),
    );
    painter.rect_filled(shape, 0.0, theme::checker_light());
    painter.rect_stroke(
        shape,
        0.0,
        egui::Stroke::new(
            1.0_f32,
            if selected {
                theme::accent()
            } else {
                theme::text_secondary()
            },
        ),
        egui::StrokeKind::Inside,
    );
    let text_x = rect.left() + 46.0;
    painter.text(
        egui::pos2(text_x, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::TextStyle::Body.resolve(ui.style()),
        theme::text_primary(),
    );
    painter.text(
        egui::pos2(text_x, rect.bottom() - 8.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{w}×{h}"),
        egui::TextStyle::Small.resolve(ui.style()),
        if selected {
            theme::accent()
        } else {
            theme::text_secondary()
        },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Paints `rect` as a background: a checkerboard for transparent, else the color.
fn paint_background(painter: &egui::Painter, rect: egui::Rect, bg: Background, cell: f32) {
    match bg {
        Background::Transparent => {
            painter.rect_filled(rect, 0.0, theme::checker_light());
            let mut mesh = egui::Mesh::default();
            let (cols, rows) = (
                (rect.width() / cell).ceil() as i32,
                (rect.height() / cell).ceil() as i32,
            );
            for y in 0..rows {
                for x in 0..cols {
                    if (x + y) % 2 == 0 {
                        continue;
                    }
                    let min = rect.min + Vec2::new(x as f32, y as f32) * cell;
                    let r = egui::Rect::from_min_size(min, Vec2::splat(cell)).intersect(rect);
                    mesh.add_colored_rect(r, theme::checker_dark());
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        Background::White => {
            painter.rect_filled(rect, 0.0, egui::Color32::WHITE);
        }
        Background::Black => {
            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
        }
    }
}

fn background_swatch(ui: &mut egui::Ui, bg: Background, selected: bool) -> egui::Response {
    let resp = ui
        .vertical(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::click());
            paint_background(ui.painter(), rect.shrink(3.0), bg, 6.0);
            let stroke = if selected {
                egui::Stroke::new(2.0_f32, theme::accent())
            } else if resp.hovered() {
                egui::Stroke::new(1.0_f32, theme::text_secondary())
            } else {
                egui::Stroke::new(1.0_f32, theme::bevel_dark())
            };
            ui.painter()
                .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            ui.label(RichText::new(bg.name()).small().color(if selected {
                theme::accent()
            } else {
                theme::text_secondary()
            }));
            resp
        })
        .inner;
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The new image's shape and background, fitted in a box, with its size.
fn preview(ui: &mut egui::Ui, state: &NewDocumentState) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(250.0, 110.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(area, 0.0, theme::background());
    let (w, h) = (state.width.max(1) as f32, state.height.max(1) as f32);
    let scale = ((area.width() - 20.0) / w).min((area.height() - 20.0) / h);
    let size = Vec2::new((w * scale).max(2.0), (h * scale).max(2.0));
    let rect = egui::Rect::from_center_size(area.center(), size)
        .round_to_pixels(ui.ctx().pixels_per_point());
    paint_background(painter, rect, state.background, scale.clamp(4.0, 10.0));
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0_f32, theme::border()),
        egui::StrokeKind::Outside,
    );
    let bytes = u64::from(state.width) * u64::from(state.height) * 4;
    let memory = if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KiB", bytes.div_ceil(1024))
    };
    ui.label(
        RichText::new(format!(
            "{} × {} px · {memory} per frame and layer",
            state.width, state.height
        ))
        .small()
        .color(theme::text_secondary()),
    );
}

/// Image → Canvas Size: the new size and where the current image goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasSizeState {
    pub old_width: u32,
    pub old_height: u32,
    pub width: u32,
    pub height: u32,
    pub anchor: Anchor,
}

impl CanvasSizeState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            old_width: width,
            old_height: height,
            width,
            height,
            anchor: Anchor::CENTER,
        }
    }

    /// Whether some existing pixels fall outside the new canvas.
    pub fn cuts_pixels(&self) -> bool {
        self.width < self.old_width || self.height < self.old_height
    }
}

pub enum CanvasSizeResult {
    Open,
    Apply,
    Cancel,
}

pub fn canvas_size(ctx: &egui::Context, state: &mut CanvasSizeState) -> CanvasSizeResult {
    let mut result = CanvasSizeResult::Open;
    let modal = Modal::new(Id::new("canvas-size")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.label(
            RichText::new("Canvas Size")
                .font(theme::font(2))
                .color(theme::text_primary()),
        );
        ui.label(
            RichText::new("Changes the canvas of every layer and frame. The image is not scaled.")
                .color(theme::text_secondary()),
        );
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(220.0);
                egui::Grid::new("canvas-size-grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
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
                    });
                ui.label(
                    RichText::new(format!("Now {} × {} px", state.old_width, state.old_height))
                        .small()
                        .color(theme::text_secondary()),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("ANCHOR")
                        .small()
                        .strong()
                        .color(theme::accent()),
                );
                ui.label(
                    RichText::new("Where the current image stays")
                        .small()
                        .color(theme::text_secondary()),
                );
                egui::Grid::new("canvas-anchor")
                    .spacing([3.0, 3.0])
                    .show(ui, |ui| {
                        for y in 0..3u8 {
                            for x in 0..3u8 {
                                let selected = state.anchor == Anchor { x, y };
                                let (rect, resp) =
                                    ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
                                let fill = if selected {
                                    theme::accent()
                                } else if resp.hovered() {
                                    theme::surface_hover()
                                } else {
                                    theme::surface()
                                };
                                ui.painter().rect_filled(rect, 0.0, fill);
                                ui.painter().rect_stroke(
                                    rect,
                                    0.0,
                                    egui::Stroke::new(1.0_f32, theme::bevel_dark()),
                                    egui::StrokeKind::Inside,
                                );
                                if resp.clicked() {
                                    state.anchor = Anchor { x, y };
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("PREVIEW")
                        .small()
                        .strong()
                        .color(theme::accent()),
                );
                canvas_size_preview(ui, state);
            });
        });
        if state.cuts_pixels() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "⚠ The new canvas is smaller: pixels outside it are cut off in every \
                     layer and frame (Undo brings them back).",
                )
                .color(theme::accent()),
            );
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let changed = state.width != state.old_width || state.height != state.old_height;
            if ui.add_enabled(changed, accent_button("Apply")).clicked() {
                result = CanvasSizeResult::Apply;
            }
            if ui.button("Cancel").clicked() {
                result = CanvasSizeResult::Cancel;
            }
        });
    });
    if modal.should_close() && matches!(result, CanvasSizeResult::Open) {
        result = CanvasSizeResult::Cancel;
    }
    state.width = state.width.clamp(1, MAX_CANVAS_SIZE);
    state.height = state.height.clamp(1, MAX_CANVAS_SIZE);
    result
}

/// The new canvas (outline) and where the current image lands in it (filled).
fn canvas_size_preview(ui: &mut egui::Ui, state: &CanvasSizeState) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(220.0, 160.0), egui::Sense::hover());
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, theme::background());
    let (dx, dy) =
        state
            .anchor
            .offset(state.old_width, state.old_height, state.width, state.height);
    // Fit the union of both rectangles into the area.
    let min_x = dx.min(0) as f32;
    let min_y = dy.min(0) as f32;
    let max_x = (dx + i64::from(state.old_width)).max(i64::from(state.width)) as f32;
    let max_y = (dy + i64::from(state.old_height)).max(i64::from(state.height)) as f32;
    let scale =
        ((area.width() - 24.0) / (max_x - min_x)).min((area.height() - 24.0) / (max_y - min_y));
    let origin = area.center() - Vec2::new((max_x + min_x) / 2.0, (max_y + min_y) / 2.0) * scale;
    let to_rect = |x: f32, y: f32, w: f32, h: f32| {
        egui::Rect::from_min_size(origin + Vec2::new(x, y) * scale, Vec2::new(w, h) * scale)
    };
    let canvas = to_rect(0.0, 0.0, state.width as f32, state.height as f32);
    let image = to_rect(
        dx as f32,
        dy as f32,
        state.old_width as f32,
        state.old_height as f32,
    );
    painter.rect_filled(canvas, 0.0, theme::checker_dark());
    painter.rect_filled(image, 0.0, theme::accent().gamma_multiply(0.45));
    painter.rect_stroke(
        canvas,
        0.0,
        egui::Stroke::new(1.0_f32, theme::text_primary()),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        image,
        0.0,
        egui::Stroke::new(1.0_f32, theme::accent()),
        egui::StrokeKind::Inside,
    );
    ui.label(
        RichText::new("Outline: new canvas · Amber: your image")
            .small()
            .color(theme::text_secondary()),
    );
}

/// Recovery copies left by a crash, and errors from opening them.
#[derive(Debug, Clone, Default)]
pub struct RecoveryState {
    pub entries: Vec<crate::services::recovery::RecoveryEntry>,
    /// Entry id → why it couldn't be opened.
    pub errors: std::collections::HashMap<String, String>,
}

pub enum RecoveryChoice {
    Keep,
    Close,
    OpenRecovered(usize),
    OpenOriginal(usize),
    Delete(usize),
}

pub fn recovery(ctx: &egui::Context, state: &RecoveryState) -> RecoveryChoice {
    use crate::services::recovery::{age, utc_time};
    let mut choice = RecoveryChoice::Keep;
    let height = (ctx.screen_rect().height() - 260.0).clamp(160.0, 420.0);
    let modal = Modal::new(Id::new("recovery")).show(ctx, |ui| {
        ui.set_width(620.0);
        ui.label(
            RichText::new("Recover Unsaved Work")
                .font(theme::font(2))
                .color(theme::text_primary()),
        );
        ui.label(
            RichText::new(
                "Kagero closed unexpectedly. These autosaved copies have unsaved changes. \
                 Opening one never replaces your original project — save it when you're ready.",
            )
            .color(theme::text_secondary()),
        );
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (i, entry) in state.entries.iter().enumerate() {
                    egui::Frame::new()
                        .fill(theme::surface())
                        .stroke(egui::Stroke::new(1.0_f32, theme::bevel_dark()))
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&entry.name)
                                        .strong()
                                        .color(theme::text_primary()),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "autosaved {} · {}",
                                        age(entry.saved_at),
                                        utc_time(entry.saved_at)
                                    ))
                                    .small()
                                    .color(theme::text_secondary()),
                                );
                            });
                            let original = entry
                                .original
                                .as_ref()
                                .map_or("Never saved as a project".to_owned(), |p| {
                                    p.display().to_string()
                                });
                            ui.label(
                                RichText::new(original)
                                    .small()
                                    .color(theme::text_secondary()),
                            );
                            if let Some(err) = state.errors.get(&entry.id) {
                                ui.label(
                                    RichText::new(format!("This copy can't be opened: {err}"))
                                        .color(theme::error()),
                                );
                            }
                            ui.horizontal(|ui| {
                                let readable = !state.errors.contains_key(&entry.id);
                                if ui
                                    .add_enabled(readable, accent_button("Open Recovered"))
                                    .clicked()
                                {
                                    choice = RecoveryChoice::OpenRecovered(i);
                                }
                                let has_original =
                                    entry.original.as_ref().is_some_and(|p| p.exists());
                                if ui
                                    .add_enabled(has_original, Button::new("Open Original"))
                                    .clicked()
                                {
                                    choice = RecoveryChoice::OpenOriginal(i);
                                }
                                if ui
                                    .button("Delete Copy")
                                    .on_hover_text("Throw this autosaved copy away")
                                    .clicked()
                                {
                                    choice = RecoveryChoice::Delete(i);
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .button("Later")
                .on_hover_text("Keep the copies; File → Recover Projects… shows them again")
                .clicked()
            {
                choice = RecoveryChoice::Close;
            }
        });
    });
    if modal.should_close() && matches!(choice, RecoveryChoice::Keep) {
        choice = RecoveryChoice::Close;
    }
    choice
}

/// New or existing frame tag being edited.
#[derive(Debug, Clone)]
pub struct TagState {
    /// `None` for a new tag.
    pub index: Option<usize>,
    pub tag: crate::core::FrameTag,
    pub frame_count: usize,
}

pub enum TagResult {
    Open,
    Cancel,
    Apply,
    Delete,
}

pub fn frame_tag(ctx: &egui::Context, state: &mut TagState) -> TagResult {
    use crate::core::tags::{clean_tag_name, MAX_TAG_NAME_CHARS, TAG_COLORS};
    let mut result = TagResult::Open;
    let title = if state.index.is_some() {
        "Edit Tag"
    } else {
        "New Tag"
    };
    let modal = Modal::new(Id::new("frame-tag")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.label(
            RichText::new(title)
                .font(theme::font(2))
                .color(theme::text_primary()),
        );
        ui.label(
            RichText::new("A named range of frames, e.g. \"walk\" or \"idle\". Tags can be looped in the preview and exported on their own.")
                .color(theme::text_secondary()),
        );
        ui.add_space(8.0);
        let count = state.frame_count.max(1) as u32;
        let tag = &mut state.tag;
        let mut first = tag.start as u32 + 1;
        let mut last = tag.end as u32 + 1;
        let mut name_resp = None;
        egui::Grid::new("frame-tag-grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Name");
                name_resp = Some(
                    ui.add(
                        egui::TextEdit::singleline(&mut tag.name)
                            .char_limit(MAX_TAG_NAME_CHARS)
                            .desired_width(220.0),
                    ),
                );
                ui.end_row();
                ui.label("Frames");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut first).range(1..=count));
                    ui.label("to");
                    ui.add(egui::DragValue::new(&mut last).range(1..=count));
                    ui.label(RichText::new(format!("of {count}")).color(theme::text_secondary()));
                });
                ui.end_row();
                ui.label("Color");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for c in TAG_COLORS {
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
                        let fill = egui::Color32::from_rgb(c.r(), c.g(), c.b());
                        ui.painter().rect_filled(rect.shrink(2.0), 3.0, fill);
                        if tag.color == c {
                            ui.painter().rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(2.0_f32, theme::text_primary()),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if resp.on_hover_text(c.to_hex()).clicked() {
                            tag.color = c;
                        }
                    }
                });
                ui.end_row();
            });
        // Keep the range ordered whichever end was dragged past the other.
        tag.start = (first.min(last) - 1) as usize;
        tag.end = (first.max(last) - 1) as usize;
        if let Some(resp) = &name_resp {
            if state.index.is_none() && !resp.has_focus() && ui.memory(|m| m.focused().is_none()) {
                resp.request_focus();
            }
        }
        let valid = clean_tag_name(&tag.name).is_some();
        if !valid {
            ui.label(RichText::new("Give the tag a name.").color(theme::error()));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let ok = if state.index.is_some() { "Apply" } else { "Add Tag" };
            let enter = name_resp
                .as_ref()
                .is_some_and(|r| r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if ui.add_enabled(valid, accent_button(ok)).clicked() || (enter && valid) {
                result = TagResult::Apply;
            }
            if ui.button("Cancel").clicked() {
                result = TagResult::Cancel;
            }
            if state.index.is_some() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Delete Tag").clicked() {
                        result = TagResult::Delete;
                    }
                });
            }
        });
    });
    if modal.should_close() && matches!(result, TagResult::Open) {
        result = TagResult::Cancel;
    }
    result
}

/// Image → Convert to Palette.
#[derive(Default)]
pub struct ConvertPaletteState {
    pub scope: crate::editor::PaletteScope,
    /// What the preview was made for, its before/after textures and color counts.
    preview: Option<(u64, [egui::TextureHandle; 2], [usize; 2])>,
    error: Option<String>,
}

pub enum ConvertPaletteResult {
    Open,
    Cancel,
    Apply,
}

fn distinct_colors(buf: &crate::core::PixelBuffer) -> usize {
    let mut set = std::collections::HashSet::new();
    for p in buf.as_bytes().chunks_exact(4) {
        if p[3] != 0 {
            set.insert([p[0], p[1], p[2], p[3]]);
        }
    }
    set.len()
}

pub fn convert_palette(
    ctx: &egui::Context,
    state: &mut ConvertPaletteState,
    editor: &crate::editor::Editor,
    palette: &crate::core::palette::Palette,
) -> ConvertPaletteResult {
    use std::hash::{Hash, Hasher};
    let mut result = ConvertPaletteResult::Open;
    // Rebuild the preview when anything it depends on changes.
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (
        state.scope as u8,
        editor.history().state_id(),
        editor.document().active_frame_index(),
        editor.document().active_index(),
        &palette.colors,
    )
        .hash(&mut h);
    let key = h.finish();
    if state.preview.as_ref().is_none_or(|(k, _, _)| *k != key) {
        match editor.palette_preview(&palette.colors, state.scope) {
            Ok((before, after)) => {
                let tex = |name: &str, b: &crate::core::PixelBuffer| {
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [b.width() as usize, b.height() as usize],
                        b.as_bytes(),
                    );
                    ctx.load_texture(name, img, egui::TextureOptions::NEAREST)
                };
                let counts = [distinct_colors(&before), distinct_colors(&after)];
                state.preview = Some((
                    key,
                    [tex("palette-before", &before), tex("palette-after", &after)],
                    counts,
                ));
                state.error = None;
            }
            Err(e) => state.error = Some(e.to_string()),
        }
    }
    let modal = Modal::new(Id::new("convert-palette")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.label(
            RichText::new("Convert to Palette")
                .font(theme::font(2))
                .color(theme::text_primary()),
        );
        ui.label(
            RichText::new(format!(
                "Replaces every color with the nearest color of \"{}\" ({} colors). Transparency is kept. One undo step.",
                palette.name,
                palette.colors.len()
            ))
            .color(theme::text_secondary()),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Change");
            egui::ComboBox::from_id_salt("convert-scope")
                .selected_text(state.scope.name())
                .show_ui(ui, |ui| {
                    for s in crate::editor::PaletteScope::ALL {
                        ui.selectable_value(&mut state.scope, s, s.name());
                    }
                });
        });
        ui.add_space(6.0);
        if let Some((_, [before, after], [cb, ca])) = &state.preview {
            ui.horizontal_top(|ui| {
                for (tex, label) in [(before, format!("Before · {cb} colors")), (after, format!("After · {ca} colors"))] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).small().color(theme::text_secondary()));
                        let size = tex.size_vec2();
                        let scale = (240.0 / size.x.max(size.y)).min(8.0);
                        let (rect, _) = ui.allocate_exact_size(size * scale, egui::Sense::hover());
                        // A checkerboard behind shows transparency.
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(70));
                        ui.painter().image(
                            tex.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    });
                }
            });
            ui.label(
                RichText::new("Preview of the current frame.")
                    .small()
                    .color(theme::text_secondary()),
            );
        }
        if let Some(e) = &state.error {
            ui.label(RichText::new(e).color(theme::error()));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(state.error.is_none(), accent_button("Convert")).clicked() {
                result = ConvertPaletteResult::Apply;
            }
            if ui.button("Cancel").clicked() {
                result = ConvertPaletteResult::Cancel;
            }
        });
    });
    if modal.should_close() && matches!(result, ConvertPaletteResult::Open) {
        result = ConvertPaletteResult::Cancel;
    }
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
                .color(theme::text_secondary()),
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
        ui.label(RichText::new(title).heading().color(theme::error()));
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
/// What the About window asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutChoice {
    Keep,
    Close,
    Licenses,
}

pub fn about(ctx: &egui::Context) -> AboutChoice {
    let mut close = false;
    let mut licenses = false;
    let modal = Modal::new(Id::new("about-dialog"))
        .frame(
            egui::Frame::new()
                .fill(theme::panel())
                .stroke(egui::Stroke::new(1.0_f32, theme::bevel_light()))
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
                                ui.label(RichText::new("·").color(theme::text_secondary()));
                                ui.hyperlink_to(
                                    "Changelog",
                                    format!("{repo}/blob/main/CHANGELOG.md"),
                                );
                                ui.label(RichText::new("·").color(theme::text_secondary()));
                                ui.hyperlink_to("GitHub", repo.clone());
                            });
                        });

                    ui.add_space(12.0);
                    ui.label(RichText::new("SHORTCUTS").color(theme::text_secondary()));
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
                                ui.label(RichText::new(v1).color(theme::text_secondary()));
                                keycaps(ui, k2);
                                ui.label(RichText::new(v2).color(theme::text_secondary()));
                                ui.end_row();
                            }
                        });

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("© 2026 {}", crate::STUDIO_NAME))
                                .color(theme::text_secondary()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(accent_button("Close")).clicked() {
                                close = true;
                            }
                            if ui.button("Licenses").clicked() {
                                licenses = true;
                            }
                        });
                    });
                });
        });
    if licenses {
        AboutChoice::Licenses
    } else if close || modal.should_close() {
        AboutChoice::Close
    } else {
        AboutChoice::Keep
    }
}

/// Kagero's license and the third-party list, embedded so they ship with the program.
const LICENSE_TEXT: &str = include_str!("../../LICENSE");
const THIRD_PARTY_TEXT: &str = include_str!("../../THIRD_PARTY_LICENSES.md");

/// The licenses window. `tab`: 0 = Kagero, 1 = third-party components. Returns
/// `false` when closed.
pub fn licenses(ctx: &egui::Context, tab: &mut u8) -> bool {
    let mut close = false;
    let height = (ctx.screen_rect().height() - 220.0).clamp(240.0, 520.0);
    let modal = Modal::new(Id::new("licenses-dialog")).show(ctx, |ui| {
        ui.set_width(680.0);
        ui.label(
            RichText::new("Licenses")
                .font(theme::font(2))
                .color(theme::text_primary()),
        );
        ui.label(
            RichText::new(
                "Kagero is MIT-licensed. It is built on open-source components whose licenses are listed here.",
            )
            .color(theme::text_secondary()),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.selectable_value(tab, 0, "Kagero (MIT)");
            ui.selectable_value(tab, 1, "Third-party components");
        });
        ui.separator();
        let text = if *tab == 0 { LICENSE_TEXT } else { THIRD_PARTY_TEXT };
        egui::ScrollArea::vertical()
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.label(RichText::new(text).monospace().color(theme::text_primary()));
            });
        ui.add_space(8.0);
        if ui.add(accent_button("Close")).clicked() {
            close = true;
        }
    });
    !(close || modal.should_close())
}

/// The dark banner at the top of About: flame, name and version badge.
fn about_banner(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(theme::background())
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
                            .color(theme::text_primary()),
                    );
                    ui.label(RichText::new("陽炎 · pixel art editor").color(theme::accent()));
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme::ember_deep())
                        .stroke(egui::Stroke::new(1.0_f32, theme::accent()))
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "VERSION {} {}",
                                    crate::APP_VERSION,
                                    crate::APP_CHANNEL.to_uppercase()
                                ))
                                .color(theme::text_primary()),
                            );
                        });
                });
            });
        });
    // A 2-pixel ember rule between the banner and the body.
    let (line, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, theme::accent_pressed());
}

fn fact(ui: &mut egui::Ui, name: &str, value: impl FnOnce(&mut egui::Ui)) {
    ui.label(RichText::new(name).color(theme::text_secondary()));
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
                .fill(theme::surface())
                .stroke(egui::Stroke::new(1.0_f32, theme::bevel_dark()))
                .inner_margin(egui::Margin::symmetric(5, 1))
                .show(ui, |ui| {
                    ui.label(RichText::new(key).color(theme::accent()));
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
            .color(theme::on_accent()),
    )
    .fill(theme::accent())
    .min_size(Vec2::new(88.0, 28.0))
}

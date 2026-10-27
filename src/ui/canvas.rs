//! The central canvas: view transform, texture sync, painting and pointer input.

use egui::{
    Color32, ColorImage, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Vec2,
};

use crate::core::raster;
use crate::editor::{ColorSlot, Dirty, EditError, Editor, ToolKind};
use crate::theme;

/// Allowed zoom factors in physical screen pixels per image pixel, so that every image
/// pixel covers a whole number of device pixels at any display scale.
pub const ZOOM_LEVELS: [f32; 15] = [
    0.25, 0.5, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0,
];
/// The pixel grid is drawn only from this zoom level up.
const GRID_MIN_ZOOM: f32 = 6.0;

/// Pure view transform between screen space and image pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewTransform {
    /// Screen position of the image's top-left corner, in points.
    pub origin: Pos2,
    /// Points per image pixel.
    pub zoom: f32,
}

impl ViewTransform {
    /// Builds the transform for an image of `size` centered in `viewport` and shifted by `pan`.
    /// `zoom` is in points per image pixel; the origin is snapped to a whole device pixel
    /// (`pixels_per_point`) so pixel edges stay crisp.
    pub fn new(viewport: Rect, size: Vec2, zoom: f32, pan: Vec2, pixels_per_point: f32) -> Self {
        let ppp = pixels_per_point.max(0.01);
        let raw = viewport.center() + pan - size * zoom / 2.0;
        let origin = Pos2::new((raw.x * ppp).round() / ppp, (raw.y * ppp).round() / ppp);
        Self { origin, zoom }
    }

    /// The image pixel under a screen position (may be outside the image).
    pub fn screen_to_pixel(&self, pos: Pos2) -> (i32, i32) {
        let p = (pos - self.origin) / self.zoom;
        (clamp_to_i32(p.x.floor()), clamp_to_i32(p.y.floor()))
    }

    pub fn pixel_to_screen(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.zoom
    }

    pub fn image_rect(&self, width: u32, height: u32) -> Rect {
        Rect::from_min_size(
            self.origin,
            Vec2::new(width as f32, height as f32) * self.zoom,
        )
    }
}

/// Checker cells across the longer image side.
const CHECKER_CELLS_ACROSS: u32 = 32;
/// Cells never get smaller than this many device pixels (when zoomed far out).
const CHECKER_MIN_DEVICE_PX: f32 = 4.0;

/// Size of one checkerboard cell in image pixels.
///
/// It follows the image resolution (about 32 cells across: 32×32 → 1 px cells,
/// 256×256 → 8 px, 2048×2048 → 64 px), so the pattern scales together with the image
/// and always sits on pixel boundaries. `device_zoom` (device pixels per image pixel)
/// only enlarges cells when zoomed out so far that they would turn into noise.
pub fn checker_cell_pixels(width: u32, height: u32, device_zoom: f32) -> f32 {
    let target = width.max(height).div_ceil(CHECKER_CELLS_ACROSS).max(1);
    let mut cell = target.next_power_of_two() as f32;
    while cell * device_zoom < CHECKER_MIN_DEVICE_PX && cell < 4096.0 {
        cell *= 2.0;
    }
    cell
}

fn clamp_to_i32(v: f32) -> i32 {
    if v.is_nan() {
        0
    } else {
        v.clamp(i32::MIN as f32, i32::MAX as f32) as i32
    }
}

/// Index into [`ZOOM_LEVELS`] of the level closest to `zoom`.
fn nearest_level(zoom: f32) -> usize {
    ZOOM_LEVELS
        .iter()
        .enumerate()
        .min_by(|a, b| {
            (a.1 - zoom)
                .abs()
                .partial_cmp(&(b.1 - zoom).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or(2, |(i, _)| i)
}

/// Canvas state that is not part of the document.
pub struct CanvasView {
    texture: Option<TextureHandle>,
    checker: Option<TextureHandle>,
    pub zoom: f32,
    pub pan: Vec2,
    pub show_grid: bool,
    /// Image pixel under the cursor, if the cursor is over the image.
    pub hover_pixel: Option<(i32, i32)>,
    stroke_button: Option<PointerButton>,
    panning: bool,
    needs_fit: bool,
    viewport: Rect,
    /// Device pixels per egui point (Windows display scale).
    ppp: f32,
}

impl CanvasView {
    pub fn new(show_grid: bool) -> Self {
        Self {
            texture: None,
            checker: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            show_grid,
            hover_pixel: None,
            stroke_button: None,
            panning: false,
            needs_fit: true,
            viewport: Rect::NOTHING,
            ppp: 1.0,
        }
    }

    /// Points per image pixel at the current zoom.
    fn scale(&self) -> f32 {
        self.zoom / self.ppp
    }

    fn transform(&self, size: Vec2) -> ViewTransform {
        ViewTransform::new(self.viewport, size, self.scale(), self.pan, self.ppp)
    }

    /// Forgets the texture and fits the next document on the next frame.
    pub fn reset(&mut self) {
        self.texture = None;
        self.needs_fit = true;
        self.pan = Vec2::ZERO;
        self.hover_pixel = None;
        self.stroke_button = None;
        self.panning = false;
    }

    pub fn center(&mut self) {
        self.pan = Vec2::ZERO;
    }

    pub fn request_fit(&mut self) {
        self.needs_fit = true;
    }

    pub fn zoom_in(&mut self) {
        let i = nearest_level(self.zoom);
        self.set_zoom_keep_center(ZOOM_LEVELS[(i + 1).min(ZOOM_LEVELS.len() - 1)]);
    }

    pub fn zoom_out(&mut self) {
        let i = nearest_level(self.zoom);
        self.set_zoom_keep_center(ZOOM_LEVELS[i.saturating_sub(1)]);
    }

    pub fn zoom_actual(&mut self) {
        self.set_zoom_keep_center(1.0);
    }

    fn set_zoom_keep_center(&mut self, zoom: f32) {
        // Keep the image point at the viewport center fixed.
        if self.zoom > 0.0 {
            self.pan *= zoom / self.zoom;
        }
        self.zoom = zoom;
    }

    /// Zooms one step in or out keeping the image point under `anchor` fixed.
    fn zoom_at(&mut self, anchor: Pos2, size: Vec2, steps: i32) {
        let i = nearest_level(self.zoom) as i32;
        let new_i = (i + steps).clamp(0, ZOOM_LEVELS.len() as i32 - 1) as usize;
        let new_zoom = ZOOM_LEVELS[new_i];
        if (new_zoom - self.zoom).abs() < f32::EPSILON {
            return;
        }
        let t = self.transform(size);
        let image_point = (anchor - t.origin) / t.zoom;
        let new_scale = new_zoom / self.ppp;
        let new_origin = anchor - image_point * new_scale;
        self.pan = new_origin - self.viewport.center() + size * new_scale / 2.0;
        self.zoom = new_zoom;
    }

    fn fit(&mut self, size: Vec2) {
        let avail = self.viewport.size() * 0.85;
        let best = ZOOM_LEVELS
            .iter()
            .rev()
            .find(|z| {
                let s = **z / self.ppp;
                size.x * s <= avail.x && size.y * s <= avail.y
            })
            .copied()
            .unwrap_or(ZOOM_LEVELS[0]);
        self.zoom = best;
        self.pan = Vec2::ZERO;
    }

    /// Uploads changed pixels to the GPU texture, only the dirty region when possible.
    fn sync_texture(&mut self, ctx: &egui::Context, editor: &mut Editor) {
        let doc = editor.document();
        let size = [doc.width() as usize, doc.height() as usize];
        let dirty = editor.take_dirty();
        let doc = editor.document();
        let full = |doc: &crate::core::Document| -> ColorImage {
            let bytes = doc
                .composite_rect(0, 0, doc.width(), doc.height())
                .map(|(_, b)| b)
                .unwrap_or_default();
            ColorImage::from_rgba_unmultiplied(size, &bytes)
        };
        match self.texture.as_mut().filter(|t| t.size() == size) {
            Some(tex) => match dirty {
                None => {}
                Some(Dirty::Full) => tex.set(full(doc), TextureOptions::NEAREST),
                Some(Dirty::Rect(x0, y0, x1, y1)) => {
                    if let Some(((x, y, w, h), bytes)) =
                        doc.composite_rect(x0, y0, x1 - x0 + 1, y1 - y0 + 1)
                    {
                        let img =
                            ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &bytes);
                        tex.set_partial([x as usize, y as usize], img, TextureOptions::NEAREST);
                    }
                }
            },
            None => {
                self.texture =
                    Some(ctx.load_texture("kagero-canvas", full(doc), TextureOptions::NEAREST));
            }
        }
        if self.checker.is_none() {
            let (l, d) = (theme::CHECKER_LIGHT, theme::CHECKER_DARK);
            let mut bytes = Vec::with_capacity(16);
            for c in [l, d, d, l] {
                bytes.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
            }
            let img = ColorImage::from_rgba_unmultiplied([2, 2], &bytes);
            self.checker =
                Some(ctx.load_texture("kagero-checker", img, TextureOptions::NEAREST_REPEAT));
        }
    }

    /// Draws the canvas and handles pointer input. Returns an error to show the user, if any.
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &mut Editor) -> Option<EditError> {
        let ctx = ui.ctx().clone();
        let viewport = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(viewport, Sense::click_and_drag());
        self.viewport = viewport;
        self.ppp = ctx.pixels_per_point();

        let (w, h) = (editor.document().width(), editor.document().height());
        let size = Vec2::new(w as f32, h as f32);
        if self.needs_fit && viewport.width() > 1.0 {
            self.fit(size);
            self.needs_fit = false;
        }
        self.sync_texture(&ctx, editor);

        let mut error = None;
        let typing = ctx.wants_keyboard_input();
        let (space, pointer_pos, scroll, zoom_delta) = ctx.input(|i| {
            (
                !typing && i.key_down(egui::Key::Space),
                i.pointer.latest_pos(),
                i.raw_scroll_delta.y,
                i.zoom_delta(),
            )
        });

        // ---- zoom (wheel or pinch/ctrl+wheel) around the cursor
        if response.hovered() {
            if let Some(pos) = pointer_pos {
                let steps = if scroll > 0.0 || zoom_delta > 1.0 {
                    1
                } else if scroll < 0.0 || zoom_delta < 1.0 {
                    -1
                } else {
                    0
                };
                if steps != 0 {
                    self.zoom_at(pos, size, steps);
                }
            }
        }

        // ---- panning: middle drag, Space + drag, or the Pan tool
        let pan_gesture = !editor.is_stroking()
            && (response.dragged_by(PointerButton::Middle)
                || (response.dragged_by(PointerButton::Primary)
                    && (space || editor.tool == ToolKind::Pan || self.panning)));
        if pan_gesture {
            self.panning = true;
            self.pan += response.drag_delta();
        } else if !response.dragged() {
            self.panning = false;
        }

        let transform = self.transform(size);

        // ---- drawing
        let (pressed, released, down) = ctx.input(|i| {
            let p = &i.pointer;
            (
                [
                    p.button_pressed(PointerButton::Primary),
                    p.button_pressed(PointerButton::Secondary),
                ],
                [
                    p.button_released(PointerButton::Primary),
                    p.button_released(PointerButton::Secondary),
                ],
                [
                    p.button_down(PointerButton::Primary),
                    p.button_down(PointerButton::Secondary),
                ],
            )
        });
        let can_start = response.hovered()
            && !space
            && !self.panning
            && editor.tool != ToolKind::Pan
            && self.stroke_button.is_none()
            && !editor.is_stroking();
        if can_start {
            if let Some(pos) = pointer_pos {
                let start = if pressed[0] {
                    Some((PointerButton::Primary, ColorSlot::Primary))
                } else if pressed[1] {
                    Some((PointerButton::Secondary, ColorSlot::Secondary))
                } else {
                    None
                };
                if let Some((button, slot)) = start {
                    let px = transform.screen_to_pixel(pos);
                    match editor.begin_stroke(px, slot) {
                        Ok(()) => {
                            if editor.is_stroking() {
                                self.stroke_button = Some(button);
                            }
                        }
                        Err(e) => error = Some(e),
                    }
                }
            }
        }
        if let Some(button) = self.stroke_button {
            let idx = usize::from(button == PointerButton::Secondary);
            if let Some(pos) = pointer_pos {
                editor.update_stroke(transform.screen_to_pixel(pos));
            }
            if released[idx] || !down[idx] {
                editor.end_stroke();
                self.stroke_button = None;
            }
        } else if editor.is_stroking() {
            // A stroke without a button (e.g. focus lost) must not stay open.
            editor.end_stroke();
        }
        // The editor may have uploaded changes during input handling.
        self.sync_texture(&ctx, editor);

        // ---- hover info and cursor
        self.hover_pixel = pointer_pos
            .filter(|_| response.hovered())
            .map(|p| transform.screen_to_pixel(p))
            .filter(|&(x, y)| x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h);
        if response.hovered() {
            let cursor = if self.panning {
                CursorIcon::Grabbing
            } else if space || editor.tool == ToolKind::Pan {
                CursorIcon::Grab
            } else {
                CursorIcon::Crosshair
            };
            ctx.set_cursor_icon(cursor);
        }

        // ---- painting
        let painter = ui.painter_at(viewport);
        painter.rect_filled(viewport, 0.0, theme::BACKGROUND);
        let image_rect = transform.image_rect(w, h);
        if let Some(checker) = &self.checker {
            // Checker cells are whole image pixels, so they line up with the pixel grid.
            let cell = checker_cell_pixels(w, h, self.zoom);
            let uv = Rect::from_min_max(
                Pos2::ZERO,
                Pos2::new(w as f32 / (cell * 2.0), h as f32 / (cell * 2.0)),
            );
            painter.image(checker.id(), image_rect, uv, Color32::WHITE);
        }
        if let Some(tex) = &self.texture {
            painter.image(
                tex.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if self.show_grid && self.zoom >= GRID_MIN_ZOOM {
            self.paint_grid(&painter, &transform, viewport, w, h);
        }
        painter.rect_stroke(
            image_rect,
            0.0,
            Stroke::new(1.0_f32, theme::BORDER),
            StrokeKind::Outside,
        );
        if let Some((x, y)) = self.hover_pixel {
            if editor.tool.edits_pixels() && !space && !self.panning {
                let size = if editor.tool.uses_brush_size() {
                    editor.settings.brush_size
                } else {
                    1
                };
                let mut min = (i32::MAX, i32::MAX);
                let mut max = (i32::MIN, i32::MIN);
                for (dx, dy) in raster::brush_offsets(size) {
                    min = (min.0.min(x + dx), min.1.min(y + dy));
                    max = (max.0.max(x + dx + 1), max.1.max(y + dy + 1));
                }
                let r = Rect::from_min_max(
                    transform.pixel_to_screen(min.0 as f32, min.1 as f32),
                    transform.pixel_to_screen(max.0 as f32, max.1 as f32),
                );
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0_f32, theme::ACCENT),
                    StrokeKind::Outside,
                );
            }
        }
        error
    }

    fn paint_grid(
        &self,
        painter: &egui::Painter,
        t: &ViewTransform,
        viewport: Rect,
        w: u32,
        h: u32,
    ) {
        let visible = t.image_rect(w, h).intersect(viewport);
        if !visible.is_positive() {
            return;
        }
        // One device pixel wide, centered on the device pixel just right of/below each edge.
        let px = 1.0 / self.ppp;
        let stroke = Stroke::new(px, Color32::from_rgba_unmultiplied(255, 255, 255, 40));
        let (x0, y0) = t.screen_to_pixel(visible.min);
        let (x1, y1) = t.screen_to_pixel(visible.max);
        let x_range = x0.max(0)..=(x1 + 1).min(w as i32);
        let y_range = y0.max(0)..=(y1 + 1).min(h as i32);
        for x in x_range {
            let sx = t.pixel_to_screen(x as f32, 0.0).x + px / 2.0;
            painter.line_segment(
                [Pos2::new(sx, visible.min.y), Pos2::new(sx, visible.max.y)],
                stroke,
            );
        }
        for y in y_range {
            let sy = t.pixel_to_screen(0.0, y as f32).y + px / 2.0;
            painter.line_segment(
                [Pos2::new(visible.min.x, sy), Pos2::new(visible.max.x, sy)],
                stroke,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_pixel_round_trip() {
        let viewport = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(400.0, 300.0));
        let t = ViewTransform::new(viewport, Vec2::new(16.0, 8.0), 8.0, Vec2::ZERO, 1.0);
        // The image is centered: 128×64 screen pixels inside the viewport.
        assert_eq!(t.origin, Pos2::new(236.0, 168.0));
        assert_eq!(t.screen_to_pixel(Pos2::new(236.0, 168.0)), (0, 0));
        assert_eq!(t.screen_to_pixel(Pos2::new(243.9, 175.9)), (0, 0));
        assert_eq!(t.screen_to_pixel(Pos2::new(244.0, 176.0)), (1, 1));
        assert_eq!(t.screen_to_pixel(Pos2::new(235.9, 168.0)), (-1, 0));
        assert_eq!(
            t.screen_to_pixel(Pos2::new(236.0 + 127.9, 168.0 + 63.9)),
            (15, 7)
        );
        for (x, y) in [(0, 0), (5, 3), (15, 7)] {
            let s = t.pixel_to_screen(x as f32 + 0.5, y as f32 + 0.5);
            assert_eq!(t.screen_to_pixel(s), (x, y));
        }
    }

    #[test]
    fn origin_snaps_to_device_pixels_at_125_percent() {
        let viewport = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(401.0, 301.0));
        // 16 device px per image pixel at 125% = 12.8 points per image pixel.
        let t = ViewTransform::new(viewport, Vec2::new(3.0, 3.0), 12.8, Vec2::ZERO, 1.25);
        let dev = t.origin.x * 1.25;
        assert!(
            (dev - dev.round()).abs() < 1e-3,
            "origin {dev} is not on a device pixel"
        );
        let edge = t.pixel_to_screen(1.0, 0.0).x * 1.25;
        assert!((edge - edge.round()).abs() < 1e-3);
    }

    #[test]
    fn checker_cells_are_whole_pixels() {
        // Follows the image resolution, independent of zoom.
        assert_eq!(checker_cell_pixels(32, 32, 16.0), 1.0);
        assert_eq!(checker_cell_pixels(256, 256, 2.0), 8.0);
        assert_eq!(checker_cell_pixels(256, 256, 32.0), 8.0);
        assert_eq!(checker_cell_pixels(320, 180, 4.0), 16.0);
        assert_eq!(checker_cell_pixels(2048, 2048, 1.0), 64.0);
        assert_eq!(checker_cell_pixels(1, 1, 64.0), 1.0);
        // Only grows when cells would be smaller than a few device pixels.
        assert_eq!(checker_cell_pixels(32, 32, 1.0), 4.0);
        assert_eq!(checker_cell_pixels(2048, 2048, 0.25), 64.0);
    }

    #[test]
    fn zoom_levels_step() {
        assert_eq!(ZOOM_LEVELS[nearest_level(1.0)], 1.0);
        assert_eq!(ZOOM_LEVELS[nearest_level(7.0)], 6.0);
        let mut v = CanvasView::new(true);
        v.zoom = 4.0;
        v.zoom_in();
        assert_eq!(v.zoom, 6.0);
        v.zoom_out();
        v.zoom_out();
        assert_eq!(v.zoom, 3.0);
    }
}

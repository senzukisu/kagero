//! The central canvas: view transform, texture sync, painting and pointer input.

use egui::{
    Color32, ColorImage, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Vec2,
};

use std::collections::HashMap;

use crate::core::onion_skin::OnionSkin;
use crate::core::selection::{Segment, SelRect, SelectMode, Selection};
use crate::core::{raster, Document, FrameId};
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

    /// The pixel boundary nearest to a screen position (for edges between pixels).
    pub fn screen_to_edge(&self, pos: Pos2) -> (i32, i32) {
        let p = (pos - self.origin) / self.zoom;
        (clamp_to_i32(p.x.round()), clamp_to_i32(p.y.round()))
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

/// Cells never get smaller than this many device pixels (when zoomed far out).
const CHECKER_MIN_DEVICE_PX: f32 = 3.0;

/// Size of one checkerboard cell in image pixels.
///
/// One cell is exactly one image pixel, for every image size, so the checkerboard
/// doubles as a pixel grid. Only when zoomed out so far that a pixel is smaller than
/// a few device pixels (`device_zoom` = device pixels per image pixel) do cells merge
/// into 2×2, 4×4, … pixel blocks, which still sit on pixel boundaries.
pub fn checker_cell_pixels(device_zoom: f32) -> f32 {
    let mut cell = 1.0_f32;
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

/// GPU memory budget for cached whole-frame textures (onion skin and playback).
const FRAME_CACHE_BYTES: usize = 256 * 1024 * 1024;

/// A composited frame uploaded as a texture, reused until the frame changes.
struct FrameTexture {
    key: (u64, u64),
    texture: TextureHandle,
    bytes: usize,
    last_used: u64,
}

/// What the canvas should show besides the active frame.
#[derive(Debug, Clone, Copy)]
pub struct CanvasOptions {
    pub onion: OnionSkin,
    /// Animation preview is running: show frames, block drawing.
    pub playing: bool,
    pub reduce_motion: bool,
    /// Experimental: show the image repeated around itself (for seamless tiles).
    pub tiled: bool,
}

/// Canvas state that is not part of the document.
pub struct CanvasView {
    texture: Option<TextureHandle>,
    checker: Option<TextureHandle>,
    /// The theme colors `checker` was made with.
    checker_colors: Option<(Color32, Color32)>,
    frame_cache: HashMap<FrameId, FrameTexture>,
    frame_tick: u64,
    pub zoom: f32,
    pub pan: Vec2,
    pub show_grid: bool,
    /// Image pixel under the cursor, if the cursor is over the image.
    pub hover_pixel: Option<(i32, i32)>,
    stroke_button: Option<PointerButton>,
    select_drag: Option<SelectDrag>,
    /// Lasso: the outline being drawn (image pixels) and how it will combine.
    lasso: Option<(Vec<(i32, i32)>, SelectMode)>,
    /// The last selection outline drawn: the selection it belongs to (kept so its mask
    /// can't be freed and its address reused) and its segments relative to its bounds.
    outline_cache: Option<(Selection, Vec<Segment>)>,
    /// Crop tool: what the current drag adjusts.
    crop_drag: Option<CropDrag>,
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
            checker_colors: None,
            frame_cache: HashMap::new(),
            frame_tick: 0,
            zoom: 1.0,
            pan: Vec2::ZERO,
            show_grid,
            hover_pixel: None,
            stroke_button: None,
            select_drag: None,
            lasso: None,
            outline_cache: None,
            crop_drag: None,
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
        self.frame_cache.clear();
        self.needs_fit = true;
        self.pan = Vec2::ZERO;
        self.hover_pixel = None;
        self.stroke_button = None;
        self.select_drag = None;
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

    /// Texture of a whole composited frame, cached until that frame or the layer
    /// structure changes. Least-recently-used entries are evicted over the budget.
    fn frame_texture(
        &mut self,
        ctx: &egui::Context,
        doc: &Document,
        index: usize,
    ) -> Option<egui::TextureId> {
        let frame = doc.frames().get(index)?;
        let key = (frame.revision(), doc.structure_revision());
        self.frame_tick += 1;
        let tick = self.frame_tick;
        if let Some(entry) = self.frame_cache.get_mut(&frame.id) {
            if entry.key == key {
                entry.last_used = tick;
                return Some(entry.texture.id());
            }
        }
        let (_, bytes) = doc.composite_frame_rect(index, 0, 0, doc.width(), doc.height())?;
        let size = [doc.width() as usize, doc.height() as usize];
        let image = ColorImage::from_rgba_unmultiplied(size, &bytes);
        let byte_len = bytes.len();
        let id = match self.frame_cache.get_mut(&frame.id) {
            Some(entry) => {
                entry.texture.set(image, TextureOptions::NEAREST);
                entry.key = key;
                entry.last_used = tick;
                entry.bytes = byte_len;
                entry.texture.id()
            }
            None => {
                let texture = ctx.load_texture(
                    format!("frame-{}", frame.id.0),
                    image,
                    TextureOptions::NEAREST,
                );
                let id = texture.id();
                self.frame_cache.insert(
                    frame.id,
                    FrameTexture {
                        key,
                        texture,
                        bytes: byte_len,
                        last_used: tick,
                    },
                );
                id
            }
        };
        // Drop textures of deleted frames, then the least recently used over budget.
        self.frame_cache
            .retain(|fid, _| doc.frames().iter().any(|f| f.id == *fid));
        while self.frame_cache.values().map(|e| e.bytes).sum::<usize>() > FRAME_CACHE_BYTES
            && self.frame_cache.len() > 1
        {
            let oldest = self
                .frame_cache
                .iter()
                .filter(|(fid, _)| **fid != frame.id)
                .min_by_key(|(_, e)| e.last_used)
                .map(|(fid, _)| *fid);
            match oldest {
                Some(fid) => {
                    self.frame_cache.remove(&fid);
                }
                None => break,
            }
        }
        Some(id)
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
    }

    /// Draws the canvas and handles pointer input. Returns an error to show the user, if any.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        editor: &mut Editor,
        options: CanvasOptions,
    ) -> Option<EditError> {
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
        // During playback frames come from the frame cache; the editing texture is
        // refreshed when playback stops.
        if !options.playing {
            self.sync_texture(&ctx, editor);
        }

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
        if options.playing
            && response.hovered()
            && !space
            && editor.tool != ToolKind::Pan
            && (pressed[0] || pressed[1])
        {
            error = Some(EditError::ReadOnly);
        }
        let can_start = response.hovered()
            && !options.playing
            && !space
            && !self.panning
            && editor.tool != ToolKind::Pan
            && !editor.tool.selects()
            && editor.tool != ToolKind::Polygon
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
        } else if editor.is_stroking() && !editor.is_drawing_polygon() {
            // A stroke without a button (e.g. focus lost) must not stay open.
            editor.end_stroke();
        }
        // ---- polygon: clicks add corners, the cursor previews the next side
        if editor.tool == ToolKind::Polygon && !options.playing && !space && !self.panning {
            if let (true, Some(pos)) = (response.hovered(), pointer_pos) {
                let p = transform.screen_to_pixel(pos);
                let slot = if pressed[0] {
                    Some(ColorSlot::Primary)
                } else if pressed[1] {
                    Some(ColorSlot::Secondary)
                } else {
                    None
                };
                if let Some(slot) = slot {
                    if let Err(e) = editor.polygon_click(p, slot) {
                        error = Some(e);
                    }
                } else if editor.is_drawing_polygon() {
                    editor.update_stroke(p);
                }
            }
        } else if editor.is_drawing_polygon() {
            editor.end_stroke();
        }
        // ---- rectangle selection
        let (ctrl, shift, alt, escape) = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.modifiers.alt,
                i.key_pressed(egui::Key::Escape),
            )
        });
        let mode = SelectMode::with_modifiers(editor.settings.select_mode, shift, alt);
        if editor.tool != ToolKind::Select || options.playing || escape {
            self.select_drag = None;
        } else {
            if self.select_drag.is_none()
                && response.hovered()
                && !space
                && !self.panning
                && pressed[0]
            {
                if let Some(pos) = pointer_pos {
                    let p = transform.screen_to_pixel(pos);
                    let inside = mode == SelectMode::Replace
                        && editor
                            .selection_shape()
                            .is_some_and(|s| s.contains(p.0, p.1));
                    self.select_drag = Some(if inside && ctrl {
                        SelectDrag::Outline { start: p }
                    } else if inside {
                        let ghost = editor.selection_block().map(|b| {
                            let img = ColorImage::from_rgba_unmultiplied(
                                [b.w as usize, b.h as usize],
                                &b.data,
                            );
                            ctx.load_texture("kagero-selection-ghost", img, TextureOptions::NEAREST)
                        });
                        SelectDrag::Move { start: p, ghost }
                    } else {
                        SelectDrag::New { start: p, mode }
                    });
                }
            }
            if let Some(drag) = &self.select_drag {
                if released[0] || !down[0] {
                    let start = drag.start();
                    let cur = pointer_pos.map_or(start, |p| transform.screen_to_pixel(p));
                    let (dx, dy) = (cur.0 - start.0, cur.1 - start.1);
                    match drag {
                        SelectDrag::New {
                            mode: SelectMode::Replace,
                            ..
                        } if cur == start => editor.deselect(),
                        SelectDrag::New { mode, .. } => editor.select(
                            Some(Selection::from(SelRect::from_corners(start, cur))),
                            *mode,
                        ),
                        SelectDrag::Move { .. } => {
                            if let Err(e) = editor.move_selection(dx, dy) {
                                error = Some(e);
                            }
                        }
                        SelectDrag::Outline { .. } => editor.move_selection_outline(dx, dy),
                    }
                    self.select_drag = None;
                }
            }
        }
        // ---- lasso
        if editor.tool != ToolKind::Lasso || options.playing || escape {
            self.lasso = None;
        } else {
            let start =
                self.lasso.is_none() && response.hovered() && !space && !self.panning && pressed[0];
            if start {
                if let Some(pos) = pointer_pos {
                    self.lasso = Some((vec![transform.screen_to_pixel(pos)], mode));
                }
            }
            if let Some((points, mode)) = &mut self.lasso {
                if let Some(pos) = pointer_pos {
                    let p = transform.screen_to_pixel(pos);
                    if points.last() != Some(&p) && points.len() < MAX_LASSO_POINTS {
                        points.push(p);
                    }
                }
                if released[0] || !down[0] {
                    if points.len() <= 1 && *mode == SelectMode::Replace {
                        editor.deselect();
                    } else {
                        editor.select_lasso(points, *mode);
                    }
                    self.lasso = None;
                }
            }
        }
        // ---- magic wand
        if editor.tool == ToolKind::MagicWand
            && !options.playing
            && response.hovered()
            && !space
            && !self.panning
            && pressed[0]
        {
            if let Some(pos) = pointer_pos {
                if let Err(e) = editor.magic_wand(transform.screen_to_pixel(pos), mode) {
                    error = Some(e);
                }
            }
        }
        // ---- crop frame
        if editor.tool != ToolKind::Crop || options.playing {
            self.crop_drag = None;
        } else {
            if self.crop_drag.is_none()
                && response.hovered()
                && !space
                && !self.panning
                && pressed[0]
            {
                if let Some(pos) = pointer_pos {
                    let p = transform.screen_to_pixel(pos);
                    self.crop_drag = Some(match editor.crop_rect {
                        Some(r) => {
                            let edges = crop_edges(&transform, r, pos);
                            if edges != (false, false, false, false) {
                                CropDrag::Edges { edges, orig: r }
                            } else if r.contains(p.0, p.1) {
                                CropDrag::Move { start: p, orig: r }
                            } else {
                                CropDrag::New { start: p }
                            }
                        }
                        None => CropDrag::New { start: p },
                    });
                }
            }
            if let (Some(drag), Some(pos)) = (&self.crop_drag, pointer_pos) {
                let (doc_w, doc_h) = (editor.document().width(), editor.document().height());
                let (w, h) = (doc_w as i32, doc_h as i32);
                // Edges snap between pixels: round to the nearest pixel boundary.
                let edge = transform.screen_to_edge(pos);
                let cur = transform.screen_to_pixel(pos);
                let rect = match *drag {
                    CropDrag::New { start } => Some(SelRect::from_corners(start, cur)),
                    CropDrag::Move { start, orig } => {
                        let dx = (cur.0 - start.0).clamp(-orig.x, w - orig.right() as i32);
                        let dy = (cur.1 - start.1).clamp(-orig.y, h - orig.bottom() as i32);
                        Some(orig.translated(dx, dy))
                    }
                    CropDrag::Edges {
                        edges: (l, r, t, b),
                        orig,
                    } => {
                        let mut x0 = orig.x;
                        let mut y0 = orig.y;
                        let mut x1 = orig.right() as i32;
                        let mut y1 = orig.bottom() as i32;
                        if l {
                            x0 = edge.0.clamp(0, x1 - 1);
                        }
                        if r {
                            x1 = edge.0.clamp(x0 + 1, w);
                        }
                        if t {
                            y0 = edge.1.clamp(0, y1 - 1);
                        }
                        if b {
                            y1 = edge.1.clamp(y0 + 1, h);
                        }
                        Some(SelRect {
                            x: x0,
                            y: y0,
                            w: (x1 - x0) as u32,
                            h: (y1 - y0) as u32,
                        })
                    }
                };
                editor.crop_rect = rect.and_then(|r| r.clip(doc_w, doc_h));
                if released[0] || !down[0] {
                    self.crop_drag = None;
                }
            }
        }
        // The editor may have uploaded changes during input handling.
        if !options.playing {
            self.sync_texture(&ctx, editor);
        }
        self.ensure_checker(&ctx);

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
        painter.rect_filled(viewport, 0.0, theme::background());
        let image_rect = transform.image_rect(w, h);
        // Checker cells are whole image pixels, so they line up with the pixel grid.
        let cell = checker_cell_pixels(self.zoom);
        let checker_uv = Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(w as f32 / (cell * 2.0), h as f32 / (cell * 2.0)),
        );
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let doc = editor.document();
        let main = if options.playing {
            self.frame_texture(&ctx, doc, doc.active_frame_index())
        } else {
            self.texture.as_ref().map(TextureHandle::id)
        };
        // Tiled view: eight dimmed copies around the image. View only.
        let tiles: &[(f32, f32)] = if options.tiled {
            &[
                (-1.0, -1.0),
                (0.0, -1.0),
                (1.0, -1.0),
                (-1.0, 0.0),
                (1.0, 0.0),
                (-1.0, 1.0),
                (0.0, 1.0),
                (1.0, 1.0),
                (0.0, 0.0),
            ]
        } else {
            &[(0.0, 0.0)]
        };
        for &(tx, ty) in tiles {
            let rect =
                image_rect.translate(Vec2::new(tx * image_rect.width(), ty * image_rect.height()));
            if !rect.intersects(viewport) {
                continue;
            }
            let tint = if tx == 0.0 && ty == 0.0 {
                Color32::WHITE
            } else {
                Color32::from_gray(150)
            };
            if let Some(checker) = &self.checker {
                painter.image(checker.id(), rect, checker_uv, tint);
            }
            if let Some(id) = main {
                painter.image(id, rect, full_uv, tint);
            }
        }
        // Onion skin: tinted, translucent neighbors drawn over the active frame. View only.
        let overlays = if options.playing {
            Vec::new()
        } else {
            options
                .onion
                .overlays(doc.frame_count(), doc.active_frame_index())
        };
        for overlay in &overlays {
            if let Some(id) = self.frame_texture(&ctx, doc, overlay.frame) {
                let [r, g, b] = overlay.side.tint();
                let alpha = (overlay.opacity * 255.0).round() as u8;
                let tint = Color32::from_rgba_unmultiplied(r, g, b, alpha);
                painter.image(id, image_rect, full_uv, tint);
            }
        }
        if self.show_grid && self.zoom >= GRID_MIN_ZOOM {
            self.paint_grid(&painter, &transform, viewport, w, h);
        }
        self.paint_selection(&painter, &transform, editor, pointer_pos, &options);
        let s = editor.settings;
        if (s.mirror_x || s.mirror_y) && editor.tool.edits_pixels() && editor.tool != ToolKind::Fill
        {
            // Symmetry axes through the image center.
            let (w, h) = (
                editor.document().width() as f32,
                editor.document().height() as f32,
            );
            let stroke = Stroke::new(1.0_f32, theme::accent().gamma_multiply(0.7));
            if s.mirror_x {
                painter.line_segment(
                    [
                        transform.pixel_to_screen(w / 2.0, 0.0),
                        transform.pixel_to_screen(w / 2.0, h),
                    ],
                    stroke,
                );
            }
            if s.mirror_y {
                painter.line_segment(
                    [
                        transform.pixel_to_screen(0.0, h / 2.0),
                        transform.pixel_to_screen(w, h / 2.0),
                    ],
                    stroke,
                );
            }
        }
        if editor.tool == ToolKind::Crop && !options.playing {
            if let Some(r) = editor.crop_rect {
                paint_crop_frame(&painter, &transform, image_rect, r);
            }
        }
        painter.rect_stroke(
            image_rect,
            0.0,
            Stroke::new(1.0_f32, theme::border()),
            StrokeKind::Outside,
        );
        if let Some((x, y)) = self.hover_pixel {
            if editor.tool.edits_pixels() && !space && !self.panning && !options.playing {
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
                    Stroke::new(1.0_f32, theme::accent()),
                    StrokeKind::Outside,
                );
            }
        }
        self.paint_badges(
            &painter,
            viewport,
            editor.document(),
            &options,
            !overlays.is_empty(),
        );
        error
    }

    /// The selection outline (marching ants) and, while dragging, a live preview.
    fn paint_selection(
        &mut self,
        painter: &egui::Painter,
        t: &ViewTransform,
        editor: &Editor,
        pointer: Option<Pos2>,
        options: &CanvasOptions,
    ) {
        let (w, h) = (editor.document().width(), editor.document().height());
        let to_screen = |r: SelRect| {
            Rect::from_min_max(
                t.pixel_to_screen(r.x as f32, r.y as f32),
                t.pixel_to_screen(r.right() as f32, r.bottom() as f32),
            )
        };
        let cur = pointer.map(|p| t.screen_to_pixel(p));
        let phase = if options.reduce_motion {
            0.0
        } else {
            (painter.ctx().input(|i| i.time) * 12.0 % 8.0) as f32
        };
        // The selection where it is (or will be after the current drag).
        let (dx, dy) = match &self.select_drag {
            Some(SelectDrag::Move { start, .. } | SelectDrag::Outline { start }) => {
                cur.map_or((0, 0), |c| (c.0 - start.0, c.1 - start.1))
            }
            _ => (0, 0),
        };
        if let (Some(SelectDrag::Move { ghost, .. }), Some(sel)) =
            (&self.select_drag, editor.selection())
        {
            // Dim the lifted area and draw the pixels where they will land.
            painter.rect_filled(to_screen(sel), 0.0, Color32::from_black_alpha(110));
            if let Some(tex) = ghost {
                painter.image(
                    tex.id(),
                    to_screen(sel.translated(dx, dy)),
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        let mut animated = false;
        if let Some(sel) = editor.selection_shape() {
            let r = sel.bounds();
            if sel.is_rectangular() {
                paint_marching_ants(painter, to_screen(r.translated(dx, dy)), phase);
            } else {
                let fresh = self
                    .outline_cache
                    .as_ref()
                    .is_some_and(|(s, _)| s.mask_id() == sel.mask_id() && s.bounds() == r);
                if !fresh {
                    let segments = sel
                        .outline()
                        .into_iter()
                        .map(|[a, b]| [(a.0 - r.x, a.1 - r.y), (b.0 - r.x, b.1 - r.y)])
                        .collect();
                    self.outline_cache = Some((sel.clone(), segments));
                }
                if let Some((_, segments)) = &self.outline_cache {
                    let (ox, oy) = (r.x + dx, r.y + dy);
                    let to =
                        |p: (i32, i32)| t.pixel_to_screen((ox + p.0) as f32, (oy + p.1) as f32);
                    if segments.len() > MAX_OUTLINE_SEGMENTS {
                        // Too detailed to draw every edge: show its bounds instead.
                        paint_marching_ants(painter, to_screen(r.translated(dx, dy)), phase);
                    } else {
                        let clip = painter.clip_rect().expand(2.0);
                        paint_ants_segments(
                            painter,
                            segments
                                .iter()
                                .map(|[a, b]| (to(*a), to(*b)))
                                .filter(|(a, b)| clip.intersects(Rect::from_two_pos(*a, *b))),
                            phase,
                        );
                    }
                }
            }
            animated = true;
        } else {
            self.outline_cache = None;
        }
        // A new rectangle being dragged.
        if let (Some(SelectDrag::New { start, .. }), Some(c)) = (&self.select_drag, cur) {
            if let Some(r) = SelRect::from_corners(*start, c).clip(w, h) {
                paint_marching_ants(painter, to_screen(r), phase);
                animated = true;
            }
        }
        // The lasso outline being drawn, closed back to its start.
        if let Some((points, _)) = &self.lasso {
            let center = |p: &(i32, i32)| t.pixel_to_screen(p.0 as f32 + 0.5, p.1 as f32 + 0.5);
            let mut line: Vec<Pos2> = points.iter().map(center).collect();
            if let Some(first) = line.first().copied() {
                line.push(first);
            }
            painter.add(egui::Shape::line(
                line.clone(),
                Stroke::new(3.0_f32, Color32::BLACK),
            ));
            painter.add(egui::Shape::line(
                line,
                Stroke::new(1.0_f32, Color32::WHITE),
            ));
        }
        if !animated {
            return;
        }
        if !options.reduce_motion {
            painter
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(80));
        }
    }

    /// Small hints in the top-left corner: preview state and onion skin legend.
    fn paint_badges(
        &self,
        painter: &egui::Painter,
        viewport: Rect,
        doc: &Document,
        options: &CanvasOptions,
        onion_visible: bool,
    ) {
        let font = theme::font(1);
        let origin = viewport.min + Vec2::new(12.0, 10.0);
        let text = if options.playing {
            format!(
                "Previewing  ·  frame {} / {}  ·  Enter to stop",
                doc.active_frame_index() + 1,
                doc.frame_count()
            )
        } else if options.onion.enabled {
            if onion_visible {
                "Onion skin  ·  previous frame warm, next frame cool  ·  O to hide".to_owned()
            } else {
                "Onion skin on  ·  no neighbor frames to show".to_owned()
            }
        } else {
            return;
        };
        let galley = painter.layout_no_wrap(text, font, theme::text_primary());
        let dot = 10.0;
        let rect = Rect::from_min_size(
            origin,
            Vec2::new(galley.size().x + dot + 22.0, galley.size().y + 10.0),
        );
        painter.rect_filled(rect, 0.0, theme::panel().gamma_multiply(0.92));
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0_f32, theme::border()),
            StrokeKind::Inside,
        );
        let dot_center = Pos2::new(rect.min.x + 10.0 + dot / 2.0, rect.center().y);
        let color = if options.playing {
            theme::accent()
        } else {
            theme::success()
        };
        let pulse = if options.playing && !options.reduce_motion {
            let t = painter.ctx().input(|i| i.time);
            0.55 + 0.45 * ((t * 4.0).sin() as f32 * 0.5 + 0.5)
        } else {
            1.0
        };
        painter.circle_filled(
            dot_center,
            dot / 2.0 * pulse.max(0.6),
            color.gamma_multiply(pulse),
        );
        painter.galley(
            Pos2::new(dot_center.x + dot / 2.0 + 8.0, rect.min.y + 5.0),
            galley,
            theme::text_primary(),
        );
    }

    fn ensure_checker(&mut self, ctx: &egui::Context) {
        let colors = (theme::checker_light(), theme::checker_dark());
        if self.checker_colors != Some(colors) {
            self.checker = None;
            self.checker_colors = Some(colors);
        }
        if self.checker.is_some() {
            return;
        }
        let (l, d) = (theme::checker_light(), theme::checker_dark());
        let mut bytes = Vec::with_capacity(16);
        for c in [l, d, d, l] {
            bytes.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
        let img = ColorImage::from_rgba_unmultiplied([2, 2], &bytes);
        self.checker =
            Some(ctx.load_texture("kagero-checker", img, TextureOptions::NEAREST_REPEAT));
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

/// Most points one lasso outline keeps.
const MAX_LASSO_POINTS: usize = 20_000;
/// Selection outlines with more edges than this are drawn as their bounds.
const MAX_OUTLINE_SEGMENTS: usize = 60_000;

/// A selection drag in progress (Select tool).
enum SelectDrag {
    /// Dragging out a new rectangle from `start`, combined with the selection by `mode`.
    New { start: (i32, i32), mode: SelectMode },
    /// Moving the selected pixels; `ghost` shows them under the cursor.
    Move {
        start: (i32, i32),
        ghost: Option<TextureHandle>,
    },
    /// Ctrl+drag: moving only the outline.
    Outline { start: (i32, i32) },
}

impl SelectDrag {
    fn start(&self) -> (i32, i32) {
        match self {
            SelectDrag::New { start, .. }
            | SelectDrag::Move { start, .. }
            | SelectDrag::Outline { start } => *start,
        }
    }
}

/// Dashed black/white lines; dashes line up across segments that meet, so a mask
/// outline looks like one continuous line.
fn paint_ants_segments(
    painter: &egui::Painter,
    segments: impl Iterator<Item = (Pos2, Pos2)>,
    phase: f32,
) {
    const DASH: f32 = 4.0;
    for (a, b) in segments {
        let len = a.distance(b);
        if len <= 0.0 {
            continue;
        }
        let dir = (b - a) / len;
        // Position along a diagonal pattern, so meeting segments continue each other.
        let mut travelled = phase + (a.x + a.y).rem_euclid(2.0 * DASH);
        let mut t = 0.0;
        while t < len {
            let seg = (DASH - travelled % DASH).min(len - t);
            let dark = ((travelled / DASH) as i64) % 2 == 0;
            let color = if dark { Color32::BLACK } else { Color32::WHITE };
            painter.line_segment(
                [a + dir * t, a + dir * (t + seg)],
                Stroke::new(1.0_f32, color),
            );
            t += seg;
            travelled += seg;
        }
    }
}

/// Dashed black/white outline; `phase` (0-8 px) animates the dashes.
fn paint_marching_ants(painter: &egui::Painter, rect: Rect, phase: f32) {
    const DASH: f32 = 4.0;
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    let mut travelled = phase;
    for edge in corners.windows(2) {
        let (a, b) = (edge[0], edge[1]);
        let len = a.distance(b);
        if len <= 0.0 {
            continue;
        }
        let dir = (b - a) / len;
        let mut t = 0.0;
        while t < len {
            let seg = (DASH - travelled % DASH).min(len - t);
            let dark = ((travelled / DASH) as i64) % 2 == 0;
            let color = if dark { Color32::BLACK } else { Color32::WHITE };
            painter.line_segment(
                [a + dir * t, a + dir * (t + seg)],
                Stroke::new(1.0_f32, color),
            );
            t += seg;
            travelled += seg;
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
        // One cell per image pixel whenever a pixel is at least 3 device pixels.
        for zoom in [3.0, 4.0, 6.0, 16.0, 96.0] {
            assert_eq!(checker_cell_pixels(zoom), 1.0);
        }
        // Zoomed far out, cells merge into power-of-two pixel blocks.
        assert_eq!(checker_cell_pixels(2.0), 2.0);
        assert_eq!(checker_cell_pixels(1.0), 4.0);
        assert_eq!(checker_cell_pixels(0.25), 16.0);
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

/// What a Crop-tool drag changes.
#[derive(Debug, Clone, Copy)]
enum CropDrag {
    New {
        start: (i32, i32),
    },
    Move {
        start: (i32, i32),
        orig: SelRect,
    },
    /// Left, right, top, bottom edges being dragged.
    Edges {
        edges: (bool, bool, bool, bool),
        orig: SelRect,
    },
}

/// Which edges of the crop frame are under the pointer (within a few screen pixels).
fn crop_edges(t: &ViewTransform, r: SelRect, pos: Pos2) -> (bool, bool, bool, bool) {
    const GRAB: f32 = 6.0;
    let min = t.pixel_to_screen(r.x as f32, r.y as f32);
    let max = t.pixel_to_screen(r.right() as f32, r.bottom() as f32);
    let in_x = pos.x > min.x - GRAB && pos.x < max.x + GRAB;
    let in_y = pos.y > min.y - GRAB && pos.y < max.y + GRAB;
    (
        in_y && (pos.x - min.x).abs() <= GRAB,
        in_y && (pos.x - max.x).abs() <= GRAB,
        in_x && (pos.y - min.y).abs() <= GRAB,
        in_x && (pos.y - max.y).abs() <= GRAB,
    )
}

/// The crop frame: everything outside it dimmed, an amber outline with handles and
/// its size.
fn paint_crop_frame(painter: &egui::Painter, t: &ViewTransform, image: Rect, r: SelRect) {
    let frame = Rect::from_min_max(
        t.pixel_to_screen(r.x as f32, r.y as f32),
        t.pixel_to_screen(r.right() as f32, r.bottom() as f32),
    );
    let shade = Color32::from_black_alpha(150);
    // Four bands around the frame, inside the image.
    for band in [
        Rect::from_min_max(image.min, Pos2::new(image.max.x, frame.min.y)),
        Rect::from_min_max(Pos2::new(image.min.x, frame.max.y), image.max),
        Rect::from_min_max(
            Pos2::new(image.min.x, frame.min.y),
            Pos2::new(frame.min.x, frame.max.y),
        ),
        Rect::from_min_max(
            Pos2::new(frame.max.x, frame.min.y),
            Pos2::new(image.max.x, frame.max.y),
        ),
    ] {
        if band.is_positive() {
            painter.rect_filled(band, 0.0, shade);
        }
    }
    painter.rect_stroke(
        frame,
        0.0,
        Stroke::new(2.0_f32, theme::accent()),
        StrokeKind::Outside,
    );
    let handle = Vec2::splat(7.0);
    for p in [
        frame.min,
        frame.max,
        Pos2::new(frame.min.x, frame.max.y),
        Pos2::new(frame.max.x, frame.min.y),
        Pos2::new(frame.center().x, frame.min.y),
        Pos2::new(frame.center().x, frame.max.y),
        Pos2::new(frame.min.x, frame.center().y),
        Pos2::new(frame.max.x, frame.center().y),
    ] {
        let h = Rect::from_center_size(p, handle);
        painter.rect_filled(h, 0.0, theme::accent());
        painter.rect_stroke(
            h,
            0.0,
            Stroke::new(1.0_f32, theme::bevel_dark()),
            StrokeKind::Outside,
        );
    }
    painter.text(
        Pos2::new(frame.center().x, frame.max.y + 10.0),
        egui::Align2::CENTER_TOP,
        format!("{} × {}", r.w, r.h),
        egui::FontId::proportional(13.0),
        theme::text_primary(),
    );
}

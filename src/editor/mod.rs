//! Application layer: one open document, its tools, and undoable editing commands.
//!
//! The UI never touches pixel buffers directly; it calls the methods here, which record
//! every change in the [`History`].

mod history;
mod paint_ops;
mod selection_ops;
mod tools;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use history::{Applied, Change, Dirty, History, PixelChange};
pub use paint_ops::{CustomBrush, PaletteScope, MAX_BRUSH_SIDE};
pub use selection_ops::{Clipboard, Transform};
pub use tools::{SampleSource, ToolKind, ToolSettings, BRUSH_SIZE_RANGE};

use crate::core::animation::clamp_duration;
use crate::core::tags::{self, FrameTag};
use crate::core::{
    raster, sanitize_layer_name, Anchor, CoreError, Document, FrameId, LayerId, LayerProps, Rgba,
};

/// Errors from editing commands, shown to the user in the status bar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("the active layer is hidden — show it to draw on it")]
    HiddenLayer,
    #[error("layer name cannot be empty")]
    EmptyName,
    #[error("stop the animation preview to edit")]
    ReadOnly,
    #[error("select an area first (M, then drag on the canvas)")]
    NoSelection,
    #[error("the active layer is locked — unlock it to change it")]
    LockedLayer,
    #[error("{0}")]
    NotPossible(&'static str),
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// Which mouse button started an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSlot {
    Primary,
    Secondary,
}

/// A drawing gesture in progress (one undo step).
#[derive(Debug)]
struct Stroke {
    tool: ToolKind,
    frame: FrameId,
    layer: LayerId,
    color: Rgba,
    start: (i32, i32),
    last: (i32, i32),
    /// Whether the stroke has painted at least once.
    drawn: bool,
    /// Original color of every pixel touched during the stroke.
    before: HashMap<u32, Rgba>,
    /// Last painted pixels of a pixel-perfect stroke (newest last).
    trail: Vec<(i32, i32)>,
    /// Polygon: the corners clicked so far.
    corners: Vec<(i32, i32)>,
}

/// An open document with its editing state.
#[derive(Debug)]
pub struct Editor {
    doc: Document,
    history: History,
    path: Option<PathBuf>,
    name: String,
    saved_state: u64,
    pub tool: ToolKind,
    pub settings: ToolSettings,
    pub primary: Rgba,
    pub secondary: Rgba,
    stroke: Option<Stroke>,
    dirty: Option<Dirty>,
    read_only: bool,
    /// Pixels copied with Ctrl+C / Ctrl+X (kept when another document opens).
    /// The Crop tool's frame while it is being adjusted (not part of the document).
    pub crop_rect: Option<crate::core::selection::SelRect>,
    pub clipboard: Option<Clipboard>,
    /// Last color painted with, for the "recent colors" list.
    used_color: Option<Rgba>,
    /// Brush tip made from a selection (Edit → Use Selection as Brush).
    pub custom_brush: Option<CustomBrush>,
}

impl Editor {
    /// Wraps a document. `path` is where it is saved as `.kgr` (if anywhere).
    pub fn new(doc: Document, path: Option<PathBuf>, name: String) -> Self {
        Self {
            doc,
            history: History::default(),
            path,
            name,
            saved_state: 0,
            tool: ToolKind::Pencil,
            settings: ToolSettings::default(),
            primary: Rgba::BLACK,
            secondary: Rgba::WHITE,
            stroke: None,
            dirty: Some(Dirty::Full),
            read_only: false,
            crop_rect: None,
            clipboard: None,
            used_color: None,
            custom_brush: None,
        }
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Display name (file name or "Untitled").
    pub fn name(&self) -> &str {
        &self.name
    }

    /// True when the document differs from the last saved (or created) state.
    pub fn is_modified(&self) -> bool {
        self.history.state_id() != self.saved_state || self.stroke.is_some()
    }

    /// Marks the document as having unsaved changes (e.g. opened from a recovery
    /// copy), so closing it asks first.
    pub fn mark_unsaved(&mut self) {
        self.saved_state = u64::MAX;
    }

    /// Remembers that the current state is saved at `path`.
    pub fn mark_saved(&mut self, path: PathBuf) {
        self.name = display_name(&path);
        self.path = Some(path);
        self.saved_state = self.history.state_id();
    }

    /// Takes the region that must be re-uploaded to the canvas texture.
    pub fn take_dirty(&mut self) -> Option<Dirty> {
        self.dirty.take()
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty = Some(Dirty::Full);
    }

    fn add_dirty(&mut self, d: Dirty) {
        self.dirty = Some(self.dirty.map_or(d, |old| old.union(d)));
    }

    pub fn color(&self, slot: ColorSlot) -> Rgba {
        match slot {
            ColorSlot::Primary => self.primary,
            ColorSlot::Secondary => self.secondary,
        }
    }

    pub fn set_color(&mut self, slot: ColorSlot, color: Rgba) {
        match slot {
            ColorSlot::Primary => self.primary = color,
            ColorSlot::Secondary => self.secondary = color,
        }
    }

    /// The color used by the last stroke or fill, once.
    pub fn take_used_color(&mut self) -> Option<Rgba> {
        self.used_color.take()
    }

    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.primary, &mut self.secondary);
    }

    pub fn set_tool(&mut self, tool: ToolKind) {
        if self.tool != tool {
            self.end_stroke();
            if self.tool == ToolKind::Crop {
                self.crop_rect = None;
            }
            self.tool = tool;
        }
    }

    // ---------------------------------------------------------------- drawing

    pub fn is_stroking(&self) -> bool {
        self.stroke.is_some()
    }

    /// Starts a tool action at image pixel `pos`.
    ///
    /// Fill and Eyedropper act immediately; drawing tools start a stroke that continues
    /// with [`Editor::update_stroke`] and finishes with [`Editor::end_stroke`].
    pub fn begin_stroke(&mut self, pos: (i32, i32), slot: ColorSlot) -> Result<(), EditError> {
        self.end_stroke();
        if self.read_only && self.tool != ToolKind::Pan {
            return Err(EditError::ReadOnly);
        }
        match self.tool {
            // Pan, the selection tools and Crop are handled by the canvas.
            ToolKind::Pan
            | ToolKind::Select
            | ToolKind::Lasso
            | ToolKind::MagicWand
            | ToolKind::Crop => Ok(()),
            ToolKind::Eyedropper => {
                if let Some(c) = self.sample(pos) {
                    self.set_color(slot, c);
                }
                Ok(())
            }
            ToolKind::Fill => {
                let color = self.color(slot);
                self.fill(pos, color)?;
                self.used_color = Some(color);
                Ok(())
            }
            tool => {
                let layer = self.doc.active_layer();
                if !layer.visible {
                    return Err(EditError::HiddenLayer);
                }
                if layer.locked {
                    return Err(EditError::LockedLayer);
                }
                let color = if tool == ToolKind::Eraser {
                    Rgba::TRANSPARENT
                } else {
                    self.color(slot)
                };
                if tool != ToolKind::Eraser {
                    self.used_color = Some(color);
                }
                self.stroke = Some(Stroke {
                    tool,
                    frame: self.doc.active_frame().id,
                    layer: layer.id,
                    color,
                    start: pos,
                    last: pos,
                    drawn: false,
                    before: HashMap::new(),
                    trail: Vec::new(),
                    corners: vec![pos],
                });
                self.update_stroke(pos);
                Ok(())
            }
        }
    }

    /// Continues the current stroke to `pos`. Freehand tools connect the points with a line.
    pub fn update_stroke(&mut self, pos: (i32, i32)) {
        let Some(mut stroke) = self.stroke.take() else {
            return;
        };
        if stroke.drawn && pos == stroke.last {
            self.stroke = Some(stroke);
            return;
        }
        stroke.drawn = true;
        let tip = self.brush_tip();
        let dot = [(0, 0)];
        let (w, h) = (self.doc.width(), self.doc.height());
        match stroke.tool {
            ToolKind::Pencil | ToolKind::Eraser => {
                let from = stroke.last;
                let mut points = Vec::new();
                raster::line(from, pos, |x, y| points.push((x, y)));
                let pixel_perfect = self.settings.pixel_perfect && tip == dot;
                for (x, y) in points {
                    if pixel_perfect {
                        self.pixel_perfect_step(&mut stroke, (x, y));
                    }
                    self.stamp(&mut stroke, x, y, &tip);
                }
            }
            ToolKind::Line
            | ToolKind::Rectangle
            | ToolKind::FilledRectangle
            | ToolKind::Ellipse => {
                // Shapes are previewed by restoring the previous preview and redrawing.
                self.restore(&mut stroke);
                let mut points = Vec::new();
                let filled = match stroke.tool {
                    ToolKind::Line => {
                        raster::line(stroke.start, pos, |x, y| points.push((x, y)));
                        false
                    }
                    ToolKind::Rectangle => {
                        raster::rect_outline(stroke.start, pos, |x, y| points.push((x, y)));
                        false
                    }
                    ToolKind::Ellipse => {
                        let filled = self.settings.shape_filled;
                        raster::ellipse(stroke.start, pos, filled, w, h, |x, y| {
                            points.push((x, y))
                        });
                        filled
                    }
                    _ => {
                        raster::rect_filled(stroke.start, pos, w, h, |x, y| points.push((x, y)));
                        true
                    }
                };
                // Filled shapes are filled pixel by pixel; outlines use the brush.
                let tip: &[(i32, i32)] = if filled { &dot } else { &tip };
                for (x, y) in points {
                    self.stamp(&mut stroke, x, y, tip);
                }
            }
            ToolKind::Polygon => self.draw_polygon(&mut stroke, Some(pos)),
            _ => {}
        }
        stroke.last = pos;
        self.stroke = Some(stroke);
    }

    /// Finishes the current stroke and records it as one undo step.
    pub fn end_stroke(&mut self) {
        let Some(mut stroke) = self.stroke.take() else {
            return;
        };
        if stroke.tool == ToolKind::Polygon {
            // Drop the line to the cursor: the shape closes on its corners.
            self.draw_polygon(&mut stroke, None);
        }
        let Some(cel) = self.doc.cel_by_id(stroke.frame, stroke.layer) else {
            return;
        };
        let mut changes: Vec<PixelChange> = stroke
            .before
            .iter()
            .filter_map(|(&index, &before)| {
                let after = cel.get_index(index)?;
                (after != before).then_some(PixelChange {
                    index,
                    before,
                    after,
                })
            })
            .collect();
        if changes.is_empty() {
            return;
        }
        changes.sort_unstable_by_key(|c| c.index);
        self.history.push(Change::Pixels {
            frame: stroke.frame,
            layer: stroke.layer,
            changes,
        });
    }

    /// Aborts the current stroke and restores the touched pixels.
    pub fn cancel_stroke(&mut self) {
        if let Some(mut stroke) = self.stroke.take() {
            self.restore(&mut stroke);
        }
    }

    /// The pixels of the brush tip around the cursor (shape and size from the settings).
    pub fn brush_tip(&self) -> Vec<(i32, i32)> {
        let size = self.settings.brush_size;
        match self.settings.brush_shape {
            raster::BrushShape::Square => raster::brush_offsets(size).collect(),
            raster::BrushShape::Circle => raster::circle_offsets(size),
            raster::BrushShape::Custom => match &self.custom_brush {
                Some(b) => b.offsets.clone(),
                None => raster::brush_offsets(size).collect(),
            },
        }
    }

    /// Where a stamp at `(x, y)` lands with symmetry on: the point itself and its
    /// mirror images, with whether the tip is flipped horizontally / vertically.
    fn mirrored(&self, x: i32, y: i32) -> Vec<(i32, i32, bool, bool)> {
        let (w, h) = (self.doc.width() as i32, self.doc.height() as i32);
        let s = self.settings;
        let mut out = vec![(x, y, false, false)];
        if s.mirror_x {
            out.push((w - 1 - x, y, true, false));
        }
        if s.mirror_y {
            out.push((x, h - 1 - y, false, true));
        }
        if s.mirror_x && s.mirror_y {
            out.push((w - 1 - x, h - 1 - y, true, true));
        }
        out
    }

    /// Paints the brush `tip` centered at `(x, y)` (and its mirror images), skipping
    /// pixels the dither pattern leaves out, and remembering original colors.
    fn stamp(&mut self, stroke: &mut Stroke, x: i32, y: i32, tip: &[(i32, i32)]) {
        let alpha_lock = self.doc.layer(stroke.layer).is_some_and(|l| l.alpha_lock);
        let dither = self.settings.dither;
        let centers = self.mirrored(x, y);
        let Some(cel) = self.doc.cel_by_id_mut(stroke.frame, stroke.layer) else {
            return;
        };
        let mut dirty: Option<Dirty> = None;
        let pixels = centers.iter().flat_map(|&(cx, cy, fx, fy)| {
            tip.iter().map(move |&(dx, dy)| {
                let dx = if fx { -dx } else { dx };
                let dy = if fy { -dy } else { dy };
                (cx.saturating_add(dx), cy.saturating_add(dy))
            })
        });
        for (px, py) in pixels {
            if !dither.paints(px, py) {
                continue;
            }
            let Some(index) = cel.index_of(px, py) else {
                continue;
            };
            let Some(old) = cel.get_index(index) else {
                continue;
            };
            let original = *stroke.before.entry(index).or_insert(old);
            // Alpha lock looks at the pixel as it was before the stroke.
            let Some(color) = alpha_locked(original, stroke.color, alpha_lock) else {
                continue;
            };
            if old != color {
                cel.set_index(index, color);
                let d = Dirty::pixel(px as u32, py as u32);
                dirty = Some(dirty.map_or(d, |a| a.union(d)));
            }
        }
        if let Some(d) = dirty {
            self.add_dirty(d);
        }
    }

    /// Puts back every pixel the stroke has touched so far.
    /// Pixel-perfect strokes: before painting `next`, takes back the previous pixel
    /// when it only forms an "L" corner between its neighbors (a doubled pixel).
    fn pixel_perfect_step(&mut self, stroke: &mut Stroke, next: (i32, i32)) {
        if stroke.trail.last() == Some(&next) {
            return;
        }
        if let [.., a, b] = stroke.trail[..] {
            let diagonal = (next.0 - a.0).abs() == 1 && (next.1 - a.1).abs() == 1;
            let corner = (b.0 == a.0 && b.1 == next.1) || (b.1 == a.1 && b.0 == next.0);
            if diagonal && corner {
                self.unstamp(stroke, b);
                stroke.trail.pop();
            }
        }
        stroke.trail.push(next);
        if stroke.trail.len() > 3 {
            stroke.trail.remove(0);
        }
    }

    /// Gives one pixel of the stroke (and its mirror images) its original color back.
    fn unstamp(&mut self, stroke: &mut Stroke, (x, y): (i32, i32)) {
        for (px, py, _, _) in self.mirrored(x, y) {
            let Some(cel) = self.doc.cel_by_id_mut(stroke.frame, stroke.layer) else {
                return;
            };
            let Some(index) = cel.index_of(px, py) else {
                continue;
            };
            if let Some(&before) = stroke.before.get(&index) {
                cel.set_index(index, before);
                self.add_dirty(Dirty::pixel(px as u32, py as u32));
            }
        }
    }

    fn restore(&mut self, stroke: &mut Stroke) {
        let w = self.doc.width();
        let Some(cel) = self.doc.cel_by_id_mut(stroke.frame, stroke.layer) else {
            return;
        };
        let mut dirty: Option<Dirty> = None;
        for (&index, &before) in &stroke.before {
            cel.set_index(index, before);
            let d = Dirty::pixel(index % w, index / w);
            dirty = Some(dirty.map_or(d, |a| a.union(d)));
        }
        stroke.before.clear();
        if let Some(d) = dirty {
            self.add_dirty(d);
        }
    }

    /// Fills the area at `pos` with `color` (one undo step). Which pixels: those
    /// similar to the clicked one within the fill tolerance, connected to it unless
    /// contiguous is off, judged on the active layer or the merged image; inside the
    /// selection when there is one; thinned out by the dither pattern.
    pub fn fill(&mut self, pos: (i32, i32), color: Rgba) -> Result<(), EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        let layer_ref = self.doc.active_layer();
        if !layer_ref.visible {
            return Err(EditError::HiddenLayer);
        }
        if layer_ref.locked {
            return Err(EditError::LockedLayer);
        }
        let alpha_lock = layer_ref.alpha_lock;
        let layer_id = layer_ref.id;
        let frame_id = self.doc.active_frame().id;
        let (w, h) = (self.doc.width() as i32, self.doc.height() as i32);
        if pos.0 < 0 || pos.1 < 0 || pos.0 >= w || pos.1 >= h {
            return Ok(());
        }
        let selection = self.doc.selection_shape();
        if selection.is_some_and(|s| !s.contains(pos.0, pos.1)) {
            return Ok(()); // outside the selection nothing can be filled
        }
        let s = self.settings;
        let merged;
        let source = match s.fill_source {
            SampleSource::ActiveLayer => self.doc.active_cel(),
            SampleSource::Merged => {
                merged = self.doc.composite_frame(self.doc.active_frame_index())?;
                &merged
            }
        };
        let mask = crate::core::region::similar_region(
            source,
            pos.0,
            pos.1,
            s.fill_tolerance,
            s.fill_contiguous,
        );
        let cel = self.doc.active_cel();
        let changes: Vec<PixelChange> = mask
            .iter()
            .enumerate()
            .filter(|(_, m)| **m)
            .filter_map(|(i, _)| {
                let (x, y) = ((i as i32) % w, (i as i32) / w);
                if !s.dither.paints(x, y) || selection.is_some_and(|sel| !sel.contains(x, y)) {
                    return None;
                }
                let index = i as u32;
                let before = cel.get_index(index)?;
                let after = alpha_locked(before, color, alpha_lock)?;
                (after != before).then_some(PixelChange {
                    index,
                    before,
                    after,
                })
            })
            .collect();
        if changes.is_empty() {
            return Ok(());
        }
        let mut change = Change::Pixels {
            frame: frame_id,
            layer: layer_id,
            changes,
        };
        let dirty = change.dirty(&self.doc);
        change.apply(&mut self.doc)?;
        if let Some(d) = dirty {
            self.add_dirty(d);
        }
        self.history.push(change);
        Ok(())
    }

    /// Reads the color under `pos` according to the eyedropper sample source.
    pub fn sample(&self, pos: (i32, i32)) -> Option<Rgba> {
        match self.settings.sample_source {
            SampleSource::ActiveLayer => self.doc.active_cel().get(pos.0, pos.1),
            SampleSource::Merged => self.doc.composite_pixel(pos.0, pos.1),
        }
    }

    // ---------------------------------------------------------------- history

    pub fn undo(&mut self) -> Result<Option<&'static str>, EditError> {
        self.cancel_stroke();
        let applied = self.history.undo(&mut self.doc)?;
        Ok(self.after_history(applied))
    }

    pub fn redo(&mut self) -> Result<Option<&'static str>, EditError> {
        self.cancel_stroke();
        let applied = self.history.redo(&mut self.doc)?;
        Ok(self.after_history(applied))
    }

    fn after_history(&mut self, applied: Option<Applied>) -> Option<&'static str> {
        let applied = applied?;
        if let Some(d) = applied.dirty {
            self.add_dirty(d);
        }
        Some(applied.label)
    }

    /// Applies a change and records it.
    fn execute(&mut self, mut change: Change) -> Result<(), EditError> {
        self.end_stroke();
        change.apply(&mut self.doc)?;
        if let Some(d) = change.dirty(&self.doc) {
            self.add_dirty(d);
        }
        self.history.push(change);
        Ok(())
    }

    // ---------------------------------------------------------------- layers

    /// Selects the active layer (not an undo step).
    pub fn select_layer(&mut self, index: usize) {
        self.end_stroke();
        self.doc.set_active_index(index);
    }

    /// Adds an empty layer above the active one.
    pub fn add_layer(&mut self) -> Result<(), EditError> {
        self.end_stroke();
        let name = self.unique_layer_name();
        let layer = self.doc.make_empty_layer(name)?;
        let prev_active = self.doc.active_index();
        self.execute(Change::AddLayer {
            index: prev_active + 1,
            layer: Some(layer),
            prev_active,
        })
    }

    fn unique_layer_name(&self) -> String {
        let mut n = self.doc.layer_count() + 1;
        loop {
            let name = format!("Layer {n}");
            if !self.doc.layers().iter().any(|l| l.name == name) {
                return name;
            }
            n += 1;
        }
    }

    pub fn delete_layer(&mut self, index: usize) -> Result<(), EditError> {
        if self.doc.layer_count() <= 1 {
            return Err(CoreError::LastLayer.into());
        }
        if index >= self.doc.layer_count() {
            return Err(CoreError::LayerIndex(index).into());
        }
        let prev_active = self.doc.active_index();
        self.execute(Change::RemoveLayer {
            index,
            layer: None,
            prev_active,
        })
    }

    /// Moves a layer one step up (towards the top of the stack).
    pub fn move_layer_up(&mut self, index: usize) -> Result<(), EditError> {
        if index + 1 >= self.doc.layer_count() {
            return Ok(());
        }
        self.execute(Change::MoveLayer {
            from: index,
            to: index + 1,
        })
    }

    /// Moves a layer one step down (towards the bottom of the stack).
    pub fn move_layer_down(&mut self, index: usize) -> Result<(), EditError> {
        if index == 0 || index >= self.doc.layer_count() {
            return Ok(());
        }
        self.execute(Change::MoveLayer {
            from: index,
            to: index - 1,
        })
    }

    pub fn rename_layer(&mut self, index: usize, name: &str) -> Result<(), EditError> {
        let layer = self
            .doc
            .layers()
            .get(index)
            .ok_or(CoreError::LayerIndex(index))?;
        let name = sanitize_layer_name(name).ok_or(EditError::EmptyName)?;
        if layer.name == name {
            return Ok(());
        }
        let change = Change::RenameLayer {
            layer: layer.id,
            before: layer.name.clone(),
            after: name,
        };
        self.execute(change)
    }

    /// Changes a layer's opacity and locks as one undo step.
    pub fn set_layer_props(&mut self, index: usize, after: LayerProps) -> Result<(), EditError> {
        let layer = self
            .doc
            .layers()
            .get(index)
            .ok_or(CoreError::LayerIndex(index))?;
        let before = layer.props();
        if before == after {
            return Ok(());
        }
        let id = layer.id;
        self.execute(Change::SetLayerProps {
            layer: id,
            before,
            after,
        })
    }

    /// Shows a layer at `opacity` while a slider is dragged, without an undo step;
    /// finish with [`Editor::commit_layer_opacity`].
    pub fn preview_layer_opacity(&mut self, index: usize, opacity: u8) {
        let Some(id) = self.doc.layers().get(index).map(|l| l.id) else {
            return;
        };
        if let Some(l) = self.doc.layer_mut(id) {
            if l.opacity != opacity {
                l.opacity = opacity;
                self.doc.touch_structure();
                self.add_dirty(Dirty::Full);
            }
        }
    }

    /// Records a slider-dragged opacity change as one undo step (`before` = the value
    /// when the drag started).
    pub fn commit_layer_opacity(&mut self, index: usize, before: u8) -> Result<(), EditError> {
        let Some(layer) = self.doc.layers().get(index) else {
            return Ok(());
        };
        let after = layer.props();
        if after.opacity == before {
            return Ok(());
        }
        let id = layer.id;
        let before_props = LayerProps {
            opacity: before,
            ..after
        };
        // The document already shows `after`; record the step without re-applying.
        self.end_stroke();
        self.history.push(Change::SetLayerProps {
            layer: id,
            before: before_props,
            after,
        });
        Ok(())
    }

    /// Copies a layer (every frame) and puts the copy right above it.
    pub fn duplicate_layer(&mut self, index: usize) -> Result<(), EditError> {
        self.end_stroke();
        let name = self
            .doc
            .layers()
            .get(index)
            .map(|l| copy_name(&l.name, |n| self.doc.layers().iter().any(|x| x.name == n)))
            .ok_or(CoreError::LayerIndex(index))?;
        let layer = self.doc.make_duplicate_layer(index, name)?;
        let prev_active = self.doc.active_index();
        self.execute_labeled(
            "Duplicate Layer",
            vec![Change::AddLayer {
                index: index + 1,
                layer: Some(layer),
                prev_active,
            }],
        )
    }

    /// Whether the layer at `index` can be merged into the one below it, and why not.
    pub fn can_merge_down(&self, index: usize) -> Result<(), EditError> {
        let layers = self.doc.layers();
        if index == 0 || index >= layers.len() {
            return Err(EditError::NotPossible(
                "there is no layer below to merge into",
            ));
        }
        let (top, bottom) = (&layers[index], &layers[index - 1]);
        if !top.visible || !bottom.visible {
            return Err(EditError::NotPossible(
                "both layers must be visible to merge — show them first",
            ));
        }
        if top.locked || bottom.locked {
            return Err(EditError::LockedLayer);
        }
        Ok(())
    }

    /// Merges the layer at `index` into the one below, in every frame, as one undo
    /// step. The result looks exactly like the two layers did.
    pub fn merge_down(&mut self, index: usize) -> Result<(), EditError> {
        self.end_stroke();
        self.can_merge_down(index)?;
        let merged = self.doc.merged_layer(index, index - 1)?;
        let prev_active = self.doc.active_index();
        self.execute_labeled(
            "Merge Down",
            vec![
                Change::SwapLayer {
                    index: index - 1,
                    data: merged,
                },
                Change::RemoveLayer {
                    index,
                    layer: None,
                    prev_active,
                },
            ],
        )?;
        self.doc.set_active_index(index - 1);
        Ok(())
    }

    /// Changes the canvas size; existing pixels stay where `anchor` says. One undo step.
    pub fn resize_canvas(
        &mut self,
        width: u32,
        height: u32,
        anchor: Anchor,
    ) -> Result<(), EditError> {
        self.end_stroke();
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        if width == self.doc.width() && height == self.doc.height() {
            return Ok(());
        }
        let geometry = self.doc.resized_geometry(width, height, anchor)?;
        self.execute(Change::Geometry {
            label: "Canvas Size",
            geometry,
        })
    }

    /// Cuts the canvas down to a rectangle in every layer and frame. One undo step.
    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), EditError> {
        self.end_stroke();
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        if x == 0 && y == 0 && width == self.doc.width() && height == self.doc.height() {
            return Ok(());
        }
        let geometry = self.doc.cropped_geometry(x, y, width, height)?;
        self.execute(Change::Geometry {
            label: "Crop",
            geometry,
        })
    }

    /// Runs several changes as one undo step.
    fn execute_labeled(
        &mut self,
        label: &'static str,
        changes: Vec<Change>,
    ) -> Result<(), EditError> {
        self.execute(Change::Group { label, changes })
    }

    pub fn toggle_layer_visibility(&mut self, index: usize) -> Result<(), EditError> {
        let layer = self
            .doc
            .layers()
            .get(index)
            .ok_or(CoreError::LayerIndex(index))?;
        let change = Change::SetVisible {
            layer: layer.id,
            visible: !layer.visible,
        };
        self.execute(change)
    }

    // ---------------------------------------------------------------- frames

    /// Blocks pixel edits (used during animation preview).
    pub fn set_read_only(&mut self, read_only: bool) {
        if read_only {
            self.end_stroke();
        }
        self.read_only = read_only;
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Shows another frame on the canvas (not an undo step).
    pub fn select_frame(&mut self, index: usize) {
        if index == self.doc.active_frame_index() || index >= self.doc.frame_count() {
            return;
        }
        self.end_stroke();
        self.doc.set_active_frame(index);
        self.add_dirty(Dirty::Full);
    }

    /// Selects the next frame, wrapping to the first.
    pub fn next_frame(&mut self) {
        let n = self.doc.frame_count();
        self.select_frame((self.doc.active_frame_index() + 1) % n);
    }

    /// Selects the previous frame, wrapping to the last.
    pub fn previous_frame(&mut self) {
        let n = self.doc.frame_count();
        self.select_frame((self.doc.active_frame_index() + n - 1) % n);
    }

    /// Adds an empty frame after the active one, with the same duration.
    pub fn add_frame(&mut self) -> Result<(), EditError> {
        self.end_stroke();
        let duration = self.doc.active_frame().duration_ms();
        let frame = self.doc.make_empty_frame(duration)?;
        let prev_active = self.doc.active_frame_index();
        let tags = tags::after_insert(self.doc.tags(), prev_active + 1);
        self.execute_with_tags(
            Change::AddFrame {
                index: prev_active + 1,
                frame: Some(frame),
                prev_active,
                duplicate: false,
            },
            tags,
        )
    }

    /// Inserts an independent copy of the active frame right after it.
    pub fn duplicate_frame(&mut self) -> Result<(), EditError> {
        self.end_stroke();
        let prev_active = self.doc.active_frame_index();
        let frame = self.doc.make_duplicate_frame(prev_active)?;
        let tags = tags::after_insert(self.doc.tags(), prev_active + 1);
        self.execute_with_tags(
            Change::AddFrame {
                index: prev_active + 1,
                frame: Some(frame),
                prev_active,
                duplicate: true,
            },
            tags,
        )
    }

    pub fn delete_frame(&mut self, index: usize) -> Result<(), EditError> {
        if self.doc.frame_count() <= 1 {
            return Err(CoreError::LastFrame.into());
        }
        if index >= self.doc.frame_count() {
            return Err(CoreError::FrameIndex(index).into());
        }
        let prev_active = self.doc.active_frame_index();
        let tags = tags::after_remove(self.doc.tags(), index);
        self.execute_with_tags(
            Change::RemoveFrame {
                index,
                frame: None,
                prev_active,
            },
            tags,
        )
    }

    /// Whether a frame holds any visible content (used to confirm deletion).
    pub fn frame_has_content(&self, index: usize) -> bool {
        self.doc.frames().get(index).is_some_and(|f| {
            f.cels()
                .iter()
                .any(|c| c.as_bytes().chunks_exact(4).any(|p| p[3] != 0))
        })
    }

    pub fn move_frame_left(&mut self, index: usize) -> Result<(), EditError> {
        if index == 0 || index >= self.doc.frame_count() {
            return Ok(());
        }
        self.execute(Change::MoveFrame {
            from: index,
            to: index - 1,
        })
    }

    pub fn move_frame_right(&mut self, index: usize) -> Result<(), EditError> {
        if index + 1 >= self.doc.frame_count() {
            return Ok(());
        }
        self.execute(Change::MoveFrame {
            from: index,
            to: index + 1,
        })
    }

    /// Sets a frame's duration in milliseconds (clamped to the allowed range).
    pub fn set_frame_duration(&mut self, index: usize, ms: u32) -> Result<(), EditError> {
        let frame = self
            .doc
            .frames()
            .get(index)
            .ok_or(CoreError::FrameIndex(index))?;
        let after = clamp_duration(ms);
        if frame.duration_ms() == after {
            return Ok(());
        }
        let change = Change::SetFrameDuration {
            frame: frame.id,
            before: frame.duration_ms(),
            after,
        };
        self.execute(change)
    }

    /// A frame change together with the tags it leaves behind (one undo step).
    fn execute_with_tags(&mut self, change: Change, tags: Vec<FrameTag>) -> Result<(), EditError> {
        if tags == self.doc.tags() {
            return self.execute(change);
        }
        let label = change.label();
        self.execute(Change::Group {
            label,
            changes: vec![change, Change::Tags { label, tags }],
        })
    }

    // ---------------------------------------------------------------- frame tags

    /// A tag for `start..=end` with a fresh name and color (not added yet).
    pub fn suggest_tag(&self, start: usize, end: usize) -> FrameTag {
        let last = self.doc.frame_count().saturating_sub(1);
        FrameTag {
            name: tags::unused_tag_name(self.doc.tags()),
            start: start.min(end).min(last),
            end: start.max(end).min(last),
            color: tags::next_tag_color(self.doc.tags()),
        }
    }

    /// Tags `start..=end` with a fresh name and color; returns the new tag's index.
    pub fn add_tag(&mut self, start: usize, end: usize) -> Result<usize, EditError> {
        if start.max(end) >= self.doc.frame_count() {
            return Err(CoreError::FrameIndex(start.max(end)).into());
        }
        self.insert_tag(self.suggest_tag(start, end))
    }

    /// Adds `tag` (name cleaned, range checked); returns its index.
    pub fn insert_tag(&mut self, tag: FrameTag) -> Result<usize, EditError> {
        if self.doc.tags().len() >= tags::MAX_TAGS {
            return Err(EditError::NotPossible(
                "This document already has the most tags allowed",
            ));
        }
        let name =
            tags::clean_tag_name(&tag.name).ok_or(EditError::NotPossible("A tag needs a name"))?;
        let (start, end) = (tag.start.min(tag.end), tag.start.max(tag.end));
        if end >= self.doc.frame_count() {
            return Err(CoreError::FrameIndex(end).into());
        }
        let mut list = self.doc.tags().to_vec();
        list.push(FrameTag {
            name,
            start,
            end,
            color: tag.color,
        });
        let index = list.len() - 1;
        self.execute(Change::Tags {
            label: "New Tag",
            tags: list,
        })?;
        Ok(index)
    }

    /// Changes a tag's name, range or color.
    pub fn update_tag(&mut self, index: usize, tag: FrameTag) -> Result<(), EditError> {
        let count = self.doc.frame_count();
        let old = self
            .doc
            .tags()
            .get(index)
            .ok_or(EditError::NotPossible("That tag no longer exists"))?;
        let name =
            tags::clean_tag_name(&tag.name).ok_or(EditError::NotPossible("A tag needs a name"))?;
        let (start, end) = (tag.start.min(tag.end), tag.start.max(tag.end));
        if end >= count {
            return Err(CoreError::FrameIndex(end).into());
        }
        let tag = FrameTag {
            name,
            start,
            end,
            color: tag.color,
        };
        if *old == tag {
            return Ok(());
        }
        let mut list = self.doc.tags().to_vec();
        list[index] = tag;
        self.execute(Change::Tags {
            label: "Edit Tag",
            tags: list,
        })
    }

    pub fn remove_tag(&mut self, index: usize) -> Result<(), EditError> {
        if index >= self.doc.tags().len() {
            return Ok(());
        }
        let mut list = self.doc.tags().to_vec();
        list.remove(index);
        self.execute(Change::Tags {
            label: "Delete Tag",
            tags: list,
        })
    }
}

/// File name without extension, used as the document title.
pub fn display_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled".to_owned())
}

/// The color a pixel gets when painting `new` over `old`, respecting alpha lock:
/// with the lock, empty pixels stay empty and colored ones keep their alpha.
/// `None` = leave the pixel alone.
pub(crate) fn alpha_locked(old: Rgba, new: Rgba, alpha_lock: bool) -> Option<Rgba> {
    if !alpha_lock {
        return Some(new);
    }
    if old.a() == 0 || new.a() == 0 {
        return None;
    }
    Some(Rgba::new(new.r(), new.g(), new.b(), old.a()))
}

/// "Name copy", or "Name copy 2", … when that is taken.
fn copy_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let base = format!("{name} copy");
    let fits = |s: String| {
        s.chars()
            .take(crate::core::MAX_LAYER_NAME_CHARS)
            .collect::<String>()
    };
    let first = fits(base.clone());
    if !taken(&first) {
        return first;
    }
    (2..)
        .map(|n| fits(format!("{base} {n}")))
        .find(|n| !taken(n))
        .unwrap_or(first)
}

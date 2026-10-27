//! Application layer: one open document, its tools, and undoable editing commands.
//!
//! The UI never touches pixel buffers directly; it calls the methods here, which record
//! every change in the [`History`].

mod history;
mod tools;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use history::{Applied, Change, Dirty, History, PixelChange};
pub use tools::{SampleSource, ToolKind, ToolSettings, BRUSH_SIZE_RANGE};

use crate::core::animation::clamp_duration;
use crate::core::{raster, sanitize_layer_name, CoreError, Document, FrameId, LayerId, Rgba};

/// Errors from editing commands, shown to the user in the status bar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("the active layer is hidden — show it to draw on it")]
    HiddenLayer,
    #[error("layer name cannot be empty")]
    EmptyName,
    #[error("stop the animation preview to edit")]
    ReadOnly,
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

    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.primary, &mut self.secondary);
    }

    pub fn set_tool(&mut self, tool: ToolKind) {
        if self.tool != tool {
            self.end_stroke();
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
            ToolKind::Pan => Ok(()),
            ToolKind::Eyedropper => {
                if let Some(c) = self.sample(pos) {
                    self.set_color(slot, c);
                }
                Ok(())
            }
            ToolKind::Fill => self.fill(pos, self.color(slot)),
            tool => {
                let layer = self.doc.active_layer();
                if !layer.visible {
                    return Err(EditError::HiddenLayer);
                }
                let color = if tool == ToolKind::Eraser {
                    Rgba::TRANSPARENT
                } else {
                    self.color(slot)
                };
                self.stroke = Some(Stroke {
                    tool,
                    frame: self.doc.active_frame().id,
                    layer: layer.id,
                    color,
                    start: pos,
                    last: pos,
                    drawn: false,
                    before: HashMap::new(),
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
        let size = self.settings.brush_size;
        match stroke.tool {
            ToolKind::Pencil | ToolKind::Eraser => {
                let from = stroke.last;
                let mut points = Vec::new();
                raster::line(from, pos, |x, y| points.push((x, y)));
                for (x, y) in points {
                    self.stamp(&mut stroke, x, y, size);
                }
            }
            ToolKind::Line | ToolKind::Rectangle | ToolKind::FilledRectangle => {
                // Shapes are previewed by restoring the previous preview and redrawing.
                self.restore(&mut stroke);
                let mut points = Vec::new();
                match stroke.tool {
                    ToolKind::Line => raster::line(stroke.start, pos, |x, y| points.push((x, y))),
                    ToolKind::Rectangle => {
                        raster::rect_outline(stroke.start, pos, |x, y| points.push((x, y)))
                    }
                    _ => raster::rect_filled(
                        stroke.start,
                        pos,
                        self.doc.width(),
                        self.doc.height(),
                        |x, y| points.push((x, y)),
                    ),
                }
                let size = if stroke.tool == ToolKind::FilledRectangle {
                    1
                } else {
                    size
                };
                for (x, y) in points {
                    self.stamp(&mut stroke, x, y, size);
                }
            }
            _ => {}
        }
        stroke.last = pos;
        self.stroke = Some(stroke);
    }

    /// Finishes the current stroke and records it as one undo step.
    pub fn end_stroke(&mut self) {
        let Some(stroke) = self.stroke.take() else {
            return;
        };
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

    /// Paints a square brush centered at `(x, y)`, remembering original colors.
    fn stamp(&mut self, stroke: &mut Stroke, x: i32, y: i32, size: u32) {
        let Some(cel) = self.doc.cel_by_id_mut(stroke.frame, stroke.layer) else {
            return;
        };
        let mut dirty: Option<Dirty> = None;
        for (dx, dy) in raster::brush_offsets(size) {
            let (px, py) = (x.saturating_add(dx), y.saturating_add(dy));
            let Some(index) = cel.index_of(px, py) else {
                continue;
            };
            let Some(old) = cel.get_index(index) else {
                continue;
            };
            stroke.before.entry(index).or_insert(old);
            if old != stroke.color {
                cel.set_index(index, stroke.color);
                let d = Dirty::pixel(px as u32, py as u32);
                dirty = Some(dirty.map_or(d, |a| a.union(d)));
            }
        }
        if let Some(d) = dirty {
            self.add_dirty(d);
        }
    }

    /// Puts back every pixel the stroke has touched so far.
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

    /// Flood-fills the 4-connected region of identical color at `pos` (one undo step).
    pub fn fill(&mut self, pos: (i32, i32), color: Rgba) -> Result<(), EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        let layer_ref = self.doc.active_layer();
        if !layer_ref.visible {
            return Err(EditError::HiddenLayer);
        }
        let layer_id = layer_ref.id;
        let frame_id = self.doc.active_frame().id;
        let cel = self.doc.active_cel();
        let Some(target) = cel.get(pos.0, pos.1) else {
            return Ok(());
        };
        if target == color {
            return Ok(());
        }
        let region = raster::flood_region(cel, pos.0, pos.1);
        let changes: Vec<PixelChange> = region
            .into_iter()
            .map(|index| PixelChange {
                index,
                before: target,
                after: color,
            })
            .collect();
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
        self.execute(Change::AddFrame {
            index: prev_active + 1,
            frame: Some(frame),
            prev_active,
            duplicate: false,
        })
    }

    /// Inserts an independent copy of the active frame right after it.
    pub fn duplicate_frame(&mut self) -> Result<(), EditError> {
        self.end_stroke();
        let prev_active = self.doc.active_frame_index();
        let frame = self.doc.make_duplicate_frame(prev_active)?;
        self.execute(Change::AddFrame {
            index: prev_active + 1,
            frame: Some(frame),
            prev_active,
            duplicate: true,
        })
    }

    pub fn delete_frame(&mut self, index: usize) -> Result<(), EditError> {
        if self.doc.frame_count() <= 1 {
            return Err(CoreError::LastFrame.into());
        }
        if index >= self.doc.frame_count() {
            return Err(CoreError::FrameIndex(index).into());
        }
        let prev_active = self.doc.active_frame_index();
        self.execute(Change::RemoveFrame {
            index,
            frame: None,
            prev_active,
        })
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
}

/// File name without extension, used as the document title.
pub fn display_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled".to_owned())
}

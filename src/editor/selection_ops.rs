//! Selection, clipboard and flip/rotate commands of the [`Editor`].
//!
//! A selection is any set of pixels (see [`Selection`]): a rectangle, a lasso shape,
//! a magic-wand area, or several of them combined. Every edit works on the selected
//! pixels of the active layer of the current frame. It is computed on a copy of that
//! image, and only the pixels that changed (plus the selection before/after) are
//! recorded, so one command is exactly one undo step that also restores the
//! selection. Rules:
//!
//! * transparent pixels of moved, pasted or transformed content never erase what is
//!   underneath, and unselected pixels inside the selection's bounds never move;
//! * content moved or rotated past the image edge is cut off, and the selection is
//!   clipped to the image;
//! * without a selection, flip/rotate apply to the whole layer.

use super::{Change, EditError, Editor, PixelChange, SampleSource};
use crate::core::region;
use crate::core::selection::{self, Block, SelRect, SelectMode, Selection};
use crate::core::PixelBuffer;

/// Pixels on the clipboard, where they came from, and their shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clipboard {
    pub block: Block,
    pub x: i32,
    pub y: i32,
    /// The copied selection (at `x`, `y`).
    pub shape: Selection,
}

/// Pixel-exact transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    FlipHorizontal,
    FlipVertical,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl Transform {
    pub fn label(self) -> &'static str {
        match self {
            Transform::FlipHorizontal => "Flip Horizontal",
            Transform::FlipVertical => "Flip Vertical",
            Transform::Rotate90 => "Rotate 90°",
            Transform::Rotate180 => "Rotate 180°",
            Transform::Rotate270 => "Rotate 270°",
        }
    }

    fn apply(self, block: &Block) -> Block {
        match self {
            Transform::FlipHorizontal => block.flip_horizontal(),
            Transform::FlipVertical => block.flip_vertical(),
            Transform::Rotate90 => block.rotate_cw(),
            Transform::Rotate180 => block.rotate_180(),
            Transform::Rotate270 => block.rotate_ccw(),
        }
    }
}

impl Editor {
    /// The rectangle around the selection.
    pub fn selection(&self) -> Option<SelRect> {
        self.doc.selection()
    }

    /// The selected pixels.
    pub fn selection_shape(&self) -> Option<&Selection> {
        self.doc.selection_shape()
    }

    /// Selects a rectangle (navigation, not an undo step). Clipped to the image.
    pub fn set_selection(&mut self, rect: Option<SelRect>) {
        self.end_stroke();
        self.doc.set_selection(rect);
    }

    /// Combines `shape` with the selection (navigation, not an undo step).
    pub fn select(&mut self, shape: Option<Selection>, mode: SelectMode) {
        self.end_stroke();
        let (w, h) = (self.doc.width(), self.doc.height());
        let combined = Selection::combine(self.doc.selection_shape(), shape.as_ref(), mode, w, h);
        self.doc.set_selection_shape(combined);
    }

    /// Selects the pixels inside a freehand outline through `points`.
    pub fn select_lasso(&mut self, points: &[(i32, i32)], mode: SelectMode) {
        let (w, h) = (self.doc.width(), self.doc.height());
        let shape =
            region::polygon_mask(points, w, h).and_then(|(r, bits)| Selection::from_mask(r, &bits));
        self.select(shape, mode);
    }

    /// Magic wand: selects pixels similar to the one at `pos`, using the tool
    /// settings (tolerance, contiguous, active layer or merged image). Returns
    /// whether `pos` was inside the image.
    pub fn magic_wand(&mut self, pos: (i32, i32), mode: SelectMode) -> Result<bool, EditError> {
        let (w, h) = (self.doc.width(), self.doc.height());
        if pos.0 < 0 || pos.1 < 0 || pos.0 >= w as i32 || pos.1 >= h as i32 {
            return Ok(false);
        }
        let s = self.settings;
        let merged;
        let source = match s.wand_source {
            SampleSource::ActiveLayer => self.doc.active_cel(),
            SampleSource::Merged => {
                merged = self.doc.composite_frame(self.doc.active_frame_index())?;
                &merged
            }
        };
        let bits =
            region::similar_region(source, pos.0, pos.1, s.wand_tolerance, s.wand_contiguous);
        self.select(Selection::from_canvas(w, h, &bits), mode);
        Ok(true)
    }

    pub fn select_all(&mut self) {
        self.set_selection(Some(self.full_rect()));
    }

    pub fn deselect(&mut self) {
        self.set_selection(None);
    }

    /// Selects everything that isn't selected (everything when nothing is).
    pub fn invert_selection(&mut self) {
        self.end_stroke();
        let (w, h) = (self.doc.width(), self.doc.height());
        let inverted = Selection::inverted(self.doc.selection_shape(), w, h);
        self.doc.set_selection_shape(inverted);
    }

    fn full_rect(&self) -> SelRect {
        SelRect {
            x: 0,
            y: 0,
            w: self.doc.width(),
            h: self.doc.height(),
        }
    }

    /// The selected pixels of the active layer in the current frame (unselected
    /// pixels in the bounds are transparent).
    pub fn selection_block(&self) -> Option<Block> {
        let sel = self.doc.selection_shape()?;
        Some(Block::copy_selected(self.doc.active_cel(), sel))
    }

    fn check_editable(&self) -> Result<(), EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        if !self.doc.active_layer().visible {
            return Err(EditError::HiddenLayer);
        }
        if self.doc.active_layer().locked {
            return Err(EditError::LockedLayer);
        }
        Ok(())
    }

    /// Edits a copy of the active cel with `edit`, then records the difference (and an
    /// optional new selection) as one undo step. Returns whether anything changed.
    pub(super) fn edit_region(
        &mut self,
        label: &'static str,
        new_selection: Option<Option<Selection>>,
        edit: impl FnOnce(&mut PixelBuffer),
    ) -> Result<bool, EditError> {
        self.check_editable()?;
        self.end_stroke();
        let frame = self.doc.active_frame().id;
        let layer = self.doc.active_layer().id;
        let alpha_lock = self.doc.active_layer().alpha_lock;
        let before = self.doc.active_cel().clone();
        let mut after = before.clone();
        edit(&mut after);
        if alpha_lock {
            // Keep transparency as it was: empty pixels stay empty, alpha stays.
            let mut bytes = after.as_bytes().to_vec();
            for (b, a) in before
                .as_bytes()
                .chunks_exact(4)
                .zip(bytes.chunks_exact_mut(4))
            {
                let old = crate::core::Rgba([b[0], b[1], b[2], b[3]]);
                let new = crate::core::Rgba([a[0], a[1], a[2], a[3]]);
                let kept = super::alpha_locked(old, new, true).unwrap_or(old);
                a.copy_from_slice(&kept.0);
            }
            after = PixelBuffer::from_raw(after.width(), after.height(), bytes)?;
        }
        let changes: Vec<PixelChange> = before
            .as_bytes()
            .chunks_exact(4)
            .zip(after.as_bytes().chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| PixelChange {
                index: i as u32,
                before: crate::core::Rgba([a[0], a[1], a[2], a[3]]),
                after: crate::core::Rgba([b[0], b[1], b[2], b[3]]),
            })
            .collect();
        let old_sel = self.doc.selection_shape().cloned();
        let (w, h) = (self.doc.width(), self.doc.height());
        let new_sel = new_selection.map(|s| s.and_then(|s| s.clip(w, h)));
        let sel_changes = new_sel.as_ref().is_some_and(|s| *s != old_sel);
        if changes.is_empty() && !sel_changes {
            return Ok(false);
        }
        let mut group = Vec::new();
        if !changes.is_empty() {
            group.push(Change::Pixels {
                frame,
                layer,
                changes,
            });
        }
        if let (true, Some(after)) = (sel_changes, new_sel) {
            group.push(Change::SetSelection {
                before: old_sel,
                after,
            });
        }
        self.execute(Change::Group {
            label,
            changes: group,
        })?;
        Ok(true)
    }

    /// Clears the selected pixels to transparent.
    pub fn delete_selection(&mut self) -> Result<bool, EditError> {
        let Some(sel) = self.doc.selection_shape().cloned() else {
            return Ok(false);
        };
        self.edit_region("Delete", None, |cel| selection::clear_selected(cel, &sel))
    }

    /// Copies the selected pixels to the clipboard. Returns `false` without a selection.
    pub fn copy_selection(&mut self) -> bool {
        let Some(sel) = self.doc.selection_shape().cloned() else {
            return false;
        };
        let r = sel.bounds();
        self.clipboard = Some(Clipboard {
            block: Block::copy_selected(self.doc.active_cel(), &sel),
            x: r.x,
            y: r.y,
            shape: sel,
        });
        true
    }

    /// Copies, then clears the selected pixels (one undo step).
    pub fn cut_selection(&mut self) -> Result<bool, EditError> {
        self.check_editable()?;
        if !self.copy_selection() {
            return Ok(false);
        }
        let Some(sel) = self.doc.selection_shape().cloned() else {
            return Ok(false);
        };
        self.edit_region("Cut", None, |cel| selection::clear_selected(cel, &sel))
    }

    /// Pastes the clipboard where it was copied from (kept inside the image when it
    /// fits) and selects it with its shape.
    pub fn paste(&mut self) -> Result<bool, EditError> {
        let Some(clip) = self.clipboard.clone() else {
            return Ok(false);
        };
        let (w, h) = (self.doc.width() as i64, self.doc.height() as i64);
        let x = i64::from(clip.x).clamp(0, (w - i64::from(clip.block.w)).max(0)) as i32;
        let y = i64::from(clip.y).clamp(0, (h - i64::from(clip.block.h)).max(0)) as i32;
        let shape = clip.shape.translated(x - clip.x, y - clip.y);
        self.edit_region("Paste", Some(Some(shape)), |cel| {
            selection::stamp(cel, &clip.block, x, y)
        })
    }

    /// Moves the selected pixels (and the selection) by `(dx, dy)`.
    pub fn move_selection(&mut self, dx: i32, dy: i32) -> Result<bool, EditError> {
        let Some(sel) = self.doc.selection_shape().cloned() else {
            return Ok(false);
        };
        if dx == 0 && dy == 0 {
            return Ok(false);
        }
        let block = Block::copy_selected(self.doc.active_cel(), &sel);
        let target = sel.translated(dx, dy);
        let at = target.bounds();
        self.edit_region("Move Selection", Some(Some(target)), |cel| {
            selection::clear_selected(cel, &sel);
            selection::stamp(cel, &block, at.x, at.y);
        })
    }

    /// Moves only the selection, not the pixels (navigation, not an undo step).
    pub fn move_selection_outline(&mut self, dx: i32, dy: i32) {
        if let Some(sel) = self.doc.selection_shape() {
            let moved = sel.translated(dx, dy);
            self.end_stroke();
            self.doc.set_selection_shape(Some(moved));
        }
    }

    /// Stamps a copy of the selection next to it (right, else below, else 1 px
    /// diagonally) and selects the copy.
    pub fn duplicate_selection(&mut self) -> Result<bool, EditError> {
        let Some(sel) = self.doc.selection_shape().cloned() else {
            return Ok(false);
        };
        let rect = sel.bounds();
        let block = Block::copy_selected(self.doc.active_cel(), &sel);
        let (w, h) = (i64::from(self.doc.width()), i64::from(self.doc.height()));
        let (dx, dy) = if rect.right() + i64::from(rect.w) <= w {
            (rect.w as i32, 0)
        } else if rect.bottom() + i64::from(rect.h) <= h {
            (0, rect.h as i32)
        } else {
            (1, 1)
        };
        let target = sel.translated(dx, dy);
        let at = target.bounds();
        self.edit_region("Duplicate Selection", Some(Some(target)), |cel| {
            selection::stamp(cel, &block, at.x, at.y)
        })
    }

    /// Flips or rotates the selection (or the whole layer without one).
    pub fn transform(&mut self, kind: Transform) -> Result<bool, EditError> {
        let had_selection = self.doc.selection_shape().cloned();
        let sel = had_selection
            .clone()
            .unwrap_or_else(|| Selection::from(self.full_rect()));
        let rect = sel.bounds();
        let block = Block::copy_selected(self.doc.active_cel(), &sel);
        let out = kind.apply(&block);
        // Rotated blocks keep the same center (rounded down).
        let x = i64::from(rect.x) + (i64::from(rect.w) - i64::from(out.w)) / 2;
        let y = i64::from(rect.y) + (i64::from(rect.h) - i64::from(out.h)) / 2;
        let (x, y) = (
            x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        );
        let target = SelRect {
            x,
            y,
            w: out.w,
            h: out.h,
        };
        let new_selection = had_selection.map(|s| s.reshaped(target, |b| kind.apply(b)));
        self.edit_region(kind.label(), new_selection, |cel| {
            selection::clear_selected(cel, &sel);
            selection::stamp(cel, &out, x, y);
        })
    }

    /// Fails with a clear error when there is nothing to work on.
    pub fn require_selection(&self) -> Result<SelRect, EditError> {
        self.doc.selection().ok_or(EditError::NoSelection)
    }
}

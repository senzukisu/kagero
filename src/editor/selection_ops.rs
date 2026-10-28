//! Selection, clipboard and flip/rotate commands of the [`Editor`].
//!
//! Every edit works on the active layer of the current frame. It is computed on a copy
//! of that image, and only the pixels that changed (plus the selection before/after)
//! are recorded, so one command is exactly one undo step that also restores the
//! selection outline. Rules:
//!
//! * transparent pixels of moved, pasted or transformed content never erase what is
//!   underneath;
//! * content moved or rotated past the image edge is cut off, and the selection is
//!   clipped to the image;
//! * without a selection, flip/rotate apply to the whole layer.

use super::{Change, EditError, Editor, PixelChange};
use crate::core::selection::{self, Block, SelRect};
use crate::core::PixelBuffer;

/// Pixels on the clipboard and where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clipboard {
    pub block: Block,
    pub x: i32,
    pub y: i32,
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
}

impl Editor {
    pub fn selection(&self) -> Option<SelRect> {
        self.doc.selection()
    }

    /// Sets the selection outline (navigation, not an undo step). Clipped to the image.
    pub fn set_selection(&mut self, rect: Option<SelRect>) {
        self.end_stroke();
        self.doc.set_selection(rect);
    }

    pub fn select_all(&mut self) {
        self.set_selection(Some(SelRect {
            x: 0,
            y: 0,
            w: self.doc.width(),
            h: self.doc.height(),
        }));
    }

    pub fn deselect(&mut self) {
        self.set_selection(None);
    }

    fn full_rect(&self) -> SelRect {
        SelRect {
            x: 0,
            y: 0,
            w: self.doc.width(),
            h: self.doc.height(),
        }
    }

    /// The selected pixels of the active layer in the current frame.
    pub fn selection_block(&self) -> Option<Block> {
        let rect = self.doc.selection()?;
        Some(Block::copy_from(self.doc.active_cel(), rect))
    }

    fn check_editable(&self) -> Result<(), EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        if !self.doc.active_layer().visible {
            return Err(EditError::HiddenLayer);
        }
        Ok(())
    }

    /// Edits a copy of the active cel with `edit`, then records the difference (and an
    /// optional new selection) as one undo step. Returns whether anything changed.
    fn edit_region(
        &mut self,
        label: &'static str,
        new_selection: Option<Option<SelRect>>,
        edit: impl FnOnce(&mut PixelBuffer),
    ) -> Result<bool, EditError> {
        self.check_editable()?;
        self.end_stroke();
        let frame = self.doc.active_frame().id;
        let layer = self.doc.active_layer().id;
        let before = self.doc.active_cel().clone();
        let mut after = before.clone();
        edit(&mut after);
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
        let old_sel = self.doc.selection();
        let new_sel =
            new_selection.map(|s| s.and_then(|r| r.clip(self.doc.width(), self.doc.height())));
        let sel_changes = new_sel.is_some_and(|s| s != old_sel);
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
        let Some(rect) = self.doc.selection() else {
            return Ok(false);
        };
        self.edit_region("Delete", None, |cel| selection::clear(cel, rect))
    }

    /// Copies the selected pixels to the clipboard. Returns `false` without a selection.
    pub fn copy_selection(&mut self) -> bool {
        let Some(rect) = self.doc.selection() else {
            return false;
        };
        self.clipboard = Some(Clipboard {
            block: Block::copy_from(self.doc.active_cel(), rect),
            x: rect.x,
            y: rect.y,
        });
        true
    }

    /// Copies, then clears the selected pixels (one undo step).
    pub fn cut_selection(&mut self) -> Result<bool, EditError> {
        self.check_editable()?;
        if !self.copy_selection() {
            return Ok(false);
        }
        let Some(rect) = self.doc.selection() else {
            return Ok(false);
        };
        self.edit_region("Cut", None, |cel| selection::clear(cel, rect))
    }

    /// Pastes the clipboard where it was copied from (kept inside the image when it
    /// fits) and selects it.
    pub fn paste(&mut self) -> Result<bool, EditError> {
        let Some(clip) = self.clipboard.clone() else {
            return Ok(false);
        };
        let (w, h) = (self.doc.width() as i64, self.doc.height() as i64);
        let x = i64::from(clip.x).clamp(0, (w - i64::from(clip.block.w)).max(0)) as i32;
        let y = i64::from(clip.y).clamp(0, (h - i64::from(clip.block.h)).max(0)) as i32;
        let rect = SelRect {
            x,
            y,
            w: clip.block.w,
            h: clip.block.h,
        };
        self.edit_region("Paste", Some(Some(rect)), |cel| {
            selection::stamp(cel, &clip.block, x, y)
        })
    }

    /// Moves the selected pixels (and the outline) by `(dx, dy)`.
    pub fn move_selection(&mut self, dx: i32, dy: i32) -> Result<bool, EditError> {
        let Some(rect) = self.doc.selection() else {
            return Ok(false);
        };
        if dx == 0 && dy == 0 {
            return Ok(false);
        }
        let block = Block::copy_from(self.doc.active_cel(), rect);
        let target = rect.translated(dx, dy);
        self.edit_region("Move Selection", Some(Some(target)), |cel| {
            selection::clear(cel, rect);
            selection::stamp(cel, &block, target.x, target.y);
        })
    }

    /// Moves only the outline, not the pixels (navigation, not an undo step).
    pub fn move_selection_outline(&mut self, dx: i32, dy: i32) {
        if let Some(rect) = self.doc.selection() {
            self.set_selection(Some(rect.translated(dx, dy)));
        }
    }

    /// Stamps a copy of the selection next to it (right, else below, else 1 px
    /// diagonally) and selects the copy.
    pub fn duplicate_selection(&mut self) -> Result<bool, EditError> {
        let Some(rect) = self.doc.selection() else {
            return Ok(false);
        };
        let block = Block::copy_from(self.doc.active_cel(), rect);
        let (w, h) = (i64::from(self.doc.width()), i64::from(self.doc.height()));
        let (dx, dy) = if rect.right() + i64::from(rect.w) <= w {
            (rect.w as i32, 0)
        } else if rect.bottom() + i64::from(rect.h) <= h {
            (0, rect.h as i32)
        } else {
            (1, 1)
        };
        let target = rect.translated(dx, dy);
        self.edit_region("Duplicate Selection", Some(Some(target)), |cel| {
            selection::stamp(cel, &block, target.x, target.y)
        })
    }

    /// Flips or rotates the selection (or the whole layer without one).
    pub fn transform(&mut self, kind: Transform) -> Result<bool, EditError> {
        let had_selection = self.doc.selection();
        let rect = had_selection.unwrap_or_else(|| self.full_rect());
        let block = Block::copy_from(self.doc.active_cel(), rect);
        let out = match kind {
            Transform::FlipHorizontal => block.flip_horizontal(),
            Transform::FlipVertical => block.flip_vertical(),
            Transform::Rotate90 => block.rotate_cw(),
            Transform::Rotate180 => block.rotate_180(),
            Transform::Rotate270 => block.rotate_ccw(),
        };
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
        let new_selection = had_selection.map(|_| Some(target));
        self.edit_region(kind.label(), new_selection, |cel| {
            selection::clear(cel, rect);
            selection::stamp(cel, &out, x, y);
        })
    }

    /// Fails with a clear error when there is nothing to work on.
    pub fn require_selection(&self) -> Result<SelRect, EditError> {
        self.doc.selection().ok_or(EditError::NoSelection)
    }
}

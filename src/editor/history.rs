//! Undoable changes and the undo/redo history.
//!
//! Every change stores only what it needs to be reversed: pixel edits keep per-pixel
//! before/after values, layer deletions keep the removed layer, and so on. The history
//! never stores whole-document snapshots.

use std::collections::VecDeque;

use crate::core::{CoreError, Document, Layer, LayerId, Rgba};

/// Default number of undo steps kept.
pub const DEFAULT_MAX_ENTRIES: usize = 256;
/// Default memory budget for the undo history.
pub const DEFAULT_MAX_BYTES: usize = 256 * 1024 * 1024;

/// One pixel that changed from `before` to `after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelChange {
    pub index: u32,
    pub before: Rgba,
    pub after: Rgba,
}

/// Part of the image that needs to be redrawn after a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dirty {
    /// Inclusive pixel rectangle `(min_x, min_y, max_x, max_y)`.
    Rect(u32, u32, u32, u32),
    /// The whole image.
    Full,
}

impl Dirty {
    pub fn union(self, other: Dirty) -> Dirty {
        match (self, other) {
            (Dirty::Rect(a0, a1, a2, a3), Dirty::Rect(b0, b1, b2, b3)) => {
                Dirty::Rect(a0.min(b0), a1.min(b1), a2.max(b2), a3.max(b3))
            }
            _ => Dirty::Full,
        }
    }

    pub fn pixel(x: u32, y: u32) -> Dirty {
        Dirty::Rect(x, y, x, y)
    }
}

/// A reversible document change.
///
/// Layer insertions and removals move the layer between the document and the change,
/// so a layer's pixels exist in exactly one place at a time.
#[derive(Debug, Clone)]
pub enum Change {
    Pixels {
        layer: LayerId,
        changes: Vec<PixelChange>,
    },
    AddLayer {
        index: usize,
        /// `Some` while the layer is *not* in the document (i.e. after an undo).
        layer: Option<Layer>,
        prev_active: usize,
    },
    RemoveLayer {
        index: usize,
        /// `Some` while the layer is *not* in the document (i.e. after it was removed).
        layer: Option<Layer>,
        prev_active: usize,
    },
    MoveLayer {
        from: usize,
        to: usize,
    },
    RenameLayer {
        layer: LayerId,
        before: String,
        after: String,
    },
    SetVisible {
        layer: LayerId,
        visible: bool,
    },
}

impl Change {
    /// Short user-facing description, used in menus and the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            Change::Pixels { .. } => "Draw",
            Change::AddLayer { .. } => "New Layer",
            Change::RemoveLayer { .. } => "Delete Layer",
            Change::MoveLayer { .. } => "Move Layer",
            Change::RenameLayer { .. } => "Rename Layer",
            Change::SetVisible { .. } => "Layer Visibility",
        }
    }

    /// Approximate memory used by this change.
    pub fn byte_size(&self) -> usize {
        let base = std::mem::size_of::<Change>();
        base + match self {
            Change::Pixels { changes, .. } => changes.len() * std::mem::size_of::<PixelChange>(),
            Change::AddLayer { layer, .. } | Change::RemoveLayer { layer, .. } => layer
                .as_ref()
                .map(|l| l.pixels.as_bytes().len() + l.name.len())
                .unwrap_or(0),
            Change::RenameLayer { before, after, .. } => before.len() + after.len(),
            Change::MoveLayer { .. } | Change::SetVisible { .. } => 0,
        }
    }

    /// Region affected by this change.
    pub fn dirty(&self, doc: &Document) -> Option<Dirty> {
        match self {
            Change::Pixels { changes, .. } => {
                let w = doc.width();
                changes.iter().fold(None, |acc: Option<Dirty>, c| {
                    let d = Dirty::pixel(c.index % w, c.index / w);
                    Some(acc.map_or(d, |a| a.union(d)))
                })
            }
            Change::RenameLayer { .. } => None,
            _ => Some(Dirty::Full),
        }
    }

    /// Applies (or re-applies) the change to the document.
    pub fn apply(&mut self, doc: &mut Document) -> Result<(), CoreError> {
        match self {
            Change::Pixels { layer, changes } => {
                let layer = doc
                    .layer_mut(*layer)
                    .ok_or(CoreError::LayerIndex(usize::MAX))?;
                for c in changes.iter() {
                    layer.pixels.set_index(c.index, c.after);
                }
                Ok(())
            }
            Change::AddLayer { index, layer, .. } => restore_layer(doc, *index, layer),
            Change::RemoveLayer { index, layer, .. } => {
                *layer = Some(doc.remove_layer(*index)?);
                Ok(())
            }
            Change::MoveLayer { from, to } => doc.move_layer(*from, *to),
            Change::RenameLayer { layer, after, .. } => {
                let l = doc
                    .layer_mut(*layer)
                    .ok_or(CoreError::LayerIndex(usize::MAX))?;
                l.name = after.clone();
                Ok(())
            }
            Change::SetVisible { layer, visible } => {
                let l = doc
                    .layer_mut(*layer)
                    .ok_or(CoreError::LayerIndex(usize::MAX))?;
                l.visible = *visible;
                Ok(())
            }
        }
    }

    /// Reverses the change.
    pub fn revert(&mut self, doc: &mut Document) -> Result<(), CoreError> {
        match self {
            Change::Pixels { layer, changes } => {
                let layer = doc
                    .layer_mut(*layer)
                    .ok_or(CoreError::LayerIndex(usize::MAX))?;
                for c in changes.iter().rev() {
                    layer.pixels.set_index(c.index, c.before);
                }
                Ok(())
            }
            Change::AddLayer {
                index,
                layer,
                prev_active,
            } => {
                *layer = Some(doc.remove_layer(*index)?);
                doc.set_active_index(*prev_active);
                Ok(())
            }
            Change::RemoveLayer {
                index,
                layer,
                prev_active,
            } => {
                restore_layer(doc, *index, layer)?;
                doc.set_active_index(*prev_active);
                Ok(())
            }
            Change::MoveLayer { from, to } => doc.move_layer(*to, *from),
            Change::RenameLayer { layer, before, .. } => {
                let l = doc
                    .layer_mut(*layer)
                    .ok_or(CoreError::LayerIndex(usize::MAX))?;
                l.name = before.clone();
                Ok(())
            }
            Change::SetVisible { layer, visible } => {
                let l = doc
                    .layer_mut(*layer)
                    .ok_or(CoreError::LayerIndex(usize::MAX))?;
                l.visible = !*visible;
                Ok(())
            }
        }
    }
}

/// Moves a stored layer back into the document. Validates first, so on error the
/// layer stays in the change untouched.
fn restore_layer(
    doc: &mut Document,
    index: usize,
    slot: &mut Option<Layer>,
) -> Result<(), CoreError> {
    let stored = slot.as_ref().ok_or(CoreError::LayerIndex(index))?;
    doc.check_insert(index, stored)?;
    match slot.take() {
        Some(layer) => doc.insert_layer(index, layer),
        None => Err(CoreError::LayerIndex(index)),
    }
}

/// Result of a successful undo or redo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    pub label: &'static str,
    /// Region to redraw, `None` if no pixels changed (e.g. a rename).
    pub dirty: Option<Dirty>,
}

#[derive(Debug)]
struct Entry {
    id: u64,
    bytes: usize,
    change: Change,
}

/// Bounded undo/redo stacks.
#[derive(Debug)]
pub struct History {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    next_id: u64,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES, DEFAULT_MAX_BYTES)
    }
}

impl History {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            next_id: 1,
            bytes: 0,
            max_entries: max_entries.max(1),
            max_bytes,
        }
    }

    /// Records a change that has already been applied. Clears the redo stack.
    pub fn push(&mut self, change: Change) {
        for e in self.redo.drain(..) {
            self.bytes = self.bytes.saturating_sub(e.bytes);
        }
        let bytes = change.byte_size();
        let id = self.next_id;
        self.next_id += 1;
        self.bytes += bytes;
        self.undo.push_back(Entry { id, bytes, change });
        while self.undo.len() > self.max_entries
            || (self.bytes > self.max_bytes && self.undo.len() > 1)
        {
            if let Some(old) = self.undo.pop_front() {
                self.bytes = self.bytes.saturating_sub(old.bytes);
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.back().map(|e| e.change.label())
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|e| e.change.label())
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Identifier of the current state; equal ids mean equal documents.
    pub fn state_id(&self) -> u64 {
        self.undo.back().map_or(0, |e| e.id)
    }

    /// Undoes the last change. Returns `None` if there was nothing to undo.
    pub fn undo(&mut self, doc: &mut Document) -> Result<Option<Applied>, CoreError> {
        let Some(mut entry) = self.undo.pop_back() else {
            return Ok(None);
        };
        let dirty = entry.change.dirty(doc);
        if let Err(e) = entry.change.revert(doc) {
            self.undo.push_back(entry);
            return Err(e);
        }
        let applied = Applied {
            label: entry.change.label(),
            dirty,
        };
        self.redo.push(entry);
        Ok(Some(applied))
    }

    /// Redoes the last undone change. Returns `None` if there was nothing to redo.
    pub fn redo(&mut self, doc: &mut Document) -> Result<Option<Applied>, CoreError> {
        let Some(mut entry) = self.redo.pop() else {
            return Ok(None);
        };
        if let Err(e) = entry.change.apply(doc) {
            self.redo.push(entry);
            return Err(e);
        }
        let applied = Applied {
            label: entry.change.label(),
            dirty: entry.change.dirty(doc),
        };
        self.undo.push_back(entry);
        Ok(Some(applied))
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.bytes = 0;
    }
}

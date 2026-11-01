//! Selections (rectangles and pixel masks) and pixel-exact block transforms (flip,
//! rotate by 90°).
//!
//! Everything works on whole pixels; there is no interpolation anywhere.

use std::sync::Arc;

use super::{PixelBuffer, Rgba};

/// A selection rectangle in image pixels. Always at least 1×1 and inside the image
/// once [`SelRect::clip`] has been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl SelRect {
    /// The rectangle spanning two pixel corners (both inclusive), in any order.
    pub fn from_corners(a: (i32, i32), b: (i32, i32)) -> Self {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        Self {
            x: x0,
            y: y0,
            w: (i64::from(x1) - i64::from(x0) + 1) as u32,
            h: (i64::from(y1) - i64::from(y0) + 1) as u32,
        }
    }

    pub fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.w)
    }

    pub fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.h)
    }

    /// The part inside a `width`×`height` image, or `None` if nothing is left.
    pub fn clip(&self, width: u32, height: u32) -> Option<SelRect> {
        let x0 = i64::from(self.x).max(0);
        let y0 = i64::from(self.y).max(0);
        let x1 = self.right().min(i64::from(width));
        let y1 = self.bottom().min(i64::from(height));
        (x1 > x0 && y1 > y0).then(|| SelRect {
            x: x0 as i32,
            y: y0 as i32,
            w: (x1 - x0) as u32,
            h: (y1 - y0) as u32,
        })
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        i64::from(x) >= i64::from(self.x)
            && i64::from(y) >= i64::from(self.y)
            && i64::from(x) < self.right()
            && i64::from(y) < self.bottom()
    }

    pub fn translated(&self, dx: i32, dy: i32) -> SelRect {
        SelRect {
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
            ..*self
        }
    }
}

/// A block of pixels lifted out of an image (also the clipboard content).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub w: u32,
    pub h: u32,
    /// RGBA rows.
    pub data: Vec<u8>,
}

impl Block {
    pub fn get(&self, x: u32, y: u32) -> Rgba {
        let i = ((y * self.w + x) * 4) as usize;
        Rgba([
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ])
    }

    fn from_fn(w: u32, h: u32, f: impl Fn(u32, u32) -> Rgba) -> Block {
        let mut data = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                data.extend_from_slice(&f(x, y).0);
            }
        }
        Block { w, h, data }
    }

    /// Copies the pixels of `rect` (already clipped to `buf`).
    pub fn copy_from(buf: &PixelBuffer, rect: SelRect) -> Block {
        Block::from_fn(rect.w, rect.h, |x, y| {
            buf.get(rect.x + x as i32, rect.y + y as i32)
                .unwrap_or_default()
        })
    }

    pub fn is_empty(&self) -> bool {
        self.data.chunks_exact(4).all(|p| p[3] == 0)
    }

    pub fn flip_horizontal(&self) -> Block {
        Block::from_fn(self.w, self.h, |x, y| self.get(self.w - 1 - x, y))
    }

    pub fn flip_vertical(&self) -> Block {
        Block::from_fn(self.w, self.h, |x, y| self.get(x, self.h - 1 - y))
    }

    /// Rotates clockwise by 90°: the result is `h × w`.
    pub fn rotate_cw(&self) -> Block {
        Block::from_fn(self.h, self.w, |x, y| self.get(y, self.h - 1 - x))
    }

    pub fn rotate_180(&self) -> Block {
        Block::from_fn(self.w, self.h, |x, y| {
            self.get(self.w - 1 - x, self.h - 1 - y)
        })
    }

    /// Rotates counter-clockwise by 90° (= 270° clockwise): the result is `h × w`.
    pub fn rotate_ccw(&self) -> Block {
        Block::from_fn(self.h, self.w, |x, y| self.get(self.w - 1 - y, x))
    }
}

/// Clears `rect` of `buf` to transparent.
pub fn clear(buf: &mut PixelBuffer, rect: SelRect) {
    for y in 0..rect.h as i32 {
        for x in 0..rect.w as i32 {
            buf.set(rect.x + x, rect.y + y, Rgba::TRANSPARENT);
        }
    }
}

/// Stamps a block with its top-left at `(x, y)`. Transparent block pixels leave what's
/// underneath; pixels falling outside the image are dropped.
pub fn stamp(buf: &mut PixelBuffer, block: &Block, x: i32, y: i32) {
    for by in 0..block.h {
        for bx in 0..block.w {
            let c = block.get(bx, by);
            if c.a() != 0 {
                buf.set(x.saturating_add(bx as i32), y.saturating_add(by as i32), c);
            }
        }
    }
}

/// A line between two pixel corners of a selection outline.
pub type Segment = [(i32, i32); 2];

/// How a new selection shape combines with the current selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectMode {
    #[default]
    Replace,
    /// Shift: add to the selection.
    Add,
    /// Alt: cut out of the selection.
    Subtract,
    /// Shift+Alt: keep only the overlap.
    Intersect,
}

impl SelectMode {
    pub const ALL: [SelectMode; 4] = [
        SelectMode::Replace,
        SelectMode::Add,
        SelectMode::Subtract,
        SelectMode::Intersect,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SelectMode::Replace => "Replace",
            SelectMode::Add => "Add",
            SelectMode::Subtract => "Subtract",
            SelectMode::Intersect => "Intersect",
        }
    }

    /// The mode held modifier keys ask for, overriding `chosen`.
    pub fn with_modifiers(chosen: SelectMode, shift: bool, alt: bool) -> SelectMode {
        match (shift, alt) {
            (true, true) => SelectMode::Intersect,
            (true, false) => SelectMode::Add,
            (false, true) => SelectMode::Subtract,
            (false, false) => chosen,
        }
    }
}

/// Selected pixels: a bounding rectangle and, unless every pixel in it is selected,
/// a mask. Masks are shared (cheap to clone into undo history) and never changed in
/// place. A selection always selects at least one pixel and its rectangle is tight
/// around the selected pixels when it has a mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    rect: SelRect,
    /// `rect.w * rect.h` flags, row by row; `None` = the whole rectangle.
    mask: Option<Arc<[bool]>>,
}

impl From<SelRect> for Selection {
    fn from(rect: SelRect) -> Self {
        Self { rect, mask: None }
    }
}

impl Selection {
    /// A selection from a mask covering `rect` (`rect.w * rect.h` flags), shrunk to the
    /// selected pixels. `None` when nothing is selected.
    pub fn from_mask(rect: SelRect, bits: &[bool]) -> Option<Selection> {
        let (w, h) = (rect.w as usize, rect.h as usize);
        if bits.len() != w * h {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        let mut count = 0usize;
        for y in 0..h {
            for x in 0..w {
                if bits[y * w + x] {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                    count += 1;
                }
            }
        }
        if count == 0 {
            return None;
        }
        let (tw, th) = (x1 - x0 + 1, y1 - y0 + 1);
        let tight = SelRect {
            x: rect.x + x0 as i32,
            y: rect.y + y0 as i32,
            w: tw as u32,
            h: th as u32,
        };
        if count == tw * th {
            return Some(Selection::from(tight));
        }
        let mut mask = Vec::with_capacity(tw * th);
        for y in y0..=y1 {
            mask.extend_from_slice(&bits[y * w + x0..=y * w + x1]);
        }
        Some(Selection {
            rect: tight,
            mask: Some(mask.into()),
        })
    }

    /// A selection from a mask of a whole `width`×`height` image.
    pub fn from_canvas(width: u32, height: u32, bits: &[bool]) -> Option<Selection> {
        Self::from_mask(
            SelRect {
                x: 0,
                y: 0,
                w: width,
                h: height,
            },
            bits,
        )
    }

    /// The rectangle around the selected pixels.
    pub fn bounds(&self) -> SelRect {
        self.rect
    }

    /// Whether every pixel of [`Selection::bounds`] is selected.
    pub fn is_rectangular(&self) -> bool {
        self.mask.is_none()
    }

    /// Identity of the mask, for caches (0 for rectangles).
    pub fn mask_id(&self) -> usize {
        self.mask
            .as_ref()
            .map_or(0, |m| Arc::as_ptr(m).cast::<bool>() as usize)
    }

    /// Whether image pixel `(x, y)` is selected.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if !self.rect.contains(x, y) {
            return false;
        }
        match &self.mask {
            None => true,
            Some(m) => {
                let (bx, by) = ((x - self.rect.x) as usize, (y - self.rect.y) as usize);
                m[by * self.rect.w as usize + bx]
            }
        }
    }

    /// Number of selected pixels.
    pub fn pixel_count(&self) -> u64 {
        match &self.mask {
            None => u64::from(self.rect.w) * u64::from(self.rect.h),
            Some(m) => m.iter().filter(|b| **b).count() as u64,
        }
    }

    /// The same shape moved by `(dx, dy)`.
    pub fn translated(&self, dx: i32, dy: i32) -> Selection {
        Selection {
            rect: self.rect.translated(dx, dy),
            mask: self.mask.clone(),
        }
    }

    /// The part inside a `width`×`height` image, or `None` if nothing is left.
    pub fn clip(&self, width: u32, height: u32) -> Option<Selection> {
        let clipped = self.rect.clip(width, height)?;
        if clipped == self.rect {
            return Some(self.clone());
        }
        if self.mask.is_none() {
            return Some(Selection::from(clipped));
        }
        let mut bits = Vec::with_capacity(clipped.w as usize * clipped.h as usize);
        for y in clipped.y..clipped.y + clipped.h as i32 {
            for x in clipped.x..clipped.x + clipped.w as i32 {
                bits.push(self.contains(x, y));
            }
        }
        Selection::from_mask(clipped, &bits)
    }

    /// One flag per pixel of a `width`×`height` image.
    pub fn to_canvas(&self, width: u32, height: u32) -> Vec<bool> {
        let mut bits = vec![false; width as usize * height as usize];
        if let Some(r) = self.rect.clip(width, height) {
            for y in r.y..r.y + r.h as i32 {
                for x in r.x..r.x + r.w as i32 {
                    bits[y as usize * width as usize + x as usize] = self.contains(x, y);
                }
            }
        }
        bits
    }

    /// `shape` combined with `current` in a `width`×`height` image.
    pub fn combine(
        current: Option<&Selection>,
        shape: Option<&Selection>,
        mode: SelectMode,
        width: u32,
        height: u32,
    ) -> Option<Selection> {
        let shape = shape.and_then(|s| s.clip(width, height));
        let current = match (mode, current) {
            (SelectMode::Replace, _) => return shape,
            (SelectMode::Add, None) => return shape,
            (SelectMode::Subtract | SelectMode::Intersect, None) => return None,
            (_, Some(c)) => c,
        };
        let Some(shape) = shape else {
            return match mode {
                SelectMode::Intersect => None,
                _ => current.clip(width, height),
            };
        };
        let mut bits = current.to_canvas(width, height);
        let add = shape.to_canvas(width, height);
        for (a, b) in bits.iter_mut().zip(add) {
            *a = match mode {
                SelectMode::Add | SelectMode::Replace => *a || b,
                SelectMode::Subtract => *a && !b,
                SelectMode::Intersect => *a && b,
            };
        }
        Selection::from_canvas(width, height, &bits)
    }

    /// Everything not selected (everything when nothing is).
    pub fn inverted(current: Option<&Selection>, width: u32, height: u32) -> Option<Selection> {
        let mut bits = match current {
            Some(s) => s.to_canvas(width, height),
            None => vec![false; width as usize * height as usize],
        };
        for b in &mut bits {
            *b = !*b;
        }
        Selection::from_canvas(width, height, &bits)
    }

    /// The mask as a block (selected = opaque white), for reusing block transforms.
    fn mask_block(&self) -> Block {
        Block::from_fn(self.rect.w, self.rect.h, |x, y| {
            if self.contains(self.rect.x + x as i32, self.rect.y + y as i32) {
                Rgba::WHITE
            } else {
                Rgba::TRANSPARENT
            }
        })
    }

    /// This shape transformed with the same block transform as its pixels, placed at
    /// `rect` (whose size must be the transformed size).
    pub fn reshaped(
        &self,
        rect: SelRect,
        transform: impl Fn(&Block) -> Block,
    ) -> Option<Selection> {
        if self.mask.is_none() {
            return Some(Selection::from(rect));
        }
        let out = transform(&self.mask_block());
        if (out.w, out.h) != (rect.w, rect.h) {
            return None;
        }
        let bits: Vec<bool> = out.data.chunks_exact(4).map(|p| p[3] != 0).collect();
        Selection::from_mask(rect, &bits)
    }

    /// The outline as segments between pixel corners (image coordinates), with
    /// neighboring unit edges merged into longer lines.
    pub fn outline(&self) -> Vec<Segment> {
        let r = self.rect;
        let (x0, y0) = (r.x, r.y);
        let (x1, y1) = (r.x + r.w as i32, r.y + r.h as i32);
        if self.mask.is_none() {
            return vec![
                [(x0, y0), (x1, y0)],
                [(x1, y0), (x1, y1)],
                [(x1, y1), (x0, y1)],
                [(x0, y1), (x0, y0)],
            ];
        }
        let mut out = Vec::new();
        // Horizontal edges: between rows y-1 and y.
        for y in y0..=y1 {
            let mut start: Option<i32> = None;
            for x in x0..=x1 {
                let edge = x < x1 && self.contains(x, y - 1) != self.contains(x, y);
                match (edge, start) {
                    (true, None) => start = Some(x),
                    (false, Some(s)) => {
                        out.push([(s, y), (x, y)]);
                        start = None;
                    }
                    _ => {}
                }
            }
        }
        // Vertical edges: between columns x-1 and x.
        for x in x0..=x1 {
            let mut start: Option<i32> = None;
            for y in y0..=y1 {
                let edge = y < y1 && self.contains(x - 1, y) != self.contains(x, y);
                match (edge, start) {
                    (true, None) => start = Some(y),
                    (false, Some(s)) => {
                        out.push([(x, s), (x, y)]);
                        start = None;
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

impl Block {
    /// Copies the selected pixels of `sel` (unselected ones become transparent). The
    /// block covers the selection's bounds.
    pub fn copy_selected(buf: &PixelBuffer, sel: &Selection) -> Block {
        let r = sel.bounds();
        Block::from_fn(r.w, r.h, |x, y| {
            let (px, py) = (r.x + x as i32, r.y + y as i32);
            if sel.contains(px, py) {
                buf.get(px, py).unwrap_or_default()
            } else {
                Rgba::TRANSPARENT
            }
        })
    }
}

/// Clears the selected pixels of `buf` to transparent.
pub fn clear_selected(buf: &mut PixelBuffer, sel: &Selection) {
    let r = sel.bounds();
    for y in r.y..r.y + r.h as i32 {
        for x in r.x..r.x + r.w as i32 {
            if sel.contains(x, y) {
                buf.set(x, y, Rgba::TRANSPARENT);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> Block {
        // 3×2:  a b c
        //       d e f
        Block::from_fn(3, 2, |x, y| Rgba::opaque((y * 3 + x) as u8, 0, 0))
    }

    fn values(b: &Block) -> Vec<u8> {
        b.data.chunks(4).map(|p| p[0]).collect()
    }

    #[test]
    fn flips_and_rotations() {
        let b = block();
        assert_eq!(values(&b.flip_horizontal()), vec![2, 1, 0, 5, 4, 3]);
        assert_eq!(values(&b.flip_vertical()), vec![3, 4, 5, 0, 1, 2]);
        let cw = b.rotate_cw();
        assert_eq!((cw.w, cw.h), (2, 3));
        assert_eq!(values(&cw), vec![3, 0, 4, 1, 5, 2]);
        assert_eq!(values(&b.rotate_180()), vec![5, 4, 3, 2, 1, 0]);
        let ccw = b.rotate_ccw();
        assert_eq!(values(&ccw), vec![2, 5, 1, 4, 0, 3]);
        assert_eq!(b.rotate_cw().rotate_cw().rotate_cw().rotate_cw(), b);
        assert_eq!(b.rotate_cw().rotate_ccw(), b);
    }

    #[test]
    fn rects_clip_and_contain() {
        let r = SelRect::from_corners((5, 4), (-2, 1));
        assert_eq!(
            r,
            SelRect {
                x: -2,
                y: 1,
                w: 8,
                h: 4
            }
        );
        assert_eq!(
            r.clip(4, 4),
            Some(SelRect {
                x: 0,
                y: 1,
                w: 4,
                h: 3
            })
        );
        assert_eq!(
            SelRect {
                x: 10,
                y: 0,
                w: 2,
                h: 2
            }
            .clip(4, 4),
            None
        );
        assert!(r.contains(-2, 1) && r.contains(5, 4) && !r.contains(6, 4));
        assert_eq!(
            SelRect::from_corners((1, 1), (1, 1)),
            SelRect {
                x: 1,
                y: 1,
                w: 1,
                h: 1
            }
        );
    }
}

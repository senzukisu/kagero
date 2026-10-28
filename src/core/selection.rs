//! Rectangular selections and pixel-exact block transforms (flip, rotate by 90°).
//!
//! Everything works on whole pixels; there is no interpolation anywhere.

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

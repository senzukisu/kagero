//! Picking pixels by color (magic wand, and later the fill tool) and by shape (lasso).
//!
//! Results are masks: one `bool` per pixel, row by row.

use super::raster;
use super::selection::SelRect;
use super::{PixelBuffer, Rgba};

/// Whether `a` and `b` are within `tolerance` of each other: every RGBA channel
/// differs by at most `tolerance`. Fully transparent pixels all count as the same
/// color, whatever RGB they carry.
pub fn similar(a: Rgba, b: Rgba, tolerance: u8) -> bool {
    if a.a() == 0 && b.a() == 0 {
        return true;
    }
    a.0.iter()
        .zip(b.0)
        .all(|(&x, y)| x.abs_diff(y) <= tolerance)
}

/// Pixels similar to the one at `(x, y)`: only those connected to it (4-connected)
/// when `contiguous`, otherwise every similar pixel of the image. An empty mask if
/// the point is outside the image.
pub fn similar_region(
    buf: &PixelBuffer,
    x: i32,
    y: i32,
    tolerance: u8,
    contiguous: bool,
) -> Vec<bool> {
    let (w, h) = (buf.width() as i32, buf.height() as i32);
    let mut mask = vec![false; (w * h) as usize];
    let Some(target) = buf.get(x, y) else {
        return mask;
    };
    let matches = |i: usize| {
        let p = &buf.as_bytes()[i * 4..i * 4 + 4];
        similar(Rgba([p[0], p[1], p[2], p[3]]), target, tolerance)
    };
    if !contiguous {
        for (i, m) in mask.iter_mut().enumerate() {
            *m = matches(i);
        }
        return mask;
    }
    // Span fill: each popped seed fills its whole horizontal run, then seeds the rows
    // above and below once per run.
    let mut stack = vec![(x, y)];
    while let Some((sx, sy)) = stack.pop() {
        let row = (sy * w) as usize;
        if mask[row + sx as usize] {
            continue;
        }
        let mut x0 = sx;
        while x0 > 0 && !mask[row + x0 as usize - 1] && matches(row + x0 as usize - 1) {
            x0 -= 1;
        }
        let mut x1 = sx;
        while x1 + 1 < w && !mask[row + x1 as usize + 1] && matches(row + x1 as usize + 1) {
            x1 += 1;
        }
        for px in x0..=x1 {
            mask[row + px as usize] = true;
        }
        for ny in [sy - 1, sy + 1] {
            if ny < 0 || ny >= h {
                continue;
            }
            let nrow = (ny * w) as usize;
            let mut inside = false;
            for px in x0..=x1 {
                let i = nrow + px as usize;
                let ok = !mask[i] && matches(i);
                if ok && !inside {
                    stack.push((px, ny));
                }
                inside = ok;
            }
        }
    }
    mask
}

/// The pixels inside a closed freehand outline through pixel `points` (even-odd
/// rule on pixel centers), plus the pixels the outline itself passes through, as a
/// mask of the box around them clipped to a `width`×`height` image.
/// `None` if nothing of it is inside the image.
pub fn polygon_mask(
    points: &[(i32, i32)],
    width: u32,
    height: u32,
) -> Option<(SelRect, Vec<bool>)> {
    let first = *points.first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.0, first.1, first.0, first.1);
    for &(x, y) in points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let x0 = x0.max(0);
    let y0 = y0.max(0);
    let x1 = x1.min(width as i32 - 1);
    let y1 = y1.min(height as i32 - 1);
    if x1 < x0 || y1 < y0 {
        return None;
    }
    let (bw, bh) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let mut mask = vec![false; bw * bh];
    // Interior: for each row, where the outline crosses the row's center line.
    let n = points.len();
    let mut crossings = Vec::new();
    for row in 0..bh {
        let y = (y0 + row as i32) as f64;
        crossings.clear();
        for i in 0..n {
            let (ax, ay) = points[i];
            let (bx, by) = points[(i + 1) % n];
            let (ay, by) = (f64::from(ay), f64::from(by));
            // Half-open so a vertex exactly on the line counts once.
            if (ay > y) != (by > y) {
                let t = (y - ay) / (by - ay);
                crossings.push(f64::from(ax) + t * f64::from(bx - ax));
            }
        }
        crossings.sort_by(f64::total_cmp);
        for pair in crossings.chunks_exact(2) {
            let from = pair[0].ceil().max(f64::from(x0)) as i32;
            let to = pair[1].floor().min(f64::from(x1)) as i32;
            for x in from..=to {
                mask[row * bw + (x - x0) as usize] = true;
            }
        }
    }
    // The outline itself, closing back to the start.
    let mut plot = |x: i32, y: i32| {
        if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
            mask[(y - y0) as usize * bw + (x - x0) as usize] = true;
        }
    };
    for i in 0..n {
        raster::line(points[i], points[(i + 1) % n], &mut plot);
    }
    Some((
        SelRect {
            x: x0,
            y: y0,
            w: bw as u32,
            h: bh as u32,
        },
        mask,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> PixelBuffer {
        // 5×3:  R R B R R
        //       R R B r R     (r = slightly different red)
        //       B B B . .     (. = transparent, different RGB each)
        let red = Rgba::opaque(200, 0, 0);
        let mut b = PixelBuffer::filled(5, 3, red).expect("valid");
        for (x, y) in [(2, 0), (2, 1), (0, 2), (1, 2), (2, 2)] {
            b.set(x, y, Rgba::opaque(0, 0, 200));
        }
        b.set(3, 1, Rgba::opaque(210, 5, 0));
        b.set(3, 2, Rgba::new(1, 2, 3, 0));
        b.set(4, 2, Rgba::new(9, 9, 9, 0));
        b
    }

    fn count(m: &[bool]) -> usize {
        m.iter().filter(|b| **b).count()
    }

    #[test]
    fn tolerance_and_transparency() {
        assert!(similar(Rgba::new(1, 2, 3, 0), Rgba::new(200, 0, 0, 0), 0));
        assert!(!similar(
            Rgba::opaque(200, 0, 0),
            Rgba::opaque(210, 5, 0),
            9
        ));
        assert!(similar(
            Rgba::opaque(200, 0, 0),
            Rgba::opaque(210, 5, 0),
            10
        ));
    }

    #[test]
    fn contiguous_stops_at_other_colors() {
        let b = picture();
        let left = similar_region(&b, 0, 0, 0, true);
        assert_eq!(count(&left), 4);
        assert!(left[0] && left[1] && left[5] && left[6]);
        // Exact color, global: both red areas except the slightly different pixel.
        assert_eq!(count(&similar_region(&b, 0, 0, 0, false)), 7);
        // With tolerance the different red joins.
        assert_eq!(count(&similar_region(&b, 0, 0, 10, false)), 8);
        // Transparent pixels with different RGB are one region.
        let clear = similar_region(&b, 3, 2, 0, true);
        assert_eq!(count(&clear), 2);
        assert_eq!(count(&similar_region(&b, 9, 9, 0, true)), 0);
    }

    #[test]
    fn contiguous_fills_around_corners() {
        // A U shape: the flood must go down, across and back up.
        let mut b = PixelBuffer::filled(5, 4, Rgba::opaque(0, 0, 0)).expect("valid");
        for y in 0..3 {
            b.set(2, y, Rgba::opaque(255, 255, 255));
        }
        let m = similar_region(&b, 0, 0, 0, true);
        assert_eq!(count(&m), 17);
        assert!(m[4], "the far top-right corner is reached");
    }

    #[test]
    fn polygon_fills_inside_and_keeps_outline() {
        let square = [(1, 1), (6, 1), (6, 6), (1, 6)];
        let (r, m) = polygon_mask(&square, 10, 10).expect("inside");
        assert_eq!((r.x, r.y, r.w, r.h), (1, 1, 6, 6));
        assert_eq!(count(&m), 36, "outline and interior");
        // A thin stroke that encloses nothing still selects what it touched.
        let (r, m) = polygon_mask(&[(0, 0), (4, 0)], 10, 10).expect("line");
        assert_eq!((r.w, r.h, count(&m)), (5, 1, 5));
        // Clipped to the image; entirely outside gives nothing.
        let (r, _) = polygon_mask(&[(-5, -5), (3, -5), (3, 3), (-5, 3)], 10, 10).expect("clip");
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 4, 4));
        assert!(polygon_mask(&[(20, 20), (30, 30)], 10, 10).is_none());
        assert!(polygon_mask(&[], 10, 10).is_none());
    }
}

//! Raster algorithms on the integer pixel grid.

use super::PixelBuffer;

/// Calls `plot` for every pixel of a Bresenham line from `a` to `b`, both ends included.
pub fn line(a: (i32, i32), b: (i32, i32), mut plot: impl FnMut(i32, i32)) {
    let (mut x0, mut y0) = (i64::from(a.0), i64::from(a.1));
    let (x1, y1) = (i64::from(b.0), i64::from(b.1));
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot(x0 as i32, y0 as i32);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Normalizes two corners into `(min_x, min_y, max_x, max_y)`.
pub fn normalize_rect(a: (i32, i32), b: (i32, i32)) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

/// Calls `plot` for each pixel of the rectangle outline with corners `a` and `b` (inclusive).
/// Each pixel is visited exactly once.
pub fn rect_outline(a: (i32, i32), b: (i32, i32), mut plot: impl FnMut(i32, i32)) {
    let (x0, y0, x1, y1) = normalize_rect(a, b);
    for x in x0..=x1 {
        plot(x, y0);
        if y1 != y0 {
            plot(x, y1);
        }
    }
    for y in (y0 + 1)..y1 {
        plot(x0, y);
        if x1 != x0 {
            plot(x1, y);
        }
    }
}

/// Calls `plot` for each pixel inside the rectangle with corners `a` and `b` (inclusive),
/// clipped to `width`×`height` so huge drags never iterate outside the image.
pub fn rect_filled(
    a: (i32, i32),
    b: (i32, i32),
    width: u32,
    height: u32,
    mut plot: impl FnMut(i32, i32),
) {
    let (x0, y0, x1, y1) = normalize_rect(a, b);
    let x0 = x0.max(0);
    let y0 = y0.max(0);
    let x1 = x1.min(width as i32 - 1);
    let y1 = y1.min(height as i32 - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            plot(x, y);
        }
    }
}

/// Calls `plot` for the outline of the ellipse inscribed in the rectangle with
/// corners `a` and `b` (inclusive), or for every pixel inside it when `filled`.
/// Filled ellipses are clipped to `width`×`height`. Pixels may be visited twice
/// for outlines of very small ellipses.
pub fn ellipse(
    a: (i32, i32),
    b: (i32, i32),
    filled: bool,
    width: u32,
    height: u32,
    mut plot: impl FnMut(i32, i32),
) {
    let (x0, y0, x1, y1) = normalize_rect(a, b);
    let (x0, y0, x1, y1) = (i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1));
    // Rows of the outline: y → leftmost and rightmost pixel.
    let mut rows: std::collections::BTreeMap<i64, (i64, i64)> = std::collections::BTreeMap::new();
    let mut outline = Vec::new();
    let mut put = |x: i64, y: i64| {
        let e = rows.entry(y).or_insert((x, x));
        e.0 = e.0.min(x);
        e.1 = e.1.max(x);
        outline.push((x, y));
    };
    // Bresenham-style ellipse in a rectangle (after A. Zingl, "A Rasterizing
    // Algorithm for Drawing Curves"); works for even and odd sizes.
    let (mut xa, mut xb) = (x0, x1);
    let a_d = x1 - x0;
    let b_d = y1 - y0;
    let b1 = b_d & 1;
    let mut dx = 4 * (1 - a_d) * b_d * b_d;
    let mut dy = 4 * (b1 + 1) * a_d * a_d;
    let mut err = dx + dy + b1 * a_d * a_d;
    let mut ya = y0 + (b_d + 1) / 2;
    let mut yb = ya - b1;
    let a8 = 8 * a_d * a_d;
    let b8 = 8 * b_d * b_d;
    loop {
        put(xb, ya);
        put(xa, ya);
        put(xa, yb);
        put(xb, yb);
        let e2 = 2 * err;
        if e2 <= dy {
            ya += 1;
            yb -= 1;
            dy += a8;
            err += dy;
        }
        if e2 >= dx || 2 * err > dy {
            xa += 1;
            xb -= 1;
            dx += b8;
            err += dx;
        }
        if xa > xb {
            break;
        }
    }
    // Very flat ellipses stop early: finish their tips.
    while ya - yb <= b_d {
        put(xa - 1, ya);
        put(xb + 1, ya);
        ya += 1;
        put(xa - 1, yb);
        put(xb + 1, yb);
        yb -= 1;
    }
    if filled {
        let (w, h) = (i64::from(width), i64::from(height));
        for (&y, &(l, r)) in &rows {
            if y < 0 || y >= h {
                continue;
            }
            for x in l.max(0)..=r.min(w - 1) {
                plot(x as i32, y as i32);
            }
        }
    } else {
        outline.sort_unstable();
        outline.dedup();
        for (x, y) in outline {
            if let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) {
                plot(x, y);
            }
        }
    }
}

/// Brush tip shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushShape {
    #[default]
    Square,
    Circle,
    /// A shape taken from a selection (see the editor's custom brush).
    Custom,
}

impl BrushShape {
    pub fn name(self) -> &'static str {
        match self {
            BrushShape::Square => "Square",
            BrushShape::Circle => "Circle",
            BrushShape::Custom => "Custom",
        }
    }
}

/// Offsets of a round brush of `size` pixels (1–3 px are a dot, a 2×2 block and a
/// plus; bigger sizes are discs).
pub fn circle_offsets(size: u32) -> Vec<(i32, i32)> {
    let s = size.clamp(1, 64) as i32;
    let start = -(s - 1) / 2;
    let center = f64::from(start) + f64::from(s - 1) / 2.0;
    let r = f64::from(s) / 2.0 - 0.25;
    let mut out = Vec::new();
    for dy in start..start + s {
        for dx in start..start + s {
            let (fx, fy) = (f64::from(dx) - center, f64::from(dy) - center);
            if fx * fx + fy * fy <= r * r + 1e-9 {
                out.push((dx, dy));
            }
        }
    }
    out
}

/// Offsets of a square brush of `size` pixels, centered on the cursor pixel.
pub fn brush_offsets(size: u32) -> impl Iterator<Item = (i32, i32)> {
    let size = size.clamp(1, 64) as i32;
    let start = -(size - 1) / 2;
    let end = start + size;
    (start..end).flat_map(move |dy| (start..end).map(move |dx| (dx, dy)))
}

/// Indices of all pixels 4-connected to `(x, y)` that have exactly the same color.
///
/// Returns an empty list if the seed is outside the buffer.
pub fn flood_region(buf: &PixelBuffer, x: i32, y: i32) -> Vec<u32> {
    let Some(seed) = buf.index_of(x, y) else {
        return Vec::new();
    };
    let Some(target) = buf.get_index(seed) else {
        return Vec::new();
    };
    let w = buf.width() as i32;
    let h = buf.height() as i32;
    let mut visited = vec![false; (w * h) as usize];
    let mut region = Vec::new();
    let mut stack = vec![(x, y)];
    visited[seed as usize] = true;
    while let Some((cx, cy)) = stack.pop() {
        region.push((cy * w + cx) as u32);
        for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let ni = (ny * w + nx) as usize;
            if visited[ni] {
                continue;
            }
            if buf.get_index(ni as u32) == Some(target) {
                visited[ni] = true;
                stack.push((nx, ny));
            }
        }
    }
    region
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_line(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        line(a, b, |x, y| v.push((x, y)));
        v
    }

    #[test]
    fn line_endpoints_and_continuity() {
        let pts = collect_line((0, 0), (7, 3));
        assert_eq!(pts.first(), Some(&(0, 0)));
        assert_eq!(pts.last(), Some(&(7, 3)));
        for w in pts.windows(2) {
            assert!((w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1);
        }
        assert_eq!(collect_line((2, 2), (2, 2)), vec![(2, 2)]);
        assert_eq!(collect_line((3, 0), (0, 0)).len(), 4);
    }

    #[test]
    fn outline_visits_once() {
        let mut v = Vec::new();
        rect_outline((3, 3), (0, 0), |x, y| v.push((x, y)));
        let mut sorted = v.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), v.len());
        assert_eq!(v.len(), 12);
    }

    #[test]
    fn ellipses_fit_their_box_and_are_symmetric() {
        for (w, h) in (1..=17).flat_map(|w| (1..=17).map(move |h| (w, h))) {
            let mut pts = Vec::new();
            ellipse(
                (10, 10),
                (10 + w - 1, 10 + h - 1),
                false,
                100,
                100,
                |x, y| pts.push((x, y)),
            );
            let (minx, maxx) = (pts.iter().map(|p| p.0).min(), pts.iter().map(|p| p.0).max());
            let (miny, maxy) = (pts.iter().map(|p| p.1).min(), pts.iter().map(|p| p.1).max());
            assert_eq!((minx, maxx), (Some(10), Some(10 + w - 1)), "{w}×{h} width");
            assert_eq!((miny, maxy), (Some(10), Some(10 + h - 1)), "{w}×{h} height");
            let set: std::collections::HashSet<_> = pts.iter().copied().collect();
            assert_eq!(set.len(), pts.len(), "outline pixels are visited once");
            for &(x, y) in &pts {
                // Mirror symmetric in both axes.
                assert!(set.contains(&(20 + w - 1 - x, y)), "{w}×{h} ({x},{y})");
                assert!(set.contains(&(x, 20 + h - 1 - y)), "{w}×{h} ({x},{y})");
            }
            let mut fill = Vec::new();
            ellipse(
                (10, 10),
                (10 + w - 1, 10 + h - 1),
                true,
                100,
                100,
                |x, y| fill.push((x, y)),
            );
            assert!(fill.len() >= pts.len());
            for p in &pts {
                assert!(fill.contains(p), "filled includes the outline");
            }
        }
        // Filled ellipses are clipped to the image.
        let mut n = 0;
        ellipse((-50, -50), (50, 50), true, 10, 10, |x, y| {
            assert!((0..10).contains(&x) && (0..10).contains(&y));
            n += 1;
        });
        assert_eq!(n, 100);
    }

    #[test]
    fn circle_brushes() {
        assert_eq!(circle_offsets(1), vec![(0, 0)]);
        assert_eq!(circle_offsets(2).len(), 4);
        assert_eq!(circle_offsets(3).len(), 5, "a plus");
        let c = circle_offsets(8);
        assert!(c.len() < 64 && c.len() > 40);
        assert!(!c.contains(&(-3, -3)), "no corners");
    }

    #[test]
    fn brush_sizes() {
        assert_eq!(brush_offsets(1).collect::<Vec<_>>(), vec![(0, 0)]);
        assert_eq!(brush_offsets(2).count(), 4);
        assert!(brush_offsets(3).any(|o| o == (-1, -1)));
    }
}

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
    fn brush_sizes() {
        assert_eq!(brush_offsets(1).collect::<Vec<_>>(), vec![(0, 0)]);
        assert_eq!(brush_offsets(2).count(), 4);
        assert!(brush_offsets(3).any(|o| o == (-1, -1)));
    }
}

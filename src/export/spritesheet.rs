//! Sprite sheets: frames laid out on one image, plus where each frame ended up.
//!
//! In grid layouts every frame gets a cell of the same size (the frame size, or the
//! largest trimmed size when trimming), so animations don't jump around in game
//! engines. Packed sheets place each (trimmed) frame on shelves instead, trying
//! several sheet widths and keeping the smallest result; identical frames can share
//! one spot. Packing is deterministic: the same frames always give the same sheet,
//! and frames never overlap. With extrude
//! `e`, each cell grows by `e` on every side and the frame's edge pixels are repeated
//! into that border. All sizes are computed with checked 64-bit math and validated
//! before the sheet is allocated.

use image::RgbaImage;

use super::render::{content_bounds, FrameImage};
use super::{check_output_size, ExportError, SheetLayout, SheetOptions};

/// A rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Where one frame lands on the sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// Position of the frame in the export (0-based).
    pub order: usize,
    /// Position of the frame in the document.
    pub frame_index: usize,
    pub duration_ms: u32,
    /// The frame's pixels on the sheet (excluding extrude).
    pub frame: Rect,
    /// Which part of the (scaled) source frame those pixels are.
    pub source: Rect,
    /// Full (scaled) source frame size.
    pub source_w: u32,
    pub source_h: u32,
    pub trimmed: bool,
    /// Packed sheets with merged duplicates: the export position of the identical
    /// frame whose pixels this one shares.
    pub duplicate_of: Option<usize>,
}

/// The computed layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetPlan {
    pub width: u32,
    pub height: u32,
    /// Grid size (0 × 0 for packed sheets).
    pub columns: u32,
    pub rows: u32,
    pub cell_w: u32,
    pub cell_h: u32,
    pub placements: Vec<Placement>,
}

/// Grid dimensions for `n` frames.
pub fn grid(layout: SheetLayout, n: u32, options: &SheetOptions) -> (u32, u32) {
    let n = n.max(1);
    match layout {
        SheetLayout::Horizontal => (n, 1),
        SheetLayout::Vertical => (1, n),
        SheetLayout::Columns => {
            let cols = options.columns.clamp(1, n);
            (cols, n.div_ceil(cols))
        }
        SheetLayout::Rows => {
            let rows = options.rows.clamp(1, n);
            let cols = n.div_ceil(rows);
            (cols, n.div_ceil(cols))
        }
    }
}

/// Size of a sheet with `columns × rows` cells of `cell_w × cell_h` pixels.
pub fn sheet_size(
    cell_w: u32,
    cell_h: u32,
    columns: u32,
    rows: u32,
    options: &SheetOptions,
) -> (u64, u64) {
    let e = u64::from(options.extrude);
    let gap = u64::from(options.spacing);
    let pad = u64::from(options.padding);
    let w = 2 * pad + u64::from(columns) * (u64::from(cell_w) + 2 * e + gap) - gap;
    let h = 2 * pad + u64::from(rows) * (u64::from(cell_h) + 2 * e + gap) - gap;
    (w, h)
}

/// Computes the layout for already-scaled frames, without allocating the sheet.
pub fn plan(frames: &[FrameImage], options: &SheetOptions) -> Result<SheetPlan, ExportError> {
    plan_scaled(frames, 1, options)
}

/// The layout the frames will have once scaled by `scale` (for sizes shown before
/// exporting; nothing is scaled here).
pub fn plan_scaled(
    frames: &[FrameImage],
    scale: u32,
    options: &SheetOptions,
) -> Result<SheetPlan, ExportError> {
    if frames.is_empty() {
        return Err(ExportError::NoFrames);
    }
    let n =
        u32::try_from(frames.len()).map_err(|_| ExportError::Invalid("too many frames".into()))?;
    let scale = scale.max(1);
    let (w0, h0) = frames[0].image.dimensions();
    let big = |v: u32| {
        v.checked_mul(scale).ok_or(ExportError::TooLarge {
            width: u64::from(w0) * u64::from(scale),
            height: u64::from(h0) * u64::from(scale),
        })
    };
    let (source_w, source_h) = (big(w0)?, big(h0)?);
    let unscaled: Vec<Rect> = frames
        .iter()
        .map(|f| {
            if options.trim || options.packed {
                // Empty frames keep a 1×1 transparent pixel so every frame has a rect.
                let (x, y, w, h) = content_bounds(&f.image).unwrap_or((0, 0, 1, 1));
                Rect { x, y, w, h }
            } else {
                Rect {
                    x: 0,
                    y: 0,
                    w: w0,
                    h: h0,
                }
            }
        })
        .collect();
    let sources: Vec<Rect> = unscaled
        .iter()
        .map(|r| {
            Ok(Rect {
                x: big(r.x)?,
                y: big(r.y)?,
                w: big(r.w)?,
                h: big(r.h)?,
            })
        })
        .collect::<Result<_, ExportError>>()?;
    if options.packed {
        let duplicates = if options.merge_duplicates {
            find_duplicates(frames, &unscaled)
        } else {
            vec![None; frames.len()]
        };
        return pack(frames, &sources, &duplicates, source_w, source_h, options);
    }
    let cell_w = sources.iter().map(|r| r.w).max().unwrap_or(1);
    let cell_h = sources.iter().map(|r| r.h).max().unwrap_or(1);
    let (columns, rows) = grid(options.layout, n, options);

    let e = u64::from(options.extrude);
    let gap = u64::from(options.spacing);
    let pad = u64::from(options.padding);
    let stride_x = u64::from(cell_w) + 2 * e + gap;
    let stride_y = u64::from(cell_h) + 2 * e + gap;
    let (width, height) = sheet_size(cell_w, cell_h, columns, rows, options);
    check_output_size(width, height)?;

    let placements = frames
        .iter()
        .zip(&sources)
        .enumerate()
        .map(|(i, (f, src))| {
            let (col, row) = (i as u64 % u64::from(columns), i as u64 / u64::from(columns));
            Placement {
                order: i,
                frame_index: f.index,
                duration_ms: f.duration_ms,
                frame: Rect {
                    x: (pad + col * stride_x + e) as u32,
                    y: (pad + row * stride_y + e) as u32,
                    w: src.w,
                    h: src.h,
                },
                source: *src,
                source_w,
                source_h,
                trimmed: options.trim && (src.w != source_w || src.h != source_h),
                duplicate_of: None,
            }
        })
        .collect();
    Ok(SheetPlan {
        width: width as u32,
        height: height as u32,
        columns,
        rows,
        cell_w,
        cell_h,
        placements,
    })
}

/// For each frame, the first earlier frame with exactly the same pixels in its
/// (unscaled, trimmed) rect, if any.
fn find_duplicates(frames: &[FrameImage], rects: &[Rect]) -> Vec<Option<usize>> {
    let pixels = |i: usize| {
        let r = rects[i];
        let img = &frames[i].image;
        (r.y..r.y + r.h).flat_map(move |y| (r.x..r.x + r.w).map(move |x| img.get_pixel(x, y).0))
    };
    let mut out: Vec<Option<usize>> = vec![None; frames.len()];
    let mut seen: std::collections::HashMap<(u32, u32, u64), Vec<usize>> =
        std::collections::HashMap::new();
    for i in 0..frames.len() {
        let r = rects[i];
        // FNV-1a over the pixels picks candidates; a full comparison confirms them.
        let hash = pixels(i).flatten().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
        });
        let bucket = seen.entry((r.w, r.h, hash)).or_default();
        out[i] = bucket.iter().copied().find(|&j| pixels(i).eq(pixels(j)));
        if out[i].is_none() {
            bucket.push(i);
        }
    }
    out
}

/// Top-left corner of a frame footprint on a packed sheet (before padding).
type Spot = (u64, u64);

/// Shelf-packs the frames (duplicates share their original's spot), trying several
/// sheet widths and keeping the smallest sheet that fits the export limits.
fn pack(
    frames: &[FrameImage],
    sources: &[Rect],
    duplicates: &[Option<usize>],
    source_w: u32,
    source_h: u32,
    options: &SheetOptions,
) -> Result<SheetPlan, ExportError> {
    let e = u64::from(options.extrude);
    let gap = u64::from(options.spacing);
    let pad = u64::from(options.padding);
    // Unique frames with their footprint (frame + extrude on both sides).
    let mut items: Vec<(usize, u64, u64)> = (0..frames.len())
        .filter(|&i| duplicates[i].is_none())
        .map(|i| {
            (
                i,
                u64::from(sources[i].w) + 2 * e,
                u64::from(sources[i].h) + 2 * e,
            )
        })
        .collect();
    // Tallest first, then widest, then export order: deterministic.
    items.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)).then(a.0.cmp(&b.0)));
    let max_w = items.iter().map(|it| it.1).max().unwrap_or(1);
    let sum_w: u64 = items.iter().map(|it| it.1 + gap).sum();
    let area: u64 = items.iter().map(|it| (it.1 + gap) * (it.2 + gap)).sum();
    let mut widths: Vec<u64> = (0..=24u64)
        .map(|k| max_w + (sum_w.saturating_sub(max_w)) * k / 24)
        .collect();
    widths.push(((area as f64).sqrt().ceil() as u64).max(max_w));
    widths.sort_unstable();
    widths.dedup();

    let shelf = |limit: u64| {
        let (mut x, mut y, mut shelf_h, mut used_w) = (0u64, 0u64, 0u64, 0u64);
        let mut spots = vec![(0u64, 0u64); frames.len()];
        for &(i, w, h) in &items {
            if x > 0 && x + w > limit {
                y += shelf_h + gap;
                x = 0;
                shelf_h = 0;
            }
            spots[i] = (x, y);
            used_w = used_w.max(x + w);
            x += w + gap;
            shelf_h = shelf_h.max(h);
        }
        (used_w + 2 * pad, y + shelf_h + 2 * pad, spots)
    };
    let mut best: Option<(u64, u64, Vec<Spot>)> = None;
    let mut smallest_rejected = None;
    for limit in widths {
        let (w, h, spots) = shelf(limit);
        if check_output_size(w, h).is_err() {
            smallest_rejected.get_or_insert((w, h));
            continue;
        }
        let better = best
            .as_ref()
            .is_none_or(|(bw, bh, _)| (w * h, w.max(h), w) < (bw * bh, (*bw).max(*bh), *bw));
        if better {
            best = Some((w, h, spots));
        }
    }
    let Some((width, height, spots)) = best else {
        let (width, height) = smallest_rejected.unwrap_or((0, 0));
        return Err(ExportError::TooLarge { width, height });
    };
    let mut placements: Vec<Placement> = Vec::with_capacity(frames.len());
    for (i, f) in frames.iter().enumerate() {
        let src = sources[i];
        let frame = match duplicates[i] {
            Some(j) => placements[j].frame,
            None => Rect {
                x: (pad + spots[i].0 + e) as u32,
                y: (pad + spots[i].1 + e) as u32,
                w: src.w,
                h: src.h,
            },
        };
        placements.push(Placement {
            order: i,
            frame_index: f.index,
            duration_ms: f.duration_ms,
            frame,
            source: src,
            source_w,
            source_h,
            trimmed: src.w != source_w || src.h != source_h,
            duplicate_of: duplicates[i],
        });
    }
    Ok(SheetPlan {
        width: width as u32,
        height: height as u32,
        columns: 0,
        rows: 0,
        cell_w: sources.iter().map(|r| r.w).max().unwrap_or(1),
        cell_h: sources.iter().map(|r| r.h).max().unwrap_or(1),
        placements,
    })
}

/// Draws the sheet. `frames` must be the ones `plan` was computed from.
pub fn compose(
    frames: &[FrameImage],
    plan: &SheetPlan,
    options: &SheetOptions,
) -> Result<RgbaImage, ExportError> {
    check_output_size(u64::from(plan.width), u64::from(plan.height))?;
    let mut sheet = RgbaImage::new(plan.width, plan.height);
    if let Some(bg) = options.background {
        for px in sheet.pixels_mut() {
            px.0 = bg.0;
        }
    }
    let e = options.extrude as i64;
    for (f, p) in frames.iter().zip(&plan.placements) {
        if p.duplicate_of.is_some() {
            continue; // already drawn by the frame it shares pixels with
        }
        let (fx, fy) = (i64::from(p.frame.x), i64::from(p.frame.y));
        // Destination rect including the extrude border; source coords are clamped
        // to the frame's rect, which repeats the edge pixels outward.
        for dy in -e..i64::from(p.frame.h) + e {
            for dx in -e..i64::from(p.frame.w) + e {
                let sx = dx.clamp(0, i64::from(p.frame.w) - 1) as u32 + p.source.x;
                let sy = dy.clamp(0, i64::from(p.frame.h) - 1) as u32 + p.source.y;
                let (tx, ty) = (fx + dx, fy + dy);
                if tx < 0 || ty < 0 || tx >= i64::from(plan.width) || ty >= i64::from(plan.height) {
                    continue;
                }
                let px = *f.image.get_pixel(sx, sy);
                if options.background.is_some() && px.0[3] < 255 {
                    let base = crate::core::Rgba(sheet.get_pixel(tx as u32, ty as u32).0);
                    let c = crate::core::Rgba(px.0).over(base);
                    sheet.put_pixel(tx as u32, ty as u32, image::Rgba(c.0));
                } else {
                    sheet.put_pixel(tx as u32, ty as u32, px);
                }
            }
        }
    }
    Ok(sheet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_shapes() {
        let o = SheetOptions {
            columns: 3,
            rows: 2,
            ..SheetOptions::default()
        };
        assert_eq!(grid(SheetLayout::Horizontal, 5, &o), (5, 1));
        assert_eq!(grid(SheetLayout::Vertical, 5, &o), (1, 5));
        assert_eq!(grid(SheetLayout::Columns, 7, &o), (3, 3));
        assert_eq!(grid(SheetLayout::Rows, 7, &o), (4, 2));
        assert_eq!(
            grid(SheetLayout::Columns, 2, &o),
            (2, 1),
            "never more columns than frames"
        );
    }
}

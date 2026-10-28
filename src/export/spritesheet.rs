//! Sprite sheets: frames laid out on one image, plus where each frame ended up.
//!
//! Every frame gets a cell of the same size (the frame size, or the largest trimmed
//! size when trimming), so animations don't jump around in game engines. With extrude
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
}

/// The computed layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetPlan {
    pub width: u32,
    pub height: u32,
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
    if frames.is_empty() {
        return Err(ExportError::NoFrames);
    }
    let n =
        u32::try_from(frames.len()).map_err(|_| ExportError::Invalid("too many frames".into()))?;
    let (source_w, source_h) = frames[0].image.dimensions();
    let sources: Vec<Rect> = frames
        .iter()
        .map(|f| {
            if options.trim {
                // Empty frames keep a 1×1 transparent pixel so every frame has a rect.
                let (x, y, w, h) = content_bounds(&f.image).unwrap_or((0, 0, 1, 1));
                Rect { x, y, w, h }
            } else {
                Rect {
                    x: 0,
                    y: 0,
                    w: source_w,
                    h: source_h,
                }
            }
        })
        .collect();
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

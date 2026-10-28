//! Stage 1 and 2: snapshot frames as RGBA images, scale and trim them.
//!
//! Nothing here keeps a reference to the document; exporters work on the snapshot.

use image::RgbaImage;

use super::{check_output_size, ExportError, LayerSource, MAX_SCALE};
use crate::core::{Document, Rgba};

/// One frame, flattened and ready to encode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameImage {
    /// Position of the frame in the document (0-based).
    pub index: usize,
    pub duration_ms: u32,
    pub image: RgbaImage,
}

/// Composites the given frames (document positions, in order). Only reads the document.
///
/// Onion skin, grid, selection outlines and other canvas overlays are never part of the
/// document, so they can't appear in the result.
pub fn snapshot(
    doc: &Document,
    frames: &[usize],
    layers: LayerSource,
) -> Result<Vec<FrameImage>, ExportError> {
    if frames.is_empty() {
        return Err(ExportError::NoFrames);
    }
    let (w, h) = (doc.width(), doc.height());
    let mut out = Vec::with_capacity(frames.len());
    for &index in frames {
        let frame = doc
            .frames()
            .get(index)
            .ok_or(crate::core::CoreError::FrameIndex(index))?;
        let bytes = match layers {
            LayerSource::Visible => doc.composite_frame(index)?.into_bytes(),
            LayerSource::ActiveLayer => doc
                .cel(index, doc.active_index())
                .ok_or(crate::core::CoreError::FrameIndex(index))?
                .as_bytes()
                .to_vec(),
        };
        let image = RgbaImage::from_raw(w, h, bytes)
            .ok_or_else(|| ExportError::Invalid("frame data has the wrong size".into()))?;
        out.push(FrameImage {
            index,
            duration_ms: frame.duration_ms(),
            image,
        });
    }
    Ok(out)
}

/// Output size of an image scaled by an integer factor, checked against the limits.
pub fn scaled_size(width: u32, height: u32, scale: u32) -> Result<(u32, u32), ExportError> {
    if !(1..=MAX_SCALE).contains(&scale) {
        return Err(ExportError::Invalid(format!(
            "scale must be between 1 and {MAX_SCALE}"
        )));
    }
    let (w, h) = (
        u64::from(width) * u64::from(scale),
        u64::from(height) * u64::from(scale),
    );
    check_output_size(w, h)?;
    Ok((w as u32, h as u32))
}

/// Nearest-neighbor upscale by an integer factor: every pixel becomes a crisp
/// `scale × scale` block. No smoothing, ever.
pub fn scale_nearest(img: &RgbaImage, scale: u32) -> Result<RgbaImage, ExportError> {
    let (w, h) = scaled_size(img.width(), img.height(), scale)?;
    if scale == 1 {
        return Ok(img.clone());
    }
    let src = img.as_raw();
    let src_stride = img.width() as usize * 4;
    let dst_stride = w as usize * 4;
    let mut out = vec![0u8; dst_stride * h as usize];
    let mut row = vec![0u8; dst_stride];
    for y in 0..img.height() as usize {
        let src_row = &src[y * src_stride..(y + 1) * src_stride];
        for (x, px) in src_row.chunks_exact(4).enumerate() {
            for k in 0..scale as usize {
                let d = (x * scale as usize + k) * 4;
                row[d..d + 4].copy_from_slice(px);
            }
        }
        for k in 0..scale as usize {
            let d = (y * scale as usize + k) * dst_stride;
            out[d..d + dst_stride].copy_from_slice(&row);
        }
    }
    RgbaImage::from_raw(w, h, out).ok_or_else(|| ExportError::Invalid("scaling failed".into()))
}

/// Bounding box `(x, y, w, h)` of pixels that are not fully transparent, or `None` if
/// the image is empty.
pub fn content_bounds(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    let mut any = false;
    for (x, y, px) in img.enumerate_pixels() {
        if px.0[3] != 0 {
            any = true;
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    any.then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Paints the image over a solid background, producing an opaque RGB image.
pub fn flatten(img: &RgbaImage, background: Rgba) -> image::RgbImage {
    let bg = Rgba::opaque(background.r(), background.g(), background.b());
    let mut out = image::RgbImage::new(img.width(), img.height());
    for (src, dst) in img.pixels().zip(out.pixels_mut()) {
        let c = Rgba(src.0).over(bg);
        dst.0 = [c.r(), c.g(), c.b()];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_scale_makes_blocks() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 0, 255, 128]));
        let big = scale_nearest(&img, 3).expect("scale");
        assert_eq!(big.dimensions(), (6, 3));
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(big.get_pixel(x, y).0, [255, 0, 0, 255]);
                assert_eq!(big.get_pixel(x + 3, y).0, [0, 0, 255, 128]);
            }
        }
    }

    #[test]
    fn size_limits_are_checked_before_allocating() {
        assert!(scaled_size(2048, 2048, 5).is_ok(), "10240² × 4 B ≈ 400 MiB");
        assert!(matches!(
            scaled_size(2048, 2048, 6),
            Err(ExportError::TooLarge { .. })
        ));
        assert!(scaled_size(10, 10, 0).is_err());
        assert!(scaled_size(10, 10, MAX_SCALE + 1).is_err());
    }

    #[test]
    fn bounds_of_content() {
        let mut img = RgbaImage::new(5, 4);
        assert_eq!(content_bounds(&img), None);
        img.put_pixel(1, 2, image::Rgba([1, 2, 3, 1]));
        img.put_pixel(3, 1, image::Rgba([1, 2, 3, 255]));
        assert_eq!(content_bounds(&img), Some((1, 1, 3, 2)));
    }

    #[test]
    fn flatten_uses_background_for_transparency() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(1, 0, image::Rgba([0, 0, 0, 255]));
        let out = flatten(&img, Rgba::opaque(10, 20, 30));
        assert_eq!(out.get_pixel(0, 0).0, [10, 20, 30]);
        assert_eq!(out.get_pixel(1, 0).0, [0, 0, 0]);
    }
}

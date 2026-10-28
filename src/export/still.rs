//! Stage 3 for still images: PNG, JPEG and WebP encoders.

use std::io::Write;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

use super::render::flatten;
use super::{ExportError, JpegOptions, StillFormat};

/// JPEG can't store images wider or taller than this.
pub const JPEG_MAX_SIDE: u32 = 65_535;

/// Warning shown for JPEG in the export dialog and docs.
pub const JPEG_WARNING: &str = "JPEG uses lossy compression and may introduce artifacts into pixel art. PNG is recommended for lossless game sprites.";

/// Encodes one image in the given format.
pub fn encode(
    format: StillFormat,
    img: &RgbaImage,
    jpeg: &JpegOptions,
    out: &mut dyn Write,
) -> Result<(), ExportError> {
    match format {
        StillFormat::Png => encode_png(img, out),
        StillFormat::Jpeg => encode_jpeg(img, jpeg, out),
        StillFormat::WebP => encode_webp(img, out),
    }
}

/// Lossless RGBA PNG; pixels round-trip exactly.
pub fn encode_png(img: &RgbaImage, out: &mut dyn Write) -> Result<(), ExportError> {
    PngEncoder::new(out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| ExportError::encode("PNG", e))
}

/// Opaque baseline JPEG; transparent areas are painted with `options.background`.
pub fn encode_jpeg(
    img: &RgbaImage,
    options: &JpegOptions,
    out: &mut dyn Write,
) -> Result<(), ExportError> {
    if img.width() > JPEG_MAX_SIDE || img.height() > JPEG_MAX_SIDE {
        return Err(ExportError::Invalid(format!(
            "JPEG images can be at most {JPEG_MAX_SIDE} px wide and tall"
        )));
    }
    let quality = options.quality.clamp(1, 100);
    let rgb = flatten(img, options.background);
    JpegEncoder::new_with_quality(out, quality)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| ExportError::encode("JPEG", e))
}

/// Lossless WebP with transparency (pure-Rust encoder; it has no lossy mode).
pub fn encode_webp(img: &RgbaImage, out: &mut dyn Write) -> Result<(), ExportError> {
    WebPEncoder::new_lossless(out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| ExportError::encode("WebP", e))
}

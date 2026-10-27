//! PNG import and export.

use std::io::{Cursor, Write};
use std::path::Path;

use image::codecs::png::{PngDecoder, PngEncoder};
use image::{ExtendedColorType, ImageDecoder, ImageEncoder, Limits};

use super::{check_file_size, write_atomic, FileError, MAX_PNG_FILE_BYTES};
use crate::core::{validate_size, Document, PixelBuffer, MAX_CANVAS_SIZE};

/// Encodes a pixel buffer as an RGBA PNG.
pub fn encode_png(buf: &PixelBuffer, out: &mut impl Write) -> Result<(), FileError> {
    PngEncoder::new(out).write_image(
        buf.as_bytes(),
        buf.width(),
        buf.height(),
        ExtendedColorType::Rgba8,
    )?;
    Ok(())
}

/// Decodes PNG bytes into an RGBA buffer, checking the size *before* decoding pixels.
pub fn decode_png(bytes: &[u8]) -> Result<PixelBuffer, FileError> {
    let mut decoder = PngDecoder::new(Cursor::new(bytes))?;
    let (width, height) = decoder.dimensions();
    validate_size(width, height).map_err(|_| {
        FileError::TooLarge(format!(
            "image is {width}×{height}; Kagero supports up to {MAX_CANVAS_SIZE}×{MAX_CANVAS_SIZE}"
        ))
    })?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_CANVAS_SIZE);
    limits.max_image_height = Some(MAX_CANVAS_SIZE);
    // RGBA16 at the largest size, plus decoder scratch space.
    limits.max_alloc = Some(u64::from(MAX_CANVAS_SIZE).pow(2) * 8 + 16 * 1024 * 1024);
    decoder.set_limits(limits)?;
    let rgba = image::DynamicImage::from_decoder(decoder)?.into_rgba8();
    Ok(PixelBuffer::from_raw(width, height, rgba.into_raw())?)
}

/// Exports the composited document as a PNG with transparency.
pub fn export_png(doc: &Document, path: &Path) -> Result<(), FileError> {
    let merged = doc.composite()?;
    write_atomic(path, |w| encode_png(&merged, w))
}

/// Opens a PNG as a new single-layer document.
pub fn import_png(path: &Path) -> Result<Document, FileError> {
    check_file_size(path, MAX_PNG_FILE_BYTES)?;
    let bytes = std::fs::read(path)?;
    let pixels = decode_png(&bytes)?;
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .and_then(|s| crate::core::sanitize_layer_name(&s))
        .unwrap_or_else(|| "Layer 1".to_owned());
    Ok(Document::from_layers(
        pixels.width(),
        pixels.height(),
        vec![(name, true, pixels)],
        0,
    )?)
}

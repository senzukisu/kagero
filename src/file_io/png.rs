//! PNG import and export.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

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

/// Exports the composited active frame as a PNG with transparency.
pub fn export_png(doc: &Document, path: &Path) -> Result<(), FileError> {
    let merged = doc.composite()?;
    write_atomic(path, |w| encode_png(&merged, w))
}

/// File names used by [`export_frames`]: `<base>_0001.png`, `<base>_0002.png`, …
pub fn frame_export_paths(doc: &Document, dir: &Path, base: &str) -> Vec<PathBuf> {
    let base = sanitize_file_stem(base);
    (1..=doc.frame_count())
        .map(|i| dir.join(format!("{base}_{i:04}.png")))
        .collect()
}

/// Exports every frame as its own flattened PNG into `dir`. Returns the written paths.
pub fn export_frames(doc: &Document, dir: &Path, base: &str) -> Result<Vec<PathBuf>, FileError> {
    let paths = frame_export_paths(doc, dir, base);
    for (index, path) in paths.iter().enumerate() {
        let merged = doc.composite_frame(index)?;
        write_atomic(path, |w| encode_png(&merged, w))?;
    }
    Ok(paths)
}

/// Keeps only characters that are safe in Windows file names.
fn sanitize_file_stem(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "frame".to_owned()
    } else {
        trimmed.chars().take(100).collect()
    }
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

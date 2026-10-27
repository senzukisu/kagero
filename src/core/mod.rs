//! Pixel engine: RGBA colors, pixel buffers, layers, documents and raster algorithms.
//!
//! Nothing in this module knows about the UI or about files.

pub mod animation;
mod buffer;
mod color;
mod document;
pub mod onion_skin;
pub mod raster;

pub use animation::{Frame, FrameId};
pub use buffer::PixelBuffer;
pub use color::Rgba;
pub use document::{Document, LayerData, LayerId, LayerInfo, PixelRect};

/// Largest allowed image side, in pixels.
pub const MAX_CANVAS_SIZE: u32 = 2048;
/// Largest number of layers in one document.
pub const MAX_LAYERS: usize = 64;
/// Largest number of frames in one document.
pub const MAX_FRAMES: usize = 1000;
/// Upper bound for the total size of all cel pixel data (layers × frames) in one document.
pub const MAX_DOCUMENT_BYTES: u64 = 512 * 1024 * 1024;
/// Longest layer name, in characters.
pub const MAX_LAYER_NAME_CHARS: usize = 64;

/// Errors produced by the pixel engine.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("image size {width}×{height} is not allowed (must be 1–{max} pixels per side)", max = MAX_CANVAS_SIZE)]
    InvalidSize { width: u32, height: u32 },
    #[error("pixel data has {actual} bytes, expected {expected}")]
    BufferSize { expected: usize, actual: usize },
    #[error("a document can have at most {max} layers", max = MAX_LAYERS)]
    TooManyLayers,
    #[error("a document can have at most {max} frames", max = MAX_FRAMES)]
    TooManyFrames,
    #[error("not enough memory budget (layers × frames × image size is limited to 512 MiB)")]
    MemoryBudget,
    #[error("a document must keep at least one layer")]
    LastLayer,
    #[error("a document must keep at least one frame")]
    LastFrame,
    #[error("layer index {0} is out of range")]
    LayerIndex(usize),
    #[error("frame index {0} is out of range")]
    FrameIndex(usize),
    #[error("expected {expected} layer images in a frame, found {actual}")]
    CelCount { expected: usize, actual: usize },
    #[error("a layer image for one of the frames is missing")]
    FrameMissing,
}

/// Checks that `width`×`height` is an allowed canvas size.
pub fn validate_size(width: u32, height: u32) -> Result<(), CoreError> {
    if width == 0 || height == 0 || width > MAX_CANVAS_SIZE || height > MAX_CANVAS_SIZE {
        return Err(CoreError::InvalidSize { width, height });
    }
    Ok(())
}

/// Bytes used by `layers` RGBA layers of the given size.
pub fn layers_bytes(width: u32, height: u32, layers: usize) -> u64 {
    u64::from(width) * u64::from(height) * 4 * layers as u64
}

/// Trims and limits a layer name; returns `None` if it is empty after trimming.
pub fn sanitize_layer_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_LAYER_NAME_CHARS)
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

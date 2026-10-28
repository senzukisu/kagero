//! Export: turns frames of a document into image files, animations and sprite sheets.
//!
//! The pipeline has three stages and never touches the document after the first one:
//!
//! 1. **Snapshot** ([`render::snapshot`]) — composites the chosen frames into plain RGBA
//!    images on the UI thread. The document, its history and the editor are only read.
//! 2. **Prepare** — nearest-neighbor scaling, trimming and sprite-sheet layout
//!    ([`render`], [`spritesheet`]).
//! 3. **Encode and write** — PNG, JPEG, WebP ([`still`]), GIF ([`gif`]) and JSON
//!    metadata ([`metadata`]), driven by [`service`], usually on a background thread
//!    with progress and cancellation.
//!
//! See `docs/user-guide/export.md` and `docs/developers/export-pipeline.md`.

pub mod gif;
pub mod metadata;
pub mod options;
pub mod render;
pub mod sequence;
pub mod service;
pub mod spritesheet;
pub mod still;

pub use options::*;

use std::path::PathBuf;

/// Largest exported image side, in pixels.
pub const MAX_OUTPUT_SIDE: u64 = 16_384;
/// Largest exported image in bytes of RGBA data (checked before allocating).
pub const MAX_OUTPUT_BYTES: u64 = 512 * 1024 * 1024;
/// Largest integer export scale.
pub const MAX_SCALE: u32 = 64;

/// Errors from exporting. The `Display` text is shown to the user.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("there are no frames to export")]
    NoFrames,
    #[error("the result would be {width}×{height} px, larger than Kagero can export ({max}×{max} px and 512 MiB). Use a smaller scale or fewer frames", max = MAX_OUTPUT_SIDE)]
    TooLarge { width: u64, height: u64 },
    #[error("{0}")]
    Invalid(String),
    #[error("the {format} encoder failed: {message}")]
    Encode {
        format: &'static str,
        message: String,
    },
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the export was canceled")]
    Canceled,
    #[error(transparent)]
    Core(#[from] crate::core::CoreError),
    #[error(transparent)]
    File(#[from] crate::file_io::FileError),
}

impl ExportError {
    pub(crate) fn encode(format: &'static str, e: impl std::fmt::Display) -> Self {
        ExportError::Encode {
            format,
            message: e.to_string(),
        }
    }
}

/// What an export produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportSummary {
    pub files: Vec<PathBuf>,
    /// Notes worth showing (e.g. colors reduced for GIF).
    pub notes: Vec<String>,
}

/// Checks an output size against the limits before any buffer is allocated.
pub fn check_output_size(width: u64, height: u64) -> Result<(), ExportError> {
    if width == 0 || height == 0 {
        return Err(ExportError::Invalid("the result would be empty".into()));
    }
    let bytes = width.checked_mul(height).and_then(|p| p.checked_mul(4));
    if width > MAX_OUTPUT_SIDE
        || height > MAX_OUTPUT_SIDE
        || bytes.is_none_or(|b| b > MAX_OUTPUT_BYTES)
    {
        return Err(ExportError::TooLarge { width, height });
    }
    Ok(())
}

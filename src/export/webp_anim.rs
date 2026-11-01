//! Animated WebP: lossless frames with full transparency and per-frame durations.
//!
//! Each frame is encoded with the same lossless encoder as still WebP images. Its
//! `VP8L` bitstream is then wrapped in the animation container from the WebP spec
//! (`RIFF`/`WEBP` with `VP8X`, `ANIM` and one `ANMF` chunk per frame). Every frame
//! covers the whole canvas and replaces the previous one ("do not blend", no
//! disposal), so what you see is exactly each exported frame.

use std::io::Write;

use super::render::FrameImage;
use super::{still, ExportError, GifLoop};

/// Largest side of a lossless WebP image (14-bit size fields).
pub const WEBP_MAX_SIDE: u32 = 16_384;
/// Longest frame duration the format can store (24 bits, milliseconds).
const MAX_DURATION_MS: u32 = 0xFF_FFFF;

/// Appends a RIFF chunk (padded to an even size).
fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) -> Result<(), ExportError> {
    let len = u32::try_from(payload.len())
        .map_err(|_| ExportError::Invalid("an animation frame is too large for WebP".into()))?;
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    Ok(())
}

fn u24(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes()[..3]);
}

/// The image chunks (`VP8L`, or `ALPH` + `VP8 `) of a still WebP file, as they are
/// stored (header, payload, padding).
fn image_chunks(file: &[u8]) -> Result<Vec<u8>, ExportError> {
    let bad = || ExportError::encode("WebP", "unexpected output from the frame encoder");
    if file.len() < 12 || &file[..4] != b"RIFF" || &file[8..12] != b"WEBP" {
        return Err(bad());
    }
    let mut out = Vec::new();
    let mut pos = 12;
    while pos + 8 <= file.len() {
        let fourcc = &file[pos..pos + 4];
        let len = u32::from_le_bytes([file[pos + 4], file[pos + 5], file[pos + 6], file[pos + 7]])
            as usize;
        let end = pos + 8 + len + len % 2;
        if pos + 8 + len > file.len() {
            return Err(bad());
        }
        if matches!(fourcc, b"VP8L" | b"VP8 " | b"ALPH") {
            out.extend_from_slice(&file[pos..end.min(file.len())]);
            if end > file.len() {
                out.push(0);
            }
        }
        pos = end;
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(out)
}

/// Encodes already-scaled frames (all the same size) as an animated WebP.
pub fn encode(
    frames: &[FrameImage],
    looping: GifLoop,
    out: &mut dyn Write,
    canceled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(usize),
) -> Result<(), ExportError> {
    let first = frames.first().ok_or(ExportError::NoFrames)?;
    let (w, h) = first.image.dimensions();
    if w == 0 || h == 0 || w > WEBP_MAX_SIDE || h > WEBP_MAX_SIDE {
        return Err(ExportError::Invalid(format!(
            "animated WebP frames can be at most {WEBP_MAX_SIDE}×{WEBP_MAX_SIDE} px"
        )));
    }
    if frames.iter().any(|f| f.image.dimensions() != (w, h)) {
        return Err(ExportError::Invalid(
            "all animation frames must have the same size".into(),
        ));
    }
    let mut body = Vec::new();
    // VP8X: animation + alpha flags, canvas size minus one (24 bits each).
    let mut vp8x = vec![0x02 | 0x10, 0, 0, 0];
    u24(&mut vp8x, w - 1);
    u24(&mut vp8x, h - 1);
    chunk(&mut body, b"VP8X", &vp8x)?;
    // ANIM: transparent background (BGRA), loop count (0 = forever).
    let loops: u16 = match looping {
        GifLoop::Forever => 0,
        GifLoop::Once => 1,
    };
    let mut anim = vec![0, 0, 0, 0];
    anim.extend_from_slice(&loops.to_le_bytes());
    chunk(&mut body, b"ANIM", &anim)?;
    for (i, frame) in frames.iter().enumerate() {
        if canceled() {
            return Err(ExportError::Canceled);
        }
        let mut still_file = Vec::new();
        still::encode_webp(&frame.image, &mut still_file)?;
        let image = image_chunks(&still_file)?;
        let mut anmf = Vec::with_capacity(16 + image.len());
        u24(&mut anmf, 0); // x / 2
        u24(&mut anmf, 0); // y / 2
        u24(&mut anmf, w - 1);
        u24(&mut anmf, h - 1);
        u24(&mut anmf, frame.duration_ms.clamp(1, MAX_DURATION_MS));
        // Reserved bits, blending = do not blend (bit 1), disposal = none (bit 0).
        anmf.push(0b10);
        anmf.extend_from_slice(&image);
        chunk(&mut body, b"ANMF", &anmf)?;
        progress(i + 1);
    }
    let size = u32::try_from(body.len() + 4)
        .map_err(|_| ExportError::Invalid("the animation is too large for WebP (4 GiB)".into()))?;
    out.write_all(b"RIFF")?;
    out.write_all(&size.to_le_bytes())?;
    out.write_all(b"WEBP")?;
    out.write_all(&body)?;
    Ok(())
}

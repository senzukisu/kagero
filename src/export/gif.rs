//! Animated GIF encoding.
//!
//! GIF has 1-bit transparency and at most 256 colors per palette, so Kagero:
//!
//! * turns each pixel either fully transparent (alpha below the threshold) or fully
//!   opaque — opaque pixels keep their own color, or are blended over a matte color if
//!   one is set, so there are no unexpected dark or light halos;
//! * builds **one palette for the whole animation** (colors don't flicker between
//!   frames). Index 0 is the transparent color. If the animation uses at most 255
//!   colors they are kept exactly; otherwise NeuQuant reduces them to 255, optionally
//!   with Floyd–Steinberg dithering (off by default — it adds noise to pixel art);
//! * writes delays in GIF's 10 ms units, rounded to the nearest unit with a minimum of
//!   20 ms, because browsers slow down shorter delays to 100 ms.

use std::collections::HashMap;
use std::io::Write;

use gif::{DisposalMethod, Encoder, Frame, Repeat};

use super::render::FrameImage;
use super::{ExportError, GifLoop, GifOptions};
use crate::core::Rgba;

/// Shortest delay written, in 10 ms units (browsers treat less as 100 ms).
pub const MIN_DELAY_CS: u16 = 2;
/// GIF dimensions are 16-bit.
pub const GIF_MAX_SIDE: u32 = 65_535;
/// At most this many pixels are sampled to train the color quantizer.
const QUANT_SAMPLE_PIXELS: usize = 1 << 20;

/// Milliseconds → GIF delay in centiseconds (nearest, at least [`MIN_DELAY_CS`]).
pub fn delay_cs(ms: u32) -> u16 {
    let cs = (u64::from(ms) + 5) / 10;
    cs.clamp(u64::from(MIN_DELAY_CS), u64::from(u16::MAX)) as u16
}

/// What the GIF encoder did, for the user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GifReport {
    /// Distinct opaque colors before encoding.
    pub source_colors: usize,
    /// True if colors had to be reduced to 255.
    pub reduced: bool,
    /// True if some pixels were semi-transparent (soft alpha is lost).
    pub soft_alpha: bool,
    /// True if any pixel is transparent in the result.
    pub transparent: bool,
}

/// Makes a pixel GIF-ready: `None` = transparent, otherwise an opaque RGB color.
fn resolve(px: [u8; 4], options: &GifOptions) -> Option<[u8; 3]> {
    if px[3] < options.alpha_threshold.max(1) {
        return None;
    }
    if px[3] == 255 {
        return Some([px[0], px[1], px[2]]);
    }
    match options.matte {
        Some(matte) => {
            let c = Rgba(px).over(Rgba::opaque(matte.r(), matte.g(), matte.b()));
            Some([c.r(), c.g(), c.b()])
        }
        None => Some([px[0], px[1], px[2]]),
    }
}

/// Analyzes frames before encoding (also used by the dialog for warnings).
pub fn analyze(frames: &[FrameImage], options: &GifOptions) -> GifReport {
    let mut colors: HashMap<[u8; 3], ()> = HashMap::new();
    let mut report = GifReport::default();
    for f in frames {
        for px in f.image.pixels() {
            let a = px.0[3];
            if a != 0 && a != 255 {
                report.soft_alpha = true;
            }
            match resolve(px.0, options) {
                None => report.transparent = true,
                Some(c) => {
                    if colors.len() <= 256 {
                        colors.insert(c, ());
                    }
                }
            }
        }
    }
    report.source_colors = colors.len();
    report.reduced = colors.len() > 255;
    report
}

/// The palette shared by all frames.
enum Palette {
    Exact(HashMap<[u8; 3], u8>, Vec<u8>),
    Quantized(color_quant::NeuQuant, Vec<u8>),
}

impl Palette {
    fn rgb_table(&self) -> &[u8] {
        match self {
            Palette::Exact(_, t) | Palette::Quantized(_, t) => t,
        }
    }

    /// Palette index (1..=255) of an opaque color.
    fn index(&self, c: [u8; 3]) -> u8 {
        match self {
            Palette::Exact(map, _) => map.get(&c).copied().unwrap_or(1),
            Palette::Quantized(nq, _) => {
                (nq.index_of(&[c[0], c[1], c[2], 255]) as u8).saturating_add(1)
            }
        }
    }

    fn color(&self, index: u8) -> [i32; 3] {
        let t = self.rgb_table();
        let i = usize::from(index) * 3;
        [i32::from(t[i]), i32::from(t[i + 1]), i32::from(t[i + 2])]
    }
}

fn build_palette(frames: &[FrameImage], options: &GifOptions) -> Palette {
    let mut map: HashMap<[u8; 3], u8> = HashMap::new();
    let mut table = vec![0u8; 3]; // index 0: transparent
    let mut overflow = false;
    'outer: for f in frames {
        for px in f.image.pixels() {
            if let Some(c) = resolve(px.0, options) {
                if !map.contains_key(&c) {
                    if map.len() == 255 {
                        overflow = true;
                        break 'outer;
                    }
                    map.insert(c, map.len() as u8 + 1);
                    table.extend_from_slice(&c);
                }
            }
        }
    }
    if !overflow {
        table.resize(256 * 3, 0);
        return Palette::Exact(map, table);
    }
    // Train NeuQuant on a bounded sample of opaque pixels from all frames.
    let total: usize = frames.iter().map(|f| f.image.pixels().len()).sum();
    let step = (total / QUANT_SAMPLE_PIXELS).max(1);
    let mut sample = Vec::with_capacity((total / step).min(QUANT_SAMPLE_PIXELS) * 4);
    for (i, px) in frames.iter().flat_map(|f| f.image.pixels()).enumerate() {
        if i % step == 0 {
            if let Some(c) = resolve(px.0, options) {
                sample.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }
    let nq = color_quant::NeuQuant::new(10, 255, &sample);
    let mut table = vec![0u8; 3];
    table.extend_from_slice(&nq.color_map_rgb());
    table.resize(256 * 3, 0);
    Palette::Quantized(nq, table)
}

/// Maps one frame to palette indices (0 = transparent).
fn index_frame(f: &FrameImage, palette: &Palette, options: &GifOptions) -> Vec<u8> {
    let (w, h) = (f.image.width() as usize, f.image.height() as usize);
    let mut out = vec![0u8; w * h];
    let dither = options.dither && matches!(palette, Palette::Quantized(..));
    // Error buffers for Floyd–Steinberg (current and next row).
    let mut err = vec![[0i32; 3]; (w + 2) * 2];
    for y in 0..h {
        let (cur, next) = if y % 2 == 0 { (0, w + 2) } else { (w + 2, 0) };
        if dither {
            for e in &mut err[next..next + w + 2] {
                *e = [0; 3];
            }
        }
        for x in 0..w {
            let px = f.image.get_pixel(x as u32, y as u32).0;
            let Some(c) = resolve(px, options) else {
                continue;
            };
            let idx = if dither {
                let e = err[cur + x + 1];
                let adj = [
                    (i32::from(c[0]) + e[0] / 16).clamp(0, 255) as u8,
                    (i32::from(c[1]) + e[1] / 16).clamp(0, 255) as u8,
                    (i32::from(c[2]) + e[2] / 16).clamp(0, 255) as u8,
                ];
                let idx = palette.index(adj);
                let got = palette.color(idx);
                let diff = [
                    i32::from(adj[0]) - got[0],
                    i32::from(adj[1]) - got[1],
                    i32::from(adj[2]) - got[2],
                ];
                for (k, weight) in [
                    (cur + x + 2, 7),
                    (next + x, 3),
                    (next + x + 1, 5),
                    (next + x + 2, 1),
                ] {
                    for ch in 0..3 {
                        err[k][ch] += diff[ch] * weight;
                    }
                }
                idx
            } else {
                palette.index(c)
            };
            out[y * w + x] = idx;
        }
    }
    out
}

/// Encodes frames (all the same size) as an animated GIF.
pub fn encode(
    frames: &[FrameImage],
    options: &GifOptions,
    out: &mut dyn Write,
    cancel: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(usize),
) -> Result<GifReport, ExportError> {
    let first = frames.first().ok_or(ExportError::NoFrames)?;
    let (w, h) = first.image.dimensions();
    if w > GIF_MAX_SIDE || h > GIF_MAX_SIDE {
        return Err(ExportError::Invalid(format!(
            "GIF images can be at most {GIF_MAX_SIDE} px wide and tall"
        )));
    }
    if frames.iter().any(|f| f.image.dimensions() != (w, h)) {
        return Err(ExportError::Invalid(
            "all GIF frames must have the same size".into(),
        ));
    }
    let report = analyze(frames, options);
    let palette = build_palette(frames, options);
    let mut encoder = Encoder::new(out, w as u16, h as u16, palette.rgb_table())
        .map_err(|e| ExportError::encode("GIF", e))?;
    if options.looping == GifLoop::Forever {
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| ExportError::encode("GIF", e))?;
    }
    for (i, f) in frames.iter().enumerate() {
        if cancel() {
            return Err(ExportError::Canceled);
        }
        let indices = index_frame(f, &palette, options);
        let frame = Frame {
            width: w as u16,
            height: h as u16,
            delay: delay_cs(f.duration_ms),
            // Clear to transparent before the next frame so transparent areas stay clear.
            dispose: DisposalMethod::Background,
            transparent: Some(0),
            buffer: std::borrow::Cow::Owned(indices),
            ..Frame::default()
        };
        encoder
            .write_frame(&frame)
            .map_err(|e| ExportError::encode("GIF", e))?;
        progress(i + 1);
    }
    drop(encoder);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_round_to_centiseconds_with_a_minimum() {
        assert_eq!(delay_cs(100), 10);
        assert_eq!(delay_cs(104), 10);
        assert_eq!(delay_cs(105), 11);
        assert_eq!(delay_cs(10), MIN_DELAY_CS);
        assert_eq!(delay_cs(0), MIN_DELAY_CS);
        assert_eq!(delay_cs(60_000), 6000);
    }

    #[test]
    fn alpha_threshold_and_matte() {
        let o = GifOptions::default();
        assert_eq!(resolve([1, 2, 3, 0], &o), None);
        assert_eq!(resolve([1, 2, 3, 127], &o), None);
        assert_eq!(
            resolve([10, 20, 30, 128], &o),
            Some([10, 20, 30]),
            "keeps own color"
        );
        let m = GifOptions {
            matte: Some(Rgba::WHITE),
            ..o
        };
        let c = resolve([0, 0, 0, 128], &m).expect("visible");
        assert!(c[0] > 100, "blended with the white matte: {c:?}");
    }
}

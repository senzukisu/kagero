//! Animation frames: each frame owns one image ("cel") per document layer.

use super::PixelBuffer;

/// Stable identifier of a frame inside one document (survives reordering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameId(pub u64);

/// Shortest allowed frame duration (100 fps).
pub const MIN_FRAME_DURATION_MS: u32 = 10;
/// Longest allowed frame duration.
pub const MAX_FRAME_DURATION_MS: u32 = 60_000;
/// Duration of new frames when there is nothing to copy it from (10 fps).
pub const DEFAULT_FRAME_DURATION_MS: u32 = 100;

/// Clamps a duration into the allowed range.
pub fn clamp_duration(ms: u32) -> u32 {
    ms.clamp(MIN_FRAME_DURATION_MS, MAX_FRAME_DURATION_MS)
}

/// One animation frame.
///
/// `cels[i]` is the image of document layer `i` in this frame, so `cels` always has
/// exactly as many entries as the document has layers, in the same order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub id: FrameId,
    duration_ms: u32,
    pub(crate) cels: Vec<PixelBuffer>,
    /// Bumped whenever a cel of this frame may have changed (for thumbnail caches).
    revision: u64,
}

impl Frame {
    pub(crate) fn new(id: FrameId, duration_ms: u32, cels: Vec<PixelBuffer>) -> Self {
        Self {
            id,
            duration_ms: clamp_duration(duration_ms),
            cels,
            revision: 0,
        }
    }

    pub fn duration_ms(&self) -> u32 {
        self.duration_ms
    }

    /// Sets the duration, clamped to the allowed range.
    pub fn set_duration_ms(&mut self, ms: u32) {
        self.duration_ms = clamp_duration(ms);
    }

    /// Layer images bottom to top.
    pub fn cels(&self) -> &[PixelBuffer] {
        &self.cels
    }

    pub fn cel(&self, layer_index: usize) -> Option<&PixelBuffer> {
        self.cels.get(layer_index)
    }

    pub(crate) fn cel_mut(&mut self, layer_index: usize) -> Option<&mut PixelBuffer> {
        self.revision = self.revision.wrapping_add(1);
        self.cels.get_mut(layer_index)
    }

    /// Changes whenever this frame's pixels may have changed.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Approximate memory used by the pixel data.
    pub fn byte_size(&self) -> usize {
        self.cels.iter().map(|c| c.as_bytes().len()).sum()
    }
}

/// Index of the frame shown at `elapsed_ms` into a looping animation, and how many
/// milliseconds remain until the next frame. Durations must be non-empty.
pub fn frame_at_time(durations: &[u32], elapsed_ms: u64) -> Option<(usize, u64)> {
    let total: u64 = durations
        .iter()
        .map(|&d| u64::from(clamp_duration(d)))
        .sum();
    if total == 0 {
        return None;
    }
    let mut t = elapsed_ms % total;
    for (i, &d) in durations.iter().enumerate() {
        let d = u64::from(clamp_duration(d));
        if t < d {
            return Some((i, d - t));
        }
        t -= d;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_clamped() {
        assert_eq!(clamp_duration(0), MIN_FRAME_DURATION_MS);
        assert_eq!(clamp_duration(u32::MAX), MAX_FRAME_DURATION_MS);
        assert_eq!(clamp_duration(120), 120);
    }

    #[test]
    fn frame_at_time_loops() {
        let d = [100, 50, 200];
        assert_eq!(frame_at_time(&d, 0), Some((0, 100)));
        assert_eq!(frame_at_time(&d, 99), Some((0, 1)));
        assert_eq!(frame_at_time(&d, 100), Some((1, 50)));
        assert_eq!(frame_at_time(&d, 150), Some((2, 200)));
        assert_eq!(frame_at_time(&d, 349), Some((2, 1)));
        // Wraps around after the last frame.
        assert_eq!(frame_at_time(&d, 350), Some((0, 100)));
        assert_eq!(frame_at_time(&d, 350 * 1000 + 120), Some((1, 30)));
        assert_eq!(frame_at_time(&[], 10), None);
    }
}

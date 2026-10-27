//! Onion skinning: which neighbor frames to show and how.
//!
//! This is a view aid only. Nothing here touches document pixels; the canvas draws the
//! returned frames as tinted, translucent overlays and exports never include them.

use serde::{Deserialize, Serialize};

/// User settings for onion skinning (persisted in the app settings, not the document).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OnionSkin {
    pub enabled: bool,
    pub show_previous: bool,
    pub show_next: bool,
    /// Overlay opacity, 0.05–0.9.
    pub opacity: f32,
}

impl Default for OnionSkin {
    fn default() -> Self {
        Self {
            enabled: false,
            show_previous: true,
            show_next: true,
            opacity: 0.35,
        }
    }
}

pub const MIN_OPACITY: f32 = 0.05;
pub const MAX_OPACITY: f32 = 0.9;

/// Whether an overlay comes before or after the active frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnionSide {
    Previous,
    Next,
}

impl OnionSide {
    /// RGB tint: warm ember for the past, cool teal for the future.
    pub fn tint(self) -> [u8; 3] {
        match self {
            OnionSide::Previous => [0xFF, 0x7A, 0x5C],
            OnionSide::Next => [0x6F, 0xD6, 0xC2],
        }
    }
}

/// One overlay to draw: frame index, side and opacity (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnionOverlay {
    pub frame: usize,
    pub side: OnionSide,
    pub opacity: f32,
}

impl OnionSkin {
    pub fn clamped_opacity(&self) -> f32 {
        if self.opacity.is_nan() {
            Self::default().opacity
        } else {
            self.opacity.clamp(MIN_OPACITY, MAX_OPACITY)
        }
    }

    /// Overlays for `active` among `frame_count` frames. Does not wrap around: the first
    /// frame has no previous overlay and the last frame has no next overlay.
    pub fn overlays(&self, frame_count: usize, active: usize) -> Vec<OnionOverlay> {
        let mut out = Vec::new();
        if !self.enabled || frame_count < 2 || active >= frame_count {
            return out;
        }
        let opacity = self.clamped_opacity();
        if self.show_previous && active > 0 {
            out.push(OnionOverlay {
                frame: active - 1,
                side: OnionSide::Previous,
                opacity,
            });
        }
        if self.show_next && active + 1 < frame_count {
            out.push(OnionOverlay {
                frame: active + 1,
                side: OnionSide::Next,
                opacity,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> OnionSkin {
        OnionSkin {
            enabled: true,
            ..OnionSkin::default()
        }
    }

    #[test]
    fn middle_frame_has_both_neighbors() {
        let o = on().overlays(5, 2);
        assert_eq!(o.len(), 2);
        assert_eq!((o[0].frame, o[0].side), (1, OnionSide::Previous));
        assert_eq!((o[1].frame, o[1].side), (3, OnionSide::Next));
    }

    #[test]
    fn edges_do_not_wrap() {
        let first = on().overlays(3, 0);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].side, OnionSide::Next);
        let last = on().overlays(3, 2);
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].side, OnionSide::Previous);
        assert!(on().overlays(1, 0).is_empty());
        assert!(on().overlays(3, 7).is_empty());
    }

    #[test]
    fn disabled_or_sides_off() {
        assert!(OnionSkin::default().overlays(5, 2).is_empty());
        let only_prev = OnionSkin {
            show_next: false,
            ..on()
        };
        assert_eq!(only_prev.overlays(5, 2).len(), 1);
    }

    #[test]
    fn opacity_is_clamped() {
        let o = OnionSkin {
            opacity: 5.0,
            ..on()
        };
        assert_eq!(o.overlays(3, 1)[0].opacity, MAX_OPACITY);
        let o = OnionSkin {
            opacity: f32::NAN,
            ..on()
        };
        assert_eq!(o.clamped_opacity(), 0.35);
        let o = OnionSkin {
            opacity: -1.0,
            ..on()
        };
        assert_eq!(o.clamped_opacity(), MIN_OPACITY);
    }
}

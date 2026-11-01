//! Ordered dithering patterns for drawing tools.
//!
//! A pattern decides per image pixel whether the tool paints it. Patterns are tied
//! to image coordinates (not to the stroke), so separate strokes line up seamlessly.

/// How much of the covered area a tool paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dither {
    /// Every pixel.
    #[default]
    Solid,
    /// One pixel in four (a sparse grid).
    Light25,
    /// Every other pixel: a checkerboard (50%).
    Checker,
    /// Three pixels in four.
    Dense75,
}

impl Dither {
    pub const ALL: [Dither; 4] = [
        Dither::Solid,
        Dither::Light25,
        Dither::Checker,
        Dither::Dense75,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Dither::Solid => "Solid",
            Dither::Light25 => "25%",
            Dither::Checker => "Checker 50%",
            Dither::Dense75 => "75%",
        }
    }

    /// Whether the tool paints image pixel `(x, y)`.
    pub fn paints(self, x: i32, y: i32) -> bool {
        // 2×2 Bayer matrix: 0 2 / 3 1.
        const BAYER: [[u8; 2]; 2] = [[0, 2], [3, 1]];
        let level = BAYER[y.rem_euclid(2) as usize][x.rem_euclid(2) as usize];
        match self {
            Dither::Solid => true,
            Dither::Light25 => level < 1,
            Dither::Checker => level < 2,
            Dither::Dense75 => level < 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(d: Dither) -> usize {
        (0..8)
            .flat_map(|y| (0..8).map(move |x| (x, y)))
            .filter(|&(x, y)| d.paints(x, y))
            .count()
    }

    #[test]
    fn patterns_cover_their_share_and_tile() {
        assert_eq!(coverage(Dither::Solid), 64);
        assert_eq!(coverage(Dither::Light25), 16);
        assert_eq!(coverage(Dither::Checker), 32);
        assert_eq!(coverage(Dither::Dense75), 48);
        // A checkerboard: neighbors differ, diagonals match, negative coords work.
        assert_ne!(Dither::Checker.paints(0, 0), Dither::Checker.paints(1, 0));
        assert_eq!(Dither::Checker.paints(-1, -1), Dither::Checker.paints(1, 1));
        // Denser patterns include the lighter ones' pixels.
        for y in -3..3 {
            for x in -3..3 {
                if Dither::Light25.paints(x, y) {
                    assert!(Dither::Checker.paints(x, y));
                }
                if Dither::Checker.paints(x, y) {
                    assert!(Dither::Dense75.paints(x, y));
                }
            }
        }
    }
}

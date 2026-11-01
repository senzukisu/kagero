use serde::{Deserialize, Serialize};

/// A straight (non-premultiplied) 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba([0, 0, 0, 0]);
    pub const BLACK: Rgba = Rgba([0, 0, 0, 255]);
    pub const WHITE: Rgba = Rgba([255, 255, 255, 255]);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba([r, g, b, a])
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Rgba([r, g, b, 255])
    }

    pub const fn r(self) -> u8 {
        self.0[0]
    }
    pub const fn g(self) -> u8 {
        self.0[1]
    }
    pub const fn b(self) -> u8 {
        self.0[2]
    }
    pub const fn a(self) -> u8 {
        self.0[3]
    }

    /// Composites `self` over `dst` with the standard "source over" operator.
    ///
    /// Integer math keeps results deterministic across platforms.
    pub fn over(self, dst: Rgba) -> Rgba {
        let sa = u32::from(self.a());
        if sa == 255 {
            return self;
        }
        if sa == 0 {
            return dst;
        }
        let da = u32::from(dst.a());
        // Everything below is scaled by 255 * 255.
        let dst_weight = da * (255 - sa);
        let out_a = sa * 255 + dst_weight;
        if out_a == 0 {
            return Rgba::TRANSPARENT;
        }
        let mix = |s: u8, d: u8| -> u8 {
            let num = u32::from(s) * sa * 255 + u32::from(d) * dst_weight;
            ((num + out_a / 2) / out_a) as u8
        };
        Rgba([
            mix(self.r(), dst.r()),
            mix(self.g(), dst.g()),
            mix(self.b(), dst.b()),
            ((out_a + 127) / 255) as u8,
        ])
    }

    /// Parses `#RGB`, `#RRGGBB` or `#RRGGBBAA` (the `#` is optional, case ignored).
    pub fn from_hex(text: &str) -> Option<Rgba> {
        let hex = text.trim();
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        match hex.len() {
            3 => {
                let nib = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|v| v * 17);
                Some(Rgba::opaque(nib(0)?, nib(1)?, nib(2)?))
            }
            6 => Some(Rgba::opaque(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Rgba::new(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// `#RRGGBBAA` (or `#RRGGBB` for opaque colors).
    pub fn to_hex(self) -> String {
        if self.a() == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r(), self.g(), self.b())
        } else {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r(),
                self.g(),
                self.b(),
                self.a()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_round_trip() {
        use super::Rgba;
        for c in [Rgba::opaque(1, 2, 3), Rgba::new(255, 128, 0, 77)] {
            assert_eq!(Rgba::from_hex(&c.to_hex()), Some(c));
        }
        assert_eq!(Rgba::from_hex(" f80 "), Some(Rgba::opaque(0xFF, 0x88, 0)));
        assert_eq!(
            Rgba::from_hex("#e86a33"),
            Some(Rgba::opaque(0xE8, 0x6A, 0x33))
        );
        for bad in ["", "#12", "#12345", "zzzzzz", "#ééé", "+12345"] {
            assert_eq!(Rgba::from_hex(bad), None, "{bad}");
        }
    }
    use super::*;

    #[test]
    fn over_extremes() {
        let red = Rgba::opaque(255, 0, 0);
        assert_eq!(red.over(Rgba::WHITE), red);
        assert_eq!(Rgba::TRANSPARENT.over(red), red);
        assert_eq!(Rgba::TRANSPARENT.over(Rgba::TRANSPARENT), Rgba::TRANSPARENT);
    }

    #[test]
    fn over_half_alpha() {
        let half_black = Rgba::new(0, 0, 0, 128);
        let out = half_black.over(Rgba::WHITE);
        assert_eq!(out.a(), 255);
        assert!((126..=128).contains(&out.r()), "got {out:?}");
        // Over a transparent background the color keeps its own value.
        let out = Rgba::new(200, 100, 50, 128).over(Rgba::TRANSPARENT);
        assert_eq!(out, Rgba::new(200, 100, 50, 128));
    }

    #[test]
    fn hex() {
        assert_eq!(Rgba::opaque(255, 157, 66).to_hex(), "#FF9D42");
        assert_eq!(Rgba::new(1, 2, 3, 4).to_hex(), "#01020304");
    }
}

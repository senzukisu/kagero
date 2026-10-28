//! Named color palettes, including Kagero's built-in sets.
//!
//! All built-in palettes are original work by Onibi Lab (MIT, like the rest of Kagero);
//! none is copied from another editor or palette collection.

use serde::{Deserialize, Serialize};

use super::Rgba;

/// Most colors a palette can hold.
pub const MAX_PALETTE_COLORS: usize = 256;
/// Longest palette name.
pub const MAX_PALETTE_NAME_CHARS: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<Rgba>,
}

fn from_hex(name: &str, hex: &[u32]) -> Palette {
    Palette {
        name: name.to_owned(),
        colors: hex
            .iter()
            .map(|&v| Rgba::opaque((v >> 16) as u8, (v >> 8) as u8, v as u8))
            .collect(),
    }
}

/// Kagero Ember: neutrals, ember warms, moss greens and night blues (the 0.1 default).
pub fn kagero_ember() -> Palette {
    from_hex(
        "Kagero Ember",
        &[
            0x0E0F13, 0x17191F, 0x2C303A, 0x4A4F5C, 0x767B88, 0xA5A9B5, 0xD6D3CC, 0xF0EEE8,
            0x3B1F1A, 0x6E2C1F, 0xA53A26, 0xF06B35, 0xFF9D42, 0xFFC56E, 0xFFE7A8, 0xC88B5A,
            0x1E3A2F, 0x2F5D45, 0x4F8A5B, 0x75C7B5, 0xA8E0C4, 0x5B6B2E, 0x93A541, 0xD3D873,
            0x141E3C, 0x223A6B, 0x3767A8, 0x5FA0E0, 0x9CD2F7, 0x4B2C5E, 0x8A4F9E, 0xE07AB4,
        ],
    )
}

/// Ember: a fire ramp from soot to white-hot, with warm earth tones for shading.
pub fn ember() -> Palette {
    from_hex(
        "Ember",
        &[
            0x1A0F0E, 0x3B1A14, 0x5E2418, 0x8A3219, 0xB8481C, 0xE0641E, 0xF38B2C, 0xFFB347,
            0xFFD27A, 0xFFF0C2, 0x4A2C2A, 0x7A4A3A, 0xA86F55, 0xD49A78, 0xF2C9A8, 0xFFFAF2,
        ],
    )
}

/// Night Sakura: deep night blues with cherry-blossom pinks and a moonlit teal.
pub fn night_sakura() -> Palette {
    from_hex(
        "Night Sakura",
        &[
            0x0D0B1E, 0x1B1838, 0x2A2A5A, 0x3D4285, 0x5B6BB3, 0x8FA3D9, 0xC9D6F2, 0xF4F1FA,
            0x3A1F3D, 0x6B2F5C, 0xA8457A, 0xE06A9C, 0xF49AC1, 0xFCC8DC, 0xFFE9F1, 0x7FD1C7,
        ],
    )
}

/// Misty Forest: layered greens, fog greys and bark browns.
pub fn misty_forest() -> Palette {
    from_hex(
        "Misty Forest",
        &[
            0x0E1712, 0x1B2A20, 0x2C4231, 0x3F5C40, 0x587A55, 0x7A9A6E, 0xA3BC8F, 0xCFDDB8,
            0xEEF1E3, 0x2D3436, 0x4F5B5E, 0x7C8A8C, 0xAEB9B8, 0xD8DEDB, 0x8C6E4E, 0xC8A57A,
        ],
    )
}

/// All built-in palettes, in display order.
pub fn builtins() -> Vec<Palette> {
    vec![kagero_ember(), ember(), night_sakura(), misty_forest()]
}

/// Names that belong to built-in palettes.
pub fn is_builtin_name(name: &str) -> bool {
    builtins().iter().any(|p| p.name == name)
}

/// Cleans a palette name: trims, removes control characters, limits the length.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_PALETTE_NAME_CHARS)
        .collect();
    if cleaned.is_empty() {
        "Palette".to_owned()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_real_and_distinct() {
        let all = builtins();
        assert_eq!(all.len(), 4);
        for p in &all {
            assert!(p.colors.len() >= 16, "{} has colors", p.name);
            let mut unique = p.colors.clone();
            unique.sort_by_key(|c| c.0);
            unique.dedup();
            assert_eq!(unique.len(), p.colors.len(), "{} has no duplicates", p.name);
            assert!(p.colors.iter().all(|c| c.a() == 255));
        }
        assert!(is_builtin_name("Night Sakura"));
        assert_eq!(sanitize_name("  \u{7}  "), "Palette");
    }
}

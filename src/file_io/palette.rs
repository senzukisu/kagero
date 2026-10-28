//! Palette files.
//!
//! * **GIMP palette** (`.gpl`): first line `GIMP Palette`, optional `Name:` and `Columns:`
//!   headers, `#` comments, then one color per line as three decimal numbers (0–255)
//!   optionally followed by a name.
//! * **HEX** (`.hex`, `.txt`): one color per line as `RRGGBB` (a leading `#` is allowed);
//!   `AARRGGBB` lines (Paint.NET style) are read with their alpha. Lines starting with
//!   `;` or `//` are comments.
//!
//! Files are limited to 1 MiB and 256 colors.

use std::fmt::Write as _;
use std::path::Path;

use super::{check_file_size, write_atomic, FileError};
use crate::core::palette::{sanitize_name, Palette, MAX_PALETTE_COLORS};
use crate::core::Rgba;

const MAX_PALETTE_FILE_BYTES: u64 = 1024 * 1024;

/// Parses a GIMP `.gpl` palette.
pub fn parse_gpl(text: &str, fallback_name: &str) -> Result<Palette, FileError> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .unwrap_or("")
        .trim_start_matches('\u{feff}')
        .trim();
    if header != "GIMP Palette" {
        return Err(FileError::Invalid(
            "not a GIMP palette (missing \"GIMP Palette\" header)".into(),
        ));
    }
    let mut name = fallback_name.to_owned();
    let mut colors = Vec::new();
    for (n, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(v) = line.strip_prefix("Name:") {
            name = v.trim().to_owned();
            continue;
        }
        if line.starts_with("Columns:") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let mut channel = || -> Result<u8, FileError> {
            parts
                .next()
                .and_then(|v| v.parse::<u8>().ok())
                .ok_or_else(|| {
                    FileError::Invalid(format!("line {}: expected \"R G B\" (0–255)", n + 2))
                })
        };
        let (r, g, b) = (channel()?, channel()?, channel()?);
        colors.push(Rgba::opaque(r, g, b));
        if colors.len() > MAX_PALETTE_COLORS {
            return Err(FileError::TooLarge(format!(
                "more than {MAX_PALETTE_COLORS} colors"
            )));
        }
    }
    finish(name, colors)
}

/// Parses a HEX palette (one `RRGGBB` or `AARRGGBB` per line).
pub fn parse_hex(text: &str, name: &str) -> Result<Palette, FileError> {
    let mut colors = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
            continue;
        }
        let hex = line.trim_start_matches('#');
        let bad = || FileError::Invalid(format!("line {}: \"{line}\" is not a hex color", n + 1));
        let value = u32::from_str_radix(hex, 16).map_err(|_| bad())?;
        let color = match hex.len() {
            6 => Rgba::opaque((value >> 16) as u8, (value >> 8) as u8, value as u8),
            8 => Rgba::new(
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
                (value >> 24) as u8,
            ),
            _ => return Err(bad()),
        };
        colors.push(color);
        if colors.len() > MAX_PALETTE_COLORS {
            return Err(FileError::TooLarge(format!(
                "more than {MAX_PALETTE_COLORS} colors"
            )));
        }
    }
    finish(name.to_owned(), colors)
}

fn finish(name: String, colors: Vec<Rgba>) -> Result<Palette, FileError> {
    if colors.is_empty() {
        return Err(FileError::Invalid("the palette has no colors".into()));
    }
    Ok(Palette {
        name: sanitize_name(&name),
        colors,
    })
}

/// Writes a GIMP palette (alpha is not part of the format and is dropped).
pub fn to_gpl(palette: &Palette) -> String {
    let mut out = format!("GIMP Palette\nName: {}\nColumns: 8\n#\n", palette.name);
    for c in &palette.colors {
        let _ = writeln!(out, "{:3} {:3} {:3}\t{}", c.r(), c.g(), c.b(), c.to_hex());
    }
    out
}

/// Writes a HEX palette (`RRGGBB`, or `AARRGGBB` for translucent colors).
pub fn to_hex(palette: &Palette) -> String {
    let mut out = String::new();
    for c in &palette.colors {
        if c.a() == 255 {
            let _ = writeln!(out, "{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
        } else {
            let _ = writeln!(out, "{:02x}{:02x}{:02x}{:02x}", c.a(), c.r(), c.g(), c.b());
        }
    }
    out
}

/// Loads a palette file; the format is chosen by extension (`.gpl`, otherwise HEX).
pub fn load_palette(path: &Path) -> Result<Palette, FileError> {
    check_file_size(path, MAX_PALETTE_FILE_BYTES)?;
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| FileError::Invalid("the palette file is not UTF-8 text".into()))?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let is_gpl = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gpl"));
    if is_gpl {
        parse_gpl(&text, &stem)
    } else {
        parse_hex(&text, &stem)
    }
}

/// Saves a palette; `.gpl` writes a GIMP palette, anything else HEX.
pub fn save_palette(palette: &Palette, path: &Path) -> Result<(), FileError> {
    let is_gpl = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gpl"));
    let text = if is_gpl {
        to_gpl(palette)
    } else {
        to_hex(palette)
    };
    write_atomic(path, |w| {
        std::io::Write::write_all(w, text.as_bytes())?;
        Ok(())
    })
}

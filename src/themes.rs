//! Color themes: the built-in ones and the user's, stored as versioned JSON files.
//!
//! A theme file looks like this (every color is required; see
//! `docs/reference/themes.md`):
//!
//! ```json
//! {
//!   "format": "kagero-theme",
//!   "version": 1,
//!   "name": "Night Sakura",
//!   "base": "dark",
//!   "colors": { "background": "#16121C", "panel": "#1F1A27", ... }
//! }
//! ```
//!
//! Themes are checked before they are used: unknown versions, missing or unreadable
//! colors and unreadable text (too little contrast) are refused with a clear message,
//! so a broken file can never make the interface unusable.

use std::path::{Path, PathBuf};

use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::core::Rgba;
use crate::theme::{ThemeColors, EMBER};

pub const THEME_FORMAT: &str = "kagero-theme";
pub const THEME_VERSION: u32 = 1;
/// Longest theme name, in characters.
pub const MAX_THEME_NAME_CHARS: usize = 48;
/// Largest theme file read.
const MAX_THEME_FILE_BYTES: u64 = 64 * 1024;
/// Most theme files loaded from the themes folder.
const MAX_THEME_FILES: usize = 200;
/// The default theme.
pub const DEFAULT_THEME: &str = "Kagero Ember";

/// Color keys of a theme file, in the order of [`ThemeColors`].
pub const COLOR_KEYS: [&str; 17] = [
    "background",
    "panel",
    "surface",
    "surface_hover",
    "border",
    "bevel_light",
    "bevel_dark",
    "text_secondary",
    "text_primary",
    "accent",
    "accent_pressed",
    "success",
    "accent_deep",
    "error",
    "on_accent",
    "checker_light",
    "checker_dark",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    /// Where a user theme is stored (`None` for built-in themes).
    pub file: Option<PathBuf>,
}

impl Theme {
    pub fn is_builtin(&self) -> bool {
        self.file.is_none()
    }
}

fn colors_to_array(c: &ThemeColors) -> [Color32; 17] {
    [
        c.background,
        c.panel,
        c.surface,
        c.surface_hover,
        c.border,
        c.bevel_light,
        c.bevel_dark,
        c.text_secondary,
        c.text_primary,
        c.accent,
        c.accent_pressed,
        c.success,
        c.ember_deep,
        c.error,
        c.on_accent,
        c.checker_light,
        c.checker_dark,
    ]
}

fn colors_from_array(dark: bool, a: [Color32; 17]) -> ThemeColors {
    ThemeColors {
        dark,
        background: a[0],
        panel: a[1],
        surface: a[2],
        surface_hover: a[3],
        border: a[4],
        bevel_light: a[5],
        bevel_dark: a[6],
        text_secondary: a[7],
        text_primary: a[8],
        accent: a[9],
        accent_pressed: a[10],
        success: a[11],
        ember_deep: a[12],
        error: a[13],
        on_accent: a[14],
        checker_light: a[15],
        checker_dark: a[16],
    }
}

fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn built(name: &str, dark: bool, hex: [u32; 17]) -> Theme {
    Theme {
        name: name.to_owned(),
        colors: colors_from_array(dark, hex.map(rgb)),
        file: None,
    }
}

/// Kagero's built-in themes (original color work by Onibi Lab).
pub fn builtins() -> Vec<Theme> {
    vec![
        Theme {
            name: DEFAULT_THEME.to_owned(),
            colors: EMBER,
            file: None,
        },
        built(
            "Night Sakura",
            true,
            [
                0x16121C, 0x1F1A27, 0x2A2333, 0x352C40, 0x342B3F, 0x4A3F58, 0x0D0A11, 0xB4A8C2,
                0xF4EEF6, 0xF28DB2, 0xD9608F, 0x8FD6C0, 0x5E2742, 0xF06A6A, 0x1C1018, 0x4A4256,
                0x3A3344,
            ],
        ),
        built(
            "Misty Forest",
            true,
            [
                0x141A18, 0x1C2421, 0x26302C, 0x303C37, 0x2F3A35, 0x45534C, 0x0B0F0D, 0xA6B5AD,
                0xEEF2EC, 0x9CCB6B, 0x73A64A, 0x7FC8B6, 0x2F4A24, 0xE36B5E, 0x121A0E, 0x46514B,
                0x37403B,
            ],
        ),
        built(
            "Paper",
            false,
            [
                0xE9E4DA, 0xF6F2EA, 0xECE6DB, 0xE0D8CA, 0xCFC6B6, 0xFFFFFF, 0x8C8374, 0x5A544B,
                0x23201B, 0xD9692B, 0xB8521C, 0x2F8F7A, 0xF6D2B8, 0xC43C3C, 0x1A1410, 0xFFFFFF,
                0xDCD6CC,
            ],
        ),
    ]
}

/// WCAG contrast ratio of two colors (1–21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn luminance(c: Color32) -> f32 {
        let lin = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.039_28 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Readability problems of a color set, if any.
pub fn readability_problem(c: &ThemeColors) -> Option<String> {
    let checks = [
        (c.text_primary, c.panel, 4.5, "text_primary on panel"),
        (c.text_primary, c.surface, 4.5, "text_primary on surface"),
        (
            c.text_primary,
            c.ember_deep,
            3.0,
            "text_primary on accent_deep",
        ),
        (c.text_secondary, c.panel, 3.0, "text_secondary on panel"),
        (c.on_accent, c.accent, 3.0, "on_accent on accent"),
    ];
    checks.iter().find_map(|&(fg, bg, min, what)| {
        let ratio = contrast(fg, bg);
        (ratio < min)
            .then(|| format!("{what} is hard to read (contrast {ratio:.1}:1, needs {min}:1)"))
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct ThemeFile {
    format: String,
    version: u32,
    name: String,
    base: String,
    colors: serde_json::Map<String, serde_json::Value>,
}

/// Reads and checks a theme from JSON text.
pub fn parse(json: &str) -> Result<Theme, String> {
    let file: ThemeFile =
        serde_json::from_str(json).map_err(|e| format!("not a valid theme file: {e}"))?;
    if file.format != THEME_FORMAT {
        return Err("not a Kagero theme (\"format\" must be \"kagero-theme\")".into());
    }
    if file.version == 0 || file.version > THEME_VERSION {
        return Err(format!(
            "theme format version {} is not supported (this Kagero reads version {THEME_VERSION})",
            file.version
        ));
    }
    let name = clean_name(&file.name).ok_or("the theme needs a name")?;
    let dark = match file.base.as_str() {
        "dark" => true,
        "light" => false,
        other => {
            return Err(format!(
                "\"base\" must be \"dark\" or \"light\", not \"{other}\""
            ))
        }
    };
    let mut values = [Color32::BLACK; 17];
    for (i, key) in COLOR_KEYS.iter().enumerate() {
        let text = file
            .colors
            .get(*key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("the color \"{key}\" is missing"))?;
        let c = Rgba::from_hex(text)
            .filter(|c| c.a() == 255)
            .ok_or_else(|| {
                format!("the color \"{key}\" is not an opaque #RRGGBB color: \"{text}\"")
            })?;
        values[i] = Color32::from_rgb(c.r(), c.g(), c.b());
    }
    let colors = colors_from_array(dark, values);
    if let Some(problem) = readability_problem(&colors) {
        return Err(problem);
    }
    Ok(Theme {
        name,
        colors,
        file: None,
    })
}

/// The theme as a theme file (JSON).
pub fn to_json(theme: &Theme) -> String {
    let mut colors = serde_json::Map::new();
    for (key, c) in COLOR_KEYS.iter().zip(colors_to_array(&theme.colors)) {
        colors.insert(
            (*key).to_owned(),
            Rgba::opaque(c.r(), c.g(), c.b()).to_hex().into(),
        );
    }
    let file = ThemeFile {
        format: THEME_FORMAT.to_owned(),
        version: THEME_VERSION,
        name: theme.name.clone(),
        base: if theme.colors.dark { "dark" } else { "light" }.to_owned(),
        colors,
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

fn clean_name(name: &str) -> Option<String> {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_THEME_NAME_CHARS)
        .collect();
    let name = name.trim().to_owned();
    (!name.is_empty()).then_some(name)
}

/// A file name for a theme: its name with only safe characters.
fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if stem.trim_matches('_').is_empty() {
        "theme".to_owned()
    } else {
        stem
    }
}

/// The user's themes in the themes folder, plus the built-in ones.
#[derive(Debug, Default)]
pub struct ThemeStore {
    dir: Option<PathBuf>,
    user: Vec<Theme>,
    /// Files in the folder that couldn't be used: file name and why.
    pub problems: Vec<(String, String)>,
}

impl ThemeStore {
    /// Loads the user's themes from `dir` (which may not exist yet).
    pub fn load(dir: Option<PathBuf>) -> Self {
        let mut store = Self {
            dir,
            ..Self::default()
        };
        store.reload();
        store
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Reads the themes folder again.
    pub fn reload(&mut self) {
        self.user.clear();
        self.problems.clear();
        let Some(dir) = &self.dir else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            })
            .collect();
        files.sort();
        for path in files.into_iter().take(MAX_THEME_FILES) {
            let label = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            match read_theme(&path) {
                Ok(mut theme) => {
                    if is_builtin_name(&theme.name) {
                        self.problems.push((
                            label,
                            format!("\"{}\" is the name of a built-in theme", theme.name),
                        ));
                    } else if self
                        .user
                        .iter()
                        .any(|t| t.name.eq_ignore_ascii_case(&theme.name))
                    {
                        self.problems.push((
                            label,
                            format!("another file already has the theme \"{}\"", theme.name),
                        ));
                    } else {
                        theme.file = Some(path);
                        self.user.push(theme);
                    }
                }
                Err(e) => self.problems.push((label, e)),
            }
        }
    }

    /// Built-in themes first, then the user's (sorted by name).
    pub fn all(&self) -> Vec<Theme> {
        let mut user = self.user.clone();
        user.sort_by_key(|t| t.name.to_lowercase());
        builtins().into_iter().chain(user).collect()
    }

    pub fn find(&self, name: &str) -> Option<Theme> {
        self.all().into_iter().find(|t| t.name == name)
    }

    /// Adds (or updates, by name) a theme from a file. Returns the theme and
    /// whether it replaced one with the same name.
    pub fn import(&mut self, path: &Path) -> Result<(Theme, bool), String> {
        let theme = read_theme(path)?;
        if is_builtin_name(&theme.name) {
            return Err(format!(
                "\"{}\" is the name of a built-in theme — rename it in the file first",
                theme.name
            ));
        }
        let dir = self.dir.clone().ok_or("there is no themes folder")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let existing = self
            .user
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(&theme.name))
            .and_then(|t| t.file.clone());
        let target = existing
            .clone()
            .unwrap_or_else(|| unique_path(&dir, &file_stem(&theme.name)));
        let json = to_json(&theme);
        crate::file_io::write_atomic(&target, |w| {
            std::io::Write::write_all(w, json.as_bytes())?;
            Ok(())
        })
        .map_err(|e| e.to_string())?;
        self.reload();
        let stored = self
            .find(&theme.name)
            .ok_or("the theme could not be read back")?;
        Ok((stored, existing.is_some()))
    }

    /// Deletes a user theme's file.
    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        let theme = self
            .user
            .iter()
            .find(|t| t.name == name)
            .ok_or("built-in themes can't be deleted")?;
        if let Some(file) = &theme.file {
            std::fs::remove_file(file).map_err(|e| e.to_string())?;
        }
        self.reload();
        Ok(())
    }
}

/// Whether `name` is a built-in theme (ignoring case).
pub fn is_builtin_name(name: &str) -> bool {
    builtins().iter().any(|t| t.name.eq_ignore_ascii_case(name))
}

fn read_theme(path: &Path) -> Result<Theme, String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_THEME_FILE_BYTES {
        return Err("the file is too large to be a theme".into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse(&text)
}

fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.json"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.json")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_readable_and_round_trip() {
        let all = builtins();
        assert_eq!(all[0].name, DEFAULT_THEME);
        assert!(all.iter().any(|t| !t.colors.dark), "a light theme");
        for theme in all {
            assert_eq!(readability_problem(&theme.colors), None, "{}", theme.name);
            let back = parse(&to_json(&theme)).expect("parses");
            assert_eq!(back.name, theme.name);
            assert_eq!(back.colors, theme.colors);
        }
    }

    #[test]
    fn broken_theme_files_are_refused_with_reasons() {
        let good = to_json(&builtins()[1]);
        let edit = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v: serde_json::Value = serde_json::from_str(&good).expect("json");
            f(&mut v);
            parse(&v.to_string())
        };
        assert!(edit(&|v| v["version"] = 2.into())
            .unwrap_err()
            .contains("version 2"));
        assert!(edit(&|v| v["format"] = "x".into())
            .unwrap_err()
            .contains("not a Kagero theme"));
        assert!(edit(&|v| {
            v["colors"].as_object_mut().expect("o").remove("accent");
        })
        .unwrap_err()
        .contains("\"accent\" is missing"));
        assert!(edit(&|v| v["colors"]["panel"] = "#12345".into())
            .unwrap_err()
            .contains("\"panel\""));
        assert!(edit(&|v| v["colors"]["text_primary"] = "#1F1A27".into())
            .unwrap_err()
            .contains("hard to read"));
        assert!(edit(&|v| v["base"] = "grey".into()).is_err());
        assert!(edit(&|v| v["name"] = "   ".into()).is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn import_update_delete_and_problems() {
        let dir = std::env::temp_dir().join(format!("kagero-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = ThemeStore::load(Some(dir.clone()));
        assert_eq!(store.all().len(), builtins().len());

        let src = dir.with_extension("src.json");
        let mut theme = builtins()[2].clone();
        theme.name = "Moss/Night".into();
        std::fs::write(&src, to_json(&theme)).expect("write");
        let (imported, replaced) = store.import(&src).expect("import");
        assert!(!replaced);
        assert_eq!(imported.name, "Moss/Night");
        assert!(!imported.is_builtin());
        assert!(imported.file.as_ref().expect("file").starts_with(&dir));
        // Importing the same name again updates it.
        theme.colors.accent = Color32::from_rgb(200, 220, 120);
        std::fs::write(&src, to_json(&theme)).expect("write");
        let (updated, replaced) = store.import(&src).expect("import");
        assert!(replaced);
        assert_eq!(updated.colors.accent, Color32::from_rgb(200, 220, 120));
        assert_eq!(store.all().len(), builtins().len() + 1);
        // Built-in names are refused; built-ins can't be deleted.
        theme.name = "paper".into();
        std::fs::write(&src, to_json(&theme)).expect("write");
        assert!(store.import(&src).is_err());
        assert!(store.delete(DEFAULT_THEME).is_err());
        // A broken file in the folder is listed as a problem, not loaded.
        std::fs::write(dir.join("broken.json"), "{").expect("write");
        store.reload();
        assert_eq!(store.problems.len(), 1);
        store.delete("Moss/Night").expect("delete");
        assert_eq!(store.all().len(), builtins().len());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&src);
    }
}

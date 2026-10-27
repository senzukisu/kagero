//! User settings persisted between sessions (recent files, palette, view options).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::Rgba;
use crate::file_io::{write_atomic, FileError};

/// How many recent projects are remembered.
pub const MAX_RECENT: usize = 10;
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const MAX_PALETTE: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub recent_files: Vec<PathBuf>,
    pub show_grid: bool,
    pub palette: Vec<Rgba>,
    pub new_width: u32,
    pub new_height: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            show_grid: true,
            palette: default_palette(),
            new_width: 32,
            new_height: 32,
        }
    }
}

/// Kagero's built-in 32-color palette: neutrals, ember warms, moss greens and night blues.
pub fn default_palette() -> Vec<Rgba> {
    [
        0x0E0F13, 0x17191F, 0x2C303A, 0x4A4F5C, 0x767B88, 0xA5A9B5, 0xD6D3CC, 0xF0EEE8, 0x3B1F1A,
        0x6E2C1F, 0xA53A26, 0xF06B35, 0xFF9D42, 0xFFC56E, 0xFFE7A8, 0xC88B5A, 0x1E3A2F, 0x2F5D45,
        0x4F8A5B, 0x75C7B5, 0xA8E0C4, 0x5B6B2E, 0x93A541, 0xD3D873, 0x141E3C, 0x223A6B, 0x3767A8,
        0x5FA0E0, 0x9CD2F7, 0x4B2C5E, 0x8A4F9E, 0xE07AB4,
    ]
    .into_iter()
    .map(|hex: u32| Rgba::opaque((hex >> 16) as u8, (hex >> 8) as u8, hex as u8))
    .collect()
}

impl Config {
    /// `%APPDATA%\OnibiLab\Kagero` on Windows, `$XDG_CONFIG_HOME/kagero` or `~/.config/kagero` elsewhere.
    pub fn default_dir() -> Option<PathBuf> {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join("OnibiLab").join("Kagero"));
        }
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("kagero"));
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("kagero"))
    }

    pub fn default_path() -> Option<PathBuf> {
        Self::default_dir().map(|d| d.join("config.json"))
    }

    /// Loads the settings; a missing or damaged file yields defaults (and a log entry).
    pub fn load(path: &Path) -> Self {
        match Self::try_load(path) {
            Ok(cfg) => cfg,
            Err(FileError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                tracing::warn!("ignoring settings file {}: {e}", path.display());
                Self::default()
            }
        }
    }

    fn try_load(path: &Path) -> Result<Self, FileError> {
        let len = std::fs::metadata(path)?.len();
        if len > MAX_CONFIG_BYTES {
            return Err(FileError::TooLarge("settings file".to_owned()));
        }
        let mut cfg: Config = serde_json::from_slice(&std::fs::read(path)?)?;
        cfg.sanitize();
        Ok(cfg)
    }

    fn sanitize(&mut self) {
        self.recent_files.truncate(MAX_RECENT);
        self.palette.truncate(MAX_PALETTE);
        if self.palette.is_empty() {
            self.palette = default_palette();
        }
        self.new_width = self.new_width.clamp(1, crate::core::MAX_CANVAS_SIZE);
        self.new_height = self.new_height.clamp(1, crate::core::MAX_CANVAS_SIZE);
    }

    pub fn save(&self, path: &Path) -> Result<(), FileError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_atomic(path, |w| {
            serde_json::to_writer_pretty(w, self)?;
            Ok(())
        })
    }

    /// Moves `path` to the top of the recent list.
    pub fn add_recent(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(MAX_RECENT);
    }

    pub fn remove_recent(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
    }

    pub fn add_palette_color(&mut self, color: Rgba) -> bool {
        if self.palette.len() >= MAX_PALETTE || self.palette.contains(&color) {
            return false;
        }
        self.palette.push(color);
        true
    }
}

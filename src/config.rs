//! User settings persisted between sessions (recent files, palette, view options).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::onion_skin::OnionSkin;
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
    pub ui: UiSettings,
    pub onion: OnionSkin,
    pub playback: PlaybackSettings,
    pub updates: UpdateSettings,
    pub discord: DiscordSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            show_grid: true,
            palette: default_palette(),
            new_width: 32,
            new_height: 32,
            ui: UiSettings::default(),
            onion: OnionSkin::default(),
            playback: PlaybackSettings::default(),
            updates: UpdateSettings::default(),
            discord: DiscordSettings::default(),
        }
    }
}

/// Workspace layout, remembered between launches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub timeline_visible: bool,
    pub timeline_height: f32,
    pub side_panel_visible: bool,
    /// Turns off decorative motion (pulsing indicators, eased panel changes).
    pub reduce_motion: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            timeline_visible: true,
            timeline_height: 156.0,
            side_panel_visible: true,
            reduce_motion: false,
        }
    }
}

pub const TIMELINE_MIN_HEIGHT: f32 = 142.0;
pub const TIMELINE_MAX_HEIGHT: f32 = 360.0;

/// How the animation preview is timed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackTiming {
    /// Each frame stays on screen for its own duration.
    FrameDurations,
    /// Every frame lasts `1000 / fps` ms, ignoring stored durations.
    FixedFps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackSettings {
    pub timing: PlaybackTiming,
    pub fps: u32,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            timing: PlaybackTiming::FrameDurations,
            fps: 12,
        }
    }
}

pub const FPS_RANGE: std::ops::RangeInclusive<u32> = 1..=60;

/// GitHub updater preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSettings {
    pub check_on_startup: bool,
    pub periodic_check: bool,
    pub interval_hours: u32,
    pub include_prereleases: bool,
    pub notify: bool,
    /// Unix time (seconds) of the last successful check.
    pub last_check_unix: Option<u64>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_on_startup: true,
            periodic_check: true,
            interval_hours: 24,
            include_prereleases: false,
            notify: true,
            last_check_unix: None,
        }
    }
}

pub const UPDATE_INTERVALS_HOURS: [u32; 4] = [6, 12, 24, 72];

/// Discord Rich Presence preferences. By default only the general activity is shared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordSettings {
    pub enabled: bool,
    pub show_mode: bool,
    pub show_project_name: bool,
    pub show_timer: bool,
    pub auto_reconnect: bool,
    pub clear_on_disable: bool,
}

impl Default for DiscordSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_mode: true,
            show_project_name: false,
            show_timer: true,
            auto_reconnect: true,
            clear_on_disable: true,
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
        let h = self.ui.timeline_height;
        self.ui.timeline_height = if h.is_finite() {
            h.clamp(TIMELINE_MIN_HEIGHT, TIMELINE_MAX_HEIGHT)
        } else {
            UiSettings::default().timeline_height
        };
        self.onion.opacity = self.onion.clamped_opacity();
        self.playback.fps = self
            .playback
            .fps
            .clamp(*FPS_RANGE.start(), *FPS_RANGE.end());
        if !UPDATE_INTERVALS_HOURS.contains(&self.updates.interval_hours) {
            self.updates.interval_hours = UpdateSettings::default().interval_hours;
        }
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

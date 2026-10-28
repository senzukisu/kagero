//! User settings persisted between sessions (recent files, palette, view options).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::onion_skin::OnionSkin;
use crate::core::palette::{self, Palette};
use crate::core::Rgba;
use crate::export::ExportSettings;
use crate::file_io::{write_atomic, FileError};

/// How many recent projects are remembered.
pub const MAX_RECENT: usize = 10;
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const MAX_PALETTE: usize = palette::MAX_PALETTE_COLORS;
/// Most palettes kept in the settings.
pub const MAX_PALETTES: usize = 64;
/// How many recently used colors are remembered.
pub const MAX_RECENT_COLORS: usize = 12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub recent_files: Vec<PathBuf>,
    pub show_grid: bool,
    /// Kagero 0.1–0.2 single palette; migrated into `palettes` on load.
    #[serde(default, skip_serializing)]
    pub palette: Vec<Rgba>,
    pub palettes: Vec<Palette>,
    pub active_palette: usize,
    pub recent_colors: Vec<Rgba>,
    pub new_width: u32,
    pub new_height: u32,
    pub ui: UiSettings,
    pub onion: OnionSkin,
    pub playback: PlaybackSettings,
    pub updates: UpdateSettings,
    pub discord: DiscordSettings,
    pub export: ExportSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            show_grid: true,
            palette: Vec::new(),
            palettes: palette::builtins(),
            active_palette: 0,
            recent_colors: Vec::new(),
            new_width: 32,
            new_height: 32,
            ui: UiSettings::default(),
            onion: OnionSkin::default(),
            playback: PlaybackSettings::default(),
            updates: UpdateSettings::default(),
            discord: DiscordSettings::default(),
            export: ExportSettings::default(),
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
    /// Open the main window maximized (remembered from the last session).
    pub window_maximized: bool,
    /// Use the experimental Kagero Pixel interface font instead of the classic one.
    pub pixel_font: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            timeline_visible: true,
            timeline_height: 156.0,
            side_panel_visible: true,
            reduce_motion: false,
            window_maximized: true,
            pixel_font: false,
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

/// Colors of Kagero's default palette (Kagero Ember).
pub fn default_palette() -> Vec<Rgba> {
    palette::kagero_ember().colors
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
        // Kagero 0.1–0.2 kept one palette: keep a customized one as "My Palette".
        let legacy = std::mem::take(&mut self.palette);
        if !legacy.is_empty() && legacy != default_palette() {
            let mut colors = legacy;
            colors.truncate(MAX_PALETTE);
            self.palettes.insert(
                0,
                Palette {
                    name: "My Palette".to_owned(),
                    colors,
                },
            );
            self.active_palette = 0;
        }
        self.palettes.truncate(MAX_PALETTES);
        for p in &mut self.palettes {
            p.name = palette::sanitize_name(&p.name);
            p.colors.truncate(MAX_PALETTE);
        }
        self.palettes.retain(|p| !p.colors.is_empty());
        if self.palettes.is_empty() {
            self.palettes = palette::builtins();
        }
        self.active_palette = self.active_palette.min(self.palettes.len() - 1);
        self.recent_colors.truncate(MAX_RECENT_COLORS);
        self.export.sanitize();
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

    /// The palette shown in the colors panel.
    pub fn active_palette(&self) -> &Palette {
        // `sanitize` and every mutator keep at least one palette and a valid index.
        &self.palettes[self.active_palette.min(self.palettes.len() - 1)]
    }

    pub fn active_palette_mut(&mut self) -> &mut Palette {
        let i = self.active_palette.min(self.palettes.len() - 1);
        &mut self.palettes[i]
    }

    /// Adds a color to the active palette (no duplicates, at most 256 colors).
    pub fn add_palette_color(&mut self, color: Rgba) -> bool {
        let p = self.active_palette_mut();
        if p.colors.len() >= MAX_PALETTE || p.colors.contains(&color) {
            return false;
        }
        p.colors.push(color);
        true
    }

    /// Removes a color from the active palette; a palette keeps at least one color.
    pub fn remove_palette_color(&mut self, index: usize) -> bool {
        let p = self.active_palette_mut();
        if index < p.colors.len() && p.colors.len() > 1 {
            p.colors.remove(index);
            true
        } else {
            false
        }
    }

    /// Moves a color of the active palette by `delta` positions.
    pub fn move_palette_color(&mut self, index: usize, delta: isize) -> bool {
        let p = self.active_palette_mut();
        let target = index as isize + delta;
        if index >= p.colors.len() || target < 0 || target as usize >= p.colors.len() {
            return false;
        }
        let c = p.colors.remove(index);
        p.colors.insert(target as usize, c);
        true
    }

    /// Adds a palette (name made unique) and selects it.
    pub fn add_palette(&mut self, mut palette: Palette) -> bool {
        if self.palettes.len() >= MAX_PALETTES {
            return false;
        }
        palette.name = self.unique_palette_name(&palette.name);
        palette.colors.truncate(MAX_PALETTE);
        self.palettes.push(palette);
        self.active_palette = self.palettes.len() - 1;
        true
    }

    /// Deletes the active palette; the last one can't be deleted.
    pub fn delete_active_palette(&mut self) -> bool {
        if self.palettes.len() <= 1 {
            return false;
        }
        self.palettes
            .remove(self.active_palette.min(self.palettes.len() - 1));
        self.active_palette = self.active_palette.min(self.palettes.len() - 1);
        true
    }

    /// Restores the built-in palettes that are missing (custom palettes are kept).
    pub fn restore_builtin_palettes(&mut self) {
        for b in palette::builtins() {
            if let Some(existing) = self.palettes.iter_mut().find(|p| p.name == b.name) {
                *existing = b;
            } else if self.palettes.len() < MAX_PALETTES {
                self.palettes.push(b);
            }
        }
    }

    pub fn unique_palette_name(&self, base: &str) -> String {
        let base = palette::sanitize_name(base);
        if !self.palettes.iter().any(|p| p.name == base) {
            return base;
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|name| !self.palettes.iter().any(|p| &p.name == name))
            .unwrap_or(base)
    }

    /// Remembers a color as recently used (most recent first, no duplicates).
    pub fn remember_color(&mut self, color: Rgba) -> bool {
        if self.recent_colors.first() == Some(&color) {
            return false;
        }
        self.recent_colors.retain(|c| *c != color);
        self.recent_colors.insert(0, color);
        self.recent_colors.truncate(MAX_RECENT_COLORS);
        true
    }
}

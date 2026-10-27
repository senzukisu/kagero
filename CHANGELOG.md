# Changelog

All notable changes to Kagero are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `kagero.exe <file>` opens a `.kgr` or `.png` on start (e.g. Explorer "Open with").
- README screenshots and logo; `examples/readme_assets.rs` regenerates the logo and
  the demo animation used for them.

## [0.2.0] Beta — 2026-10-27

The animation update.

### Added

- **Frame-by-frame animation.** Documents have frames; each frame has its own image
  for every layer, and the layer structure is shared by all frames.
- Frame operations: new, duplicate (independent copy), delete (with confirmation when
  the frame has drawings), move left/right, select, duration 10–60 000 ms. All undoable.
- **Timeline** panel with real cached thumbnails, frame counter, duration field,
  horizontal scrolling, resizing and collapsing (F7).
- **Animation preview**: real-time looping playback from the current frame, by frame
  durations or fixed FPS (1–60); drawing is blocked while it plays.
- **Onion skin**: previous frame (warm) and next frame (cool) overlays with adjustable
  opacity; never saved or exported.
- Export the current frame or **all frames** as numbered PNG files.
- **`.kgr` format version 2** storing frames, durations and stable ids; Kagero 0.1
  projects (format 1) open as one-frame documents.
- **GitHub updater** (Settings → Updates, Help → Check for Updates): semantic version
  checks, optional prereleases, startup and periodic checks, download with progress and
  cancel, SHA-256 and minisign signature verification, installation through a separate
  `kagero-updater.exe` with backup, automatic rollback and "Restore Previous Version".
- **Discord Rich Presence** (Settings → Discord) through the local Discord desktop
  client: activity per mode, privacy options (project name off by default), automatic
  reconnect, status and Connect/Disconnect buttons.
- Settings window (Ctrl+,) with Kagero Ember toggle switches; workspace layout,
  onion skin and playback settings persist between launches.
- New shortcuts: Enter, `,` `.` ← →, Ctrl+Home, Alt+N, Alt+D, O, F7, F8,
  Ctrl+Shift+E, Ctrl+,.
- Frame menu; frame and preview state in the status bar; canvas hints for preview and
  onion skin; "Reduce motion" option.

### Changed

- Transparency checkerboard squares are exactly one image pixel and zoom is measured in
  device pixels, so pixels stay even at 125%/150% display scaling.
- Pressed and selected controls use a deep ember color so their text stays readable.
- The memory budget now covers layers × frames.

## [0.1.0] Alpha — 2026-10-27

First public alpha.

### Added

- Start screen with Create New, Open Project and recent projects.
- New images from 1×1 to 2048×2048 with transparent, white or black background.
- Tools: Pencil, Eraser, Fill, Eyedropper, Line, Rectangle, Filled Rectangle, Pan;
  brush size 1–32; eyedropper samples the merged image or the active layer.
- Canvas with nearest-neighbor zoom (25%–9600%), zoom at cursor, Space/middle-drag pan,
  transparency checkerboard, pixel grid, center and fit.
- Layers: add, delete, rename, reorder, select, show/hide, alpha compositing.
- Undo/Redo for all document changes, limited to 256 steps / 256 MiB.
- `.kgr` project format v1 (ZIP + JSON + PNG layers) with validation.
- PNG import and flattened PNG export with transparency.
- Atomic saving, unsaved-changes prompts, close-window guard, error dialogs, log file.
- Persistent settings: recent files, palette, grid toggle, last image size.
- Kagero visual theme and original pixel icons.

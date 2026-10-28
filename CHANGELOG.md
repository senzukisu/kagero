# Changelog

All notable changes to Kagero are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [0.3.5] Beta — 2026-10-28

The pixel interface.

### Added

- **Kagero Pixel**, Kagero's own pixel font (MIT): ASCII, Cyrillic and the interface's
  typographic symbols, 186 glyphs with 10-pixel capitals (text stays about as large as
  in 0.3.0). Source in `assets/fonts/kagero-pixel.txt`, built into `KageroPixel.ttf`
  by `scripts/build-font.py` and embedded in the executable.
  Text sizes snap so one font pixel is always whole device pixels (sharp at 100–200 %
  display scale); headings use 2×. Characters the font lacks fall back to egui's fonts.
- **Pixel-style interface**: square corners, hard offset shadows on windows, menus and
  the start card, bevelled icon buttons (raised on hover, pressed in when held or
  selected), outlined text buttons, rectangular slider handles, solid scrollbars.

- **Redesigned About window**: banner with the flame and a version badge, studio,
  license and links (documentation, changelog, GitHub), and the main shortcuts drawn
  as pixel keycaps.

### Changed

- All 31 icons redrawn as 16×16 pixel art with outlines, shading and color; 2× in the
  toolbar (now 36 px buttons), 1× elsewhere; disabled icons fade to gray. The active
  tool is shown pressed in with an amber outline instead of a solid orange fill.
- Pixel bitmaps (icons, logo) are drawn as one mesh per bitmap on whole device pixels.

### Fixed

- **Discord Rich Presence showed no pictures**: it referred to art assets that were
  never uploaded to the Discord application. The Kagero flame and the drawing/animation
  icons now load from image URLs in this repository (`docs/images/discord/`).
- Kagero opened in a small window placed off to the side. It now opens **maximized**;
  if you restore it to a window, the next start opens a centered window instead.
- 1-pixel-wide bitmap pixels could disappear because egui's anti-aliased rectangle fill
  collapses them; bitmaps no longer use it.

## [0.3.0] Beta — 2026-10-28

Export & sprite workflow.

### Added

- **Export dialog** (Ctrl+Shift+E, File → Export…) with Image, Animation and Sprite
  Sheet tabs, live preview, output size, background export with progress and cancel,
  Open Folder, and settings remembered per tab and format.
- A unified export service (`src/export`): snapshot → prepare → encode. Exports never
  modify the document; files are written atomically.
- **Image export**: PNG (pixel-exact), JPEG (quality 1–100, white/black/custom
  background, lossy-compression warning) and lossless WebP.
- **Scale** 100/200/300/400/800 % or any integer ×1–×64, nearest neighbor; sizes over
  16 384 px per side or 512 MiB are refused before allocation.
- **Frame choice**: current, selected (Ctrl/Shift+click in the timeline) or all;
  numbered names (`sprite_0001.png`) with configurable base name and digits; a single
  file is not numbered unless asked; one confirmation before replacing existing files.
- **Animated GIF**: per-frame durations (10 ms steps, min. 20 ms), loop forever or play
  once, transparency threshold, soft-edge matte, exact palette up to 255 colors,
  reduction with optional Floyd–Steinberg dithering (off by default), warnings.
- **Sprite sheets**: horizontal/vertical strips and grids by columns or rows, spacing,
  padding, trim, extrude, transparent or solid background, preview, overflow checks.
- **Sprite sheet metadata**: Kagero JSON (`kagero-sprite-sheet`, version 1) and JSON
  Hash.
- **Rectangle Select** tool (M): move (drag, arrows), move outline (Ctrl+drag), copy,
  cut, paste, delete, duplicate, select all, deselect, flip horizontal/vertical, rotate
  90/180/270°. Each is one undo step that also restores the selection. Without a
  selection, flip/rotate apply to the whole layer.
- **Image** menu (flip/rotate) and selection items in the Edit menu.
- **Palettes**: several palettes, new/duplicate/rename/delete, add/remove/reorder
  colors, GIMP `.gpl` and HEX import/export, click-to-copy HEX, RGBA readout, 12 recent
  colors, built-in Kagero Ember, Ember, Night Sakura and Misty Forest.
- `kagero.exe <file>` opens a `.kgr` or `.png` on start (e.g. Explorer "Open with").
- README screenshots and logo; `examples/readme_assets.rs` regenerates the logo and
  the demo animation used for them.
- Tests: export (PNG/JPEG/WebP/GIF/sheets/metadata), selection, palettes and settings
  migration, headless shortcut and Export dialog tests.

### Changed

- Ctrl+Shift+E opens the Export dialog (it used to export all frames as PNGs directly;
  choose *All* frames in the dialog for the same result).
- Ctrl+D duplicates the selection when there is one, otherwise the frame as before.
- Arrow keys nudge the selection when there is one, otherwise they change frames.
- A customized 0.2 palette is migrated into a palette named "My Palette".

### Not included

- Animated WebP and lossy WebP (the WebP encoder used is lossless-only); the dialog
  says so.
- Packed (bin-packing) sprite-sheet layouts.

The `.kgr` project format is unchanged (version 2).

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

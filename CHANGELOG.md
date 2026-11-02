# Changelog

All notable changes to Kagero are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [0.4.2] Beta — 2026-11-02

Reliability, tests and a tidier tool strip. No new features and no change to the project
format: 0.4.1 and 0.4.2 open each other's projects.

### Changed

- **Tool strip:** the buttons sit on an exact grid, centered, with even gaps and thin
  lines between the groups. The strip is thinner: 44 px instead of about 56 px with one
  column, 78 px with two (before, the panel margin was counted twice).
- **Logs:** opening, saving and exporting write the file path to `kagero.log`, and a
  failure writes the path with the reason. Recovery copies that can't be deleted, and
  a Discord connection that doesn't close in time on exit, are logged instead of being
  ignored.

### Fixed

- If an export crashed inside its background thread, the Export dialog kept waiting
  forever. It now shows "the export stopped unexpectedly" and can be used again.

### Tests

- **Long editing sessions:** 40 seeded sessions of 400 random operations each (drawing
  with every shape tool and symmetry, switching tools mid-stroke, fills, unfinished
  polygons, layers, layer properties and locks, frames, tags, selections, transforms,
  canvas size, crop, palette conversion). Every step must undo and redo exactly, saving
  and reopening must change nothing, and a refused operation must change nothing.
- **Compatibility projects:** a hand-made `.kgr` file of each format version (1, 2 and 3)
  in `tests/fixtures/`, written by `scripts/make-test-fixtures.py` from the format
  description, not by Kagero. Each must open as intended and survive a round trip.
- **Damaged files:** 1 200 damaged copies of those projects (flipped bytes, cut-off
  files, inserted or removed bytes). None may crash the reader.
- These found no new bugs in editing, undo or files. The test suite has 290 tests.

### Developers

- `src/app.rs` (1 850 lines) is split by job: `app/mod.rs` (the app, layout, commands),
  `files.rs`, `dialog_flow.rs`, `settings_actions.rs` and `background.rs`. The main
  menu is one function per menu instead of one 600-line function. The code was moved,
  not rewritten.
- The developer docs describe the new layout, the error-handling rules, thumbnail
  caching, the compatibility tests and how to regenerate them. The `.kgr` writer's
  documentation now says when it writes version 3.
- Dependencies updated to their latest compatible patch versions; the third-party
  license list is regenerated. Benchmarks (`cargo run --release --example bench`)
  measure the same as 0.4.1; nothing in this release was meant to change speed.

### Removed

- Unused code for a "Soon" badge in the Settings menu.

## [0.4.1] Beta — 2026-11-01

Performance, stability and code quality. This is also the first published release
with everything from 0.4.0 (0.4.0 was not published on its own).

### Changed

- **Exported images are big enough to see.** Still images and animations now use
  **Auto size** by default: the smallest whole-number scale that makes the result at
  least 512 px on its longest side (a 10×10 sprite becomes 520×520, still pixel-sharp).
  Turn it off in the Export dialog to pick the scale yourself; sprite sheets keep their
  own scale. **Export Current Frame as PNG** (Ctrl+E) follows the same setting and says
  which size it wrote.
- **Touchpads:** a two-finger swipe moves the view (also sideways), and pinching zooms
  smoothly around the fingers. Before, every tiny touchpad scroll event jumped a whole
  zoom level. Mouse wheels still zoom one level per notch; high-resolution wheels add up
  partial notches.

### Performance

Measured with the new `cargo run --release --example bench`, 0.4.0 and 0.4.1 built from
the same benchmark and run alternately on the same machine. The ranges are the spread
of the runs, which was large because the machine was busy.

| Case | 0.4.0 | 0.4.1 |
|------|-------|-------|
| Animated GIF export, 120 frames ×4 | 2.6–5.8 s | 0.5–1.4 s (4–5× faster) |
| Compositing 1024² × 1 layer | 10–13 ms | 3.4–4.2 ms |
| Compositing 512² × 16 layers | 31–35 ms | 14–21 ms |
| Save `.kgr` (56 MB, 320 images) | 517–521 ms | 138–304 ms |
| Open `.kgr` (same project) | 575–605 ms | 153–357 ms |
| Undo memory after filling 2048² | 50.3 MB | under 0.1 MB |

- Compositing (canvas, preview, onion skin, thumbnails, export) works row by row on the
  raw pixels instead of pixel by pixel through every layer, with fast paths for opaque
  pixels. A test checks that every pixel matches the previous method exactly.
- GIF frames are mapped to the palette and compressed on several threads, with a fast
  color lookup and a cache for reduced palettes.
- Projects are saved and opened on several threads (each layer image is encoded or
  decoded on its own), in batches that keep the extra memory bounded.
- The undo history stores big uniform changes (fills, long strokes) as runs instead of
  one entry per pixel; undo and redo are unchanged.
- Pencil strokes no longer allocate for every brush stamp. They measured about the same
  as 0.4.0 within the noise.

### Fixed

- The Settings window grew to the width of the screen on the Themes page (found while
  taking the documentation screenshots). The Settings test now checks the window's width.

### Removed

- The `anyhow` dependency (unused).
- Unused code left over from older versions: the old flood fill (replaced by the 0.4
  fill), `Block::copy_from` (replaced by shaped selections), the 0.1 "export all
  frames" helpers (replaced by the Export dialog) and several unused accessors
  (`APP_NAME`, `PixelBuffer::coords_of`, `Document::layer_data`, `Document::tag_at`,
  `Document::active_frame_cel`, `History::can_undo`/`can_redo`, `Editor::is_read_only`,
  `StillFormat::keeps_transparency`, `UpdateManager::with_download_dir`). Each was
  checked for any use first.
- 22 outdated images in `docs/images` that nothing referenced (the documentation uses
  `docs/.gitbook/assets`).

### Developers

- `examples/bench.rs`: repeatable benchmarks of compositing, thumbnails, onion skin,
  saving, opening, exporting, filling and drawing, including the undo history's memory.
- `core::hash`: a fast hasher for pixel indices and colors (the GIF encoder and strokes
  use it).
- New tests: fast compositing against pixel-by-pixel compositing, compact undo history
  (exact undo and redo), Auto size, telling mouse wheels and touchpads apart, and the
  Settings window width.

## [0.4.0] Beta — 2026-10-30

*Not published on its own; its features shipped with 0.4.1.*

Everyday comfort and pixel-artist tools.

### Added

- **Canvas Size** (Image → Canvas Size…, Ctrl+Alt+C): new width and height with a 3×3 anchor,
  a preview, and a warning when pixels would be cut off. It works on all layers and frames
  and is one undo step.
- **Crop tool** (C): drag a frame, adjust it by its handles, Enter crops all layers and
  frames, Esc cancels. Also **Crop to Selection**.
- **Layer opacity** (slider in the Layers panel): affects the canvas, preview and every
  export, never the layer's pixels. Dragging the slider is one undo step.
- **Duplicate Layer** (Ctrl+Alt+D, "Layer copy") and **Merge Down** (Ctrl+Alt+M, keeps the
  look including opacity, works on all frames, one undo step).
- **Lock** and **Alpha Lock** per layer, and a right-click menu on layers.
- **Autosave and crash recovery**: while there are unsaved changes, a copy is written every
  2 minutes (1–60, Settings → General) to `%APPDATA%\OnibiLab\Kagero\recovery`. It is never
  written next to or over your project. After a crash, the next start offers each copy
  (Open Recovered · Open Original · Delete Copy), and **File → Recover Projects…** shows them
  any time. Copies are written atomically; old ones are trimmed (10 copies, 512 MB).
- **Frame tags**: named, colored frame ranges shown above the timeline. You can loop a
  tag in the preview, jump between tags (Alt+, / Alt+.), export just a tag, and the tags
  are written to sprite-sheet JSON (`tags` in Kagero JSON, Aseprite-style
  `meta.frameTags` in JSON Hash). Tags follow frame insertions and deletions with exact
  undo.
- **Lasso** (Q) and **Magic Wand** (W, with tolerance, contiguous, and active layer or all
  visible layers) selections, and selection modes **Add** (Shift), **Subtract** (Alt),
  **Intersect** (Shift+Alt) and **Invert Selection** (Ctrl+Shift+I). Copy, cut, paste,
  delete, move, duplicate, flip and rotate work on the exact selected shape.
- **Animated WebP** export: lossless, with full transparency, millisecond durations, and
  loop forever or play once.
- **Packed sprite sheets**: frames are trimmed and packed tightly, deterministically and
  without overlaps; identical frames can share one spot (`duplicateOf` in the metadata).
- **Ellipse** (K) and **Polygon** (P) tools, outlined or filled.
- **Symmetry** (mirror left↔right and/or top↔bottom), **dithering** (25%, checker 50%,
  75%), **round brushes**, and **custom brushes** made from a selection (Ctrl+B).
- **Fill** options: tolerance, contiguous on/off, active layer or all visible layers;
  stays inside the selection.
- **Exact color entry**: HEX (`#RGB`, `#RRGGBB`, `#RRGGBBAA`) and R, G, B, A fields.
- **Convert to Palette** (Image menu): maps every color to the nearest color of the
  current palette, with a before/after preview, for the active layer, its frames, or all
  unlocked layers.
- **Themes** (Settings → Themes): Kagero Ember, Night Sakura, Misty Forest and Paper
  (light), live preview with Keep/Revert, and import, export or delete of your own themes
  (versioned JSON; unreadable color combinations are refused).
- **Plugins** (Settings → Plugins, Plugins menu): Lua 5.4 scripts that change the active
  layer, run in a sandbox (no files, network or programs; memory and time limits) on a
  background thread, as one undo step. New plugins start turned off. Includes an
  installable example plugin and documentation of the API (version 1).
- **Program icon** for `kagero.exe` (Explorer, taskbar, shortcuts) and version information.
- **Installer** `kagero-<version>-setup.exe`: per user (no administrator rights) in
  `%LOCALAPPDATA%\Programs\Kagero`, with a Start menu shortcut, an optional desktop
  shortcut and an "Installed apps" entry. Updates keep working. Uninstalling keeps your
  projects, settings, themes, plugins and autosave copies.

### Changed

- `.kgr` projects are written as format version 3 only when a layer has opacity below
  100%; otherwise they stay version 2, readable by Kagero 0.3. Frame tags are an optional
  addition that older versions ignore. Kagero 0.4 opens all earlier projects.
- New settings are stored so that going back to an older Kagero (Settings → Updates →
  Restore Previous Version) still reads your settings.
- The tool strip switches to two columns when the window is too short for one.
- The minimum timeline height grew a little to fit the tag row.

### Known limitations

- Pencil, shapes and other drawing tools don't stay inside the selection (Fill does).
- Lasso and Magic Wand can't drag the selected pixels; use Rectangle Select (M) or the
  arrow keys.
- Custom brushes are kept until Kagero closes; they aren't saved.
- Filling the largest canvas (2048×2048) takes about a tenth of a second.
- The plugin API covers the active layer, the selection and the colors; there's no access
  to other layers, frames or new tools yet.
- The installer and Kagero aren't signed with a paid certificate, so Windows SmartScreen
  may warn on first start. The release files are signed with minisign.

## [0.3.8] Beta — 2026-10-29

Settings, shortcuts and experiments.

### Added

- **Custom shortcuts** (Settings → Shortcuts): every shortcut can be changed — click a
  key, press the new combination. A key that's already used moves over and the page
  says where it came from; ↺ restores one default, *Reset all* every default; search by
  action or key. Menus and tooltips show your keys. Ctrl+C/X/V, Esc, Space and the
  arrow keys stay fixed and are listed.
- **Settings → Experimental**, for features switched on only there:
  - **Pixel font** (moved here from General),
  - **Pixel-perfect pencil**: 1 px Pencil and Eraser strokes skip the doubled pixel in
    "L" corners, so freehand lines stay one pixel thin,
  - **Tiled view**: the image repeated around itself, dimmed, to check seamless tiles.
- **Settings → Plugins** and **Themes**: "Coming soon" pages with what's planned.
- **Help → Licenses…** (also in About): Kagero's MIT license and every third-party
  license, built into the program.
- Discord Rich Presence: an **Exporting Sprites** status while the Export dialog is
  open, and a **Get Kagero** button on your profile card (can be turned off).

### Changed

- **Redesigned Settings**: a side menu with icons in App / Integrations / Labs groups,
  and a header for each page.
- **Redesigned New Image**: named preset cards (Icon, Sprite, Character, Portrait,
  Large, Tile strip, Screen, Wide screen) with shape previews, width ⇄ height swap,
  background swatches and a live preview with the memory size.
- **Clearer icons**: a real paint bucket for Fill, ⇄ arrows for Swap Colors, three
  overlapping frames for Onion Skin, a frame with an arrow for Move Frame, and new
  icons for the settings pages (keyboard, download, chat, flask, plug, palette).
- The release package contains only `kagero.exe`, `kagero-updater.exe` and
  `SHA256SUMS.txt` (checksums of both); the license texts are inside the program and
  the readme and changelog are on GitHub. Updating from older versions works as before.
- Documentation rewritten and reorganized (Getting started · Draw · Animate · Export ·
  Reference · Developers · About), with new screenshots of every screen. Updates and
  Discord moved into *Install & update* and *Settings*; the updater and Discord
  internals into *Services*; testing and releasing into *Contributing*. Old page
  addresses redirect to the new ones.

### Fixed

- Discord Rich Presence could wait half a minute or longer when Discord stalled the
  connection handshake (it does after several quick reconnects). Connecting now gives
  up after 8 seconds and retries with back-off, closing Kagero never waits on Discord,
  and the status reads "Connecting…" instead of "Not configured" while it starts.
- Settings → Discord refreshes its connection status on its own.

## [0.3.7] Beta — 2026-10-29

A hotfix for 0.3.6.

### Fixed

- The Export dialog could be taller than the screen (its title and the Export button
  off-screen at 1920×1080): the divider between options and preview stretched to the
  full screen height. It now has the height of its row, and the options scroll when
  the screen is too short.

### Changed

- README rewritten for 0.3.6 with new screenshots (editor with onion skin, animation
  preview, sprite sheet export).

## [0.3.6] Beta — 2026-10-29

The window update.

### Added

- **Custom title bar**: Kagero draws its own window frame — the flame, the menus,
  the document title and pixel-style minimize, maximize/restore and close buttons
  in one 32 px bar. Drag the bar to move the window, double-click it to maximize or
  restore; resize a restored window from its edges. Close still asks about unsaved
  changes.
- **Settings → General → Pixel font (experimental)** switches the interface to
  Kagero Pixel.

### Changed

- The interface font is the classic font from 0.3.0 again (as large as before).
  Kagero Pixel stays available as the experimental option above while it is being
  improved; it is now larger and wider (11-pixel capitals, 6-pixel-wide letters).

### Fixed

- Kagero could still open "maximized" but only 1280×800 (larger with display
  scaling) at the top-left of the screen: Windows marked the window maximized at
  creation before it had its final size. The window is now created normally and
  maximized on the first frame, filling the work area.

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

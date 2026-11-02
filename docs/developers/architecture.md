---
description: "How the code is organized: layers, data model, history, export."
icon: sitemap
---

# Architecture

Kagero is one Rust crate with a library and three binaries: `kagero` (the editor),
`kagero-updater` (the update helper) and `kagero-setup` (the installer, also used as
`uninstall.exe`). The code is split into layers; each depends only
on the layers below it.

```
┌───────────────────────────────────────────────────────────────────┐
│ UI           app/*, ui/*       title bar, canvas, timeline, panels, │
│                                dialogs, export dialog, shortcuts   │
├───────────────────────────────────────────────────────────────────┤
│ Application  editor/*          Editor, tools, selection ops, History│
├───────────────────────────────────────────────────────────────────┤
│ Pixel engine core/*            Rgba, PixelBuffer, Document, frames,│
│                                onion skin, raster, selection, palette│
├───────────────────────────────────────────────────────────────────┤
│ I/O          file_io/*         .kgr, PNG, palette files, atomic writes│
│              export/*          snapshot → prepare → encode          │
└───────────────────────────────────────────────────────────────────┘
  services/      updater, update installer, setup/uninstall, recovery, dialogs, logging
  integrations/  Discord Rich Presence
  config.rs  settings · theme.rs  pixel style and current colors · themes.rs  theme files
  plugins.rs  Lua plugins · release.rs  public IDs/keys
```

## Pixel engine — `src/core`

* `Rgba` — straight-alpha 8-bit color with integer "source over" compositing.
* `PixelBuffer` — `width × height × 4` bytes, bounds-checked, size validated
  (1–2048 per side) before allocation.
* `Document` — layers (`LayerInfo`: id, name, visibility, opacity, locks) shared by all
  frames, `Frame`s (id, duration, one cel per layer, revision) and frame tags (`tags.rs`:
  name, first/last frame position, color). At least one layer and frame; active
  indices always in range. `LayerId`/`FrameId` stay stable across reordering. The
  current selection lives here in memory only. Compositing (canvas, preview, export)
  goes through `composite_frame*`, which applies layer opacity.
* `structure_revision` and per-frame `revision` change with pixels or layer
  order/visibility; thumbnail and texture caches key on them.
* `hash`: a fast hasher for pixel indices and colors (stroke maps, GIF palettes).
* `raster` (lines, rectangles, ellipses, square and round brushes), `region`
  (similar-color areas for Fill and the magic wand, polygon masks for the lasso),
  `dither` (ordered patterns), `onion_skin` (view-only overlay plan), `selection`
  (`SelRect`, `Selection` = bounds plus an optional shared mask with
  combine/invert/outline, `Block` with flips/rotations), `palette` (built-ins,
  nearest color).
* Limits: 2048×2048, 64 layers, 1000 frames, all cels ≤ 512 MiB.

## Application — `src/editor`

`Editor` wraps one `Document` and is the **only** way the UI changes it.

* **Strokes** remember the frame and layer they started on and each touched pixel's
  original color; `end_stroke` turns that into one `Change::Pixels`.
* **Frame and layer commands** are `Change`s too. Navigation is not an undo step.
* **Selection operations** (`selection_ops.rs`) diff the active cel and record a
  `Change::Group` of the pixel change plus `SetSelection`, so undo restores pixels and
  the outline together.
* **Tools** paint through one `stamp` that applies the brush tip, symmetry, dithering
  and alpha lock. Polygons keep their corners in the stroke until they are closed.
* **Frame edits that move tags** record a `Group` of the frame change plus a `Tags`
  swap, so undo restores tags removed with their last frame.
* **Read-only mode** blocks pixel edits during the preview.
* **Saved state** compares the top of the undo stack with the state at the last save.

The history is bounded (256 entries, 256 MiB). Big uniform pixel changes are stored
as runs (`PixelRuns`) when that is smaller. Data moves between the document and a
`Change` instead of being copied; undoing a change on another frame switches to it.

| Change | Stored data |
|--------|-------------|
| `Pixels` | frame id, layer id, per pixel: index, before, after |
| `AddLayer` / `RemoveLayer` | index; the layer and its cels while removed |
| `MoveLayer`, `RenameLayer`, `SetVisible` | from/to, old/new name, visibility |
| `AddFrame` / `RemoveFrame` / `MoveFrame` | index; the frame while removed; from/to |
| `SetFrameDuration` | frame id, before/after |
| `SetSelection` · `Group` | selection before/after · a labeled list of changes |
| `SetLayerProps` | layer id, opacity and locks before/after |
| `SwapLayer` · `Geometry` · `Tags` | self-inverse swaps: a whole layer (Merge Down), the canvas size with all cels (Canvas Size, Crop), the tag list |

## Files — `src/file_io`

* `kgr.rs` — `.kgr` writer (version 2, or 3 when a layer has opacity) and readers for
  versions 1–3 that validate before allocating ([format](project-format.md)).
* `parallel_map` — decodes and encodes cel PNGs on several threads, in batches of 64.
* `png.rs` — PNG import (one frame, one layer) and quick current-frame export.
* `palette.rs` — GIMP `.gpl` and HEX palette files (1 MiB, 256 colors max).
* `write_atomic` — temporary file, flush, `fsync`, rename.

## Export — `src/export`

1. **Snapshot** (`render::snapshot`, UI thread) copies the composited frames
   (`FrameImage`: index, duration, RGBA). The document is never touched again.
2. **Prepare** — `scale_nearest`; sprite-sheet `plan` (cells, grid, positions in checked
   64-bit math) and `compose`; GIF palette analysis.
3. **Encode** — `still.rs` (PNG, JPEG over a background, lossless WebP), `gif.rs` (one
   global palette, index 0 transparent, exact ≤ 255 colors or NeuQuant, optional
   Floyd–Steinberg), `metadata.rs` ([Kagero JSON, JSON Hash](metadata-format.md)).
   Files are encoded in memory and written with `write_atomic`.

`service.rs` turns an `ExportRequest` into files; `ExportJob` runs it on a thread with a
cancel flag and progress. Every output size is checked against 16 384 px per side and
512 MiB before allocation. `ExportSettings` is stored in the settings file.

## UI — `src/app`, `src/ui`

* `KageroApp` (`app/mod.rs`) owns the config, `Editor`, `CanvasView`, `Timeline`,
  `Player`, the dialogs, `UpdateManager` and `DiscordPresence`, lays out the panels and
  executes commands. Panels return `ui::Command`s; the app executes them. The rest of
  the app is split by job: `files.rs` (new, open, save, PNG and palette files),
  `dialog_flow.rs` (what happens when a dialog is answered), `settings_actions.rs`
  (settings, themes, updates, Discord status) and `background.rs` (plugin commands,
  autosave). Shortcuts are collected in `ui/shortcuts.rs` from the user's keymap
  (`ui/keymap.rs`: every `Action`, its default key, and the overrides stored in the
  settings) and never steal keys from text fields.
* `title_bar.rs` — Kagero's own window frame (the system frame is off): menus, title,
  window buttons, drag/double-click, edge resizing. The window is maximized on the
  first frame, which is reliable on Windows with display scaling.
* `canvas.rs` — view transform in device pixels, partial texture uploads while
  drawing, a frame-texture cache for onion skin and preview, selection drawing.
* `timeline.rs` — controls and frame cards with thumbnails cached by frame id and
  revision (a few rebuilt per UI frame); Ctrl/Shift+click selects frames for export.
* `toolbar.rs` — the tool strip: an exact grid of 32 px buttons, one column, or two
  when the window is too short. `menu.rs` — the main menu, one function per menu.
* `export_dialog.rs`, `palette.rs`, `layers.rs`, `settings/`, `dialogs.rs`, `home.rs`.
* `icons.rs` — 16×16 pixel icons drawn as one mesh of whole device pixels.
* `theme.rs` — colors, square corners, hard shadows, and the classic or Kagero Pixel
  font ([UI design](ui-design.md)).
* Settings writes are debounced (600 ms) and flushed on exit.

## Services and integrations

The updater, the installer, autosave and Discord Rich Presence are described in
[Services](services.md). Plugins are documented in [Plugins](../plugins/README.md) and
themes in [Themes](../reference/themes.md). None of them is required: failures are shown quietly and
editing continues.

## Error handling

* Typed errors (`thiserror`) with user-facing messages: dialogs for file and update
  errors, the status bar for editing errors, quiet status lines for network/Discord.
* No `unwrap()`/`expect()` in user-reachable code; tests use `expect` freely.
* Errors are not dropped silently. A `let _ =` is used only where nothing more can be
  done, with a comment saying why (best-effort cleanup, the updater helper's own log).
* Background work never blocks the editor: the export job finishes with an error even
  if its thread panics, a plugin or autosave thread that stops is reported as an error,
  and on exit Kagero waits for Discord at most 1.5 s.
* Logs: `%APPDATA%\OnibiLab\Kagero\kagero.log`. Opening, saving and exporting log the
  file path, and failures log the path with the reason.

## Extending

* **New tool:** a `ToolKind` variant → `Editor::begin_stroke`/`update_stroke`, an icon in
  `icons.rs`, a shortcut.
* **New undoable operation:** a `Change` variant with `apply`/`revert`/`byte_size`/`dirty`.
* **New project format version:** bump `KGR_FORMAT_VERSION`, add a reader branch, keep
  the older ones. Write the new version only when the file needs it (as version 3 does
  for layer opacity), so older Kagero keeps opening ordinary projects.
* **New settings:** add fields with `#[serde(default)]` and never add variants to enums
  older versions read; a downgrade must still load the settings file.

---
description: "How the code is organized: layers, data model, history, export."
icon: sitemap
---

# Architecture

Kagero is one Rust crate with a library and two binaries: `kagero` (the editor) and
`kagero-updater` (the install helper). The code is split into layers; each depends only
on the layers below it.

```
┌───────────────────────────────────────────────────────────────────┐
│ UI           app.rs, ui/*      title bar, canvas, timeline, panels, │
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
  services/      updater, update installer, native dialogs, logging
  integrations/  Discord Rich Presence
  config.rs  settings · theme.rs  Kagero Ember style · release.rs  public IDs/keys
```

## Pixel engine — `src/core`

* `Rgba` — straight-alpha 8-bit color with integer "source over" compositing.
* `PixelBuffer` — `width × height × 4` bytes, bounds-checked, size validated
  (1–2048 per side) before allocation.
* `Document` — layers (`LayerInfo`: id, name, visibility) shared by all frames, and
  `Frame`s (id, duration, one cel per layer, revision). At least one layer and frame;
  active indices always in range. `LayerId`/`FrameId` stay stable across reordering.
  The current selection lives here in memory only.
* `structure_revision` and per-frame `revision` change with pixels or layer
  order/visibility; thumbnail and texture caches key on them.
* `raster` (lines, rectangles, brushes, flood fill), `onion_skin` (view-only overlay
  plan), `selection` (`SelRect`, `Block` with flips/rotations), `palette` (built-ins).
* Limits: 2048×2048, 64 layers, 1000 frames, all cels ≤ 512 MiB.

## Application — `src/editor`

`Editor` wraps one `Document` and is the **only** way the UI changes it.

* **Strokes** remember the frame and layer they started on and each touched pixel's
  original color; `end_stroke` turns that into one `Change::Pixels`.
* **Frame and layer commands** are `Change`s too. Navigation is not an undo step.
* **Selection operations** (`selection_ops.rs`) diff the active cel and record a
  `Change::Group` of the pixel change plus `SetSelection`, so undo restores pixels and
  the outline together.
* **Read-only mode** blocks pixel edits during the preview.
* **Saved state** compares the top of the undo stack with the state at the last save.

The history is bounded (256 entries, 256 MiB). Data moves between the document and a
`Change` instead of being copied; undoing a change on another frame switches to it.

| Change | Stored data |
|--------|-------------|
| `Pixels` | frame id, layer id, per pixel: index, before, after |
| `AddLayer` / `RemoveLayer` | index; the layer and its cels while removed |
| `MoveLayer`, `RenameLayer`, `SetVisible` | from/to, old/new name, visibility |
| `AddFrame` / `RemoveFrame` / `MoveFrame` | index; the frame while removed; from/to |
| `SetFrameDuration` | frame id, before/after |
| `SetSelection` · `Group` | selection before/after · a labeled list of changes |

## Files — `src/file_io`

* `kgr.rs` — `.kgr` v2 writer, v1 and v2 readers that validate before allocating
  ([format](project-format.md)).
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

## UI — `src/app.rs`, `src/ui`

* `KageroApp` owns the config, `Editor`, `CanvasView`, `Timeline`, `Player`, the dialogs,
  `UpdateManager` and `DiscordPresence`. Panels return `ui::Command`s; the app executes
  them. Shortcuts are collected in `ui/shortcuts.rs` from the user's keymap
  (`ui/keymap.rs`: every `Action`, its default key, and the overrides stored in the
  settings) and never steal keys from text fields.
* `title_bar.rs` — Kagero's own window frame (the system frame is off): menus, title,
  window buttons, drag/double-click, edge resizing. The window is maximized on the
  first frame, which is reliable on Windows with display scaling.
* `canvas.rs` — view transform in device pixels, partial texture uploads while
  drawing, a frame-texture cache for onion skin and preview, selection drawing.
* `timeline.rs` — controls and frame cards with lazily rebuilt thumbnails;
  Ctrl/Shift+click selects frames for export.
* `export_dialog.rs`, `palette.rs`, `layers.rs`, `settings/`, `dialogs.rs`, `home.rs`.
* `icons.rs` — 16×16 pixel icons drawn as one mesh of whole device pixels.
* `theme.rs` — colors, square corners, hard shadows, and the classic or Kagero Pixel
  font ([UI design](ui-design.md)).
* Settings writes are debounced (600 ms) and flushed on exit.

## Services and integrations

The updater, the install helper and Discord Rich Presence are described in
[Services](services.md). None of them is required: failures are shown quietly and
editing continues.

## Error handling

* Typed errors (`thiserror`) with user-facing messages: dialogs for file and update
  errors, the status bar for editing errors, quiet status lines for network/Discord.
* No `unwrap()`/`expect()` in user-reachable code; tests use `expect` freely.
* Logs: `%APPDATA%\OnibiLab\Kagero\kagero.log`.

## Extending

* **New tool:** a `ToolKind` variant → `Editor::begin_stroke`/`update_stroke`, an icon in
  `icons.rs`, a shortcut.
* **New undoable operation:** a `Change` variant with `apply`/`revert`/`byte_size`/`dirty`.
* **New project format version:** bump `KGR_FORMAT_VERSION`, add a reader branch, keep
  the older ones.

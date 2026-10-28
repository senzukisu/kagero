---
description: "How the code is organized: layers, data model, history."
icon: sitemap
---

# Kagero Architecture

Kagero is one Rust crate with a library and two binaries (`kagero`, `kagero-updater`).
The code is split into layers; each depends only on the layers below it.

```
┌──────────────────────────────────────────────────────────────────┐
│ UI           app.rs, ui/*          panels, canvas, timeline,      │
│                                     settings, dialogs, shortcuts  │
├──────────────────────────────────────────────────────────────────┤
│ Application  editor/*              Editor, tools, History         │
├──────────────────────────────────────────────────────────────────┤
│ Pixel Engine core/*                Rgba, PixelBuffer, Document,   │
│                                     frames, onion skin, raster    │
├──────────────────────────────────────────────────────────────────┤
│ File I/O     file_io/*             KGR v1/v2, PNG, palettes,      │
│                                     atomic writes                 │
└──────────────────────────────────────────────────────────────────┘
  export/        snapshot → prepare → encode: PNG/JPEG/WebP, GIF,
                 sprite sheets, JSON metadata, background jobs
  services/      native dialogs, logging, updater, update installer
  integrations/  Discord Rich Presence
  config.rs      persisted settings     theme.rs   Kagero Ember style
  release.rs     update source, signing key, Discord application ID
```

## Pixel engine (`src/core`)

- `Rgba` — straight-alpha 8-bit color; integer "source over" compositing.
- `PixelBuffer` — `width × height × 4` bytes, bounds-checked accessors, size validated
  (`1..=2048` per side) before allocating.
- `Document` — layers (`LayerInfo`: id, name, visibility; bottom to top) shared by all
  frames, and `Frame`s (`animation.rs`: id, duration, one cel per layer, revision
  counter). Invariants: ≥ 1 layer and frame, each frame has exactly one cel per layer.
  The active layer and frame are indices kept in range by every mutator.
- `LayerId` / `FrameId` are stable across reordering; history entries use them.
- `structure_revision` and per-frame `revision` change when pixels or layer
  order/visibility change; caches (thumbnails, frame textures) key on them.
- `onion_skin.rs` — which neighbor frames to overlay and how (pure, view-only).
- `raster` — Bresenham lines, rectangles, square brushes, 4-connected flood fill.
- `selection.rs` — `SelRect` (clipped rectangles) and `Block` (a copied pixel block with
  flips and rotations), plus `clear`/`stamp`. The document keeps the current selection
  in memory only.
- `palette.rs` — `Palette` and the built-in palettes.
- Limits: 2048×2048, 64 layers, 1000 frames, `layers × frames` cels ≤ 512 MiB.

## Application layer (`src/editor`)

`Editor` wraps one `Document` and is the only way the UI changes it.

- **Strokes** record the frame and layer they started on and the original color of
  every touched pixel; `end_stroke` turns that into one `Change::Pixels`.
- **Frame commands**: add, duplicate, delete, move, set duration — each a `Change`.
  `select_frame`/`next_frame`/`previous_frame` are navigation, not undo steps.
- **Selection operations** (`selection_ops.rs`): copy, cut, paste, delete, move,
  duplicate, flip/rotate. Each is one undo step: a `Change::Group` with the pixel diff
  of the active cel and a `SetSelection` change, so undo restores pixels *and* the
  selection outline. Without a selection, transforms apply to the whole layer.
- **Read-only mode** blocks pixel edits during animation preview.
- **Dirty tracking** reports the rectangle to re-upload; frame switches report `Full`.
- **Saved state** compares the id on top of the undo stack with the id at last save.

### History

| Change             | Stored data                                         |
|--------------------|-----------------------------------------------------|
| `Pixels`           | frame id, layer id, per pixel: index, before, after |
| `AddLayer`         | index; the layer and its cels only while undone     |
| `RemoveLayer`      | index; the layer and all its cels while removed     |
| `MoveLayer`        | from/to                                             |
| `RenameLayer`      | old/new name                                        |
| `SetVisible`       | new visibility                                      |
| `AddFrame`         | index; the frame only while undone (`duplicate` flag for the label) |
| `RemoveFrame`      | index; the removed frame while removed              |
| `MoveFrame`        | from/to                                             |
| `SetFrameDuration` | frame id, before/after                              |
| `SetSelection`     | selection before/after                              |
| `Group`            | label and a list of changes, reverted in reverse order |

Data moves between the document and the change instead of being copied. Undoing or
redoing a pixel change on another frame switches to that frame. The history is bounded
by 256 entries and 256 MiB.

## File I/O (`src/file_io`)

- `kgr.rs` — `.kgr` v2 writer, v1 + v2 readers with validation before allocation
  (see [The .kgr project format](project-format.md)).
- `png.rs` — PNG import (one frame, one layer), current-frame export, all-frames
  export with sanitized file names.
- `palette.rs` — GIMP `.gpl` and HEX palette files (1 MiB, 256 colors max).
- `write_atomic` — temporary file, flush, `fsync`, rename.

## Export (`src/export`)

Every export goes through three stages:

1. **Snapshot** (`render::snapshot`, UI thread) — copies the composited frames
   (`FrameImage`: index, duration, RGBA image). The document is never touched again.
2. **Prepare** — `scale_nearest`, sprite-sheet `plan` (cell size, grid, positions with
   checked 64-bit math) and `compose`, GIF palette analysis.
3. **Encode** — `still.rs` (PNG, JPEG over a background, lossless WebP), `gif.rs`
   (one global palette, index 0 transparent, exact colors ≤ 255 or NeuQuant, optional
   Floyd–Steinberg), `metadata.rs` (Kagero JSON v1, JSON Hash). Files are encoded in
   memory and written with `write_atomic`.

`service.rs` turns an `ExportRequest` into files; `ExportJob` runs it on a thread with
a cancel flag and progress counter. All output sizes are checked against
`MAX_OUTPUT_SIDE` (16 384) and `MAX_OUTPUT_BYTES` (512 MiB) before allocation.
`ExportSettings` (options.rs) is stored in the settings file and sanitized on load.

## UI (`src/app.rs`, `src/ui`)

- `KageroApp` owns config, `Editor`, `CanvasView`, `Timeline`, `Player`, the settings
  window, `UpdateManager` and `DiscordPresence`. Panels return `ui::Command`s; the app
  executes them. Shortcuts live in `ui/shortcuts.rs`.
- `canvas.rs` — view transform in device pixels, partial texture uploads for editing, a
  frame-texture cache (256 MiB, LRU) for onion skin and preview, badges.
- `timeline.rs` — controls and frame strip; thumbnails cached per frame and rebuilt
  lazily (≤ 12 per UI frame, visible cards only); Ctrl/Shift+click selects frames for
  export.
- `export_dialog.rs` — the Export window: tabs, options, cached preview textures, the
  running `ExportJob`, overwrite confirmation.
- `palette.rs` — colors panel: palette picker and menu, swatch context menu, recent colors.
- Clipboard: Ctrl+C/X/V arrive from egui as clipboard events, and Ctrl+V only when the
  system clipboard holds text, so copying also puts a short text marker there.
- `animation_preview.rs` — `Player`: the shown frame is computed from a monotonic clock
  (`frame_at_time`), and the next repaint is scheduled for the next frame change.
- `settings/` — General, Updates, Discord pages; `toggle` is the shared switch widget.
- Settings writes are debounced (600 ms) and flushed on exit.

## Services and integrations

- `services/updater.rs` — GitHub Releases client behind an `HttpClient` trait (real
  client: `ureq` + OS TLS), semantic version selection, download with checksum and
  minisign verification, and a background `UpdateManager`. See [Updater internals](updater-internals.md).
- `services/update_installer.rs` + `src/bin/kagero-updater.rs` — package extraction,
  backup/rollback install after Kagero exits, restore.
- `integrations/discord_rpc.rs` — presence building with privacy rules, a transport-
  independent connection state machine and a background IPC worker. See
  [Discord RPC internals](discord-rpc-internals.md).

None of these is required: failures are shown quietly and editing continues.

## Error handling

- Typed errors (`thiserror`) with user-facing messages; dialogs for file and update
  errors, the status bar for editing errors, quiet status lines for network/Discord.
- No `unwrap()`/`expect()` in user-reachable paths; tests use `expect` freely.
- Logs: `%APPDATA%\OnibiLab\Kagero\kagero.log`.

## Extending

- New tool: `ToolKind` variant → `Editor::begin_stroke`/`update_stroke`, icon, shortcut.
- New undoable operation: `Change` variant with `apply`/`revert`/`byte_size`/`dirty`.
- New format version: bump `KGR_FORMAT_VERSION`, add a reader branch, keep older ones.

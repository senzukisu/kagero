# Kagero Architecture

Kagero is a single Rust crate (library + binary) split into four layers. Each layer only
depends on the layers below it.

```
┌──────────────────────────────────────────────────────────────┐
│ UI          app.rs, ui/*            egui panels, canvas, input │
├──────────────────────────────────────────────────────────────┤
│ Application editor/*                Editor, tools, History    │
├──────────────────────────────────────────────────────────────┤
│ Pixel Engine core/*                 Rgba, PixelBuffer, Layer, │
│                                     Document, raster algos    │
├──────────────────────────────────────────────────────────────┤
│ File I/O    file_io/*               KGR, PNG, atomic writes   │
└──────────────────────────────────────────────────────────────┘
   support: config.rs (settings), theme.rs (visual system),
            services/ (native dialogs, logging)
```

## Pixel engine (`src/core`)

- `Rgba` is a straight-alpha 8-bit color. `Rgba::over` is "source over" compositing
  done in integer math so results are identical on every machine.
- `PixelBuffer` owns `width × height × 4` bytes. Every accessor checks bounds and returns
  `None`/`false` for coordinates outside the image. Sizes are validated
  (`1..=2048` per side) **before** allocating.
- `Document` holds layers bottom-to-top plus the active layer index. Layers have a stable
  `LayerId` so history entries keep pointing at the right layer after reordering.
- `raster` contains Bresenham lines, rectangle outlines/fills, square brush stamps and a
  4-connected flood fill (explicit stack, no recursion).
- Limits: 2048×2048 pixels, 64 layers, 512 MiB of layer pixel data per document.

## Application layer (`src/editor`)

`Editor` wraps one open `Document` and is the only way the UI changes it.

- **Strokes.** `begin_stroke` / `update_stroke` / `end_stroke` implement Pencil, Eraser,
  Line, Rectangle and Filled Rectangle. During a stroke the editor writes pixels directly
  to the active layer and remembers the *original* color of every touched pixel. Shapes
  are previewed by restoring those pixels and redrawing. `end_stroke` turns the record
  into one `Change::Pixels` (one undo step per gesture). `cancel_stroke` (Esc) restores.
- **One-shot tools.** Fill and Eyedropper act in `begin_stroke`.
- **Commands.** Layer operations build a `Change`, apply it and push it to `History`.
- **Dirty tracking.** Every edit records the changed rectangle; the canvas takes it with
  `take_dirty()` and re-uploads only that region of the texture.
- **Saved state.** Each history entry has an id. The editor remembers the id that was on
  top of the undo stack at the last save; the document is "modified" whenever the
  current top id differs. Undoing back to the saved point makes it clean again.

### History

`Change` is a reversible operation that stores only what it needs:

| Change          | Stored data                                   |
|-----------------|-----------------------------------------------|
| `Pixels`        | per pixel: index, before, after               |
| `AddLayer`      | index; the layer itself only while undone     |
| `RemoveLayer`   | index; the removed layer while it is removed  |
| `MoveLayer`     | from/to                                       |
| `RenameLayer`   | old/new name                                  |
| `SetVisible`    | new visibility                                |

Layers move between the document and the change instead of being copied. The history is
bounded by entry count (256) and memory (256 MiB); the oldest entries are dropped first.
Pushing a new change clears the redo stack.

## File I/O (`src/file_io`)

- `kgr.rs` — the project format (see [PROJECT_FORMAT.md](PROJECT_FORMAT.md)).
- `png.rs` — PNG import (single layer) and export (flattened, with alpha). Image
  dimensions are read from the header and checked before decoding; the decoder also gets
  explicit allocation limits.
- `write_atomic` writes to a temporary file in the same folder, flushes and `fsync`s it,
  then renames it over the target. If anything fails the old file is untouched and the
  temporary file is removed.

## UI (`src/app.rs`, `src/ui`)

- `KageroApp` owns the config, the optional `Editor`, the `CanvasView` and the current
  modal dialog. Panels never mutate the document; they return `ui::Command` values that
  `KageroApp::execute` dispatches.
- Actions that would discard the document (New, Open, Close, Exit, window close) go
  through `request()`, which shows the "Unsaved changes" dialog if needed.
- `CanvasView` holds zoom/pan, the GPU texture and the pointer state. `ViewTransform`
  is the pure screen↔pixel mapping (unit-tested). Zoom levels are device pixels per image
  pixel (converted with egui's `pixels_per_point`), and the image origin is snapped to a
  device pixel, so nearest-neighbor scaling stays crisp at any Windows display scale. The
  transparency checkerboard is a 2×2 repeating texture whose cells are whole image pixels.
- Keyboard shortcuts are collected once per frame. When a text field has focus, only
  Ctrl-based file shortcuts are handled; letters, Space and Ctrl+Z go to the text field.

## Error handling

- File and core errors are typed (`thiserror`) and their messages are written for users.
- Errors are shown in a modal dialog (file operations) or in the status bar (editing).
- No `unwrap()`/`expect()` in user-reachable paths; tests use `expect` freely.
- Logs go to `%APPDATA%\OnibiLab\Kagero\kagero.log`.

## Extending

- New tool: add a `ToolKind` variant, handle it in `Editor::begin_stroke` /
  `update_stroke`, add an icon in `ui/icons.rs` and a shortcut in `app.rs`.
- New undoable operation: add a `Change` variant with `apply`/`revert`/`byte_size`/`dirty`.
- New format version: bump `KGR_FORMAT_VERSION`, keep reading older versions.

# Testing

## Automated checks

Run before every commit:

```
cargo fmt --check
cargo clippy --all-targets
cargo test
```

| File                         | Covers                                                        |
|------------------------------|---------------------------------------------------------------|
| `src/core/color.rs`          | alpha compositing edge cases, hex formatting                  |
| `src/core/buffer.rs`         | bounds checks, raw buffer validation                          |
| `src/core/raster.rs`         | line continuity/endpoints, rectangle outline, brush sizes     |
| `src/ui/canvas.rs`           | screen ↔ pixel mapping, zoom steps                            |
| `tests/document.rs`          | creation, size limits, bounds, compositing, transparency, layer/memory limits |
| `tests/editor.rs`            | every tool, stroke = one undo step, cancel, hidden layers, fill region, eyedropper modes, layer operations with full undo/redo, saved-state tracking |
| `tests/history.rs`           | entry and byte limits, redo clearing                          |
| `tests/file_io.rs`           | KGR round trip (memory and disk), atomic replace, PNG export/import, hidden layers in export, damaged/truncated archives, missing entries, newer/foreign versions, bad sizes, path traversal names, corrupted PNGs, oversized PNGs |

## Manual test checklist (Windows 10/11)

1. Launch `kagero.exe`: the start screen shows the flame, Create New, Open Project,
   Recent Projects and "Onibi Lab".
2. **Create New** → 32×32 transparent → canvas is centered with a checkerboard.
3. Pencil (B): draw a fast diagonal — no gaps. Right button draws the secondary color.
4. Ctrl+Z removes the whole stroke; Ctrl+Y brings it back.
5. Eraser (E) leaves checkerboard; Fill (G) stays inside closed shapes; Eyedropper (I)
   picks a palette color (left = primary, right = secondary).
6. Line (L), Rectangle (U), Filled Rectangle (Shift+U): preview while dragging, Esc cancels.
7. Mouse wheel zooms at the cursor; Space+drag and middle-drag pan without drawing;
   Home centers; grid appears from 600%.
8. Layers: add, rename (double-click), hide, move up/down, delete; drawing on a hidden
   layer shows a status message; every action is undoable.
9. Ctrl+S → save `test.kgr`; title loses the "•"; close the window → no prompt.
10. Reopen Kagero → Recent Projects → `test.kgr` → layers, names, order, visibility match.
11. Draw, then close the window → "Unsaved changes" prompt; Cancel keeps the window.
12. Ctrl+E → export PNG; open it in another viewer → transparency preserved.
13. Ctrl+O a PNG → opens as one layer; Ctrl+S asks for a new `.kgr` path.
14. Open a renamed `.txt` as `.kgr` → readable error dialog, app keeps running.

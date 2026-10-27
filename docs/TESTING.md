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
| `src/ui/canvas.rs`           | screen ↔ pixel mapping, device-pixel snapping, checker cells, zoom steps |
| `src/core/animation.rs`      | duration clamping, playback timing across loop boundaries     |
| `src/core/onion_skin.rs`     | neighbors, first/last frame, disabled sides, opacity clamping |
| `src/ui/timeline.rs`         | thumbnails keep aspect, sample real pixels                    |
| `tests/document.rs`          | creation, size limits, bounds, compositing, transparency, layer/memory limits |
| `tests/editor.rs`            | every tool, stroke = one undo step, cancel, hidden layers, fill region, eyedropper modes, layer operations with full undo/redo, saved-state tracking |
| `tests/history.rs`           | entry and byte limits, redo clearing                          |
| `tests/file_io.rs`           | KGR round trip (memory and disk), atomic replace, PNG export/import, hidden layers in export, damaged/truncated archives, missing entries, newer/foreign versions, bad sizes, path traversal names, corrupted PNGs, oversized PNGs; **v2**: multi-frame round trip (ids, durations, active frame, frames not merged), 0.1 project migration, damaged v2 documents (missing/odd/wrong-size cels, cel counts, ids, indices), memory budget before decoding, future versions, current/all-frames export |
| `tests/animation.rs`         | new/duplicate (independence)/delete/move frames, last frame protected, durations, undo/redo incl. switching to the edited frame, shared layers vs per-frame pixels, navigation wrap, read-only preview, frame limit, playback timing, onion skin not touching pixels or export |
| `tests/updater.rs`           | semver ordering, no update, newest stable, prereleases on/off, invalid API data, untrusted/missing assets, network errors, verified download, unverified without key, bad signature, size and checksum mismatch, cancel, check interval; installer: package validation, install + backup + restore, failed install rollback, unsafe names, unwritable folder, helper arguments, dev builds never install |
| `tests/discord.rs`           | statuses per mode, privacy (project name opt-in, no paths), no application id, Discord not running with back-off, manual retry, change-only updates and rate limit, reconnect after disconnect, disabling clears, user disconnect |

All network and Discord tests use fakes: they never contact GitHub or need Discord.

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

### Animation (0.2)

15. Open a 0.1 `.kgr` → one frame with the same layers; save; reopen.
16. Alt+N, Alt+D, draw on several frames; thumbnails update after each stroke.
17. Delete a frame with drawings → confirmation; Ctrl+Z brings it back.
18. Change a duration, move frames left/right, undo/redo each.
19. Enter → preview loops at the set durations; clicking the canvas shows a hint and
    draws nothing; Enter/Esc stops on the shown frame. Try Fixed FPS.
20. O → onion skin: warm previous, cool next; first/last frame show one side only;
    Ctrl+E export contains no onion colors.
21. Ctrl+Shift+E → folder → numbered PNGs; exporting again asks before replacing.
22. F7/F8 collapse panels; drag the timeline edge; restart → layout is kept.

### Services

23. Offline: Help → Check for Updates shows a short network message; editing works.
24. Settings → Discord with Discord closed: *Not connected — Discord desktop app is
    not running*, no dialogs. Start Discord → *Connected*, profile shows the activity.
25. Turn on *Show the project name* → state shows only the file name.

## Performance

`tests/performance.rs` is an ignored benchmark:

```
cargo test --release --test performance -- --ignored --nocapture
```

Results on the development machine (Windows 11, release build, 2026-10-27):

| Document                         | Build  | Save (size)     | Load   | Thumbnail   | Full composite |
|----------------------------------|--------|-----------------|--------|-------------|----------------|
| 64×64, 300 frames, 3 layers      | 9 ms   | 10 ms (419 KiB) | 14 ms  | 0.03 ms/frame | 0.01 ms/frame |
| 256×256, 60 frames, 2 layers     | 10 ms  | 8 ms (241 KiB)  | 14 ms  | 0.04 ms/frame | 0.15 ms/frame |
| 64×64, 1000 frames, 1 layer      | 22 ms  | 15 ms (536 KiB) | 18 ms  | 0.03 ms/frame | 0.01 ms/frame |

These cover the engine and file I/O (mostly transparent test frames, so PNGs are small).
UI smoothness was checked by hand only. Results depend on the machine; very large
documents (2048² × many frames) are bounded by the 512 MiB memory budget by design.
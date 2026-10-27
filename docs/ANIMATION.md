# Animation

Kagero 0.2 adds frame-by-frame animation. It is a pixel-art tool, not a motion
editor: there are no tracks, bones, curves or tweens.

## Model

```
Document
├── layers: [LayerInfo]          name, visibility, id — shared by every frame
└── frames: [Frame]              playback order
     └── Frame { id, duration_ms, cels: [PixelBuffer; layers] }
```

- Every frame owns one image (a *cel*) per layer, in layer order. Editing a frame never
  touches another frame's pixels.
- Layer **structure** is shared: adding, deleting, renaming, reordering or hiding a layer
  applies to all frames. Switching frames therefore never makes a hidden layer visible
  and never changes which layer is selected.
- Frames and layers have stable ids that survive reordering and are saved in `.kgr`.
- Limits: 1000 frames, 64 layers, and `layers × frames × width × height × 4 ≤ 512 MiB`.
- Durations are 10–60 000 ms (default 100 ms). Input outside the range is clamped.

## Frame operations

| Action            | Where                                   | Shortcut |
|-------------------|-----------------------------------------|----------|
| New empty frame   | Timeline `+`, Frame menu                | Alt+N    |
| Duplicate frame   | Timeline, Frame menu, card right-click  | Alt+D    |
| Delete frame      | Timeline, Frame menu, card right-click  | —        |
| Move left/right   | Timeline arrows, Frame menu             | —        |
| Select frame      | Click a card                            | `,` `.` ← → |
| First frame       | Timeline, Frame menu                    | Ctrl+Home |
| Set duration      | Timeline "Duration" field               | —        |

All of them are undoable. Deleting a frame that has drawings asks for confirmation; the
last frame can't be deleted. Undoing a stroke made on another frame switches to that
frame so the change is visible.

## Timeline

The bottom panel shows controls and a scrollable strip of frame cards. Each card has a
real thumbnail (nearest-neighbor, at most 64 px), the frame number and its duration.

- Resize it by dragging its top edge; collapse/expand with the arrow button or **F7**.
- Thumbnails are cached per frame and rebuilt only when that frame's pixels or the layer
  structure change, at most 12 per UI frame and only for visible cards. The active
  frame's thumbnail updates when a stroke ends.

## Preview

**Enter** (or the play button) loops the animation from the current frame; **Enter** or
**Esc** stops and stays on the frame that was showing.

- Timing uses a monotonic clock: the shown frame is computed from elapsed real time, so
  slow repaints never slow the animation down and errors never accumulate.
- *Frame durations* mode uses each frame's duration; *Fixed FPS* (1–60) ignores them.
- While previewing, drawing is blocked (clicks show a hint) and edits stop the preview.
- Frames are shown from a GPU texture cache (256 MiB budget, least-recently-used
  eviction), so the edit texture isn't re-uploaded every frame.

## Onion skin

**O** toggles it. The previous frame is drawn over the canvas with a warm tint, the next
frame with a cool tint, at the chosen opacity (5–90 %, default 35 %). The first frame
has no previous overlay and the last has no next one — it does not wrap around.

Onion skin is a view setting saved in the app settings. It never changes frame pixels,
is not saved in `.kgr`, is not exported, and is hidden while previewing. A badge on the
canvas explains the extra images while it is on.

## Export

- **Export Current Frame as PNG** (Ctrl+E): the active frame, flattened.
- **Export All Frames as PNGs** (Ctrl+Shift+E): pick a folder; files are named
  `<project>_0001.png`, `<project>_0002.png`, … Kagero asks before replacing files.
- GIF and sprite-sheet export are planned for a later version.

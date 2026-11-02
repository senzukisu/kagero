---
description: "The .kgr file format, versions, migration and validation."
icon: file-zipper
---

# The `.kgr` Project Format

Format id `kagero-project`. Kagero 0.4 writes **version 2**, or **version 3** when a
layer has opacity below 100 %, and reads versions 1, 2 and 3. Kagero 0.2 and 0.3 write
version 2. The format version is independent of the application version.

| Version | Written by | Adds |
|---------|-----------|------|
| 1 | 0.1 | Layers (one image) |
| 2 | 0.2+ | Frames, durations, ids |
| 3 | 0.4+, only when needed | Layer opacity (a project that older Kagero would show wrongly is refused by them instead) |

{% hint style="info" %}
The selection, the clipboard, palettes and export settings are not part of a project:
palettes and export settings live in the user settings file (`config.json`), the
selection and clipboard only in memory.
{% endhint %}

A `.kgr` file is a standard ZIP archive.

## Version 2 and 3

```
project.kgr
├── manifest.json            format id and version (Deflate)
├── document.json            size, layers, frames, durations, ids (Deflate)
└── frames/
    ├── 0000/                first frame
    │   ├── 0000.png         image of layer 0 (bottom) in this frame, RGBA8 (Stored)
    │   └── 0001.png
    ├── 0001/
    └── …
```

### manifest.json

```json
{
  "format": "kagero-project",
  "format_version": 2,
  "app_version": "0.2.0"
}
```

- `format` must be exactly `kagero-project`.
- `format_version` is checked first. Newer versions are refused with a message saying
  which version is supported; nothing else in the archive is read.
- `app_version` is informational.

### document.json

```json
{
  "width": 32,
  "height": 32,
  "active_layer": 1,
  "active_frame": 0,
  "layers": [
    { "id": 1, "name": "Background", "visible": true, "opacity": 255 },
    { "id": 2, "name": "Ink",        "visible": false, "opacity": 128, "locked": true }
  ],
  "frames": [
    { "id": 1, "duration_ms": 100, "cels": ["frames/0000/0000.png", "frames/0000/0001.png"] },
    { "id": 4, "duration_ms": 250, "cels": ["frames/0001/0000.png", "frames/0001/0001.png"] }
  ],
  "tags": [
    { "name": "walk", "start": 0, "end": 1, "color": "#E86A33" }
  ]
}
```

- `layers` are bottom to top and shared by all frames (name, visibility, id).
- `opacity` (0–255, default 255) is how strongly a layer is shown; a value below 255
  makes the file version 3. `locked` and `alpha_lock` (default `false`) are written only
  when `true`.
- `tags` (optional, written only when there are any): frame tags with a name (1–48
  characters), the first and last frame position (0-based, inclusive) and an `#RRGGBB`
  color. At most 256. Tags with a range outside the frames or an empty name are dropped
  when opening, rather than refusing the file. Older Kagero ignores them.
- `frames` are in playback order. Each frame lists one PNG per layer in layer order.
- Cel paths must be exactly `frames/FFFF/LLLL.png` (frame and layer positions,
  zero-padded). Any other path is rejected.
- Ids are positive and unique among layers and among frames; they survive reordering.
- `duration_ms` is clamped to 10–60 000.
- Unknown fields are ignored, so optional data can be added within version 2.

### Cel PNGs

8-bit RGBA, exactly `width × height`, straight (non-premultiplied) alpha. Fully
transparent pixels are `0,0,0,0`. Every frame stores its own images, so saving never
merges frames.

## Version 1 (Kagero 0.1)

```
project.kgr
├── manifest.json     { "format": "kagero-project", "format_version": 1 }
├── document.json     { "width", "height", "active_layer", "layers": [{ "name", "visible", "file" }] }
└── layers/0000.png …
```

`file` must be exactly `layers/NNNN.png`.

### Migration

Opening a version 1 project creates a document with **one frame** (100 ms) holding the
layers in their original order, names, visibility and pixels, with the same active
layer. No user action is needed; the next save writes version 2.

## Validation when opening

Checked in this order, before any large allocation:

1. File size ≤ 1 GiB; ZIP has ≤ 64 016 entries.
2. `manifest.json` (≤ 8 MiB): id, then version.
3. `document.json` (≤ 8 MiB): `1 ≤ width, height ≤ 2048`, `1 ≤ layers ≤ 64`,
   `1 ≤ frames ≤ 1000`, `width × height × 4 × layers × frames ≤ 512 MiB`, active indices
   in range, ids valid, no more archive entries than cels + 16.
4. For each cel: canonical name, entry ≤ 20 MiB (declared size and actual bytes), PNG
   header dimensions checked before decoding, decoder allocation limits, decoded size
   must equal the document size.
5. Layer names are trimmed, control characters removed, limited to 64 characters.

Entries are read into memory by exact name. Nothing is ever extracted to disk, so
paths inside the archive cannot reach the file system.

## Saving

Projects are written to a temporary file next to the target
(`.<name>.kgr.<pid>.tmp`), flushed and synced, then renamed over the target. A crash or
error during saving leaves the previous file intact.

## Compatibility tests

`tests/fixtures/` holds one small project per format version, written by
`scripts/make-test-fixtures.py` from this description rather than by Kagero:

| File | Version | Checks |
|------|---------|--------|
| `v1-layers.kgr` | 1 | layers become one frame; names, visibility, active layer, pixels |
| `v2-animation.kgr` | 2 | frame ids and durations, layer ids that aren't 1, 2, 3…, defaults for opacity, locks and tags |
| `v3-opacity-tags.kgr` | 3 | layer opacity in compositing, lock and alpha lock, frame tags |

`tests/fixtures.rs` opens each one, saves and reopens it, and opens 400 damaged copies
of each (flipped bytes, cut-off files, inserted or removed bytes): every copy is either
refused with an error or opens as a valid document, and none crashes the reader.

## Versioning policy

- Additive, optional changes keep the current version.
- Any change older readers would misinterpret bumps `format_version`. New Kagero
  versions keep reading all older versions.

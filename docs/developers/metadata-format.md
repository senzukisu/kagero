---
description: 'The JSON files written next to sprite sheets: Kagero JSON v1 and JSON Hash.'
icon: brackets-curly
---

# Sprite sheet metadata

When a [sprite sheet](../export/export/sprite-sheets.md) is exported with metadata, Kagero writes `name.json` next to `name.png`. Both formats are UTF-8 JSON, written atomically. Coordinates are in pixels of the **output** image (after scaling); `(0, 0)` is the top-left corner.

The examples below come from a real export: two 16×16 frames, horizontal strip, spacing 2, padding 1, extrude 1, trim on.

## Kagero JSON (version 1)

```json
{
  "format": "kagero-sprite-sheet",
  "version": 1,
  "app": "Kagero 0.3.0",
  "image": "ember.png",
  "size": { "w": 24, "h": 16 },
  "scale": 1,
  "layout": "horizontal",
  "cell": { "w": 8, "h": 12 },
  "columns": 2,
  "rows": 1,
  "spacing": 2,
  "padding": 1,
  "extrude": 1,
  "frameCount": 2,
  "totalDuration": 250,
  "frames": [
    {
      "name": "ember_0001",
      "index": 0,
      "frame": { "x": 2, "y": 2, "w": 6, "h": 10 },
      "trimmed": true,
      "source": { "x": 5, "y": 4, "w": 6, "h": 10 },
      "sourceSize": { "w": 16, "h": 16 },
      "duration": 100
    },
    {
      "name": "ember_0002",
      "index": 1,
      "frame": { "x": 14, "y": 2, "w": 8, "h": 12 },
      "trimmed": true,
      "source": { "x": 4, "y": 2, "w": 8, "h": 12 },
      "sourceSize": { "w": 16, "h": 16 },
      "duration": 150
    }
  ]
}
```

| Field                           | Meaning                                                                        |
| ------------------------------- | ------------------------------------------------------------------------------ |
| `format`                        | Always `"kagero-sprite-sheet"`.                                                |
| `version`                       | Format version, currently `1`. Readers should refuse versions they don't know. |
| `app`                           | The Kagero version that wrote the file (informational).                        |
| `image`                         | File name of the sheet image, relative to the JSON file.                       |
| `size`                          | Sheet size.                                                                    |
| `scale`                         | Integer export scale (`1` = 100 %).                                            |
| `layout`                        | `horizontal`, `vertical`, `columns` or `rows`.                                 |
| `cell`                          | Size of every cell, without extrude.                                           |
| `columns`, `rows`               | Grid size.                                                                     |
| `spacing`, `padding`, `extrude` | As chosen in the dialog, in output pixels.                                     |
| `frameCount`, `totalDuration`   | Number of frames and the sum of durations in ms.                               |
| `frames`                        | Frames **in playback order**.                                                  |
| `frames[].name`                 | `base_0001`, … (same numbering as exported PNG files).                         |
| `frames[].index`                | Position of the frame in the Kagero document (0-based).                        |
| `frames[].frame`                | Where the frame's pixels are on the sheet (extrude border not included).       |
| `frames[].trimmed`              | `true` if transparent borders were cut off.                                    |
| `frames[].source`               | Which part of the full frame `frame` holds; `x`/`y` is the trim offset.        |
| `frames[].sourceSize`           | The full (scaled) frame size before trimming.                                  |
| `frames[].duration`             | Frame duration in milliseconds.                                                |

To draw frame _n_ at its original position: copy `frame` from the sheet to `(source.x, source.y)` inside a `sourceSize` box.

**Compatibility.** New optional fields may be added without changing `version`; removing or changing the meaning of a field increases it.

## JSON Hash

The widely used texture-atlas layout. Frames are an object keyed by name (written in playback order).

```json
{
  "frames": {
    "ember_0001": {
      "frame": { "x": 2, "y": 2, "w": 6, "h": 10 },
      "rotated": false,
      "trimmed": true,
      "spriteSourceSize": { "x": 5, "y": 4, "w": 6, "h": 10 },
      "sourceSize": { "w": 16, "h": 16 },
      "duration": 100
    },
    "ember_0002": { "...": "..." }
  },
  "meta": {
    "app": "https://github.com/senzukisu/kagero",
    "version": "0.3.0",
    "image": "ember.png",
    "format": "RGBA8888",
    "size": { "w": 24, "h": 16 },
    "scale": "1"
  }
}
```

* `spriteSourceSize` is the same as Kagero's `source`.
* `rotated` is always `false` — Kagero never rotates frames on the sheet.
* `meta.scale` is a string, as in other tools that write this format.
* `duration` (ms) is an extension that several engines read for animations.

Implementation: `src/export/metadata.rs`. Round-trip tests: `tests/export.rs` (`sheet_with_json_metadata_round_trips`).

---
description: "Sprite sheets for game engines: layouts, spacing, padding, trim, extrude and metadata."
icon: table-cells
---

# Sprite sheets

The **Sprite Sheet** tab (**File → Export Sprite Sheet…**) puts the chosen frames on one
image and, optionally, writes a JSON file that says where each frame is. The preview
shows the real sheet before you export.

## Layouts

| Layout | Arrangement |
|--------|-------------|
| **Horizontal strip** | All frames in one row. |
| **Vertical strip** | All frames in one column. |
| **Grid (fixed columns)** | You choose the number of columns; rows follow. |
| **Grid (fixed rows)** | You choose the number of rows; columns follow. |

Frames are placed left to right, top to bottom, in timeline order. Every frame gets a
cell of the **same size** so animations don't jump in game engines.

## Spacing, padding and extrude

```
padding
┌──────────────────────────────────┐
│  ┌─extrude─┐ spacing ┌─────────┐ │
│  │ frame 1 │         │ frame 2 │ │
│  └─────────┘         └─────────┘ │
└──────────────────────────────────┘
```

| Option | Range | Meaning |
|--------|-------|---------|
| **Spacing** | 0–256 px | Gap between cells. |
| **Padding** | 0–256 px | Empty border around the whole sheet. |
| **Extrude** | 0–16 px | Repeats each frame's edge pixels outward. Stops neighboring frames from bleeding in when an engine filters or scales the texture. |

Spacing, padding and extrude are in **output pixels** (after scaling).

## Trim

**Trim empty borders** cuts the transparent border off each frame. The cell size
becomes the largest trimmed frame, and the metadata records where each trimmed frame
came from, so engines can restore the original position. A completely empty frame is
kept as a single transparent pixel.

## Background

*Transparent* (default) or a solid *Color*, which also fills spacing and padding.

## Image format and size

The sheet is saved as **PNG** (recommended) or **lossless WebP**, both with full
transparency, at the chosen [scale](export.md#scale). The dialog shows the final size and
refuses sheets larger than **16 384 px** per side or 512 MiB before allocating
anything — reduce the scale, use a grid layout or export fewer frames.

## Metadata

| Option | File | For |
|--------|------|-----|
| **Kagero JSON** | `name.json` | Kagero's own versioned format: order, durations, layout. |
| **JSON Hash (atlas)** | `name.json` | The common texture-atlas layout read by many engines and tools (Phaser, PixiJS, and others). |
| **No metadata** | — | Just the image. |

Frame names follow the [file-name rules](export.md#file-names): `name_0001`,
`name_0002`, … Both formats are described field by field in
[Sprite sheet metadata](../developers/metadata-format.md).

{% hint style="info" %}
Packed (bin-packing) layouts that place frames of different sizes tightly are not
available in this version; every frame uses an equal cell.
{% endhint %}

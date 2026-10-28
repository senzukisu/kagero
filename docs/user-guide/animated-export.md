---
description: "Exporting animated GIFs: timing, looping, transparency and colors."
icon: film
---

# Animated GIF

Open the **Animation** tab of the Export dialog (**File → Export Animation as GIF…**).
Choose the frames (current, selected or all), the [scale](export.md#scale) and a
folder; Kagero writes `name.gif`. The preview in the dialog plays with the real
frame durations.

## Timing

Each frame keeps its own duration from the timeline. GIF stores delays in steps of
10 ms, so Kagero rounds to the nearest step, with a minimum of **20 ms** (many viewers
play shorter delays far too slowly). The dialog warns when rounding changes your
timing.

## Repeat

| Option | Result |
|--------|--------|
| **Loop forever** (default) | The animation repeats. |
| **Play once** | It stops on the last frame. |

## Transparency

GIF pixels are either fully transparent or fully opaque. Kagero handles that for you:

* **Transparent below alpha** (1–255, default 128): pixels with less alpha become
  transparent; the rest become opaque.
* **Soft edges**: *Keep color* uses the pixel's own color; *Blend over* mixes
  semi-transparent pixels with a color you choose — useful when you know the page or
  game background.
* Each frame fully replaces the previous one, so transparent areas never show leftovers
  from earlier frames.

The dialog warns when the image contains semi-transparent pixels.

## Colors

A GIF holds at most 256 colors; Kagero keeps one for transparency, leaving **255**.

* With 255 colors or fewer (almost all pixel art) every color is kept **exactly**.
* With more, the colors are reduced to 255 and the dialog says so beforehand.
  **Dither** (off by default) mixes nearby colors to hide banding; leave it off to keep
  flat pixel-art areas clean.

## Limits

* All frames have the same size (they always do in a Kagero document).
* A GIF can be at most 65 535 px per side; Kagero's general 16 384 px limit applies
  first.
* A single frame produces a still GIF; the dialog mentions it.

{% hint style="info" %}
Animated WebP is not supported in this version. Use GIF for animations, or a
[sprite sheet](sprite-sheets.md) for game engines.
{% endhint %}

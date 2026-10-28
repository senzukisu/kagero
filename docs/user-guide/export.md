---
description: "The Export dialog: PNG, JPEG and WebP images, frame choice, scale and file names."
icon: file-export
---

# Exporting images

Everything leaves Kagero through one **Export** dialog
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>, or **File → Export…**). It has three tabs:

| Tab | Produces | Page |
|-----|----------|------|
| **Image** | One file per frame: PNG, JPEG or WebP | this page |
| **Animation** | An animated GIF | [Animated GIF](animated-export.md) |
| **Sprite Sheet** | One image with all frames, plus optional JSON | [Sprite sheets](sprite-sheets.md) |

The dialog remembers its settings per tab and per format, so GIF can stay at ×4 while
PNG stays at ×1. <kbd>Ctrl</kbd>+<kbd>E</kbd> still exports the current frame as a PNG
in one step.

{% hint style="success" %}
Exporting never changes your project. Kagero takes a copy of the frames first and
encodes in the background, so you can keep drawing — or press **Cancel Export**.
{% endhint %}

## Frames and layers

| Option | What gets exported |
|--------|-------------------|
| **Current** | The frame you are on. |
| **Selected** | Frames picked in the timeline: <kbd>Ctrl</kbd>+click toggles a frame, <kbd>Shift</kbd>+click selects a range. With nothing picked, the current frame is used. |
| **All** | Every frame, in timeline order. |

**Layers:** *Visible layers (as shown)* merges what you see on the canvas; *Active layer
only* exports just the selected layer. Hidden layers and the onion skin are never
exported.

## Scale

Pick **100 %, 200 %, 300 %, 400 %** or **800 %**, or type any whole number (×1–×64).
Pixels are enlarged as sharp square blocks (nearest neighbor) — never blurred. The
dialog shows the output size and refuses sizes above **16 384 px** per side or 512 MiB
of pixel data before anything is written.

## Formats

| Format | Transparency | Compression | Use it for |
|--------|--------------|-------------|------------|
| **PNG** | Full alpha | Lossless | Game sprites, anything you will edit again. **Recommended.** |
| **JPEG** | None — transparent pixels become the background color | Lossy | Previews and sharing where file size matters. |
| **WebP (lossless)** | Full alpha | Lossless | Web pages; usually smaller than PNG. |

### PNG

Pixel-exact: every color and alpha value is written as is.

### JPEG

* **Quality** 1–100 (default 90).
* **Background**: white, black or any color — JPEG has no transparency, so transparent
  and semi-transparent pixels are blended over it.

{% hint style="warning" %}
JPEG uses lossy compression and may introduce artifacts into pixel art. PNG is
recommended for lossless game sprites.
{% endhint %}

### WebP

Kagero writes **lossless** WebP with transparency. Lossy WebP quality and **animated
WebP are not available in this version** — the dialog shows them as unavailable rather
than writing a file that is not what you asked for. Use GIF for animations.

## File names

* **Base name** — defaults to the project name. Characters Windows does not allow in
  file names are replaced.
* **Several frames** get a number: `sprite_0001.png`, `sprite_0002.png`, … The
  **Digits** option (1–8) sets the padding.
* **One frame** is saved without a number (`sprite.png`) unless you tick *Number the
  file even when exporting one frame*.

Before writing, Kagero checks the folder. If any file already exists it asks once:
**Replace** or **Cancel**. Files are written to a temporary name first and then moved
into place, so a failed export never leaves a half-written file.

When the export finishes the dialog lists the files; **Open Folder** shows them in
Explorer (or tick *Open the folder after exporting*).

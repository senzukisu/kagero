---
description: "The tools, brush size and moving around the canvas."
icon: pencil
---

# Tools & canvas

<figure><img src="../.gitbook/assets/tool-icons.png" alt="Kagero's tool icons: pencil, eraser, fill, eyedropper, line, rectangle, filled rectangle, pan, rectangle select"><figcaption></figcaption></figure>

| Tool | Key | What it does |
|------|-----|--------------|
| **Pencil** | <kbd>B</kbd> | Paints with a square brush. Fast strokes are joined into continuous lines. |
| **Eraser** | <kbd>E</kbd> | Makes pixels fully transparent. |
| **Fill** | <kbd>G</kbd> | Fills the connected area of exactly the same color on the active layer. |
| **Eyedropper** | <kbd>I</kbd> | Picks a color from the merged image or from the active layer only. |
| **Line** | <kbd>L</kbd> | Drag to draw a pixel-perfect line. |
| **Rectangle** | <kbd>U</kbd> | Drag to draw an outline. |
| **Filled Rectangle** | <kbd>Shift</kbd>+<kbd>U</kbd> | Drag to draw a solid rectangle. |
| **Pan** | <kbd>H</kbd> | Drag to move the view. <kbd>Space</kbd>+drag works with any tool. |
| **Rectangle Select** | <kbd>M</kbd> | Select pixels to move, copy, flip or rotate — see [Selection & transforms](selection.md). |

* **Left** button = primary color, **right** button = secondary color.
* Pencil, Eraser, Line and Rectangle use a square brush from **1 to 32 px** (tool
  options bar). An amber outline on the canvas shows exactly which pixels will change.
* Want cleaner 1 px freehand lines? Turn on **Pixel-perfect pencil** in
  [Settings → Experimental](../reference/settings.md#experimental-features).
* One stroke — from mouse down to mouse up — is **one undo step**. Shapes preview
  while you drag, and <kbd>Esc</kbd> cancels them.
* Drawing outside the image is ignored; the stroke continues when you come back in.

{% hint style="info" %}
You can't draw on a **hidden** layer or while the animation preview plays — the status
bar tells you why instead of painting invisibly.
{% endhint %}

## Moving around

| Action | How |
|--------|-----|
| Zoom at the cursor | Mouse wheel |
| Zoom in / out | <kbd>+</kbd> / <kbd>−</kbd> |
| Actual size · fit to window | <kbd>1</kbd> · <kbd>0</kbd> |
| Center the image | <kbd>Home</kbd> |
| Pan | <kbd>Space</kbd>+drag, middle-drag or the Pan tool |
| Pixel grid | <kbd>Ctrl</kbd>+<kbd>G</kbd> (visible from 600 % zoom) |

The transparency checkerboard is aligned to the image: one square is one pixel, at any
zoom and display scale.

## The Image menu

**Flip Horizontal / Vertical** and **Rotate 90° / 180° / 270°** work on the selection,
or on the whole active layer when nothing is selected.

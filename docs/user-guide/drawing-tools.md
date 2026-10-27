---
description: "Pencil, eraser, fill, shapes, eyedropper and pan."
icon: pencil
---

# Drawing tools

| Tool | Key | What it does |
|------|-----|--------------|
| **Pencil** | <kbd>B</kbd> | Paints with a square brush. Fast strokes are joined into continuous lines. |
| **Eraser** | <kbd>E</kbd> | Makes pixels fully transparent. |
| **Fill** | <kbd>G</kbd> | Fills the connected area of exactly the same color on the active layer. |
| **Eyedropper** | <kbd>I</kbd> | Picks a color from the merged image or the active layer only. |
| **Line** | <kbd>L</kbd> | Drag to draw a pixel-perfect line. |
| **Rectangle** | <kbd>U</kbd> | Drag to draw an outline. |
| **Filled Rectangle** | <kbd>Shift</kbd>+<kbd>U</kbd> | Drag to draw a solid rectangle. |
| **Pan** | <kbd>H</kbd> | Drag to move the view. <kbd>Space</kbd>+drag works with any tool. |

## Colors

* **Left** button = primary color, **right** button = secondary color.
* Click a palette swatch with the left/right button to set primary/secondary.
* Click the big swatches to open a color picker (with alpha); <kbd>X</kbd> swaps them.
* **Add primary** puts the current color into the palette; middle-click a swatch to
  remove it; **Reset** restores the Kagero palette. The palette is saved with your settings.

## Brush size

Pencil, Eraser, Line and Rectangle use a square brush from **1 to 32 px** (tool options
bar). The accent outline on the canvas shows exactly which pixels will be painted.

## Shapes and strokes

* One stroke — from mouse down to mouse up — is **one undo step**.
* Lines and rectangles preview while you drag. <kbd>Esc</kbd> cancels the shape.
* Drawing outside the image is ignored; the stroke continues when you come back in.

{% hint style="info" %}
You can't draw on a **hidden** layer — Kagero tells you in the status bar instead of
painting invisibly. Show the layer first (eye icon).
{% endhint %}

## Navigating the canvas

| Action | How |
|--------|-----|
| Zoom at the cursor | Mouse wheel |
| Zoom in / out | <kbd>+</kbd> / <kbd>-</kbd> |
| Actual size / fit to window | <kbd>1</kbd> / <kbd>0</kbd> |
| Center | <kbd>Home</kbd> |
| Pan | <kbd>Space</kbd>+drag, middle-drag or the Pan tool |
| Pixel grid | <kbd>Ctrl</kbd>+<kbd>G</kbd> (visible from 600 %) |

The transparency checkerboard is aligned to image pixels: one square is one pixel.

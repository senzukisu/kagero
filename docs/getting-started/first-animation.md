---
description: "Draw a tiny bouncing sprite and export it — in about five minutes."
icon: wand-magic-sparkles
---

# Your first animation

This walkthrough makes a small bouncing ball in 4 frames. It touches every core feature
once: drawing, layers, frames, onion skin, preview, saving and export.

{% stepper %}
{% step %}
### Create an image

Click **Create New** on the start screen (or press <kbd>Ctrl</kbd>+<kbd>N</kbd>). Pick the
**32×32** preset with a **Transparent** background and click **Create**.
{% endstep %}

{% step %}
### Draw the first pose

Press <kbd>B</kbd> for the Pencil, choose an orange from the palette on the right and
draw a small ball near the top of the canvas. Use <kbd>G</kbd> (Fill) to fill it.

{% hint style="info" %}
The **left** mouse button paints with the primary color, the **right** button with the
secondary color. <kbd>X</kbd> swaps them.
{% endhint %}
{% endstep %}

{% step %}
### Add frames

Press <kbd>Alt</kbd>+<kbd>D</kbd> to **duplicate** the frame. In the copy, erase the ball
(<kbd>E</kbd>) and redraw it a bit lower. Repeat until the ball touches the ground in
frame 3, then duplicate frame 2 as frame 4 so it bounces back up.

Each frame card in the timeline at the bottom shows a live thumbnail.
{% endstep %}

{% step %}
### Turn on onion skin

Press <kbd>O</kbd>. The previous frame appears in a **warm** tint and the next one in a
**cool** tint, so you can space the motion evenly. Onion skin is only a drawing aid — it
is never saved or exported.
{% endstep %}

{% step %}
### Preview

Press <kbd>Enter</kbd> to play the loop, <kbd>Enter</kbd> or <kbd>Esc</kbd> to stop.
Change a frame's timing in the **Duration** field of the timeline (e.g. make the
bottom frame shorter for a snappier bounce).
{% endstep %}

{% step %}
### Save and export

* <kbd>Ctrl</kbd>+<kbd>S</kbd> saves a `.kgr` project with all frames and layers.
* <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> exports every frame as
  `name_0001.png`, `name_0002.png`, … into a folder you pick.
{% endstep %}
{% endstepper %}

Next: [a tour of the workspace](interface-tour.md).

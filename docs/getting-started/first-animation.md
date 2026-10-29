---
description: >-
  Draw a small animated sprite and export it as a GIF and a sprite sheet — in
  about ten minutes.
icon: wand-magic-sparkles
---

# Quick start

This walkthrough makes a small bouncing ball in four frames and exports it. It touches every core feature once: drawing, frames, onion skin, preview, saving and export.

{% stepper %}
{% step %}
### Create an image

Click **Create New** on the start screen (or press <kbd>Ctrl</kbd>+<kbd>N</kbd>). Pick the **32×32** preset, keep the **Transparent** background and click **Create**.

<figure><img src="../.gitbook/assets/new-image.png" alt="The New Image dialog with size, background and presets" width="420"><figcaption></figcaption></figure>
{% endstep %}

{% step %}
### Draw the first pose

Press <kbd>B</kbd> for the Pencil, pick an orange in the palette on the right and draw a small ball near the top. Press <kbd>G</kbd> and click inside it to fill it.

{% hint style="info" %}
The **left** mouse button paints with the primary color, the **right** button with the secondary color. <kbd>X</kbd> swaps them. Zoom with the mouse wheel.
{% endhint %}
{% endstep %}

{% step %}
### Add frames

Press <kbd>Alt</kbd>+<kbd>D</kbd> to **duplicate** the frame. In the copy, select the ball with <kbd>M</kbd> (drag around it) and nudge it down with the arrow keys. Repeat until the ball touches the ground in frame 3, then duplicate frame 2 as frame 4 so it bounces back up.

Every frame appears as a card with a live thumbnail in the timeline at the bottom.
{% endstep %}

{% step %}
### Turn on onion skin

Press <kbd>O</kbd>. The previous frame shows in a **warm** tint and the next one in a **cool** tint, so you can space the motion evenly. Onion skin is only a drawing aid — it is never saved or exported.
{% endstep %}

{% step %}
### Play it

Press <kbd>Enter</kbd> to play the loop; <kbd>Enter</kbd> or <kbd>Esc</kbd> stops it. Give a frame more or less time in the timeline's **Duration** field — a shorter bottom frame makes a snappier bounce.

<figure><img src="../.gitbook/assets/timeline.png" alt="The timeline: playback buttons, frame counter, duration, onion skin and frame cards"><figcaption><p>The timeline with eight frames; the current one has an amber border.</p></figcaption></figure>
{% endstep %}

{% step %}
### Save and export

* <kbd>Ctrl</kbd>+<kbd>S</kbd> saves a `.kgr` project with all frames and layers.
* <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> opens the **Export** dialog. Choose the **Animation** tab for a GIF, or **Sprite Sheet** for a game engine, pick a scale such as 400 % and click **Export**.
{% endstep %}
{% endstepper %}

**Next:** [the workspace](interface-tour.md) explains every part of the window, and [Export](../export/export/) covers all export options.

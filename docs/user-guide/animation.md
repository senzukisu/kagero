---
description: Frames, the timeline, preview and onion skin.
icon: film
---

# Animation

Kagero animates frame by frame. It is a pixel-art tool, not a motion editor: there are
no tracks, bones or tweens — you draw each frame.

![Onion skin and the timeline](../images/screenshot-onion-skin.png)

## Frames

Every frame has its **own image on each layer** and its own **duration**. The layer list
is shared by all frames (see [Layers](layers.md#layers-and-frames)).

| Action | Where | Shortcut |
|--------|-------|----------|
| New empty frame (after the current) | Timeline **+**, Frame menu | <kbd>Alt</kbd>+<kbd>N</kbd> |
| Duplicate frame (independent copy) | Timeline, Frame menu, right-click a card | <kbd>Alt</kbd>+<kbd>D</kbd> |
| Delete frame | Timeline, Frame menu, right-click a card | — |
| Move frame left / right | Timeline arrows, Frame menu | — |
| Select a frame | Click its card | <kbd>,</kbd> <kbd>.</kbd> <kbd>←</kbd> <kbd>→</kbd> |
| First frame | Timeline, Frame menu | <kbd>Ctrl</kbd>+<kbd>Home</kbd> |
| Set duration | **Duration** field in the timeline | — |

* Every frame operation can be undone. If you undo a stroke that was made on another
  frame, Kagero jumps to that frame so you can see what changed.
* Deleting a frame that has drawings asks for confirmation. The last frame can't be deleted.
* Durations range from **10 ms to 60 s**; new frames copy the current frame's duration.
* Up to 1000 frames, within the memory budget.

## The timeline

The bottom panel has the playback controls, frame actions, duration and onion-skin
settings, and a scrollable strip of **frame cards** — a live thumbnail, the frame
number and its duration. The current frame has an amber border.

* Drag the panel's top edge to resize it; <kbd>F7</kbd> collapses it to the controls row.
* Right-click a card for Duplicate, Move and Delete.
* Thumbnails update when a stroke ends.

## Preview

Press <kbd>Enter</kbd> (or the play button) to loop the animation from the current frame.
<kbd>Enter</kbd> or <kbd>Esc</kbd> stops on the frame that was showing.

{% tabs %}
{% tab title="Frame durations" %}
Each frame stays on screen for its own duration — use this to give key poses more time.
{% endtab %}

{% tab title="Fixed FPS" %}
Every frame lasts the same time (1–60 fps), ignoring stored durations — handy for
checking a rough animation at a game's frame rate.
{% endtab %}
{% endtabs %}

* Timing follows the real clock, so a busy computer never slows the animation down.
* While the preview plays, drawing is paused — clicking the canvas shows a hint.
  Editing commands stop the preview first.

![Preview playing](../images/screenshot-preview.png)

## Onion skin

Press <kbd>O</kbd> (or the onion button) to see neighbor frames over the current one:

* the **previous** frame in a **warm** tint,
* the **next** frame in a **cool** tint,
* opacity from 5 % to 90 % (default 35 %).

The first frame has no previous overlay and the last has no next one — it doesn't wrap
around. A badge on the canvas reminds you that onion skin is on.

{% hint style="success" %}
Onion skin is purely visual. It never changes your pixels, is never saved into the
project and never appears in exports. It is hidden while the preview plays.
{% endhint %}

## Export

* <kbd>Ctrl</kbd>+<kbd>E</kbd> — the current frame as PNG.
* <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> — all frames as numbered PNGs.

See [Projects, import and export](files-and-export.md#exporting-png).

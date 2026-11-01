---
description: "Layers with opacity and locks, duplicate and merge, canvas size and crop."
icon: layer-group
---

# Layers

Layers are listed **top layer first** below the colors. The active layer has an amber
bar; everything you draw goes there.

| Action | How |
|--------|-----|
| New layer (above the active one) | **+** or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>N</kbd> |
| Duplicate ("Layer copy") | Duplicate button or <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>D</kbd> |
| Merge Down (into the layer below) | Merge button or <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>M</kbd> |
| Select | Click the row |
| Show / hide | Eye icon |
| Rename | Double-click, <kbd>F2</kbd> or right-click → Rename; <kbd>Enter</kbd> saves, <kbd>Esc</kbd> cancels |
| Move up / down | Arrow buttons or <kbd>Ctrl</kbd>+<kbd>]</kbd> / <kbd>Ctrl</kbd>+<kbd>[</kbd> |
| Delete | Trash button or right-click → Delete |

Right-clicking a layer also offers Duplicate, Merge Down, Show/Hide, Lock and Alpha
Lock. Every layer action can be undone.

## Opacity and locks

Below the layer buttons are the active layer's settings:

* **Opacity** (0–100 %): how strongly the layer shows on the canvas, in the preview and
  in every export. It never changes the layer's pixels, so you can always go back to
  100 %. Dragging the slider is one undo step. Rows show the opacity when it is below
  100 %.
* **Lock**: the layer can't be drawn on or changed.
* **Alpha Lock**: only pixels that already have color can be recolored. Empty pixels
  stay empty and the transparency stays as it is, which is handy for shading inside a
  shape.

**Merge Down** combines the layer with the one below in every frame, exactly as it
looks, opacity included. It is one undo step.

## Layers and frames

The **list of layers is shared by all frames**, but every frame has its **own image**
on each layer.

* Adding, deleting, renaming, reordering, hiding or locking a layer and changing its
  opacity affects every frame.
* Drawing changes only the current frame.

{% hint style="info" %}
A typical setup: a **Ground** layer that is the same in every frame (duplicating
frames copies it) and a **Character** layer that changes from frame to frame.
{% endhint %}

## Canvas size and crop

* **Image → Canvas Size…** (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd>) gives the
  canvas a new width and height without scaling.
  * The **3×3 anchor** decides where the old image sits: centered, in a corner or
    along an edge.
  * A preview shows the result, and a warning appears when pixels would be cut off.
* The **Crop** tool (<kbd>C</kbd>):
  * Drag a frame, then adjust it by its handles or drag it inside.
  * <kbd>Enter</kbd> (or **Crop** in the tool options) crops; <kbd>Esc</kbd> cancels.
  * **Crop to Selection** crops to the selection's bounding box.

Both work on every layer and frame at once, keep the selection where it was (clipped
to the new canvas), and are one undo step.

## Limits

Up to 64 layers. Layers × frames × width × height × 4 bytes may not exceed 512 MiB;
Kagero tells you when an action would go over it.

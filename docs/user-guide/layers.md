---
description: Stack, hide, rename and reorder layers.
icon: layer-group
---

# Layers

Layers are listed **top layer first** in the panel on the right. The active layer has an
amber bar; everything you draw goes there.

| Action | How |
|--------|-----|
| New layer (above the active one) | **+** button or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>N</kbd> |
| Select | Click the row |
| Show / hide | Eye icon |
| Rename | Double-click, <kbd>F2</kbd> or right-click → Rename. <kbd>Enter</kbd> saves, <kbd>Esc</kbd> cancels |
| Move up / down | Arrow buttons or <kbd>Ctrl</kbd>+<kbd>]</kbd> / <kbd>Ctrl</kbd>+<kbd>[</kbd> |
| Delete | Trash button or right-click → Delete |

Every layer action can be undone with <kbd>Ctrl</kbd>+<kbd>Z</kbd>.

## Layers and frames

The **list of layers is shared by all frames**, but every frame has its **own image**
on each layer.

* Adding, deleting, renaming, reordering or hiding a layer affects every frame.
* Drawing on a layer changes only the current frame.
* Switching frames never makes a hidden layer visible or changes the selected layer.

{% hint style="info" %}
Typical setup: a **Background** layer that is the same in every frame (duplicate frames
to copy it) and a **Character** layer that changes from frame to frame.
{% endhint %}

## Limits

Up to 64 layers. Very large documents are limited by a memory budget: layers × frames ×
width × height × 4 bytes may not exceed 512 MiB. Kagero tells you when an action would
go over it.

---
description: "Rectangle selection: move, copy, cut, paste, duplicate, flip and rotate."
icon: vector-square
---

# Selection and transforms

Press <kbd>M</kbd> (or pick **Rectangle Select** in the toolbar) to work with a part of
the active layer.

## Making a selection

| Action | How |
|--------|-----|
| New selection | Drag on the canvas. Marching ants show the outline. |
| Select all | <kbd>Ctrl</kbd>+<kbd>A</kbd> |
| Deselect | <kbd>Esc</kbd>, or click outside the selection |
| Move only the outline | <kbd>Ctrl</kbd>+drag inside the selection |

The selection is clipped to the image and is never saved in the project.

## Working with the selected pixels

| Action | How |
|--------|-----|
| Move | Drag inside the selection, or nudge with arrows (<kbd>Shift</kbd>: 10 px). A ghost shows the pixels while you drag. |
| Copy / Cut | <kbd>Ctrl</kbd>+<kbd>C</kbd> / <kbd>Ctrl</kbd>+<kbd>X</kbd> |
| Paste | <kbd>Ctrl</kbd>+<kbd>V</kbd> — at the original place, moved inside the image if needed; the pasted pixels become the selection. |
| Delete | <kbd>Delete</kbd> makes the selected pixels transparent. |
| Duplicate | <kbd>Ctrl</kbd>+<kbd>D</kbd> places a copy next to the selection. |
| Flip | <kbd>Shift</kbd>+<kbd>H</kbd> horizontal, <kbd>Shift</kbd>+<kbd>V</kbd> vertical |
| Rotate | <kbd>Ctrl</kbd>+<kbd>R</kbd> 90° clockwise, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> counterclockwise; 180° in the **Image** menu or tool options |

* Transparent pixels in a moved or pasted block don't erase what's below.
* Rotating a non-square selection keeps its center; parts outside the image are cut off.
* With **no selection**, flip and rotate apply to the whole active layer.
* Only the active layer of the current frame changes; other layers and frames are
  never touched. Hidden layers and the animation preview block edits, as when drawing.
* Every operation is **one undo step**, and undo restores the selection outline too.

The clipboard lives inside Kagero (it survives opening another document) and holds
pixels with transparency.

---
description: "Projects (.kgr), PNG import and safe saving."
icon: floppy-disk
---

# Projects and import

## Projects (`.kgr`)

A `.kgr` project keeps **everything**: frames, durations, layers, their order, names and
visibility, and every pixel including transparency.

| Action | Shortcut |
|--------|----------|
| New | <kbd>Ctrl</kbd>+<kbd>N</kbd> |
| Open (`.kgr` or `.png`) | <kbd>Ctrl</kbd>+<kbd>O</kbd> |
| Save / Save As | <kbd>Ctrl</kbd>+<kbd>S</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> |
| Close document | <kbd>Ctrl</kbd>+<kbd>W</kbd> |

You can also open a file by passing it to `kagero.exe` (for example with
*Open with → Kagero* in Explorer).

{% hint style="success" %}
**Saving is safe.** Kagero writes to a temporary file and only then replaces the old
project, so a crash or a full disk never leaves a half-written file behind.
{% endhint %}

* The title bar shows **•** and the status bar **Unsaved changes** when there is
  something to save. Undoing back to the saved state clears it.
* Closing the window, opening another file or creating a new one with unsaved changes
  asks **Save / Don't Save / Cancel**.

### Projects from Kagero 0.1

Open them as usual. They become a one-frame animation with all layers intact; saving
writes the new format. Nothing to convert by hand.

## Importing PNG

<kbd>Ctrl</kbd>+<kbd>O</kbd> a PNG (up to 2048×2048) to edit it as a one-layer document.
<kbd>Ctrl</kbd>+<kbd>S</kbd> then asks where to save the `.kgr` project — the original PNG
is never overwritten.

## Exporting

| Export | Shortcut | Result |
|--------|----------|--------|
| **Current frame as PNG** | <kbd>Ctrl</kbd>+<kbd>E</kbd> | One PNG of the visible layers of the current frame. |
| **Export dialog** | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> | PNG / JPEG / WebP images, animated GIF and sprite sheets, at any integer scale. |

See [Exporting images](export.md), [Animated GIF](animated-export.md) and
[Sprite sheets](sprite-sheets.md).

## Limits

| Limit | Value |
|-------|-------|
| Image size | 1×1 to 2048×2048 |
| Layers | 64 |
| Frames | 1000 |
| Memory budget | 512 MiB of pixel data |

Damaged or unsupported files show a clear error and never crash Kagero. The technical
format is described in [The .kgr project format](../developers/project-format.md).

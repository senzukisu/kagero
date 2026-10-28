---
description: "What changed in each version."
icon: clock-rotate-left
---

# Changelog

The complete, always up-to-date list is in
[CHANGELOG.md on GitHub](https://github.com/senzukisu/kagero/blob/main/CHANGELOG.md).

## 0.3.7 Beta — hotfix (2026-10-29)

* **Fix**: the Export dialog could be taller than a 1080p screen, with its Export
  button out of reach. It now fits, and its options scroll when space is short.

[Download 0.3.7](https://github.com/senzukisu/kagero/releases/tag/v0.3.7)

## 0.3.6 Beta — the window update (2026-10-29)

* **Custom title bar** with the flame, menus, title and pixel window buttons; drag to
  move, double-click to maximize, resize from the edges.
* **Classic font is back** as the default, as large as in 0.3.0. Kagero Pixel (now
  larger and wider) is an experimental option in Settings → General.
* **Fix**: Kagero now really opens maximized, filling the screen.

[Download 0.3.6](https://github.com/senzukisu/kagero/releases/tag/v0.3.6)

## 0.3.5 Beta — pixel interface (2026-10-28)

* **Pixel interface**: square corners, hard drop shadows, bevelled buttons — Kagero now
  looks like the art you make in it.
* **Kagero Pixel**, Kagero's own pixel font (Latin, Cyrillic and interface symbols),
  about as large as before and always sharp at 100–200 % display scale.
* **New icons**: all 31 redrawn as colorful 16×16 pixel art; disabled icons fade to gray.
* **New About window** with the flame banner, links and shortcut keycaps.
* **Fix**: Discord Rich Presence now shows the Kagero flame and activity icons.
* **Fix**: Kagero now opens maximized (and remembers if you prefer a window, which then
  opens centered) instead of a small window off to the side.

[Download 0.3.5](https://github.com/senzukisu/kagero/releases/tag/v0.3.5)

## 0.3.0 Beta — export & sprite workflow (2026-10-28)

* **Export dialog** (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>) with Image, Animation
  and Sprite Sheet tabs, live preview, background export with progress and cancel.
* **PNG, JPEG, lossless WebP**; integer scale 100–800 % or custom; current, selected
  or all frames; numbered names with overwrite confirmation.
* **Animated GIF** with real durations, loop or play-once, transparency threshold and
  exact colors (dithering optional).
* **Sprite sheets**: strips and grids, spacing, padding, trim, extrude, background,
  **Kagero JSON** and **JSON Hash** metadata.
* **Rectangle selection** (<kbd>M</kbd>): move, copy/cut/paste, delete, duplicate,
  flip and rotate — all undoable.
* **Palettes**: several palettes, GPL/HEX import and export, click-to-copy HEX, RGBA
  readout, recent colors, built-in Ember, Night Sakura and Misty Forest.
* `.kgr` format unchanged — 0.3 opens and writes the same projects as 0.2.

[Download 0.3.0](https://github.com/senzukisu/kagero/releases/tag/v0.3.0)

## 0.2.0 Beta — the animation update (2026-10-27)

* **Animation**: frames with per-frame layer images and durations; new, duplicate,
  delete and move frames — all undoable.
* **Timeline** with live thumbnails, resizable and collapsible.
* **Preview** in real time by frame durations or fixed FPS.
* **Onion skin** with warm/cool tints and adjustable opacity.
* **Export** the current frame or all frames as PNG.
* **`.kgr` format 2**; projects from 0.1 open automatically.
* **Updater** with signature verification, rollback and restore.
* **Discord Rich Presence**, private by default.
* Settings window, remembered layout, new shortcuts, readable pressed buttons,
  pixel-exact checkerboard.

[Download 0.2.0](https://github.com/senzukisu/kagero/releases/tag/v0.2.0)

## 0.1.0 Alpha (2026-10-27)

First public alpha: drawing tools, layers, undo/redo, `.kgr` projects, PNG import and
export, safe saving.

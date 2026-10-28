---
description: "What's done and what comes next."
icon: map
---

# Roadmap

## 0.1.0 Alpha — done

Single-image editor: tools, layers, undo/redo, `.kgr` v1, PNG import/export, safe saving.

## 0.2.0 Beta — the animation update — done

- [x] Frames with per-frame layer images and durations; stable frame ids
- [x] New / duplicate / delete / move frames, all undoable
- [x] Timeline with cached real thumbnails, resizable and collapsible
- [x] Real-time looping preview (frame durations or fixed FPS)
- [x] Onion skin (previous/next, opacity), never saved or exported
- [x] `.kgr` format v2 with automatic migration of v1 projects
- [x] Export current frame / all frames as PNG
- [x] GitHub updater: checks, prereleases, download, SHA-256 + minisign verification,
      helper-based install with backup, rollback and restore
- [x] Discord Rich Presence with privacy settings
- [x] Settings window, persistent workspace layout, new shortcuts, readable pressed states

## 0.3.6 Beta — the window update (this release)

- [x] Custom pixel title bar with window buttons, drag, double-click and edge resize
- [x] Classic font by default; Kagero Pixel as an experimental option
- [x] Reliable maximized start

## 0.3.5 Beta — pixel interface — done

- [x] Kagero Pixel font, pixel-style frames, bevels and hard shadows
- [x] 16×16 colored pixel icons
- [x] Opens maximized; remembers windowed mode
- [x] Redesigned About window; Discord presence images

## 0.3.0 Beta — export & sprite workflow — done

- [x] One Export dialog with Image / Animation / Sprite Sheet tabs, preview, progress
      and cancel; settings remembered per format
- [x] PNG (pixel-exact), JPEG (quality, background), lossless WebP
- [x] Integer export scale 100–800 % and custom (nearest neighbor)
- [x] Current / selected / all frames; numbered file names; overwrite confirmation
- [x] Animated GIF: durations, loop/once, transparency threshold, exact palette ≤ 255
      colors, optional dithering
- [x] Sprite sheets: strips and grids, spacing, padding, trim, extrude, background,
      Kagero JSON and JSON Hash metadata
- [x] Rectangle selection: move, copy/cut/paste, delete, duplicate, flip, rotate — all undoable
- [x] Multiple palettes, GPL/HEX import and export, recent colors, built-in Ember,
      Night Sakura and Misty Forest

Not in 0.3.0: animated WebP and lossy WebP (the WebP encoder Kagero uses is
lossless-only), packed (bin-packing) sprite sheets.

## 0.4 — everyday comfort

- Kagero Pixel as the default interface font once it is polished

- Canvas resize/crop
- Layer opacity, duplicate and merge down
- Autosave to a recovery folder and restore after a crash
- Frame tags/ranges for previewing and exporting a part of the animation
- Lasso / magic-wand selection
- Animated WebP and packed sprite sheets
- Executable icon and installer

## Later

- Tilemap mode, custom brushes, symmetry, dithering tools
- Linux and macOS builds and update packages

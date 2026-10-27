---
description: What's done and what comes next.
icon: map
---

# Roadmap

## 0.1.0 Alpha — done

Single-image editor: tools, layers, undo/redo, `.kgr` v1, PNG import/export, safe saving.

## 0.2.0 Beta — the animation update (this release)

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

Known limitations of the beta: no GIF/sprite-sheet export, no selection tools, no layer
opacity, Windows-only packages, no automatic crash recovery.

## 0.3 — everyday comfort

- GIF and sprite-sheet export, PNG export at integer scale (×2, ×4, …)
- Selection (rectangle), move, copy/paste; flip/rotate; canvas resize/crop
- Layer opacity, duplicate and merge down
- Autosave to a recovery folder and restore after a crash
- Frame tags/ranges for previewing a part of the animation
- Palette import/export (GPL, hex lists); executable icon and installer

## Later

- Tilemap mode, custom brushes, symmetry, dithering tools
- Linux and macOS builds and update packages

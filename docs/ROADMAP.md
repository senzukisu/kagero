# Roadmap

## 0.1.0 Alpha (this release)

- [x] Start screen with Create New, Open Project and recent projects
- [x] Documents from 1×1 to 2048×2048, transparent/white/black background
- [x] Tools: Pencil, Eraser, Fill, Eyedropper, Line, Rectangle, Filled Rectangle, Pan
- [x] Brush size 1–32 px; eyedropper from merged image or active layer
- [x] Canvas: nearest-neighbor zoom 25%–9600%, wheel zoom at cursor, Space/middle-drag pan,
      checkerboard, pixel grid, centering, fit, partial texture updates
- [x] Layers: add, delete, rename, reorder, select, show/hide, alpha compositing
- [x] Undo/Redo for every document change, bounded by count and memory
- [x] `.kgr` projects (ZIP + JSON + PNG layers), versioned and validated
- [x] PNG import and flattened PNG export with transparency
- [x] Safe saving (temporary file + rename), unsaved-changes prompts, window close guard
- [x] Keyboard shortcuts, status bar, error dialogs, log file
- [x] Persistent settings: recent files, palette, grid, last new-image size

Not in 0.1.0 by design: autosave/crash recovery, animation, onion skin, sprite sheets,
tiles, plugins, scripting, cloud features, color profiles, effects, advanced brushes.

## 0.2 — everyday comfort

- Autosave to a recovery folder and restore after a crash
- Selection (rectangle), move, copy/paste
- Layer opacity and duplicate/merge down
- Canvas resize/crop, flip/rotate
- PNG export at integer scale (×2, ×4, …)
- Palette import/export (GPL, hex lists)
- Windows executable icon and installer/zip release

## 0.3 — animation foundation

- Frames and a simple timeline
- Onion skinning
- Sprite-sheet export

## Later

- Tilemap mode, custom brushes, symmetry, dithering tools
- Linux and macOS builds

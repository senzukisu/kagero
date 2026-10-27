# Changelog

All notable changes to Kagero are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [0.1.0] Alpha — 2026-10-27

First public alpha.

### Added

- Start screen with Create New, Open Project and recent projects.
- New images from 1×1 to 2048×2048 with transparent, white or black background.
- Tools: Pencil, Eraser, Fill, Eyedropper, Line, Rectangle, Filled Rectangle, Pan;
  brush size 1–32; eyedropper samples the merged image or the active layer.
- Canvas with nearest-neighbor zoom (25%–9600%), zoom at cursor, Space/middle-drag pan,
  transparency checkerboard, pixel grid, center and fit.
- Layers: add, delete, rename, reorder, select, show/hide, alpha compositing.
- Undo/Redo for all document changes, limited to 256 steps / 256 MiB.
- `.kgr` project format v1 (ZIP + JSON + PNG layers) with validation.
- PNG import and flattened PNG export with transparency.
- Atomic saving, unsaved-changes prompts, close-window guard, error dialogs, log file.
- Persistent settings: recent files, palette, grid toggle, last image size.
- Kagero visual theme and original pixel icons.

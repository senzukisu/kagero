# Roadmap

Kagero is in **beta**. This page shows what comes next, in which order, and why. It is a
plan, not a promise. Versions can change when something turns out harder, or more
important, than expected. The [changelog](changelog.md) lists what each released version
actually contains.

> [!NOTE]
> **Current version: 0.4.3 Beta.** Next up: **0.5 — Precise drawing.**

## How versions work

| Kind | Example | What it brings |
|------|---------|----------------|
| **Feature version** | 0.5, 0.6 … | One theme, a handful of features around it. |
| **Fix version** | 0.4.1, 0.4.3 | Bug fixes, speed, small polish, tests. No new file format. |
| **1.0** | — | The first stable version: everything on this page up to 1.0 is done. |

Every version, big or small, keeps these promises:

- **Your projects keep opening.** Every Kagero opens every older `.kgr` project. A new
  version writes a newer format only when a project uses something the old format can't
  hold, so ordinary projects still open in the previous version.
- **Nothing gets lost.** Saving is atomic, autosave and recovery stay on, and every edit
  can be undone.
- **No placeholders.** A button or menu item ships only when it works.
- **Updates are safe.** Each release is signed, and the updater keeps the previous
  version to roll back to.
- **Plugins keep working.** A plugin written for API 1 runs in every 0.x and 1.x version.

## The plan to 1.0

### 0.5 — Precise drawing

Drawing should do exactly what you expect, especially inside a selection.

- **All tools respect the selection.** Pencil, eraser, shapes and symmetry draw only
  inside it, as Fill already does.
- **Move any selection.** Drag the pixels of a Lasso or Magic Wand selection, not only
  a rectangle.
- **Brushes are kept.** Custom brushes are saved between sessions, in a small brush
  library.
- **Pixel-perfect pencil** leaves the Experimental page and becomes a normal pencil
  option.
- **Custom grid:** a second grid with your own cell size, such as 8×8 or 16×16 tiles,
  in addition to the pixel grid.

### 0.6 — Animation

Make animating faster and get existing animations in.

- **Playback per tag:** forward, reverse or ping-pong, saved with the project and used
  by the preview and exports.
- **Linked cels:** one image used in several frames. Change it once, and every frame
  that uses it changes.
- **Import animations:** open an animated GIF, or cut a sprite sheet into frames.
- **Animated PNG** export, next to GIF and WebP.

### 0.7 — Layers and color

- **Layer groups:** fold, hide and lock several layers together.
- **Blend modes** for layers: multiply, screen, overlay, add, and a few more.
- **Reference layer:** a picture to trace over that never appears in exports.
- **Color ramps:** shade along a palette ramp with one tool, darker or lighter.
- **Palette lock:** optionally, draw only with colors from the current palette.

### 0.8 — Plugins, version 2

The plugin API grows from "change the active layer" to a real extension system.

- Read and change **layers, frames and tags**, not only the active layer.
- **Options dialogs:** a plugin asks for numbers, colors or choices before it runs.
- **Plugin tools** that draw on the canvas with the mouse.
- **More file formats** through plugins: import and export.
- A list of **ready-made plugins** in the documentation, like Pixel Cleanup and Color
  Tools today.

Plugins for API 1 keep running unchanged.

### 0.9 — Ready for everyone

The road to 1.0: polish, not new features.

- **Interface languages:** Russian first, then more. Every text in Kagero becomes
  translatable.
- **Kagero Pixel** becomes the default interface font, once it covers every language
  Kagero speaks.
- **Bigger projects:** canvases larger than 2048×2048 and more layers, measured with the
  benchmarks before the limits are raised.
- **Accessibility:** keyboard navigation in every dialog and readable sizes at every
  display scale.
- A full pass over the documentation, with up-to-date screenshots.

### 1.0 — Stable

- The `.kgr` format and plugin API 1 and 2 are **frozen**: changes from now on only add,
  never break.
- **Linux** build and updates.
- Every known limitation below is either fixed or clearly documented.

## Later

Ideas that come after 1.0, or earlier if there is time. They are not scheduled yet.

- **Tilemap mode:** draw with tiles, see the map update, and export the tileset.
- **macOS** build.
- **Text tool** with pixel fonts.
- **Navigator:** a small view of the whole image while you work zoomed in.
- **Gradient and pattern fills** with dithering.

## Not planned

Kagero stays a focused pixel-art and animation editor. These are out of scope:

- Painting with soft brushes, photo editing or vector drawing.
- Accounts, cloud storage or online features beyond update checks and the optional
  Discord status.
- Copying another editor's code, assets or file formats one-to-one.

## Known limitations

What doesn't work yet, and when it's planned:

| Limitation | Planned |
|------------|---------|
| Pencil and shape tools don't stay inside the selection (Fill does) | 0.5 |
| Lasso and Magic Wand selections can't be dragged; use Rectangle Select or the arrow keys | 0.5 |
| Custom brushes are lost when Kagero closes | 0.5 |
| The plugin API covers only the active layer, the selection and the colors | 0.8 |
| Canvas up to 2048×2048, 64 layers, 1000 frames | 0.9 |
| English interface only | 0.9 |
| Windows only | 1.0 (Linux), later (macOS) |

## Ideas and requests

Missing something? Open an issue on [GitHub](https://github.com/senzukisu/kagero/issues).
Tell us what you want to make, and what gets in the way. That helps more than a feature
name. Ideas that fit a version's theme are the most likely to land there.

## Done

| Version | Highlights |
|---------|------------|
| **0.4.3** | Fix for the stripe beside the tool strip; Pixel Cleanup and Color Tools plugins |
| **0.4.2** | Tool strip on an exact grid, a fix for exports that crash, clearer logs, long-session and compatibility tests, app code split by job |
| **0.4.1** | Auto size for exports, touchpad pan and pinch zoom, faster GIF export, compositing and project files, compact undo history, code cleanup |
| **0.4.0** | Canvas size and crop, layer opacity/duplicate/merge/locks, autosave and recovery, frame tags, lasso and magic wand, ellipse and polygon, symmetry, dithering, brushes, fill tolerance, convert to palette, animated WebP, packed sheets, themes, Lua plugins, program icon and installer |
| **0.3.8** | Custom shortcuts, redesigned Settings and New Image, Experimental page (pixel font, pixel-perfect pencil, tiled view), clearer icons, licenses in the program, Discord improvements |
| **0.3.5 – 0.3.7** | Kagero's own title bar, pixel-style interface and icons, Kagero Pixel font (optional), maximized start, Discord images, About window; Export dialog fits on small screens |
| **0.3.0** | Export dialog: PNG, JPEG, WebP, animated GIF, sprite sheets with JSON metadata, integer scaling; rectangle selection and transforms; multiple palettes |
| **0.2.0** | Frames and timeline, real-time preview, onion skin, `.kgr` v2, GitHub updater with signature checks, Discord Rich Presence, Settings |
| **0.1.0** | Drawing tools, layers, undo/redo, `.kgr` projects, PNG import and export, safe saving |

Details for every version: [Changelog](changelog.md).

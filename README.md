<div align="center">

<img src="docs/images/logo.png" width="128" height="128" alt="Kagero flame logo">

# Kagero 陽炎

**An independent pixel-art and sprite-animation editor by Onibi Lab.**

Version 0.3.5 Beta · Windows 10/11 x64 · Written in Rust · MIT License

[Download](https://github.com/senzukisu/kagero/releases) ·
[Changelog](CHANGELOG.md) ·
[Documentation](https://kagero.gitbook.io/kagero-docs)

</div>

---

Kagero ("heat haze") is a small, fast desktop editor for pixel art and frame-by-frame
animation. It is inspired by the general workflow of dedicated sprite editors, but its
design, code, pixel icons and pixel font are its own. The interface uses the warm,
dark **Kagero Ember** style — a quiet workshop lit by amber light — drawn in crisp
pixels: square corners, hard shadows and the Kagero Pixel font.

![Kagero with an 8-frame animation open: onion skin shows the neighbor frames in warm and cool tints, the timeline shows frame thumbnails](docs/images/screenshot-onion-skin.png)

<p align="center"><sub>Editing frame 4 of an animation with onion skin on: the previous frame
glows warm, the next one cool.</sub></p>

![Kagero playing the animation preview](docs/images/screenshot-preview.png)

<p align="center"><sub>Animation preview playing in real time (Enter to start and stop).</sub></p>

## Features

**Drawing**
- Images from 1×1 to 2048×2048 with transparent, white or black backgrounds
- Pencil, Eraser, Fill, Eyedropper, Line, Rectangle, Filled Rectangle, Pan and
  Rectangle Select
- Square brushes 1–32 px; left button paints the primary color, right the secondary
- Pixel-exact canvas: nearest-neighbor zoom, a transparency checkerboard aligned to
  pixels, an optional pixel grid, crisp at any Windows display scale

**Selection and palettes** (new in 0.3)
- Rectangle selection: move, copy/cut/paste, delete, duplicate, flip and rotate
  90/180/270° — each one undo step that also restores the selection
- Several palettes with GPL and HEX import/export, click-to-copy HEX, recent colors,
  built-in Kagero Ember, Ember, Night Sakura and Misty Forest

**Layers and history**
- Add, delete, rename, reorder and hide layers with alpha compositing
- Undo/Redo for every change, including all animation operations

**Animation**
- Frames with their own duration (10–60 000 ms); new, duplicate, delete, reorder
- Timeline with real thumbnails, resizable and collapsible
- Real-time looping preview (per-frame durations or fixed FPS)
- Onion skin: previous frame in a warm tint, next frame in a cool tint

**Export & sprite workflow** (new in 0.3)
- One Export dialog with live preview, background export, progress and cancel
- PNG (pixel-exact), JPEG (quality, background) and lossless WebP
- Integer scale 100–800 % or custom, always nearest neighbor
- Current, selected or all frames; numbered file names; asks before overwriting
- Animated GIF with real frame durations, loop or play once, clean transparency and
  exact colors
- Sprite sheets: strips and grids, spacing, padding, trim, extrude, plus Kagero JSON or
  JSON Hash metadata for game engines

**Files**
- `.kgr` projects (ZIP + JSON + PNG) keep frames, layers and settings; projects from
  Kagero 0.1 open automatically
- PNG import with transparency
- Safe saving: a crash or error never leaves a half-written project

**Integrations** (optional)
- Built-in updater: checks GitHub Releases, verifies the minisign signature of the
  package and installs it with automatic rollback
- Discord Rich Presence through the Discord desktop app, private by default

Everything works offline. Discord and GitHub are never required.

## Install

1. Download `kagero-0.3.5-windows-x64.zip` from the
   [Releases page](https://github.com/senzukisu/kagero/releases).
2. Extract it to a folder you own, for example `%LOCALAPPDATA%\Programs\Kagero`.
3. Run `kagero.exe`.

Kagero needs an OpenGL 2.0+ graphics driver. No installer, runtime or account is
needed. Settings and the log file are in `%APPDATA%\OnibiLab\Kagero`.

To check a download by hand:

```
rsign verify kagero-0.3.5-windows-x64.zip -p kagero.pub
```

## Keyboard shortcuts

| Keys                     | Action                                   |
|--------------------------|------------------------------------------|
| Ctrl+N / Ctrl+O          | New / Open                               |
| Ctrl+S / Ctrl+Shift+S    | Save / Save As                           |
| Ctrl+E / Ctrl+Shift+E    | Export current frame as PNG / Export dialog |
| Ctrl+Z / Ctrl+Y          | Undo / Redo (also Ctrl+Shift+Z)          |
| B E G I L U              | Pencil, Eraser, Fill, Eyedropper, Line, Rectangle |
| Shift+U / H / M          | Filled Rectangle / Pan / Rectangle Select |
| Space + drag             | Pan (also middle mouse button)           |
| Mouse wheel, + / −       | Zoom                                     |
| Home / 0 / 1             | Center / Fit / 100%                      |
| X                        | Swap primary and secondary colors        |
| Enter                    | Play / stop animation preview            |
| , / . (or ← / →)         | Previous / next frame                    |
| Ctrl+Home                | First frame                              |
| Alt+N / Alt+D            | New / duplicate frame                    |
| O                        | Onion skin                               |
| Ctrl+Shift+N, F2         | New layer, rename layer                  |
| Ctrl+] / Ctrl+[          | Move layer up / down                     |
| Ctrl+G, F7, F8           | Grid, timeline, side panel               |
| Ctrl+,                   | Settings                                 |
| Ctrl+A / Esc             | Select all / deselect (Esc also cancels a stroke or the preview) |
| Ctrl+C / Ctrl+X / Ctrl+V | Copy / cut / paste the selection         |
| Delete / Ctrl+D          | Clear / duplicate the selection          |
| Shift+H / Shift+V        | Flip horizontal / vertical               |
| Ctrl+R / Ctrl+Shift+R    | Rotate 90° clockwise / counterclockwise  |

## Building from source

Requirements: Rust stable (1.85+) with the MSVC toolchain on Windows.

```
cargo run                # debug build
cargo test               # all tests
cargo build --release    # target\release\kagero.exe
```

Source builds can check for updates but never install them; official packages are
built with `scripts\package-release.ps1` (see the
[release process](docs/developers/release-process.md)).

## Documentation

📖 **Read it online: [kagero.gitbook.io/kagero-docs](https://kagero.gitbook.io/kagero-docs)**

The GitBook site is built from the [`docs/`](docs/README.md) folder of this repository
and stays in sync with it.

| Section | Pages |
|---------|-------|
| **Getting started** | [Install](docs/getting-started/installation.md) · [Your first animation](docs/getting-started/first-animation.md) · [Workspace tour](docs/getting-started/interface-tour.md) |
| **User guide** | [Drawing tools](docs/user-guide/drawing-tools.md) · [Selection](docs/user-guide/selection.md) · [Palettes](docs/user-guide/palettes.md) · [Layers](docs/user-guide/layers.md) · [Animation](docs/user-guide/animation.md) · [Projects](docs/user-guide/files-and-export.md) · [Export](docs/user-guide/export.md) · [Animated GIF](docs/user-guide/animated-export.md) · [Sprite sheets](docs/user-guide/sprite-sheets.md) · [Settings](docs/user-guide/settings.md) · [Shortcuts](docs/user-guide/keyboard-shortcuts.md) |
| **Integrations** | [Updates](docs/integrations/updates.md) · [Discord Rich Presence](docs/integrations/discord.md) |
| **Developers** | [Architecture](docs/developers/architecture.md) · [Project format](docs/developers/project-format.md) · [Sprite sheet metadata](docs/developers/metadata-format.md) · [Updater internals](docs/developers/updater-internals.md) · [Discord RPC internals](docs/developers/discord-rpc-internals.md) · [UI design](docs/developers/ui-design.md) · [Testing](docs/developers/testing.md) · [Release process](docs/developers/release-process.md) · [Contributing](docs/developers/contributing.md) |
| **About** | [Roadmap](docs/about/roadmap.md) · [AI assistance](docs/about/ai-assistance.md) · [FAQ](docs/about/faq.md) · [License](docs/about/license.md) |

## How Kagero is made

Yes — large language models (LLMs) are used in Kagero's development, **but not for all
of the code**. Most often an LLM works as an assistant: helping with code (suggestions,
bug hunting, tests, reviews) and helping with building and publishing releases and
documentation. Decisions about what Kagero is and whether a change goes in stay with
the developer, and every change goes through the same formatting, Clippy and test
checks. Commits made with an assistant's help carry a `Co-Authored-By` line.
More in [How Kagero is made](docs/about/ai-assistance.md).

## Beta notes

0.3.5 is a beta. Known limitations:
- WebP export is lossless only; animated WebP is not available (use GIF).
- Sprite sheets use equal cells; packed (bin-packing) layouts are not available.
- Rectangle selection only; no layer opacity or canvas resize yet.
- Official packages are Windows x64 only.
- Installing updates needs Kagero to live in a folder your user can write to.

Please report problems on the [issue tracker](https://github.com/senzukisu/kagero/issues).

## License

Kagero's source code is released under the [MIT License](LICENSE).
Third-party components are listed in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
Kagero is not affiliated with Aseprite, Igara Studio or Discord.

© 2026 Onibi Lab

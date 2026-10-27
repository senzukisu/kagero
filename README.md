<div align="center">

# Kagero 陽炎

**An independent pixel-art and sprite-animation editor by Onibi Lab.**

Version 0.2.0 Beta · Windows 10/11 x64 · Written in Rust · MIT License

[Download](https://github.com/senzukisu/kagero/releases) ·
[Changelog](CHANGELOG.md) ·
[Documentation](#documentation)

</div>

---

Kagero ("heat haze") is a small, fast desktop editor for pixel art and frame-by-frame
animation. It is inspired by the general workflow of dedicated sprite editors, but its
design, code and pixel icons are its own. The interface uses the warm, dark
**Kagero Ember** style: a quiet workshop lit by amber light.

## Features

**Drawing**
- Images from 1×1 to 2048×2048 with transparent, white or black backgrounds
- Pencil, Eraser, Fill, Eyedropper, Line, Rectangle, Filled Rectangle and Pan
- Square brushes 1–32 px; left button paints the primary color, right the secondary
- Pixel-exact canvas: nearest-neighbor zoom, a transparency checkerboard aligned to
  pixels, an optional pixel grid, crisp at any Windows display scale

**Layers and history**
- Add, delete, rename, reorder and hide layers with alpha compositing
- Undo/Redo for every change, including all animation operations

**Animation** (new in 0.2)
- Frames with their own duration (10–60 000 ms); new, duplicate, delete, reorder
- Timeline with real thumbnails, resizable and collapsible
- Real-time looping preview (per-frame durations or fixed FPS)
- Onion skin: previous frame in a warm tint, next frame in a cool tint
- Export the current frame or all frames as PNG files

**Files**
- `.kgr` projects (ZIP + JSON + PNG) keep frames, layers and settings; projects from
  Kagero 0.1 open automatically
- PNG import and export with transparency
- Safe saving: a crash or error never leaves a half-written project

**Integrations** (optional)
- Built-in updater: checks GitHub Releases, verifies the minisign signature of the
  package and installs it with automatic rollback
- Discord Rich Presence through the Discord desktop app, private by default

Everything works offline. Discord and GitHub are never required.

## Install

1. Download `kagero-0.2.0-windows-x64.zip` from the
   [Releases page](https://github.com/senzukisu/kagero/releases).
2. Extract it to a folder you own, for example `%LOCALAPPDATA%\Programs\Kagero`.
3. Run `kagero.exe`.

Kagero needs an OpenGL 2.0+ graphics driver. No installer, runtime or account is
needed. Settings and the log file are in `%APPDATA%\OnibiLab\Kagero`.

To check a download by hand:

```
rsign verify kagero-0.2.0-windows-x64.zip -p kagero.pub
```

## Keyboard shortcuts

| Keys                     | Action                                   |
|--------------------------|------------------------------------------|
| Ctrl+N / Ctrl+O          | New / Open                               |
| Ctrl+S / Ctrl+Shift+S    | Save / Save As                           |
| Ctrl+E / Ctrl+Shift+E    | Export current frame / all frames        |
| Ctrl+Z / Ctrl+Y          | Undo / Redo (also Ctrl+Shift+Z)          |
| B E G I L U              | Pencil, Eraser, Fill, Eyedropper, Line, Rectangle |
| Shift+U / H              | Filled Rectangle / Pan                   |
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
| Esc                      | Cancel a stroke / stop the preview       |

## Building from source

Requirements: Rust stable (1.85+) with the MSVC toolchain on Windows.

```
cargo run                # debug build
cargo test               # all tests
cargo build --release    # target\release\kagero.exe
```

Source builds can check for updates but never install them; official packages are
built with `scripts\package-release.ps1` (see [RELEASE_PROCESS](docs/RELEASE_PROCESS.md)).

## Documentation

| Document | Contents |
|----------|----------|
| [ARCHITECTURE](docs/ARCHITECTURE.md) | Layers of the code, data model, history |
| [ANIMATION](docs/ANIMATION.md) | Frames, timeline, preview, onion skin |
| [PROJECT_FORMAT](docs/PROJECT_FORMAT.md) | The `.kgr` file format and migration |
| [UI_DESIGN](docs/UI_DESIGN.md) | Kagero Ember visual system |
| [UPDATER](docs/UPDATER.md) | Update checks, verification, installation |
| [DISCORD_RPC](docs/DISCORD_RPC.md) | Rich Presence setup and privacy |
| [RELEASE_PROCESS](docs/RELEASE_PROCESS.md) | How releases are built and signed |
| [TESTING](docs/TESTING.md) | Automated tests and manual checklist |
| [ROADMAP](docs/ROADMAP.md) | What's next |

## Beta notes

0.2.0 is a beta. Known limitations:
- No GIF or sprite-sheet export yet (export frames as PNGs instead).
- No selection/move tools and no layer opacity yet.
- Official packages are Windows x64 only.
- Installing updates needs Kagero to live in a folder your user can write to.

Please report problems on the [issue tracker](https://github.com/senzukisu/kagero/issues).

## License

Kagero's source code is released under the [MIT License](LICENSE).
Third-party components are listed in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
Kagero is not affiliated with Aseprite, Igara Studio or Discord.

© 2026 Onibi Lab

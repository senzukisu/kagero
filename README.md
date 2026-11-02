<div align="center">

<img src="docs/images/logo.png" width="112" height="112" alt="Kagero flame logo">

# Kagero 陽炎

**Pixel art and sprite animation, from the first pixel to a game-ready sprite sheet.**

[![Latest release](https://img.shields.io/github/v/release/senzukisu/kagero?label=download&color=ff9d42)](https://github.com/senzukisu/kagero/releases/latest)
![Windows 10/11](https://img.shields.io/badge/Windows-10%20%7C%2011-3a3d46)
[![MIT License](https://img.shields.io/badge/license-MIT-75c7b5)](LICENSE)

[**Download**](https://github.com/senzukisu/kagero/releases/latest) ·
[Documentation](https://kagero.gitbook.io/kagero-docs) ·
[What's new](CHANGELOG.md) ·
[Report a bug](https://github.com/senzukisu/kagero/issues)

</div>

<br>

Kagero ("heat haze") is a small, fast desktop editor for pixel art and frame-by-frame
animation by **Onibi Lab**. Draw on a pixel-exact canvas, animate with a timeline and
onion skin, and export PNGs, GIFs, WebPs and sprite sheets for your game engine. It runs
offline and needs no account.

![Kagero editing frame 4 of an 8-frame animation, with onion skin showing the neighbor frames and the timeline below](docs/images/screenshot-onion-skin.png)

<table>
<tr>
<td width="50%"><img src="docs/images/screenshot-preview.png" alt="Kagero playing the animation preview"></td>
<td width="50%"><img src="docs/images/screenshot-export.png" alt="The Export dialog on the Sprite Sheet tab with a live preview"></td>
</tr>
<tr>
<td align="center"><sub>Real-time animation preview</sub></td>
<td align="center"><sub>Sprite sheet export with a live preview</sub></td>
</tr>
</table>

## Features

| | |
|---|---|
| **Draw** | Pencil, eraser, fill with tolerance, line, rectangle, ellipse, polygon · square, round and custom brushes · symmetry · dithering · a pixel-exact canvas at any zoom and display scale |
| **Select** | Rectangle, lasso and magic wand · add, subtract, intersect, invert · move, copy, flip and rotate the exact shape |
| **Layers** | Opacity, lock, alpha lock, duplicate, merge down · canvas size and crop for all layers and frames |
| **Color** | Palettes with GIMP `.gpl` and HEX files · HEX/RGBA entry · convert an image to a palette |
| **Animate** | Frames with their own timing · live timeline thumbnails · real-time preview · onion skin · frame tags to loop and export parts |
| **Export** | PNG, JPEG, WebP · animated GIF and WebP · sprite sheets (grid or packed) with JSON metadata · pixel-sharp scaling, automatically enlarged for small sprites |
| **Comfort** | Autosave with crash recovery · undo for everything · custom shortcuts · themes · touchpad pan and pinch zoom |
| **Extend** | Plugins written in Lua, running in a sandbox |

Everything works offline. Updates (signed and verified) and Discord Rich Presence are
optional.

## Install

1. Download **`kagero-<version>-setup.exe`** from the
   [latest release](https://github.com/senzukisu/kagero/releases/latest) and run it.
   It installs for your user account, with no administrator rights, and adds a Start
   menu shortcut.
2. Start Kagero → **Create New** → pick a size → draw.

Prefer a portable copy? Extract `kagero-<version>-windows-x64.zip` to any folder you
own. Already have Kagero? **Settings → Updates → Check for Updates**.

Kagero needs Windows 10 or 11 (64-bit) and an OpenGL 2.0 graphics driver. Windows
SmartScreen may warn on first start because the program isn't signed with a paid
certificate; the release files are signed with [minisign](kagero.pub) instead.
→ [Install & update](docs/getting-started/installation.md)

## Documentation

**[kagero.gitbook.io/kagero-docs](https://kagero.gitbook.io/kagero-docs)**, built from the
[`docs/`](docs/README.md) folder:

- **Start:** [Install & update](docs/getting-started/installation.md) ·
  [Quick start](docs/getting-started/first-animation.md) ·
  [The workspace](docs/getting-started/interface-tour.md)
- **Use:** [Tools](docs/draw/drawing-tools.md) · [Selection](docs/draw/selection.md) ·
  [Colors](docs/draw/palettes.md) · [Layers](docs/draw/layers.md) ·
  [Animation](docs/animate/animation.md) · [Export](docs/export/export/README.md)
- **Reference:** [Shortcuts](docs/reference/keyboard-shortcuts.md) ·
  [Settings](docs/reference/settings.md) · [Themes](docs/reference/themes.md) ·
  [Plugins](docs/plugins/README.md) · [FAQ](docs/reference/faq.md)

## Build from source

Requires Rust 1.85 or newer with the MSVC toolchain on Windows.

```
cargo run --release      # build and start Kagero
cargo test               # all tests, no window or network needed
```

The code, its layers and the file formats are described in the
[developer documentation](docs/developers/architecture.md);
[Contributing](docs/developers/contributing.md) covers checks, benchmarks and releasing.

## About

Kagero is developed by Onibi Lab with help from AI assistants, as explained in
[How Kagero is made](docs/about/ai-assistance.md). Its code, icons and the Kagero Pixel
font are released under the [MIT License](LICENSE); third-party components are listed in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md). Kagero is not affiliated with
Aseprite, Igara Studio or Discord.

<div align="center"><sub>© 2026 Onibi Lab</sub></div>

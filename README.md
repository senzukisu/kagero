# Kagero 陽炎

**An independent pixel-art editor by Onibi Lab.** Version 0.1.0 Alpha.

Kagero is a small, fast desktop editor for pixel art, written in Rust with egui. It is
inspired by the general workflow of dedicated sprite editors, but its design, code and
assets are its own.

## Features (0.1.0)

- Images from 1×1 to 2048×2048 with a transparent, white or black background
- Tools: Pencil, Eraser, Fill, Eyedropper, Line, Rectangle, Filled Rectangle, Pan
- Layers: add, delete, rename, reorder, show/hide, with alpha compositing
- Unlimited-feeling Undo/Redo for every change (bounded by memory)
- Pixel-exact canvas: nearest-neighbor zoom, transparency checkerboard, pixel grid
- `.kgr` project files that keep layers, plus PNG import and export
- Safe saving, unsaved-changes prompts and readable error messages

## Download and run

Kagero needs Windows 10/11 x64 and an OpenGL 2.0+ graphics driver. No installer, server
or runtime is needed: run `kagero.exe`.

Settings and the log file are in `%APPDATA%\OnibiLab\Kagero`.

## Keyboard shortcuts

| Keys                  | Action                         |
|-----------------------|--------------------------------|
| Ctrl+N / Ctrl+O       | New / Open                     |
| Ctrl+S / Ctrl+Shift+S | Save / Save As                 |
| Ctrl+E                | Export PNG                     |
| Ctrl+W                | Close document                 |
| Ctrl+Z / Ctrl+Y       | Undo / Redo (also Ctrl+Shift+Z)|
| B E G I L U           | Pencil, Eraser, Fill, Eyedropper, Line, Rectangle |
| Shift+U / H           | Filled Rectangle / Pan         |
| Space + drag          | Pan (also middle mouse button) |
| Mouse wheel, + / −    | Zoom                           |
| Home / 0 / 1          | Center / Fit / 100%            |
| X                     | Swap primary and secondary     |
| Ctrl+G                | Toggle pixel grid              |
| Ctrl+Shift+N          | New layer                      |
| Ctrl+] / Ctrl+[       | Move layer up / down           |
| F2                    | Rename layer                   |
| Esc                   | Cancel the current stroke      |

Left mouse button paints with the primary color, right button with the secondary color.

## Building from source

Requirements: Rust (stable, 1.85 or newer) with the MSVC toolchain on Windows.

```
cargo run              # debug build
cargo build --release  # target\release\kagero.exe
cargo test             # all tests
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [UI design](docs/UI_DESIGN.md)
- [Project format](docs/PROJECT_FORMAT.md)
- [Roadmap](docs/ROADMAP.md)
- [Testing](docs/TESTING.md)

## License

Kagero's source code is released under the [MIT License](LICENSE).
Third-party components are listed in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
Kagero is not affiliated with Aseprite or Igara Studio.

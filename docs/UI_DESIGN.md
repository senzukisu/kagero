# Kagero UI Design

**Mood:** a dark Japanese workshop at night, lit by warm amber. Calm, precise, functional.
Kagero (陽炎) means "heat haze"; the flame mark is the shimmer above an ember.

## Palette

| Token            | Hex       | Use                                       |
|------------------|-----------|-------------------------------------------|
| Background       | `#17191F` | window, canvas surroundings, bars         |
| Panel            | `#20232B` | side panels, dialogs                      |
| Surface          | `#2C303A` | buttons, inputs, hovered rows             |
| Secondary text   | `#A5A9B5` | labels, hints, captions                   |
| Primary text     | `#F0EEE8` | content text, icons                       |
| Accent           | `#FF9D42` | selection, primary buttons, focus, cursor |
| Accent pressed   | `#F06B35` | pressed state                             |
| Success          | `#75C7B5` | "Saved", success messages                 |
| Error            | `#E85D5D` | error text (derived, not in the brief)    |

Text on accent uses `#1A1410`. Corner radius 4 px for widgets, 6–8 px for windows/cards.
All tokens live in `src/theme.rs`.

## Layout

```
┌ Menu bar ─────────────────────────────────────────────────────┐
├ Tool options: [icon] Pencil │ Size ▬▬●▬ │ hint …     [✓ Grid] ┤
├──┬─────────────────────────────────────────────┬──────────────┤
│T │                                             │ COLORS       │
│o │                                             │ [P][S] ⇄     │
│o │            canvas (checkerboard,            │ ■■■■■■■■     │
│l │            nearest-neighbor, grid)          │ ■■■■■■■■     │
│s │                                             ├──────────────┤
│  │                                             │ LAYERS + 🗑 ↑↓│
│  │                                             │ 👁 Layer 2    │
│  │                                             │ 👁 Background │
├──┴─────────────────────────────────────────────┴──────────────┤
└ Status: tool │ cursor x,y │ W×H │ zoom │ layer     ● Saved      ┘
```

- **Start screen:** centered card with the flame mark, "Kagero", "陽炎 · pixel art editor",
  "by Onibi Lab", *Create New* (accent) and *Open Project*, plus a Recent Projects list
  (missing files are dimmed and can be removed). Footer: studio, version, license.
- **Toolbar:** 34 px icon buttons in groups (Pencil/Eraser · Fill/Eyedropper ·
  Line/Rect/Filled · Pan). The active tool has an accent background. Tooltips show the
  name and shortcut.
- **Tool options:** only options that exist for the active tool (brush size for
  Pencil/Eraser/Line/Rectangle, sample source for Eyedropper) and a one-line hint.
- **Canvas:** checkerboard behind transparency that follows the image resolution — about
  32 cells across the longer side, rounded to a power of two (32×32 → 1 px cells,
  256×256 → 8 px, 2048×2048 → 64 px). It scales together with the image when zooming and
  cell edges always sit on pixel edges; cells only grow when zoomed out so far that they
  would be under 4 device pixels; 1 px border; pixel grid from 600% zoom; accent outline of the brush
  footprint under the cursor. Zoom is measured in *device* pixels per image pixel and the
  image origin is snapped to a device pixel, so pixels stay perfectly even at 125%/150%
  Windows display scaling.
- **Layers:** top layer first, accent bar on the active row, eye toggle, double-click or F2
  to rename inline, right-click menu (Rename, Show/Hide, Delete).
- **Status bar:** facts on the left, transient messages and the save indicator on the right.

## Icons

All icons are original 12×12 pixel bitmaps defined in `src/ui/icons.rs` and drawn as
rectangles, so they stay crisp at any UI scale. The window icon is the 16×16 flame mark
scaled ×4. No third-party icon assets are used.

## Fonts

egui's bundled fonts. On Windows, a system Japanese font (Yu Gothic / MS Gothic / Meiryo)
is loaded at runtime as a fallback so "陽炎" renders. It is never redistributed; if none
is found, the kanji simply do not render.

## Dialogs

Modal, centered, with the primary action as an accent button on the left:
New Image, Unsaved changes (Save / Don't Save / Cancel), error (title in error color and a
plain-language message), About (with a shortcut list). Escape or clicking outside closes
a dialog as "Cancel".

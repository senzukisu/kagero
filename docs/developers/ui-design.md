---
description: "The Kagero Ember visual system: palette, pixel font, icons and layout."
icon: palette
---

# Kagero UI Design

**Mood:** a dark Japanese workshop at night, lit by warm amber. Calm, precise, functional —
and, since 0.3.5, drawn in pixels like the art made in it.
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
| Ember deep       | `#6B361C` | pressed buttons and selected items, behind light text |
| Bevel light      | `#4A4F5C` | top-left edge of raised buttons, window outline |
| Bevel dark       | `#0E0F13` | bottom-right edge, button outlines, icon outlines |

Text on accent uses `#1A1410`. All tokens live in `src/theme.rs`.

## Pixel style (0.3.5)

* **Square corners** everywhere (`RADIUS = 0`).
* **Hard shadows**: windows, menus and the start card cast an offset shadow with no
  blur (4 px windows, 3 px menus, 6 px start card).
* **Bevels**: icon buttons are flat until hovered (raised: light top-left, dark
  bottom-right) and look pressed in when held or selected (dark top-left, ember fill,
  1 px amber outline for the active tool). Held icons sink by one pixel.
* **Outlined buttons**: text buttons sit in a 1 px dark outline; hover turns it amber.
* Slider handles are small rectangles, scrollbars are solid 8 px bars.

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
- **Toolbar:** 36 px icon buttons with 2× icons in groups (Pencil/Eraser ·
  Fill/Eyedropper · Line/Rect/Filled · Pan/Select). The active tool is pressed in with an
  amber outline. Tooltips show the name and shortcut.
- **Tool options:** only options that exist for the active tool (brush size for
  Pencil/Eraser/Line/Rectangle, sample source for Eyedropper) and a one-line hint.
- **Canvas:** checkerboard behind transparency where one square is exactly one image
  pixel, for every image size, so it doubles as a pixel grid and scales with zoom. Only
  when a pixel would be smaller than 3 device pixels (far zoomed out) do squares merge
  into 2×2, 4×4, … pixel blocks, still on pixel boundaries; 1 px border; pixel grid from 600% zoom; accent outline of the brush
  footprint under the cursor. Zoom is measured in *device* pixels per image pixel and the
  image origin is snapped to a device pixel, so pixels stay perfectly even at 125%/150%
  Windows display scaling.
- **Layers:** top layer first, accent bar on the active row, eye toggle, double-click or F2
  to rename inline, right-click menu (Rename, Show/Hide, Delete).
- **Status bar:** facts on the left, transient messages and the save indicator on the right.

## Icons

All icons are original 16×16 pixel bitmaps in `src/ui/icons.rs`, colored from a small
fixed palette (`k` outline, `#` light, `g` gray, `d` dim, `o` ember, `p` flame, `r` rust,
`y` glow, `b` sky, `n` navy, `t` mint, `w` wood, `e` blush) with dark outlines and
two-tone shading. They are drawn as one mesh of whole device pixels — 2× in the
toolbar, 1× in compact rows — and fade to gray when disabled. The window icon is the
16×16 flame mark scaled ×4. No third-party icon assets are used.

## Fonts

The default interface font is the **classic** one (egui's bundled fonts, body 14 pt,
headings 20 pt), as in Kagero 0.3.0.

**Kagero Pixel** (Settings → General → *Pixel font (experimental)*) is Kagero's own
bitmap-style font (MIT, Onibi Lab): 186 glyphs — ASCII, the typographic marks the
interface uses (· × … — ° “ ” → …) and Cyrillic. Capitals are 11 pixels tall and
usually 6 wide, lowercase 8, descenders 2. Glyphs are drawn on a compact 8×5 grid; the
build draws rows 2, 6 and 7 and one interior column per glyph twice, choosing ones
that hold no horizontal (rows) or vertical (columns) strokes, so strokes stay one
pixel thin. It is meant to become the default once it is polished.

* Source: `assets/fonts/kagero-pixel.txt` — every glyph drawn as `#`/`.` rows.
* Build: `python scripts/build-font.py` (needs `fonttools`) writes
  `assets/fonts/KageroPixel.ttf`, which is embedded in the executable. Each pixel is a
  square outline, so the font stays sharp.
* Sizes: one font pixel is always a whole number of *device* pixels (1 px at 100 % and
  125 % display scale, 2 px at 150–200 %), so text never blurs; headings use 2×, the
  start-screen and About titles 3×. `theme::font(n)` returns the current interface
  font at n× (classic: 14 / 20 / 34 pt).
* Fallbacks: egui's bundled fonts for characters Kagero Pixel doesn't have, and on
  Windows a system Japanese font (Yu Gothic / MS Gothic / Meiryo) loaded at runtime so
  "陽炎" renders. The system font is never redistributed.

## Title bar (0.3.6)

Kagero draws its own window frame (`src/ui/title_bar.rs`; the system frame is off):
one 32 px bar with the flame, the menus, the document title centered on the window,
and 46 px minimize / maximize-restore / close buttons drawn as 10×10 pixel glyphs
(close turns red on hover). Dragging empty space moves the window, double-click
maximizes or restores, and a restored window resizes from a 5 px border. The window
is created centered and maximized on the first frame when it was maximized last time.

## Dialogs

Modal, centered, with the primary action as an accent button on the left:
New Image, Unsaved changes (Save / Don't Save / Cancel), error (title in error color and a
plain-language message), About (with a shortcut list). Escape or clicking outside closes
a dialog as "Cancel".

## 0.2 additions

- **Timeline** (bottom, full width): a 38 px controls row — playback (first, previous,
  play/stop, next, `3 / 12` counter), frame actions (new, duplicate, delete, move),
  duration, onion skin toggle with Prev/Next/opacity, and on the right the preview
  timing and the collapse arrow — above a scrollable strip of 64 px frame cards
  (checkered thumbnail, number, duration). The active card has a 2 px amber border.
  The panel is resizable (142–360 px) and collapses to the controls row (F7).
- **Canvas badges** (top-left, small panel-colored pill): a pulsing amber dot while
  previewing, a teal dot while onion skin is on. "Reduce motion" stops the pulse.
- **Onion tints**: previous frame warm (#FF7A5C), next frame cool (#6FD6C2), so past and
  future are told apart without reading labels.
- **Settings window** (Ctrl+,): tabs on the left, pages with small uppercase section
  titles, plain-language hints, status lines with a colored dot, and one switch style
  (amber pill when on) used for every on/off setting.
- **Pressed and selected states** use *Ember deep* with light text instead of bright
  orange, so labels stay readable while clicked.
- New pixel icons: play, stop, first/previous/next frame, duplicate, move left/right,
  onion skin, settings.
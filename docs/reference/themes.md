# Themes

Themes change Kagero's colors. Open **Settings → Themes**.

## Built-in themes

| Theme | Look |
| --- | --- |
| **Kagero Ember** | Dark workshop with warm amber (the default) |
| **Night Sakura** | Dark plum with sakura pink |
| **Misty Forest** | Dark green-gray with moss green |
| **Paper** | Light paper with orange |

## Trying a theme

1. Click **Preview** on a theme card. The whole window changes right away.
2. Click **Keep This Theme** to use it, or **Revert** to go back.

If you close Settings without keeping it, Kagero returns to your theme.

## Your own themes

- **Export…** saves any theme, including the built-in ones, as a `.json` file. Exporting a built-in theme is the easiest way to start your own.
- **Import Theme…** adds a theme file. If you import a theme with the same name again, it is updated.
- **Delete** removes one of your themes (it asks once more). Built-in themes can't be deleted. If you delete the theme you're using, Kagero switches back to Kagero Ember.
- Your themes are stored in `%APPDATA%\OnibiLab\Kagero\themes`. After changing files there, click **Reload**.

## Theme file format (version 1)

```json
{
  "format": "kagero-theme",
  "version": 1,
  "name": "My Theme",
  "base": "dark",
  "colors": {
    "background": "#17191F",
    "panel": "#20232B",
    "surface": "#2C303A",
    "surface_hover": "#373C48",
    "border": "#343843",
    "bevel_light": "#4A4F5C",
    "bevel_dark": "#0E0F13",
    "text_secondary": "#A5A9B5",
    "text_primary": "#F0EEE8",
    "accent": "#FF9D42",
    "accent_pressed": "#F06B35",
    "success": "#75C7B5",
    "accent_deep": "#6B361C",
    "error": "#E85D5D",
    "on_accent": "#1A1410",
    "checker_light": "#4A4E58",
    "checker_dark": "#3A3D46"
  }
}
```

- **`base`** is `"dark"` or `"light"`. It decides which default style egui's widgets start from.
- **Colors:** all 17 are required, as opaque `#RRGGBB` (`#RGB` works too). The table below says where each is used.
- **Name:** must not be the name of a built-in theme. It can be up to 48 characters.
- **Size:** files larger than 64 KB are refused.

| Key | Used for |
| --- | --- |
| `background` | Behind the canvas and in text fields |
| `panel` | Panels, menus and windows |
| `surface`, `surface_hover` | Buttons and cards, normal and hovered |
| `border` | Thin lines between areas |
| `bevel_light`, `bevel_dark` | Pixel bevels and hard shadows |
| `text_primary`, `text_secondary` | Main text and hints |
| `accent`, `accent_pressed` | Highlights, the selected tool and progress |
| `accent_deep` | Behind selected items (light text sits on it) |
| `on_accent` | Text drawn on the accent color |
| `success`, `error` | Good and bad messages |
| `checker_light`, `checker_dark` | The transparency checkerboard |

### Readability check

A theme is refused when its text would be hard to read. The minimum contrast ratios (as in WCAG) are:

- 4.5:1 for `text_primary` on `panel` and on `surface`;
- 3:1 for `text_primary` on `accent_deep`;
- 3:1 for `text_secondary` on `panel`;
- 3:1 for `on_accent` on `accent`.

Settings lists each theme file that can't be used, with the reason (for example, a missing color or a newer format version).

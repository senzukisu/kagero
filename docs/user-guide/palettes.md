---
description: "Primary and secondary colors, palettes, palette files and recent colors."
icon: palette
---

# Colors and palettes

## Primary and secondary

* **Left** mouse button paints with the primary color, **right** with the secondary.
* Click a big swatch to open the color picker (with alpha); <kbd>X</kbd> swaps them.
* Under each swatch is its **HEX** code — click it to copy (`#RRGGBB`, or `#RRGGBBAA`
  for translucent colors). The line below shows the primary color as **R G B A**.

## Recent colors

The last 12 colors you painted or filled with appear in the **Recent** row. Click one
to use it again. They are kept between sessions.

## Palettes

Pick the palette in the drop-down above the swatches. Built-in palettes:

| Palette | Colors |
|---------|------|
| **Kagero Ember** | The default: neutrals, ember warms, moss greens and night blues. |
| **Ember** | A fire ramp from soot to white-hot, with warm earth tones. |
| **Night Sakura** | Deep night blues, cherry-blossom pinks and a moonlit teal. |
| **Misty Forest** | Layered greens, fog greys and bark browns. |

Click a swatch to make it the primary color. Right-click a swatch for **Use as
Primary / Secondary**, **Copy HEX**, **Move Left / Right** and **Remove from Palette**.

The **...** menu next to the palette picker:

| Item | What it does |
|------|--------------|
| Add Primary Color | Adds the current primary color (duplicates are skipped). |
| New / Duplicate Palette | Creates a palette and switches to it. |
| Rename / Delete Palette | The last remaining palette can't be deleted. |
| Import Palette… | Reads a `.gpl` or `.hex` file. |
| Export Palette… | Saves the current palette as `.gpl` or `.hex`. |
| Restore Built-in Palettes | Brings back the built-ins as shipped; your own palettes are kept. |

Switching palettes never changes your drawing. Palettes are saved with your settings,
not in projects. A palette holds up to 256 colors; you can keep up to 64 palettes.

## Palette files

| Format | Contents |
|--------|----------|
| **GIMP palette** (`.gpl`) | `GIMP Palette` header, optional `Name:`, then `R G B name` lines. No alpha. |
| **HEX** (`.hex`, `.txt`) | One `RRGGBB` per line; `AARRGGBB` lines keep alpha. `;` and `//` start comments. |

Files over 1 MiB or with more than 256 colors are refused with a clear message.

{% hint style="info" %}
Upgrading from 0.2: if you had customized the single 0.2 palette, it appears as
**My Palette**.
{% endhint %}

"""Builds assets/fonts/KageroPixel.ttf from assets/fonts/kagero-pixel.txt.

Glyphs are drawn on a compact grid (capitals 8×5). The build enlarges them by
drawing some rows and columns twice, chosen so that strokes stay one pixel thin:

* rows 1, 5 and 6 — rows that hold vertical strokes or diagonals in almost every
  glyph — giving 11-pixel capitals and an 8-pixel x-height;
* one interior column per glyph 4–6 pixels wide: the column with the fewest
  vertical-stroke pixels (closest to the middle on ties), so stems stay one pixel
  wide and only horizontal strokes get longer — 5-pixel capitals become 6.

Every pixel becomes a square outline of 100 font units, so the font renders with
perfectly sharp edges whenever one font pixel covers whole screen pixels (Kagero
picks its text sizes that way). The vertical metrics are in pixels: ascent 12 (cap
height 11 plus one row of air), descent 2, line gap 2. The font size that maps one
font pixel to one point is therefore 14.

Requires fontTools (`pip install fonttools`). Optional: `--preview out.png` writes a
sample sheet rendered straight from the glyph source.
"""

import argparse
import pathlib
import re
import struct
import zlib

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "assets" / "fonts" / "kagero-pixel.txt"
OUTPUT = ROOT / "assets" / "fonts" / "KageroPixel.ttf"

UNIT = 100  # font units per pixel
SOURCE_ABOVE = 8  # rows above the baseline in the source
TALL_ROWS = (1, 5, 6)  # source rows drawn twice
ABOVE = SOURCE_ABOVE + len(TALL_ROWS)  # rows above the baseline in the font
ASCENT, DESCENT, LINE_GAP = ABOVE + 1, 2, 2
X_HEIGHT = 8
FAMILY = "Kagero Pixel"
VERSION = "1.000"


def parse(text):
    glyphs, aliases = {}, {}
    name, rows = None, []

    def flush():
        if name is not None:
            if not rows:
                raise ValueError(f"glyph {name!r} has no rows")
            width = len(rows[0])
            if any(len(r) != width for r in rows):
                raise ValueError(f"glyph {name!r}: rows have different lengths")
            if len(rows) not in (SOURCE_ABOVE, SOURCE_ABOVE + 1, SOURCE_ABOVE + 2):
                raise ValueError(f"glyph {name!r}: {len(rows)} rows (need 8–10)")
            if name in glyphs:
                raise ValueError(f"glyph {name!r} defined twice")
            glyphs[name] = rows[:]

    for line in text.splitlines():
        line = line.rstrip()
        # Comments: "# text", or any "#" line before the first glyph.
        if not line or line.startswith("# ") or (name is None and line.startswith("#")):
            continue
        m = re.fullmatch(r"\[(.+)\]", line)
        if m and not set(line) <= set("#."):
            flush()
            key = m.group(1)
            if key == "space":
                name = " "
            elif re.fullmatch(r"U\+[0-9A-Fa-f]{4,6}", key):
                name = chr(int(key[2:], 16))
            elif len(key) == 1:
                name = key
            else:
                raise ValueError(f"bad glyph header {line!r}")
            rows = []
            continue
        if name is not None and not rows and line.startswith("= "):
            aliases[name] = line[2:]
            name = None
            continue
        if name is None or not set(line) <= set("#."):
            raise ValueError(f"unexpected line {line!r}")
        rows.append(line)
    flush()
    for alias, target in aliases.items():
        if target not in glyphs:
            raise ValueError(f"alias {alias!r} points to missing glyph {target!r}")
        glyphs[alias] = glyphs[target]
    return {ch: tall(wide(rows)) for ch, rows in glyphs.items()}


def wide(rows):
    """Doubles the interior column that touches the fewest vertical strokes."""
    width = len(rows[0])
    if not 4 <= width <= 6:
        return rows

    def stem_pixels(c):
        return sum(
            1
            for r, row in enumerate(rows)
            if row[c] == "#"
            and ((r > 0 and rows[r - 1][c] == "#") or (r + 1 < len(rows) and rows[r + 1][c] == "#"))
        )

    middle = (width - 1) / 2
    col = min(range(1, width - 1), key=lambda c: (stem_pixels(c), abs(c - middle)))
    return [row[: col + 1] + row[col] + row[col + 1 :] for row in rows]


def tall(rows):
    """Doubles the TALL_ROWS of a source glyph."""
    out = []
    for i, row in enumerate(rows):
        out.append(row)
        if i in TALL_ROWS:
            out.append(row)
    return out


def glyph_outline(rows):
    """Horizontal runs of pixels as rectangles (clockwise, TrueType convention)."""
    pen = TTGlyphPen(None)
    for r, row in enumerate(rows):
        top = (ABOVE - r) * UNIT
        bottom = top - UNIT
        x = 0
        while x < len(row):
            if row[x] != "#":
                x += 1
                continue
            start = x
            while x < len(row) and row[x] == "#":
                x += 1
            left, right = start * UNIT, x * UNIT
            pen.moveTo((left, bottom))
            pen.lineTo((left, top))
            pen.lineTo((right, top))
            pen.lineTo((right, bottom))
            pen.closePath()
    return pen.glyph()


def glyph_name(ch):
    return "space" if ch == " " else f"uni{ord(ch):04X}"


def build(glyphs, out):
    order = [".notdef"] + [glyph_name(c) for c in sorted(glyphs)]
    fb = FontBuilder(unitsPerEm=(ASCENT + DESCENT) * UNIT, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap({ord(c): glyph_name(c) for c in glyphs})

    notdef = ["#####"] + ["#...#"] * (ABOVE - 2) + ["#####"]
    outlines = {".notdef": glyph_outline(notdef)}
    metrics = {".notdef": ((len(notdef[0]) + 1) * UNIT, 0)}
    for ch, rows in glyphs.items():
        outlines[glyph_name(ch)] = glyph_outline(rows)
        metrics[glyph_name(ch)] = ((len(rows[0]) + 1) * UNIT, 0)
    fb.setupGlyf(outlines)
    # Left side bearings must match the outlines' xMin.
    glyf = fb.font["glyf"]
    hmtx = {}
    for name, (adv, _) in metrics.items():
        g = glyf[name]
        hmtx[name] = (adv, getattr(g, "xMin", 0) if g.numberOfContours else 0)
    fb.setupHorizontalMetrics(hmtx)
    fb.setupHorizontalHeader(ascent=ASCENT * UNIT, descent=-DESCENT * UNIT, lineGap=LINE_GAP * UNIT)
    fb.setupNameTable(
        {
            "familyName": FAMILY,
            "styleName": "Regular",
            "uniqueFontIdentifier": f"OnibiLab:{FAMILY}:{VERSION}",
            "fullName": f"{FAMILY} Regular",
            "psName": "KageroPixel-Regular",
            "version": f"Version {VERSION}",
            "copyright": "Copyright (c) 2026 Onibi Lab",
            "licenseDescription": "MIT License",
            "manufacturer": "Onibi Lab",
        }
    )
    fb.setupOS2(
        sTypoAscender=ASCENT * UNIT,
        sTypoDescender=-DESCENT * UNIT,
        sTypoLineGap=LINE_GAP * UNIT,
        usWinAscent=ASCENT * UNIT,
        usWinDescent=DESCENT * UNIT,
        sxHeight=X_HEIGHT * UNIT,
        sCapHeight=ABOVE * UNIT,
        fsType=0,
    )
    fb.setupPost()
    fb.setupHead(unitsPerEm=(ASCENT + DESCENT) * UNIT, fontRevision=float(VERSION))
    fb.font.save(str(out))


def write_png(path, pixels, width, height):
    raw = b"".join(b"\x00" + bytes(pixels[y * width:(y + 1) * width]) for y in range(height))
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    pathlib.Path(path).write_bytes(png)


def preview(glyphs, path, scale=3):
    lines = [
        "Kagero Pixel - the quick brown fox jumps over the lazy dog.",
        "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789",
        "File Edit Image Tools View Layer Frame Help - Export... Settings",
        "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ · × … — ° “quoted” → ²",
        "Frames: 8 · 32×32 px · 100 ms — Rotate 90° — Kagero 0.3.5 Beta",
        "Съешь же ещё этих мягких французских булок, да выпей чаю",
        "ШИРОКАЯ ЭЛЕКТРИФИКАЦИЯ ЮЖНЫХ ДЕРЕВЕНЬ ЁЖ Й Щ Б Ъ C:\Документы № 5",
    ]
    line_h = ASCENT + DESCENT + LINE_GAP
    width = max(sum(len(glyphs.get(c, glyphs["?"])[0]) + 1 for c in s) for s in lines) + 4
    height = line_h * len(lines) + 4
    canvas = [24] * (width * height)
    for li, s in enumerate(lines):
        x, base = 2, 2 + li * line_h + 1
        for c in s:
            rows = glyphs.get(c, glyphs["?"])
            for r, row in enumerate(rows):
                for cx, p in enumerate(row):
                    if p == "#":
                        canvas[(base + r) * width + x + cx] = 235
            x += len(rows[0]) + 1
    big = []
    for y in range(height * scale):
        row = canvas[(y // scale) * width:(y // scale + 1) * width]
        big.extend(v for v in row for _ in range(scale))
    write_png(path, big, width * scale, height * scale)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview", help="also write a PNG sample sheet")
    args = ap.parse_args()
    glyphs = parse(SOURCE.read_text(encoding="utf-8"))
    for ch in map(chr, range(0x20, 0x7F)):
        if ch not in glyphs:
            raise SystemExit(f"missing ASCII glyph {ch!r}")
    build(glyphs, OUTPUT)
    print(f"{OUTPUT.relative_to(ROOT)}: {len(glyphs)} glyphs, {OUTPUT.stat().st_size} bytes")
    if args.preview:
        preview(glyphs, args.preview)


if __name__ == "__main__":
    main()

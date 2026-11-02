"""Writes the .kgr compatibility fixtures in tests/fixtures by hand.

They are made without Kagero's own writer, exactly as each format version is
documented (docs/developers/project-format.md), so tests/fixtures.rs checks that
Kagero still opens projects of every version the way they were meant.

    python scripts/make-test-fixtures.py      (needs Pillow)
"""
import io
import json
import pathlib
import zipfile

from PIL import Image

OUT = pathlib.Path(__file__).resolve().parent.parent / "tests" / "fixtures"


def png(w, h, pixels):
    """RGBA PNG bytes; `pixels` maps (x, y) to an (r, g, b, a) color."""
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    for (x, y), c in pixels.items():
        img.putpixel((x, y), c)
    buf = io.BytesIO()
    img.save(buf, "PNG")
    return buf.getvalue()


def write(name, manifest, document, files):
    OUT.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(OUT / name, "w", zipfile.ZIP_DEFLATED) as z:
        # A fixed date keeps the files byte-identical when regenerated.
        def add(path, data):
            info = zipfile.ZipInfo(path, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
        add("manifest.json", json.dumps(manifest, indent=2))
        add("document.json", json.dumps(document, indent=2))
        for path, data in files.items():
            add(path, data)
    print("wrote", OUT / name)


RED = (220, 30, 30, 255)
BLUE = (30, 60, 220, 255)
GREEN = (40, 180, 70, 255)
HALF = (250, 200, 60, 128)

# Version 1 (Kagero 0.1): layers only.
write(
    "v1-layers.kgr",
    {"format": "kagero-project", "format_version": 1},
    {
        "width": 4,
        "height": 3,
        "active_layer": 1,
        "layers": [
            {"name": "Back", "visible": True, "file": "layers/0000.png"},
            {"name": "Ink", "visible": False, "file": "layers/0001.png"},
        ],
    },
    {
        "layers/0000.png": png(4, 3, {(x, y): RED for x in range(4) for y in range(3)}),
        "layers/0001.png": png(4, 3, {(2, 1): BLUE}),
    },
)

# Version 2 (Kagero 0.2–0.3): frames, durations and ids that aren't 1, 2, 3…
frames = []
files = {}
for fi, (fid, ms) in enumerate([(2, 100), (5, 150), (9, 200)]):
    cels = []
    for li in range(2):
        path = f"frames/{fi:04}/{li:04}.png"
        cels.append(path)
        files[path] = png(5, 4, {(x, 3): GREEN for x in range(5)} if li == 0 else {(fi, 1): RED})
    frames.append({"id": fid, "duration_ms": ms, "cels": cels})
write(
    "v2-animation.kgr",
    {"format": "kagero-project", "format_version": 2, "app_version": "0.3.8"},
    {
        "width": 5,
        "height": 4,
        "active_layer": 1,
        "active_frame": 2,
        "layers": [
            {"id": 3, "name": "Ground", "visible": True},
            {"id": 7, "name": "Ball", "visible": True},
        ],
        "frames": frames,
    },
    files,
)

# Version 3 (Kagero 0.4): layer opacity, locks and frame tags.
files = {}
frames = []
for fi in range(2):
    cels = []
    for li in range(2):
        path = f"frames/{fi:04}/{li:04}.png"
        cels.append(path)
        files[path] = png(3, 3, {(1, 1): HALF} if li == 1 else {(0, 0): BLUE})
    frames.append({"id": fi + 1, "duration_ms": 120, "cels": cels})
write(
    "v3-opacity-tags.kgr",
    {"format": "kagero-project", "format_version": 3, "app_version": "0.4.1"},
    {
        "width": 3,
        "height": 3,
        "active_layer": 0,
        "active_frame": 1,
        "layers": [
            {"id": 1, "name": "Background", "visible": True, "opacity": 255, "locked": True},
            {"id": 2, "name": "Glow", "visible": True, "opacity": 128, "alpha_lock": True},
        ],
        "frames": frames,
        "tags": [{"name": "walk", "start": 0, "end": 1, "color": "#E86A33"}],
    },
    files,
)

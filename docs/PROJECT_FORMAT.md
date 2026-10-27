# The `.kgr` Project Format

Format id `kagero-project`, version **1**.

A `.kgr` file is a standard ZIP archive:

```
project.kgr
├── manifest.json        format id and version (Deflate)
├── document.json        image size, layer list, active layer (Deflate)
└── layers/
    ├── 0000.png         bottom layer, RGBA8 PNG (Stored)
    ├── 0001.png
    └── …
```

## manifest.json

```json
{
  "format": "kagero-project",
  "format_version": 1,
  "app_version": "0.1.0"
}
```

- `format` must be exactly `kagero-project`.
- `format_version` is an integer. Readers refuse versions newer than they support and
  say which version is required.
- `app_version` is informational.

## document.json

```json
{
  "width": 32,
  "height": 32,
  "active_layer": 1,
  "layers": [
    { "name": "Background", "visible": true, "file": "layers/0000.png" },
    { "name": "Ink",        "visible": true, "file": "layers/0001.png" }
  ]
}
```

- `layers` are ordered **bottom to top**.
- `file` must be exactly `layers/NNNN.png`, where `NNNN` is the zero-padded position in
  the list. Any other path is rejected.
- Unknown fields are ignored, so later 1.x writers may add optional data.

## Layer PNGs

Each layer is an 8-bit RGBA PNG with exactly `width × height` pixels and straight
(non-premultiplied) alpha. Fully transparent pixels are stored as `0,0,0,0`.

## Validation when opening

Checked in this order, before any large allocation:

1. File size ≤ 1 GiB; ZIP has ≤ 80 entries.
2. `manifest.json` (≤ 1 MiB) — id and version.
3. `document.json` (≤ 1 MiB) — `1 ≤ width, height ≤ 2048`, `1 ≤ layers ≤ 64`,
   `width × height × 4 × layers ≤ 512 MiB`, `active_layer < layers`.
4. For each layer: canonical file name, entry ≤ 20 MiB, PNG header dimensions checked
   before decoding, decoder allocation limits, decoded size must equal the document size.
5. Layer names are trimmed, control characters removed, limited to 64 characters.

Entries are read into memory by exact name. Nothing is ever extracted to disk, so paths
inside the archive cannot reach the file system.

## Saving

Projects are written to a temporary file next to the target
(`.<name>.kgr.<pid>.tmp`), flushed and synced, then renamed over the target. A crash or
error during saving leaves the previous file intact.

## Versioning policy

- Additive, optional changes keep version 1.
- Any change that older readers would misinterpret bumps `format_version`. New Kagero
  versions must keep reading all older versions.

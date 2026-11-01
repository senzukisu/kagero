//! JSON metadata written next to a sprite sheet.
//!
//! Two layouts, both documented in `docs/developers/metadata-format.md`:
//!
//! * **Kagero JSON** (`"format": "kagero-sprite-sheet"`, versioned): frames as an
//!   ordered array with durations and layout information.
//! * **JSON Hash**: the common texture-atlas layout (`frames` object keyed by frame
//!   name with `frame`, `rotated`, `trimmed`, `spriteSourceSize`, `sourceSize`, plus a
//!   `meta` block). Kagero never rotates frames, so `rotated` is always `false`.

use serde::Serialize;

use super::sequence::frame_stem;
use super::spritesheet::{Rect, SheetPlan};
use super::{SheetLayout, SheetOptions};
use crate::core::FrameTag;

/// Version of the Kagero sprite-sheet JSON format.
pub const KAGERO_SHEET_FORMAT_VERSION: u32 = 1;

#[derive(Serialize)]
struct Size {
    w: u32,
    h: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KageroFrame {
    name: String,
    /// Position of the frame in the document (0-based).
    index: usize,
    frame: Rect,
    trimmed: bool,
    /// Part of the source frame stored in `frame` (offset = trim offset).
    source: Rect,
    source_size: Size,
    duration: u32,
    /// Packed sheets: this frame reuses the pixels of the frame at this position.
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate_of: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KageroSheet {
    format: &'static str,
    version: u32,
    app: String,
    image: String,
    size: Size,
    scale: u32,
    layout: &'static str,
    cell: Size,
    columns: u32,
    rows: u32,
    spacing: u32,
    padding: u32,
    extrude: u32,
    frame_count: usize,
    total_duration: u64,
    frames: Vec<KageroFrame>,
    /// Frame tags, as positions in `frames` (inclusive). Absent when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<SheetTag>,
}

#[derive(Serialize)]
struct SheetTag {
    name: String,
    from: usize,
    to: usize,
    color: String,
}

/// Aseprite-style tag entry (`meta.frameTags`), understood by many engines.
#[derive(Serialize)]
struct HashTag {
    name: String,
    from: usize,
    to: usize,
    direction: &'static str,
    color: String,
}

/// Tags that have frames on the sheet, as (name, first, last sheet position, color).
fn sheet_tags(plan: &SheetPlan, tags: &[FrameTag]) -> Vec<SheetTag> {
    tags.iter()
        .filter_map(|t| {
            let mut orders = plan
                .placements
                .iter()
                .filter(|p| t.contains(p.frame_index))
                .map(|p| p.order);
            let first = orders.next()?;
            let (from, to) = orders.fold((first, first), |(a, b), o| (a.min(o), b.max(o)));
            Some(SheetTag {
                name: t.name.clone(),
                from,
                to,
                color: t.color.to_hex(),
            })
        })
        .collect()
}

fn layout_name(options: &SheetOptions) -> &'static str {
    if options.packed {
        return "packed";
    }
    match options.layout {
        SheetLayout::Horizontal => "horizontal",
        SheetLayout::Vertical => "vertical",
        SheetLayout::Columns => "columns",
        SheetLayout::Rows => "rows",
    }
}

/// Builds Kagero sprite-sheet JSON.
pub fn kagero_json(
    plan: &SheetPlan,
    options: &SheetOptions,
    image_name: &str,
    base: &str,
    digits: u8,
    scale: u32,
    tags: &[FrameTag],
) -> Result<String, serde_json::Error> {
    let sheet = KageroSheet {
        format: "kagero-sprite-sheet",
        version: KAGERO_SHEET_FORMAT_VERSION,
        app: format!("Kagero {}", crate::APP_VERSION),
        image: image_name.to_owned(),
        size: Size {
            w: plan.width,
            h: plan.height,
        },
        scale,
        layout: layout_name(options),
        cell: Size {
            w: plan.cell_w,
            h: plan.cell_h,
        },
        columns: plan.columns,
        rows: plan.rows,
        spacing: options.spacing,
        padding: options.padding,
        extrude: options.extrude,
        frame_count: plan.placements.len(),
        total_duration: plan
            .placements
            .iter()
            .map(|p| u64::from(p.duration_ms))
            .sum(),
        frames: plan
            .placements
            .iter()
            .map(|p| KageroFrame {
                name: frame_stem(base, p.order + 1, digits),
                index: p.frame_index,
                frame: p.frame,
                trimmed: p.trimmed,
                source: p.source,
                source_size: Size {
                    w: p.source_w,
                    h: p.source_h,
                },
                duration: p.duration_ms,
                duplicate_of: p.duplicate_of,
            })
            .collect(),
        tags: sheet_tags(plan, tags),
    };
    serde_json::to_string_pretty(&sheet)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashFrame {
    frame: Rect,
    rotated: bool,
    trimmed: bool,
    sprite_source_size: Rect,
    source_size: Size,
    duration: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashMeta {
    app: &'static str,
    version: &'static str,
    image: String,
    format: &'static str,
    size: Size,
    scale: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    frame_tags: Vec<HashTag>,
}

#[derive(Serialize)]
struct HashSheet {
    frames: serde_json::Map<String, serde_json::Value>,
    meta: HashMeta,
}

/// Builds JSON Hash atlas metadata.
pub fn json_hash(
    plan: &SheetPlan,
    image_name: &str,
    base: &str,
    digits: u8,
    scale: u32,
    tags: &[FrameTag],
) -> Result<String, serde_json::Error> {
    let mut frames = serde_json::Map::new();
    for p in &plan.placements {
        let entry = HashFrame {
            frame: p.frame,
            rotated: false,
            trimmed: p.trimmed,
            sprite_source_size: p.source,
            source_size: Size {
                w: p.source_w,
                h: p.source_h,
            },
            duration: p.duration_ms,
        };
        frames.insert(
            frame_stem(base, p.order + 1, digits),
            serde_json::to_value(entry)?,
        );
    }
    let sheet = HashSheet {
        frames,
        meta: HashMeta {
            app: "https://github.com/senzukisu/kagero",
            version: crate::APP_VERSION,
            image: image_name.to_owned(),
            format: "RGBA8888",
            size: Size {
                w: plan.width,
                h: plan.height,
            },
            scale: scale.to_string(),
            frame_tags: sheet_tags(plan, tags)
                .into_iter()
                .map(|t| HashTag {
                    name: t.name,
                    from: t.from,
                    to: t.to,
                    direction: "forward",
                    // Aseprite writes #rrggbbaa.
                    color: format!("{}ff", t.color.to_lowercase()),
                })
                .collect(),
        },
    };
    serde_json::to_string_pretty(&sheet)
}

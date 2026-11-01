//! Export: every format is written to disk and read back with an independent decoder.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use image::RgbaImage;
use kagero::core::{Document, PixelBuffer, Rgba};
use kagero::export::render::{snapshot, FrameImage};
use kagero::export::service::{run, ExportRequest, ExportTarget};
use kagero::export::spritesheet::{compose, plan};
use kagero::export::{
    sequence, ExportError, GifLoop, GifOptions, JpegOptions, LayerSource, MetadataFormat,
    NamingOptions, SheetLayout, SheetOptions, StillFormat,
};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("kagero-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

const RED: Rgba = Rgba::opaque(255, 0, 0);
const GREEN: Rgba = Rgba::opaque(0, 200, 0);
const HALF_BLUE: Rgba = Rgba::new(0, 0, 255, 128);

/// 4×3 document, 3 frames: a red pixel moving right, plus a fixed semi-transparent
/// blue pixel on frame 1 and a green pixel on a hidden layer (must never be exported).
fn animated() -> Document {
    let mut cels_hidden = Vec::new();
    let mut frames = Vec::new();
    for f in 0..3 {
        let mut c = PixelBuffer::new(4, 3).expect("valid");
        c.set(f, 1, RED);
        if f == 0 {
            c.set(3, 2, HALF_BLUE);
        }
        let mut hidden = PixelBuffer::new(4, 3).expect("valid");
        hidden.set(0, 0, GREEN);
        cels_hidden.push(hidden.clone());
        frames.push((100 + 50 * f as u32, vec![c, hidden]));
    }
    Document::from_parts(
        4,
        3,
        vec![("Ink".into(), true), ("Hidden".into(), false)],
        frames,
        0,
        0,
    )
    .expect("valid")
}

fn request(frames: Vec<FrameImage>, scale: u32, target: ExportTarget) -> ExportRequest {
    ExportRequest {
        frames,
        scale,
        jpeg: JpegOptions::default(),
        gif: GifOptions::default(),
        sheet: SheetOptions::default(),
        base_name: "walk".into(),
        digits: 4,
        tags: Vec::new(),
        target,
    }
}

fn go(req: &ExportRequest) -> Result<kagero::export::ExportSummary, ExportError> {
    run(req, &AtomicBool::new(false), &mut |_, _| {})
}

fn all_frames(doc: &Document) -> Vec<FrameImage> {
    snapshot(
        doc,
        &(0..doc.frame_count()).collect::<Vec<_>>(),
        LayerSource::Visible,
    )
    .expect("snapshot")
}

// ---------------------------------------------------------------- snapshot

#[test]
fn snapshot_never_changes_the_document() {
    let doc = animated();
    let before = (doc.active_frame_index(), doc.composite().expect("c"));
    let frames = all_frames(&doc);
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[2].duration_ms, 200);
    assert_eq!(
        (doc.active_frame_index(), doc.composite().expect("c")),
        before
    );
    // Hidden layers are not part of the visible export.
    assert_eq!(frames[0].image.get_pixel(0, 0).0[3], 0);
    let active = snapshot(&doc, &[1], LayerSource::ActiveLayer).expect("active");
    assert_eq!(active[0].image.get_pixel(1, 1).0, RED.0);
    assert!(matches!(
        snapshot(&doc, &[], LayerSource::Visible),
        Err(ExportError::NoFrames)
    ));
    assert!(snapshot(&doc, &[9], LayerSource::Visible).is_err());
}

// ---------------------------------------------------------------- PNG

#[test]
fn png_round_trip_is_pixel_exact_with_transparency_and_scale() {
    let dir = temp_dir("exp-png");
    let doc = animated();
    let frames = all_frames(&doc);
    let path = dir.join("one.png");
    let req = request(
        vec![frames[0].clone()],
        3,
        ExportTarget::Images {
            format: StillFormat::Png,
            paths: vec![path.clone()],
        },
    );
    go(&req).expect("export");
    let img = image::open(&path).expect("decode").into_rgba8();
    assert_eq!(img.dimensions(), (12, 9));
    for y in 0..9 {
        for x in 0..12 {
            let src = frames[0].image.get_pixel(x / 3, y / 3).0;
            assert_eq!(img.get_pixel(x, y).0, src, "pixel {x},{y}");
        }
    }
    assert_eq!(
        img.get_pixel(9, 6).0,
        HALF_BLUE.0,
        "soft alpha kept exactly"
    );
}

#[test]
fn png_sequence_writes_numbered_files() {
    let dir = temp_dir("exp-seq");
    let doc = animated();
    let naming = NamingOptions::default();
    let names = sequence::file_names(&naming, "walk", 3, "png");
    let paths = sequence::paths_in(&dir, &names);
    assert!(sequence::existing(&paths).is_empty());
    let req = request(
        all_frames(&doc),
        1,
        ExportTarget::Images {
            format: StillFormat::Png,
            paths: paths.clone(),
        },
    );
    let summary = go(&req).expect("export");
    assert_eq!(summary.files, paths);
    for (i, p) in paths.iter().enumerate() {
        assert!(p.ends_with(format!("walk_{:04}.png", i + 1)));
        let img = image::open(p).expect("decode").into_rgba8();
        assert_eq!(img.get_pixel(i as u32, 1).0, RED.0);
    }
    assert_eq!(
        sequence::existing(&paths).len(),
        3,
        "conflicts are detected"
    );
}

// ---------------------------------------------------------------- JPEG

#[test]
fn jpeg_is_opaque_with_chosen_background() {
    let dir = temp_dir("exp-jpg");
    let doc = animated();
    let frames = all_frames(&doc);
    for (bg, name) in [
        (Rgba::WHITE, "white"),
        (Rgba::BLACK, "black"),
        (Rgba::opaque(20, 120, 220), "custom"),
    ] {
        let path = dir.join(format!("{name}.jpg"));
        let mut req = request(
            vec![frames[1].clone()],
            8,
            ExportTarget::Images {
                format: StillFormat::Jpeg,
                paths: vec![path.clone()],
            },
        );
        req.jpeg = JpegOptions {
            quality: 95,
            background: bg,
        };
        let summary = go(&req).expect("export");
        assert!(summary.notes.iter().any(|n| n.contains("background")));
        let img = image::open(&path).expect("decode");
        assert!(!img.color().has_alpha(), "JPEG has no alpha");
        let rgb = img.into_rgb8();
        // A pixel far from the red block: the background color (±JPEG error).
        let p = rgb.get_pixel(30, 2).0;
        for c in 0..3 {
            assert!(
                (i32::from(p[c]) - i32::from(bg.0[c])).abs() <= 6,
                "{name}: {p:?}"
            );
        }
        // Inside the red block.
        let r = rgb.get_pixel(12, 12).0;
        assert!(r[0] > 200 && r[1] < 60, "{name}: red {r:?}");
    }
}

#[test]
fn jpeg_quality_reaches_the_encoder() {
    let dir = temp_dir("exp-jpgq");
    let mut img = RgbaImage::new(64, 64);
    for (x, y, p) in img.enumerate_pixels_mut() {
        p.0 = [(x * 4) as u8, (y * 4) as u8, ((x ^ y) * 4) as u8, 255];
    }
    let frame = FrameImage {
        index: 0,
        duration_ms: 100,
        image: img,
    };
    let mut sizes = Vec::new();
    for q in [10u8, 95] {
        let path = dir.join(format!("q{q}.jpg"));
        let mut req = request(
            vec![frame.clone()],
            1,
            ExportTarget::Images {
                format: StillFormat::Jpeg,
                paths: vec![path.clone()],
            },
        );
        req.jpeg.quality = q;
        go(&req).expect("export");
        sizes.push(std::fs::metadata(&path).expect("file").len());
    }
    assert!(
        sizes[1] > sizes[0] * 2,
        "higher quality → bigger file: {sizes:?}"
    );
}

// ---------------------------------------------------------------- WebP

#[test]
fn webp_is_lossless_with_alpha() {
    let dir = temp_dir("exp-webp");
    let doc = animated();
    let frames = all_frames(&doc);
    let path = dir.join("f.webp");
    let req = request(
        vec![frames[0].clone()],
        2,
        ExportTarget::Images {
            format: StillFormat::WebP,
            paths: vec![path.clone()],
        },
    );
    go(&req).expect("export");
    let img = image::open(&path).expect("decode").into_rgba8();
    assert_eq!(img.dimensions(), (8, 6));
    assert_eq!(img.get_pixel(0, 2).0, RED.0);
    assert_eq!(
        img.get_pixel(6, 4).0,
        HALF_BLUE.0,
        "lossless keeps soft alpha"
    );
    assert_eq!(img.get_pixel(7, 0).0[3], 0);
}

// ---------------------------------------------------------------- GIF

/// Loop setting, (delay, indices) per frame, and the logical screen size.
type GifDump = (gif::Repeat, Vec<(u16, Vec<u8>)>, (u16, u16));

fn read_gif(path: &std::path::Path) -> GifDump {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut dec = opts
        .read_info(std::fs::File::open(path).expect("open"))
        .expect("valid GIF");
    let size = (dec.width(), dec.height());
    let mut frames = Vec::new();
    while let Some(f) = dec.read_next_frame().expect("frame") {
        frames.push((f.delay, f.buffer.to_vec()));
    }
    (dec.repeat(), frames, size)
}

#[test]
fn gif_frames_durations_loop_and_transparency() {
    let dir = temp_dir("exp-gif");
    let doc = animated();
    let path = dir.join("walk.gif");
    let req = request(
        all_frames(&doc),
        2,
        ExportTarget::Gif { path: path.clone() },
    );
    let summary = go(&req).expect("export");
    assert!(summary.notes.iter().any(|n| n.contains("Semi-transparent")));
    let (repeat, frames, size) = read_gif(&path);
    assert_eq!(repeat, gif::Repeat::Infinite);
    assert_eq!(size, (8, 6));
    assert_eq!(frames.len(), 3);
    assert_eq!(
        frames.iter().map(|f| f.0).collect::<Vec<_>>(),
        vec![10, 15, 20]
    );
    for (i, (_, rgba)) in frames.iter().enumerate() {
        let px = |x: usize, y: usize| &rgba[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4];
        assert_eq!(px(i * 2, 2), RED.0, "frame {i}: red block exact");
        assert_eq!(
            px(0, 0)[3],
            0,
            "frame {i}: transparent background, no hidden layer"
        );
    }
    // The 50%-alpha blue pixel (alpha 128 ≥ threshold) becomes opaque blue, no halo.
    assert_eq!(
        &frames[0].1[(4 * 8 + 6) * 4..(4 * 8 + 6) * 4 + 4],
        [0, 0, 255, 255]
    );
}

#[test]
fn gif_play_once_and_threshold() {
    let dir = temp_dir("exp-gif-once");
    let doc = animated();
    let path = dir.join("once.gif");
    let mut req = request(
        all_frames(&doc),
        1,
        ExportTarget::Gif { path: path.clone() },
    );
    req.gif = GifOptions {
        looping: GifLoop::Once,
        alpha_threshold: 200,
        ..GifOptions::default()
    };
    go(&req).expect("export");
    let (repeat, frames, _) = read_gif(&path);
    assert_eq!(
        repeat,
        gif::Repeat::Finite(0),
        "no loop extension = play once"
    );
    assert_eq!(
        frames[0].1[(2 * 4 + 3) * 4 + 3],
        0,
        "alpha 128 < 200 → transparent"
    );
}

#[test]
fn gif_reduces_more_than_255_colors() {
    let dir = temp_dir("exp-gif-many");
    let mut img = RgbaImage::new(32, 32);
    for (x, y, p) in img.enumerate_pixels_mut() {
        p.0 = [(x * 8) as u8, (y * 8) as u8, 128, 255];
    }
    let frame = FrameImage {
        index: 0,
        duration_ms: 100,
        image: img,
    };
    for dither in [false, true] {
        let path = dir.join(format!("many-{dither}.gif"));
        let mut req = request(
            vec![frame.clone()],
            1,
            ExportTarget::Gif { path: path.clone() },
        );
        req.gif.dither = dither;
        let summary = go(&req).expect("export");
        assert!(summary.notes.iter().any(|n| n.contains("255")));
        let (_, frames, _) = read_gif(&path);
        let rgba = &frames[0].1;
        // Colors stay close to the source.
        let err: i64 = rgba
            .chunks(4)
            .zip(frame.image.pixels())
            .map(|(a, b)| {
                (0..3)
                    .map(|c| (i64::from(a[c]) - i64::from(b.0[c])).abs())
                    .sum::<i64>()
            })
            .sum::<i64>()
            / 1024;
        assert!(err < 40, "average error {err}");
    }
}

#[test]
fn gif_rejects_mismatched_frames_and_can_be_canceled() {
    let small = FrameImage {
        index: 0,
        duration_ms: 100,
        image: RgbaImage::new(2, 2),
    };
    let big = FrameImage {
        index: 1,
        duration_ms: 100,
        image: RgbaImage::new(3, 2),
    };
    let dir = temp_dir("exp-gif-bad");
    let req = request(
        vec![small.clone(), big],
        1,
        ExportTarget::Gif {
            path: dir.join("x.gif"),
        },
    );
    assert!(matches!(go(&req), Err(ExportError::Invalid(_))));
    let req = request(
        vec![small],
        1,
        ExportTarget::Gif {
            path: dir.join("y.gif"),
        },
    );
    let err = run(&req, &AtomicBool::new(true), &mut |_, _| {}).expect_err("canceled");
    assert!(matches!(err, ExportError::Canceled));
    assert!(!dir.join("y.gif").exists(), "nothing written when canceled");
}

// ---------------------------------------------------------------- sprite sheets

fn sheet(layout: SheetLayout) -> SheetOptions {
    SheetOptions {
        layout,
        columns: 2,
        rows: 2,
        ..SheetOptions::default()
    }
}

fn assert_no_overlap(p: &kagero::export::spritesheet::SheetPlan, extrude: u32) {
    for (i, a) in p.placements.iter().enumerate() {
        for b in &p.placements[i + 1..] {
            let (ax0, ax1) = (a.frame.x - extrude, a.frame.x + a.frame.w + extrude);
            let (bx0, bx1) = (b.frame.x - extrude, b.frame.x + b.frame.w + extrude);
            let (ay0, ay1) = (a.frame.y - extrude, a.frame.y + a.frame.h + extrude);
            let (by0, by1) = (b.frame.y - extrude, b.frame.y + b.frame.h + extrude);
            let overlap = ax0 < bx1 && bx0 < ax1 && ay0 < by1 && by0 < ay1;
            assert!(!overlap, "frames {} and {} overlap", a.order, b.order);
        }
        assert!(a.frame.x + a.frame.w + extrude <= p.width);
        assert!(a.frame.y + a.frame.h + extrude <= p.height);
    }
}

#[test]
fn sheet_layouts_sizes_and_positions() {
    let frames = all_frames(&animated());
    let h = plan(&frames, &sheet(SheetLayout::Horizontal)).expect("plan");
    assert_eq!((h.width, h.height), (12, 3));
    assert_eq!(h.placements[2].frame.x, 8);
    let v = plan(&frames, &sheet(SheetLayout::Vertical)).expect("plan");
    assert_eq!((v.width, v.height), (4, 9));
    assert_eq!(v.placements[1].frame.y, 3);
    let g = plan(&frames, &sheet(SheetLayout::Columns)).expect("plan");
    assert_eq!((g.columns, g.rows, g.width, g.height), (2, 2, 8, 6));
    assert_eq!((g.placements[2].frame.x, g.placements[2].frame.y), (0, 3));
    for p in [&h, &v, &g] {
        assert_no_overlap(p, 0);
    }
}

#[test]
fn sheet_padding_spacing_extrude() {
    let frames = all_frames(&animated());
    let o = SheetOptions {
        padding: 2,
        spacing: 3,
        extrude: 1,
        ..sheet(SheetLayout::Horizontal)
    };
    let p = plan(&frames, &o).expect("plan");
    // 2 + (1+4+1) + 3 + (1+4+1) + 3 + (1+4+1) + 2
    assert_eq!((p.width, p.height), (2 + 18 + 6 + 2, 2 + 5 + 2));
    assert_eq!((p.placements[0].frame.x, p.placements[0].frame.y), (3, 3));
    assert_eq!(p.placements[1].frame.x, 3 + 4 + 1 + 3 + 1);
    assert_no_overlap(&p, 1);
    let img = compose(&frames, &p, &o).expect("compose");
    // Frame 0's red pixel at (0,1) is extruded to the left border pixel.
    assert_eq!(img.get_pixel(3, 4).0, RED.0);
    assert_eq!(img.get_pixel(2, 4).0, RED.0, "extruded edge");
    assert_eq!(img.get_pixel(0, 0).0[3], 0, "padding stays transparent");
}

#[test]
fn sheet_trim_keeps_offsets_and_equal_cells() {
    let frames = all_frames(&animated());
    let o = SheetOptions {
        trim: true,
        ..sheet(SheetLayout::Horizontal)
    };
    let p = plan(&frames, &o).expect("plan");
    // Frame 0 spans (0,1)–(3,2) because of the blue pixel; frames 1–2 are one pixel.
    assert_eq!(
        p.placements[0].source,
        kagero::export::spritesheet::Rect {
            x: 0,
            y: 1,
            w: 4,
            h: 2
        }
    );
    assert_eq!(
        p.placements[1].source,
        kagero::export::spritesheet::Rect {
            x: 1,
            y: 1,
            w: 1,
            h: 1
        }
    );
    assert!(p.placements[1].trimmed);
    assert_eq!(
        (p.cell_w, p.cell_h),
        (4, 2),
        "cells use the largest trimmed size"
    );
    assert_eq!((p.placements[1].source_w, p.placements[1].source_h), (4, 3));
    let img = compose(&frames, &p, &o).expect("compose");
    assert_eq!(
        img.get_pixel(p.placements[1].frame.x, p.placements[1].frame.y)
            .0,
        RED.0
    );
}

#[test]
fn sheet_size_overflow_is_refused_before_allocating() {
    let frame = FrameImage {
        index: 0,
        duration_ms: 100,
        image: RgbaImage::new(2048, 2048),
    };
    let frames = vec![frame; 9];
    let err = plan(&frames, &sheet(SheetLayout::Horizontal)).expect_err("too wide");
    assert!(matches!(err, ExportError::TooLarge { .. }));
}

#[test]
fn sheet_with_json_metadata_round_trips() {
    let dir = temp_dir("exp-sheet");
    let doc = animated();
    for (fmt, name) in [
        (MetadataFormat::Kagero, "k"),
        (MetadataFormat::JsonHash, "h"),
    ] {
        let image_path = dir.join(format!("{name}.png"));
        let json_path = dir.join(format!("{name}.json"));
        let mut req = request(
            all_frames(&doc),
            2,
            ExportTarget::Sheet {
                image_path: image_path.clone(),
                json_path: Some(json_path.clone()),
            },
        );
        req.sheet = SheetOptions {
            metadata: fmt,
            spacing: 1,
            ..sheet(SheetLayout::Columns)
        };
        let summary = go(&req).expect("export");
        assert_eq!(summary.files, vec![image_path.clone(), json_path.clone()]);
        let img = image::open(&image_path).expect("png").into_rgba8();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_path).expect("json"))
                .expect("parse");
        let rects: Vec<(serde_json::Value, String, u64)> = match fmt {
            MetadataFormat::Kagero => {
                assert_eq!(json["format"], "kagero-sprite-sheet");
                assert_eq!(json["version"], 1);
                assert_eq!(json["frameCount"], 3);
                assert_eq!(json["totalDuration"], 450);
                assert_eq!(json["image"], "k.png");
                json["frames"]
                    .as_array()
                    .expect("array")
                    .iter()
                    .map(|f| {
                        (
                            f["frame"].clone(),
                            f["name"].as_str().expect("name").to_owned(),
                            f["duration"].as_u64().expect("d"),
                        )
                    })
                    .collect()
            }
            _ => {
                assert_eq!(json["meta"]["image"], "h.png");
                assert_eq!(json["meta"]["size"]["w"], img.width());
                json["frames"]
                    .as_object()
                    .expect("object")
                    .iter()
                    .map(|(k, f)| {
                        assert_eq!(f["rotated"], false);
                        (
                            f["frame"].clone(),
                            k.clone(),
                            f["duration"].as_u64().expect("d"),
                        )
                    })
                    .collect()
            }
        };
        assert_eq!(rects.len(), 3);
        for (i, (r, frame_name, duration)) in rects.iter().enumerate() {
            assert_eq!(frame_name, &format!("walk_{:04}", i + 1), "order is kept");
            assert_eq!(*duration, 100 + 50 * i as u64);
            let (x, y) = (
                r["x"].as_u64().expect("x") as u32,
                r["y"].as_u64().expect("y") as u32,
            );
            assert_eq!((r["w"].as_u64(), r["h"].as_u64()), (Some(8), Some(6)));
            // The JSON rect points at the frame: its red block is at (2i, 2) scaled ×2.
            assert_eq!(img.get_pixel(x + 2 * i as u32, y + 2).0, RED.0, "frame {i}");
        }
    }
}

#[test]
fn export_requests_list_their_outputs() {
    let req = request(
        vec![],
        1,
        ExportTarget::Sheet {
            image_path: PathBuf::from("a.png"),
            json_path: Some(PathBuf::from("a.json")),
        },
    );
    assert_eq!(req.output_paths().len(), 2);
    assert!(matches!(go(&req), Err(ExportError::NoFrames)));
}

#[test]
fn sheet_metadata_lists_tags_as_sheet_positions() {
    use kagero::core::FrameTag;
    let dir = temp_dir("exp-sheet-tags");
    let doc = animated();
    let tags = vec![
        FrameTag {
            name: "walk".into(),
            start: 0,
            end: 1,
            color: Rgba::opaque(0xE8, 0x6A, 0x33),
        },
        FrameTag {
            name: "end".into(),
            start: 2,
            end: 2,
            color: Rgba::opaque(0x4A, 0x8F, 0xD8),
        },
    ];
    // Only frames 1 and 2 are exported: "walk" keeps frame 1 (sheet position 0) and
    // "end" maps to sheet position 1.
    let frames = snapshot(&doc, &[1, 2], LayerSource::Visible).expect("snapshot");
    for (fmt, name) in [
        (MetadataFormat::Kagero, "k"),
        (MetadataFormat::JsonHash, "h"),
    ] {
        let json_path = dir.join(format!("{name}.json"));
        let mut req = request(
            frames.clone(),
            1,
            ExportTarget::Sheet {
                image_path: dir.join(format!("{name}.png")),
                json_path: Some(json_path.clone()),
            },
        );
        req.tags = tags.clone();
        req.sheet = SheetOptions {
            metadata: fmt,
            ..sheet(SheetLayout::Horizontal)
        };
        go(&req).expect("export");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_path).expect("json"))
                .expect("parse");
        let list = match fmt {
            MetadataFormat::Kagero => json["tags"].clone(),
            _ => json["meta"]["frameTags"].clone(),
        };
        let list = list.as_array().expect("tags").clone();
        assert_eq!(list.len(), 2, "{json}");
        assert_eq!(list[0]["name"], "walk");
        assert_eq!(
            (list[0]["from"].as_u64(), list[0]["to"].as_u64()),
            (Some(0), Some(0))
        );
        assert_eq!(
            (list[1]["from"].as_u64(), list[1]["to"].as_u64()),
            (Some(1), Some(1))
        );
        match fmt {
            MetadataFormat::Kagero => assert_eq!(list[1]["color"], "#4A8FD8"),
            _ => {
                assert_eq!(list[1]["color"], "#4a8fd8ff");
                assert_eq!(list[1]["direction"], "forward");
            }
        }
    }
    // Without tags, neither format writes the field.
    let json_path = dir.join("none.json");
    let req = request(
        frames,
        1,
        ExportTarget::Sheet {
            image_path: dir.join("none.png"),
            json_path: Some(json_path.clone()),
        },
    );
    go(&req).expect("export");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_path).expect("json")).expect("parse");
    assert!(json.get("tags").is_none());
}

// ---------------------------------------------------------------- animated WebP

/// Decodes an animated WebP with the image crate (an independent decoder).
fn read_webp_animation(path: &std::path::Path) -> Vec<(RgbaImage, u32)> {
    use image::AnimationDecoder;
    let file = std::io::BufReader::new(std::fs::File::open(path).expect("open"));
    let decoder = image::codecs::webp::WebPDecoder::new(file).expect("webp");
    decoder
        .into_frames()
        .collect_frames()
        .expect("frames")
        .into_iter()
        .map(|f| {
            let (num, den) = f.delay().numer_denom_ms();
            (f.into_buffer(), num / den.max(1))
        })
        .collect()
}

#[test]
fn animated_webp_is_lossless_with_exact_durations_and_alpha() {
    let dir = temp_dir("exp-webp-anim");
    let doc = animated();
    let path = dir.join("walk.webp");
    let mut req = request(
        all_frames(&doc),
        2,
        ExportTarget::WebPAnimation { path: path.clone() },
    );
    req.gif.looping = GifLoop::Forever;
    let summary = go(&req).expect("export");
    assert_eq!(summary.files, vec![path.clone()]);
    let bytes = std::fs::read(&path).expect("read");
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..16], b"WEBPVP8X");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().expect("4")) as usize,
        bytes.len() - 8,
        "RIFF size"
    );
    let frames = read_webp_animation(&path);
    assert_eq!(frames.len(), 3);
    let expected = all_frames(&doc);
    for (i, ((img, ms), src)) in frames.iter().zip(&expected).enumerate() {
        assert_eq!(*ms, 100 + 50 * i as u32, "frame {i} duration");
        assert_eq!(img.dimensions(), (8, 6));
        for y in 0..6 {
            for x in 0..8 {
                assert_eq!(
                    img.get_pixel(x, y).0,
                    src.image.get_pixel(x / 2, y / 2).0,
                    "frame {i} pixel ({x},{y})"
                );
            }
        }
    }
    // Semi-transparent pixels survive exactly (GIF can't do this).
    assert_eq!(frames[0].0.get_pixel(6, 4).0, HALF_BLUE.0);
    // Transparent areas of a later frame stay transparent (no blending with frame 0).
    assert_eq!(frames[1].0.get_pixel(6, 4).0[3], 0);
}

#[test]
fn animated_webp_loop_count_and_limits() {
    let dir = temp_dir("exp-webp-loop");
    let doc = animated();
    let path = dir.join("once.webp");
    let mut req = request(
        all_frames(&doc),
        1,
        ExportTarget::WebPAnimation { path: path.clone() },
    );
    req.gif.looping = GifLoop::Once;
    go(&req).expect("export");
    let bytes = std::fs::read(&path).expect("read");
    let anim = bytes
        .windows(4)
        .position(|w| w == b"ANIM")
        .expect("ANIM chunk");
    // Payload: 4 bytes background, then the loop count.
    assert_eq!(u16::from_le_bytes([bytes[anim + 12], bytes[anim + 13]]), 1);
    // Frames of different sizes are refused.
    let mut frames = all_frames(&doc);
    frames[1].image = RgbaImage::new(5, 3);
    let bad = request(
        frames,
        1,
        ExportTarget::WebPAnimation {
            path: dir.join("bad.webp"),
        },
    );
    assert!(matches!(go(&bad), Err(ExportError::Invalid(_))));
    assert!(!dir.join("bad.webp").exists(), "nothing written on error");
}

// ---------------------------------------------------------------- packed sheets

/// Frames of different content sizes, with frame 3 identical to frame 1.
fn varied_frames() -> Vec<FrameImage> {
    let mut out = Vec::new();
    for (i, (w, h)) in [(5, 2), (2, 6), (7, 7), (2, 6), (1, 1)]
        .into_iter()
        .enumerate()
    {
        let mut img = RgbaImage::new(10, 10);
        let shade = if i == 3 { 1 } else { i as u8 };
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(
                    x + 1,
                    y + 2,
                    image::Rgba([40 * shade + 10, x as u8 * 20, y as u8 * 20, 255]),
                );
            }
        }
        out.push(FrameImage {
            index: i,
            duration_ms: 100,
            image: img,
        });
    }
    out
}

fn packed(spacing: u32, extrude: u32, padding: u32, merge: bool) -> SheetOptions {
    SheetOptions {
        packed: true,
        merge_duplicates: merge,
        spacing,
        extrude,
        padding,
        ..SheetOptions::default()
    }
}

#[test]
fn packed_sheets_are_tight_deterministic_and_never_overlap() {
    let frames = varied_frames();
    for (spacing, extrude, padding) in [(0, 0, 0), (1, 0, 0), (2, 1, 3)] {
        let o = packed(spacing, extrude, padding, false);
        let p = plan(&frames, &o).expect("plan");
        assert_eq!(
            p,
            plan(&frames, &o).expect("plan"),
            "same input, same sheet"
        );
        assert_no_overlap(&p, extrude);
        assert_eq!((p.columns, p.rows), (0, 0));
        // Much smaller than a strip of the full frames with the same spacing.
        let strip = plan(&frames, &SheetOptions { packed: false, ..o }).expect("strip");
        let area =
            |p: &kagero::export::spritesheet::SheetPlan| u64::from(p.width) * u64::from(p.height);
        assert!(
            area(&p) * 2 < area(&strip),
            "{}×{} vs {}×{}",
            p.width,
            p.height,
            strip.width,
            strip.height
        );
        // At least `spacing` empty pixels between any two frames (with extrude).
        let e = i64::from(extrude);
        for (i, a) in p.placements.iter().enumerate() {
            for b in &p.placements[i + 1..] {
                let (ax0, ay0) = (i64::from(a.frame.x) - e, i64::from(a.frame.y) - e);
                let (ax1, ay1) = (
                    ax0 + i64::from(a.frame.w) + 2 * e,
                    ay0 + i64::from(a.frame.h) + 2 * e,
                );
                let (bx0, by0) = (i64::from(b.frame.x) - e, i64::from(b.frame.y) - e);
                let (bx1, by1) = (
                    bx0 + i64::from(b.frame.w) + 2 * e,
                    by0 + i64::from(b.frame.h) + 2 * e,
                );
                let gap = (bx0 - ax1).max(ax0 - bx1).max(by0 - ay1).max(ay0 - by1);
                assert!(
                    gap >= i64::from(spacing),
                    "frames {} and {} are {gap} px apart",
                    a.order,
                    b.order
                );
            }
            assert!(a.frame.x >= padding + extrude && a.frame.y >= padding + extrude);
        }
        // Every frame's trimmed pixels are on the sheet where the plan says.
        let img = compose(&frames, &p, &o).expect("compose");
        for (f, pl) in frames.iter().zip(&p.placements) {
            assert!(pl.trimmed);
            for y in 0..pl.frame.h {
                for x in 0..pl.frame.w {
                    assert_eq!(
                        img.get_pixel(pl.frame.x + x, pl.frame.y + y),
                        f.image.get_pixel(pl.source.x + x, pl.source.y + y)
                    );
                }
            }
        }
    }
}

#[test]
fn packed_sheets_merge_identical_frames() {
    let frames = varied_frames();
    let merged = plan(&frames, &packed(0, 0, 0, true)).expect("plan");
    let separate = plan(&frames, &packed(0, 0, 0, false)).expect("plan");
    assert_eq!(merged.placements[3].duplicate_of, Some(1));
    assert_eq!(merged.placements[3].frame, merged.placements[1].frame);
    assert_eq!(
        merged
            .placements
            .iter()
            .filter(|p| p.duplicate_of.is_some())
            .count(),
        1
    );
    assert!(
        u64::from(merged.width) * u64::from(merged.height)
            <= u64::from(separate.width) * u64::from(separate.height)
    );
    // The metadata says which frame is shared.
    let dir = temp_dir("exp-packed");
    let json_path = dir.join("p.json");
    let mut req = request(
        frames,
        1,
        ExportTarget::Sheet {
            image_path: dir.join("p.png"),
            json_path: Some(json_path.clone()),
        },
    );
    req.sheet = packed(0, 0, 0, true);
    go(&req).expect("export");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_path).expect("json")).expect("parse");
    assert_eq!(json["layout"], "packed");
    assert_eq!(json["frames"][3]["duplicateOf"], 1);
    assert!(json["frames"][2].get("duplicateOf").is_none());
}

#[test]
fn packed_sizes_match_after_scaling_and_respect_limits() {
    use kagero::export::spritesheet::plan_scaled;
    let frames = varied_frames();
    let o = packed(1, 1, 2, true);
    // The size shown before exporting equals the size of the scaled export.
    let scaled: Vec<FrameImage> = frames
        .iter()
        .map(|f| FrameImage {
            image: kagero::export::render::scale_nearest(&f.image, 3).expect("scale"),
            ..f.clone()
        })
        .collect();
    assert_eq!(
        plan_scaled(&frames, 3, &o).expect("plan"),
        plan(&scaled, &o).expect("plan")
    );
    // Too big for any sheet: refused before allocating.
    assert!(matches!(
        plan_scaled(&frames, 4000, &o),
        Err(ExportError::TooLarge { .. })
    ));
}

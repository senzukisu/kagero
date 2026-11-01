//! Performance benchmarks for the hot paths of Kagero (no window needed).
//!
//! `cargo run --release --example bench` prints the median time of each case (and the
//! memory of the undo history for edits). Compare runs before and after a change on
//! the same machine; the numbers are only meaningful relative to each other.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use kagero::core::{Document, PixelBuffer, Rgba};
use kagero::editor::{ColorSlot, Editor, ToolKind};
use kagero::export::render::snapshot;
use kagero::export::service::{run, ExportRequest, ExportTarget};
use kagero::export::{
    GifOptions, JpegOptions, LayerSource, SheetLayout, SheetOptions, StillFormat,
};

/// Deterministic "drawing-like" content: shapes, gradients and transparent holes.
fn cel(w: u32, h: u32, seed: u32) -> PixelBuffer {
    let mut b = PixelBuffer::new(w, h).expect("size");
    let mut s = seed.wrapping_mul(2_654_435_761).max(1);
    for y in 0..h {
        for x in 0..w {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            let inside = ((x as i32 - (w / 2) as i32).pow(2) + (y as i32 - (h / 2) as i32).pow(2))
                < ((w.min(h) / 3).pow(2) as i32 + (seed % 7) as i32 * 40);
            let a = if inside {
                255
            } else if s % 5 == 0 {
                120
            } else {
                0
            };
            b.set(
                x as i32,
                y as i32,
                Rgba::new((x * 3 + seed) as u8, (y * 5) as u8, (s >> 8) as u8, a),
            );
        }
    }
    b
}

fn document(w: u32, h: u32, layers: usize, frames: usize) -> Document {
    let infos = (0..layers).map(|i| (format!("Layer {i}"), true)).collect();
    let frames = (0..frames)
        .map(|f| {
            (
                100,
                (0..layers)
                    .map(|l| cel(w, h, (f * 31 + l) as u32))
                    .collect(),
            )
        })
        .collect();
    Document::from_parts(w, h, infos, frames, 0, 0).expect("document")
}

/// Median of `runs` timings of `f`, in milliseconds.
fn time(runs: usize, mut f: impl FnMut()) -> f64 {
    let mut v: Vec<f64> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn row(name: &str, ms: f64, extra: &str) {
    println!("{name:<46} {ms:>10.2} ms  {extra}");
}

fn main() {
    let dir: PathBuf = std::env::temp_dir().join(format!("kagero-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    println!("Kagero {} benchmarks (median)\n", kagero::APP_VERSION);

    // Compositing: canvas redraw of a full frame, and a small dirty rectangle.
    let big = document(512, 512, 16, 2);
    row(
        "composite 512² × 16 layers (full frame)",
        time(15, || {
            std::hint::black_box(big.composite_frame(0).expect("c"));
        }),
        "",
    );
    row(
        "composite 512² × 16 layers (64² dirty rect)",
        time(51, || {
            std::hint::black_box(big.composite_frame_rect(0, 100, 100, 64, 64));
        }),
        "",
    );
    let one = document(1024, 1024, 1, 1);
    row(
        "composite 1024² × 1 layer (full frame)",
        time(15, || {
            std::hint::black_box(one.composite_frame(0).expect("c"));
        }),
        "",
    );

    // Animation: thumbnails of a 120-frame timeline, and playback frame switches.
    let anim = document(128, 128, 4, 120);
    row(
        "timeline thumbnails 120 frames 128² × 4",
        time(5, || {
            for f in 0..anim.frame_count() {
                std::hint::black_box(kagero::ui::timeline::thumbnail(&anim, f));
            }
        }),
        "",
    );
    row(
        "onion skin: composite 2 neighbor frames",
        time(31, || {
            std::hint::black_box(anim.composite_frame(9).expect("c"));
            std::hint::black_box(anim.composite_frame(11).expect("c"));
        }),
        "",
    );

    // Projects: save and open.
    let project = document(256, 256, 8, 40);
    let path = dir.join("bench.kgr");
    row(
        "save .kgr 256² × 8 layers × 40 frames",
        time(3, || {
            kagero::file_io::save_kgr(&project, &path).expect("save");
        }),
        &format!(
            "{:.1} MB",
            std::fs::metadata(&path).map_or(0, |m| m.len()) as f64 / 1e6
        ),
    );
    row(
        "open .kgr 256² × 8 layers × 40 frames",
        time(3, || {
            std::hint::black_box(kagero::file_io::load_kgr(&path).expect("load"));
        }),
        "",
    );

    // Export: snapshot + PNG sheet at ×4, and an animated GIF.
    let indices: Vec<usize> = (0..anim.frame_count()).collect();
    row(
        "export snapshot 120 frames 128² × 4",
        time(5, || {
            std::hint::black_box(snapshot(&anim, &indices, LayerSource::Visible).expect("snap"));
        }),
        "",
    );
    let frames = snapshot(&anim, &indices, LayerSource::Visible).expect("snap");
    let request = |target| ExportRequest {
        frames: frames.clone(),
        scale: 4,
        jpeg: JpegOptions::default(),
        gif: GifOptions::default(),
        sheet: SheetOptions {
            layout: SheetLayout::Columns,
            columns: 12,
            image_format: StillFormat::Png,
            ..SheetOptions::default()
        },
        base_name: "bench".into(),
        digits: 4,
        tags: Vec::new(),
        target,
    };
    let sheet = request(ExportTarget::Sheet {
        image_path: dir.join("sheet.png"),
        json_path: Some(dir.join("sheet.json")),
    });
    row(
        "export PNG sheet 120 frames ×4 (6144×5120)",
        time(3, || {
            run(&sheet, &AtomicBool::new(false), &mut |_, _| {}).expect("sheet");
        }),
        "",
    );
    let gif = request(ExportTarget::Gif {
        path: dir.join("anim.gif"),
    });
    row(
        "export GIF 120 frames ×4",
        time(3, || {
            run(&gif, &AtomicBool::new(false), &mut |_, _| {}).expect("gif");
        }),
        "",
    );
    let pngs = request(ExportTarget::Images {
        format: StillFormat::Png,
        paths: (0..120).map(|i| dir.join(format!("f{i}.png"))).collect(),
    });
    row(
        "export 120 PNG files ×4",
        time(3, || {
            run(&pngs, &AtomicBool::new(false), &mut |_, _| {}).expect("pngs");
        }),
        "",
    );

    // Editing: big fill and a long pencil stroke, with the history's memory.
    let fill_ms = time(5, || {
        let doc = Document::new(2048, 2048, Rgba::TRANSPARENT).expect("doc");
        let mut e = Editor::new(doc, None, "b".into());
        e.fill((5, 5), Rgba::opaque(200, 10, 10)).expect("fill");
        std::hint::black_box(e.history().bytes());
    });
    let doc = Document::new(2048, 2048, Rgba::TRANSPARENT).expect("doc");
    let mut e = Editor::new(doc, None, "b".into());
    e.fill((5, 5), Rgba::opaque(200, 10, 10)).expect("fill");
    row(
        "fill 2048² (new editor + fill)",
        fill_ms,
        &format!("history {:.1} MB", e.history().bytes() as f64 / 1e6),
    );
    let stroke_ms = time(5, || {
        let doc = Document::new(1024, 1024, Rgba::TRANSPARENT).expect("doc");
        let mut e = Editor::new(doc, None, "b".into());
        e.set_tool(ToolKind::Pencil);
        e.settings.brush_size = 8;
        e.begin_stroke((0, 0), ColorSlot::Primary).expect("begin");
        for i in 0..2000 {
            e.update_stroke(((i * 7) % 1024, (i * 13) % 1024));
        }
        e.end_stroke();
    });
    row(
        "pencil stroke 2000 points, 8 px brush, 1024²",
        stroke_ms,
        "",
    );

    let _ = std::fs::remove_dir_all(&dir);
}

//! Long, mixed editing sessions: every operation must undo and redo exactly, and the
//! document must survive saving and opening unchanged.

use std::io::Cursor;

use kagero::core::selection::{SelRect, SelectMode};
use kagero::core::{Anchor, Document, Rgba};
use kagero::editor::{ColorSlot, Editor, PaletteScope, ToolKind, Transform};
use kagero::file_io::kgr::{read_kgr, write_kgr};

/// Small deterministic random numbers (no external crates).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    fn point(&mut self, doc: &Document) -> (i32, i32) {
        (
            self.below(u64::from(doc.width()) + 6) as i32 - 3,
            self.below(u64::from(doc.height()) + 6) as i32 - 3,
        )
    }

    fn color(&mut self) -> Rgba {
        let v = self.next();
        Rgba::new(
            v as u8,
            (v >> 8) as u8,
            (v >> 16) as u8,
            [0, 90, 255, 255][(v >> 24) as usize % 4],
        )
    }
}

/// Everything a document is made of that saving and undo must preserve.
fn fingerprint(doc: &Document) -> String {
    let mut s = format!("{}x{}|", doc.width(), doc.height());
    for l in doc.layers() {
        s += &format!(
            "L{}:{}:{}:{}:{}:{}|",
            l.id.0, l.name, l.visible, l.opacity, l.locked, l.alpha_lock
        );
    }
    for (fi, f) in doc.frames().iter().enumerate() {
        s += &format!("F{}:{}:", f.id.0, f.duration_ms());
        for li in 0..doc.layer_count() {
            let bytes = doc
                .cel(fi, li)
                .map(|c| c.as_bytes().to_vec())
                .unwrap_or_default();
            // FNV-1a of the pixels: different images give different prints.
            let h = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
                (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
            });
            s += &format!("{h:016x},");
        }
        s += "|";
    }
    for t in doc.tags() {
        s += &format!("T{}:{}-{}:{}|", t.name, t.start, t.end, t.color.to_hex());
    }
    s
}

/// Applies one random operation. Operations that are refused (a locked layer, the
/// last frame…) are fine: they must change nothing.
fn random_step(e: &mut Editor, rng: &mut Rng) -> &'static str {
    let doc = e.document();
    let (layers, frames) = (doc.layer_count(), doc.frame_count());
    let active = doc.active_index();
    let op = rng.below(24);
    let name = OPS[op as usize];
    match op {
        0..=3 => {
            let tools = [
                ToolKind::Pencil,
                ToolKind::Eraser,
                ToolKind::Line,
                ToolKind::Rectangle,
                ToolKind::FilledRectangle,
                ToolKind::Ellipse,
            ];
            e.set_tool(tools[rng.below(tools.len() as u64) as usize]);
            e.settings.brush_size = 1 + rng.below(4) as u32;
            e.settings.mirror_x = rng.below(3) == 0;
            e.primary = rng.color();
            let start = rng.point(e.document());
            if e.begin_stroke(start, ColorSlot::Primary).is_ok() {
                for _ in 0..rng.below(6) {
                    let p = rng.point(e.document());
                    e.update_stroke(p);
                }
                // Sometimes switch tools in the middle of a stroke: it must still end
                // as one complete undo step.
                if rng.below(4) == 0 {
                    e.set_tool(ToolKind::Fill);
                }
                e.end_stroke();
            }
        }
        4 => {
            e.settings.fill_tolerance = rng.below(40) as u8;
            e.settings.fill_contiguous = rng.below(2) == 0;
            let p = rng.point(e.document());
            let c = rng.color();
            let _ = e.fill(p, c);
        }
        5 => {
            e.set_tool(ToolKind::Polygon);
            for _ in 0..2 + rng.below(3) {
                let p = rng.point(e.document());
                let _ = e.polygon_click(p, ColorSlot::Primary);
            }
            // Leave it open and switch tools: the polygon is finished, not lost.
            e.set_tool(ToolKind::Pencil);
        }
        6 => {
            let _ = e.add_layer();
        }
        7 => {
            let _ = e.duplicate_layer(active);
        }
        8 => {
            let _ = e.merge_down(active);
        }
        9 => {
            let _ = e.delete_layer(rng.below(layers as u64) as usize);
        }
        10 => {
            let _ = e.move_layer_up(active);
        }
        11 => {
            let i = rng.below(layers as u64) as usize;
            let mut props = e.document().layers()[i].props();
            match rng.below(3) {
                0 => props.opacity = rng.below(256) as u8,
                1 => props.locked = !props.locked,
                _ => props.alpha_lock = !props.alpha_lock,
            }
            let _ = e.set_layer_props(i, props);
        }
        12 => e.select_layer(rng.below(layers as u64) as usize),
        13 => {
            let _ = if rng.below(2) == 0 {
                e.add_frame()
            } else {
                e.duplicate_frame()
            };
        }
        14 => {
            let _ = e.delete_frame(rng.below(frames as u64) as usize);
        }
        15 => {
            let _ = e.move_frame_left(rng.below(frames as u64) as usize);
        }
        16 => e.select_frame(rng.below(frames as u64) as usize),
        17 => {
            let a = rng.below(frames as u64) as usize;
            let b = rng.below(frames as u64) as usize;
            let _ = e.add_tag(a, b);
        }
        18 => {
            let p = rng.point(e.document());
            let q = rng.point(e.document());
            let mode =
                [SelectMode::Replace, SelectMode::Add, SelectMode::Subtract][rng.below(3) as usize];
            if rng.below(2) == 0 {
                e.select(Some(SelRect::from_corners(p, q).into()), mode);
            } else {
                let r = rng.point(e.document());
                e.select_lasso(&[p, q, r], mode);
            }
        }
        19 => {
            let _ = e.move_selection(rng.below(5) as i32 - 2, rng.below(5) as i32 - 2);
        }
        20 => {
            let t = [
                Transform::FlipHorizontal,
                Transform::FlipVertical,
                Transform::Rotate90,
                Transform::Rotate180,
            ][rng.below(4) as usize];
            let _ = e.transform(t);
        }
        21 => {
            let _ = if rng.below(2) == 0 {
                e.delete_selection()
            } else {
                e.duplicate_selection()
            };
        }
        22 => {
            let w = 6 + rng.below(20) as u32;
            let h = 6 + rng.below(20) as u32;
            let anchor = Anchor {
                x: rng.below(3) as u8,
                y: rng.below(3) as u8,
            };
            let _ = e.resize_canvas(w, h, anchor);
        }
        _ => {
            if rng.below(2) == 0 {
                let colors: Vec<Rgba> = (0..6).map(|_| rng.color()).collect();
                let _ = e.convert_to_palette(&colors, PaletteScope::All);
            } else {
                let d = e.document();
                let (w, h) = (d.width(), d.height());
                let _ = e.crop(
                    rng.below(u64::from(w) / 2) as u32,
                    rng.below(u64::from(h) / 2) as u32,
                    w / 2 + 1,
                    h / 2 + 1,
                );
            }
        }
    }
    e.end_stroke();
    name
}

const OPS: [&str; 24] = [
    "stroke",
    "stroke",
    "stroke",
    "stroke",
    "fill",
    "polygon",
    "add layer",
    "duplicate layer",
    "merge down",
    "delete layer",
    "layer up",
    "layer props",
    "select layer",
    "add frame",
    "delete frame",
    "move frame",
    "select frame",
    "add tag",
    "select",
    "move selection",
    "transform",
    "delete/duplicate selection",
    "canvas size",
    "palette/crop",
];

fn session(seed: u64, steps: usize) {
    let doc = Document::new(16, 12, Rgba::TRANSPARENT).expect("doc");
    let mut e = Editor::new(doc, None, "stress".into());
    let mut rng = Rng(seed);
    // States after every recorded step, by history position.
    let mut states = vec![(e.history().state_id(), fingerprint(e.document()), "start")];
    for _ in 0..steps {
        let op = random_step(&mut e, &mut rng);
        let id = e.history().state_id();
        if states.last().map(|s| s.0) != Some(id) {
            states.push((id, fingerprint(e.document()), op));
        } else {
            // Not an undo step (refused, or navigation only): the pixels must not change.
            assert_eq!(
                fingerprint(e.document()),
                states.last().expect("state").1,
                "seed {seed}: a change was made without an undo step"
            );
        }
    }
    let n = states.len() - 1;
    assert!(
        n > steps / 4,
        "seed {seed}: only {n} of {steps} steps changed something"
    );

    // Save and open: identical document.
    let bytes = write_kgr(e.document(), Cursor::new(Vec::new()))
        .expect("save")
        .into_inner();
    let reopened = read_kgr(Cursor::new(bytes)).expect("open");
    assert_eq!(
        fingerprint(&reopened),
        states[n].1,
        "seed {seed}: save/open changed the document"
    );

    // Undo as far as the history reaches (it keeps the last 256 steps), one state at
    // a time, checking each one, then redo all of it.
    let reach = n.min(256);
    for i in (n - reach..n).rev() {
        assert!(
            e.undo().expect("undo").is_some(),
            "seed {seed}: undo {i} refused"
        );
        assert_eq!(
            fingerprint(e.document()),
            states[i].1,
            "seed {seed}: undoing \"{}\" (state {}) did not restore state {i}",
            states[i + 1].2,
            i + 1
        );
    }
    assert!(
        e.undo().expect("undo").is_none() || n < 256,
        "seed {seed}: history longer than its limit"
    );
    for (i, state) in states.iter().enumerate().skip(n - reach + 1) {
        assert!(
            e.redo().expect("redo").is_some(),
            "seed {seed}: redo {i} refused"
        );
        assert_eq!(
            fingerprint(e.document()),
            state.1,
            "seed {seed}: redoing \"{}\" (state {i})",
            state.2
        );
    }
}

#[test]
fn long_mixed_sessions_undo_redo_and_save_exactly() {
    // 40 sessions of 400 operations each (about a second in a debug build).
    for seed in 1..=40u64 {
        session(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1, 400);
    }
}

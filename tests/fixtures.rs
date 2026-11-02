//! Compatibility: projects of every format version (written by hand in
//! `scripts/make-test-fixtures.py`, not by Kagero) open as intended, survive a
//! save/open round trip, and damaged copies are refused without crashing.

use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use kagero::core::{Document, Rgba};
use kagero::file_io::kgr::{read_kgr, write_kgr};

const RED: Rgba = Rgba::opaque(220, 30, 30);
const BLUE: Rgba = Rgba::opaque(30, 60, 220);
const GREEN: Rgba = Rgba::opaque(40, 180, 70);

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn open(name: &str) -> Document {
    read_kgr(Cursor::new(fixture(name))).expect(name)
}

fn px(doc: &Document, frame: usize, layer: usize, x: i32, y: i32) -> Rgba {
    doc.cel(frame, layer)
        .and_then(|c| c.get(x, y))
        .expect("pixel")
}

/// Saving and opening again gives the same document.
fn round_trip(doc: &Document) {
    let bytes = write_kgr(doc, Cursor::new(Vec::new()))
        .expect("save")
        .into_inner();
    let again = read_kgr(Cursor::new(bytes)).expect("open");
    assert_eq!(again.layers(), doc.layers());
    assert_eq!(again.tags(), doc.tags());
    assert_eq!(again.frame_count(), doc.frame_count());
    for f in 0..doc.frame_count() {
        assert_eq!(again.frames()[f].id, doc.frames()[f].id);
        assert_eq!(
            again.frames()[f].duration_ms(),
            doc.frames()[f].duration_ms()
        );
        for l in 0..doc.layer_count() {
            assert_eq!(again.cel(f, l), doc.cel(f, l), "frame {f} layer {l}");
        }
    }
}

#[test]
fn version_1_layers_become_one_frame() {
    let doc = open("v1-layers.kgr");
    assert_eq!((doc.width(), doc.height()), (4, 3));
    assert_eq!(doc.frame_count(), 1);
    let names: Vec<_> = doc
        .layers()
        .iter()
        .map(|l| (l.name.as_str(), l.visible))
        .collect();
    assert_eq!(names, [("Back", true), ("Ink", false)]);
    assert_eq!(doc.active_index(), 1);
    assert_eq!(px(&doc, 0, 0, 3, 2), RED);
    assert_eq!(px(&doc, 0, 1, 2, 1), BLUE);
    assert_eq!(px(&doc, 0, 1, 0, 0), Rgba::TRANSPARENT);
    // The hidden layer isn't composited.
    assert_eq!(doc.composite_frame_pixel(0, 2, 1), Some(RED));
    round_trip(&doc);
}

#[test]
fn version_2_keeps_frames_durations_and_ids() {
    let doc = open("v2-animation.kgr");
    assert_eq!((doc.width(), doc.height()), (5, 4));
    let frames: Vec<_> = doc
        .frames()
        .iter()
        .map(|f| (f.id.0, f.duration_ms()))
        .collect();
    assert_eq!(frames, [(2, 100), (5, 150), (9, 200)]);
    let ids: Vec<_> = doc.layers().iter().map(|l| l.id.0).collect();
    assert_eq!(ids, [3, 7]);
    assert_eq!((doc.active_index(), doc.active_frame_index()), (1, 2));
    for f in 0..3 {
        assert_eq!(px(&doc, f, 0, 4, 3), GREEN, "ground in frame {f}");
        assert_eq!(px(&doc, f, 1, f as i32, 1), RED, "ball in frame {f}");
    }
    // Version 2 knows no opacity, locks or tags: defaults apply.
    assert!(doc
        .layers()
        .iter()
        .all(|l| l.opacity == 255 && !l.locked && !l.alpha_lock));
    assert!(doc.tags().is_empty());
    round_trip(&doc);
}

#[test]
fn version_3_keeps_opacity_locks_and_tags() {
    let doc = open("v3-opacity-tags.kgr");
    let layers: Vec<_> = doc
        .layers()
        .iter()
        .map(|l| (l.name.as_str(), l.opacity, l.locked, l.alpha_lock))
        .collect();
    assert_eq!(
        layers,
        [("Background", 255, true, false), ("Glow", 128, false, true)]
    );
    let tags: Vec<_> = doc
        .tags()
        .iter()
        .map(|t| (t.name.as_str(), t.start, t.end, t.color.to_hex()))
        .collect();
    assert_eq!(tags, [("walk", 0, 1, "#E86A33".to_owned())]);
    assert_eq!(px(&doc, 1, 1, 1, 1), Rgba::new(250, 200, 60, 128));
    // Opacity applies when compositing: half of a half-transparent pixel.
    let c = doc.composite_frame_pixel(0, 1, 1).expect("pixel");
    assert_eq!(c.a(), 64);
    round_trip(&doc);
}

/// Deterministic damage: flipped bytes, cut-off files and inserted garbage.
fn damaged(original: &[u8], n: u64) -> Vec<u8> {
    let mut b = original.to_vec();
    let mut s = n.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    match n % 4 {
        0 => {
            for _ in 0..1 + next() % 8 {
                let i = (next() as usize) % b.len();
                b[i] ^= (next() as u8) | 1;
            }
        }
        1 => b.truncate((next() as usize) % b.len()),
        2 => {
            let i = (next() as usize) % b.len();
            let junk: Vec<u8> = (0..1 + next() % 64).map(|_| next() as u8).collect();
            b.splice(i..i, junk);
        }
        _ => {
            let i = (next() as usize) % b.len();
            let len = (1 + next() % 32) as usize;
            b.drain(i..(i + len).min(b.len()));
        }
    }
    b
}

#[test]
fn damaged_projects_are_refused_without_crashing() {
    for name in ["v1-layers.kgr", "v2-animation.kgr", "v3-opacity-tags.kgr"] {
        let original = fixture(name);
        let mut refused = 0;
        for n in 0..400 {
            let bytes = damaged(&original, n);
            let result = catch_unwind(AssertUnwindSafe(|| read_kgr(Cursor::new(bytes))));
            match result {
                Err(_) => panic!("{name}: damage #{n} crashed the reader"),
                Ok(Err(_)) => refused += 1,
                // Some damage hits bytes nothing reads (e.g. unused ZIP fields); such a
                // file must still be a valid document.
                Ok(Ok(doc)) => assert!(doc.width() > 0 && doc.frame_count() > 0),
            }
        }
        assert!(
            refused > 300,
            "{name}: only {refused} of 400 damaged copies were refused"
        );
    }
}

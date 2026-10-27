//! Timing checks for larger animations. Ignored by default; run with
//! `cargo test --release --test performance -- --ignored --nocapture`.

use std::io::Cursor;
use std::time::Instant;

use kagero::core::{Document, Rgba};
use kagero::editor::{ColorSlot, Editor};
use kagero::file_io::{read_kgr, write_kgr};

fn animation(size: u32, frames: usize, layers: usize) -> Editor {
    let doc = Document::new(size, size, Rgba::TRANSPARENT).expect("valid");
    let mut e = Editor::new(doc, None, "perf".into());
    for _ in 1..layers {
        e.add_layer().expect("layer");
    }
    for f in 0..frames {
        if f > 0 {
            e.duplicate_frame().expect("frame");
        }
        e.primary = Rgba::opaque((f * 7 % 255) as u8, 80, 160);
        let x = (f % size as usize) as i32;
        e.begin_stroke((x, 0), ColorSlot::Primary).expect("draw");
        e.update_stroke((x, size as i32 - 1));
        e.end_stroke();
    }
    e
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "timing benchmark"]
fn hundreds_of_frames() {
    for (size, frames, layers) in [(64, 300, 3), (256, 60, 2), (64, 1000, 1)] {
        let t = Instant::now();
        let editor = animation(size, frames, layers);
        let build = ms(t);
        let doc = editor.document();

        let t = Instant::now();
        let bytes = write_kgr(doc, Cursor::new(Vec::new()))
            .expect("write")
            .into_inner();
        let save = ms(t);

        let t = Instant::now();
        let loaded = read_kgr(Cursor::new(&bytes)).expect("read");
        let load = ms(t);
        assert_eq!(loaded.frame_count(), frames);

        let t = Instant::now();
        for i in 0..doc.frame_count() {
            let _ = kagero::ui::timeline::thumbnail(doc, i);
        }
        let thumbs = ms(t) / frames as f64;

        let t = Instant::now();
        for i in 0..doc.frame_count() {
            let _ = doc.composite_frame(i).expect("composite");
        }
        let composite = ms(t) / frames as f64;

        println!(
            "{size}x{size} x{frames} frames x{layers} layers: build {build:.0} ms, \
             save {save:.0} ms ({} KiB), load {load:.0} ms, thumbnail {thumbs:.2} ms/frame, \
             full composite {composite:.2} ms/frame",
            bytes.len() / 1024
        );
    }
}

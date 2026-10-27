//! Generates the README assets: `docs/images/logo.png` and a demo animation project.
//!
//! `cargo run --example readme_assets -- <demo-output.kgr>`

use std::path::PathBuf;

use kagero::core::{Document, PixelBuffer, Rgba};
use kagero::file_io::{png::encode_png, save_kgr, write_atomic};
use kagero::ui::icons::logo_rgba;

const SIZE: u32 = 32;
const FRAMES: usize = 8;

fn hex(v: u32) -> Rgba {
    Rgba::opaque((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Ground strip with a soft shadow that shrinks while the spirit floats up.
fn ground(lift: i32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(SIZE, SIZE).unwrap_or_else(|e| panic!("{e}"));
    for y in 27..32 {
        for x in 0..SIZE as i32 {
            let c = if y == 27 {
                hex(0x4F8A5B)
            } else if (x + y) % 5 == 0 {
                hex(0x2F5D45)
            } else {
                hex(0x1E3A2F)
            };
            buf.set(x, y, c);
        }
    }
    let rx = 7 - lift;
    for x in (16 - rx)..=(16 + rx) {
        buf.set(x, 27, hex(0x2F5D45));
    }
    buf
}

/// The ember spirit: a flame body with a swaying tip, glowing core and eyes.
fn spirit(frame: usize) -> PixelBuffer {
    const BOB: [i32; FRAMES] = [0, -1, -2, -3, -3, -2, -1, 0];
    const SWAY: [f32; FRAMES] = [0.0, 1.0, 1.5, 1.0, 0.0, -1.0, -1.5, -1.0];
    let mut buf = PixelBuffer::new(SIZE, SIZE).unwrap_or_else(|e| panic!("{e}"));
    let base = 25 + BOB[frame];
    let (cx, cy) = (16.0_f32, base as f32 - 5.0);
    let inside = |x: i32, y: i32| -> bool {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let body = ((fx - cx) / 6.2).powi(2) + ((fy - cy) / 6.0).powi(2) <= 1.0;
        let top = cy - 11.0;
        let tip = if fy >= top && fy <= cy {
            let t = (fy - top) / (cy - top);
            let shift = SWAY[frame] * (1.0 - t) * 2.0;
            (fx - (cx + shift)).abs() <= 6.0 * t.powf(0.8)
        } else {
            false
        };
        body || tip
    };
    let core = |x: i32, y: i32| -> bool {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        ((fx - cx) / 3.4).powi(2) + ((fy - cy - 1.0) / 3.6).powi(2) <= 1.0
    };
    for y in 0..SIZE as i32 {
        for x in 0..SIZE as i32 {
            if !inside(x, y) {
                continue;
            }
            let edge =
                !inside(x - 1, y) || !inside(x + 1, y) || !inside(x, y - 1) || !inside(x, y + 1);
            let c = if edge {
                hex(0x6E2C1F)
            } else if core(x, y) {
                hex(0xFFE7A8)
            } else if y < cy as i32 - 3 {
                hex(0xF06B35)
            } else {
                hex(0xFF9D42)
            };
            buf.set(x, y, c);
        }
    }
    let eye_y = cy as i32;
    let blink = frame == 5;
    for ex in [13, 18] {
        if blink {
            buf.set(ex, eye_y, hex(0x3B1F1A));
            buf.set(ex + 1, eye_y, hex(0x3B1F1A));
        } else {
            for (dx, dy) in [(0, -1), (1, -1), (0, 0), (1, 0)] {
                buf.set(ex + dx, eye_y + dy, hex(0x17191F));
            }
        }
    }
    // A spark that drifts up with the flame.
    let spark_y = base - 18 - (frame as i32 % 4);
    buf.set(20 + (frame as i32 % 3) - 1, spark_y, hex(0xFFC56E));
    buf
}

fn main() {
    let demo = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ember-spirit.kgr"));

    let (rgba, w, h) = logo_rgba(16);
    let logo = PixelBuffer::from_raw(w, h, rgba).unwrap_or_else(|e| panic!("{e}"));
    std::fs::create_dir_all("docs/images").unwrap_or_else(|e| panic!("{e}"));
    write_atomic(std::path::Path::new("docs/images/logo.png"), |out| {
        encode_png(&logo, out)
    })
    .unwrap_or_else(|e| panic!("{e}"));

    const LIFT: [i32; FRAMES] = [0, 1, 2, 3, 3, 2, 1, 0];
    let frames = (0..FRAMES)
        .map(|f| (90, vec![ground(LIFT[f]), spirit(f)]))
        .collect();
    let doc = Document::from_parts(
        SIZE,
        SIZE,
        vec![("Ground".into(), true), ("Spirit".into(), true)],
        frames,
        1,
        2,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    save_kgr(&doc, &demo).unwrap_or_else(|e| panic!("{e}"));
    println!("wrote docs/images/logo.png and {}", demo.display());
}

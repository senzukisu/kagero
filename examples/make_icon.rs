//! Writes `assets/kagero.ico`, the program icon of the Windows executables, from the
//! pixel logo in `src/ui/icons.rs`: the 16×16 logo enlarged by whole pixels (sharp)
//! to 16, 32, 48, 64, 128 and 256 px, stored as PNG images inside the icon.
//!
//! Run after changing the logo: `cargo run --example make_icon`

use image::ImageEncoder;

fn png(size_scale: u32) -> Vec<u8> {
    let (rgba, w, h) = kagero::ui::icons::logo_rgba(size_scale);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .expect("png");
    out
}

fn main() {
    let scales = [1u32, 2, 3, 4, 8, 16];
    let images: Vec<(u32, Vec<u8>)> = scales.iter().map(|&s| (16 * s, png(s))).collect();
    // ICONDIR, then one ICONDIRENTRY per image, then the PNG data.
    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, data) in &images {
        let side = if *size >= 256 { 0 } else { *size as u8 };
        ico.extend_from_slice(&[side, side, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(data.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in &images {
        ico.extend_from_slice(data);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/kagero.ico");
    std::fs::write(&path, &ico).expect("write icon");
    println!("wrote {} ({} bytes)", path.display(), ico.len());
}

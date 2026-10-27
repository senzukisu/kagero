//! Document creation, bounds, size limits and compositing.

use kagero::core::{
    layers_bytes, validate_size, CoreError, Document, PixelBuffer, Rgba, MAX_CANVAS_SIZE,
    MAX_DOCUMENT_BYTES, MAX_LAYERS,
};

#[test]
fn creates_documents_of_allowed_sizes() {
    for (w, h) in [(1, 1), (32, 32), (320, 180), (MAX_CANVAS_SIZE, 1)] {
        let doc = Document::new(w, h, Rgba::TRANSPARENT).expect("valid size");
        assert_eq!((doc.width(), doc.height()), (w, h));
        assert_eq!(doc.layer_count(), 1);
        assert_eq!(doc.active_index(), 0);
        let layer = doc.active_layer();
        assert_eq!(layer.pixels.as_bytes().len(), (w * h * 4) as usize);
    }
}

#[test]
fn rejects_invalid_sizes_before_allocating() {
    for (w, h) in [(0, 1), (1, 0), (MAX_CANVAS_SIZE + 1, 1), (1, u32::MAX)] {
        assert_eq!(
            Document::new(w, h, Rgba::WHITE).err(),
            Some(CoreError::InvalidSize {
                width: w,
                height: h
            })
        );
        assert!(validate_size(w, h).is_err());
        assert!(PixelBuffer::new(w, h).is_err());
    }
}

#[test]
fn background_color_fills_layer() {
    let doc = Document::new(3, 2, Rgba::WHITE).expect("valid");
    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(doc.active_layer().pixels.get(x, y), Some(Rgba::WHITE));
        }
    }
}

#[test]
fn pixel_access_is_bounds_checked() {
    let doc = Document::new(4, 4, Rgba::TRANSPARENT).expect("valid");
    assert_eq!(doc.composite_pixel(-1, 0), None);
    assert_eq!(doc.composite_pixel(0, 4), None);
    assert_eq!(doc.composite_pixel(4, 0), None);
    assert_eq!(doc.composite_pixel(3, 3), Some(Rgba::TRANSPARENT));
    assert!(doc.composite_rect(4, 0, 1, 1).is_none());
    let (rect, bytes) = doc.composite_rect(2, 2, 100, 100).expect("clipped");
    assert_eq!(rect, (2, 2, 2, 2));
    assert_eq!(bytes.len(), 2 * 2 * 4);
}

#[test]
fn composites_bottom_to_top_and_skips_hidden() {
    let red = Rgba::opaque(255, 0, 0);
    let half_blue = Rgba::new(0, 0, 255, 128);
    let mut bottom = PixelBuffer::new(2, 1).expect("valid");
    bottom.set(0, 0, red);
    bottom.set(1, 0, red);
    let mut top = PixelBuffer::new(2, 1).expect("valid");
    top.set(0, 0, half_blue);
    let doc = Document::from_layers(
        2,
        1,
        vec![
            ("bottom".into(), true, bottom.clone()),
            ("top".into(), true, top.clone()),
        ],
        1,
    )
    .expect("valid");
    let mixed = doc.composite_pixel(0, 0).expect("in bounds");
    assert_eq!(mixed.a(), 255);
    assert!(mixed.r() > 100 && mixed.b() > 100, "{mixed:?}");
    assert_eq!(doc.composite_pixel(1, 0), Some(red));

    let doc = Document::from_layers(
        2,
        1,
        vec![("bottom".into(), false, bottom), ("top".into(), true, top)],
        0,
    )
    .expect("valid");
    assert_eq!(doc.composite_pixel(0, 0), Some(half_blue));
    assert_eq!(doc.composite_pixel(1, 0), Some(Rgba::TRANSPARENT));
    let merged = doc.composite().expect("valid");
    assert_eq!(merged.get(1, 0), Some(Rgba::TRANSPARENT));
}

#[test]
fn transparent_pixels_stay_transparent_in_composite() {
    let doc = Document::new(8, 8, Rgba::TRANSPARENT).expect("valid");
    let merged = doc.composite().expect("valid");
    assert!(merged.as_bytes().iter().all(|&b| b == 0));
}

#[test]
fn layer_limits_are_enforced() {
    let mut doc = Document::new(4, 4, Rgba::TRANSPARENT).expect("valid");
    for i in 1..MAX_LAYERS {
        let layer = doc.make_empty_layer(format!("L{i}")).expect("below limit");
        let at = doc.layer_count();
        doc.insert_layer(at, layer).expect("insert");
    }
    assert_eq!(doc.layer_count(), MAX_LAYERS);
    assert_eq!(
        doc.make_empty_layer("too many".into()).err(),
        Some(CoreError::TooManyLayers)
    );
}

#[test]
fn memory_budget_limits_large_documents() {
    // 2048×2048 RGBA is 16 MiB per layer; the 512 MiB budget allows exactly 32 of them.
    assert_eq!(
        layers_bytes(MAX_CANVAS_SIZE, MAX_CANVAS_SIZE, 32),
        MAX_DOCUMENT_BYTES
    );
    assert!(layers_bytes(MAX_CANVAS_SIZE, MAX_CANVAS_SIZE, 33) > MAX_DOCUMENT_BYTES);
    // Small images are limited by the layer count instead.
    assert!(layers_bytes(64, 64, MAX_LAYERS) < MAX_DOCUMENT_BYTES);
}

#[test]
fn the_last_layer_cannot_be_removed() {
    let mut doc = Document::new(2, 2, Rgba::TRANSPARENT).expect("valid");
    assert_eq!(doc.remove_layer(0).err(), Some(CoreError::LastLayer));
}

//! History limits and stack behavior.

use kagero::core::{Document, Rgba};
use kagero::editor::{Change, History, PixelChange};

fn pixel_change(doc: &Document, index: u32) -> Change {
    Change::Pixels {
        frame: doc.active_frame().id,
        layer: doc.active_layer().id,
        changes: vec![PixelChange {
            index,
            before: Rgba::TRANSPARENT,
            after: Rgba::WHITE,
        }],
    }
}

#[test]
fn entry_limit_drops_oldest() {
    let mut doc = Document::new(8, 8, Rgba::TRANSPARENT).expect("valid");
    let mut history = History::new(5, usize::MAX);
    for i in 0..10 {
        let mut c = pixel_change(&doc, i);
        c.apply(&mut doc).expect("apply");
        history.push(c);
    }
    assert_eq!(history.undo_len(), 5);
    for _ in 0..5 {
        assert!(history.undo(&mut doc).expect("undo").is_some());
    }
    assert!(history.undo(&mut doc).expect("empty").is_none());
    // The five oldest changes can no longer be undone.
    let layer = doc.active_cel();
    assert_eq!(layer.get_index(0), Some(Rgba::WHITE));
    assert_eq!(layer.get_index(4), Some(Rgba::WHITE));
    assert_eq!(layer.get_index(5), Some(Rgba::TRANSPARENT));
}

#[test]
fn byte_limit_is_respected() {
    let mut doc = Document::new(8, 8, Rgba::TRANSPARENT).expect("valid");
    let one = pixel_change(&doc, 0).byte_size();
    let mut history = History::new(1000, one * 3);
    for i in 0..10 {
        let mut c = pixel_change(&doc, i);
        c.apply(&mut doc).expect("apply");
        history.push(c);
    }
    assert_eq!(history.undo_len(), 3);
    assert!(history.bytes() <= one * 3);
}

#[test]
fn push_clears_redo_and_frees_its_memory() {
    let mut doc = Document::new(8, 8, Rgba::TRANSPARENT).expect("valid");
    let mut history = History::default();
    for i in 0..3 {
        let mut c = pixel_change(&doc, i);
        c.apply(&mut doc).expect("apply");
        history.push(c);
    }
    let full = history.bytes();
    history.undo(&mut doc).expect("undo");
    history.undo(&mut doc).expect("undo");
    assert_eq!(history.redo_len(), 2);
    assert_eq!(history.bytes(), full);
    let mut c = pixel_change(&doc, 7);
    c.apply(&mut doc).expect("apply");
    history.push(c);
    assert_eq!(history.redo_len(), 0);
    assert_eq!(history.undo_len(), 2);
    assert_eq!(history.bytes(), full / 3 * 2);
}

#[test]
fn big_edits_are_stored_compactly_and_undo_exactly() {
    use kagero::editor::{ColorSlot, Editor, ToolKind};
    let doc = Document::new(512, 512, Rgba::TRANSPARENT).expect("valid");
    let mut e = Editor::new(doc, None, "runs".into());
    // A fill of the whole layer: 262 144 pixels, one color change.
    e.fill((3, 3), Rgba::opaque(10, 200, 30)).expect("fill");
    assert!(
        e.history().bytes() < 4096,
        "a uniform fill is stored as runs, not per pixel: {} bytes",
        e.history().bytes()
    );
    // A stroke over it, then a pixel-varied change (each pixel a different color).
    e.set_tool(ToolKind::Pencil);
    e.settings.brush_size = 6;
    e.primary = Rgba::new(250, 20, 20, 200);
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("stroke");
    for i in 0..200 {
        e.update_stroke(((i * 5) % 512, (i * 3) % 512));
    }
    e.end_stroke();
    let after_stroke = e.document().active_cel().clone();
    let colors: Vec<Rgba> = (0..64)
        .map(|i| Rgba::opaque(i * 4, 255 - i * 4, i))
        .collect();
    e.convert_to_palette(&colors, kagero::editor::PaletteScope::LayerFrame)
        .expect("convert");
    let converted = e.document().active_cel().clone();
    // Undo all three, then redo: every pixel must match each state exactly.
    e.undo().expect("undo convert");
    assert_eq!(e.document().active_cel(), &after_stroke);
    e.undo().expect("undo stroke");
    e.undo().expect("undo fill");
    assert!(e.document().active_cel().as_bytes().iter().all(|b| *b == 0));
    e.redo().expect("redo");
    e.redo().expect("redo");
    assert_eq!(e.document().active_cel(), &after_stroke);
    e.redo().expect("redo");
    assert_eq!(e.document().active_cel(), &converted);
}

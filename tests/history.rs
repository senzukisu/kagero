//! History limits and stack behavior.

use kagero::core::{Document, Rgba};
use kagero::editor::{Change, History, PixelChange};

fn pixel_change(doc: &Document, index: u32) -> Change {
    Change::Pixels {
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
    let layer = &doc.active_layer().pixels;
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

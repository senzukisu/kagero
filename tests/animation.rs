//! Frames: create, duplicate, delete, reorder, durations, undo/redo, onion skin.

use kagero::core::animation::{frame_at_time, MAX_FRAME_DURATION_MS, MIN_FRAME_DURATION_MS};
use kagero::core::onion_skin::OnionSkin;
use kagero::core::{CoreError, Document, Rgba, MAX_FRAMES};
use kagero::editor::{ColorSlot, EditError, Editor, ToolKind};

const RED: Rgba = Rgba::opaque(255, 0, 0);
const BLUE: Rgba = Rgba::opaque(0, 0, 255);

fn editor(w: u32, h: u32) -> Editor {
    let doc = Document::new(w, h, Rgba::TRANSPARENT).expect("valid size");
    let mut e = Editor::new(doc, None, "Anim".into());
    e.primary = RED;
    e
}

fn dot(e: &mut Editor, x: i32, y: i32, color: Rgba) {
    e.primary = color;
    e.begin_stroke((x, y), ColorSlot::Primary).expect("draw");
    e.end_stroke();
}

fn px(e: &Editor, frame: usize, x: i32, y: i32) -> Rgba {
    e.document()
        .composite_frame_pixel(frame, x, y)
        .expect("in bounds")
}

fn frame_ids(e: &Editor) -> Vec<u64> {
    e.document().frames().iter().map(|f| f.id.0).collect()
}

#[test]
fn new_documents_have_one_frame() {
    let e = editor(4, 4);
    assert_eq!(e.document().frame_count(), 1);
    assert_eq!(e.document().active_frame_index(), 0);
    assert_eq!(e.document().active_frame().cels().len(), 1);
}

#[test]
fn add_frame_creates_empty_frame_with_all_layers() {
    let mut e = editor(4, 4);
    e.add_layer().expect("layer");
    dot(&mut e, 1, 1, RED);
    e.add_frame().expect("frame");
    let doc = e.document();
    assert_eq!(doc.frame_count(), 2);
    assert_eq!(doc.active_frame_index(), 1);
    assert_eq!(doc.active_frame().cels().len(), 2, "one cel per layer");
    assert_eq!(px(&e, 1, 1, 1), Rgba::TRANSPARENT, "new frame is empty");
    assert_eq!(px(&e, 0, 1, 1), RED);
    assert_ne!(doc.frames()[0].id, doc.frames()[1].id);
}

#[test]
fn duplicated_frames_are_independent() {
    let mut e = editor(4, 4);
    dot(&mut e, 0, 0, RED);
    e.set_frame_duration(0, 250).expect("duration");
    e.duplicate_frame().expect("duplicate");
    assert_eq!(e.document().active_frame_index(), 1);
    assert_eq!(px(&e, 1, 0, 0), RED, "copy has the content");
    assert_eq!(e.document().frames()[1].duration_ms(), 250);

    // Editing the copy leaves the original untouched and vice versa.
    dot(&mut e, 0, 0, BLUE);
    assert_eq!(px(&e, 1, 0, 0), BLUE);
    assert_eq!(px(&e, 0, 0, 0), RED);
    e.select_frame(0);
    dot(&mut e, 3, 3, BLUE);
    assert_eq!(px(&e, 1, 3, 3), Rgba::TRANSPARENT);
}

#[test]
fn layers_are_shared_but_pixels_are_per_frame() {
    let mut e = editor(4, 4);
    e.add_frame().expect("frame");
    e.add_layer().expect("layer");
    assert_eq!(e.document().layer_count(), 2);
    for f in e.document().frames() {
        assert_eq!(f.cels().len(), 2, "every frame got the new layer");
    }
    e.toggle_layer_visibility(0).expect("hide");
    e.select_frame(0);
    assert!(
        !e.document().layers()[0].visible,
        "hidden stays hidden after switching frames"
    );
    assert_eq!(e.document().active_index(), 1, "layer selection is kept");
}

#[test]
fn delete_frame_and_undo_restores_content_and_id() {
    let mut e = editor(4, 4);
    e.add_frame().expect("frame");
    dot(&mut e, 2, 2, RED);
    let ids = frame_ids(&e);
    e.delete_frame(1).expect("delete");
    assert_eq!(e.document().frame_count(), 1);
    assert_eq!(e.document().active_frame_index(), 0);
    e.undo().expect("undo");
    assert_eq!(frame_ids(&e), ids);
    assert_eq!(px(&e, 1, 2, 2), RED);
    e.redo().expect("redo");
    assert_eq!(e.document().frame_count(), 1);
}

#[test]
fn last_frame_cannot_be_deleted() {
    let mut e = editor(4, 4);
    assert_eq!(
        e.delete_frame(0),
        Err(EditError::Core(CoreError::LastFrame))
    );
    assert_eq!(
        e.delete_frame(5),
        Err(EditError::Core(CoreError::LastFrame)),
        "still a single frame"
    );
}

#[test]
fn moving_frames_keeps_content_and_selection() {
    let mut e = editor(4, 4);
    dot(&mut e, 0, 0, RED);
    e.add_frame().expect("frame");
    dot(&mut e, 1, 1, BLUE);
    e.add_frame().expect("frame");
    let ids = frame_ids(&e);
    e.select_frame(0);
    e.move_frame_right(0).expect("move");
    assert_eq!(frame_ids(&e), vec![ids[1], ids[0], ids[2]]);
    assert_eq!(
        e.document().active_frame_index(),
        1,
        "selection follows the frame"
    );
    assert_eq!(px(&e, 1, 0, 0), RED);
    assert_eq!(px(&e, 0, 1, 1), BLUE);
    e.move_frame_left(0).expect("no-op at the start");
    e.move_frame_right(2).expect("no-op at the end");
    e.undo().expect("undo move");
    assert_eq!(frame_ids(&e), ids);
}

#[test]
fn durations_are_validated_and_undoable() {
    let mut e = editor(4, 4);
    let start = e.document().frames()[0].duration_ms();
    e.set_frame_duration(0, 0).expect("clamped");
    assert_eq!(
        e.document().frames()[0].duration_ms(),
        MIN_FRAME_DURATION_MS
    );
    e.set_frame_duration(0, u32::MAX).expect("clamped");
    assert_eq!(
        e.document().frames()[0].duration_ms(),
        MAX_FRAME_DURATION_MS
    );
    e.undo().expect("undo");
    e.undo().expect("undo");
    assert_eq!(e.document().frames()[0].duration_ms(), start);
    assert!(e.set_frame_duration(3, 100).is_err());
}

#[test]
fn undoing_a_stroke_on_another_frame_switches_to_it() {
    let mut e = editor(4, 4);
    dot(&mut e, 0, 0, RED);
    e.add_frame().expect("frame");
    assert_eq!(e.document().active_frame_index(), 1);
    e.undo().expect("undo add frame");
    e.undo().expect("undo stroke");
    assert_eq!(e.document().active_frame_index(), 0);
    assert_eq!(px(&e, 0, 0, 0), Rgba::TRANSPARENT);
    e.redo().expect("redo stroke");
    assert_eq!(px(&e, 0, 0, 0), RED);
}

#[test]
fn frame_navigation_wraps() {
    let mut e = editor(2, 2);
    e.add_frame().expect("frame");
    e.add_frame().expect("frame");
    e.select_frame(2);
    e.next_frame();
    assert_eq!(e.document().active_frame_index(), 0);
    e.previous_frame();
    assert_eq!(e.document().active_frame_index(), 2);
    e.select_frame(99);
    assert_eq!(
        e.document().active_frame_index(),
        2,
        "out of range is ignored"
    );
}

#[test]
fn read_only_blocks_edits() {
    let mut e = editor(4, 4);
    e.set_read_only(true);
    assert_eq!(
        e.begin_stroke((0, 0), ColorSlot::Primary),
        Err(EditError::ReadOnly)
    );
    e.set_tool(ToolKind::Fill);
    assert_eq!(
        e.begin_stroke((0, 0), ColorSlot::Primary),
        Err(EditError::ReadOnly)
    );
    assert_eq!(px(&e, 0, 0, 0), Rgba::TRANSPARENT);
    e.set_read_only(false);
    e.begin_stroke((0, 0), ColorSlot::Primary)
        .expect("editable again");
}

#[test]
fn frame_has_content_detects_pixels() {
    let mut e = editor(4, 4);
    assert!(!e.frame_has_content(0));
    dot(&mut e, 1, 1, RED);
    assert!(e.frame_has_content(0));
    assert!(!e.frame_has_content(9));
}

#[test]
fn frame_limit_is_enforced() {
    let mut doc = Document::new(1, 1, Rgba::TRANSPARENT).expect("valid");
    for _ in 1..MAX_FRAMES {
        let f = doc.make_empty_frame(100).expect("below limit");
        let at = doc.frame_count();
        doc.insert_frame(at, f).expect("insert");
    }
    assert_eq!(doc.frame_count(), MAX_FRAMES);
    assert_eq!(
        doc.make_empty_frame(100).err(),
        Some(CoreError::TooManyFrames)
    );
}

#[test]
fn playback_timing_respects_each_duration_and_loops() {
    let durations = [100, 300, 50];
    assert_eq!(frame_at_time(&durations, 0).map(|f| f.0), Some(0));
    assert_eq!(frame_at_time(&durations, 120).map(|f| f.0), Some(1));
    assert_eq!(frame_at_time(&durations, 399).map(|f| f.0), Some(1));
    assert_eq!(frame_at_time(&durations, 400).map(|f| f.0), Some(2));
    // Crossing the loop boundary many times never drifts.
    assert_eq!(
        frame_at_time(&durations, 450 * 10_000).map(|f| f.0),
        Some(0)
    );
    assert_eq!(
        frame_at_time(&durations, 450 * 10_000 + 449).map(|f| f.0),
        Some(2)
    );
}

#[test]
fn onion_skin_never_changes_document_or_export() {
    let mut e = editor(3, 3);
    dot(&mut e, 0, 0, RED);
    e.add_frame().expect("frame");
    dot(&mut e, 2, 2, BLUE);
    let before = e.document().composite().expect("composite");
    let onion = OnionSkin {
        enabled: true,
        ..OnionSkin::default()
    };
    let overlays = onion.overlays(
        e.document().frame_count(),
        e.document().active_frame_index(),
    );
    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].frame, 0);
    // Onion skin is a view setting: the composite (what PNG export writes) is unchanged.
    assert_eq!(e.document().composite().expect("composite"), before);
    assert_eq!(px(&e, 1, 0, 0), Rgba::TRANSPARENT);
}

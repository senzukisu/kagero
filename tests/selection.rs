//! Rectangular selection, clipboard and transforms, including undo/redo and edges.

use kagero::core::selection::SelRect;
use kagero::core::{Document, Rgba};
use kagero::editor::{ColorSlot, EditError, Editor, Transform};

const R: Rgba = Rgba::opaque(255, 0, 0);
const G: Rgba = Rgba::opaque(0, 255, 0);
const B: Rgba = Rgba::opaque(0, 0, 255);
const T: Rgba = Rgba::TRANSPARENT;

fn editor(w: u32, h: u32) -> Editor {
    Editor::new(Document::new(w, h, T).expect("valid"), None, "Sel".into())
}

fn dot(e: &mut Editor, x: i32, y: i32, c: Rgba) {
    e.primary = c;
    e.begin_stroke((x, y), ColorSlot::Primary).expect("draw");
    e.end_stroke();
}

fn px(e: &Editor, x: i32, y: i32) -> Rgba {
    e.document().active_cel().get(x, y).expect("in bounds")
}

fn rect(x: i32, y: i32, w: u32, h: u32) -> SelRect {
    SelRect { x, y, w, h }
}

/// 6×4 with an L shape:  R G . . . .
///                       B . . . . .
fn sample() -> Editor {
    let mut e = editor(6, 4);
    dot(&mut e, 0, 0, R);
    dot(&mut e, 1, 0, G);
    dot(&mut e, 0, 1, B);
    e
}

#[test]
fn selections_are_clipped_and_can_be_cleared() {
    let mut e = sample();
    e.set_selection(Some(rect(-2, -2, 4, 4)));
    assert_eq!(
        e.selection(),
        Some(rect(0, 0, 2, 2)),
        "clipped to the image"
    );
    e.set_selection(Some(rect(10, 10, 2, 2)));
    assert_eq!(e.selection(), None, "fully outside = no selection");
    e.select_all();
    assert_eq!(e.selection(), Some(rect(0, 0, 6, 4)));
    e.deselect();
    assert_eq!(e.selection(), None);
    assert_eq!(e.history().undo_len(), 3, "selecting alone is not an edit");
}

#[test]
fn move_selection_moves_pixels_and_outline_with_undo() {
    let mut e = sample();
    e.set_selection(Some(rect(0, 0, 2, 2)));
    assert!(e.move_selection(3, 1).expect("move"));
    assert_eq!(e.selection(), Some(rect(3, 1, 2, 2)));
    assert_eq!((px(&e, 3, 1), px(&e, 4, 1), px(&e, 3, 2)), (R, G, B));
    assert_eq!(
        (px(&e, 0, 0), px(&e, 1, 0), px(&e, 0, 1)),
        (T, T, T),
        "source cleared"
    );
    assert_eq!(e.history().undo_label(), Some("Move Selection"));
    e.undo().expect("undo");
    assert_eq!(e.selection(), Some(rect(0, 0, 2, 2)), "outline restored");
    assert_eq!((px(&e, 0, 0), px(&e, 3, 1)), (R, T));
    e.redo().expect("redo");
    assert_eq!(e.selection(), Some(rect(3, 1, 2, 2)));
    assert_eq!(px(&e, 3, 1), R);
}

#[test]
fn moving_past_the_edge_cuts_off_and_clips() {
    let mut e = sample();
    e.set_selection(Some(rect(0, 0, 2, 2)));
    e.move_selection(5, 0).expect("move");
    assert_eq!(e.selection(), Some(rect(5, 0, 1, 2)));
    assert_eq!((px(&e, 5, 0), px(&e, 5, 1)), (R, B));
    // G went past the right edge and is gone.
    assert!((0..6).all(|x| (0..4).all(|y| px(&e, x, y) != G)));
}

#[test]
fn transparent_pixels_do_not_erase_when_moving() {
    let mut e = sample();
    dot(&mut e, 4, 0, G);
    e.set_selection(Some(rect(0, 0, 3, 1))); // R G and a transparent pixel
    e.move_selection(2, 0).expect("move");
    assert_eq!((px(&e, 2, 0), px(&e, 3, 0)), (R, G));
    assert_eq!(
        px(&e, 4, 0),
        G,
        "the transparent pixel of the selection did not erase"
    );
}

#[test]
fn copy_cut_paste_delete() {
    let mut e = sample();
    assert!(!e.copy_selection(), "nothing to copy without a selection");
    e.set_selection(Some(rect(0, 0, 2, 1)));
    assert!(e.copy_selection());
    assert_eq!(e.history().undo_len(), 3, "copy is not an edit");
    assert!(e.cut_selection().expect("cut"));
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0)), (T, T));
    e.set_selection(None);
    assert!(e.paste().expect("paste"));
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0)), (R, G));
    assert_eq!(
        e.selection(),
        Some(rect(0, 0, 2, 1)),
        "pasted area is selected"
    );
    e.set_selection(Some(rect(0, 1, 1, 1)));
    assert!(e.delete_selection().expect("delete"));
    assert_eq!(px(&e, 0, 1), T);
    // Undo everything: delete, paste, cut.
    e.undo().expect("undo delete");
    assert_eq!(px(&e, 0, 1), B);
    e.undo().expect("undo paste");
    assert_eq!(px(&e, 0, 0), T);
    e.undo().expect("undo cut");
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0)), (R, G));
}

#[test]
fn paste_into_a_smaller_image_stays_inside() {
    let mut big = sample();
    big.set_selection(Some(rect(0, 0, 6, 4)));
    big.copy_selection();
    let mut small = editor(3, 2);
    small.clipboard = big.clipboard.clone();
    small.paste().expect("paste");
    assert_eq!(small.selection(), Some(rect(0, 0, 3, 2)));
    assert_eq!(px(&small, 0, 0), R);
}

#[test]
fn duplicate_places_a_copy_next_to_the_selection() {
    let mut e = sample();
    e.set_selection(Some(rect(0, 0, 2, 2)));
    e.duplicate_selection().expect("dup");
    assert_eq!(e.selection(), Some(rect(2, 0, 2, 2)));
    assert_eq!((px(&e, 2, 0), px(&e, 3, 0), px(&e, 2, 1)), (R, G, B));
    assert_eq!(px(&e, 0, 0), R, "original stays");
}

#[test]
fn flips_and_rotations_are_pixel_exact_and_undoable() {
    let mut e = sample();
    e.set_selection(Some(rect(0, 0, 2, 2)));
    e.transform(Transform::FlipHorizontal).expect("flip");
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0), px(&e, 1, 1)), (G, R, B));
    e.undo().expect("undo");
    e.transform(Transform::FlipVertical).expect("flip");
    assert_eq!((px(&e, 0, 1), px(&e, 1, 1), px(&e, 0, 0)), (R, G, B));
    e.undo().expect("undo");
    e.transform(Transform::Rotate90).expect("rotate");
    //  R G      B R
    //  B .  →   . G
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0), px(&e, 1, 1)), (B, R, G));
    e.transform(Transform::Rotate270).expect("rotate back");
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0), px(&e, 0, 1)), (R, G, B));
    e.transform(Transform::Rotate180).expect("rotate");
    assert_eq!((px(&e, 1, 1), px(&e, 0, 1), px(&e, 1, 0)), (R, G, B));
    for _ in 0..3 {
        e.undo().expect("undo");
    }
    assert_eq!((px(&e, 0, 0), px(&e, 1, 0), px(&e, 0, 1)), (R, G, B));
}

#[test]
fn rotating_a_non_square_selection_keeps_its_center_and_clips() {
    let mut e = editor(4, 4);
    dot(&mut e, 0, 1, R);
    dot(&mut e, 3, 1, G);
    e.set_selection(Some(rect(0, 1, 4, 1))); // 4×1 bar: R . . G
    e.transform(Transform::Rotate90).expect("rotate");
    assert_eq!(
        e.selection(),
        Some(rect(1, 0, 1, 4)),
        "1×4 bar around the same center"
    );
    assert_eq!((px(&e, 1, 0), px(&e, 1, 3)), (R, G));
    assert_eq!(px(&e, 0, 1), T);
}

#[test]
fn without_selection_transforms_apply_to_the_whole_layer() {
    let mut e = sample();
    e.transform(Transform::FlipHorizontal).expect("flip");
    assert_eq!((px(&e, 5, 0), px(&e, 4, 0), px(&e, 5, 1)), (R, G, B));
    assert_eq!(e.selection(), None);
}

#[test]
fn edge_cases_single_pixel_corner_empty_and_transparent() {
    let mut e = sample();
    // Single pixel in the corner.
    e.set_selection(Some(rect(5, 3, 1, 1)));
    assert!(
        !e.transform(Transform::Rotate90).expect("noop"),
        "transparent pixel: nothing changes"
    );
    dot(&mut e, 5, 3, R);
    e.set_selection(Some(rect(5, 3, 1, 1)));
    e.transform(Transform::Rotate90).expect("rotate 1×1");
    assert_eq!(px(&e, 5, 3), R);
    // Fully transparent area: delete changes nothing and records nothing.
    let before = e.history().undo_len();
    e.set_selection(Some(rect(2, 2, 2, 2)));
    assert!(!e.delete_selection().expect("empty delete"));
    assert_eq!(e.history().undo_len(), before);
    // No selection.
    e.deselect();
    assert!(!e.move_selection(1, 0).expect("nothing"));
    assert_eq!(e.require_selection(), Err(EditError::NoSelection));
}

#[test]
fn other_layers_are_never_touched() {
    let mut e = sample();
    e.add_layer().expect("layer");
    dot(&mut e, 0, 0, G);
    e.select_layer(0);
    e.set_selection(Some(rect(0, 0, 2, 2)));
    e.move_selection(2, 2).expect("move");
    let top = e.document().cel(0, 1).expect("top");
    assert_eq!(top.get(0, 0), Some(G), "upper layer unchanged");
    assert_eq!(top.get(2, 2), Some(T));
}

#[test]
fn hidden_layers_and_preview_block_edits() {
    let mut e = sample();
    e.set_selection(Some(rect(0, 0, 2, 2)));
    e.toggle_layer_visibility(0).expect("hide");
    assert_eq!(e.move_selection(1, 0), Err(EditError::HiddenLayer));
    e.toggle_layer_visibility(0).expect("show");
    e.set_read_only(true);
    assert_eq!(e.delete_selection(), Err(EditError::ReadOnly));
}

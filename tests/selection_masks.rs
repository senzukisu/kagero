//! Shaped selections: masks, lasso, magic wand, add/subtract/invert, and every
//! selection edit respecting the shape (Kagero 0.4).

use kagero::core::selection::{SelRect, SelectMode, Selection};
use kagero::core::{Document, PixelBuffer, Rgba};
use kagero::editor::{Editor, SampleSource, Transform};

const RED: Rgba = Rgba::opaque(220, 30, 30);
const NEAR_RED: Rgba = Rgba::opaque(228, 36, 30);
const BLUE: Rgba = Rgba::opaque(30, 60, 220);

fn rect(x: i32, y: i32, w: u32, h: u32) -> SelRect {
    SelRect { x, y, w, h }
}

/// 6×4 image:   R R B R R R
///              R R B R n R      (n = NEAR_RED)
///              B B B . . .      (. = transparent)
///              . . . . . .
fn editor() -> Editor {
    let mut buf = PixelBuffer::new(6, 4).expect("valid");
    for y in 0..2 {
        for x in 0..6 {
            buf.set(x, y, RED);
        }
    }
    for (x, y) in [(2, 0), (2, 1), (0, 2), (1, 2), (2, 2)] {
        buf.set(x, y, BLUE);
    }
    buf.set(4, 1, NEAR_RED);
    let doc = Document::from_layers(6, 4, vec![("Ink".into(), true, buf)], 0).expect("doc");
    Editor::new(doc, None, "Masks".into())
}

fn selected(e: &Editor) -> Vec<(i32, i32)> {
    let Some(s) = e.selection_shape() else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for y in 0..4 {
        for x in 0..6 {
            if s.contains(x, y) {
                v.push((x, y));
            }
        }
    }
    v
}

fn px(e: &Editor, x: i32, y: i32) -> Rgba {
    e.document().active_cel().get(x, y).expect("inside")
}

// ---------------------------------------------------------------- the model

#[test]
fn masks_shrink_to_their_pixels_and_full_masks_become_rectangles() {
    let mut bits = vec![false; 16];
    bits[5] = true;
    bits[6] = true;
    bits[9] = true;
    let s = Selection::from_mask(rect(10, 10, 4, 4), &bits).expect("some");
    assert_eq!(s.bounds(), rect(11, 11, 2, 2));
    assert!(!s.is_rectangular());
    assert_eq!(s.pixel_count(), 3);
    assert!(s.contains(11, 12) && !s.contains(12, 12));
    bits[10] = true;
    let full = Selection::from_mask(rect(10, 10, 4, 4), &bits).expect("some");
    assert!(full.is_rectangular(), "a filled mask is just a rectangle");
    assert!(Selection::from_mask(rect(0, 0, 4, 4), &[false; 16]).is_none());
    assert!(
        Selection::from_mask(rect(0, 0, 4, 4), &[true; 3]).is_none(),
        "wrong size"
    );
}

#[test]
fn combine_and_invert() {
    let (w, h) = (6, 4);
    let a = Selection::from(rect(0, 0, 3, 2));
    let b = Selection::from(rect(2, 1, 3, 2));
    let add = Selection::combine(Some(&a), Some(&b), SelectMode::Add, w, h).expect("add");
    assert_eq!(add.pixel_count(), 6 + 6 - 1);
    assert_eq!(add.bounds(), rect(0, 0, 5, 3));
    let sub = Selection::combine(Some(&a), Some(&b), SelectMode::Subtract, w, h).expect("sub");
    assert_eq!(sub.pixel_count(), 5);
    assert!(!sub.contains(2, 1));
    let both = Selection::combine(Some(&a), Some(&b), SelectMode::Intersect, w, h).expect("and");
    assert_eq!(both.bounds(), rect(2, 1, 1, 1));
    let replaced = Selection::combine(Some(&a), Some(&b), SelectMode::Replace, w, h);
    assert_eq!(replaced, Some(b.clone()));
    // Subtracting everything leaves nothing; nothing to subtract from stays nothing.
    assert!(Selection::combine(Some(&a), Some(&a), SelectMode::Subtract, w, h).is_none());
    assert!(Selection::combine(None, Some(&a), SelectMode::Subtract, w, h).is_none());
    assert_eq!(
        Selection::combine(None, Some(&a), SelectMode::Add, w, h),
        Some(a.clone())
    );
    let inv = Selection::inverted(Some(&a), w, h).expect("inverse");
    assert_eq!(inv.pixel_count(), 24 - 6);
    assert_eq!(
        Selection::inverted(None, w, h),
        Some(Selection::from(rect(0, 0, 6, 4)))
    );
    assert!(Selection::inverted(Some(&Selection::from(rect(0, 0, 6, 4))), w, h).is_none());
}

#[test]
fn outlines_follow_the_mask() {
    // An L: 2×2 block missing its top-right pixel → 6 outline segments.
    let s = Selection::from_mask(rect(0, 0, 2, 2), &[true, false, true, true]).expect("L");
    let segs = s.outline();
    assert_eq!(segs.len(), 6, "{segs:?}");
    let length: i32 = segs
        .iter()
        .map(|[a, b]| (a.0 - b.0).abs() + (a.1 - b.1).abs())
        .sum();
    assert_eq!(length, 8, "perimeter of the L");
    assert_eq!(Selection::from(rect(1, 1, 3, 2)).outline().len(), 4);
}

// ---------------------------------------------------------------- tools

#[test]
fn magic_wand_tolerance_contiguous_and_source() {
    let mut e = editor();
    // Contiguous, exact: the top-left red block only.
    e.magic_wand((0, 0), SelectMode::Replace).expect("wand");
    assert_eq!(selected(&e), [(0, 0), (1, 0), (0, 1), (1, 1)]);
    // Global, exact: every red pixel except the slightly different one.
    e.settings.wand_contiguous = false;
    e.magic_wand((0, 0), SelectMode::Replace).expect("wand");
    assert_eq!(selected(&e).len(), 2 * 6 - 2 - 1);
    // With tolerance the near-red pixel joins.
    e.settings.wand_tolerance = 10;
    e.magic_wand((0, 0), SelectMode::Replace).expect("wand");
    assert_eq!(selected(&e).len(), 2 * 6 - 2);
    // Transparent area, contiguous.
    e.settings.wand_contiguous = true;
    e.magic_wand((5, 3), SelectMode::Replace).expect("wand");
    assert_eq!(selected(&e).len(), 3 + 6);
    // Shift-style add, Alt-style subtract.
    e.magic_wand((2, 0), SelectMode::Add).expect("wand");
    assert_eq!(selected(&e).len(), 9 + 5);
    e.magic_wand((5, 3), SelectMode::Subtract).expect("wand");
    assert_eq!(selected(&e).len(), 5);
    // Clicking outside the image does nothing.
    assert!(!e.magic_wand((-1, 0), SelectMode::Replace).expect("wand"));
    assert_eq!(selected(&e).len(), 5);
}

#[test]
fn magic_wand_can_use_the_merged_image() {
    let mut e = editor();
    e.add_layer().expect("layer");
    // The new empty top layer: on its own everything is one transparent area.
    e.settings.wand_source = SampleSource::ActiveLayer;
    e.magic_wand((0, 0), SelectMode::Replace).expect("wand");
    assert_eq!(selected(&e).len(), 24);
    e.settings.wand_source = SampleSource::Merged;
    e.magic_wand((0, 0), SelectMode::Replace).expect("wand");
    assert_eq!(selected(&e).len(), 4);
}

#[test]
fn lasso_selects_inside_and_combines() {
    let mut e = editor();
    e.select_lasso(&[(0, 0), (4, 0), (0, 3)], SelectMode::Replace);
    let s = selected(&e);
    assert!(s.contains(&(0, 0)) && s.contains(&(1, 1)) && s.contains(&(0, 3)));
    assert!(!s.contains(&(4, 3)) && !s.contains(&(5, 0)));
    let before = s.len();
    e.select_lasso(&[(5, 3)], SelectMode::Add);
    assert_eq!(selected(&e).len(), before + 1);
    e.invert_selection();
    assert_eq!(selected(&e).len(), 24 - before - 1);
}

// ---------------------------------------------------------------- edits respect the shape

/// Selects the blue L (2,0),(2,1),(0,2),(1,2),(2,2) with the wand.
fn select_blue(e: &mut Editor) {
    e.settings.wand_contiguous = true;
    e.magic_wand((2, 2), SelectMode::Replace).expect("wand");
    assert_eq!(selected(e).len(), 5);
}

#[test]
fn delete_and_cut_touch_only_selected_pixels() {
    let mut e = editor();
    select_blue(&mut e);
    assert!(e.delete_selection().expect("delete"));
    assert_eq!(px(&e, 2, 0), Rgba::TRANSPARENT);
    assert_eq!(px(&e, 0, 2), Rgba::TRANSPARENT);
    // Inside the bounds but not selected: kept.
    assert_eq!(px(&e, 0, 0), RED);
    assert_eq!(px(&e, 1, 1), RED);
    e.undo().expect("undo");
    assert_eq!(px(&e, 2, 0), BLUE);
    assert_eq!(selected(&e).len(), 5, "the shape is still selected");
}

#[test]
fn copy_paste_keeps_the_shape() {
    let mut e = editor();
    select_blue(&mut e);
    assert!(e.copy_selection());
    let clip = e.clipboard.clone().expect("clipboard");
    assert_eq!((clip.block.w, clip.block.h), (3, 3));
    // Unselected pixels in the bounds are transparent on the clipboard.
    assert_eq!(clip.block.get(0, 0), Rgba::TRANSPARENT);
    assert_eq!(clip.block.get(2, 0), BLUE);
    e.deselect();
    assert!(e.paste().expect("paste"));
    assert_eq!(selected(&e).len(), 5, "pasted with its shape");
}

#[test]
fn moving_a_shape_moves_only_its_pixels() {
    let mut e = editor();
    select_blue(&mut e);
    let shape = e.selection_shape().cloned().expect("shape");
    assert!(e.move_selection(3, 1).expect("move"));
    // Old blue pixels are gone, the red ones in the old bounds stay.
    assert_eq!(px(&e, 2, 0), Rgba::TRANSPARENT);
    assert_eq!(px(&e, 0, 0), RED);
    // The L landed 3 right, 1 down: (5,1),(5,2),(3,3),(4,3),(5,3).
    for (x, y) in [(5, 1), (5, 2), (3, 3), (4, 3), (5, 3)] {
        assert_eq!(px(&e, x, y), BLUE, "({x},{y})");
    }
    // Pixels covered by the moved bounds but outside the shape are untouched.
    assert_eq!(px(&e, 3, 1), RED);
    assert_eq!(e.selection_shape(), Some(&shape.translated(3, 1)));
    e.undo().expect("undo");
    assert_eq!(px(&e, 2, 0), BLUE);
    assert_eq!(px(&e, 5, 1), RED);
    assert_eq!(e.selection_shape(), Some(&shape));
}

#[test]
fn flipping_a_shape_flips_its_mask_too() {
    let mut e = editor();
    select_blue(&mut e);
    assert!(e.transform(Transform::FlipHorizontal).expect("flip"));
    // The L ┘ becomes └: blue at (0,0),(0,1),(0,2),(1,2),(2,2).
    let mut expect = vec![(0, 0), (0, 1), (0, 2), (1, 2), (2, 2)];
    expect.sort_by_key(|&(x, y)| (y, x));
    assert_eq!(selected(&e), expect);
    for (x, y) in expect {
        assert_eq!(px(&e, x, y), BLUE, "({x},{y})");
    }
    // The old corner pixel now has nothing on it; red that wasn't selected stays.
    assert_eq!(px(&e, 2, 0), Rgba::TRANSPARENT);
    assert_eq!(px(&e, 1, 0), RED);
}

#[test]
fn canvas_size_keeps_and_clips_the_shape() {
    use kagero::core::Anchor;
    let mut e = editor();
    select_blue(&mut e);
    let shape = e.selection_shape().cloned().expect("shape");
    // Grow to the right and down: nothing moves.
    e.resize_canvas(8, 6, Anchor { x: 0, y: 0 })
        .expect("resize");
    assert_eq!(e.selection_shape(), Some(&shape));
    e.undo().expect("undo");
    // Cut 1 px off the left: the shape moves left and loses its left column.
    e.resize_canvas(5, 4, Anchor { x: 2, y: 0 })
        .expect("resize");
    let s = e.selection_shape().expect("kept");
    assert_eq!(s.pixel_count(), 4);
    assert_eq!(s.bounds(), rect(0, 0, 2, 3));
}

//! Tools, strokes, undo/redo and layer operations through the editor layer.

use kagero::core::{Document, Rgba};
use kagero::editor::{ColorSlot, EditError, Editor, SampleSource, ToolKind};

const RED: Rgba = Rgba::opaque(255, 0, 0);
const GREEN: Rgba = Rgba::opaque(0, 255, 0);

fn editor(w: u32, h: u32) -> Editor {
    let doc = Document::new(w, h, Rgba::TRANSPARENT).expect("valid size");
    let mut e = Editor::new(doc, None, "Test".into());
    e.primary = RED;
    e.secondary = GREEN;
    e
}

fn px(e: &Editor, x: i32, y: i32) -> Rgba {
    e.document()
        .active_layer()
        .pixels
        .get(x, y)
        .expect("in bounds")
}

fn drag(e: &mut Editor, points: &[(i32, i32)]) {
    let (first, rest) = points.split_first().expect("at least one point");
    e.begin_stroke(*first, ColorSlot::Primary)
        .expect("stroke starts");
    for p in rest {
        e.update_stroke(*p);
    }
    e.end_stroke();
}

#[test]
fn pencil_connects_points_and_is_one_undo_step() {
    let mut e = editor(16, 16);
    drag(&mut e, &[(0, 0), (10, 0), (10, 5)]);
    for x in 0..=10 {
        assert_eq!(px(&e, x, 0), RED, "gap at x={x}");
    }
    for y in 0..=5 {
        assert_eq!(px(&e, 10, y), RED, "gap at y={y}");
    }
    assert_eq!(e.history().undo_len(), 1);
    assert!(e.is_modified());
    e.undo().expect("undo");
    assert!((0..16).all(|x| px(&e, x, 0) == Rgba::TRANSPARENT));
    assert!(!e.is_modified());
}

#[test]
fn secondary_button_uses_secondary_color() {
    let mut e = editor(4, 4);
    e.begin_stroke((1, 1), ColorSlot::Secondary)
        .expect("starts");
    e.end_stroke();
    assert_eq!(px(&e, 1, 1), GREEN);
}

#[test]
fn drawing_outside_the_image_is_ignored() {
    let mut e = editor(4, 4);
    drag(&mut e, &[(-5, -5), (-1, -1)]);
    assert_eq!(
        e.history().undo_len(),
        0,
        "no pixels changed, no history entry"
    );
    drag(&mut e, &[(-3, 1), (6, 1)]);
    for x in 0..4 {
        assert_eq!(px(&e, x, 1), RED);
    }
    drag(&mut e, &[(i32::MAX, i32::MIN)]);
}

#[test]
fn brush_size_paints_a_square() {
    let mut e = editor(8, 8);
    e.settings.brush_size = 3;
    drag(&mut e, &[(4, 4)]);
    let painted = (0..8)
        .flat_map(|y| (0..8).map(move |x| (x, y)))
        .filter(|&(x, y)| px(&e, x, y) == RED)
        .count();
    assert_eq!(painted, 9);
    assert_eq!(px(&e, 3, 3), RED);
    assert_eq!(px(&e, 5, 5), RED);
}

#[test]
fn eraser_makes_pixels_transparent() {
    let doc = Document::new(4, 4, Rgba::WHITE).expect("valid");
    let mut e = Editor::new(doc, None, "T".into());
    e.set_tool(ToolKind::Eraser);
    drag(&mut e, &[(0, 0), (3, 0)]);
    for x in 0..4 {
        assert_eq!(px(&e, x, 0), Rgba::TRANSPARENT);
    }
    assert_eq!(px(&e, 0, 1), Rgba::WHITE);
    let merged = e.document().composite().expect("valid");
    assert_eq!(merged.get(2, 0).map(|c| c.a()), Some(0));
}

#[test]
fn fill_is_limited_to_the_connected_region() {
    let mut e = editor(6, 6);
    // A vertical wall at x = 2 splits the image.
    drag(&mut e, &[(2, 0), (2, 5)]);
    e.set_tool(ToolKind::Fill);
    e.primary = GREEN;
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("fill");
    for y in 0..6 {
        assert_eq!(px(&e, 0, y), GREEN);
        assert_eq!(px(&e, 1, y), GREEN);
        assert_eq!(px(&e, 2, y), RED, "the wall is not filled");
        assert_eq!(px(&e, 3, y), Rgba::TRANSPARENT, "other side untouched");
    }
    assert_eq!(e.history().undo_len(), 2);
    e.undo().expect("undo fill");
    assert_eq!(px(&e, 0, 0), Rgba::TRANSPARENT);
    // Filling with the same color, or outside the image, changes nothing.
    e.primary = RED;
    e.begin_stroke((2, 2), ColorSlot::Primary)
        .expect("same color");
    e.begin_stroke((-1, 99), ColorSlot::Primary)
        .expect("outside");
    assert_eq!(e.history().undo_len(), 1);
}

#[test]
fn fill_covers_whole_empty_image() {
    let mut e = editor(64, 64);
    e.set_tool(ToolKind::Fill);
    e.begin_stroke((10, 10), ColorSlot::Primary).expect("fill");
    let merged = e.document().composite().expect("valid");
    assert!(merged.as_bytes().chunks(4).all(|p| p == RED.0));
}

#[test]
fn eyedropper_respects_sample_source() {
    let mut e = editor(4, 4);
    drag(&mut e, &[(1, 1)]);
    e.add_layer().expect("layer");
    e.set_tool(ToolKind::Eyedropper);

    e.settings.sample_source = SampleSource::Merged;
    e.primary = GREEN;
    e.begin_stroke((1, 1), ColorSlot::Primary).expect("pick");
    assert_eq!(e.primary, RED, "merged image shows the lower layer");

    e.settings.sample_source = SampleSource::ActiveLayer;
    e.begin_stroke((1, 1), ColorSlot::Secondary).expect("pick");
    assert_eq!(
        e.secondary,
        Rgba::TRANSPARENT,
        "the new layer is empty there"
    );

    e.begin_stroke((9, 9), ColorSlot::Primary).expect("outside");
    assert_eq!(e.primary, RED, "picking outside keeps the color");
    assert_eq!(e.history().undo_len(), 2, "picking is not an edit");
}

#[test]
fn line_and_rectangles_follow_the_pixel_grid() {
    let mut e = editor(10, 10);
    e.set_tool(ToolKind::Line);
    drag(&mut e, &[(0, 0), (5, 5), (9, 0)]);
    // Only the final shape stays; the preview at (5,5) was replaced.
    assert_eq!(px(&e, 0, 0), RED);
    assert_eq!(px(&e, 9, 0), RED);
    assert_eq!(px(&e, 5, 5), Rgba::TRANSPARENT);
    assert_eq!(e.history().undo_len(), 1);
    e.undo().expect("undo");

    e.set_tool(ToolKind::Rectangle);
    drag(&mut e, &[(7, 7), (2, 3)]);
    for x in 2..=7 {
        assert_eq!(px(&e, x, 3), RED);
        assert_eq!(px(&e, x, 7), RED);
    }
    for y in 3..=7 {
        assert_eq!(px(&e, 2, y), RED);
        assert_eq!(px(&e, 7, y), RED);
    }
    assert_eq!(px(&e, 4, 5), Rgba::TRANSPARENT, "outline only");
    e.undo().expect("undo");

    e.set_tool(ToolKind::FilledRectangle);
    drag(&mut e, &[(1, 1), (3, 2)]);
    let count = (0..10)
        .flat_map(|y| (0..10).map(move |x| (x, y)))
        .filter(|&(x, y)| px(&e, x, y) == RED)
        .count();
    assert_eq!(count, 6);
}

#[test]
fn canceling_a_stroke_restores_pixels() {
    let mut e = editor(8, 8);
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("start");
    e.update_stroke((7, 7));
    e.cancel_stroke();
    assert!((0..8).all(|i| px(&e, i, i) == Rgba::TRANSPARENT));
    assert_eq!(e.history().undo_len(), 0);
}

#[test]
fn hidden_layers_cannot_be_drawn_on() {
    let mut e = editor(4, 4);
    e.toggle_layer_visibility(0).expect("hide");
    assert_eq!(
        e.begin_stroke((0, 0), ColorSlot::Primary),
        Err(EditError::HiddenLayer)
    );
    e.set_tool(ToolKind::Fill);
    assert_eq!(
        e.begin_stroke((0, 0), ColorSlot::Primary),
        Err(EditError::HiddenLayer)
    );
}

#[test]
fn redo_reapplies_and_new_change_clears_redo() {
    let mut e = editor(4, 4);
    drag(&mut e, &[(0, 0)]);
    drag(&mut e, &[(1, 1)]);
    e.undo().expect("undo");
    e.undo().expect("undo");
    assert_eq!(e.history().redo_len(), 2);
    e.redo().expect("redo");
    assert_eq!(px(&e, 0, 0), RED);
    assert_eq!(px(&e, 1, 1), Rgba::TRANSPARENT);
    drag(&mut e, &[(3, 3)]);
    assert_eq!(e.history().redo_len(), 0, "a new change clears redo");
    assert_eq!(e.redo().expect("nothing"), None);
}

#[test]
fn saved_state_tracks_undo_position() {
    let mut e = editor(4, 4);
    drag(&mut e, &[(0, 0)]);
    e.mark_saved(std::path::PathBuf::from("x.kgr"));
    assert!(!e.is_modified());
    e.undo().expect("undo");
    assert!(e.is_modified());
    e.redo().expect("redo");
    assert!(!e.is_modified());
}

#[test]
fn layer_operations_and_their_undo() {
    let mut e = editor(4, 4);
    drag(&mut e, &[(0, 0)]);
    e.add_layer().expect("add");
    assert_eq!(e.document().layer_count(), 2);
    assert_eq!(e.document().active_index(), 1);
    assert_eq!(e.document().active_layer().name, "Layer 2");

    e.rename_layer(1, "  Ink  ").expect("rename");
    assert_eq!(e.document().layers()[1].name, "Ink");
    assert_eq!(e.rename_layer(1, "   "), Err(EditError::EmptyName));

    e.move_layer_down(1).expect("down");
    assert_eq!(e.document().layers()[0].name, "Ink");
    assert_eq!(
        e.document().active_layer().name,
        "Ink",
        "active follows the move"
    );

    e.toggle_layer_visibility(0).expect("hide");
    assert!(!e.document().layers()[0].visible);

    e.delete_layer(1).expect("delete Background");
    assert_eq!(e.document().layer_count(), 1);
    assert!(e.delete_layer(0).is_err(), "the last layer stays");

    // Undo everything in reverse order.
    e.undo().expect("undo delete");
    assert_eq!(e.document().layer_count(), 2);
    assert_eq!(e.document().layers()[1].name, "Background");
    assert_eq!(
        e.document().layers()[1].pixels.get(0, 0),
        Some(RED),
        "deleted pixels come back"
    );
    e.undo().expect("undo hide");
    assert!(e.document().layers()[0].visible);
    e.undo().expect("undo move");
    assert_eq!(e.document().layers()[1].name, "Ink");
    e.undo().expect("undo rename");
    assert_eq!(e.document().layers()[1].name, "Layer 2");
    e.undo().expect("undo add");
    assert_eq!(e.document().layer_count(), 1);
    assert_eq!(e.document().active_index(), 0);

    // And redo it all.
    for _ in 0..5 {
        e.redo().expect("redo");
    }
    assert_eq!(e.document().layer_count(), 1);
    assert_eq!(e.document().layers()[0].name, "Ink");
    assert!(!e.document().layers()[0].visible);
}

#[test]
fn drawing_affects_only_the_active_layer() {
    let mut e = editor(4, 4);
    e.add_layer().expect("add");
    drag(&mut e, &[(2, 2)]);
    assert_eq!(
        e.document().layers()[0].pixels.get(2, 2),
        Some(Rgba::TRANSPARENT)
    );
    assert_eq!(e.document().layers()[1].pixels.get(2, 2), Some(RED));
    e.select_layer(0);
    assert_eq!(e.document().active_index(), 0);
    e.select_layer(99);
    assert_eq!(e.document().active_index(), 0, "invalid index is ignored");
}

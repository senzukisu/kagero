//! Drawing tools of Kagero 0.4: ellipse, polygon, symmetry, dithering, brush tips,
//! custom brushes, the new fill options and Convert to Palette.

use kagero::core::dither::Dither;
use kagero::core::raster::BrushShape;
use kagero::core::selection::SelRect;
use kagero::core::{Document, PixelBuffer, Rgba};
use kagero::editor::{ColorSlot, EditError, Editor, PaletteScope, SampleSource, ToolKind};

const INK: Rgba = Rgba::opaque(20, 20, 30);
const RED: Rgba = Rgba::opaque(220, 30, 30);
const NEAR_RED: Rgba = Rgba::opaque(226, 34, 30);
const BLUE: Rgba = Rgba::opaque(30, 60, 220);

fn editor(w: u32, h: u32) -> Editor {
    let doc = Document::new(w, h, Rgba::TRANSPARENT).expect("valid size");
    let mut e = Editor::new(doc, None, "Draw".into());
    e.primary = INK;
    e
}

fn painted(e: &Editor) -> Vec<(i32, i32)> {
    let cel = e.document().active_cel();
    let mut v = Vec::new();
    for y in 0..cel.height() as i32 {
        for x in 0..cel.width() as i32 {
            if cel.get(x, y).is_some_and(|c| c.a() != 0) {
                v.push((x, y));
            }
        }
    }
    v
}

fn drag(e: &mut Editor, tool: ToolKind, a: (i32, i32), b: (i32, i32)) {
    e.set_tool(tool);
    e.begin_stroke(a, ColorSlot::Primary).expect("begin");
    e.update_stroke(b);
    e.end_stroke();
}

#[test]
fn ellipse_outline_and_filled_in_one_undo_step() {
    let mut e = editor(16, 16);
    drag(&mut e, ToolKind::Ellipse, (2, 3), (11, 8));
    let px = painted(&e);
    assert_eq!(px.iter().map(|p| p.0).min(), Some(2));
    assert_eq!(px.iter().map(|p| p.0).max(), Some(11));
    assert_eq!(px.iter().map(|p| p.1).min(), Some(3));
    assert_eq!(px.iter().map(|p| p.1).max(), Some(8));
    assert!(!px.contains(&(6, 5)), "an outline is hollow");
    e.undo().expect("undo");
    assert!(painted(&e).is_empty());
    e.settings.shape_filled = true;
    drag(&mut e, ToolKind::Ellipse, (2, 3), (11, 8));
    assert!(painted(&e).contains(&(6, 5)), "filled");
    assert!(painted(&e).len() > px.len());
}

#[test]
fn polygon_is_built_from_clicks_and_closes() {
    let mut e = editor(12, 12);
    e.set_tool(ToolKind::Polygon);
    e.settings.shape_filled = true;
    e.polygon_click((1, 1), ColorSlot::Primary).expect("click");
    assert!(e.is_drawing_polygon());
    e.polygon_click((10, 1), ColorSlot::Primary).expect("click");
    e.update_stroke((5, 9)); // hover preview: a triangle already
    assert!(painted(&e).contains(&(5, 4)));
    e.update_stroke((10, 10)); // the preview follows the cursor
    e.polygon_click((1, 10), ColorSlot::Primary).expect("click");
    // Clicking the last corner again finishes (that's what a double-click does).
    e.polygon_click((1, 10), ColorSlot::Primary)
        .expect("finish");
    assert!(!e.is_drawing_polygon());
    let px = painted(&e);
    assert!(px.contains(&(1, 1)) && px.contains(&(10, 1)) && px.contains(&(1, 10)));
    assert!(px.contains(&(3, 5)), "inside is filled");
    assert!(!px.contains(&(10, 10)), "the hover point is not part of it");
    e.undo().expect("undo");
    assert!(painted(&e).is_empty(), "one undo step");

    // Outline only, finished with Enter (end_stroke); Esc (cancel) leaves nothing.
    e.settings.shape_filled = false;
    for p in [(1, 1), (10, 1), (5, 9)] {
        e.polygon_click(p, ColorSlot::Primary).expect("click");
    }
    e.end_stroke();
    let outline = painted(&e);
    assert!(outline.contains(&(5, 1)) && !outline.contains(&(5, 4)));
    e.polygon_click((0, 0), ColorSlot::Primary).expect("click");
    e.polygon_click((6, 6), ColorSlot::Primary).expect("click");
    e.cancel_stroke();
    assert_eq!(painted(&e), outline);
}

#[test]
fn symmetry_mirrors_strokes_in_one_undo_step() {
    let mut e = editor(8, 6);
    e.settings.mirror_x = true;
    e.settings.mirror_y = true;
    drag(&mut e, ToolKind::Pencil, (1, 1), (1, 1));
    assert_eq!(painted(&e), [(1, 1), (6, 1), (1, 4), (6, 4)]);
    e.undo().expect("undo");
    assert!(painted(&e).is_empty());
    // An asymmetric 2×2 brush is mirrored too, so the result stays symmetric.
    e.settings.mirror_y = false;
    e.settings.brush_size = 2;
    drag(&mut e, ToolKind::Pencil, (2, 2), (2, 2));
    let px = painted(&e);
    for &(x, y) in &px {
        assert!(px.contains(&(7 - x, y)), "({x},{y}) has no mirror");
    }
}

#[test]
fn dithering_paints_its_pattern() {
    let mut e = editor(8, 8);
    e.settings.dither = Dither::Checker;
    drag(&mut e, ToolKind::FilledRectangle, (0, 0), (7, 7));
    assert_eq!(painted(&e).len(), 32);
    for (x, y) in painted(&e) {
        assert!(Dither::Checker.paints(x, y));
    }
    // A second pass with 25% adds nothing new: its pixels are part of the checker.
    e.settings.dither = Dither::Light25;
    drag(&mut e, ToolKind::FilledRectangle, (0, 0), (7, 7));
    assert_eq!(painted(&e).len(), 32);
}

#[test]
fn brush_tips_and_custom_brushes() {
    let mut e = editor(16, 16);
    e.settings.brush_shape = BrushShape::Circle;
    e.settings.brush_size = 3;
    drag(&mut e, ToolKind::Pencil, (5, 5), (5, 5));
    assert_eq!(painted(&e).len(), 5, "a 3 px round brush is a plus");
    e.undo().expect("undo");

    // Make an L-shaped brush from a selection.
    for p in [(0, 0), (0, 1), (1, 1)] {
        drag(&mut e, ToolKind::Pencil, p, p);
    }
    e.settings.brush_shape = BrushShape::Square;
    e.settings.brush_size = 1;
    for _ in 0..3 {
        e.undo().expect("undo");
    }
    assert!(matches!(
        e.brush_from_selection(),
        Err(EditError::NoSelection)
    ));
    for p in [(0, 0), (0, 1), (1, 1)] {
        drag(&mut e, ToolKind::Pencil, p, p);
    }
    e.set_selection(Some(SelRect {
        x: 0,
        y: 0,
        w: 2,
        h: 2,
    }));
    e.brush_from_selection().expect("brush");
    assert_eq!(e.settings.brush_shape, BrushShape::Custom);
    let brush = e.custom_brush.clone().expect("brush");
    assert_eq!(brush.offsets.len(), 3, "only the colored pixels");
    e.deselect();
    drag(&mut e, ToolKind::Pencil, (10, 10), (10, 10));
    let px = painted(&e);
    assert!(px.contains(&(10, 10)) && px.contains(&(10, 11)) && px.contains(&(11, 11)));
    assert!(!px.contains(&(11, 10)));
    // Too large selections are refused.
    e.set_selection(Some(SelRect {
        x: 0,
        y: 0,
        w: 16,
        h: 16,
    }));
    let doc = Document::new(100, 100, Rgba::TRANSPARENT).expect("doc");
    let mut big = Editor::new(doc, None, "Big".into());
    big.select_all();
    assert!(matches!(
        big.brush_from_selection(),
        Err(EditError::NotPossible(_))
    ));
}

/// 6×3:  R R B n R R       (n = NEAR_RED)
///       R R B R R R
///       B B B R R R
fn picture() -> Editor {
    let mut buf = PixelBuffer::filled(6, 3, RED).expect("buf");
    for (x, y) in [(2, 0), (2, 1), (0, 2), (1, 2), (2, 2)] {
        buf.set(x, y, BLUE);
    }
    buf.set(3, 0, NEAR_RED);
    let doc = Document::from_layers(6, 3, vec![("Ink".into(), true, buf)], 0).expect("doc");
    let mut e = Editor::new(doc, None, "Fill".into());
    e.set_tool(ToolKind::Fill);
    e
}

fn count(e: &Editor, c: Rgba) -> usize {
    e.document()
        .active_cel()
        .as_bytes()
        .chunks_exact(4)
        .filter(|p| *p == c.0)
        .count()
}

#[test]
fn fill_tolerance_contiguous_and_exact_undo() {
    let mut e = picture();
    let green = Rgba::opaque(0, 200, 0);
    e.fill((0, 0), green).expect("fill");
    assert_eq!(count(&e, green), 4, "the top-left red block only");
    e.undo().expect("undo");
    e.settings.fill_tolerance = 10;
    e.fill((4, 1), green).expect("fill");
    assert_eq!(
        count(&e, green),
        9,
        "the right block including the near-red pixel"
    );
    e.undo().expect("undo");
    assert_eq!(
        count(&e, NEAR_RED),
        1,
        "undo restores each pixel's own color"
    );
    e.settings.fill_contiguous = false;
    e.fill((0, 0), green).expect("fill");
    assert_eq!(
        count(&e, green),
        13,
        "every similar pixel (tolerance still 10)"
    );
    e.undo().expect("undo");
    // Filling with the same color changes nothing and adds no undo step.
    let state = e.history().state_id();
    e.settings.fill_tolerance = 0;
    e.fill((0, 0), RED).expect("fill");
    assert_eq!(e.history().state_id(), state);
}

#[test]
fn fill_stays_in_the_selection_and_uses_dither() {
    let mut e = picture();
    let green = Rgba::opaque(0, 200, 0);
    e.set_selection(Some(SelRect {
        x: 3,
        y: 1,
        w: 3,
        h: 2,
    }));
    e.fill((0, 0), green).expect("outside: nothing");
    assert_eq!(count(&e, green), 0);
    e.fill((4, 1), green).expect("fill");
    assert_eq!(count(&e, green), 6);
    e.undo().expect("undo");
    e.deselect();
    e.settings.dither = Dither::Checker;
    e.fill((4, 1), green).expect("fill");
    assert!(count(&e, green) < 8 && count(&e, green) > 0);
}

#[test]
fn fill_can_use_the_merged_image() {
    let mut e = picture();
    e.add_layer().expect("layer"); // empty top layer becomes active
    let green = Rgba::opaque(0, 200, 0);
    e.settings.fill_source = SampleSource::Merged;
    e.fill((0, 0), green).expect("fill");
    // The area comes from the image below; the paint goes on the new layer.
    assert_eq!(count(&e, green), 4);
}

#[test]
fn big_fills_finish_quickly() {
    let mut e = editor(2048, 2048);
    let start = std::time::Instant::now();
    e.fill((10, 10), RED).expect("fill");
    let took = start.elapsed();
    assert_eq!(count(&e, RED), 2048 * 2048);
    // Generous bound for slow debug builds; release takes a fraction of this.
    eprintln!("2048×2048 fill: {took:?}");
    assert!(took.as_secs_f64() < 10.0, "{took:?}");
    e.undo().expect("undo");
    assert_eq!(count(&e, RED), 0);
}

#[test]
fn convert_to_palette_with_preview_and_undo() {
    let mut e = picture();
    // A second, locked layer with a color that would change.
    e.add_layer().expect("layer");
    e.set_tool(ToolKind::Pencil);
    e.primary = Rgba::new(250, 250, 250, 128);
    drag(&mut e, ToolKind::Pencil, (5, 2), (5, 2));
    let mut props = e.document().active_layer().props();
    props.locked = true;
    e.set_layer_props(1, props).expect("lock");
    e.select_layer(0);
    let palette = [
        Rgba::opaque(255, 0, 0),
        Rgba::opaque(0, 0, 255),
        Rgba::TRANSPARENT,
    ];

    let (before, after) = e
        .palette_preview(&palette, PaletteScope::All)
        .expect("preview");
    assert_eq!(before.width(), 6);
    let changed = e
        .convert_to_palette(&palette, PaletteScope::All)
        .expect("convert");
    assert_eq!(changed, 18, "every pixel of the unlocked layer");
    assert_eq!(count(&e, Rgba::opaque(255, 0, 0)), 13);
    assert_eq!(count(&e, Rgba::opaque(0, 0, 255)), 5);
    // The locked layer keeps its pixel (with its alpha).
    assert_eq!(
        e.document().cel(0, 1).and_then(|c| c.get(5, 2)),
        Some(Rgba::new(250, 250, 250, 128))
    );
    // The preview showed exactly the result.
    assert_eq!(after, e.document().composite_frame(0).expect("composite"));
    e.undo().expect("undo");
    assert_eq!(count(&e, NEAR_RED), 1);
    // Converting again with colors already in the palette changes nothing.
    assert_eq!(
        e.convert_to_palette(&[RED, NEAR_RED, BLUE], PaletteScope::LayerFrame)
            .expect("convert"),
        0
    );
    assert!(matches!(
        e.convert_to_palette(&[Rgba::TRANSPARENT], PaletteScope::Layer),
        Err(EditError::NotPossible(_))
    ));
}

//! Layer opacity, locks, duplicate, merge down, canvas size and crop (Kagero 0.4).

use std::io::Cursor;

use kagero::core::{Anchor, Document, LayerProps, Rgba};
use kagero::editor::{ColorSlot, EditError, Editor, ToolKind};
use kagero::file_io::kgr::{read_kgr, write_kgr};

const RED: Rgba = Rgba::opaque(255, 0, 0);
const BLUE: Rgba = Rgba::opaque(0, 0, 255);

fn editor(w: u32, h: u32) -> Editor {
    let doc = Document::new(w, h, Rgba::TRANSPARENT).expect("valid size");
    let mut e = Editor::new(doc, None, "Test".into());
    e.primary = RED;
    e
}

fn dot(e: &mut Editor, x: i32, y: i32, c: Rgba) {
    e.set_tool(ToolKind::Pencil);
    e.primary = c;
    e.begin_stroke((x, y), ColorSlot::Primary).expect("draw");
    e.end_stroke();
}

fn px(e: &Editor, x: i32, y: i32) -> Rgba {
    e.document().active_cel().get(x, y).expect("in bounds")
}

fn composite(e: &Editor, frame: usize) -> Vec<u8> {
    e.document()
        .composite_frame(frame)
        .expect("composite")
        .as_bytes()
        .to_vec()
}

/// Two layers in two frames: red on the bottom, blue on top.
fn two_layers_two_frames() -> Editor {
    let mut e = editor(4, 4);
    dot(&mut e, 0, 0, RED);
    dot(&mut e, 1, 1, RED);
    e.add_layer().expect("layer");
    dot(&mut e, 1, 1, BLUE);
    dot(&mut e, 2, 2, BLUE);
    e.duplicate_frame().expect("frame");
    dot(&mut e, 3, 3, BLUE);
    e
}

fn round_trip(doc: &Document) -> (Document, u32) {
    let bytes = write_kgr(doc, Cursor::new(Vec::new()))
        .expect("write")
        .into_inner();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.clone())).expect("zip");
    let manifest: serde_json::Value =
        serde_json::from_reader(zip.by_name("manifest.json").expect("manifest")).expect("json");
    let version = manifest["format_version"].as_u64().expect("version") as u32;
    (read_kgr(Cursor::new(bytes)).expect("read"), version)
}

// ---------------------------------------------------------------- opacity

#[test]
fn opacity_changes_the_look_but_not_the_pixels() {
    let mut e = editor(2, 1);
    dot(&mut e, 0, 0, RED);
    let props = LayerProps {
        opacity: 128,
        ..e.document().active_layer().props()
    };
    e.set_layer_props(0, props).expect("opacity");
    assert_eq!(px(&e, 0, 0), RED, "the layer's own pixels are untouched");
    let c = e.document().composite_pixel(0, 0).expect("pixel");
    assert_eq!((c.r(), c.a()), (255, 128));
    e.undo().expect("undo");
    assert_eq!(e.document().composite_pixel(0, 0), Some(RED));
    e.redo().expect("redo");
    assert_eq!(e.document().active_layer().opacity, 128);
}

#[test]
fn slider_drags_are_one_undo_step() {
    let mut e = editor(2, 1);
    for v in [200, 150, 90] {
        e.preview_layer_opacity(0, v);
    }
    e.commit_layer_opacity(0, 255).expect("commit");
    assert_eq!(e.document().active_layer().opacity, 90);
    e.undo().expect("undo");
    assert_eq!(e.document().active_layer().opacity, 255);
    assert!(e.undo().is_err() || e.document().active_layer().opacity == 255);
}

#[test]
fn opacity_is_saved_and_needs_format_3_only_when_used() {
    let mut e = editor(3, 3);
    dot(&mut e, 1, 1, RED);
    let (_, version) = round_trip(e.document());
    assert_eq!(
        version, 2,
        "projects without opacity stay readable by Kagero 0.3"
    );

    let mut props = e.document().active_layer().props();
    props.opacity = 77;
    props.alpha_lock = true;
    e.set_layer_props(0, props).expect("props");
    let (back, version) = round_trip(e.document());
    assert_eq!(version, 3);
    assert_eq!(back.layers()[0].opacity, 77);
    assert!(back.layers()[0].alpha_lock);
    assert_eq!(
        back.composite().expect("c").as_bytes(),
        e.document().composite().expect("c").as_bytes()
    );
}

// ---------------------------------------------------------------- locks

#[test]
fn locked_layers_refuse_every_edit() {
    let mut e = editor(4, 4);
    dot(&mut e, 0, 0, RED);
    let mut props = e.document().active_layer().props();
    props.locked = true;
    e.set_layer_props(0, props).expect("lock");
    e.set_tool(ToolKind::Pencil);
    assert!(matches!(
        e.begin_stroke((1, 1), ColorSlot::Primary),
        Err(EditError::LockedLayer)
    ));
    assert!(matches!(e.fill((2, 2), BLUE), Err(EditError::LockedLayer)));
    e.select_all();
    assert!(matches!(e.delete_selection(), Err(EditError::LockedLayer)));
    assert_eq!(px(&e, 0, 0), RED);
}

#[test]
fn alpha_lock_only_recolors_existing_pixels() {
    let mut e = editor(4, 1);
    dot(&mut e, 0, 0, Rgba::new(255, 0, 0, 128));
    let mut props = e.document().active_layer().props();
    props.alpha_lock = true;
    e.set_layer_props(0, props).expect("alpha lock");
    e.set_tool(ToolKind::Pencil);
    e.primary = BLUE;
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("draw");
    e.update_stroke((3, 0));
    e.end_stroke();
    assert_eq!(
        px(&e, 0, 0),
        Rgba::new(0, 0, 255, 128),
        "recolored, alpha kept"
    );
    assert_eq!(px(&e, 2, 0), Rgba::TRANSPARENT, "empty pixels stay empty");
    // Erasing would change alpha, so it does nothing.
    e.set_tool(ToolKind::Eraser);
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("erase");
    e.end_stroke();
    assert_eq!(px(&e, 0, 0).a(), 128);
    // Fill also respects it.
    e.fill((1, 0), RED).expect("fill");
    assert_eq!(px(&e, 1, 0), Rgba::TRANSPARENT);
}

// ---------------------------------------------------------------- duplicate

#[test]
fn duplicate_copies_every_frame_independently() {
    let mut e = two_layers_two_frames();
    let mut props = e.document().active_layer().props();
    props.opacity = 100;
    e.set_layer_props(1, props).expect("opacity");
    e.duplicate_layer(1).expect("duplicate");
    let doc = e.document();
    assert_eq!(doc.layer_count(), 3);
    assert_eq!(doc.layers()[2].name, "Layer 2 copy");
    assert_eq!(doc.layers()[2].opacity, 100);
    assert_ne!(doc.layers()[2].id, doc.layers()[1].id);
    for f in 0..2 {
        assert_eq!(doc.cel(f, 2), doc.cel(f, 1), "frame {f} copied");
    }
    // Drawing on the copy leaves the original alone.
    e.select_layer(2);
    dot(&mut e, 0, 3, RED);
    assert_ne!(e.document().cel(1, 2), e.document().cel(1, 1));
    // A second copy gets a numbered name; undo removes it.
    e.duplicate_layer(1).expect("again");
    assert_eq!(e.document().layers()[2].name, "Layer 2 copy 2");
    e.undo().expect("undo");
    assert_eq!(e.document().layer_count(), 3);
}

// ---------------------------------------------------------------- merge down

#[test]
fn merge_down_looks_the_same_in_every_frame_and_undoes() {
    let mut e = two_layers_two_frames();
    let mut props = e.document().active_layer().props();
    props.opacity = 140;
    e.set_layer_props(1, props).expect("opacity");
    let mut bottom = e.document().layers()[0].props();
    bottom.opacity = 200;
    e.set_layer_props(0, bottom).expect("opacity");
    let before: Vec<Vec<u8>> = (0..2).map(|f| composite(&e, f)).collect();
    let snapshot: Vec<_> = (0..2)
        .map(|f| {
            (
                e.document().cel(f, 0).cloned(),
                e.document().cel(f, 1).cloned(),
            )
        })
        .collect();

    e.merge_down(1).expect("merge");
    assert_eq!(e.document().layer_count(), 1);
    assert_eq!(e.document().active_index(), 0);
    assert_eq!(e.document().layers()[0].opacity, 255, "opacity is baked in");
    for (f, before) in before.iter().enumerate() {
        let after = composite(&e, f);
        for (a, b) in before.chunks(4).zip(after.chunks(4)) {
            for (x, y) in a.iter().zip(b) {
                assert!(x.abs_diff(*y) <= 1, "frame {f}: {a:?} vs {b:?}");
            }
        }
    }

    e.undo().expect("undo");
    assert_eq!(e.document().layer_count(), 2);
    for (f, (b, t)) in snapshot.iter().enumerate() {
        assert_eq!(e.document().cel(f, 0).cloned(), *b);
        assert_eq!(e.document().cel(f, 1).cloned(), *t);
    }
    assert_eq!(e.document().layers()[0].opacity, 200);
    e.redo().expect("redo");
    assert_eq!(e.document().layer_count(), 1);
}

#[test]
fn merge_down_refuses_unsuitable_layers() {
    let mut e = two_layers_two_frames();
    assert!(e.merge_down(0).is_err(), "nothing below the bottom layer");
    e.toggle_layer_visibility(0).expect("hide");
    assert!(e.merge_down(1).is_err(), "hidden layers are not merged");
    e.toggle_layer_visibility(0).expect("show");
    let mut props = e.document().layers()[1].props();
    props.locked = true;
    e.set_layer_props(1, props).expect("lock");
    assert!(matches!(e.merge_down(1), Err(EditError::LockedLayer)));
    assert_eq!(e.document().layer_count(), 2);
}

// ---------------------------------------------------------------- canvas size

#[test]
fn canvas_size_keeps_pixels_at_the_anchor_in_every_layer_and_frame() {
    let mut e = two_layers_two_frames();
    let before: Vec<Vec<u8>> = (0..2).map(|f| composite(&e, f)).collect();
    e.resize_canvas(8, 6, Anchor::CENTER).expect("resize");
    let doc = e.document();
    assert_eq!((doc.width(), doc.height()), (8, 6));
    // Centered: old (0,0) moves to (2,1).
    assert_eq!(doc.cel(0, 0).expect("cel").get(2, 1), Some(RED));
    assert_eq!(
        doc.cel(1, 1).expect("cel").get(5, 4),
        Some(BLUE),
        "frame 2 too"
    );
    assert_eq!(
        doc.cel(0, 0).expect("cel").get(0, 0),
        Some(Rgba::TRANSPARENT)
    );

    e.undo().expect("undo");
    assert_eq!((e.document().width(), e.document().height()), (4, 4));
    for (f, b) in before.iter().enumerate() {
        assert_eq!(&composite(&e, f), b);
    }

    // Top-left anchor, smaller: pixels on the right and bottom are cut off.
    e.resize_canvas(2, 2, Anchor { x: 0, y: 0 })
        .expect("shrink");
    assert_eq!(e.document().cel(0, 0).expect("cel").get(0, 0), Some(RED));
    assert_eq!(e.document().cel(0, 0).expect("cel").get(1, 1), Some(RED));
    // Bottom-right anchor keeps (2,2)…(3,3).
    e.undo().expect("undo");
    e.resize_canvas(2, 2, Anchor { x: 2, y: 2 })
        .expect("shrink");
    assert_eq!(e.document().cel(0, 1).expect("cel").get(0, 0), Some(BLUE));
}

#[test]
fn canvas_size_checks_limits_before_allocating() {
    let mut e = editor(4, 4);
    assert!(e.resize_canvas(0, 4, Anchor::CENTER).is_err());
    assert!(e.resize_canvas(4097, 4, Anchor::CENTER).is_err());
    // 2048² × 4 bytes × 64 cels is over the 512 MiB budget.
    for _ in 0..39 {
        e.add_layer().expect("layer");
    }
    for _ in 0..1 {
        e.duplicate_frame().expect("frame");
    }
    let err = e.resize_canvas(2048, 2048, Anchor::CENTER);
    assert!(err.is_err(), "{err:?}");
    assert_eq!(e.document().width(), 4, "nothing changed");
}

#[test]
fn resize_moves_and_clips_the_selection() {
    let mut e = editor(4, 4);
    e.select_all();
    e.resize_canvas(2, 2, Anchor { x: 0, y: 0 })
        .expect("shrink");
    let sel = e.document().selection().expect("still selected");
    assert_eq!((sel.x, sel.y, sel.w, sel.h), (0, 0, 2, 2));
}

// ---------------------------------------------------------------- crop

#[test]
fn crop_cuts_every_layer_and_frame_at_the_same_place() {
    let mut e = two_layers_two_frames();
    e.crop(1, 1, 3, 3).expect("crop");
    let doc = e.document();
    assert_eq!((doc.width(), doc.height()), (3, 3));
    assert_eq!(
        doc.cel(0, 0).expect("cel").get(0, 0),
        Some(RED),
        "old (1,1)"
    );
    assert_eq!(
        doc.cel(0, 1).expect("cel").get(1, 1),
        Some(BLUE),
        "old (2,2)"
    );
    assert_eq!(
        doc.cel(1, 1).expect("cel").get(2, 2),
        Some(BLUE),
        "frame 2: old (3,3)"
    );
    assert!(e.crop(2, 2, 5, 5).is_err(), "outside the canvas");
    e.undo().expect("undo");
    assert_eq!(e.document().width(), 4);
    assert_eq!(e.document().cel(0, 0).expect("cel").get(0, 0), Some(RED));
}

#[test]
fn cropped_projects_round_trip() {
    let mut e = two_layers_two_frames();
    e.crop(0, 0, 3, 2).expect("crop");
    let (back, _) = round_trip(e.document());
    assert_eq!((back.width(), back.height()), (3, 2));
    assert_eq!(back.frame_count(), 2);
    assert_eq!(back.layer_count(), 2);
}

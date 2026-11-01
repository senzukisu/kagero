//! Frame tags: editing, undo, staying valid through frame edits, saving, playback.

use std::io::Cursor;

use kagero::core::tags::{MAX_TAGS, TAG_COLORS};
use kagero::core::{Document, FrameTag, Rgba};
use kagero::editor::{EditError, Editor};
use kagero::file_io::kgr::{read_kgr, write_kgr};

/// An editor with `n` frames.
fn editor(n: usize) -> Editor {
    let doc = Document::new(4, 4, Rgba::TRANSPARENT).expect("valid size");
    let mut e = Editor::new(doc, None, "Tags".into());
    for _ in 1..n {
        e.add_frame().expect("frame");
    }
    e.select_frame(0);
    e
}

fn ranges(e: &Editor) -> Vec<(usize, usize)> {
    e.document()
        .tags()
        .iter()
        .map(|t| (t.start, t.end))
        .collect()
}

fn tag(name: &str, start: usize, end: usize) -> FrameTag {
    FrameTag {
        name: name.into(),
        start,
        end,
        color: TAG_COLORS[2],
    }
}

#[test]
fn add_edit_delete_with_undo() {
    let mut e = editor(6);
    let i = e.add_tag(1, 3).expect("tag");
    assert_eq!(i, 0);
    assert_eq!(e.document().tags()[0].name, "Tag 1");
    // Reversed ranges are put in order.
    e.add_tag(5, 4).expect("tag");
    assert_eq!(ranges(&e), [(1, 3), (4, 5)]);
    assert_ne!(e.document().tags()[0].color, e.document().tags()[1].color);

    e.update_tag(0, tag("  walk  ", 0, 2)).expect("edit");
    assert_eq!(e.document().tags()[0].name, "walk");
    assert_eq!(ranges(&e), [(0, 2), (4, 5)]);
    e.remove_tag(1).expect("delete");
    assert_eq!(ranges(&e), [(0, 2)]);

    e.undo().expect("undo delete");
    assert_eq!(ranges(&e), [(0, 2), (4, 5)]);
    e.undo().expect("undo edit");
    assert_eq!(e.document().tags()[0].name, "Tag 1");
    e.undo().expect("undo add");
    e.undo().expect("undo add");
    assert!(e.document().tags().is_empty());
    e.redo().expect("redo");
    assert_eq!(ranges(&e), [(1, 3)]);
}

#[test]
fn invalid_tags_are_refused() {
    let mut e = editor(3);
    assert!(e.add_tag(1, 3).is_err(), "past the last frame");
    assert!(matches!(
        e.insert_tag(tag("   ", 0, 1)),
        Err(EditError::NotPossible(_))
    ));
    e.add_tag(0, 0).expect("tag");
    assert!(e.update_tag(0, tag("x", 0, 9)).is_err());
    assert!(e.update_tag(7, tag("x", 0, 0)).is_err());
    // An unchanged edit is not an undo step.
    let before = e.history().state_id();
    let same = e.document().tags()[0].clone();
    e.update_tag(0, same).expect("same");
    assert_eq!(e.history().state_id(), before);
}

#[test]
fn tag_count_is_limited() {
    let mut e = editor(1);
    for _ in 0..MAX_TAGS {
        e.add_tag(0, 0).expect("tag");
    }
    assert!(matches!(e.add_tag(0, 0), Err(EditError::NotPossible(_))));
}

#[test]
fn new_frames_inside_or_right_after_a_tag_join_it() {
    let mut e = editor(5);
    e.add_tag(1, 2).expect("walk");
    e.add_tag(3, 4).expect("idle");
    // Duplicating the last frame of "walk" adds it to "walk" and shifts "idle".
    e.select_frame(2);
    e.duplicate_frame().expect("dup");
    assert_eq!(ranges(&e), [(1, 3), (4, 5)]);
    // A frame before every tag shifts them all.
    e.select_frame(0);
    e.add_frame().expect("add");
    // Inserted at 1 = the start of "walk": it lands before it.
    assert_eq!(ranges(&e), [(2, 4), (5, 6)]);
    // One undo step per frame edit restores the tags exactly.
    e.undo().expect("undo");
    assert_eq!(ranges(&e), [(1, 3), (4, 5)]);
    e.undo().expect("undo");
    assert_eq!(ranges(&e), [(1, 2), (3, 4)]);
}

#[test]
fn deleting_frames_shrinks_and_undo_brings_tags_back() {
    let mut e = editor(5);
    e.add_tag(1, 3).expect("walk");
    e.add_tag(4, 4).expect("blink");
    e.delete_frame(2).expect("delete");
    assert_eq!(ranges(&e), [(1, 2), (3, 3)]);
    // Deleting the only frame of a tag removes the tag...
    e.delete_frame(3).expect("delete");
    assert_eq!(ranges(&e), [(1, 2)]);
    // ...and one undo restores the frame and the tag with its name and color.
    e.undo().expect("undo");
    assert_eq!(ranges(&e), [(1, 2), (3, 3)]);
    assert_eq!(e.document().tags()[1].name, "Tag 2");
    e.undo().expect("undo");
    assert_eq!(ranges(&e), [(1, 3), (4, 4)]);
    assert_eq!(e.document().frame_count(), 5);
    e.redo().expect("redo");
    e.redo().expect("redo");
    assert_eq!(ranges(&e), [(1, 2)]);
}

#[test]
fn moving_frames_keeps_tag_ranges() {
    let mut e = editor(4);
    e.add_tag(1, 2).expect("tag");
    let first = e.document().frames()[2].id;
    e.move_frame_right(2).expect("move");
    // The tag marks positions 1–2; the moved frame left it, frame 3's content entered.
    assert_eq!(ranges(&e), [(1, 2)]);
    assert_eq!(e.document().frames()[3].id, first);
    e.move_frame_left(1).expect("move");
    assert_eq!(ranges(&e), [(1, 2)]);
}

#[test]
fn tags_survive_save_and_keep_the_old_format_version() {
    let mut e = editor(4);
    e.insert_tag(FrameTag {
        name: "walk cycle".into(),
        start: 1,
        end: 3,
        color: TAG_COLORS[5],
    })
    .expect("tag");
    e.add_tag(0, 0).expect("tag");
    let bytes = write_kgr(e.document(), Cursor::new(Vec::new()))
        .expect("write")
        .into_inner();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.clone())).expect("zip");
    let manifest: serde_json::Value =
        serde_json::from_reader(zip.by_name("manifest.json").expect("manifest")).expect("json");
    // Tags don't need a new format version: older Kagero simply ignores them.
    assert_eq!(manifest["format_version"], 2);
    let loaded = read_kgr(Cursor::new(bytes)).expect("read");
    assert_eq!(loaded.tags(), e.document().tags());

    // A project without tags writes no "tags" field at all.
    let plain = write_kgr(editor(2).document(), Cursor::new(Vec::new()))
        .expect("write")
        .into_inner();
    let mut zip = zip::ZipArchive::new(Cursor::new(plain)).expect("zip");
    let doc: serde_json::Value =
        serde_json::from_reader(zip.by_name("document.json").expect("doc")).expect("json");
    assert!(doc.get("tags").is_none());
}

#[test]
fn damaged_tags_in_a_file_are_dropped_not_fatal() {
    let mut e = editor(3);
    e.add_tag(0, 1).expect("tag");
    let bytes = write_kgr(e.document(), Cursor::new(Vec::new()))
        .expect("write")
        .into_inner();
    // Rewrite document.json with broken tags.
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip");
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).expect("entry");
        let name = f.name().to_owned();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data).expect("read");
        if name == "document.json" {
            let mut doc: serde_json::Value = serde_json::from_slice(&data).expect("json");
            doc["tags"] = serde_json::json!([
                {"name": "ok", "start": 1, "end": 2, "color": "#e86a33"},
                {"name": "past the end", "start": 1, "end": 9, "color": "#e86a33"},
                {"name": "backwards", "start": 2, "end": 1, "color": "#e86a33"},
                {"name": "   ", "start": 0, "end": 0, "color": "#e86a33"},
                {"name": "bad color", "start": 0, "end": 0, "color": "nope"}
            ]);
            data = serde_json::to_vec(&doc).expect("json");
        }
        out.start_file(name, zip::write::SimpleFileOptions::default())
            .expect("start");
        std::io::Write::write_all(&mut out, &data).expect("write");
    }
    let bytes = out.finish().expect("finish").into_inner();
    let loaded = read_kgr(Cursor::new(bytes)).expect("still opens");
    let names: Vec<&str> = loaded.tags().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["ok", "bad color"]);
    assert_eq!(
        loaded.tags()[1].color,
        TAG_COLORS[0],
        "unreadable color → default"
    );
}

#[test]
fn preview_loops_only_the_tag() {
    use kagero::config::PlaybackSettings;
    use kagero::ui::animation_preview::Player;
    let mut e = editor(6);
    let ctx = egui::Context::default();
    let settings = PlaybackSettings::default();
    let mut player = Player::default();
    // The active frame (0) is outside the range: playback starts at the tag.
    player.start(&mut e, &settings, Some(2..5));
    assert_eq!(e.document().active_frame_index(), 2);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..40 {
        player.tick(&ctx, &mut e, &settings);
        let f = e.document().active_frame_index();
        assert!((2..5).contains(&f), "frame {f} is outside the loop");
        seen.insert(f);
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    assert!(seen.contains(&2) && seen.contains(&4), "it loops: {seen:?}");
    player.stop(&mut e);
    // An empty or out-of-range range plays everything.
    player.start(&mut e, &settings, Some(4..9));
    player.tick(&ctx, &mut e, &settings);
    player.stop(&mut e);
}

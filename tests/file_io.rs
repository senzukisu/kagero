//! KGR round trips, PNG import/export, damaged files and limits.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use kagero::core::{Document, PixelBuffer, Rgba};
use kagero::editor::{ColorSlot, Editor};
use kagero::file_io::{
    self, export_png, import_png, load_kgr, read_kgr, save_kgr, write_kgr, FileError,
};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// A fresh, empty directory for one test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("kagero-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sample_document() -> Document {
    let doc = Document::new(5, 3, Rgba::WHITE).expect("valid");
    let mut e = Editor::new(doc, None, "Sample".into());
    e.primary = Rgba::new(255, 157, 66, 255);
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("draw");
    e.update_stroke((4, 0));
    e.end_stroke();
    e.add_layer().expect("layer");
    e.primary = Rgba::new(10, 20, 30, 128);
    e.begin_stroke((2, 2), ColorSlot::Primary).expect("draw");
    e.end_stroke();
    e.rename_layer(1, "Ink — 陽炎").expect("rename");
    e.add_layer().expect("layer");
    e.toggle_layer_visibility(2).expect("hide");
    e.select_layer(1);
    e.document().clone()
}

fn assert_same(a: &Document, b: &Document) {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    assert_eq!(a.layer_count(), b.layer_count());
    assert_eq!(a.active_index(), b.active_index());
    assert_eq!(a.frame_count(), b.frame_count());
    assert_eq!(a.active_frame_index(), b.active_frame_index());
    for (la, lb) in a.layers().iter().zip(b.layers()) {
        assert_eq!(la.id, lb.id);
        assert_eq!(la.name, lb.name);
        assert_eq!(la.visible, lb.visible);
    }
    for (fa, fb) in a.frames().iter().zip(b.frames()) {
        assert_eq!(fa.id, fb.id);
        assert_eq!(fa.duration_ms(), fb.duration_ms());
        assert_eq!(fa.cels(), fb.cels(), "pixels of frame {:?}", fa.id);
    }
}

fn kgr_bytes(doc: &Document) -> Vec<u8> {
    write_kgr(doc, Cursor::new(Vec::new()))
        .expect("write")
        .into_inner()
}

/// Builds an archive from raw entries.
fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(*name, SimpleFileOptions::default())
            .expect("start");
        zip.write_all(data).expect("write");
    }
    zip.finish().expect("finish").into_inner()
}

const MANIFEST: &[u8] = br#"{"format":"kagero-project","format_version":1}"#;

fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let buf = PixelBuffer::filled(w, h, Rgba::WHITE).expect("valid");
    let mut out = Vec::new();
    file_io::png::encode_png(&buf, &mut out).expect("encode");
    out
}

#[test]
fn kgr_round_trip_in_memory() {
    let doc = sample_document();
    let loaded = read_kgr(Cursor::new(kgr_bytes(&doc))).expect("read");
    assert_same(&doc, &loaded);
    assert_eq!(loaded.layers()[1].name, "Ink — 陽炎");
    assert!(!loaded.layers()[2].visible);
    assert_eq!(
        loaded.cel(0, 1).and_then(|c| c.get(2, 2)),
        Some(Rgba::new(10, 20, 30, 128)),
        "semi-transparent pixels are stored exactly"
    );
}

#[test]
fn kgr_round_trip_on_disk_replaces_existing_file() {
    let dir = temp_dir("roundtrip");
    let path = dir.join("art.kgr");
    std::fs::write(&path, b"old contents").expect("seed");
    let doc = sample_document();
    save_kgr(&doc, &path).expect("save");
    let loaded = load_kgr(&path).expect("load");
    assert_same(&doc, &loaded);
    // No temporary files are left behind.
    let names: Vec<_> = std::fs::read_dir(&dir)
        .expect("list")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("art.kgr")]);
}

#[test]
fn failed_save_keeps_the_old_file() {
    let dir = temp_dir("failed-save");
    let path = dir.join("missing-dir").join("art.kgr");
    let err = save_kgr(&sample_document(), &path).expect_err("directory does not exist");
    assert!(matches!(err, FileError::Io(_)));
    assert!(!path.exists());
}

#[test]
fn png_export_flattens_with_transparency() {
    let dir = temp_dir("export");
    let path = dir.join("out.png");
    let mut doc = Document::new(3, 2, Rgba::TRANSPARENT).expect("valid");
    let mut e = Editor::new(doc.clone(), None, "x".into());
    e.primary = Rgba::opaque(255, 0, 0);
    e.begin_stroke((1, 1), ColorSlot::Primary).expect("draw");
    e.end_stroke();
    doc = e.document().clone();
    export_png(&doc, &path).expect("export");

    let img = image::open(&path).expect("decodes").into_rgba8();
    assert_eq!(img.dimensions(), (3, 2));
    assert_eq!(img.get_pixel(1, 1).0, [255, 0, 0, 255]);
    assert_eq!(img.get_pixel(0, 0).0[3], 0, "transparent stays transparent");

    let imported = import_png(&path).expect("import");
    assert_eq!(imported.layer_count(), 1);
    assert_eq!(imported.active_layer().name, "out");
    assert_eq!(*imported.active_cel(), doc.composite().expect("valid"));
}

#[test]
fn png_export_skips_hidden_layers() {
    let dir = temp_dir("export-hidden");
    let path = dir.join("hidden.png");
    let doc = sample_document();
    export_png(&doc, &path).expect("export");
    let img = image::open(&path).expect("decodes").into_rgba8();
    assert_eq!(img.as_raw(), doc.composite().expect("valid").as_bytes());
}

#[test]
fn rejects_garbage_and_truncated_archives() {
    assert!(matches!(
        read_kgr(Cursor::new(b"definitely not a zip".to_vec())),
        Err(FileError::Zip(_))
    ));
    let mut bytes = kgr_bytes(&sample_document());
    bytes.truncate(bytes.len() / 2);
    assert!(read_kgr(Cursor::new(bytes)).is_err());
    assert!(read_kgr(Cursor::new(Vec::new())).is_err());
}

#[test]
fn rejects_missing_entries() {
    let no_manifest = zip_with(&[("document.json", b"{}")]);
    let err = read_kgr(Cursor::new(no_manifest)).expect_err("missing manifest");
    assert!(err.to_string().contains("manifest.json"), "{err}");

    let doc_json = br#"{"width":2,"height":2,"layers":[{"name":"a","visible":true,"file":"layers/0000.png"}]}"#;
    let missing_layer = zip_with(&[("manifest.json", MANIFEST), ("document.json", doc_json)]);
    let err = read_kgr(Cursor::new(missing_layer)).expect_err("missing layer");
    assert!(err.to_string().contains("layers/0000.png"), "{err}");
}

#[test]
fn rejects_newer_and_foreign_formats() {
    let newer = zip_with(&[(
        "manifest.json",
        br#"{"format":"kagero-project","format_version":99}"#,
    )]);
    assert!(matches!(
        read_kgr(Cursor::new(newer)),
        Err(FileError::UnsupportedVersion { found: 99, .. })
    ));
    let foreign = zip_with(&[("manifest.json", br#"{"format":"other","format_version":1}"#)]);
    assert!(matches!(
        read_kgr(Cursor::new(foreign)),
        Err(FileError::Invalid(_))
    ));
    let broken_json = zip_with(&[("manifest.json", b"{not json")]);
    assert!(matches!(
        read_kgr(Cursor::new(broken_json)),
        Err(FileError::Json(_))
    ));
}

#[test]
fn rejects_bad_sizes_before_decoding_layers() {
    for json in [
        r#"{"width":0,"height":4,"layers":[]}"#.to_owned(),
        r#"{"width":100000,"height":4,"layers":[]}"#.to_owned(),
        r#"{"width":4,"height":4,"layers":[]}"#.to_owned(),
        // 40 layers at 2048×2048 would need 640 MiB.
        format!(
            r#"{{"width":2048,"height":2048,"layers":[{}]}}"#,
            (0..40)
                .map(|i| format!(r#"{{"name":"l","visible":true,"file":"layers/{i:04}.png"}}"#))
                .collect::<Vec<_>>()
                .join(",")
        ),
    ] {
        let archive = zip_with(&[
            ("manifest.json", MANIFEST),
            ("document.json", json.as_bytes()),
        ]);
        assert!(read_kgr(Cursor::new(archive)).is_err(), "accepted {json}");
    }
}

#[test]
fn rejects_layer_size_mismatch_and_odd_paths() {
    let doc_json = br#"{"width":4,"height":4,"layers":[{"name":"a","visible":true,"file":"layers/0000.png"}]}"#;
    let png = png_bytes(3, 4);
    let mismatch = zip_with(&[
        ("manifest.json", MANIFEST),
        ("document.json", doc_json),
        ("layers/0000.png", &png),
    ]);
    let err = read_kgr(Cursor::new(mismatch)).expect_err("size mismatch");
    assert!(err.to_string().contains("3×4"), "{err}");

    let traversal =
        br#"{"width":4,"height":4,"layers":[{"name":"a","visible":true,"file":"../../evil.png"}]}"#;
    let archive = zip_with(&[
        ("manifest.json", MANIFEST),
        ("document.json", traversal),
        ("../../evil.png", &png_bytes(4, 4)),
    ]);
    let err = read_kgr(Cursor::new(archive)).expect_err("path traversal");
    assert!(matches!(err, FileError::Invalid(_)), "{err}");
}

#[test]
fn rejects_corrupted_layer_png() {
    let doc_json = br#"{"width":4,"height":4,"layers":[{"name":"a","visible":true,"file":"layers/0000.png"}]}"#;
    let mut png = png_bytes(4, 4);
    let mid = png.len() / 2;
    png.truncate(mid);
    let archive = zip_with(&[
        ("manifest.json", MANIFEST),
        ("document.json", doc_json),
        ("layers/0000.png", &png),
    ]);
    assert!(read_kgr(Cursor::new(archive)).is_err());
}

#[test]
fn bad_active_layer_and_long_names_are_handled() {
    let png = png_bytes(2, 2);
    let doc_json = br#"{"width":2,"height":2,"active_layer":5,"layers":[{"name":"a","visible":true,"file":"layers/0000.png"}]}"#;
    let archive = zip_with(&[
        ("manifest.json", MANIFEST),
        ("document.json", doc_json),
        ("layers/0000.png", &png),
    ]);
    assert!(read_kgr(Cursor::new(archive)).is_err());

    let long = "x".repeat(500);
    let doc_json = format!(
        r#"{{"width":2,"height":2,"layers":[{{"name":"{long}","visible":true,"file":"layers/0000.png"}}]}}"#
    );
    let archive = zip_with(&[
        ("manifest.json", MANIFEST),
        ("document.json", doc_json.as_bytes()),
        ("layers/0000.png", &png),
    ]);
    let doc = read_kgr(Cursor::new(archive)).expect("long names are trimmed");
    assert_eq!(
        doc.layers()[0].name.chars().count(),
        kagero::core::MAX_LAYER_NAME_CHARS
    );
}

#[test]
fn png_import_rejects_oversized_and_invalid_images() {
    let dir = temp_dir("import");
    let big = dir.join("big.png");
    let img = image::RgbaImage::new(kagero::core::MAX_CANVAS_SIZE + 1, 1);
    img.save(&big).expect("write big png");
    assert!(matches!(import_png(&big), Err(FileError::TooLarge(_))));

    let fake = dir.join("fake.png");
    std::fs::write(&fake, b"not a png").expect("write");
    assert!(import_png(&fake).is_err());

    assert!(matches!(
        import_png(&dir.join("missing.png")),
        Err(FileError::Io(_))
    ));
}

// ---------------------------------------------------------------- format v2 / animation

/// Extra archive entries for a test case.
type Entries<'a> = Vec<(&'a str, &'a [u8])>;

const MANIFEST_V2: &[u8] = br#"{"format":"kagero-project","format_version":2}"#;

fn animated_document() -> Document {
    let doc = Document::new(4, 3, Rgba::TRANSPARENT).expect("valid");
    let mut e = Editor::new(doc, None, "Anim".into());
    e.add_layer().expect("layer");
    e.primary = Rgba::opaque(255, 0, 0);
    e.begin_stroke((0, 0), ColorSlot::Primary).expect("draw");
    e.end_stroke();
    e.set_frame_duration(0, 80).expect("duration");
    e.duplicate_frame().expect("dup");
    e.primary = Rgba::new(0, 0, 255, 100);
    e.begin_stroke((3, 2), ColorSlot::Primary).expect("draw");
    e.end_stroke();
    e.add_frame().expect("frame");
    e.set_frame_duration(2, 333).expect("duration");
    e.toggle_layer_visibility(0).expect("hide");
    // Delete and re-add to make ids non-sequential.
    e.add_frame().expect("frame");
    e.delete_frame(1).expect("delete");
    e.select_frame(1);
    e.document().clone()
}

#[test]
fn multi_frame_round_trip_keeps_everything() {
    let doc = animated_document();
    assert_eq!(doc.frame_count(), 3);
    let loaded = read_kgr(Cursor::new(kgr_bytes(&doc))).expect("read");
    assert_same(&doc, &loaded);
    assert_eq!(loaded.frames()[1].duration_ms(), 333);
    // Frames are not merged into one image.
    assert_ne!(loaded.frames()[0].cels(), loaded.frames()[1].cels());
}

#[test]
fn multi_frame_round_trip_on_disk() {
    let dir = temp_dir("anim-roundtrip");
    let path = dir.join("walk.kgr");
    let doc = animated_document();
    save_kgr(&doc, &path).expect("save");
    let loaded = load_kgr(&path).expect("load");
    assert_same(&doc, &loaded);
    // Saving the loaded document again produces an identical project.
    let again = read_kgr(Cursor::new(kgr_bytes(&loaded))).expect("read");
    assert_same(&loaded, &again);
}

#[test]
fn kagero_0_1_projects_migrate_to_one_frame() {
    let red = PixelBuffer::filled(2, 2, Rgba::opaque(255, 0, 0)).expect("valid");
    let mut ink = PixelBuffer::new(2, 2).expect("valid");
    ink.set(1, 1, Rgba::new(0, 0, 0, 77));
    let mut png_a = Vec::new();
    let mut png_b = Vec::new();
    file_io::png::encode_png(&red, &mut png_a).expect("encode");
    file_io::png::encode_png(&ink, &mut png_b).expect("encode");
    let doc_json = br#"{"width":2,"height":2,"active_layer":1,"layers":[
        {"name":"Background","visible":false,"file":"layers/0000.png"},
        {"name":"Ink","visible":true,"file":"layers/0001.png"}]}"#;
    let archive = zip_with(&[
        ("manifest.json", MANIFEST),
        ("document.json", doc_json),
        ("layers/0000.png", &png_a),
        ("layers/0001.png", &png_b),
    ]);
    let doc = read_kgr(Cursor::new(archive)).expect("v1 opens");
    assert_eq!(doc.frame_count(), 1);
    assert_eq!(doc.layer_count(), 2);
    assert_eq!(doc.active_index(), 1);
    assert_eq!(doc.layers()[0].name, "Background");
    assert!(!doc.layers()[0].visible);
    assert_eq!(doc.cel(0, 0), Some(&red));
    assert_eq!(doc.cel(0, 1), Some(&ink));

    // Saving writes format v2, which opens again identically.
    let resaved = read_kgr(Cursor::new(kgr_bytes(&doc))).expect("v2 opens");
    assert_same(&doc, &resaved);
}

#[test]
fn rejects_damaged_v2_documents() {
    let png = png_bytes(2, 2);
    let small = png_bytes(1, 2);
    let good_doc = r#"{"width":2,"height":2,"layers":[{"id":1,"name":"a","visible":true}],
        "frames":[{"id":1,"duration_ms":100,"cels":["frames/0000/0000.png"]}]}"#;
    let ok = zip_with(&[
        ("manifest.json", MANIFEST_V2),
        ("document.json", good_doc.as_bytes()),
        ("frames/0000/0000.png", &png),
    ]);
    read_kgr(Cursor::new(ok)).expect("the valid baseline opens");

    let cases: Vec<(&str, String, Entries)> = vec![
        ("missing cel", good_doc.to_owned(), vec![]),
        (
            "wrong cel size",
            good_doc.to_owned(),
            vec![("frames/0000/0000.png", &small)],
        ),
        (
            "odd path",
            good_doc.replace("frames/0000/0000.png", "../evil.png"),
            vec![("../evil.png", &png)],
        ),
        (
            "cel count",
            good_doc.replace(r#"["frames/0000/0000.png"]"#, "[]"),
            vec![],
        ),
        (
            "no frames",
            good_doc.replace(
                r#"[{"id":1,"duration_ms":100,"cels":["frames/0000/0000.png"]}]"#,
                "[]",
            ),
            vec![],
        ),
        (
            "active frame",
            good_doc.replace(r#""width":2"#, r#""active_frame":3,"width":2"#),
            vec![("frames/0000/0000.png", &png)],
        ),
        (
            "zero id",
            good_doc.replace(r#"{"id":1,"duration_ms"#, r#"{"id":0,"duration_ms"#),
            vec![("frames/0000/0000.png", &png)],
        ),
    ];
    for (what, json, extra) in cases {
        let mut entries: Vec<(&str, &[u8])> = vec![
            ("manifest.json", MANIFEST_V2),
            ("document.json", json.as_bytes()),
        ];
        entries.extend(extra);
        let archive = zip_with(&entries);
        assert!(read_kgr(Cursor::new(archive)).is_err(), "accepted: {what}");
    }
}

#[test]
fn rejects_animations_over_the_memory_budget_before_decoding() {
    // 40 frames at 2048×2048 would need 640 MiB.
    let frames: Vec<String> = (0..40)
        .map(|i| {
            format!(
                r#"{{"id":{},"duration_ms":100,"cels":["frames/{i:04}/0000.png"]}}"#,
                i + 1
            )
        })
        .collect();
    let json = format!(
        r#"{{"width":2048,"height":2048,"layers":[{{"id":1,"name":"a","visible":true}}],"frames":[{}]}}"#,
        frames.join(",")
    );
    let archive = zip_with(&[
        ("manifest.json", MANIFEST_V2),
        ("document.json", json.as_bytes()),
    ]);
    assert!(matches!(
        read_kgr(Cursor::new(archive)),
        Err(FileError::TooLarge(_))
    ));
}

#[test]
fn rejects_future_format_versions() {
    let newer = zip_with(&[
        (
            "manifest.json",
            br#"{"format":"kagero-project","format_version":3}"#,
        ),
        ("document.json", b"{}"),
    ]);
    assert!(matches!(
        read_kgr(Cursor::new(newer)),
        Err(FileError::UnsupportedVersion {
            found: 3,
            supported: 2
        })
    ));
}

#[test]
fn export_current_and_all_frames() {
    let dir = temp_dir("export-frames");
    let doc = animated_document();
    let current = dir.join("current.png");
    export_png(&doc, &current).expect("export current");
    let img = image::open(&current).expect("decodes").into_rgba8();
    assert_eq!(img.as_raw(), doc.composite().expect("valid").as_bytes());

    let paths = file_io::export_frames(&doc, &dir, "walk: cycle").expect("export all");
    assert_eq!(paths.len(), 3);
    assert_eq!(
        paths[0].file_name().and_then(|n| n.to_str()),
        Some("walk_ cycle_0001.png"),
        "unsafe characters are replaced"
    );
    for (i, path) in paths.iter().enumerate() {
        let img = image::open(path).expect("decodes").into_rgba8();
        assert_eq!(
            img.as_raw(),
            doc.composite_frame(i).expect("valid").as_bytes()
        );
    }
    assert_eq!(
        file_io::frame_export_paths(&doc, &dir, "walk: cycle"),
        paths
    );
}

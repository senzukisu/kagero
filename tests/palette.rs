//! Palette files, built-in palettes and palette settings.

use std::path::PathBuf;

use kagero::config::{Config, MAX_PALETTES, MAX_RECENT_COLORS};
use kagero::core::palette::{self, Palette, MAX_PALETTE_COLORS};
use kagero::core::Rgba;
use kagero::file_io::palette::{load_palette, parse_gpl, parse_hex, save_palette, to_gpl, to_hex};
use kagero::file_io::FileError;

/// A fresh, empty directory for one test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("kagero-tests")
        .join(format!("palette-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sample() -> Palette {
    Palette {
        name: "Test".into(),
        colors: vec![
            Rgba::opaque(255, 0, 0),
            Rgba::opaque(0, 128, 255),
            Rgba::new(10, 20, 30, 128),
        ],
    }
}

#[test]
fn gpl_parses_names_comments_and_colors() {
    let text = "GIMP Palette\nName: Sunset\nColumns: 4\n# a comment\n\n255   0   0\tRed\n  0 255 128 Mint green\n";
    let p = parse_gpl(text, "fallback").expect("valid gpl");
    assert_eq!(p.name, "Sunset");
    assert_eq!(
        p.colors,
        vec![Rgba::opaque(255, 0, 0), Rgba::opaque(0, 255, 128)]
    );

    let p = parse_gpl("\u{feff}GIMP Palette\n1 2 3\n", "From File").expect("BOM is fine");
    assert_eq!(p.name, "From File");
}

#[test]
fn gpl_rejects_bad_input() {
    assert!(matches!(
        parse_gpl("1 2 3\n", "x"),
        Err(FileError::Invalid(_))
    ));
    assert!(matches!(
        parse_gpl("GIMP Palette\n256 0 0\n", "x"),
        Err(FileError::Invalid(_))
    ));
    assert!(matches!(
        parse_gpl("GIMP Palette\n1 2\n", "x"),
        Err(FileError::Invalid(_))
    ));
    assert!(matches!(
        parse_gpl("GIMP Palette\n# only comments\n", "x"),
        Err(FileError::Invalid(_))
    ));
    let many: String = std::iter::once("GIMP Palette\n".to_owned())
        .chain((0..=MAX_PALETTE_COLORS).map(|i| format!("{} 0 0\n", i % 256)))
        .collect();
    assert!(matches!(parse_gpl(&many, "x"), Err(FileError::TooLarge(_))));
}

#[test]
fn hex_parses_rgb_and_argb() {
    let p = parse_hex(
        "; comment\n// another\n#ff8000\n00ff00\n80102030\n\n",
        "Hex",
    )
    .expect("valid");
    assert_eq!(
        p.colors,
        vec![
            Rgba::opaque(255, 128, 0),
            Rgba::opaque(0, 255, 0),
            Rgba::new(0x10, 0x20, 0x30, 0x80),
        ]
    );
    assert!(
        parse_hex("fff\n", "x").is_err(),
        "3-digit hex is not accepted"
    );
    assert!(parse_hex("zzzzzz\n", "x").is_err());
    assert!(parse_hex("", "x").is_err(), "empty palette");
}

#[test]
fn text_round_trips() {
    let p = sample();
    let hex = parse_hex(&to_hex(&p), "Test").expect("hex round trip");
    assert_eq!(hex.colors, p.colors, "HEX keeps alpha");

    let gpl = parse_gpl(&to_gpl(&p), "x").expect("gpl round trip");
    assert_eq!(gpl.name, "Test");
    // GPL has no alpha channel.
    let opaque: Vec<Rgba> = p
        .colors
        .iter()
        .map(|c| Rgba::opaque(c.r(), c.g(), c.b()))
        .collect();
    assert_eq!(gpl.colors, opaque);
}

#[test]
fn files_save_and_load_by_extension() {
    let dir = temp_dir("files");
    let p = sample();
    for name in ["test.gpl", "test.hex", "TEST.GPL"] {
        let path = dir.join(name);
        save_palette(&p, &path).expect("save");
        let loaded = load_palette(&path).expect("load");
        assert_eq!(loaded.colors.len(), 3, "{name}");
    }
    let text = std::fs::read_to_string(dir.join("test.gpl")).unwrap();
    assert!(text.starts_with("GIMP Palette"));

    // Oversized files are refused before reading.
    let big = dir.join("big.hex");
    std::fs::write(&big, vec![b'a'; 2 * 1024 * 1024]).unwrap();
    assert!(matches!(load_palette(&big), Err(FileError::TooLarge(_))));
    // Non-UTF-8 text is an error, not a panic.
    let bad = dir.join("bad.hex");
    std::fs::write(&bad, [0xff, 0xfe, 0x00]).unwrap();
    assert!(load_palette(&bad).is_err());
}

#[test]
fn builtins_are_present_and_valid() {
    let names: Vec<String> = palette::builtins().into_iter().map(|p| p.name).collect();
    for wanted in ["Ember", "Night Sakura", "Misty Forest"] {
        assert!(names.iter().any(|n| n == wanted), "missing {wanted}");
        assert!(palette::is_builtin_name(wanted));
    }
    for p in palette::builtins() {
        assert!(
            !p.colors.is_empty() && p.colors.len() <= MAX_PALETTE_COLORS,
            "{}",
            p.name
        );
        assert!(
            p.colors.iter().all(|c| c.a() == 255),
            "{} has translucent colors",
            p.name
        );
    }
}

#[test]
fn palette_edits_keep_settings_valid() {
    let mut cfg = Config::default();
    let start = cfg.palettes.len();

    // New palettes get unique names and become active.
    assert!(cfg.add_palette(sample()));
    assert!(cfg.add_palette(sample()));
    assert_eq!(cfg.palettes.len(), start + 2);
    assert_eq!(cfg.active_palette().name, "Test 2");

    // Add (no duplicates), move, remove (the last color stays).
    assert!(!cfg.add_palette_color(Rgba::opaque(255, 0, 0)));
    assert!(cfg.add_palette_color(Rgba::BLACK));
    assert_eq!(cfg.active_palette().colors.len(), 4);
    assert!(cfg.move_palette_color(3, -3));
    assert_eq!(cfg.active_palette().colors[0], Rgba::BLACK);
    assert!(!cfg.move_palette_color(0, -1), "can't move past the start");
    for _ in 0..3 {
        assert!(cfg.remove_palette_color(0));
    }
    assert!(!cfg.remove_palette_color(0), "a palette keeps one color");

    // Delete until one is left.
    while cfg.palettes.len() > 1 {
        assert!(cfg.delete_active_palette());
        assert!(cfg.active_palette < cfg.palettes.len());
    }
    assert!(!cfg.delete_active_palette());

    // Restoring brings the built-ins back without dropping the custom one.
    cfg.restore_builtin_palettes();
    for b in palette::builtins() {
        assert!(cfg.palettes.iter().any(|p| p.name == b.name));
    }
}

#[test]
fn palette_count_is_limited() {
    let mut cfg = Config::default();
    while cfg.palettes.len() < MAX_PALETTES {
        assert!(cfg.add_palette(sample()));
    }
    assert!(!cfg.add_palette(sample()));
}

#[test]
fn recent_colors_are_unique_and_bounded() {
    let mut cfg = Config::default();
    assert!(cfg.remember_color(Rgba::BLACK));
    assert!(
        !cfg.remember_color(Rgba::BLACK),
        "same color again changes nothing"
    );
    assert!(cfg.remember_color(Rgba::WHITE));
    assert!(cfg.remember_color(Rgba::BLACK));
    assert_eq!(cfg.recent_colors, vec![Rgba::BLACK, Rgba::WHITE]);
    for i in 0..40u8 {
        cfg.remember_color(Rgba::opaque(i, 0, 0));
    }
    assert_eq!(cfg.recent_colors.len(), MAX_RECENT_COLORS);
    assert_eq!(cfg.recent_colors[0], Rgba::opaque(39, 0, 0));
}

#[test]
fn legacy_single_palette_is_migrated() {
    let dir = temp_dir("legacy");
    let path = dir.join("config.json");
    // A Kagero 0.2 settings file with a customized palette.
    std::fs::write(&path, r#"{ "palette": [[1,2,3,255],[4,5,6,255]] }"#).unwrap();
    let cfg = Config::load(&path);
    assert_eq!(cfg.active_palette().name, "My Palette");
    assert_eq!(
        cfg.active_palette().colors,
        vec![Rgba::opaque(1, 2, 3), Rgba::opaque(4, 5, 6)]
    );
    assert!(
        cfg.palettes.iter().any(|p| p.name == "Ember"),
        "built-ins are still there"
    );

    // Saving no longer writes the legacy field, and loading again doesn't duplicate.
    cfg.save(&path).expect("save");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("\"palette\":"));
    let again = Config::load(&path);
    assert_eq!(again.palettes, cfg.palettes);

    // An untouched default palette is not migrated.
    let default: Vec<[u8; 4]> = kagero::config::default_palette()
        .iter()
        .map(|c| c.0)
        .collect();
    std::fs::write(&path, serde_json::json!({ "palette": default }).to_string()).unwrap();
    let cfg = Config::load(&path);
    assert!(cfg.palettes.iter().all(|p| p.name != "My Palette"));
}

#[test]
fn damaged_palette_settings_are_repaired() {
    let dir = temp_dir("repair");
    let path = dir.join("config.json");
    std::fs::write(
        &path,
        r#"{ "palettes": [{ "name": "", "colors": [] }], "active_palette": 99, "recent_colors": [] }"#,
    )
    .unwrap();
    let cfg = Config::load(&path);
    assert!(!cfg.palettes.is_empty());
    assert!(cfg.active_palette < cfg.palettes.len());
    assert!(cfg
        .palettes
        .iter()
        .all(|p| !p.colors.is_empty() && !p.name.is_empty()));
}

//! Plugins v1: manifest checks, the sandbox, limits, the API and the example plugin.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use kagero::core::selection::{SelRect, Selection};
use kagero::core::Rgba;
use kagero::plugins::{
    self, load_plugin, parse_manifest, run, run_limited, Manifest, PluginInput, PluginStore,
    EXAMPLE_MANIFEST, EXAMPLE_SCRIPT,
};

fn manifest(commands: &[&str]) -> Manifest {
    let list: Vec<String> = commands
        .iter()
        .map(|c| format!(r#"{{ "id": "{c}", "title": "Run {c}" }}"#))
        .collect();
    parse_manifest(&format!(
        r#"{{ "format": "kagero-plugin", "api": 1, "id": "test.plugin", "name": "Test",
             "version": "0.1.0", "commands": [{}] }}"#,
        list.join(",")
    ))
    .expect("valid manifest")
}

/// 3×2 image: red, green, transparent / blue, white, half-black.
fn input() -> PluginInput {
    let px: Vec<u8> = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 0, 0],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
        [0, 0, 0, 128],
    ]
    .concat();
    PluginInput {
        width: 3,
        height: 2,
        pixels: px,
        selection: None,
        primary: Rgba::opaque(10, 20, 30),
        secondary: Rgba::WHITE,
    }
}

fn go(script: &str, command: &str, input: PluginInput) -> Result<plugins::PluginOutput, String> {
    run(
        script,
        &manifest(&[command]),
        command,
        input,
        &AtomicBool::new(false),
    )
}

#[test]
fn manifests_are_checked() {
    let ok = r#"{ "format": "kagero-plugin", "api": 1, "id": "a.b", "name": "A",
                 "version": "1.2.3", "commands": [ { "id": "go", "title": "Go" } ] }"#;
    assert!(parse_manifest(ok).is_ok());
    let bad = |from: &str, to: &str| parse_manifest(&ok.replace(from, to)).unwrap_err();
    assert!(bad("\"api\": 1", "\"api\": 2").contains("update Kagero"));
    assert!(bad("\"api\": 1", "\"api\": 0").contains("api"));
    assert!(bad("kagero-plugin", "other").contains("format"));
    assert!(bad("\"a.b\"", "\"Bad Id!\"").contains("id"));
    assert!(bad("1.2.3", "one").contains("version"));
    assert!(bad("\"name\": \"A\"", "\"name\": \"  \"").contains("name"));
    assert!(bad(r#"[ { "id": "go", "title": "Go" } ]"#, "[]").contains("commands"));
    assert!(bad(
        r#"[ { "id": "go", "title": "Go" } ]"#,
        r#"[ { "id": "go", "title": "Go" }, { "id": "go", "title": "Again" } ]"#
    )
    .contains("twice"));
    assert!(parse_manifest("{").is_err());
}

#[test]
fn the_api_reads_and_writes_pixels() {
    let script = r#"
        local c = {}
        function c.go()
          assert(kagero.api_version == 1)
          assert(kagero.width == 3 and kagero.height == 2)
          local r, g, b, a = kagero.get_pixel(0, 0)
          assert(r == 255 and g == 0 and b == 0 and a == 255)
          assert(kagero.get_pixel(5, 5) == nil, "outside is nil")
          assert(kagero.selection() == nil)
          kagero.set_pixel(2, 0, 1, 2, 3)          -- alpha defaults to 255
          kagero.set_pixel(1, 1, 300, -5, 7.6, 9)  -- values are clamped and rounded
          assert(kagero.set_pixel(9, 9, 0, 0, 0) == false)
          local pr = kagero.primary()
          kagero.set_primary(pr + 1, 0, 0)
          print("hello", 42)
        end
        return c
    "#;
    let out = go(script, "go", input()).expect("runs");
    let px = out.pixels.expect("changed");
    assert_eq!(&px[8..12], &[1, 2, 3, 255]);
    assert_eq!(&px[16..20], &[255, 0, 8, 9]);
    assert_eq!(&px[0..4], &[255, 0, 0, 255], "untouched");
    assert_eq!(out.primary, Some(Rgba::opaque(11, 0, 0)));
    assert_eq!(out.messages, ["hello 42"]);
    // A command that changes nothing returns no pixels.
    let out = go("return { go = function() end }", "go", input()).expect("runs");
    assert_eq!(out.pixels, None);
}

#[test]
fn writes_outside_the_selection_are_ignored() {
    let mut inp = input();
    inp.selection = Some(Selection::from(SelRect {
        x: 0,
        y: 0,
        w: 1,
        h: 2,
    }));
    let script = r#"return { go = function()
        local x, y, w, h = kagero.selection()
        assert(x == 0 and y == 0 and w == 1 and h == 2)
        assert(kagero.is_selected(0, 1) and not kagero.is_selected(1, 0))
        for y = 0, 1 do for x = 0, 2 do kagero.set_pixel(x, y, 9, 9, 9) end end
    end }"#;
    let px = go(script, "go", inp)
        .expect("runs")
        .pixels
        .expect("changed");
    assert_eq!(&px[0..4], &[9, 9, 9, 255]);
    assert_eq!(&px[4..8], &[0, 255, 0, 255], "outside the selection");
}

#[test]
fn the_sandbox_has_no_files_programs_or_code_loading() {
    for (code, what) in [
        ("io.open('x', 'w')", "io"),
        ("os.execute('calc')", "os"),
        ("require('socket')", "require"),
        ("load('return 1')()", "load"),
        ("dofile('x.lua')", "dofile"),
        ("string.dump(print)", "dump"),
        ("debug.getinfo(1)", "debug"),
        ("setmetatable({}, {})", "setmetatable"),
    ] {
        let script = format!("return {{ go = function() {code} end }}");
        let err = go(&script, "go", input()).expect_err(what);
        assert!(err.contains("nil"), "{what}: {err}");
    }
    // Bytecode is refused even at load time.
    let err = go("\x1bLua", "go", input()).expect_err("bytecode");
    assert!(!err.is_empty());
}

#[test]
fn runaway_scripts_are_stopped() {
    let m = manifest(&["go"]);
    let start = std::time::Instant::now();
    let err = run_limited(
        "return { go = function() while true do end end }",
        &m,
        "go",
        input(),
        &AtomicBool::new(false),
        Duration::from_millis(300),
    )
    .expect_err("stopped");
    assert!(err.contains("longer than"), "{err}");
    assert!(start.elapsed() < Duration::from_secs(5));
    // Too much memory.
    let err = go(
        "return { go = function() local t = {} for i = 1, 1e9 do t[i] = ('x'):rep(64) .. i end end }",
        "go",
        input(),
    )
    .expect_err("memory");
    assert!(err.to_lowercase().contains("memory"), "{err}");
    // Canceling from another thread.
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let err = run(
        "return { go = function() while true do end end }",
        &m,
        "go",
        input(),
        &cancel,
    )
    .expect_err("canceled");
    assert!(err.contains("canceled"), "{err}");
}

#[test]
fn script_mistakes_have_clear_errors() {
    assert!(go("return 5", "go", input())
        .unwrap_err()
        .contains("must end with"));
    assert!(go("return {}", "go", input())
        .unwrap_err()
        .contains("\"go\""));
    let err = go(
        "return { go = function() error('boom') end }",
        "go",
        input(),
    )
    .unwrap_err();
    assert!(
        err.contains("boom") && err.contains("test.plugin/main.lua"),
        "{err}"
    );
    let m = manifest(&["go"]);
    let err = run("return {}", &m, "other", input(), &AtomicBool::new(false)).unwrap_err();
    assert!(err.contains("no command"));
}

#[test]
fn the_example_plugin_works() {
    let m = parse_manifest(EXAMPLE_MANIFEST).expect("valid");
    let cancel = AtomicBool::new(false);
    let out = run(EXAMPLE_SCRIPT, &m, "invert", input(), &cancel).expect("invert");
    let px = out.pixels.expect("changed");
    assert_eq!(&px[0..4], &[0, 255, 255, 255]);
    assert_eq!(&px[8..12], &[0, 0, 0, 0], "transparent pixels stay");
    assert_eq!(&px[20..24], &[255, 255, 255, 128], "alpha is kept");
    assert_eq!(out.messages, ["Inverted 5 pixels"]);
    let px = run(EXAMPLE_SCRIPT, &m, "grayscale", input(), &cancel)
        .expect("gray")
        .pixels
        .expect("changed");
    assert_eq!(&px[0..4], &[76, 76, 76, 255]);
    // Outline paints the transparent pixel next to colored ones with the primary color.
    let px = run(EXAMPLE_SCRIPT, &m, "outline", input(), &cancel)
        .expect("outline")
        .pixels
        .expect("changed");
    assert_eq!(&px[8..12], &[10, 20, 30, 255]);
}

#[test]
fn the_plugins_folder_lists_good_and_broken_plugins() {
    let dir = std::env::temp_dir().join(format!("kagero-plugins-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut store = PluginStore::load(Some(dir.clone()));
    assert!(store.plugins.is_empty());
    let example = store.install_example().expect("install");
    assert!(example.join("main.lua").exists());
    assert!(store.find("kagero.example-tools").is_some());

    let broken = dir.join("broken");
    std::fs::create_dir_all(&broken).expect("dir");
    std::fs::write(
        broken.join("plugin.json"),
        EXAMPLE_MANIFEST.replace("kagero.example-tools", "x.broken"),
    )
    .expect("w");
    std::fs::write(broken.join("main.lua"), "return {").expect("w");
    let missing = dir.join("no-manifest");
    std::fs::create_dir_all(&missing).expect("dir");
    let twin = dir.join("twin");
    std::fs::create_dir_all(&twin).expect("dir");
    std::fs::write(twin.join("plugin.json"), EXAMPLE_MANIFEST).expect("w");
    std::fs::write(twin.join("main.lua"), EXAMPLE_SCRIPT).expect("w");
    store.reload();
    assert_eq!(store.plugins.len(), 4);
    let problem = |name: &str| {
        store
            .plugins
            .iter()
            .find(|p| p.dir.ends_with(name))
            .and_then(|p| p.problem.clone())
    };
    assert!(
        problem("broken").is_some_and(|e| e.contains("main.lua")),
        "syntax error found"
    );
    assert!(problem("no-manifest").is_some_and(|e| e.contains("plugin.json")));
    assert!(
        problem("twin").is_some_and(|e| e.contains("already")),
        "duplicate id"
    );
    assert!(load_plugin(&example).is_valid());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The bundled plugins in `examples/plugins` (published with each release).
mod bundled {
    use super::*;

    const CLEANUP: (&str, &str) = (
        include_str!("../examples/plugins/kagero.pixel-cleanup/plugin.json"),
        include_str!("../examples/plugins/kagero.pixel-cleanup/main.lua"),
    );
    const COLORS: (&str, &str) = (
        include_str!("../examples/plugins/kagero.color-tools/plugin.json"),
        include_str!("../examples/plugins/kagero.color-tools/main.lua"),
    );

    const R: [u8; 4] = [200, 40, 40, 255];
    const E: [u8; 4] = [0, 0, 0, 0];

    /// 5×5: a 2×2 red block at (1,1), a stray red pixel at (4,4), a soft pixel at (0,4).
    fn image() -> PluginInput {
        let mut px = [E; 25];
        for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2), (4, 4)] {
            px[y * 5 + x] = R;
        }
        px[4 * 5] = [10, 10, 10, 200];
        PluginInput {
            width: 5,
            height: 5,
            pixels: px.concat(),
            selection: None,
            primary: Rgba::opaque(1, 2, 3),
            secondary: Rgba::opaque(9, 9, 9),
        }
    }

    fn at(px: &[u8], x: usize, y: usize) -> [u8; 4] {
        let i = (y * 5 + x) * 4;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    }

    fn run_cmd(plugin: (&str, &str), command: &str, input: PluginInput) -> plugins::PluginOutput {
        let m = parse_manifest(plugin.0).expect("valid manifest");
        assert!(m.commands.iter().any(|c| c.id == command), "{command}");
        run(plugin.1, &m, command, input, &AtomicBool::new(false)).expect(command)
    }

    #[test]
    fn manifests_are_valid_and_every_command_runs() {
        for plugin in [CLEANUP, COLORS] {
            let m = parse_manifest(plugin.0).expect("valid manifest");
            for c in &m.commands {
                run(plugin.1, &m, &c.id, image(), &AtomicBool::new(false))
                    .unwrap_or_else(|e| panic!("{}: {e}", c.id));
            }
        }
    }

    #[test]
    fn pixel_cleanup() {
        let out = run_cmd(CLEANUP, "remove-strays", image());
        let px = out.pixels.expect("changed");
        assert_eq!(at(&px, 4, 4), E, "the stray pixel is removed");
        assert_eq!(at(&px, 1, 1), R, "the block stays");
        assert_eq!(out.messages, ["Removed 2 stray pixels"]);

        let px = run_cmd(CLEANUP, "harden-edges", image())
            .pixels
            .expect("changed");
        assert_eq!(at(&px, 0, 4), [10, 10, 10, 255]);

        let px = run_cmd(CLEANUP, "drop-shadow", image())
            .pixels
            .expect("changed");
        assert_eq!(at(&px, 3, 3), [9, 9, 9, 255], "below-right of the block");
        assert_eq!(at(&px, 2, 2), R, "colored pixels are not covered");
        assert_eq!(at(&px, 0, 0), E);

        let px = run_cmd(CLEANUP, "inner-outline", image())
            .pixels
            .expect("changed");
        assert_eq!(at(&px, 1, 1), [1, 2, 3, 255], "a 2×2 block is all edge");
        assert_eq!(at(&px, 0, 0), E, "the shape doesn't grow");
    }

    #[test]
    fn color_tools() {
        let mut input = image();
        input.primary = Rgba::new(R[0], R[1], R[2], R[3]);
        let px = run_cmd(COLORS, "replace-color", input)
            .pixels
            .expect("changed");
        assert_eq!(at(&px, 1, 1), [9, 9, 9, 255]);
        assert_eq!(at(&px, 0, 4), [10, 10, 10, 200], "other colors stay");

        // Red shifted by +30° is orange, and back again is red.
        let px = run_cmd(COLORS, "hue-forward", image())
            .pixels
            .expect("changed");
        assert_eq!(at(&px, 1, 1), [200, 120, 40, 255]);
        let mut again = image();
        again.pixels = px;
        let px = run_cmd(COLORS, "hue-back", again).pixels.expect("changed");
        assert_eq!(at(&px, 1, 1), R);
        assert_eq!(at(&px, 0, 0), E, "transparent pixels stay");

        let px = run_cmd(COLORS, "lighten", image()).pixels.expect("changed");
        assert_eq!(at(&px, 1, 1), [214, 94, 94, 255]);
        let px = run_cmd(COLORS, "darken", image()).pixels.expect("changed");
        assert_eq!(at(&px, 1, 1), [150, 30, 30, 255]);

        let out = run_cmd(COLORS, "most-used", image());
        assert!(out.pixels.is_none(), "picking changes no pixels");
        assert_eq!(out.primary, Some(Rgba::new(200, 40, 40, 255)));
        assert_eq!(out.messages, ["Primary is now #C82828 (5 pixels)"]);
    }
}

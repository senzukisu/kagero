//! Plugins v1: small Lua scripts that change the active layer.
//!
//! **Why Lua (and not Rust):** a Rust plugin would be a native DLL. It would run
//! with Kagero's full rights (files, network, other programs), could crash Kagero,
//! and would have to match Kagero's compiler version exactly, since Rust has no
//! stable ABI. Lua 5.4 (embedded, ~300 KB) is made for this. Kagero opens only a
//! few safe parts of it, limits its memory and running time, and shares nothing but
//! the pixels of one image. Writing a plugin needs only a text editor.
//!
//! **A plugin** is a folder in `%APPDATA%\OnibiLab\Kagero\plugins` with:
//!
//! * `plugin.json`: the manifest (format `kagero-plugin`, `api` 1, id, name,
//!   version, commands);
//! * `main.lua`: a script that returns a table of functions, one per command.
//!
//! **Sandbox:** only Lua's `table`, `string`, `math` and `utf8` libraries plus a
//! few base functions. There is no `io`, `os`, `package`/`require`, `debug`,
//! `load`/`dofile`, and no bytecode. Memory is capped at 64 MiB and a run is
//! stopped after [`TIME_LIMIT`]. Scripts run on a background thread with a copy
//! of the active layer. The result becomes one undo step, and only if the image
//! didn't change meanwhile.
//!
//! The API is documented in `docs/plugins/README.md`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Value, VmState};
use serde::Deserialize;

use crate::core::selection::Selection;
use crate::core::Rgba;

pub const PLUGIN_FORMAT: &str = "kagero-plugin";
/// The plugin API this Kagero provides.
pub const API_VERSION: u32 = 1;
/// Longest a command may run.
pub const TIME_LIMIT: Duration = Duration::from_secs(20);
/// Most memory a command may use.
pub const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_SCRIPT_BYTES: u64 = 1024 * 1024;
const MAX_COMMANDS: usize = 32;
const MAX_PLUGINS: usize = 100;
const MAX_MESSAGES: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub api: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub commands: Vec<PluginCommand>,
}

/// A plugin folder and whether it can be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub dir: PathBuf,
    /// The manifest, if it could be read.
    pub manifest: Option<Manifest>,
    /// Why the plugin can't be used, if it can't.
    pub problem: Option<String>,
}

impl Plugin {
    pub fn id(&self) -> String {
        self.manifest.as_ref().map_or_else(
            || {
                self.dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            },
            |m| m.id.clone(),
        )
    }

    pub fn is_valid(&self) -> bool {
        self.problem.is_none() && self.manifest.is_some()
    }
}

fn valid_id(id: &str) -> bool {
    let ok_len = (2..=64).contains(&id.len());
    let mut chars = id.chars();
    ok_len
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// Checks a manifest's text.
pub fn parse_manifest(json: &str) -> Result<Manifest, String> {
    let m: Manifest =
        serde_json::from_str(json).map_err(|e| format!("plugin.json is not valid: {e}"))?;
    if m.format != PLUGIN_FORMAT {
        return Err("plugin.json: \"format\" must be \"kagero-plugin\"".into());
    }
    if m.api > API_VERSION {
        return Err(format!(
            "needs plugin API {} (this Kagero provides {API_VERSION}) — update Kagero",
            m.api
        ));
    }
    if m.api == 0 {
        return Err("plugin.json: \"api\" must be 1".into());
    }
    if !valid_id(&m.id) {
        return Err(format!(
            "plugin.json: the id \"{}\" may use only a-z, 0-9, '.', '-', '_' (2–64 characters)",
            m.id
        ));
    }
    if m.name.trim().is_empty() || m.name.chars().count() > 48 {
        return Err("plugin.json: \"name\" must be 1–48 characters".into());
    }
    semver::Version::parse(&m.version)
        .map_err(|_| format!("plugin.json: \"{}\" is not a version like 1.0.0", m.version))?;
    if m.commands.is_empty() || m.commands.len() > MAX_COMMANDS {
        return Err(format!("plugin.json: needs 1–{MAX_COMMANDS} commands"));
    }
    for (i, c) in m.commands.iter().enumerate() {
        if !valid_id(&c.id) {
            return Err(format!(
                "plugin.json: the command id \"{}\" is not valid",
                c.id
            ));
        }
        if m.commands[..i].iter().any(|o| o.id == c.id) {
            return Err(format!(
                "plugin.json: the command \"{}\" is listed twice",
                c.id
            ));
        }
        if c.title.trim().is_empty() || c.title.chars().count() > 48 {
            return Err(format!(
                "plugin.json: the title of \"{}\" must be 1–48 characters",
                c.id
            ));
        }
    }
    Ok(m)
}

fn read_limited(path: &Path, limit: u64, what: &str) -> Result<String, String> {
    let len = std::fs::metadata(path)
        .map_err(|_| format!("{what} is missing"))?
        .len();
    if len > limit {
        return Err(format!("{what} is too large"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("{what} can't be read: {e}"))
}

/// A Lua state with only the safe parts.
fn sandbox() -> Result<Lua, String> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
        LuaOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    lua.set_memory_limit(MEMORY_LIMIT)
        .map_err(|e| e.to_string())?;
    let g = lua.globals();
    for name in [
        "dofile",
        "loadfile",
        "load",
        "require",
        "collectgarbage",
        "rawequal",
        "rawget",
        "rawset",
        "rawlen",
        "setmetatable",
        "getmetatable",
    ] {
        g.raw_remove(name).map_err(|e| e.to_string())?;
    }
    // string.dump would expose bytecode.
    if let Ok(string) = g.get::<mlua::Table>("string") {
        let _ = string.raw_remove("dump");
    }
    Ok(lua)
}

/// Loads `main.lua` without running it (syntax check).
fn check_script(source: &str, chunk_name: &str) -> Result<(), String> {
    let lua = sandbox()?;
    lua.load(source)
        .set_name(chunk_name)
        .set_mode(mlua::ChunkMode::Text)
        .into_function()
        .map(|_| ())
        .map_err(|e| short_error(&e))
}

fn short_error(e: &mlua::Error) -> String {
    let text = match e {
        mlua::Error::CallbackError { cause, .. } => return short_error(cause),
        other => other.to_string(),
    };
    let first = text.lines().next().unwrap_or_default();
    first.chars().take(300).collect()
}

/// Reads one plugin folder.
pub fn load_plugin(dir: &Path) -> Plugin {
    let mut plugin = Plugin {
        dir: dir.to_path_buf(),
        manifest: None,
        problem: None,
    };
    let manifest = read_limited(&dir.join("plugin.json"), MAX_MANIFEST_BYTES, "plugin.json")
        .and_then(|t| parse_manifest(&t));
    match manifest {
        Ok(m) => {
            let chunk = format!("={}/main.lua", m.id);
            plugin.problem = read_limited(&dir.join("main.lua"), MAX_SCRIPT_BYTES, "main.lua")
                .and_then(|src| check_script(&src, &chunk))
                .err();
            plugin.manifest = Some(m);
        }
        Err(e) => plugin.problem = Some(e),
    }
    plugin
}

/// The plugins folder.
#[derive(Debug, Default)]
pub struct PluginStore {
    dir: Option<PathBuf>,
    pub plugins: Vec<Plugin>,
}

impl PluginStore {
    pub fn load(dir: Option<PathBuf>) -> Self {
        let mut store = Self {
            dir,
            plugins: Vec::new(),
        };
        store.reload();
        store
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    pub fn reload(&mut self) {
        self.plugins.clear();
        let Some(dir) = &self.dir else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for d in dirs.into_iter().take(MAX_PLUGINS) {
            let mut p = load_plugin(&d);
            if p.is_valid() && self.plugins.iter().any(|o| o.id() == p.id()) {
                p.problem = Some(format!(
                    "another folder already has the plugin \"{}\"",
                    p.id()
                ));
            }
            self.plugins.push(p);
        }
    }

    pub fn find(&self, id: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.is_valid() && p.id() == id)
    }

    /// Writes the bundled example plugin into the plugins folder.
    pub fn install_example(&mut self) -> Result<PathBuf, String> {
        let dir = self.dir.clone().ok_or("there is no plugins folder")?;
        let target = dir.join(EXAMPLE_ID);
        std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        for (name, text) in [
            ("plugin.json", EXAMPLE_MANIFEST),
            ("main.lua", EXAMPLE_SCRIPT),
        ] {
            crate::file_io::write_atomic(&target.join(name), |w| {
                std::io::Write::write_all(w, text.as_bytes())?;
                Ok(())
            })
            .map_err(|e| e.to_string())?;
        }
        self.reload();
        Ok(target)
    }
}

pub const EXAMPLE_ID: &str = "kagero.example-tools";
pub const EXAMPLE_MANIFEST: &str =
    include_str!("../examples/plugins/kagero.example-tools/plugin.json");
pub const EXAMPLE_SCRIPT: &str = include_str!("../examples/plugins/kagero.example-tools/main.lua");

/// What a plugin command gets.
#[derive(Debug, Clone)]
pub struct PluginInput {
    pub width: u32,
    pub height: u32,
    /// RGBA of the active layer in the current frame.
    pub pixels: Vec<u8>,
    pub selection: Option<Selection>,
    pub primary: Rgba,
    pub secondary: Rgba,
}

/// What a finished command returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginOutput {
    /// The layer's new pixels, if the command changed any.
    pub pixels: Option<Vec<u8>>,
    pub primary: Option<Rgba>,
    pub messages: Vec<String>,
}

struct State {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    changed: bool,
    selection: Option<Selection>,
    primary: Rgba,
    secondary: Rgba,
    new_primary: Option<Rgba>,
    messages: Vec<String>,
}

impl State {
    fn index(&self, x: i64, y: i64) -> Option<usize> {
        (x >= 0 && y >= 0 && x < i64::from(self.width) && y < i64::from(self.height))
            .then(|| (y as usize * self.width as usize + x as usize) * 4)
    }

    fn selected(&self, x: i64, y: i64) -> bool {
        self.selection.as_ref().is_none_or(|s| {
            i32::try_from(x).is_ok_and(|x| i32::try_from(y).is_ok_and(|y| s.contains(x, y)))
        })
    }
}

fn channel(v: Option<f64>, default: u8) -> u8 {
    v.map_or(default, |v| {
        if v.is_nan() {
            0
        } else {
            v.round().clamp(0.0, 255.0) as u8
        }
    })
}

/// Runs one command of a plugin (blocking; [`PluginJob`] runs it in the background).
pub fn run(
    source: &str,
    manifest: &Manifest,
    command: &str,
    input: PluginInput,
    cancel: &AtomicBool,
) -> Result<PluginOutput, String> {
    run_limited(source, manifest, command, input, cancel, TIME_LIMIT)
}

/// [`run`] with a custom time limit.
pub fn run_limited(
    source: &str,
    manifest: &Manifest,
    command: &str,
    input: PluginInput,
    cancel: &AtomicBool,
    limit: Duration,
) -> Result<PluginOutput, String> {
    if !manifest.commands.iter().any(|c| c.id == command) {
        return Err(format!("the plugin has no command \"{command}\""));
    }
    let lua = sandbox()?;
    let started = Instant::now();
    let state = Rc::new(RefCell::new(State {
        width: input.width,
        height: input.height,
        pixels: input.pixels,
        changed: false,
        selection: input.selection,
        primary: input.primary,
        secondary: input.secondary,
        new_primary: None,
        messages: Vec::new(),
    }));
    // The hook can't borrow `cancel` (it must be 'static): it reads `stop`, which a
    // watcher thread sets when `cancel` is set while the command runs.
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(20_000),
            move |_, _| {
                if stop.load(Ordering::Relaxed) {
                    return Err(mlua::Error::runtime("canceled"));
                }
                if started.elapsed() > limit {
                    return Err(mlua::Error::runtime(format!(
                        "stopped: the command ran longer than {:.1} seconds",
                        limit.as_secs_f64()
                    )));
                }
                Ok(VmState::Continue)
            },
        );
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("canceled".into());
    }
    let api = build_api(&lua, &state).map_err(|e| short_error(&e))?;
    lua.globals()
        .set("kagero", api)
        .map_err(|e| short_error(&e))?;
    {
        // `print` goes to the status bar like kagero.message.
        let state = state.clone();
        let print = lua
            .create_function(move |_, args: mlua::Variadic<Value>| {
                let text: Vec<String> = args
                    .iter()
                    .map(|v| v.to_string().unwrap_or_else(|_| "?".into()))
                    .collect();
                push_message(&state, text.join(" "));
                Ok(())
            })
            .map_err(|e| short_error(&e))?;
        lua.globals()
            .set("print", print)
            .map_err(|e| short_error(&e))?;
    }
    let chunk = format!("={}/main.lua", manifest.id);
    let exports: Value = lua
        .load(source)
        .set_name(chunk.as_str())
        .set_mode(mlua::ChunkMode::Text)
        .eval()
        .map_err(|e| short_error(&e))?;
    let Value::Table(exports) = exports else {
        return Err("main.lua must end with `return { command_id = function() ... end }`".into());
    };
    let func: mlua::Function = exports
        .get(command)
        .map_err(|_| format!("main.lua doesn't return a function for the command \"{command}\""))?;
    // The flag may be set from another thread while the script runs.
    let watcher_stop = stop.clone();
    let cancel_flag: &AtomicBool = cancel;
    let done = AtomicBool::new(false);
    std::thread::scope(|s| {
        let done_ref = &done;
        s.spawn(move || {
            while !done_ref.load(Ordering::Relaxed) {
                if cancel_flag.load(Ordering::Relaxed) {
                    watcher_stop.store(true, Ordering::Relaxed);
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let result = func.call::<()>(()).map_err(|e| short_error(&e));
        done.store(true, Ordering::Relaxed);
        result
    })?;
    drop(exports);
    lua.remove_hook();
    // The API's functions (still owned by Lua) share the state: take the results out.
    let mut s = state.borrow_mut();
    Ok(PluginOutput {
        pixels: s.changed.then(|| std::mem::take(&mut s.pixels)),
        primary: s.new_primary,
        messages: std::mem::take(&mut s.messages),
    })
}

fn push_message(state: &Rc<RefCell<State>>, text: String) {
    let mut s = state.borrow_mut();
    if s.messages.len() < MAX_MESSAGES {
        s.messages.push(text.chars().take(200).collect());
    }
}

fn build_api(lua: &Lua, state: &Rc<RefCell<State>>) -> mlua::Result<mlua::Table> {
    let api = lua.create_table()?;
    let (w, h) = {
        let s = state.borrow();
        (s.width, s.height)
    };
    api.set("api_version", API_VERSION)?;
    api.set("width", w)?;
    api.set("height", h)?;
    let s = state.clone();
    api.set(
        "get_pixel",
        lua.create_function(move |_, (x, y): (i64, i64)| {
            let s = s.borrow();
            // Four nils outside the image.
            Ok(match s.index(x, y) {
                Some(i) => {
                    let p = &s.pixels[i..i + 4];
                    (Some(p[0]), Some(p[1]), Some(p[2]), Some(p[3]))
                }
                None => (None, None, None, None),
            })
        })?,
    )?;
    let s = state.clone();
    api.set(
        "set_pixel",
        lua.create_function(
            move |_,
                  (x, y, r, g, b, a): (
                i64,
                i64,
                Option<f64>,
                Option<f64>,
                Option<f64>,
                Option<f64>,
            )| {
                let mut s = s.borrow_mut();
                if !s.selected(x, y) {
                    return Ok(false);
                }
                let Some(i) = s.index(x, y) else {
                    return Ok(false);
                };
                let new = [channel(r, 0), channel(g, 0), channel(b, 0), channel(a, 255)];
                if s.pixels[i..i + 4] != new {
                    s.pixels[i..i + 4].copy_from_slice(&new);
                    s.changed = true;
                }
                Ok(true)
            },
        )?,
    )?;
    let s = state.clone();
    api.set(
        "selection",
        lua.create_function(move |_, ()| {
            // Four nils without a selection.
            Ok(
                match s.borrow().selection.as_ref().map(|sel| sel.bounds()) {
                    Some(r) => (Some(r.x), Some(r.y), Some(r.w), Some(r.h)),
                    None => (None, None, None, None),
                },
            )
        })?,
    )?;
    let s = state.clone();
    api.set(
        "is_selected",
        lua.create_function(move |_, (x, y): (i64, i64)| {
            let s = s.borrow();
            Ok(s.index(x, y).is_some() && s.selected(x, y))
        })?,
    )?;
    for (name, secondary) in [("primary", false), ("secondary", true)] {
        let s = state.clone();
        api.set(
            name,
            lua.create_function(move |_, ()| {
                let s = s.borrow();
                let c = if secondary {
                    s.secondary
                } else {
                    s.new_primary.unwrap_or(s.primary)
                };
                Ok((c.r(), c.g(), c.b(), c.a()))
            })?,
        )?;
    }
    let s = state.clone();
    api.set(
        "set_primary",
        lua.create_function(
            move |_, (r, g, b, a): (Option<f64>, Option<f64>, Option<f64>, Option<f64>)| {
                s.borrow_mut().new_primary = Some(Rgba::new(
                    channel(r, 0),
                    channel(g, 0),
                    channel(b, 0),
                    channel(a, 255),
                ));
                Ok(())
            },
        )?,
    )?;
    let s = state.clone();
    api.set(
        "message",
        lua.create_function(move |_, text: String| {
            push_message(&s, text);
            Ok(())
        })?,
    )?;
    Ok(api)
}

/// A plugin command running on a background thread.
pub struct PluginJob {
    pub plugin_name: String,
    pub title: String,
    /// History state of the document when the command started.
    pub state_id: u64,
    pub started: Instant,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Result<PluginOutput, String>>,
}

impl PluginJob {
    pub fn spawn(
        plugin: &Plugin,
        command: &str,
        input: PluginInput,
        state_id: u64,
    ) -> Result<Self, String> {
        let manifest = plugin.manifest.clone().ok_or("the plugin can't be used")?;
        let title = manifest
            .commands
            .iter()
            .find(|c| c.id == command)
            .map(|c| c.title.clone())
            .ok_or_else(|| format!("the plugin has no command \"{command}\""))?;
        let source = read_limited(&plugin.dir.join("main.lua"), MAX_SCRIPT_BYTES, "main.lua")?;
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let flag = cancel.clone();
        let command = command.to_owned();
        let name = manifest.name.clone();
        std::thread::Builder::new()
            .name("kagero-plugin".into())
            .spawn(move || {
                let result = run(&source, &manifest, &command, input, &flag);
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            plugin_name: name,
            title,
            state_id,
            started: Instant::now(),
            cancel,
            rx,
        })
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The result once the command has finished.
    pub fn poll(&self) -> Option<Result<PluginOutput, String>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("the plugin stopped unexpectedly".into())),
        }
    }
}

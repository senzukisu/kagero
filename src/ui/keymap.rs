//! Rebindable keyboard shortcuts.
//!
//! Every [`Action`] has a default key; the settings file stores only the user's
//! changes (`action id → "Ctrl+Shift+E"`, or `""` for no key). A few keys are fixed
//! and not listed as actions: Ctrl+C/X/V (Windows delivers them as clipboard events),
//! Esc, Space (hold to pan) and the arrow keys (nudge the selection or change frames).

use std::collections::BTreeMap;

use egui::{Key, Modifiers};

use super::settings::SettingsTab;
use super::Command;
use crate::editor::{ToolKind, Transform};
use crate::export::ExportKind;

/// A key with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl Binding {
    const fn new(ctrl: bool, shift: bool, alt: bool, key: Key) -> Self {
        Self {
            ctrl,
            shift,
            alt,
            key,
        }
    }

    /// Parses `"Ctrl+Shift+E"`; modifiers in any order, key last.
    pub fn parse(text: &str) -> Option<Self> {
        let mut b = Binding::new(false, false, false, Key::A);
        let text = text.trim();
        // The plus key itself: "+" or "Ctrl++".
        let (mods, key) = if text == "+" {
            ("", "+")
        } else if let Some(mods) = text.strip_suffix("++") {
            (mods, "+")
        } else {
            text.rsplit_once('+').unwrap_or(("", text))
        };
        if key.is_empty() {
            return None;
        }
        for m in mods.split('+').map(str::trim).filter(|m| !m.is_empty()) {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" => b.ctrl = true,
                "shift" => b.shift = true,
                "alt" => b.alt = true,
                _ => return None,
            }
        }
        b.key = Key::from_name(key)?;
        Some(b)
    }

    /// The text stored in the settings, e.g. `"Ctrl+Shift+E"`.
    pub fn to_config(self) -> String {
        let mut s = String::new();
        for (on, name) in [
            (self.ctrl, "Ctrl+"),
            (self.shift, "Shift+"),
            (self.alt, "Alt+"),
        ] {
            if on {
                s.push_str(name);
            }
        }
        s.push_str(self.key.name());
        s
    }

    /// The text shown to the user, e.g. `"Ctrl+Shift+E"` or `"Esc"`.
    pub fn label(self) -> String {
        let key = match self.key {
            Key::Escape => "Esc",
            Key::ArrowLeft => "←",
            Key::ArrowRight => "→",
            Key::ArrowUp => "↑",
            Key::ArrowDown => "↓",
            Key::Minus => "−",
            Key::Plus => "+",
            Key::Equals => "=",
            Key::Comma => ",",
            Key::Period => ".",
            Key::Semicolon => ";",
            Key::Slash => "/",
            Key::Backslash => "\\",
            Key::OpenBracket => "[",
            Key::CloseBracket => "]",
            Key::Backtick => "`",
            Key::Quote => "'",
            k => k.name(),
        };
        let mut s = String::new();
        for (on, name) in [
            (self.ctrl, "Ctrl+"),
            (self.shift, "Shift+"),
            (self.alt, "Alt+"),
        ] {
            if on {
                s.push_str(name);
            }
        }
        s.push_str(key);
        s
    }

    /// Whether a key event with `mods` triggers this binding. Modifiers must match
    /// exactly, except Shift for keys that need it on some layouts (`+`, `=`, `-`).
    pub fn matches(self, key: Key, mods: Modifiers) -> bool {
        let shift_ok = mods.shift == self.shift
            || (!self.shift && matches!(self.key, Key::Plus | Key::Equals | Key::Minus));
        key == self.key && mods.command == self.ctrl && mods.alt == self.alt && shift_ok
    }

    /// Number of modifiers; more specific bindings are checked first.
    fn specificity(self) -> u8 {
        u8::from(self.ctrl) + u8::from(self.shift) + u8::from(self.alt)
    }
}

/// Groups shown in Settings → Shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Files,
    Edit,
    Tools,
    View,
    Animation,
    Layers,
}

impl Group {
    pub const ALL: [Group; 6] = [
        Group::Files,
        Group::Edit,
        Group::Tools,
        Group::View,
        Group::Animation,
        Group::Layers,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Files => "Files",
            Group::Edit => "Edit & selection",
            Group::Tools => "Tools",
            Group::View => "View",
            Group::Animation => "Animation",
            Group::Layers => "Layers",
        }
    }
}

macro_rules! actions {
    ($( $name:ident => $id:literal, $label:literal, $group:ident, $default:expr; )*) => {
        /// Everything a shortcut can do.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Action { $( $name, )* }

        impl Action {
            pub const ALL: &'static [Action] = &[ $( Action::$name, )* ];

            /// Stable id used in the settings file.
            pub fn id(self) -> &'static str {
                match self { $( Action::$name => $id, )* }
            }

            pub fn label(self) -> &'static str {
                match self { $( Action::$name => $label, )* }
            }

            pub fn group(self) -> Group {
                match self { $( Action::$name => Group::$group, )* }
            }

            pub fn default_binding(self) -> Option<Binding> {
                match self { $( Action::$name => $default, )* }
            }
        }
    };
}

const fn key(k: Key) -> Option<Binding> {
    Some(Binding::new(false, false, false, k))
}
const fn ctrl(k: Key) -> Option<Binding> {
    Some(Binding::new(true, false, false, k))
}
const fn ctrl_shift(k: Key) -> Option<Binding> {
    Some(Binding::new(true, true, false, k))
}
const fn shift(k: Key) -> Option<Binding> {
    Some(Binding::new(false, true, false, k))
}
const fn alt(k: Key) -> Option<Binding> {
    Some(Binding::new(false, false, true, k))
}

actions! {
    NewDocument => "file.new", "New image", Files, ctrl(Key::N);
    Open => "file.open", "Open", Files, ctrl(Key::O);
    Save => "file.save", "Save", Files, ctrl(Key::S);
    SaveAs => "file.save_as", "Save As", Files, ctrl_shift(Key::S);
    ExportFrame => "file.export_frame", "Export current frame as PNG", Files, ctrl(Key::E);
    Export => "file.export", "Export dialog", Files, ctrl_shift(Key::E);
    Close => "file.close", "Close document", Files, ctrl(Key::W);
    Settings => "file.settings", "Settings", Files, ctrl(Key::Comma);

    Undo => "edit.undo", "Undo", Edit, ctrl(Key::Z);
    Redo => "edit.redo", "Redo", Edit, ctrl(Key::Y);
    SelectAll => "edit.select_all", "Select all", Edit, ctrl(Key::A);
    DeleteSelection => "edit.delete", "Clear selection", Edit, key(Key::Delete);
    Duplicate => "edit.duplicate", "Duplicate selection (or frame)", Edit, ctrl(Key::D);
    FlipHorizontal => "edit.flip_h", "Flip horizontal", Edit, shift(Key::H);
    FlipVertical => "edit.flip_v", "Flip vertical", Edit, shift(Key::V);
    RotateCw => "edit.rotate_cw", "Rotate 90° clockwise", Edit, ctrl(Key::R);
    RotateCcw => "edit.rotate_ccw", "Rotate 90° counterclockwise", Edit, ctrl_shift(Key::R);
    SwapColors => "edit.swap_colors", "Swap colors", Edit, key(Key::X);

    Pencil => "tool.pencil", "Pencil", Tools, key(Key::B);
    Eraser => "tool.eraser", "Eraser", Tools, key(Key::E);
    Fill => "tool.fill", "Fill", Tools, key(Key::G);
    Eyedropper => "tool.eyedropper", "Eyedropper", Tools, key(Key::I);
    Line => "tool.line", "Line", Tools, key(Key::L);
    Rectangle => "tool.rectangle", "Rectangle", Tools, key(Key::U);
    FilledRectangle => "tool.filled_rectangle", "Filled rectangle", Tools, shift(Key::U);
    Pan => "tool.pan", "Pan", Tools, key(Key::H);
    Select => "tool.select", "Rectangle select", Tools, key(Key::M);

    ZoomIn => "view.zoom_in", "Zoom in", View, key(Key::Plus);
    ZoomOut => "view.zoom_out", "Zoom out", View, key(Key::Minus);
    ActualSize => "view.actual_size", "Actual size", View, key(Key::Num1);
    Fit => "view.fit", "Fit to window", View, key(Key::Num0);
    Center => "view.center", "Center canvas", View, key(Key::Home);
    Grid => "view.grid", "Pixel grid", View, ctrl(Key::G);
    Timeline => "view.timeline", "Timeline", View, key(Key::F7);
    SidePanel => "view.side_panel", "Colors & layers panel", View, key(Key::F8);

    Play => "anim.play", "Play / stop preview", Animation, key(Key::Enter);
    PrevFrame => "anim.prev", "Previous frame", Animation, key(Key::Comma);
    NextFrame => "anim.next", "Next frame", Animation, key(Key::Period);
    FirstFrame => "anim.first", "First frame", Animation, ctrl(Key::Home);
    NewFrame => "anim.new_frame", "New frame", Animation, alt(Key::N);
    DuplicateFrame => "anim.duplicate_frame", "Duplicate frame", Animation, alt(Key::D);
    OnionSkin => "anim.onion", "Onion skin", Animation, key(Key::O);

    NewLayer => "layer.new", "New layer", Layers, ctrl_shift(Key::N);
    RenameLayer => "layer.rename", "Rename layer", Layers, key(Key::F2);
    LayerUp => "layer.up", "Move layer up", Layers, ctrl(Key::CloseBracket);
    LayerDown => "layer.down", "Move layer down", Layers, ctrl(Key::OpenBracket);
}

/// Shortcuts that can't be changed, for the list in Settings → Shortcuts.
pub const FIXED: [(&str, &str); 6] = [
    ("Ctrl+C · Ctrl+X · Ctrl+V", "Copy · cut · paste"),
    ("Esc", "Cancel a stroke, stop the preview, deselect"),
    ("Space (hold)", "Pan with any tool"),
    (
        "Arrows · Shift+arrows",
        "Nudge the selection 1 · 10 px, or change frames",
    ),
    ("Ctrl+Shift+Z", "Redo (also)"),
    ("=", "Zoom in (also)"),
];

/// Editor state that changes what some keys do.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyContext {
    pub active_layer: usize,
    /// With a selection, arrows nudge it and Duplicate copies it.
    pub has_selection: bool,
}

impl Action {
    /// Whether the action also works while a text field has focus (file actions use
    /// Ctrl and never conflict with typing).
    pub fn works_while_typing(self) -> bool {
        self.group() == Group::Files
    }

    pub fn from_id(id: &str) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| a.id() == id)
    }

    pub fn command(self, keys: KeyContext) -> Command {
        use Action as A;
        match self {
            A::NewDocument => Command::NewDocument,
            A::Open => Command::Open,
            A::Save => Command::Save,
            A::SaveAs => Command::SaveAs,
            A::ExportFrame => Command::ExportPng,
            A::Export => Command::OpenExport(ExportKind::Image),
            A::Close => Command::CloseDocument,
            A::Settings => Command::OpenSettings(SettingsTab::General),
            A::Undo => Command::Undo,
            A::Redo => Command::Redo,
            A::SelectAll => Command::SelectAll,
            A::DeleteSelection => Command::DeleteSelection,
            A::Duplicate if keys.has_selection => Command::DuplicateSelection,
            A::Duplicate => Command::DuplicateFrame,
            A::FlipHorizontal => Command::Transform(Transform::FlipHorizontal),
            A::FlipVertical => Command::Transform(Transform::FlipVertical),
            A::RotateCw => Command::Transform(Transform::Rotate90),
            A::RotateCcw => Command::Transform(Transform::Rotate270),
            A::SwapColors => Command::SwapColors,
            A::Pencil => Command::SetTool(ToolKind::Pencil),
            A::Eraser => Command::SetTool(ToolKind::Eraser),
            A::Fill => Command::SetTool(ToolKind::Fill),
            A::Eyedropper => Command::SetTool(ToolKind::Eyedropper),
            A::Line => Command::SetTool(ToolKind::Line),
            A::Rectangle => Command::SetTool(ToolKind::Rectangle),
            A::FilledRectangle => Command::SetTool(ToolKind::FilledRectangle),
            A::Pan => Command::SetTool(ToolKind::Pan),
            A::Select => Command::SetTool(ToolKind::Select),
            A::ZoomIn => Command::ZoomIn,
            A::ZoomOut => Command::ZoomOut,
            A::ActualSize => Command::ZoomActual,
            A::Fit => Command::FitCanvas,
            A::Center => Command::CenterCanvas,
            A::Grid => Command::ToggleGrid,
            A::Timeline => Command::ToggleTimeline,
            A::SidePanel => Command::ToggleSidePanel,
            A::Play => Command::TogglePlayback,
            A::PrevFrame => Command::PrevFrame,
            A::NextFrame => Command::NextFrame,
            A::FirstFrame => Command::FirstFrame,
            A::NewFrame => Command::AddFrame,
            A::DuplicateFrame => Command::DuplicateFrame,
            A::OnionSkin => Command::ToggleOnionSkin,
            A::NewLayer => Command::AddLayer,
            A::RenameLayer => Command::BeginRenameLayer(keys.active_layer),
            A::LayerUp => Command::LayerUp,
            A::LayerDown => Command::LayerDown,
        }
    }

    /// The action that selects `tool`.
    pub fn for_tool(tool: ToolKind) -> Action {
        match tool {
            ToolKind::Pencil => Action::Pencil,
            ToolKind::Eraser => Action::Eraser,
            ToolKind::Fill => Action::Fill,
            ToolKind::Eyedropper => Action::Eyedropper,
            ToolKind::Line => Action::Line,
            ToolKind::Rectangle => Action::Rectangle,
            ToolKind::FilledRectangle => Action::FilledRectangle,
            ToolKind::Pan => Action::Pan,
            ToolKind::Select => Action::Select,
        }
    }
}

/// The user's shortcut changes as stored in the settings: action id → binding text
/// (`""` = no key). Unknown ids and unparsable keys are ignored.
pub type Overrides = BTreeMap<String, String>;

/// The resolved shortcuts.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<(Action, Option<Binding>)>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::from_overrides(&Overrides::new())
    }
}

impl Keymap {
    pub fn from_overrides(overrides: &Overrides) -> Self {
        let bindings = Action::ALL
            .iter()
            .map(|&a| {
                let b = match overrides.get(a.id()) {
                    Some(text) if text.is_empty() => None,
                    Some(text) => Binding::parse(text).or_else(|| a.default_binding()),
                    None => a.default_binding(),
                };
                (a, b)
            })
            .collect();
        Self { bindings }
    }

    pub fn binding(&self, action: Action) -> Option<Binding> {
        self.bindings
            .iter()
            .find(|(a, _)| *a == action)
            .and_then(|(_, b)| *b)
    }

    /// The key as text for menus and tooltips (`""` when there is none).
    pub fn label(&self, action: Action) -> String {
        self.binding(action).map(Binding::label).unwrap_or_default()
    }

    /// The action that already uses `binding`, if any.
    pub fn action_for(&self, binding: Binding) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(_, b)| *b == Some(binding))
            .map(|(a, _)| *a)
    }

    /// Bindings ordered from most to least specific (Ctrl+Shift+S before Ctrl+S).
    pub fn ordered(&self) -> Vec<(Action, Binding)> {
        let mut v: Vec<(Action, Binding)> = self
            .bindings
            .iter()
            .filter_map(|(a, b)| b.map(|b| (*a, b)))
            .collect();
        v.sort_by_key(|(_, b)| std::cmp::Reverse(b.specificity()));
        v
    }
}

/// Sets `action` to `binding` in the overrides. A key used by another action is
/// taken away from it; returns that action so the UI can say so.
pub fn assign(
    overrides: &mut Overrides,
    action: Action,
    binding: Option<Binding>,
) -> Option<Action> {
    let current = Keymap::from_overrides(overrides);
    let mut taken_from = None;
    if let Some(b) = binding {
        if let Some(other) = current.action_for(b).filter(|o| *o != action) {
            store(overrides, other, None);
            taken_from = Some(other);
        }
    }
    store(overrides, action, binding);
    taken_from
}

fn store(overrides: &mut Overrides, action: Action, binding: Option<Binding>) {
    if binding == action.default_binding() {
        overrides.remove(action.id());
    } else {
        overrides.insert(
            action.id().to_owned(),
            binding.map(Binding::to_config).unwrap_or_default(),
        );
    }
}

/// Keeps only known actions and valid keys (settings files can be edited by hand).
pub fn sanitize(overrides: &mut Overrides) {
    overrides.retain(|id, text| {
        Action::from_id(id).is_some() && (text.is_empty() || Binding::parse(text).is_some())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_unique() {
        let map = Keymap::default();
        for (i, (a, b)) in map.ordered().iter().enumerate() {
            for (c, d) in map.ordered().iter().skip(i + 1) {
                assert!(b != d, "{a:?} and {c:?} share {}", b.label());
            }
        }
    }

    #[test]
    fn ids_are_unique_and_round_trip() {
        for a in Action::ALL {
            assert_eq!(Action::from_id(a.id()), Some(*a));
            if let Some(b) = a.default_binding() {
                assert_eq!(Binding::parse(&b.to_config()), Some(b), "{a:?}");
            }
        }
    }

    #[test]
    fn parsing() {
        assert_eq!(
            Binding::parse("Ctrl+Shift+E"),
            Some(Binding::new(true, true, false, Key::E))
        );
        assert_eq!(
            Binding::parse("shift+alt+F7"),
            Some(Binding::new(false, true, true, Key::F7))
        );
        assert_eq!(
            Binding::parse("Ctrl++"),
            Some(Binding::new(true, false, false, Key::Plus))
        );
        assert_eq!(Binding::parse("Hyper+E"), None);
        assert_eq!(Binding::parse("Ctrl+NotAKey"), None);
        assert_eq!(Binding::parse(""), None);
    }

    #[test]
    fn assigning_moves_a_taken_key_and_resets_to_default() {
        let mut o = Overrides::new();
        let b = Binding::parse("B").unwrap();
        // B belongs to the Pencil: giving it to Fill unbinds the Pencil.
        assert_eq!(assign(&mut o, Action::Fill, Some(b)), Some(Action::Pencil));
        let map = Keymap::from_overrides(&o);
        assert_eq!(map.binding(Action::Fill), Some(b));
        assert_eq!(map.binding(Action::Pencil), None);
        // Back to the defaults: nothing stored.
        assign(&mut o, Action::Fill, Action::Fill.default_binding());
        assign(&mut o, Action::Pencil, Action::Pencil.default_binding());
        assert!(o.is_empty(), "{o:?}");
    }

    #[test]
    fn exact_modifiers() {
        let e = Binding::parse("E").unwrap();
        assert!(e.matches(Key::E, Modifiers::NONE));
        assert!(!e.matches(Key::E, Modifiers::COMMAND));
        assert!(!e.matches(Key::E, Modifiers::ALT));
        assert!(!e.matches(Key::E, Modifiers::SHIFT));
        let plus = Binding::parse("+").unwrap();
        assert!(
            plus.matches(Key::Plus, Modifiers::SHIFT),
            "+ needs Shift on many layouts"
        );
    }

    #[test]
    fn broken_overrides_are_dropped() {
        let mut o = Overrides::new();
        o.insert("tool.pencil".into(), "Ctrl+P".into());
        o.insert("tool.nope".into(), "P".into());
        o.insert("tool.fill".into(), "Ctrl+Banana".into());
        o.insert("tool.eraser".into(), String::new());
        sanitize(&mut o);
        assert_eq!(o.len(), 2);
        let map = Keymap::from_overrides(&o);
        assert_eq!(map.label(Action::Pencil), "Ctrl+P");
        assert_eq!(map.binding(Action::Eraser), None);
    }
}

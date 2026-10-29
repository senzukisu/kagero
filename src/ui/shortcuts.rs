//! Keyboard shortcuts → commands, using the user's [`Keymap`].
//!
//! When a text field has focus only file shortcuts (Ctrl+S, Ctrl+O, …) are handled;
//! letters, Space, arrows, Enter, Delete and Ctrl+A/Z/C/X/V stay with the text field.

use egui::{Event, Key, Modifiers};

pub use super::keymap::KeyContext;
use super::keymap::{Action, Keymap};
use super::Command;

/// Shortcuts pressed this frame, plus whether Escape was pressed.
pub fn collect(ctx: &egui::Context, keymap: &Keymap, keys: KeyContext) -> (Vec<Command>, bool) {
    let typing = ctx.wants_keyboard_input();
    let mut commands = Vec::new();
    let mut escape = false;
    let ordered = keymap.ordered();
    ctx.input_mut(|i| {
        let mut consumed = Vec::new();
        for (index, event) in i.events.iter().enumerate() {
            match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let found = ordered.iter().find(|(a, b)| {
                        b.matches(*key, *modifiers) && (!typing || a.works_while_typing())
                    });
                    if let Some((action, _)) = found {
                        commands.push(action.command(keys));
                        consumed.push(index);
                    } else if !typing {
                        if let Some(command) = fixed_key(*key, *modifiers, keymap, keys) {
                            commands.push(command);
                            consumed.push(index);
                        }
                    }
                }
                // egui delivers Ctrl+C/X/V as clipboard events rather than key presses.
                Event::Copy if !typing => commands.push(Command::Copy),
                Event::Cut if !typing => commands.push(Command::Cut),
                Event::Paste(_) if !typing => commands.push(Command::Paste),
                _ => {}
            }
        }
        for index in consumed.into_iter().rev() {
            i.events.remove(index);
        }
        escape = i.key_pressed(Key::Escape);
    });
    (commands, escape)
}

/// Keys that can't be rebound: arrows, the Redo and Zoom aliases.
fn fixed_key(key: Key, mods: Modifiers, keymap: &Keymap, keys: KeyContext) -> Option<Command> {
    let arrow = match key {
        Key::ArrowLeft => Some((-1, 0)),
        Key::ArrowRight => Some((1, 0)),
        Key::ArrowUp => Some((0, -1)),
        Key::ArrowDown => Some((0, 1)),
        _ => None,
    };
    if let Some((dx, dy)) = arrow {
        if keys.has_selection && !mods.command && !mods.alt {
            // Arrows nudge the selected pixels (Shift: 10 px).
            let step = if mods.shift { 10 } else { 1 };
            return Some(Command::MoveSelection(dx * step, dy * step));
        }
        if !keys.has_selection && mods.is_none() {
            return match key {
                Key::ArrowLeft => Some(Command::PrevFrame),
                Key::ArrowRight => Some(Command::NextFrame),
                _ => None,
            };
        }
        return None;
    }
    let free =
        |b: &str| super::keymap::Binding::parse(b).is_some_and(|b| keymap.action_for(b).is_none());
    if key == Key::Z && mods.command && mods.shift && !mods.alt && free("Ctrl+Shift+Z") {
        return Some(Command::Redo);
    }
    if key == Key::Equals && !mods.command && !mods.alt && free("=") {
        return keymap.binding(Action::ZoomIn).map(|_| Command::ZoomIn);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{ToolKind as T, Transform};
    use crate::export::ExportKind;
    use crate::ui::keymap::{assign, Binding, Overrides};

    fn key(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// Runs one frame with `events` and returns the collected commands. With
    /// `text_focus`, a focused text field is shown first (as when renaming a layer).
    fn run_with(
        keymap: &Keymap,
        events: Vec<egui::Event>,
        keys: KeyContext,
        text_focus: bool,
    ) -> Vec<Command> {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let mut frame = |events: Vec<egui::Event>, collect_now: bool| {
            let mut found = Vec::new();
            let modifiers = events
                .iter()
                .find_map(|e| match e {
                    egui::Event::Key { modifiers, .. } => Some(*modifiers),
                    _ => None,
                })
                .unwrap_or_default();
            let input = egui::RawInput {
                events,
                modifiers,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if text_focus {
                        ui.text_edit_singleline(&mut text).request_focus();
                    }
                });
                if collect_now {
                    found = collect(ctx, keymap, keys).0;
                }
            });
            found
        };
        frame(Vec::new(), false);
        frame(events, true)
    }

    fn run(events: Vec<egui::Event>, keys: KeyContext, text_focus: bool) -> Vec<Command> {
        run_with(&Keymap::default(), events, keys, text_focus)
    }

    #[test]
    fn export_and_clipboard_shortcuts() {
        let ctx = KeyContext::default();
        assert_eq!(
            run(
                vec![key(Key::E, Modifiers::COMMAND | Modifiers::SHIFT)],
                ctx,
                false
            ),
            vec![Command::OpenExport(ExportKind::Image)]
        );
        assert_eq!(
            run(vec![key(Key::E, Modifiers::COMMAND)], ctx, false),
            vec![Command::ExportPng]
        );
        assert_eq!(
            run(vec![egui::Event::Copy], ctx, false),
            vec![Command::Copy]
        );
        assert_eq!(run(vec![egui::Event::Cut], ctx, false), vec![Command::Cut]);
        assert_eq!(
            run(vec![egui::Event::Paste("x".into())], ctx, false),
            vec![Command::Paste]
        );
        assert_eq!(
            run(vec![key(Key::M, Modifiers::NONE)], ctx, false),
            vec![Command::SetTool(T::Select)]
        );
        // Shift+U is the filled rectangle, not the rectangle.
        assert_eq!(
            run(vec![key(Key::U, Modifiers::SHIFT)], ctx, false),
            vec![Command::SetTool(T::FilledRectangle)]
        );
        // Plain letters don't fire with Alt held.
        assert!(run(vec![key(Key::E, Modifiers::ALT)], ctx, false).is_empty());
    }

    #[test]
    fn selection_changes_arrows_and_ctrl_d() {
        let none = KeyContext::default();
        let sel = KeyContext {
            has_selection: true,
            ..KeyContext::default()
        };
        assert_eq!(
            run(vec![key(Key::ArrowRight, Modifiers::NONE)], none, false),
            vec![Command::NextFrame]
        );
        assert_eq!(
            run(vec![key(Key::ArrowRight, Modifiers::NONE)], sel, false),
            vec![Command::MoveSelection(1, 0)]
        );
        assert_eq!(
            run(vec![key(Key::ArrowUp, Modifiers::SHIFT)], sel, false),
            vec![Command::MoveSelection(0, -10)]
        );
        assert_eq!(
            run(vec![key(Key::D, Modifiers::COMMAND)], none, false),
            vec![Command::DuplicateFrame]
        );
        assert_eq!(
            run(vec![key(Key::D, Modifiers::COMMAND)], sel, false),
            vec![Command::DuplicateSelection]
        );
        assert_eq!(
            run(
                vec![key(Key::R, Modifiers::COMMAND | Modifiers::SHIFT)],
                sel,
                false
            ),
            vec![Command::Transform(Transform::Rotate270)]
        );
        assert_eq!(
            run(
                vec![key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT)],
                none,
                false
            ),
            vec![Command::Redo]
        );
    }

    #[test]
    fn text_fields_keep_their_keys() {
        let sel = KeyContext {
            has_selection: true,
            ..KeyContext::default()
        };
        for event in [
            egui::Event::Copy,
            egui::Event::Cut,
            egui::Event::Paste("x".into()),
            key(Key::Delete, Modifiers::NONE),
            key(Key::ArrowLeft, Modifiers::NONE),
            key(Key::A, Modifiers::COMMAND),
            key(Key::D, Modifiers::COMMAND),
            key(Key::Z, Modifiers::COMMAND),
            key(Key::M, Modifiers::NONE),
        ] {
            let found = run(vec![event.clone()], sel, true);
            assert!(
                found.is_empty(),
                "{event:?} was taken from the text field: {found:?}"
            );
        }
        // File shortcuts still work while typing.
        assert_eq!(
            run(
                vec![key(Key::E, Modifiers::COMMAND | Modifiers::SHIFT)],
                sel,
                true
            ),
            vec![Command::OpenExport(ExportKind::Image)]
        );
    }

    #[test]
    fn custom_bindings_are_used() {
        let mut o = Overrides::new();
        assign(&mut o, Action::Pencil, Binding::parse("P"));
        assign(&mut o, Action::Undo, Binding::parse("Ctrl+Alt+Z"));
        let map = Keymap::from_overrides(&o);
        let ctx = KeyContext::default();
        assert_eq!(
            run_with(&map, vec![key(Key::P, Modifiers::NONE)], ctx, false),
            vec![Command::SetTool(T::Pencil)]
        );
        assert!(run_with(&map, vec![key(Key::B, Modifiers::NONE)], ctx, false).is_empty());
        assert_eq!(
            run_with(
                &map,
                vec![key(Key::Z, Modifiers::COMMAND | Modifiers::ALT)],
                ctx,
                false
            ),
            vec![Command::Undo]
        );
        assert!(run_with(&map, vec![key(Key::Z, Modifiers::COMMAND)], ctx, false).is_empty());
    }
}

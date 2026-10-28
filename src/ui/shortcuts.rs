//! Keyboard shortcuts → commands.
//!
//! When a text field has focus only Ctrl-based file shortcuts are handled; letters,
//! Space, arrows, Enter, Delete and Ctrl+Z/C/X/V stay with the text field.

use egui::{Key, KeyboardShortcut as Shortcut, Modifiers};

use super::settings::SettingsTab;
use super::Command;
use crate::editor::{ToolKind as T, Transform};
use crate::export::ExportKind;

/// Editor state that changes what some keys do.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyContext {
    pub active_layer: usize,
    /// With a selection, arrows nudge it and Ctrl+D duplicates it.
    pub has_selection: bool,
}

/// Shortcuts pressed this frame, plus whether Escape was pressed.
pub fn collect(ctx: &egui::Context, keys: KeyContext) -> (Vec<Command>, bool) {
    let typing = ctx.wants_keyboard_input();
    let mut commands = Vec::new();
    let mut escape = false;
    let cmd = Modifiers::COMMAND;
    let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    ctx.input_mut(|i| {
        // Shift variants first: plain Ctrl+S would also match Ctrl+Shift+S.
        let global = [
            (cmd_shift, Key::S, Command::SaveAs),
            (cmd, Key::S, Command::Save),
            (cmd_shift, Key::N, Command::AddLayer),
            (cmd, Key::N, Command::NewDocument),
            (cmd, Key::O, Command::Open),
            (cmd_shift, Key::E, Command::OpenExport(ExportKind::Image)),
            (cmd, Key::E, Command::ExportPng),
            (cmd, Key::W, Command::CloseDocument),
            (cmd, Key::G, Command::ToggleGrid),
            (cmd, Key::Comma, Command::OpenSettings(SettingsTab::General)),
        ];
        for (mods, key, command) in global {
            if i.consume_shortcut(&Shortcut::new(mods, key)) {
                commands.push(command);
            }
        }
        if typing {
            return;
        }
        // egui delivers Ctrl+C/X/V as clipboard events rather than key presses.
        for event in &i.events {
            match event {
                egui::Event::Copy => commands.push(Command::Copy),
                egui::Event::Cut => commands.push(Command::Cut),
                egui::Event::Paste(_) => commands.push(Command::Paste),
                _ => {}
            }
        }
        let duplicate = if keys.has_selection {
            Command::DuplicateSelection
        } else {
            Command::DuplicateFrame
        };
        let editing = [
            (cmd_shift, Key::Z, Command::Redo),
            (cmd, Key::Z, Command::Undo),
            (cmd, Key::Y, Command::Redo),
            (cmd, Key::A, Command::SelectAll),
            (cmd, Key::D, duplicate),
            (cmd_shift, Key::R, Command::Transform(Transform::Rotate270)),
            (cmd, Key::R, Command::Transform(Transform::Rotate90)),
            (cmd, Key::CloseBracket, Command::LayerUp),
            (cmd, Key::OpenBracket, Command::LayerDown),
            (cmd, Key::Home, Command::FirstFrame),
            (Modifiers::ALT, Key::N, Command::AddFrame),
            (Modifiers::ALT, Key::D, Command::DuplicateFrame),
            (
                Modifiers::SHIFT,
                Key::H,
                Command::Transform(Transform::FlipHorizontal),
            ),
            (
                Modifiers::SHIFT,
                Key::V,
                Command::Transform(Transform::FlipVertical),
            ),
        ];
        for (mods, key, command) in editing {
            if i.consume_shortcut(&Shortcut::new(mods, key)) {
                commands.push(command);
            }
        }
        if i.modifiers.shift_only() && i.key_pressed(Key::U) {
            commands.push(Command::SetTool(T::FilledRectangle));
        }
        if keys.has_selection && (i.modifiers.is_none() || i.modifiers.shift_only()) {
            // Arrows nudge the selected pixels (Shift: 10 px).
            let step = if i.modifiers.shift { 10 } else { 1 };
            for (key, dx, dy) in [
                (Key::ArrowLeft, -step, 0),
                (Key::ArrowRight, step, 0),
                (Key::ArrowUp, 0, -step),
                (Key::ArrowDown, 0, step),
            ] {
                if i.key_pressed(key) {
                    commands.push(Command::MoveSelection(dx, dy));
                }
            }
        }
        if i.modifiers.is_none() {
            let plain = [
                (Key::B, Command::SetTool(T::Pencil)),
                (Key::E, Command::SetTool(T::Eraser)),
                (Key::G, Command::SetTool(T::Fill)),
                (Key::I, Command::SetTool(T::Eyedropper)),
                (Key::L, Command::SetTool(T::Line)),
                (Key::U, Command::SetTool(T::Rectangle)),
                (Key::H, Command::SetTool(T::Pan)),
                (Key::M, Command::SetTool(T::Select)),
                (Key::X, Command::SwapColors),
                (Key::O, Command::ToggleOnionSkin),
                (Key::Home, Command::CenterCanvas),
                (Key::Num0, Command::FitCanvas),
                (Key::Num1, Command::ZoomActual),
                (Key::F2, Command::BeginRenameLayer(keys.active_layer)),
                (Key::F7, Command::ToggleTimeline),
                (Key::F8, Command::ToggleSidePanel),
                (Key::Enter, Command::TogglePlayback),
                (Key::Comma, Command::PrevFrame),
                (Key::Period, Command::NextFrame),
                (Key::Delete, Command::DeleteSelection),
            ];
            for (key, command) in plain {
                if i.key_pressed(key) {
                    commands.push(command);
                }
            }
            if !keys.has_selection {
                if i.key_pressed(Key::ArrowLeft) {
                    commands.push(Command::PrevFrame);
                }
                if i.key_pressed(Key::ArrowRight) {
                    commands.push(Command::NextFrame);
                }
            }
        }
        if !i.modifiers.command && !i.modifiers.alt {
            if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                commands.push(Command::ZoomIn);
            }
            if i.key_pressed(Key::Minus) {
                commands.push(Command::ZoomOut);
            }
        }
        escape = i.key_pressed(Key::Escape);
    });
    (commands, escape)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn run(events: Vec<egui::Event>, keys: KeyContext, text_focus: bool) -> Vec<Command> {
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
                    found = collect(ctx, keys).0;
                }
            });
            found
        };
        frame(Vec::new(), false);
        frame(events, true)
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
}

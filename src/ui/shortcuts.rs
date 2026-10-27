//! Keyboard shortcuts → commands.
//!
//! When a text field has focus only Ctrl-based file shortcuts are handled; letters,
//! Space, arrows, Enter and Ctrl+Z stay with the text field.

use egui::{Key, KeyboardShortcut as Shortcut, Modifiers};

use super::settings::SettingsTab;
use super::Command;
use crate::editor::ToolKind as T;

/// Shortcuts pressed this frame, plus whether Escape was pressed.
pub fn collect(ctx: &egui::Context, active_layer: usize) -> (Vec<Command>, bool) {
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
            (cmd_shift, Key::E, Command::ExportAllFrames),
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
        let editing = [
            (cmd_shift, Key::Z, Command::Redo),
            (cmd, Key::Z, Command::Undo),
            (cmd, Key::Y, Command::Redo),
            (cmd, Key::CloseBracket, Command::LayerUp),
            (cmd, Key::OpenBracket, Command::LayerDown),
            (cmd, Key::Home, Command::FirstFrame),
            (Modifiers::ALT, Key::N, Command::AddFrame),
            (Modifiers::ALT, Key::D, Command::DuplicateFrame),
        ];
        for (mods, key, command) in editing {
            if i.consume_shortcut(&Shortcut::new(mods, key)) {
                commands.push(command);
            }
        }
        if i.modifiers.shift_only() && i.key_pressed(Key::U) {
            commands.push(Command::SetTool(T::FilledRectangle));
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
                (Key::X, Command::SwapColors),
                (Key::O, Command::ToggleOnionSkin),
                (Key::Home, Command::CenterCanvas),
                (Key::Num0, Command::FitCanvas),
                (Key::Num1, Command::ZoomActual),
                (Key::F2, Command::BeginRenameLayer(active_layer)),
                (Key::F7, Command::ToggleTimeline),
                (Key::F8, Command::ToggleSidePanel),
                (Key::Enter, Command::TogglePlayback),
                (Key::Comma, Command::PrevFrame),
                (Key::Period, Command::NextFrame),
                (Key::ArrowLeft, Command::PrevFrame),
                (Key::ArrowRight, Command::NextFrame),
            ];
            for (key, command) in plain {
                if i.key_pressed(key) {
                    commands.push(command);
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

//! Left tool strip.

use super::icons::{icon_button, Icon};
use super::keymap::{Action, Keymap};
use super::Command;
use crate::editor::ToolKind;

pub fn show(ui: &mut egui::Ui, current: ToolKind, keymap: &Keymap, commands: &mut Vec<Command>) {
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        for (i, tool) in ToolKind::ALL.into_iter().enumerate() {
            // Visual groups: freehand | fill & pick | shapes | navigation.
            if matches!(i, 2 | 4 | 7) {
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);
            }
            let key = keymap.label(Action::for_tool(tool));
            let tip = match (key.is_empty(), tool) {
                (true, ToolKind::Pan) => format!("{} (hold Space)", tool.name()),
                (true, _) => tool.name().to_owned(),
                (false, ToolKind::Pan) => format!("{} ({key} / hold Space)", tool.name()),
                (false, _) => format!("{} ({key})", tool.name()),
            };
            if icon_button(ui, Icon::for_tool(tool), 36.0, tool == current, true, &tip).clicked() {
                commands.push(Command::SetTool(tool));
            }
        }
    });
}

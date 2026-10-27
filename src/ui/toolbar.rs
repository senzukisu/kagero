//! Left tool strip.

use super::icons::{icon_button, Icon};
use super::Command;
use crate::editor::ToolKind;

pub fn show(ui: &mut egui::Ui, current: ToolKind, commands: &mut Vec<Command>) {
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        for (i, tool) in ToolKind::ALL.into_iter().enumerate() {
            // Visual groups: freehand | fill & pick | shapes | navigation.
            if matches!(i, 2 | 4 | 7) {
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);
            }
            let tip = format!("{} ({})", tool.name(), tool.shortcut());
            if icon_button(ui, Icon::for_tool(tool), 34.0, tool == current, true, &tip).clicked() {
                commands.push(Command::SetTool(tool));
            }
        }
    });
}

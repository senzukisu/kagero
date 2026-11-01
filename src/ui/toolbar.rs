//! Left tool strip: one column, or two when the window is too short for one.

use super::icons::{icon_button, Icon};
use super::keymap::{Action, Keymap};
use super::Command;
use crate::editor::ToolKind;

const BUTTON: f32 = 36.0;
/// Tools in visual groups: freehand | fill & pick | shapes | navigation & selection.
const GROUPS: [&[ToolKind]; 4] = [
    &[ToolKind::Pencil, ToolKind::Eraser],
    &[ToolKind::Fill, ToolKind::Eyedropper],
    &[
        ToolKind::Line,
        ToolKind::Rectangle,
        ToolKind::FilledRectangle,
        ToolKind::Ellipse,
        ToolKind::Polygon,
    ],
    &[
        ToolKind::Pan,
        ToolKind::Select,
        ToolKind::Lasso,
        ToolKind::MagicWand,
        ToolKind::Crop,
    ],
];

fn columns_id() -> egui::Id {
    egui::Id::new("toolbar-columns")
}

/// Width of the tool panel: one or two columns, as decided by the last frame.
pub fn panel_width(ctx: &egui::Context) -> f32 {
    let two = ctx
        .data(|d| d.get_temp::<bool>(columns_id()))
        .unwrap_or(false);
    if two {
        88.0
    } else {
        48.0
    }
}

pub fn show(ui: &mut egui::Ui, current: ToolKind, keymap: &Keymap, commands: &mut Vec<Command>) {
    let gap = ui.spacing().item_spacing.y;
    let one_column =
        ToolKind::ALL.len() as f32 * (BUTTON + gap) + (GROUPS.len() - 1) as f32 * 10.0 + 8.0;
    let two = ui.available_height() < one_column;
    let was_two = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(columns_id()))
        .unwrap_or(false);
    if two != was_two {
        ui.ctx().data_mut(|d| d.insert_temp(columns_id(), two));
        ui.ctx().request_repaint();
    }
    let mut button = |ui: &mut egui::Ui, tool: ToolKind| {
        let key = keymap.label(Action::for_tool(tool));
        let tip = match (key.is_empty(), tool) {
            (true, ToolKind::Pan) => format!("{} (hold Space)", tool.name()),
            (true, _) => tool.name().to_owned(),
            (false, ToolKind::Pan) => format!("{} ({key} / hold Space)", tool.name()),
            (false, _) => format!("{} ({key})", tool.name()),
        };
        if icon_button(
            ui,
            Icon::for_tool(tool),
            BUTTON,
            tool == current,
            true,
            &tip,
        )
        .clicked()
        {
            commands.push(Command::SetTool(tool));
        }
    };
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        for (g, group) in GROUPS.iter().enumerate() {
            if g > 0 {
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);
            }
            if two && was_two {
                for pair in group.chunks(2) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for &tool in pair {
                            button(ui, tool);
                        }
                    });
                }
            } else {
                for &tool in *group {
                    button(ui, tool);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_is_in_exactly_one_group() {
        let grouped: Vec<ToolKind> = GROUPS.iter().flat_map(|g| g.iter().copied()).collect();
        assert_eq!(grouped.len(), ToolKind::ALL.len());
        for tool in ToolKind::ALL {
            assert_eq!(
                grouped.iter().filter(|t| **t == tool).count(),
                1,
                "{tool:?}"
            );
        }
    }
}

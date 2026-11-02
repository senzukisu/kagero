//! Left tool strip: one column, or two when the window is too short for one.

use super::icons::{icon_button, Icon};
use super::keymap::{Action, Keymap};
use super::Command;
use crate::editor::ToolKind;

/// Side of a tool button, the gap between buttons, and the panel's inner margin.
const BUTTON: f32 = 32.0;
const GAP: f32 = 2.0;
pub const MARGIN: i8 = 6;
/// Space taken by the line between two groups.
const GROUP_GAP: f32 = 9.0;
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

/// Width of a grid of `columns` buttons.
fn grid_width(columns: usize) -> f32 {
    columns as f32 * BUTTON + (columns - 1) as f32 * GAP
}

/// Height of all groups laid out in `columns` columns.
fn grid_height(columns: usize) -> f32 {
    let rows: usize = GROUPS.iter().map(|g| g.len().div_ceil(columns)).sum();
    rows as f32 * BUTTON
        + (rows - GROUPS.len()) as f32 * GAP
        + (GROUPS.len() - 1) as f32 * GROUP_GAP
}

/// Width of the tool panel's contents (its frame adds `MARGIN` on each side): one or two
/// columns, as decided by the last frame.
pub fn panel_width(ctx: &egui::Context) -> f32 {
    let two = ctx
        .data(|d| d.get_temp::<bool>(columns_id()))
        .unwrap_or(false);
    grid_width(if two { 2 } else { 1 })
}

pub fn show(ui: &mut egui::Ui, current: ToolKind, keymap: &Keymap, commands: &mut Vec<Command>) {
    // Two columns when one doesn't fit; the panel width follows on the next frame.
    let two = ui.available_height() < grid_height(1) + 4.0;
    let was_two = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(columns_id()))
        .unwrap_or(false);
    if two != was_two {
        ui.ctx().data_mut(|d| d.insert_temp(columns_id(), two));
        ui.ctx().request_repaint();
    }
    let columns = if was_two { 2 } else { 1 };
    // An exact grid, centered in the panel: every button lines up.
    let area = ui.available_rect_before_wrap();
    let left = area.center().x - grid_width(columns) / 2.0;
    let mut y = area.top() + 2.0;
    let line = egui::Stroke::new(1.0_f32, crate::theme::border());
    for (g, group) in GROUPS.iter().enumerate() {
        if g > 0 {
            let mid = (y - GAP + GROUP_GAP / 2.0).round() + 0.5;
            ui.painter()
                .hline(left..=left + grid_width(columns), mid, line);
            y += GROUP_GAP - GAP;
        }
        for (i, &tool) in group.iter().enumerate() {
            let (col, row) = (i % columns, i / columns);
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    left + col as f32 * (BUTTON + GAP),
                    y + row as f32 * (BUTTON + GAP),
                ),
                egui::Vec2::splat(BUTTON),
            );
            let key = keymap.label(Action::for_tool(tool));
            let tip = match (key.is_empty(), tool) {
                (true, ToolKind::Pan) => format!("{} (hold Space)", tool.name()),
                (true, _) => tool.name().to_owned(),
                (false, ToolKind::Pan) => format!("{} ({key} / hold Space)", tool.name()),
                (false, _) => format!("{} ({key})", tool.name()),
            };
            let clicked = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    icon_button(
                        ui,
                        Icon::for_tool(tool),
                        BUTTON,
                        tool == current,
                        true,
                        &tip,
                    )
                    .clicked()
                })
                .inner;
            if clicked {
                commands.push(Command::SetTool(tool));
            }
        }
        y += group.len().div_ceil(columns) as f32 * (BUTTON + GAP);
    }
    ui.allocate_rect(
        egui::Rect::from_min_max(area.min, egui::pos2(area.max.x, y)),
        egui::Sense::hover(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_fits_the_panel() {
        // One column: 32 px buttons in a 44 px panel; two columns: 78 px.
        assert_eq!(grid_width(1) + 2.0 * f32::from(MARGIN), 44.0);
        assert_eq!(grid_width(2) + 2.0 * f32::from(MARGIN), 78.0);
        // Two columns need fewer rows, so less height.
        assert!(grid_height(2) < grid_height(1));
    }

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

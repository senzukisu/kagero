//! Options bar for the active tool, plus view toggles.

use egui::RichText;

use super::icons::{paint_icon, Icon};
use super::Command;
use crate::editor::{Editor, SampleSource, ToolKind, BRUSH_SIZE_RANGE};
use crate::theme;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, show_grid: bool, commands: &mut Vec<Command>) {
    ui.horizontal_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
        paint_icon(
            ui.painter(),
            rect,
            Icon::for_tool(editor.tool),
            theme::TEXT_PRIMARY,
            theme::ACCENT,
        );
        ui.label(
            RichText::new(editor.tool.name())
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.separator();

        if editor.tool.uses_brush_size() {
            ui.label("Size");
            let mut size = editor.settings.brush_size;
            let resp = ui.add(
                egui::Slider::new(&mut size, BRUSH_SIZE_RANGE)
                    .suffix(" px")
                    .clamping(egui::SliderClamping::Always),
            );
            if resp.changed() {
                editor.settings.brush_size = size;
            }
        }
        match editor.tool {
            ToolKind::Eyedropper => {
                ui.label("Sample from");
                let mut source = editor.settings.sample_source;
                egui::ComboBox::from_id_salt("sample-source")
                    .selected_text(source.name())
                    .show_ui(ui, |ui| {
                        for s in [SampleSource::Merged, SampleSource::ActiveLayer] {
                            ui.selectable_value(&mut source, s, s.name());
                        }
                    });
                editor.settings.sample_source = source;
                hint(
                    ui,
                    "Left click: primary color · Right click: secondary color",
                );
            }
            ToolKind::Fill => hint(
                ui,
                "Fills the connected area of the same color on the active layer",
            ),
            ToolKind::Pan => hint(ui, "Drag to move the view · Home centers the canvas"),
            ToolKind::Eraser => hint(ui, "Erases to full transparency"),
            ToolKind::Line | ToolKind::Rectangle => hint(ui, "Drag to draw · Esc cancels"),
            ToolKind::FilledRectangle => hint(ui, "Drag to draw a filled rectangle · Esc cancels"),
            ToolKind::Pencil => hint(ui, "Left: primary · Right: secondary"),
            ToolKind::Select => selection_options(ui, editor, commands),
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut grid = show_grid;
            if ui
                .checkbox(&mut grid, "Grid")
                .on_hover_text("Show the pixel grid when zoomed in (Ctrl+G)")
                .changed()
            {
                commands.push(Command::ToggleGrid);
            }
        });
    });
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::TEXT_SECONDARY));
}

fn selection_options(ui: &mut egui::Ui, editor: &Editor, commands: &mut Vec<Command>) {
    use super::icons::{icon_button, Icon};
    use crate::editor::Transform;
    let has = editor.selection().is_some();
    match editor.selection() {
        Some(r) => {
            ui.label(
                RichText::new(format!("{}×{} at {}, {}", r.w, r.h, r.x, r.y))
                    .monospace()
                    .color(theme::TEXT_PRIMARY),
            );
        }
        None => hint(ui, "Drag to select · Ctrl+A selects all"),
    }
    ui.separator();
    let buttons = [
        (
            Icon::FlipH,
            "Flip horizontal (Shift+H)",
            Command::Transform(Transform::FlipHorizontal),
        ),
        (
            Icon::FlipV,
            "Flip vertical (Shift+V)",
            Command::Transform(Transform::FlipVertical),
        ),
        (
            Icon::Rotate,
            "Rotate 90° clockwise (Ctrl+R)",
            Command::Transform(Transform::Rotate90),
        ),
    ];
    for (icon, tip, command) in buttons {
        if icon_button(ui, icon, 26.0, false, true, tip).clicked() {
            commands.push(command);
        }
    }
    if ui
        .small_button("180°")
        .on_hover_text("Rotate 180°")
        .clicked()
    {
        commands.push(Command::Transform(Transform::Rotate180));
    }
    if ui
        .small_button("270°")
        .on_hover_text("Rotate 90° counter-clockwise (Ctrl+Shift+R)")
        .clicked()
    {
        commands.push(Command::Transform(Transform::Rotate270));
    }
    ui.separator();
    if icon_button(ui, Icon::Copy, 26.0, false, has, "Copy (Ctrl+C)").clicked() {
        commands.push(Command::Copy);
    }
    if ui
        .add_enabled(has, egui::Button::new("Cut").small())
        .on_hover_text("Ctrl+X")
        .clicked()
    {
        commands.push(Command::Cut);
    }
    let can_paste = editor.clipboard.is_some();
    if ui
        .add_enabled(can_paste, egui::Button::new("Paste").small())
        .on_hover_text("Ctrl+V")
        .clicked()
    {
        commands.push(Command::Paste);
    }
    if ui
        .add_enabled(has, egui::Button::new("Deselect").small())
        .on_hover_text("Esc")
        .clicked()
    {
        commands.push(Command::Deselect);
    }
    hint(
        ui,
        "Drag inside: move pixels · Ctrl+drag: move outline · Arrows: nudge",
    );
}

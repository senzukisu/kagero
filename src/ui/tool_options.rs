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

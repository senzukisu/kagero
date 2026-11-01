//! Options bar for the active tool, plus view toggles.

use egui::RichText;

use super::icons::{paint_icon, Icon, Tint};
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
            Tint::Normal,
        );
        ui.label(
            RichText::new(editor.tool.name())
                .strong()
                .color(theme::text_primary()),
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
            brush_shape(ui, editor);
        }
        if matches!(
            editor.tool,
            ToolKind::Pencil
                | ToolKind::Eraser
                | ToolKind::Line
                | ToolKind::Rectangle
                | ToolKind::FilledRectangle
                | ToolKind::Ellipse
                | ToolKind::Polygon
                | ToolKind::Fill
        ) {
            paint_options(ui, editor);
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
            ToolKind::Fill => fill_options(ui, editor),
            ToolKind::Ellipse => {
                ui.checkbox(&mut editor.settings.shape_filled, "Filled");
                hint(ui, "Drag a box · Esc cancels");
            }
            ToolKind::Polygon => {
                ui.checkbox(&mut editor.settings.shape_filled, "Filled");
                hint(
                    ui,
                    "Click corners · double-click or Enter closes · Esc cancels",
                );
            }
            ToolKind::Pan => hint(ui, "Drag to move the view · Home centers the canvas"),
            ToolKind::Eraser => hint(ui, "Erases to full transparency"),
            ToolKind::Line | ToolKind::Rectangle => hint(ui, "Drag to draw · Esc cancels"),
            ToolKind::FilledRectangle => hint(ui, "Drag to draw a filled rectangle · Esc cancels"),
            ToolKind::Pencil => hint(ui, "Left: primary · Right: secondary"),
            ToolKind::Select | ToolKind::Lasso | ToolKind::MagicWand => {
                selection_options(ui, editor, commands)
            }
            ToolKind::Crop => crop_options(ui, editor, commands),
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

/// Brush tip: square, circle, or the custom brush made from a selection.
fn brush_shape(ui: &mut egui::Ui, editor: &mut Editor) {
    use crate::core::raster::BrushShape;
    let has_custom = editor.custom_brush.is_some();
    let mut shape = editor.settings.brush_shape;
    if shape == BrushShape::Custom && !has_custom {
        shape = BrushShape::Square;
    }
    egui::ComboBox::from_id_salt("brush-shape")
        .selected_text(shape.name())
        .width(78.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut shape, BrushShape::Square, "Square");
            ui.selectable_value(&mut shape, BrushShape::Circle, "Circle");
            ui.add_enabled_ui(has_custom, |ui| {
                ui.selectable_value(&mut shape, BrushShape::Custom, "Custom")
                    .on_disabled_hover_text(
                        "Select pixels, then Edit → Use Selection as Brush (Ctrl+B)",
                    );
            });
        })
        .response
        .on_hover_text("Brush tip");
    editor.settings.brush_shape = shape;
    if let (BrushShape::Custom, Some(b)) = (shape, &editor.custom_brush) {
        hint(ui, &format!("{}×{}", b.width, b.height));
    }
}

/// Dither pattern and symmetry, shared by the painting tools.
fn paint_options(ui: &mut egui::Ui, editor: &mut Editor) {
    use crate::core::dither::Dither;
    let s = &mut editor.settings;
    egui::ComboBox::from_id_salt("dither")
        .selected_text(s.dither.name())
        .width(92.0)
        .show_ui(ui, |ui| {
            for d in Dither::ALL {
                ui.selectable_value(&mut s.dither, d, d.name());
            }
        })
        .response
        .on_hover_text("Dither: paint only part of the pixels, in a fixed pattern");
    if editor.tool != ToolKind::Fill {
        ui.toggle_value(&mut s.mirror_x, "Mirror ↔")
            .on_hover_text("Symmetry: also paint mirrored left to right");
        ui.toggle_value(&mut s.mirror_y, "Mirror ↕")
            .on_hover_text("Symmetry: also paint mirrored top to bottom");
    }
    ui.separator();
}

fn fill_options(ui: &mut egui::Ui, editor: &mut Editor) {
    let s = &mut editor.settings;
    ui.label("Tolerance");
    ui.add(egui::DragValue::new(&mut s.fill_tolerance).range(0..=255))
        .on_hover_text("How different a color may be and still be filled (0 = exactly the same)");
    ui.checkbox(&mut s.fill_contiguous, "Contiguous")
        .on_hover_text("Only the connected area; off = every similar pixel of the layer");
    egui::ComboBox::from_id_salt("fill-source")
        .selected_text(match s.fill_source {
            SampleSource::ActiveLayer => "Active layer",
            SampleSource::Merged => "All visible layers",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut s.fill_source,
                SampleSource::ActiveLayer,
                "Active layer",
            );
            ui.selectable_value(
                &mut s.fill_source,
                SampleSource::Merged,
                "All visible layers",
            );
        })
        .response
        .on_hover_text("Which image decides the area (the fill always goes on the active layer)");
    hint(ui, "Stays inside the selection, if any");
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::text_secondary()));
}

fn selection_options(ui: &mut egui::Ui, editor: &mut Editor, commands: &mut Vec<Command>) {
    use super::icons::{icon_button, Icon};
    use crate::core::selection::SelectMode;
    use crate::editor::Transform;
    // How new selections combine; held keys override it.
    let mut mode = editor.settings.select_mode;
    ui.spacing_mut().item_spacing.x = 2.0;
    for m in SelectMode::ALL {
        let tip = match m {
            SelectMode::Replace => "New selection replaces the old one",
            SelectMode::Add => "Add to the selection (hold Shift)",
            SelectMode::Subtract => "Cut out of the selection (hold Alt)",
            SelectMode::Intersect => "Keep only the overlap (hold Shift+Alt)",
        };
        ui.selectable_value(&mut mode, m, m.name())
            .on_hover_text(tip);
    }
    editor.settings.select_mode = mode;
    ui.spacing_mut().item_spacing.x = 8.0;
    if ui
        .small_button("Invert")
        .on_hover_text("Select everything that isn't selected (Ctrl+Shift+I)")
        .clicked()
    {
        commands.push(Command::InvertSelection);
    }
    ui.separator();
    if editor.tool == ToolKind::MagicWand {
        let s = &mut editor.settings;
        ui.label("Tolerance");
        ui.add(egui::DragValue::new(&mut s.wand_tolerance).range(0..=255))
            .on_hover_text("How different a color may be and still count (0 = exactly the same, 255 = everything)");
        ui.checkbox(&mut s.wand_contiguous, "Contiguous")
            .on_hover_text("Only pixels connected to the one you click; off = every similar pixel");
        egui::ComboBox::from_id_salt("wand-source")
            .selected_text(match s.wand_source {
                SampleSource::ActiveLayer => "Active layer",
                SampleSource::Merged => "All visible layers",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut s.wand_source,
                    SampleSource::ActiveLayer,
                    "Active layer",
                );
                ui.selectable_value(
                    &mut s.wand_source,
                    SampleSource::Merged,
                    "All visible layers",
                );
            })
            .response
            .on_hover_text("Which colors the wand compares");
        ui.separator();
    }
    let has = editor.selection().is_some();
    match editor.selection_shape() {
        Some(s) if s.is_rectangular() => {
            let r = s.bounds();
            ui.label(
                RichText::new(format!("{}×{} at {}, {}", r.w, r.h, r.x, r.y))
                    .monospace()
                    .color(theme::text_primary()),
            );
        }
        Some(s) => {
            let r = s.bounds();
            ui.label(
                RichText::new(format!("{} px in {}×{}", s.pixel_count(), r.w, r.h))
                    .monospace()
                    .color(theme::text_primary()),
            )
            .on_hover_text(format!("Bounds at {}, {}", r.x, r.y));
        }
        None => hint(
            ui,
            match editor.tool {
                ToolKind::Lasso => "Drag around pixels to select them",
                ToolKind::MagicWand => "Click a color to select it",
                _ => "Drag to select · Ctrl+A selects all",
            },
        ),
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
    let tip = if editor.tool == ToolKind::Select {
        "Drag inside: move pixels · Ctrl+drag: move outline · Arrows: nudge"
    } else {
        "Shift: add · Alt: subtract · Arrows: nudge · M to move pixels"
    };
    hint(ui, tip);
}

/// Crop: the frame's size, Apply / Cancel, and cropping to the selection.
fn crop_options(ui: &mut egui::Ui, editor: &Editor, commands: &mut Vec<Command>) {
    match editor.crop_rect {
        Some(r) => {
            ui.label(
                RichText::new(format!("{}×{} at {}, {}", r.w, r.h, r.x, r.y))
                    .monospace()
                    .color(theme::text_primary()),
            );
            if ui
                .add(crate::ui::dialogs::accent_button("Crop"))
                .on_hover_text("Cut every layer and frame to the frame (Enter)")
                .clicked()
            {
                commands.push(Command::ApplyCrop);
            }
            if ui.button("Cancel").on_hover_text("Esc").clicked() {
                commands.push(Command::CancelCrop);
            }
        }
        None => hint(
            ui,
            "Drag a frame on the canvas, adjust its edges, then press Enter to crop",
        ),
    }
    if ui
        .add_enabled(
            editor.selection().is_some(),
            egui::Button::new("Crop to Selection"),
        )
        .clicked()
    {
        commands.push(Command::CropToSelection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Document, Rgba};

    #[test]
    fn every_tool_has_an_options_bar() {
        let doc = Document::new(8, 8, Rgba::TRANSPARENT).expect("valid");
        let mut e = Editor::new(doc, None, "T".into());
        let ctx = egui::Context::default();
        for tool in ToolKind::ALL {
            e.set_tool(tool);
            for _ in 0..2 {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let mut commands = Vec::new();
                        show(ui, &mut e, true, &mut commands);
                        assert!(commands.is_empty(), "{tool:?}: {commands:?}");
                    });
                });
            }
        }
    }
}

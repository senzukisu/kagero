//! Tool definitions and their settings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Pencil,
    Eraser,
    Fill,
    Eyedropper,
    Line,
    Rectangle,
    FilledRectangle,
    /// Ellipse inside the dragged rectangle (outline or filled).
    Ellipse,
    /// Click corners; double-click, Enter or clicking the last corner again finishes.
    Polygon,
    Pan,
    /// Rectangular selection: drag to select, drag inside to move the pixels.
    Select,
    /// Freehand selection: drag around the pixels to select.
    Lasso,
    /// Selects pixels of a similar color with one click.
    MagicWand,
    /// Crop frame: drag a rectangle, adjust it, Enter cuts the canvas to it.
    Crop,
}

impl ToolKind {
    pub const ALL: [ToolKind; 14] = [
        ToolKind::Pencil,
        ToolKind::Eraser,
        ToolKind::Fill,
        ToolKind::Eyedropper,
        ToolKind::Line,
        ToolKind::Rectangle,
        ToolKind::FilledRectangle,
        ToolKind::Ellipse,
        ToolKind::Polygon,
        ToolKind::Pan,
        ToolKind::Select,
        ToolKind::Lasso,
        ToolKind::MagicWand,
        ToolKind::Crop,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ToolKind::Pencil => "Pencil",
            ToolKind::Eraser => "Eraser",
            ToolKind::Fill => "Fill",
            ToolKind::Eyedropper => "Eyedropper",
            ToolKind::Line => "Line",
            ToolKind::Rectangle => "Rectangle",
            ToolKind::FilledRectangle => "Filled Rectangle",
            ToolKind::Ellipse => "Ellipse",
            ToolKind::Polygon => "Polygon",
            ToolKind::Pan => "Pan",
            ToolKind::Select => "Rectangle Select",
            ToolKind::Lasso => "Lasso",
            ToolKind::MagicWand => "Magic Wand",
            ToolKind::Crop => "Crop",
        }
    }

    /// Keyboard shortcut shown in tooltips.
    pub fn shortcut(self) -> &'static str {
        match self {
            ToolKind::Pencil => "B",
            ToolKind::Eraser => "E",
            ToolKind::Fill => "G",
            ToolKind::Eyedropper => "I",
            ToolKind::Line => "L",
            ToolKind::Rectangle => "U",
            ToolKind::FilledRectangle => "Shift+U",
            ToolKind::Ellipse => "K",
            ToolKind::Polygon => "P",
            ToolKind::Pan => "H / Space",
            ToolKind::Select => "M",
            ToolKind::Lasso => "Q",
            ToolKind::MagicWand => "W",
            ToolKind::Crop => "C",
        }
    }

    /// Whether the tool draws with a square brush whose size can be changed.
    pub fn uses_brush_size(self) -> bool {
        matches!(
            self,
            ToolKind::Pencil
                | ToolKind::Eraser
                | ToolKind::Line
                | ToolKind::Rectangle
                | ToolKind::Ellipse
                | ToolKind::Polygon
        )
    }

    /// Whether the tool modifies pixels of the active layer.
    pub fn edits_pixels(self) -> bool {
        !matches!(
            self,
            ToolKind::Eyedropper
                | ToolKind::Pan
                | ToolKind::Select
                | ToolKind::Lasso
                | ToolKind::MagicWand
                | ToolKind::Crop
        )
    }

    /// Whether the tool makes selections (and shares the selection options).
    pub fn selects(self) -> bool {
        matches!(
            self,
            ToolKind::Select | ToolKind::Lasso | ToolKind::MagicWand
        )
    }
}

/// Where the eyedropper reads colors from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleSource {
    /// Only the active layer, including its transparency.
    ActiveLayer,
    /// The composited image of all visible layers.
    Merged,
}

impl SampleSource {
    pub fn name(self) -> &'static str {
        match self {
            SampleSource::ActiveLayer => "Active layer",
            SampleSource::Merged => "Merged image",
        }
    }
}

/// Smallest and largest brush size.
pub const BRUSH_SIZE_RANGE: std::ops::RangeInclusive<u32> = 1..=32;

/// Per-tool options shown in the tool options bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolSettings {
    pub brush_size: u32,
    pub sample_source: SampleSource,
    /// Experimental: 1-px Pencil/Eraser strokes skip the doubled pixel in corners.
    pub pixel_perfect: bool,
    /// How new selections combine with the current one (Shift/Alt override it).
    pub select_mode: crate::core::selection::SelectMode,
    /// Magic wand: largest per-channel difference that still counts as similar.
    pub wand_tolerance: u8,
    /// Magic wand: only connected pixels (otherwise every similar pixel).
    pub wand_contiguous: bool,
    /// Magic wand: compare colors of the active layer or of the merged image.
    pub wand_source: SampleSource,
    /// Brush tip of the drawing tools.
    pub brush_shape: crate::core::raster::BrushShape,
    /// Which pixels the drawing tools and Fill paint.
    pub dither: crate::core::dither::Dither,
    /// Symmetry: also paint mirrored left↔right (around the vertical center line).
    pub mirror_x: bool,
    /// Symmetry: also paint mirrored top↔bottom (around the horizontal center line).
    pub mirror_y: bool,
    /// Ellipse and Polygon: filled instead of outlined.
    pub shape_filled: bool,
    /// Fill: largest per-channel difference that still counts as the same color.
    pub fill_tolerance: u8,
    /// Fill: only the connected area (otherwise every similar pixel).
    pub fill_contiguous: bool,
    /// Fill: which image decides the area (pixels always change on the active layer).
    pub fill_source: SampleSource,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            brush_size: 1,
            sample_source: SampleSource::Merged,
            pixel_perfect: false,
            select_mode: crate::core::selection::SelectMode::Replace,
            wand_tolerance: 0,
            wand_contiguous: true,
            wand_source: SampleSource::ActiveLayer,
            brush_shape: crate::core::raster::BrushShape::Square,
            dither: crate::core::dither::Dither::Solid,
            mirror_x: false,
            mirror_y: false,
            shape_filled: false,
            fill_tolerance: 0,
            fill_contiguous: true,
            fill_source: SampleSource::ActiveLayer,
        }
    }
}

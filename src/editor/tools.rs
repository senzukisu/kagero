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
    Pan,
}

impl ToolKind {
    pub const ALL: [ToolKind; 8] = [
        ToolKind::Pencil,
        ToolKind::Eraser,
        ToolKind::Fill,
        ToolKind::Eyedropper,
        ToolKind::Line,
        ToolKind::Rectangle,
        ToolKind::FilledRectangle,
        ToolKind::Pan,
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
            ToolKind::Pan => "Pan",
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
            ToolKind::Pan => "H / Space",
        }
    }

    /// Whether the tool draws with a square brush whose size can be changed.
    pub fn uses_brush_size(self) -> bool {
        matches!(
            self,
            ToolKind::Pencil | ToolKind::Eraser | ToolKind::Line | ToolKind::Rectangle
        )
    }

    /// Whether the tool modifies pixels of the active layer.
    pub fn edits_pixels(self) -> bool {
        !matches!(self, ToolKind::Eyedropper | ToolKind::Pan)
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
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            brush_size: 1,
            sample_source: SampleSource::Merged,
        }
    }
}

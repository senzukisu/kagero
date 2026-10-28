//! Export choices. All of them are remembered between launches (per format).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::Rgba;

/// Which frames to export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameSelection {
    Current,
    /// Frames selected in the timeline (Ctrl/Shift+click).
    Selected,
    All,
}

impl FrameSelection {
    pub fn label(self) -> &'static str {
        match self {
            FrameSelection::Current => "Current frame",
            FrameSelection::Selected => "Selected frames",
            FrameSelection::All => "All frames",
        }
    }
}

/// What each frame image contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerSource {
    /// All visible layers composited, as on the canvas.
    Visible,
    /// Only the active layer.
    ActiveLayer,
}

impl LayerSource {
    pub fn label(self) -> &'static str {
        match self {
            LayerSource::Visible => "Visible layers (as shown)",
            LayerSource::ActiveLayer => "Active layer only",
        }
    }
}

/// Still-image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StillFormat {
    Png,
    Jpeg,
    WebP,
}

impl StillFormat {
    pub const ALL: [StillFormat; 3] = [StillFormat::Png, StillFormat::Jpeg, StillFormat::WebP];

    pub fn label(self) -> &'static str {
        match self {
            StillFormat::Png => "PNG",
            StillFormat::Jpeg => "JPEG",
            StillFormat::WebP => "WebP (lossless)",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            StillFormat::Png => "png",
            StillFormat::Jpeg => "jpg",
            StillFormat::WebP => "webp",
        }
    }

    pub fn keeps_transparency(self) -> bool {
        !matches!(self, StillFormat::Jpeg)
    }
}

/// JPEG options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct JpegOptions {
    /// 1–100.
    pub quality: u8,
    /// Color painted under transparent pixels (JPEG has no transparency).
    pub background: Rgba,
}

impl Default for JpegOptions {
    fn default() -> Self {
        Self {
            quality: 90,
            background: Rgba::WHITE,
        }
    }
}

/// How a GIF repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GifLoop {
    Forever,
    Once,
}

/// GIF options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GifOptions {
    pub looping: GifLoop,
    /// Pixels with alpha below this become transparent; others become opaque.
    pub alpha_threshold: u8,
    /// When set, semi-transparent pixels that stay visible are blended over this color
    /// first. When `None`, they keep their own color at full opacity (no halos).
    pub matte: Option<Rgba>,
    /// Floyd–Steinberg dithering, used only when colors have to be reduced to 255.
    pub dither: bool,
}

impl Default for GifOptions {
    fn default() -> Self {
        Self {
            looping: GifLoop::Forever,
            alpha_threshold: 128,
            matte: None,
            dither: false,
        }
    }
}

/// How frames are arranged on a sprite sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SheetLayout {
    Horizontal,
    Vertical,
    /// A grid with a fixed number of columns.
    Columns,
    /// A grid with a fixed number of rows.
    Rows,
}

impl SheetLayout {
    pub const ALL: [SheetLayout; 4] = [
        SheetLayout::Horizontal,
        SheetLayout::Vertical,
        SheetLayout::Columns,
        SheetLayout::Rows,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SheetLayout::Horizontal => "Horizontal strip",
            SheetLayout::Vertical => "Vertical strip",
            SheetLayout::Columns => "Grid (fixed columns)",
            SheetLayout::Rows => "Grid (fixed rows)",
        }
    }
}

/// Which JSON is written next to a sprite sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataFormat {
    None,
    /// Kagero's own documented format.
    Kagero,
    /// The widely used "JSON Hash" texture-atlas layout.
    JsonHash,
}

impl MetadataFormat {
    pub fn label(self) -> &'static str {
        match self {
            MetadataFormat::None => "No metadata",
            MetadataFormat::Kagero => "Kagero JSON",
            MetadataFormat::JsonHash => "JSON Hash (atlas)",
        }
    }
}

/// Sprite sheet options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SheetOptions {
    pub layout: SheetLayout,
    /// Used by [`SheetLayout::Columns`].
    pub columns: u32,
    /// Used by [`SheetLayout::Rows`].
    pub rows: u32,
    /// Empty pixels between frames.
    pub spacing: u32,
    /// Empty pixels around the whole sheet.
    pub padding: u32,
    /// Cut away fully transparent borders of each frame (positions go to the metadata).
    pub trim: bool,
    /// Repeat each frame's edge pixels outward (prevents texture bleeding in engines).
    pub extrude: u32,
    /// Fill color of empty areas; `None` keeps them transparent.
    pub background: Option<Rgba>,
    pub image_format: StillFormat,
    pub metadata: MetadataFormat,
}

impl Default for SheetOptions {
    fn default() -> Self {
        Self {
            layout: SheetLayout::Horizontal,
            columns: 4,
            rows: 2,
            spacing: 0,
            padding: 0,
            trim: false,
            extrude: 0,
            background: None,
            image_format: StillFormat::Png,
            metadata: MetadataFormat::Kagero,
        }
    }
}

pub const MAX_SPACING: u32 = 256;
pub const MAX_EXTRUDE: u32 = 16;

/// File naming for exported files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NamingOptions {
    /// Base name; empty means "use the project name".
    pub base_name: String,
    /// Digits in frame numbers (`sprite_0001` = 4).
    pub digits: u8,
    /// Add a number even when only one file is exported.
    pub number_single: bool,
}

impl Default for NamingOptions {
    fn default() -> Self {
        Self {
            base_name: String::new(),
            digits: 4,
            number_single: false,
        }
    }
}

/// Which tab of the export dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportKind {
    /// One image or a numbered sequence of still images.
    Image,
    /// An animated GIF.
    Animation,
    /// A sprite sheet (+ JSON).
    SpriteSheet,
}

/// Everything the export dialog remembers, per kind and format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub kind: ExportKind,
    pub still_format: StillFormat,
    pub frames: FrameSelection,
    pub layers: LayerSource,
    /// Scale per kind/format so e.g. GIF can stay at ×4 while PNG uses ×1.
    pub scale_png: u32,
    pub scale_jpeg: u32,
    pub scale_webp: u32,
    pub scale_gif: u32,
    pub scale_sheet: u32,
    pub jpeg: JpegOptions,
    pub gif: GifOptions,
    pub sheet: SheetOptions,
    pub naming: NamingOptions,
    pub last_dir: Option<PathBuf>,
    pub open_folder_after: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            kind: ExportKind::Image,
            still_format: StillFormat::Png,
            frames: FrameSelection::Current,
            layers: LayerSource::Visible,
            scale_png: 1,
            scale_jpeg: 1,
            scale_webp: 1,
            scale_gif: 4,
            scale_sheet: 1,
            jpeg: JpegOptions::default(),
            gif: GifOptions::default(),
            sheet: SheetOptions::default(),
            naming: NamingOptions::default(),
            last_dir: None,
            open_folder_after: false,
        }
    }
}

impl ExportSettings {
    /// The scale remembered for the current kind and format.
    pub fn scale(&self) -> u32 {
        match self.kind {
            ExportKind::Image => match self.still_format {
                StillFormat::Png => self.scale_png,
                StillFormat::Jpeg => self.scale_jpeg,
                StillFormat::WebP => self.scale_webp,
            },
            ExportKind::Animation => self.scale_gif,
            ExportKind::SpriteSheet => self.scale_sheet,
        }
        .clamp(1, super::MAX_SCALE)
    }

    pub fn scale_mut(&mut self) -> &mut u32 {
        match self.kind {
            ExportKind::Image => match self.still_format {
                StillFormat::Png => &mut self.scale_png,
                StillFormat::Jpeg => &mut self.scale_jpeg,
                StillFormat::WebP => &mut self.scale_webp,
            },
            ExportKind::Animation => &mut self.scale_gif,
            ExportKind::SpriteSheet => &mut self.scale_sheet,
        }
    }

    /// Clamps values loaded from a settings file into their valid ranges.
    pub fn sanitize(&mut self) {
        for s in [
            &mut self.scale_png,
            &mut self.scale_jpeg,
            &mut self.scale_webp,
            &mut self.scale_gif,
            &mut self.scale_sheet,
        ] {
            *s = (*s).clamp(1, super::MAX_SCALE);
        }
        self.jpeg.quality = self.jpeg.quality.clamp(1, 100);
        self.naming.digits = self.naming.digits.clamp(1, 8);
        self.naming.base_name = self.naming.base_name.chars().take(100).collect();
        let s = &mut self.sheet;
        s.columns = s.columns.clamp(1, 1000);
        s.rows = s.rows.clamp(1, 1000);
        s.spacing = s.spacing.min(MAX_SPACING);
        s.padding = s.padding.min(MAX_SPACING);
        s.extrude = s.extrude.min(MAX_EXTRUDE);
        if s.image_format == StillFormat::Jpeg {
            s.image_format = StillFormat::Png;
        }
    }
}

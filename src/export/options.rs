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

/// File format of animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AnimationFormat {
    #[default]
    Gif,
    /// Lossless animated WebP with full transparency.
    WebP,
}

impl AnimationFormat {
    pub fn label(self) -> &'static str {
        match self {
            AnimationFormat::Gif => "GIF",
            AnimationFormat::WebP => "Animated WebP",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            AnimationFormat::Gif => "gif",
            AnimationFormat::WebP => "webp",
        }
    }
}

/// How a GIF (or animated WebP) repeats.
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
    /// Pack trimmed frames as tightly as possible instead of using `layout`'s grid.
    /// (A separate switch, so settings stay readable by older Kagero versions.)
    pub packed: bool,
    /// Packed sheets: identical frames share one spot on the sheet.
    pub merge_duplicates: bool,
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
            packed: false,
            merge_duplicates: true,
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
    /// Export the frames of this tag instead of `frames` (kept separate from
    /// `frames` so older Kagero versions still read these settings).
    pub tag: Option<String>,
    pub layers: LayerSource,
    /// Scale per kind/format so e.g. GIF can stay at ×4 while PNG uses ×1.
    pub scale_png: u32,
    pub scale_jpeg: u32,
    pub scale_webp: u32,
    pub scale_gif: u32,
    pub scale_sheet: u32,
    pub jpeg: JpegOptions,
    pub gif: GifOptions,
    /// GIF or animated WebP (the Animation tab).
    pub animation_format: AnimationFormat,
    pub sheet: SheetOptions,
    pub naming: NamingOptions,
    pub last_dir: Option<PathBuf>,
    pub open_folder_after: bool,
    /// Images and animations: pick the scale so the result is at least
    /// [`AUTO_SCALE_TARGET`] px on its longest side (sprite sheets always use their
    /// own scale).
    pub auto_scale: bool,
}

/// Longest side, in pixels, that "Auto size" scales images and animations up to.
pub const AUTO_SCALE_TARGET: u32 = 512;

/// The smallest whole-number scale that makes a `width`×`height` image at least
/// [`AUTO_SCALE_TARGET`] px on its longest side (1 for images that big already).
pub fn auto_scale_for(width: u32, height: u32) -> u32 {
    let side = width.max(height).max(1);
    AUTO_SCALE_TARGET
        .div_ceil(side)
        .clamp(1, super::MAX_SCALE)
        .min((super::MAX_OUTPUT_SIDE as u32 / side).max(1))
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            kind: ExportKind::Image,
            still_format: StillFormat::Png,
            frames: FrameSelection::Current,
            tag: None,
            layers: LayerSource::Visible,
            scale_png: 1,
            scale_jpeg: 1,
            scale_webp: 1,
            scale_gif: 4,
            scale_sheet: 1,
            jpeg: JpegOptions::default(),
            gif: GifOptions::default(),
            animation_format: AnimationFormat::Gif,
            sheet: SheetOptions::default(),
            naming: NamingOptions::default(),
            last_dir: None,
            open_folder_after: false,
            auto_scale: true,
        }
    }
}

impl ExportSettings {
    /// Whether "Auto size" decides the scale for the current kind.
    pub fn uses_auto_scale(&self) -> bool {
        self.auto_scale && self.kind != ExportKind::SpriteSheet
    }

    /// The scale used for frames of `width`×`height`: automatic or the remembered one.
    pub fn effective_scale(&self, width: u32, height: u32) -> u32 {
        if self.uses_auto_scale() {
            auto_scale_for(width, height)
        } else {
            self.scale()
        }
    }

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

#[cfg(test)]
mod auto_scale_tests {
    use super::*;

    #[test]
    fn small_images_are_enlarged_to_at_least_512_px() {
        assert_eq!(auto_scale_for(10, 10), 52, "10×10 → 520×520");
        assert_eq!(auto_scale_for(32, 32), 16);
        assert_eq!(auto_scale_for(10, 300), 2, "the longest side counts");
        assert_eq!(auto_scale_for(512, 64), 1);
        assert_eq!(auto_scale_for(2048, 2048), 1);
        assert_eq!(auto_scale_for(1, 1), 64, "capped at the largest scale");
        let s = ExportSettings::default();
        assert!(s.uses_auto_scale());
        assert_eq!(s.effective_scale(10, 10), 52);
        let manual = ExportSettings {
            auto_scale: false,
            scale_png: 3,
            ..ExportSettings::default()
        };
        assert_eq!(manual.effective_scale(10, 10), 3);
        let sheet = ExportSettings {
            kind: ExportKind::SpriteSheet,
            ..ExportSettings::default()
        };
        assert_eq!(
            sheet.effective_scale(10, 10),
            1,
            "sheets keep their own scale"
        );
    }
}

//! Polygons, custom brushes and converting pixels to a palette.

use super::{Change, EditError, Editor, PixelChange, Stroke};
use crate::core::{palette, raster, region, Document, PixelBuffer, Rgba};

/// Largest side of a custom brush, in pixels.
pub const MAX_BRUSH_SIDE: u32 = 64;

/// A brush tip made from a selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomBrush {
    /// Pixels relative to the cursor.
    pub offsets: Vec<(i32, i32)>,
    pub width: u32,
    pub height: u32,
}

/// Which pixels Convert to Palette changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaletteScope {
    /// The active layer in the current frame.
    #[default]
    LayerFrame,
    /// The active layer in every frame.
    Layer,
    /// Every unlocked layer in every frame.
    All,
}

impl PaletteScope {
    pub const ALL: [PaletteScope; 3] = [
        PaletteScope::LayerFrame,
        PaletteScope::Layer,
        PaletteScope::All,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PaletteScope::LayerFrame => "Active layer, this frame",
            PaletteScope::Layer => "Active layer, all frames",
            PaletteScope::All => "All unlocked layers, all frames",
        }
    }
}

impl Editor {
    /// Polygon tool: adds a corner, or starts a polygon. Clicking the last corner
    /// again (or double-clicking) finishes it.
    pub fn polygon_click(
        &mut self,
        pos: (i32, i32),
        slot: super::ColorSlot,
    ) -> Result<(), EditError> {
        match &mut self.stroke {
            Some(stroke) if stroke.tool == super::ToolKind::Polygon => {
                if stroke.corners.last() == Some(&pos) {
                    self.end_stroke();
                } else {
                    stroke.corners.push(pos);
                    self.update_stroke(pos);
                }
                Ok(())
            }
            _ => self.begin_stroke(pos, slot),
        }
    }

    /// Whether a polygon is being drawn.
    pub fn is_drawing_polygon(&self) -> bool {
        self.stroke
            .as_ref()
            .is_some_and(|s| s.tool == super::ToolKind::Polygon)
    }

    /// Redraws the polygon preview: its corners plus `hover` (the cursor), closed.
    pub(super) fn draw_polygon(&mut self, stroke: &mut Stroke, hover: Option<(i32, i32)>) {
        self.restore(stroke);
        let mut corners = stroke.corners.clone();
        if let Some(h) = hover {
            if corners.last() != Some(&h) {
                corners.push(h);
            }
        }
        let (w, h) = (self.doc.width(), self.doc.height());
        if self.settings.shape_filled && corners.len() >= 3 {
            if let Some((r, bits)) = region::polygon_mask(&corners, w, h) {
                let dot = [(0, 0)];
                for (i, on) in bits.iter().enumerate() {
                    if *on {
                        let (x, y) = (r.x + (i as u32 % r.w) as i32, r.y + (i as u32 / r.w) as i32);
                        self.stamp(stroke, x, y, &dot);
                    }
                }
                return;
            }
        }
        let tip = self.brush_tip();
        let mut points = Vec::new();
        let n = corners.len();
        for i in 0..n {
            // Close the shape only once it has an inside.
            if i + 1 == n && n < 3 {
                break;
            }
            raster::line(corners[i], corners[(i + 1) % n], |x, y| points.push((x, y)));
        }
        if n == 1 {
            points.push(corners[0]);
        }
        for (x, y) in points {
            self.stamp(stroke, x, y, &tip);
        }
    }

    /// Makes a brush tip from the selection: its pixels that have color on the
    /// active layer (or the whole shape when it covers only transparent pixels).
    pub fn brush_from_selection(&mut self) -> Result<(), EditError> {
        let sel = self.doc.selection_shape().ok_or(EditError::NoSelection)?;
        let r = sel.bounds();
        if r.w > MAX_BRUSH_SIDE || r.h > MAX_BRUSH_SIDE {
            return Err(EditError::NotPossible(
                "Brushes can be at most 64×64 pixels — select a smaller area",
            ));
        }
        let cel = self.doc.active_cel();
        let (cx, cy) = (r.x + (r.w as i32 - 1) / 2, r.y + (r.h as i32 - 1) / 2);
        let pixels = |need_color: bool| -> Vec<(i32, i32)> {
            let mut v = Vec::new();
            for y in r.y..r.y + r.h as i32 {
                for x in r.x..r.x + r.w as i32 {
                    let colored = cel.get(x, y).is_some_and(|c| c.a() != 0);
                    if sel.contains(x, y) && (colored || !need_color) {
                        v.push((x - cx, y - cy));
                    }
                }
            }
            v
        };
        let mut offsets = pixels(true);
        if offsets.is_empty() {
            offsets = pixels(false);
        }
        self.custom_brush = Some(CustomBrush {
            offsets,
            width: r.w,
            height: r.h,
        });
        self.settings.brush_shape = raster::BrushShape::Custom;
        Ok(())
    }

    /// Replaces the active layer's pixels in the current frame (e.g. with a plugin's
    /// result) as one undo step. Returns whether anything changed.
    pub fn replace_active_pixels(
        &mut self,
        label: &'static str,
        rgba: Vec<u8>,
    ) -> Result<bool, EditError> {
        let after = PixelBuffer::from_raw(self.doc.width(), self.doc.height(), rgba)?;
        self.edit_region(label, None, move |cel| *cel = after)
    }

    /// Cels (frame index, layer index) that Convert to Palette would change.
    fn palette_cels(&self, scope: PaletteScope) -> Vec<(usize, usize)> {
        let doc = &self.doc;
        let frames = 0..doc.frame_count();
        let active_layer = doc.active_index();
        let active_frame = doc.active_frame_index();
        match scope {
            PaletteScope::LayerFrame => vec![(active_frame, active_layer)],
            PaletteScope::Layer => frames.map(|f| (f, active_layer)).collect(),
            PaletteScope::All => frames
                .flat_map(|f| (0..doc.layer_count()).map(move |l| (f, l)))
                .filter(|&(_, l)| !doc.layers()[l].locked)
                .collect(),
        }
    }

    /// Replaces every colored pixel in `scope` with the nearest of `colors` (alpha
    /// kept), as one undo step. Returns how many pixels changed.
    pub fn convert_to_palette(
        &mut self,
        colors: &[Rgba],
        scope: PaletteScope,
    ) -> Result<usize, EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        if !colors.iter().any(|c| c.a() != 0) {
            return Err(EditError::NotPossible("The palette has no opaque colors"));
        }
        if scope != PaletteScope::All && self.doc.active_layer().locked {
            return Err(EditError::LockedLayer);
        }
        self.end_stroke();
        let mut group = Vec::new();
        let mut count = 0;
        for (f, l) in self.palette_cels(scope) {
            let (Some(cel), Some(frame)) = (self.doc.cel(f, l), self.doc.frames().get(f)) else {
                continue;
            };
            let changes: Vec<PixelChange> = cel
                .as_bytes()
                .chunks_exact(4)
                .enumerate()
                .filter_map(|(i, p)| {
                    let before = Rgba([p[0], p[1], p[2], p[3]]);
                    let after = palette::nearest(colors, before);
                    (after != before).then_some(PixelChange {
                        index: i as u32,
                        before,
                        after,
                    })
                })
                .collect();
            if changes.is_empty() {
                continue;
            }
            count += changes.len();
            group.push(Change::Pixels {
                frame: frame.id,
                layer: self.doc.layers()[l].id,
                changes,
            });
        }
        if group.is_empty() {
            return Ok(0);
        }
        self.execute(Change::Group {
            label: "Convert to Palette",
            changes: group,
        })?;
        Ok(count)
    }

    /// The current frame as it looks now and as it would look after Convert to
    /// Palette (composited exactly like the canvas).
    pub fn palette_preview(
        &self,
        colors: &[Rgba],
        scope: PaletteScope,
    ) -> Result<(PixelBuffer, PixelBuffer), EditError> {
        let doc = &self.doc;
        let frame = doc.active_frame_index();
        let before = doc.composite_frame(frame)?;
        let cels = self.palette_cels(scope);
        let layers = (0..doc.layer_count())
            .map(|l| {
                let info = &doc.layers()[l];
                let mut cel = doc
                    .cel(frame, l)
                    .cloned()
                    .unwrap_or(PixelBuffer::new(doc.width(), doc.height())?);
                if cels.contains(&(frame, l)) {
                    let bytes: Vec<u8> = cel
                        .as_bytes()
                        .chunks_exact(4)
                        .flat_map(|p| palette::nearest(colors, Rgba([p[0], p[1], p[2], p[3]])).0)
                        .collect();
                    cel = PixelBuffer::from_raw(doc.width(), doc.height(), bytes)?;
                }
                Ok((info.name.clone(), info.visible, cel))
            })
            .collect::<Result<Vec<_>, crate::core::CoreError>>()?;
        let mut preview =
            Document::from_layers(doc.width(), doc.height(), layers, doc.active_index())?;
        let ids: Vec<_> = preview.layers().iter().map(|l| l.id).collect();
        for (id, info) in ids.into_iter().zip(doc.layers()) {
            if let Some(l) = preview.layer_mut(id) {
                l.opacity = info.opacity;
            }
        }
        let after = preview.composite_frame(0)?;
        Ok((before, after))
    }
}

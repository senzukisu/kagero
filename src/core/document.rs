use super::animation::{Frame, FrameId, DEFAULT_FRAME_DURATION_MS};
use super::selection::SelRect;
use super::{
    layers_bytes, validate_size, CoreError, PixelBuffer, Rgba, MAX_DOCUMENT_BYTES, MAX_FRAMES,
    MAX_LAYERS,
};

/// A pixel rectangle as `(x, y, width, height)`.
pub type PixelRect = (u32, u32, u32, u32);

/// Stable identifier of a layer inside one document (survives reordering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(pub u64);

/// Layer properties shared by all frames. Pixels live in each frame's cels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerInfo {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
}

/// A layer taken out of the document: its properties plus its image in every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerData {
    pub info: LayerInfo,
    /// One image per frame, in frame order at the time of removal.
    pub cels: Vec<(FrameId, PixelBuffer)>,
}

impl LayerData {
    pub fn byte_size(&self) -> usize {
        self.cels
            .iter()
            .map(|(_, c)| c.as_bytes().len())
            .sum::<usize>()
            + self.info.name.len()
    }
}

/// An animated image: layers (bottom to top) shared by all frames, and frames that hold
/// one image per layer.
///
/// The low-level mutators here do no history bookkeeping; the editor layer wraps them
/// in undoable commands. Invariants: at least one layer and one frame, every frame has
/// exactly one cel per layer, all cels have the document size.
#[derive(Debug, Clone)]
pub struct Document {
    width: u32,
    height: u32,
    layers: Vec<LayerInfo>,
    frames: Vec<Frame>,
    active_layer: usize,
    active_frame: usize,
    next_layer_id: u64,
    next_frame_id: u64,
    /// Bumped when layer order or visibility changes (affects every frame's look).
    structure_revision: u64,
    /// Rectangular selection (editor state, not saved in project files).
    selection: Option<SelRect>,
}

impl Document {
    /// A new document with one frame and one layer filled with `background`.
    pub fn new(width: u32, height: u32, background: Rgba) -> Result<Self, CoreError> {
        validate_size(width, height)?;
        let pixels = PixelBuffer::filled(width, height, background)?;
        Ok(Self {
            width,
            height,
            layers: vec![LayerInfo {
                id: LayerId(1),
                name: "Background".to_owned(),
                visible: true,
            }],
            frames: vec![Frame::new(
                FrameId(1),
                DEFAULT_FRAME_DURATION_MS,
                vec![pixels],
            )],
            active_layer: 0,
            active_frame: 0,
            next_layer_id: 2,
            next_frame_id: 2,
            structure_revision: 0,
            selection: None,
        })
    }

    /// A single-frame document from already-decoded layers (bottom to top).
    pub fn from_layers(
        width: u32,
        height: u32,
        layers: Vec<(String, bool, PixelBuffer)>,
        active: usize,
    ) -> Result<Self, CoreError> {
        let mut infos = Vec::with_capacity(layers.len());
        let mut cels = Vec::with_capacity(layers.len());
        for (name, visible, pixels) in layers {
            infos.push((name, visible));
            cels.push(pixels);
        }
        Self::from_parts(
            width,
            height,
            infos,
            vec![(DEFAULT_FRAME_DURATION_MS, cels)],
            active,
            0,
        )
    }

    /// Builds a document from decoded parts, validating every invariant.
    ///
    /// `layers` are `(name, visible)` bottom to top; each frame is `(duration, cels)`
    /// with one cel per layer.
    pub fn from_parts(
        width: u32,
        height: u32,
        layers: Vec<(String, bool)>,
        frames: Vec<(u32, Vec<PixelBuffer>)>,
        active_layer: usize,
        active_frame: usize,
    ) -> Result<Self, CoreError> {
        validate_size(width, height)?;
        if layers.is_empty() {
            return Err(CoreError::LastLayer);
        }
        if frames.is_empty() {
            return Err(CoreError::LastFrame);
        }
        if layers.len() > MAX_LAYERS {
            return Err(CoreError::TooManyLayers);
        }
        if frames.len() > MAX_FRAMES {
            return Err(CoreError::TooManyFrames);
        }
        if layers_bytes(width, height, layers.len() * frames.len()) > MAX_DOCUMENT_BYTES {
            return Err(CoreError::MemoryBudget);
        }
        let infos: Vec<LayerInfo> = layers
            .into_iter()
            .enumerate()
            .map(|(i, (name, visible))| LayerInfo {
                id: LayerId(i as u64 + 1),
                name,
                visible,
            })
            .collect();
        let mut out_frames = Vec::with_capacity(frames.len());
        for (i, (duration, cels)) in frames.into_iter().enumerate() {
            if cels.len() != infos.len() {
                return Err(CoreError::CelCount {
                    expected: infos.len(),
                    actual: cels.len(),
                });
            }
            if let Some(bad) = cels
                .iter()
                .find(|c| c.width() != width || c.height() != height)
            {
                return Err(CoreError::InvalidSize {
                    width: bad.width(),
                    height: bad.height(),
                });
            }
            out_frames.push(Frame::new(FrameId(i as u64 + 1), duration, cels));
        }
        Ok(Self {
            width,
            height,
            active_layer: active_layer.min(infos.len() - 1),
            active_frame: active_frame.min(out_frames.len() - 1),
            next_layer_id: infos.len() as u64 + 1,
            next_frame_id: out_frames.len() as u64 + 1,
            layers: infos,
            frames: out_frames,
            structure_revision: 0,
            selection: None,
        })
    }

    /// Replaces the sequential ids from [`Document::from_parts`] with stored ones
    /// (e.g. from a project file). Ids must be non-zero, unique and one per item.
    pub fn restore_ids(&mut self, layer_ids: &[u64], frame_ids: &[u64]) -> Result<(), CoreError> {
        fn valid(ids: &[u64], len: usize) -> bool {
            let mut sorted = ids.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            ids.len() == len && sorted.len() == len && !ids.contains(&0)
        }
        if !valid(layer_ids, self.layers.len()) {
            return Err(CoreError::LayerIndex(layer_ids.len()));
        }
        if !valid(frame_ids, self.frames.len()) {
            return Err(CoreError::FrameIndex(frame_ids.len()));
        }
        for (layer, &id) in self.layers.iter_mut().zip(layer_ids) {
            layer.id = LayerId(id);
        }
        for (frame, &id) in self.frames.iter_mut().zip(frame_ids) {
            frame.id = FrameId(id);
        }
        self.next_layer_id = layer_ids.iter().max().map_or(1, |m| m.saturating_add(1));
        self.next_frame_id = frame_ids.iter().max().map_or(1, |m| m.saturating_add(1));
        Ok(())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    // ------------------------------------------------------------ selection

    /// The current selection, always clipped to the image.
    pub fn selection(&self) -> Option<SelRect> {
        self.selection
    }

    /// Sets (and clips) the selection. `None` or an empty result deselects.
    pub fn set_selection(&mut self, rect: Option<SelRect>) {
        self.selection = rect.and_then(|r| r.clip(self.width, self.height));
    }

    // ------------------------------------------------------------ layers

    /// Layers bottom to top.
    pub fn layers(&self) -> &[LayerInfo] {
        &self.layers
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Index of the active layer.
    pub fn active_index(&self) -> usize {
        self.active_layer
    }

    pub fn set_active_index(&mut self, index: usize) {
        if index < self.layers.len() {
            self.active_layer = index;
        }
    }

    pub fn active_layer(&self) -> &LayerInfo {
        // `active_layer` is kept in range by every mutator, and there is always one layer.
        &self.layers[self.active_layer.min(self.layers.len() - 1)]
    }

    pub fn index_of(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn layer(&self, id: LayerId) -> Option<&LayerInfo> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Mutable layer properties. Marks the document structure as changed.
    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut LayerInfo> {
        self.structure_revision = self.structure_revision.wrapping_add(1);
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Changes whenever layer order or visibility changes.
    pub fn structure_revision(&self) -> u64 {
        self.structure_revision
    }

    fn allocate_layer_id(&mut self) -> LayerId {
        let id = LayerId(self.next_layer_id);
        self.next_layer_id += 1;
        id
    }

    fn check_budget(&self, layers: usize, frames: usize) -> Result<(), CoreError> {
        if layers_bytes(self.width, self.height, layers * frames) > MAX_DOCUMENT_BYTES {
            return Err(CoreError::MemoryBudget);
        }
        Ok(())
    }

    /// Checks whether one more layer fits into the layer and memory limits.
    pub fn can_add_layer(&self) -> Result<(), CoreError> {
        if self.layers.len() >= MAX_LAYERS {
            return Err(CoreError::TooManyLayers);
        }
        self.check_budget(self.layers.len() + 1, self.frames.len())
    }

    /// Creates an empty transparent layer for every frame (not inserted yet).
    pub fn make_empty_layer(&mut self, name: String) -> Result<LayerData, CoreError> {
        self.can_add_layer()?;
        let mut cels = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            cels.push((frame.id, PixelBuffer::new(self.width, self.height)?));
        }
        Ok(LayerData {
            info: LayerInfo {
                id: self.allocate_layer_id(),
                name,
                visible: true,
            },
            cels,
        })
    }

    /// Checks that `layer` could be inserted at `index` without changing anything.
    pub fn check_insert(&self, index: usize, layer: &LayerData) -> Result<(), CoreError> {
        if index > self.layers.len() {
            return Err(CoreError::LayerIndex(index));
        }
        if layer.cels.len() != self.frames.len() {
            return Err(CoreError::CelCount {
                expected: self.frames.len(),
                actual: layer.cels.len(),
            });
        }
        for frame in &self.frames {
            let cel = layer
                .cels
                .iter()
                .find(|(id, _)| *id == frame.id)
                .map(|(_, c)| c)
                .ok_or(CoreError::FrameMissing)?;
            if cel.width() != self.width || cel.height() != self.height {
                return Err(CoreError::InvalidSize {
                    width: cel.width(),
                    height: cel.height(),
                });
            }
        }
        self.can_add_layer()
    }

    /// Inserts a layer at `index` (0 = bottom) in every frame and makes it active.
    pub fn insert_layer(&mut self, index: usize, layer: LayerData) -> Result<(), CoreError> {
        self.check_insert(index, &layer)?;
        let LayerData { info, mut cels } = layer;
        self.next_layer_id = self.next_layer_id.max(info.id.0 + 1);
        for frame in &mut self.frames {
            if let Some(pos) = cels.iter().position(|(id, _)| *id == frame.id) {
                let (_, cel) = cels.swap_remove(pos);
                frame.cels.insert(index, cel);
                frame.touch();
            }
        }
        self.layers.insert(index, info);
        self.active_layer = index;
        self.structure_revision = self.structure_revision.wrapping_add(1);
        Ok(())
    }

    /// Removes the layer at `index` from every frame. The last layer cannot be removed.
    pub fn remove_layer(&mut self, index: usize) -> Result<LayerData, CoreError> {
        if self.layers.len() <= 1 {
            return Err(CoreError::LastLayer);
        }
        if index >= self.layers.len() {
            return Err(CoreError::LayerIndex(index));
        }
        let info = self.layers.remove(index);
        let mut cels = Vec::with_capacity(self.frames.len());
        for frame in &mut self.frames {
            cels.push((frame.id, frame.cels.remove(index)));
            frame.touch();
        }
        if self.active_layer >= self.layers.len() || self.active_layer > index {
            self.active_layer = self.active_layer.saturating_sub(1);
        }
        self.active_layer = self.active_layer.min(self.layers.len() - 1);
        self.structure_revision = self.structure_revision.wrapping_add(1);
        Ok(LayerData { info, cels })
    }

    /// Moves a layer from one stack position to another; the active layer follows it.
    pub fn move_layer(&mut self, from: usize, to: usize) -> Result<(), CoreError> {
        let len = self.layers.len();
        if from >= len {
            return Err(CoreError::LayerIndex(from));
        }
        if to >= len {
            return Err(CoreError::LayerIndex(to));
        }
        let active_id = self.active_layer().id;
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        for frame in &mut self.frames {
            let cel = frame.cels.remove(from);
            frame.cels.insert(to, cel);
            frame.touch();
        }
        self.active_layer = self.index_of(active_id).unwrap_or(0);
        self.structure_revision = self.structure_revision.wrapping_add(1);
        Ok(())
    }

    // ------------------------------------------------------------ frames

    /// Frames in playback order.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn active_frame_index(&self) -> usize {
        self.active_frame
    }

    pub fn active_frame(&self) -> &Frame {
        &self.frames[self.active_frame.min(self.frames.len() - 1)]
    }

    /// Switches the canvas to another frame (not an undo step). Out-of-range is ignored.
    pub fn set_active_frame(&mut self, index: usize) {
        if index < self.frames.len() {
            self.active_frame = index;
        }
    }

    pub fn frame_index_of(&self, id: FrameId) -> Option<usize> {
        self.frames.iter().position(|f| f.id == id)
    }

    pub fn frame_mut(&mut self, id: FrameId) -> Option<&mut Frame> {
        self.frames.iter_mut().find(|f| f.id == id)
    }

    fn allocate_frame_id(&mut self) -> FrameId {
        let id = FrameId(self.next_frame_id);
        self.next_frame_id += 1;
        id
    }

    /// Checks whether one more frame fits into the frame and memory limits.
    pub fn can_add_frame(&self) -> Result<(), CoreError> {
        if self.frames.len() >= MAX_FRAMES {
            return Err(CoreError::TooManyFrames);
        }
        self.check_budget(self.layers.len(), self.frames.len() + 1)
    }

    /// A new frame with transparent cels for every layer (not inserted yet).
    pub fn make_empty_frame(&mut self, duration_ms: u32) -> Result<Frame, CoreError> {
        self.can_add_frame()?;
        let mut cels = Vec::with_capacity(self.layers.len());
        for _ in &self.layers {
            cels.push(PixelBuffer::new(self.width, self.height)?);
        }
        Ok(Frame::new(self.allocate_frame_id(), duration_ms, cels))
    }

    /// An independent deep copy of the frame at `index` with a new id (not inserted yet).
    pub fn make_duplicate_frame(&mut self, index: usize) -> Result<Frame, CoreError> {
        self.can_add_frame()?;
        let source = self.frames.get(index).ok_or(CoreError::FrameIndex(index))?;
        let (duration, cels) = (source.duration_ms(), source.cels.clone());
        Ok(Frame::new(self.allocate_frame_id(), duration, cels))
    }

    /// Checks that `frame` could be inserted at `index` without changing anything.
    pub fn check_insert_frame(&self, index: usize, frame: &Frame) -> Result<(), CoreError> {
        if index > self.frames.len() {
            return Err(CoreError::FrameIndex(index));
        }
        if frame.cels.len() != self.layers.len() {
            return Err(CoreError::CelCount {
                expected: self.layers.len(),
                actual: frame.cels.len(),
            });
        }
        if let Some(bad) = frame
            .cels
            .iter()
            .find(|c| c.width() != self.width || c.height() != self.height)
        {
            return Err(CoreError::InvalidSize {
                width: bad.width(),
                height: bad.height(),
            });
        }
        self.can_add_frame()
    }

    /// Inserts a frame at `index` and makes it active.
    pub fn insert_frame(&mut self, index: usize, mut frame: Frame) -> Result<(), CoreError> {
        self.check_insert_frame(index, &frame)?;
        self.next_frame_id = self.next_frame_id.max(frame.id.0 + 1);
        frame.touch();
        self.frames.insert(index, frame);
        self.active_frame = index;
        Ok(())
    }

    /// Removes the frame at `index`. The last remaining frame cannot be removed.
    pub fn remove_frame(&mut self, index: usize) -> Result<Frame, CoreError> {
        if self.frames.len() <= 1 {
            return Err(CoreError::LastFrame);
        }
        if index >= self.frames.len() {
            return Err(CoreError::FrameIndex(index));
        }
        let frame = self.frames.remove(index);
        if self.active_frame > index || self.active_frame >= self.frames.len() {
            self.active_frame = self.active_frame.saturating_sub(1);
        }
        self.active_frame = self.active_frame.min(self.frames.len() - 1);
        Ok(frame)
    }

    /// Moves a frame to another position; the active frame follows its content.
    pub fn move_frame(&mut self, from: usize, to: usize) -> Result<(), CoreError> {
        let len = self.frames.len();
        if from >= len {
            return Err(CoreError::FrameIndex(from));
        }
        if to >= len {
            return Err(CoreError::FrameIndex(to));
        }
        let active_id = self.active_frame().id;
        let frame = self.frames.remove(from);
        self.frames.insert(to, frame);
        self.active_frame = self.frame_index_of(active_id).unwrap_or(0);
        Ok(())
    }

    // ------------------------------------------------------------ cels

    /// Image of layer `layer` in frame `frame` (indices).
    pub fn cel(&self, frame: usize, layer: usize) -> Option<&PixelBuffer> {
        self.frames.get(frame)?.cel(layer)
    }

    /// Image of the active layer in the active frame.
    pub fn active_cel(&self) -> &PixelBuffer {
        let frame = self.active_frame();
        // Every frame has one cel per layer and `active_layer` is kept in range.
        &frame.cels[self.active_layer.min(frame.cels.len() - 1)]
    }

    /// Image of layer `layer` in the active frame.
    pub fn active_frame_cel(&self, layer: usize) -> Option<&PixelBuffer> {
        self.active_frame().cel(layer)
    }

    pub fn cel_by_id(&self, frame: FrameId, layer: LayerId) -> Option<&PixelBuffer> {
        let li = self.index_of(layer)?;
        self.frames.iter().find(|f| f.id == frame)?.cel(li)
    }

    /// Mutable image of a layer in a frame; marks that frame as changed.
    pub fn cel_by_id_mut(&mut self, frame: FrameId, layer: LayerId) -> Option<&mut PixelBuffer> {
        let li = self.index_of(layer)?;
        self.frames.iter_mut().find(|f| f.id == frame)?.cel_mut(li)
    }

    // ------------------------------------------------------------ compositing

    /// Color of one pixel of the active frame after compositing all visible layers.
    pub fn composite_pixel(&self, x: i32, y: i32) -> Option<Rgba> {
        self.composite_frame_pixel(self.active_frame, x, y)
    }

    /// Color of one pixel of frame `frame` after compositing all visible layers.
    pub fn composite_frame_pixel(&self, frame: usize, x: i32, y: i32) -> Option<Rgba> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        let frame = self.frames.get(frame)?;
        let mut out = Rgba::TRANSPARENT;
        for (info, cel) in self.layers.iter().zip(&frame.cels) {
            if info.visible {
                if let Some(px) = cel.get(x, y) {
                    out = px.over(out);
                }
            }
        }
        Some(out)
    }

    /// Composites the active frame inside a rectangle into RGBA bytes (row-major).
    pub fn composite_rect(&self, x: u32, y: u32, w: u32, h: u32) -> Option<(PixelRect, Vec<u8>)> {
        self.composite_frame_rect(self.active_frame, x, y, w, h)
    }

    /// Composites visible layers of frame `frame` inside the given rectangle.
    ///
    /// The rectangle is clipped to the document; the returned tuple is the clipped rect.
    pub fn composite_frame_rect(
        &self,
        frame: usize,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Option<(PixelRect, Vec<u8>)> {
        if x >= self.width || y >= self.height || w == 0 || h == 0 {
            return None;
        }
        let frame = self.frames.get(frame)?;
        let w = w.min(self.width - x);
        let h = h.min(self.height - y);
        let mut out = vec![0u8; w as usize * h as usize * 4];
        let visible: Vec<&PixelBuffer> = self
            .layers
            .iter()
            .zip(&frame.cels)
            .filter(|(info, _)| info.visible)
            .map(|(_, cel)| cel)
            .collect();
        for row in 0..h {
            let src_row = (y + row) as usize * self.width as usize;
            for col in 0..w {
                let src_index = src_row + (x + col) as usize;
                let mut px = Rgba::TRANSPARENT;
                for cel in &visible {
                    if let Some(c) = cel.get_index(src_index as u32) {
                        px = c.over(px);
                    }
                }
                let dst = (row as usize * w as usize + col as usize) * 4;
                out[dst..dst + 4].copy_from_slice(&px.0);
            }
        }
        Some(((x, y, w, h), out))
    }

    /// The fully composited active frame.
    pub fn composite(&self) -> Result<PixelBuffer, CoreError> {
        self.composite_frame(self.active_frame)
    }

    /// The fully composited frame at `frame`.
    pub fn composite_frame(&self, frame: usize) -> Result<PixelBuffer, CoreError> {
        let bytes = self
            .composite_frame_rect(frame, 0, 0, self.width, self.height)
            .map(|(_, bytes)| bytes)
            .ok_or(CoreError::FrameIndex(frame))?;
        PixelBuffer::from_raw(self.width, self.height, bytes)
    }
}

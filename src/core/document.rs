use super::{
    layers_bytes, validate_size, CoreError, PixelBuffer, Rgba, MAX_DOCUMENT_BYTES, MAX_LAYERS,
};

/// A pixel rectangle as `(x, y, width, height)`.
pub type PixelRect = (u32, u32, u32, u32);

/// Stable identifier of a layer inside one document (survives reordering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(pub u64);

/// One raster layer with its own pixel data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub pixels: PixelBuffer,
}

/// An image made of layers, stored bottom to top.
///
/// The low-level mutators here do no history bookkeeping; the editor layer wraps them
/// in undoable commands.
#[derive(Debug, Clone)]
pub struct Document {
    width: u32,
    height: u32,
    layers: Vec<Layer>,
    active: usize,
    next_id: u64,
}

impl Document {
    /// A new document with a single layer filled with `background`.
    pub fn new(width: u32, height: u32, background: Rgba) -> Result<Self, CoreError> {
        validate_size(width, height)?;
        let pixels = PixelBuffer::filled(width, height, background)?;
        Ok(Self {
            width,
            height,
            layers: vec![Layer {
                id: LayerId(1),
                name: "Background".to_owned(),
                visible: true,
                pixels,
            }],
            active: 0,
            next_id: 2,
        })
    }

    /// Builds a document from already-decoded layers (bottom to top), e.g. when loading.
    pub fn from_layers(
        width: u32,
        height: u32,
        layers: Vec<(String, bool, PixelBuffer)>,
        active: usize,
    ) -> Result<Self, CoreError> {
        validate_size(width, height)?;
        if layers.is_empty() {
            return Err(CoreError::LastLayer);
        }
        if layers.len() > MAX_LAYERS {
            return Err(CoreError::TooManyLayers);
        }
        if layers_bytes(width, height, layers.len()) > MAX_DOCUMENT_BYTES {
            return Err(CoreError::MemoryBudget);
        }
        let mut out = Vec::with_capacity(layers.len());
        for (i, (name, visible, pixels)) in layers.into_iter().enumerate() {
            if pixels.width() != width || pixels.height() != height {
                return Err(CoreError::InvalidSize {
                    width: pixels.width(),
                    height: pixels.height(),
                });
            }
            out.push(Layer {
                id: LayerId(i as u64 + 1),
                name,
                visible,
                pixels,
            });
        }
        let next_id = out.len() as u64 + 1;
        let active = active.min(out.len() - 1);
        Ok(Self {
            width,
            height,
            layers: out,
            active,
            next_id,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Layers bottom to top.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn set_active_index(&mut self, index: usize) {
        if index < self.layers.len() {
            self.active = index;
        }
    }

    pub fn active_layer(&self) -> &Layer {
        // `active` is kept in range by every mutator, and there is always one layer.
        &self.layers[self.active.min(self.layers.len() - 1)]
    }

    pub fn index_of(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Allocates an id for a new layer.
    pub fn next_layer_id(&mut self) -> LayerId {
        let id = LayerId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Checks whether one more layer fits into the layer and memory limits.
    pub fn can_add_layer(&self) -> Result<(), CoreError> {
        if self.layers.len() >= MAX_LAYERS {
            return Err(CoreError::TooManyLayers);
        }
        if layers_bytes(self.width, self.height, self.layers.len() + 1) > MAX_DOCUMENT_BYTES {
            return Err(CoreError::MemoryBudget);
        }
        Ok(())
    }

    /// Creates an empty transparent layer (not inserted yet).
    pub fn make_empty_layer(&mut self, name: String) -> Result<Layer, CoreError> {
        self.can_add_layer()?;
        let pixels = PixelBuffer::new(self.width, self.height)?;
        Ok(Layer {
            id: self.next_layer_id(),
            name,
            visible: true,
            pixels,
        })
    }

    /// Inserts a layer at `index` (0 = bottom) and makes it active.
    pub fn insert_layer(&mut self, index: usize, layer: Layer) -> Result<(), CoreError> {
        self.check_insert(index, &layer)?;
        self.next_id = self.next_id.max(layer.id.0 + 1);
        self.layers.insert(index, layer);
        self.active = index;
        Ok(())
    }

    /// Checks that `layer` could be inserted at `index` without changing anything.
    pub fn check_insert(&self, index: usize, layer: &Layer) -> Result<(), CoreError> {
        if index > self.layers.len() {
            return Err(CoreError::LayerIndex(index));
        }
        if layer.pixels.width() != self.width || layer.pixels.height() != self.height {
            return Err(CoreError::InvalidSize {
                width: layer.pixels.width(),
                height: layer.pixels.height(),
            });
        }
        self.can_add_layer()
    }

    /// Removes the layer at `index`. The last remaining layer cannot be removed.
    pub fn remove_layer(&mut self, index: usize) -> Result<Layer, CoreError> {
        if self.layers.len() <= 1 {
            return Err(CoreError::LastLayer);
        }
        if index >= self.layers.len() {
            return Err(CoreError::LayerIndex(index));
        }
        let layer = self.layers.remove(index);
        if self.active >= self.layers.len() || self.active > index {
            self.active = self.active.saturating_sub(1);
        }
        self.active = self.active.min(self.layers.len() - 1);
        Ok(layer)
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
        self.active = self.index_of(active_id).unwrap_or(0);
        Ok(())
    }

    /// The color of one pixel after compositing all visible layers.
    pub fn composite_pixel(&self, x: i32, y: i32) -> Option<Rgba> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        let mut out = Rgba::TRANSPARENT;
        for layer in self.layers.iter().filter(|l| l.visible) {
            if let Some(px) = layer.pixels.get(x, y) {
                out = px.over(out);
            }
        }
        Some(out)
    }

    /// Composites visible layers inside the given rectangle into RGBA bytes (row-major).
    ///
    /// The rectangle is clipped to the document; the returned tuple is the clipped rect.
    pub fn composite_rect(&self, x: u32, y: u32, w: u32, h: u32) -> Option<(PixelRect, Vec<u8>)> {
        if x >= self.width || y >= self.height || w == 0 || h == 0 {
            return None;
        }
        let w = w.min(self.width - x);
        let h = h.min(self.height - y);
        let mut out = vec![0u8; w as usize * h as usize * 4];
        let visible: Vec<&Layer> = self.layers.iter().filter(|l| l.visible).collect();
        for row in 0..h {
            let src_row = (y + row) as usize * self.width as usize;
            for col in 0..w {
                let src_index = src_row + (x + col) as usize;
                let mut px = Rgba::TRANSPARENT;
                for layer in &visible {
                    if let Some(c) = layer.pixels.get_index(src_index as u32) {
                        px = c.over(px);
                    }
                }
                let dst = (row as usize * w as usize + col as usize) * 4;
                out[dst..dst + 4].copy_from_slice(&px.0);
            }
        }
        Some(((x, y, w, h), out))
    }

    /// The fully composited image.
    pub fn composite(&self) -> Result<PixelBuffer, CoreError> {
        let bytes = self
            .composite_rect(0, 0, self.width, self.height)
            .map(|(_, bytes)| bytes)
            .unwrap_or_default();
        PixelBuffer::from_raw(self.width, self.height, bytes)
    }
}

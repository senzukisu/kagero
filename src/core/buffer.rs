use super::{validate_size, CoreError, Rgba};

/// A rectangular RGBA8 pixel buffer, rows top to bottom.
///
/// Every accessor checks bounds; out-of-range coordinates are ignored or return `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelBuffer {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl PixelBuffer {
    /// Creates a fully transparent buffer. The size is validated before allocating.
    pub fn new(width: u32, height: u32) -> Result<Self, CoreError> {
        Self::filled(width, height, Rgba::TRANSPARENT)
    }

    /// Creates a buffer filled with `color`. The size is validated before allocating.
    pub fn filled(width: u32, height: u32, color: Rgba) -> Result<Self, CoreError> {
        validate_size(width, height)?;
        let count = width as usize * height as usize;
        let mut data = Vec::with_capacity(count * 4);
        for _ in 0..count {
            data.extend_from_slice(&color.0);
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Wraps raw RGBA bytes, checking size and length.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Result<Self, CoreError> {
        validate_size(width, height)?;
        let expected = width as usize * height as usize * 4;
        if data.len() != expected {
            return Err(CoreError::BufferSize {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    /// Linear pixel index of `(x, y)` if it is inside the buffer.
    pub fn index_of(&self, x: i32, y: i32) -> Option<u32> {
        if self.in_bounds(x, y) {
            Some(y as u32 * self.width + x as u32)
        } else {
            None
        }
    }

    /// Coordinates of a linear pixel index.
    pub fn coords_of(&self, index: u32) -> (i32, i32) {
        ((index % self.width) as i32, (index / self.width) as i32)
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgba> {
        self.index_of(x, y).and_then(|i| self.get_index(i))
    }

    pub fn get_index(&self, index: u32) -> Option<Rgba> {
        let start = index as usize * 4;
        let px = self.data.get(start..start + 4)?;
        Some(Rgba([px[0], px[1], px[2], px[3]]))
    }

    /// Sets a pixel; returns `false` (and changes nothing) when out of bounds.
    pub fn set(&mut self, x: i32, y: i32, color: Rgba) -> bool {
        match self.index_of(x, y) {
            Some(i) => self.set_index(i, color),
            None => false,
        }
    }

    pub fn set_index(&mut self, index: u32, color: Rgba) -> bool {
        let start = index as usize * 4;
        match self.data.get_mut(start..start + 4) {
            Some(px) => {
                px.copy_from_slice(&color.0);
                true
            }
            None => false,
        }
    }

    pub fn fill(&mut self, color: Rgba) {
        for px in self.data.chunks_exact_mut(4) {
            px.copy_from_slice(&color.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_are_checked() {
        let mut buf = PixelBuffer::new(4, 3).expect("valid size");
        assert!(buf.set(3, 2, Rgba::WHITE));
        assert!(!buf.set(4, 0, Rgba::WHITE));
        assert!(!buf.set(-1, 0, Rgba::WHITE));
        assert!(!buf.set(0, 3, Rgba::WHITE));
        assert_eq!(buf.get(3, 2), Some(Rgba::WHITE));
        assert_eq!(buf.get(4, 2), None);
        assert_eq!(buf.get(i32::MIN, i32::MAX), None);
    }

    #[test]
    fn rejects_bad_raw_length() {
        assert!(PixelBuffer::from_raw(2, 2, vec![0; 15]).is_err());
        assert!(PixelBuffer::from_raw(2, 2, vec![0; 16]).is_ok());
    }
}

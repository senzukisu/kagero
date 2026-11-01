//! A fast hasher for small integer keys (pixel indices, packed colors).
//!
//! Rust's default hasher resists hash flooding, which matters for untrusted keys
//! but makes per-pixel maps several times slower. Kagero only hashes its own pixel
//! indices and colors, so a multiplicative hash is safe here.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub struct FastHasher(u64);

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        // Multiplying mixes keys into the high bits, but hash maps pick buckets with
        // the low bits: rotate the well-mixed bits down (as rustc-hash does).
        self.0.rotate_left(26)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }

    fn write_u32(&mut self, v: u32) {
        self.write_u64(u64::from(v));
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0 ^ v).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// A `HashMap` using [`FastHasher`].
pub type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;

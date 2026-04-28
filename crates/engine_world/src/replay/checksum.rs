//! Checksum utilities for deterministic replay verification.

use serde::{Deserialize, Serialize};

/// A compact per-step checksum for desync debugging.
///
/// Uses CRC32 internally for fast, deterministic hashing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepChecksum {
    /// The CRC32 checksum value.
    value: u32,
}

impl StepChecksum {
    /// Create a checksum from a raw u32 value.
    #[must_use]
    pub const fn from_raw(value: u32) -> Self {
        Self { value }
    }

    /// Get the raw checksum value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.value
    }

    /// Combine two checksums (order-dependent).
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.value.to_le_bytes());
        hasher.update(&other.value.to_le_bytes());
        Self {
            value: hasher.finalize(),
        }
    }

    /// Check if two checksums match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl From<u32> for StepChecksum {
    fn from(value: u32) -> Self {
        Self::from_raw(value)
    }
}

impl From<StepChecksum> for u32 {
    fn from(checksum: StepChecksum) -> Self {
        checksum.value
    }
}

/// Builder for constructing deterministic checksums from simulation state.
///
/// Feeds data in order to produce a final checksum. Order matters for
/// determinism - feeding the same data in different orders produces
/// different checksums.
#[derive(Debug)]
pub struct ChecksumBuilder {
    hasher: crc32fast::Hasher,
}

impl ChecksumBuilder {
    /// Create a new checksum builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: crc32fast::Hasher::new(),
        }
    }

    /// Feed a u64 value (e.g., tick, event ID).
    pub fn feed_u64(&mut self, value: u64) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a u32 value.
    pub fn feed_u32(&mut self, value: u32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed an i32 value (e.g., position coordinate).
    pub fn feed_i32(&mut self, value: i32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a f32 value.
    ///
    /// Note: floating point bit patterns are used directly, so NaN values
    /// with different bit patterns will produce different checksums.
    pub fn feed_f32(&mut self, value: f32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed raw bytes.
    pub fn feed_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.hasher.update(bytes);
        self
    }

    /// Feed a string (length-prefixed for unambiguous parsing).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "strings > 4GB are not expected"
    )]
    pub fn feed_str(&mut self, s: &str) -> &mut Self {
        let len = s.len() as u32;
        self.hasher.update(&len.to_le_bytes());
        self.hasher.update(s.as_bytes());
        self
    }

    /// Feed a position tuple.
    pub fn feed_position(&mut self, x: i32, y: i32, z: i32) -> &mut Self {
        self.hasher.update(&x.to_le_bytes());
        self.hasher.update(&y.to_le_bytes());
        self.hasher.update(&z.to_le_bytes());
        self
    }

    /// Feed an optional value (presence flag + value if present).
    pub fn feed_option_u64(&mut self, opt: Option<u64>) -> &mut Self {
        match opt {
            Some(v) => {
                self.hasher.update(&[1u8]);
                self.hasher.update(&v.to_le_bytes());
            }
            None => {
                self.hasher.update(&[0u8]);
            }
        }
        self
    }

    /// Finalize and produce the checksum, consuming the builder.
    #[must_use]
    pub fn build(self) -> StepChecksum {
        StepChecksum {
            value: self.hasher.finalize(),
        }
    }

    /// Finalize and produce the checksum, resetting for reuse.
    #[must_use]
    pub fn finish(&mut self) -> StepChecksum {
        let value = self.hasher.clone().finalize();
        self.hasher = crc32fast::Hasher::new();
        StepChecksum { value }
    }
}

impl Default for ChecksumBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_from_raw() {
        let cs = StepChecksum::from_raw(0xDEAD_BEEF);
        assert_eq!(cs.value(), 0xDEAD_BEEF);
    }

    #[test]
    fn checksum_combine_order_matters() {
        let a = StepChecksum::from_raw(100);
        let b = StepChecksum::from_raw(200);

        let ab = a.combine(&b);
        let ba = b.combine(&a);

        assert_ne!(ab, ba);
    }

    #[test]
    fn checksum_combine_deterministic() {
        let a = StepChecksum::from_raw(0x1234_5678);
        let b = StepChecksum::from_raw(0xABCD_EF00);

        let combined1 = a.combine(&b);
        let combined2 = a.combine(&b);

        assert_eq!(combined1, combined2);
    }

    #[test]
    fn checksum_matches() {
        let a = StepChecksum::from_raw(42);
        let b = StepChecksum::from_raw(42);
        let c = StepChecksum::from_raw(43);

        assert!(a.matches(&b));
        assert!(!a.matches(&c));
    }

    #[test]
    fn builder_deterministic() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_u64(100).feed_i32(5).feed_str("test");
        let cs1 = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_u64(100).feed_i32(5).feed_str("test");
        let cs2 = b2.build();

        assert_eq!(cs1, cs2);
    }

    #[test]
    fn builder_order_matters() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_u64(1).feed_u64(2);
        let cs1 = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_u64(2).feed_u64(1);
        let cs2 = b2.build();

        assert_ne!(cs1, cs2);
    }

    #[test]
    fn builder_different_values() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_u64(100);
        let cs1 = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_u64(101);
        let cs2 = b2.build();

        assert_ne!(cs1, cs2);
    }

    #[test]
    fn builder_position() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_position(1, 2, 3);
        let cs1 = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_i32(1).feed_i32(2).feed_i32(3);
        let cs2 = b2.build();

        assert_eq!(cs1, cs2);
    }

    #[test]
    fn builder_option() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_option_u64(Some(42));
        let some_cs = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_option_u64(None);
        let none_cs = b2.build();

        assert_ne!(some_cs, none_cs);
    }

    #[test]
    fn builder_finish_resets() {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u64(100);
        let _ = builder.finish();
        builder.feed_u64(200);
        let cs1 = builder.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_u64(200);
        let cs2 = b2.build();

        assert_eq!(cs1, cs2);
    }

    #[test]
    fn builder_f32() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_f32(1.5);
        let cs1 = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_f32(1.5);
        let cs2 = b2.build();

        let mut b3 = ChecksumBuilder::new();
        b3.feed_f32(1.6);
        let cs3 = b3.build();

        assert_eq!(cs1, cs2);
        assert_ne!(cs1, cs3);
    }

    #[test]
    fn builder_bytes() {
        let mut b1 = ChecksumBuilder::new();
        b1.feed_bytes(&[1, 2, 3, 4]);
        let cs1 = b1.build();

        let mut b2 = ChecksumBuilder::new();
        b2.feed_bytes(&[1, 2, 3, 4]);
        let cs2 = b2.build();

        let mut b3 = ChecksumBuilder::new();
        b3.feed_bytes(&[4, 3, 2, 1]);
        let cs3 = b3.build();

        assert_eq!(cs1, cs2);
        assert_ne!(cs1, cs3);
    }

    #[test]
    fn serde_round_trip() {
        let cs = StepChecksum::from_raw(0xCAFE_BABE);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: StepChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cs);
    }

    #[test]
    fn conversions() {
        let cs: StepChecksum = 12345u32.into();
        assert_eq!(cs.value(), 12345);

        let val: u32 = cs.into();
        assert_eq!(val, 12345);
    }
}

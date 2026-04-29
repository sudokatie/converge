//! Stable fingerprinting for diagnostic data.

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Stable fingerprint for diagnostic state comparison.
///
/// Provides a deterministic hash suitable for detecting changes in diagnostic
/// samples between frames or across network boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticFingerprint {
    hash: u64,
}

impl DiagnosticFingerprint {
    /// Create a new empty fingerprint.
    #[must_use]
    pub const fn new() -> Self {
        Self { hash: 0 }
    }

    /// Create a fingerprint from a raw hash value.
    #[must_use]
    pub const fn from_hash(hash: u64) -> Self {
        Self { hash }
    }

    /// Get the raw hash value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.hash
    }

    /// Update the fingerprint with a u8 value.
    pub fn update_u8(&mut self, value: u8) {
        self.hash = self.hash.wrapping_mul(31).wrapping_add(u64::from(value));
    }

    /// Update the fingerprint with a u32 value.
    pub fn update_u32(&mut self, value: u32) {
        self.hash = self.hash.wrapping_mul(31).wrapping_add(u64::from(value));
    }

    /// Update the fingerprint with a u64 value.
    pub fn update_u64(&mut self, value: u64) {
        self.hash = self.hash.wrapping_mul(31).wrapping_add(value);
    }

    /// Update the fingerprint with a usize value.
    #[allow(clippy::cast_possible_truncation)]
    pub fn update_usize(&mut self, value: usize) {
        self.hash = self.hash.wrapping_mul(31).wrapping_add(value as u64);
    }

    /// Update the fingerprint with an i32 value.
    #[allow(clippy::cast_sign_loss)]
    pub fn update_i32(&mut self, value: i32) {
        self.hash = self
            .hash
            .wrapping_mul(31)
            .wrapping_add(u64::from(value as u32));
    }

    /// Update the fingerprint with an i32 array.
    pub fn update_i32_array(&mut self, values: &[i32; 3]) {
        for &v in values {
            self.update_i32(v);
        }
    }

    /// Update the fingerprint with an f32 value (via bits).
    pub fn update_f32(&mut self, value: f32) {
        self.update_u32(value.to_bits());
    }

    /// Update the fingerprint with a string slice.
    pub fn update_str(&mut self, value: &str) {
        for byte in value.bytes() {
            self.update_u8(byte);
        }
        self.update_usize(value.len());
    }

    /// Update the fingerprint with a hashable value.
    pub fn update_hash<H: Hash>(&mut self, value: &H) {
        let mut hasher = FnvHasher::new();
        value.hash(&mut hasher);
        self.update_u64(hasher.finish());
    }

    /// Combine two fingerprints into one.
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        Self {
            hash: self.hash.wrapping_mul(31).wrapping_add(other.hash),
        }
    }

    /// Check if this fingerprint matches another.
    #[must_use]
    pub const fn matches(self, other: Self) -> bool {
        self.hash == other.hash
    }
}

/// Simple FNV-1a hasher for deterministic hashing.
struct FnvHasher {
    hash: u64,
}

impl FnvHasher {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    fn new() -> Self {
        Self {
            hash: Self::OFFSET_BASIS,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hash ^= u64::from(byte);
            self.hash = self.hash.wrapping_mul(Self::PRIME);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_fingerprint_is_zero() {
        let fp = DiagnosticFingerprint::new();
        assert_eq!(fp.as_u64(), 0);
    }

    #[test]
    fn test_from_hash() {
        let fp = DiagnosticFingerprint::from_hash(12345);
        assert_eq!(fp.as_u64(), 12345);
    }

    #[test]
    fn test_update_changes_hash() {
        let mut fp = DiagnosticFingerprint::new();
        let original = fp.as_u64();
        fp.update_u8(42);
        assert_ne!(fp.as_u64(), original);
    }

    #[test]
    fn test_same_updates_same_hash() {
        let mut fp1 = DiagnosticFingerprint::new();
        let mut fp2 = DiagnosticFingerprint::new();
        fp1.update_u32(100);
        fp1.update_i32(-50);
        fp2.update_u32(100);
        fp2.update_i32(-50);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_different_updates_different_hash() {
        let mut fp1 = DiagnosticFingerprint::new();
        let mut fp2 = DiagnosticFingerprint::new();
        fp1.update_u32(100);
        fp2.update_u32(200);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_order_matters() {
        let mut fp1 = DiagnosticFingerprint::new();
        let mut fp2 = DiagnosticFingerprint::new();
        fp1.update_u8(1);
        fp1.update_u8(2);
        fp2.update_u8(2);
        fp2.update_u8(1);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_combine() {
        let mut fp1 = DiagnosticFingerprint::new();
        fp1.update_u8(1);
        let mut fp2 = DiagnosticFingerprint::new();
        fp2.update_u8(2);
        let combined = fp1.combine(fp2);
        assert_ne!(combined, fp1);
        assert_ne!(combined, fp2);
    }

    #[test]
    fn test_matches() {
        let fp1 = DiagnosticFingerprint::from_hash(12345);
        let fp2 = DiagnosticFingerprint::from_hash(12345);
        let fp3 = DiagnosticFingerprint::from_hash(54321);
        assert!(fp1.matches(fp2));
        assert!(!fp1.matches(fp3));
    }

    #[test]
    fn test_i32_array() {
        let mut fp1 = DiagnosticFingerprint::new();
        let mut fp2 = DiagnosticFingerprint::new();
        fp1.update_i32_array(&[1, 2, 3]);
        fp2.update_i32(1);
        fp2.update_i32(2);
        fp2.update_i32(3);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_f32_deterministic() {
        let mut fp1 = DiagnosticFingerprint::new();
        let mut fp2 = DiagnosticFingerprint::new();
        fp1.update_f32(1.5);
        fp2.update_f32(1.5);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_str_update() {
        let mut fp1 = DiagnosticFingerprint::new();
        let mut fp2 = DiagnosticFingerprint::new();
        fp1.update_str("hello");
        fp2.update_str("hello");
        assert_eq!(fp1, fp2);

        let mut fp3 = DiagnosticFingerprint::new();
        fp3.update_str("world");
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn test_serde_round_trip() {
        let mut fp = DiagnosticFingerprint::new();
        fp.update_u32(42);
        fp.update_str("test");
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: DiagnosticFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(fp, recovered);
    }
}

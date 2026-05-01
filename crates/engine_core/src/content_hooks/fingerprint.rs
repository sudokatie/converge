//! Stable fingerprinting for content hooks.
//!
//! Provides deterministic checksums for detecting changes in hook content.

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// A stable fingerprint/checksum for content hook data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHookFingerprint(u64);

impl ContentHookFingerprint {
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn compute<T: Hash>(data: &T) -> Self {
        let mut hasher = StableHasher::new();
        data.hash(&mut hasher);
        Self(hasher.finish())
    }

    #[must_use]
    pub fn combine(fingerprints: &[Self]) -> Self {
        let mut hasher = StableHasher::new();
        for fp in fingerprints {
            fp.0.hash(&mut hasher);
        }
        Self(hasher.finish())
    }

    #[must_use]
    pub const fn matches(self, other: Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Display for ContentHookFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// A stable hasher using FNV-1a algorithm for deterministic fingerprinting.
struct StableHasher {
    state: u64,
}

impl StableHasher {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}

/// Builder for computing fingerprints from multiple components.
#[derive(Default)]
pub struct HookFingerprintBuilder {
    hasher: Option<StableHasher>,
}

impl HookFingerprintBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: Some(StableHasher::new()),
        }
    }

    pub fn add<T: Hash>(&mut self, data: &T) -> &mut Self {
        if let Some(ref mut hasher) = self.hasher {
            data.hash(hasher);
        }
        self
    }

    #[must_use]
    pub fn finish(&mut self) -> ContentHookFingerprint {
        self.hasher
            .take()
            .map(|h| ContentHookFingerprint(h.finish()))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_display() {
        let fp = ContentHookFingerprint::from_raw(0x1234_5678_9ABC_DEF0);
        assert_eq!(format!("{fp}"), "123456789abcdef0");
    }

    #[test]
    fn fingerprint_stability() {
        let data = ("hook_test", 42_u32, vec![1_u8, 2, 3]);

        let fp1 = ContentHookFingerprint::compute(&data);
        let fp2 = ContentHookFingerprint::compute(&data);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn fingerprint_different_data() {
        let fp1 = ContentHookFingerprint::compute(&"event_a");
        let fp2 = ContentHookFingerprint::compute(&"event_b");

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_combine() {
        let fp1 = ContentHookFingerprint::compute(&"a");
        let fp2 = ContentHookFingerprint::compute(&"b");
        let fp3 = ContentHookFingerprint::compute(&"c");

        let combined1 = ContentHookFingerprint::combine(&[fp1, fp2, fp3]);
        let combined2 = ContentHookFingerprint::combine(&[fp1, fp2, fp3]);

        assert_eq!(combined1, combined2);

        let combined_different = ContentHookFingerprint::combine(&[fp1, fp3, fp2]);
        assert_ne!(combined1, combined_different);
    }

    #[test]
    fn fingerprint_builder() {
        let mut builder = HookFingerprintBuilder::new();
        builder.add(&"event").add(&42_u32).add(&[1_u8, 2, 3]);
        let fp = builder.finish();

        assert_ne!(fp.raw(), 0);
    }

    #[test]
    fn fingerprint_builder_order_matters() {
        let mut b1 = HookFingerprintBuilder::new();
        b1.add(&"event").add(&"action");
        let fp1 = b1.finish();

        let mut b2 = HookFingerprintBuilder::new();
        b2.add(&"action").add(&"event");
        let fp2 = b2.finish();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_serde_roundtrip() {
        let fp = ContentHookFingerprint::compute(&"test hook data");

        let json = serde_json::to_string(&fp).unwrap();
        let restored: ContentHookFingerprint = serde_json::from_str(&json).unwrap();

        assert_eq!(fp, restored);
    }

    #[test]
    fn fingerprint_bincode_roundtrip() {
        let fp = ContentHookFingerprint::compute(&vec![1_u32, 2, 3, 4, 5]);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: ContentHookFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(fp, restored);
    }
}

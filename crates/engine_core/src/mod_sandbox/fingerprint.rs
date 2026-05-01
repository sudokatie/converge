//! Stable fingerprinting for mod sandbox configurations.
//!
//! Provides deterministic checksums for detecting changes in mod sets and policies.

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// A stable fingerprint/checksum for sandbox configurations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SandboxFingerprint(u64);

impl SandboxFingerprint {
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

impl std::fmt::Display for SandboxFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

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
pub struct SandboxFingerprintBuilder {
    hasher: Option<StableHasher>,
}

impl SandboxFingerprintBuilder {
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
    pub fn finish(&mut self) -> SandboxFingerprint {
        self.hasher
            .take()
            .map(|h| SandboxFingerprint(h.finish()))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_display() {
        let fp = SandboxFingerprint::from_raw(0x1234_5678_9ABC_DEF0);
        assert_eq!(format!("{fp}"), "123456789abcdef0");
    }

    #[test]
    fn fingerprint_stability() {
        let data = ("mod_name", 1_u32, 2_u32, 3_u32);

        let fp1 = SandboxFingerprint::compute(&data);
        let fp2 = SandboxFingerprint::compute(&data);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn fingerprint_different_data() {
        let fp1 = SandboxFingerprint::compute(&"mod_a");
        let fp2 = SandboxFingerprint::compute(&"mod_b");

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_combine() {
        let fp1 = SandboxFingerprint::compute(&"a");
        let fp2 = SandboxFingerprint::compute(&"b");
        let fp3 = SandboxFingerprint::compute(&"c");

        let combined1 = SandboxFingerprint::combine(&[fp1, fp2, fp3]);
        let combined2 = SandboxFingerprint::combine(&[fp1, fp2, fp3]);

        assert_eq!(combined1, combined2);

        let combined_different = SandboxFingerprint::combine(&[fp1, fp3, fp2]);
        assert_ne!(combined1, combined_different);
    }

    #[test]
    fn fingerprint_builder() {
        let mut builder = SandboxFingerprintBuilder::new();
        builder.add(&"mod").add(&1_u32).add(&"version");
        let fp = builder.finish();

        assert_ne!(fp.raw(), 0);
    }

    #[test]
    fn fingerprint_builder_order_matters() {
        let mut b1 = SandboxFingerprintBuilder::new();
        b1.add(&"first").add(&"second");
        let fp1 = b1.finish();

        let mut b2 = SandboxFingerprintBuilder::new();
        b2.add(&"second").add(&"first");
        let fp2 = b2.finish();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_serde_roundtrip() {
        let fp = SandboxFingerprint::compute(&"test data");

        let json = serde_json::to_string(&fp).unwrap();
        let restored: SandboxFingerprint = serde_json::from_str(&json).unwrap();

        assert_eq!(fp, restored);
    }

    #[test]
    fn fingerprint_bincode_roundtrip() {
        let fp = SandboxFingerprint::compute(&vec![1_u32, 2, 3, 4, 5]);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: SandboxFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(fp, restored);
    }
}

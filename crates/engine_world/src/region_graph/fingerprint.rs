//! Fingerprinting and checksums for region graphs.

use serde::{Deserialize, Serialize};

/// Stable fingerprint for a region graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphFingerprint {
    value: u32,
}

impl GraphFingerprint {
    /// Create a fingerprint from a raw value.
    #[must_use]
    pub const fn from_raw(value: u32) -> Self {
        Self { value }
    }

    /// Get the raw value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.value
    }

    /// Combine two fingerprints.
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.value.to_le_bytes());
        hasher.update(&other.value.to_le_bytes());
        Self {
            value: hasher.finalize(),
        }
    }

    /// Check if two fingerprints match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl std::fmt::Display for GraphFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}", self.value)
    }
}

/// Builder for constructing deterministic graph fingerprints.
#[derive(Debug)]
pub struct FingerprintBuilder {
    hasher: crc32fast::Hasher,
}

impl FingerprintBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: crc32fast::Hasher::new(),
        }
    }

    /// Feed a u64 value.
    pub fn feed_u64(&mut self, value: u64) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a u32 value.
    pub fn feed_u32(&mut self, value: u32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a u8 value.
    pub fn feed_u8(&mut self, value: u8) -> &mut Self {
        self.hasher.update(&[value]);
        self
    }

    /// Feed a string (length-prefixed).
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

    /// Feed a boolean.
    pub fn feed_bool(&mut self, value: bool) -> &mut Self {
        self.hasher.update(&[u8::from(value)]);
        self
    }

    /// Feed a region node.
    pub fn feed_region(&mut self, id: u64, kind: u8, tier: u8, tag_count: usize) -> &mut Self {
        self.feed_u64(id);
        self.feed_u8(kind);
        self.feed_u8(tier);
        #[expect(clippy::cast_possible_truncation, reason = "tag count always small")]
        self.feed_u32(tag_count as u32);
        self
    }

    /// Feed an edge.
    pub fn feed_edge(
        &mut self,
        id: u64,
        from: u64,
        to: u64,
        kind: u8,
        cost: u32,
        bidirectional: bool,
    ) -> &mut Self {
        self.feed_u64(id);
        self.feed_u64(from);
        self.feed_u64(to);
        self.feed_u8(kind);
        self.feed_u32(cost);
        self.feed_bool(bidirectional);
        self
    }

    /// Build the fingerprint.
    #[must_use]
    pub fn build(self) -> GraphFingerprint {
        GraphFingerprint {
            value: self.hasher.finalize(),
        }
    }

    /// Build and reset for reuse.
    #[must_use]
    pub fn finish(&mut self) -> GraphFingerprint {
        let value = self.hasher.clone().finalize();
        self.hasher = crc32fast::Hasher::new();
        GraphFingerprint { value }
    }
}

impl Default for FingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Checksum for graph state verification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphChecksum {
    /// Structure checksum (topology).
    pub structure: u32,
    /// State checksum (visited, gates).
    pub state: u32,
}

impl GraphChecksum {
    /// Create a new checksum.
    #[must_use]
    pub const fn new(structure: u32, state: u32) -> Self {
        Self { structure, state }
    }

    /// Check if checksums match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.structure == other.structure && self.state == other.state
    }

    /// Check if structure matches (ignores state).
    #[must_use]
    pub const fn structure_matches(&self, other: &Self) -> bool {
        self.structure == other.structure
    }
}

impl std::fmt::Display for GraphChecksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}:{:08x}", self.structure, self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_deterministic() {
        let mut b1 = FingerprintBuilder::new();
        b1.feed_u64(100).feed_str("test").feed_u8(5);
        let fp1 = b1.build();

        let mut b2 = FingerprintBuilder::new();
        b2.feed_u64(100).feed_str("test").feed_u8(5);
        let fp2 = b2.build();

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn fingerprint_order_matters() {
        let mut b1 = FingerprintBuilder::new();
        b1.feed_u64(1).feed_u64(2);
        let fp1 = b1.build();

        let mut b2 = FingerprintBuilder::new();
        b2.feed_u64(2).feed_u64(1);
        let fp2 = b2.build();

        assert!(!fp1.matches(&fp2));
    }

    #[test]
    fn fingerprint_combine() {
        let fp1 = GraphFingerprint::from_raw(100);
        let fp2 = GraphFingerprint::from_raw(200);

        let combined1 = fp1.combine(&fp2);
        let combined2 = fp2.combine(&fp1);

        assert_ne!(combined1, combined2);
    }

    #[test]
    fn fingerprint_display() {
        let fp = GraphFingerprint::from_raw(0xDEAD_BEEF);
        assert_eq!(format!("{fp}"), "deadbeef");
    }

    #[test]
    fn checksum_matching() {
        let cs1 = GraphChecksum::new(100, 200);
        let cs2 = GraphChecksum::new(100, 200);
        let cs3 = GraphChecksum::new(100, 300);
        let cs4 = GraphChecksum::new(101, 200);

        assert!(cs1.matches(&cs2));
        assert!(!cs1.matches(&cs3));
        assert!(!cs1.matches(&cs4));
        assert!(cs1.structure_matches(&cs3));
    }

    #[test]
    fn builder_region_and_edge() {
        let mut builder = FingerprintBuilder::new();
        builder
            .feed_region(1, 0, 0, 2)
            .feed_edge(1, 1, 2, 0, 10, true);
        let fp = builder.build();

        assert_ne!(fp.value(), 0);
    }

    #[test]
    fn serde_roundtrip() {
        let fp = GraphFingerprint::from_raw(0xCAFE_BABE);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: GraphFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(fp, recovered);

        let cs = GraphChecksum::new(123, 456);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: GraphChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(cs, recovered);
    }
}

//! Fingerprinting and checksums for topology.

use serde::{Deserialize, Serialize};

/// Stable fingerprint for a topology.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopologyFingerprint {
    value: u32,
}

impl TopologyFingerprint {
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

impl std::fmt::Display for TopologyFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}", self.value)
    }
}

/// Data for fingerprinting a node.
pub struct NodeData {
    /// Node ID.
    pub id: u64,
    /// Node role.
    pub role: u8,
    /// X position.
    pub x: f32,
    /// Y position.
    pub y: f32,
    /// Z position.
    pub z: f32,
    /// Radius.
    pub radius: f32,
    /// Height.
    pub height: f32,
    /// Depth.
    pub depth: u32,
}

/// Data for fingerprinting a segment.
pub struct SegmentData {
    /// Segment ID.
    pub id: u64,
    /// From node ID.
    pub from: u64,
    /// To node ID.
    pub to: u64,
    /// Segment kind.
    pub kind: u8,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Length.
    pub length: f32,
    /// Bidirectional flag.
    pub bidirectional: bool,
    /// Cost.
    pub cost: u32,
}

/// Builder for constructing deterministic topology fingerprints.
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

    /// Feed a f32 value (as bits).
    pub fn feed_f32(&mut self, value: f32) -> &mut Self {
        self.hasher.update(&value.to_bits().to_le_bytes());
        self
    }

    /// Feed a boolean.
    pub fn feed_bool(&mut self, value: bool) -> &mut Self {
        self.hasher.update(&[u8::from(value)]);
        self
    }

    /// Feed a topology node.
    pub fn feed_node(&mut self, node: &NodeData) -> &mut Self {
        self.feed_u64(node.id);
        self.feed_u8(node.role);
        self.feed_f32(node.x);
        self.feed_f32(node.y);
        self.feed_f32(node.z);
        self.feed_f32(node.radius);
        self.feed_f32(node.height);
        self.feed_u32(node.depth);
        self
    }

    /// Feed a topology segment.
    pub fn feed_segment(&mut self, seg: &SegmentData) -> &mut Self {
        self.feed_u64(seg.id);
        self.feed_u64(seg.from);
        self.feed_u64(seg.to);
        self.feed_u8(seg.kind);
        self.feed_f32(seg.width);
        self.feed_f32(seg.height);
        self.feed_f32(seg.length);
        self.feed_bool(seg.bidirectional);
        self.feed_u32(seg.cost);
        self
    }

    /// Build the fingerprint.
    #[must_use]
    pub fn build(self) -> TopologyFingerprint {
        TopologyFingerprint {
            value: self.hasher.finalize(),
        }
    }

    /// Build and reset for reuse.
    #[must_use]
    pub fn finish(&mut self) -> TopologyFingerprint {
        let value = self.hasher.clone().finalize();
        self.hasher = crc32fast::Hasher::new();
        TopologyFingerprint { value }
    }
}

impl Default for FingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Checksum for topology state verification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopologyChecksum {
    /// Structure checksum (nodes and segments).
    pub structure: u32,
    /// Annotations checksum.
    pub annotations: u32,
}

impl TopologyChecksum {
    /// Create a new checksum.
    #[must_use]
    pub const fn new(structure: u32, annotations: u32) -> Self {
        Self {
            structure,
            annotations,
        }
    }

    /// Check if checksums match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.structure == other.structure && self.annotations == other.annotations
    }

    /// Check if structure matches (ignores annotations).
    #[must_use]
    pub const fn structure_matches(&self, other: &Self) -> bool {
        self.structure == other.structure
    }

    /// Combine with another checksum.
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.structure.to_le_bytes());
        hasher.update(&other.structure.to_le_bytes());
        let structure = hasher.finalize();

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.annotations.to_le_bytes());
        hasher.update(&other.annotations.to_le_bytes());
        let annotations = hasher.finalize();

        Self {
            structure,
            annotations,
        }
    }
}

impl std::fmt::Display for TopologyChecksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}:{:08x}", self.structure, self.annotations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_deterministic() {
        let mut b1 = FingerprintBuilder::new();
        b1.feed_u64(100).feed_f32(1.5).feed_u8(5);
        let fp1 = b1.build();

        let mut b2 = FingerprintBuilder::new();
        b2.feed_u64(100).feed_f32(1.5).feed_u8(5);
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
        let fp1 = TopologyFingerprint::from_raw(100);
        let fp2 = TopologyFingerprint::from_raw(200);

        let combined1 = fp1.combine(&fp2);
        let combined2 = fp2.combine(&fp1);

        assert_ne!(combined1, combined2);
    }

    #[test]
    fn fingerprint_display() {
        let fp = TopologyFingerprint::from_raw(0xDEAD_BEEF);
        assert_eq!(format!("{fp}"), "deadbeef");
    }

    #[test]
    fn checksum_matching() {
        let cs1 = TopologyChecksum::new(100, 200);
        let cs2 = TopologyChecksum::new(100, 200);
        let cs3 = TopologyChecksum::new(100, 300);
        let cs4 = TopologyChecksum::new(101, 200);

        assert!(cs1.matches(&cs2));
        assert!(!cs1.matches(&cs3));
        assert!(!cs1.matches(&cs4));
        assert!(cs1.structure_matches(&cs3));
    }

    #[test]
    fn builder_node_and_segment() {
        let mut builder = FingerprintBuilder::new();
        builder
            .feed_node(&NodeData {
                id: 1,
                role: 0,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                radius: 5.0,
                height: 4.0,
                depth: 0,
            })
            .feed_segment(&SegmentData {
                id: 1,
                from: 1,
                to: 2,
                kind: 0,
                width: 3.0,
                height: 2.5,
                length: 10.0,
                bidirectional: true,
                cost: 100,
            });
        let fp = builder.build();

        assert_ne!(fp.value(), 0);
    }

    #[test]
    fn builder_finish_resets() {
        let mut builder = FingerprintBuilder::new();
        builder.feed_u64(42);
        let fp1 = builder.finish();

        builder.feed_u64(42);
        let fp2 = builder.finish();

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn serde_roundtrip() {
        let fp = TopologyFingerprint::from_raw(0xCAFE_BABE);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: TopologyFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(fp, recovered);

        let cs = TopologyChecksum::new(123, 456);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: TopologyChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(cs, recovered);
    }
}

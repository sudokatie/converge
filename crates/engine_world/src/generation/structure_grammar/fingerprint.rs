//! Fingerprinting and checksums for generated layouts.

use serde::{Deserialize, Serialize};

use super::layout::{GeneratedLayout, Placement};

/// Stable fingerprint for a generated layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayoutFingerprint {
    value: u32,
}

impl LayoutFingerprint {
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

    /// Check if two fingerprints match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.value == other.value
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
}

impl std::fmt::Display for LayoutFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}", self.value)
    }
}

/// Builder for constructing deterministic layout fingerprints.
#[derive(Debug)]
pub struct LayoutFingerprintBuilder {
    hasher: crc32fast::Hasher,
}

impl LayoutFingerprintBuilder {
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

    /// Feed an i32 value.
    pub fn feed_i32(&mut self, value: i32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a u8 value.
    pub fn feed_u8(&mut self, value: u8) -> &mut Self {
        self.hasher.update(&[value]);
        self
    }

    /// Feed a string.
    #[expect(clippy::cast_possible_truncation, reason = "string length fits in u32")]
    pub fn feed_str(&mut self, value: &str) -> &mut Self {
        self.hasher.update(&(value.len() as u32).to_le_bytes());
        self.hasher.update(value.as_bytes());
        self
    }

    /// Feed a placement.
    #[expect(clippy::cast_possible_truncation, reason = "tag count fits in u32")]
    pub fn feed_placement(&mut self, placement: &Placement) -> &mut Self {
        self.feed_u64(placement.id.value());
        self.feed_u64(placement.template_id.value());
        self.feed_u8(placement.kind.as_raw());
        self.feed_i32(placement.position[0]);
        self.feed_i32(placement.position[1]);
        self.feed_i32(placement.position[2]);
        self.feed_i32(placement.bounds.min[0]);
        self.feed_i32(placement.bounds.min[1]);
        self.feed_i32(placement.bounds.min[2]);
        self.feed_i32(placement.bounds.max[0]);
        self.feed_i32(placement.bounds.max[1]);
        self.feed_i32(placement.bounds.max[2]);
        self.feed_u32(placement.depth);
        self.feed_u64(placement.parent.map_or(u64::MAX, |p| p.value()));

        self.feed_u32(placement.tags.len() as u32);
        for tag in &placement.tags {
            self.feed_str(tag);
        }

        self
    }

    /// Build the fingerprint.
    #[must_use]
    pub fn build(self) -> LayoutFingerprint {
        LayoutFingerprint {
            value: self.hasher.finalize(),
        }
    }

    /// Build and reset for reuse.
    #[must_use]
    pub fn finish(&mut self) -> LayoutFingerprint {
        let value = self.hasher.clone().finalize();
        self.hasher = crc32fast::Hasher::new();
        LayoutFingerprint { value }
    }
}

impl Default for LayoutFingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Checksum for layout state verification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayoutChecksum {
    /// Structure checksum (placements and bounds).
    pub structure: u32,
    /// Metadata checksum (tags and metadata).
    pub metadata: u32,
}

impl LayoutChecksum {
    /// Create a new checksum.
    #[must_use]
    pub const fn new(structure: u32, metadata: u32) -> Self {
        Self {
            structure,
            metadata,
        }
    }

    /// Check if checksums match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.structure == other.structure && self.metadata == other.metadata
    }

    /// Check if structure matches (ignores metadata).
    #[must_use]
    pub const fn structure_matches(&self, other: &Self) -> bool {
        self.structure == other.structure
    }
}

impl std::fmt::Display for LayoutChecksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}:{:08x}", self.structure, self.metadata)
    }
}

impl GeneratedLayout {
    /// Compute a fingerprint of the layout.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "placement count fits in u32"
    )]
    pub fn fingerprint(&self) -> LayoutFingerprint {
        let mut builder = LayoutFingerprintBuilder::new();
        builder.feed_u64(self.seed);
        builder.feed_u32(self.placement_count() as u32);
        builder.feed_u32(self.max_depth);

        for placement in self.placements() {
            builder.feed_placement(placement);
        }

        builder.build()
    }

    /// Compute a checksum of the layout.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> LayoutChecksum {
        let mut struct_builder = LayoutFingerprintBuilder::new();
        struct_builder.feed_u64(self.seed);
        struct_builder.feed_u32(self.placement_count() as u32);

        for placement in self.placements() {
            struct_builder.feed_u64(placement.id.value());
            struct_builder.feed_u64(placement.template_id.value());
            struct_builder.feed_i32(placement.position[0]);
            struct_builder.feed_i32(placement.position[1]);
            struct_builder.feed_i32(placement.position[2]);
            struct_builder.feed_i32(placement.bounds.min[0]);
            struct_builder.feed_i32(placement.bounds.min[1]);
            struct_builder.feed_i32(placement.bounds.min[2]);
            struct_builder.feed_i32(placement.bounds.max[0]);
            struct_builder.feed_i32(placement.bounds.max[1]);
            struct_builder.feed_i32(placement.bounds.max[2]);
        }

        let structure = struct_builder.build().value();

        let mut meta_builder = LayoutFingerprintBuilder::new();
        meta_builder.feed_u32(self.placement_count() as u32);

        for placement in self.placements() {
            meta_builder.feed_u64(placement.id.value());
            meta_builder.feed_u32(placement.tags.len() as u32);
            for tag in &placement.tags {
                meta_builder.feed_str(tag);
            }
            meta_builder.feed_u32(placement.metadata.len() as u32);
            for (k, v) in &placement.metadata {
                meta_builder.feed_str(k);
                meta_builder.feed_str(v);
            }
        }

        let metadata = meta_builder.build().value();

        LayoutChecksum::new(structure, metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::structure_grammar::id::{PlacementId, TemplateId};
    use crate::generation::structure_grammar::template::Bounds;

    fn test_placement(id: u64, pos: [i32; 3]) -> Placement {
        Placement::new(
            PlacementId::new(id),
            TemplateId::new(1),
            pos,
            Bounds::from_size(5, 5, 5).translate(pos),
        )
    }

    #[test]
    fn fingerprint_deterministic() {
        let mut layout1 = GeneratedLayout::new(42);
        layout1.add_placement(test_placement(1, [0, 0, 0]));
        layout1.add_placement(test_placement(2, [10, 0, 0]));

        let mut layout2 = GeneratedLayout::new(42);
        layout2.add_placement(test_placement(1, [0, 0, 0]));
        layout2.add_placement(test_placement(2, [10, 0, 0]));

        assert!(layout1.fingerprint().matches(&layout2.fingerprint()));
    }

    #[test]
    fn fingerprint_different_seeds() {
        let mut layout1 = GeneratedLayout::new(111);
        layout1.add_placement(test_placement(1, [0, 0, 0]));

        let mut layout2 = GeneratedLayout::new(222);
        layout2.add_placement(test_placement(1, [0, 0, 0]));

        assert!(!layout1.fingerprint().matches(&layout2.fingerprint()));
    }

    #[test]
    fn fingerprint_different_placements() {
        let mut layout1 = GeneratedLayout::new(42);
        layout1.add_placement(test_placement(1, [0, 0, 0]));

        let mut layout2 = GeneratedLayout::new(42);
        layout2.add_placement(test_placement(1, [10, 0, 0]));

        assert!(!layout1.fingerprint().matches(&layout2.fingerprint()));
    }

    #[test]
    fn fingerprint_display() {
        let fp = LayoutFingerprint::from_raw(0xDEAD_BEEF);
        assert_eq!(format!("{fp}"), "deadbeef");
    }

    #[test]
    fn fingerprint_combine() {
        let fp1 = LayoutFingerprint::from_raw(100);
        let fp2 = LayoutFingerprint::from_raw(200);

        let combined1 = fp1.combine(&fp2);
        let combined2 = fp2.combine(&fp1);

        assert!(!combined1.matches(&combined2));
    }

    #[test]
    fn checksum_structure_vs_metadata() {
        let mut layout1 = GeneratedLayout::new(42);
        layout1.add_placement(test_placement(1, [0, 0, 0]));

        let mut layout2 = GeneratedLayout::new(42);
        layout2
            .add_placement(test_placement(1, [0, 0, 0]).with_tags(vec!["different".to_string()]));

        let cs1 = layout1.checksum();
        let cs2 = layout2.checksum();

        assert!(cs1.structure_matches(&cs2));
        assert!(!cs1.matches(&cs2));
    }

    #[test]
    fn builder_feed_methods() {
        let mut builder = LayoutFingerprintBuilder::new();
        builder
            .feed_u64(42)
            .feed_u32(100)
            .feed_i32(-50)
            .feed_u8(255)
            .feed_str("test");
        let fp = builder.build();

        assert_ne!(fp.value(), 0);
    }

    #[test]
    fn builder_finish_resets() {
        let mut builder = LayoutFingerprintBuilder::new();
        builder.feed_u64(42);
        let fp1 = builder.finish();

        builder.feed_u64(42);
        let fp2 = builder.finish();

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn checksum_display() {
        let cs = LayoutChecksum::new(0xCAFE, 0xBABE);
        let display = format!("{cs}");
        assert!(display.contains("cafe"));
        assert!(display.contains("babe"));
    }

    #[test]
    fn serde_roundtrip() {
        let fp = LayoutFingerprint::from_raw(0xDEAD_BEEF);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: LayoutFingerprint = serde_json::from_str(&json).unwrap();
        assert!(fp.matches(&recovered));

        let cs = LayoutChecksum::new(123, 456);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: LayoutChecksum = serde_json::from_str(&json).unwrap();
        assert!(cs.matches(&recovered));
    }
}

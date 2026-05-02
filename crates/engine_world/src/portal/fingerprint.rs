//! Fingerprinting and checksums for portal graphs.

use serde::{Deserialize, Serialize};

use super::graph::PortalGraph;
use super::id::{PortalId, ZoneId};
use super::portal::Portal;

/// Stable fingerprint for a portal configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortalFingerprint {
    value: u32,
}

impl PortalFingerprint {
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

impl std::fmt::Display for PortalFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}", self.value)
    }
}

/// Builder for constructing deterministic portal fingerprints.
#[derive(Debug)]
pub struct PortalFingerprintBuilder {
    hasher: crc32fast::Hasher,
}

impl PortalFingerprintBuilder {
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

    /// Feed a bool value.
    pub fn feed_bool(&mut self, value: bool) -> &mut Self {
        self.hasher.update(&[u8::from(value)]);
        self
    }

    /// Feed a f32 value.
    pub fn feed_f32(&mut self, value: f32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a string.
    #[expect(clippy::cast_possible_truncation, reason = "string length fits in u32")]
    pub fn feed_str(&mut self, value: &str) -> &mut Self {
        self.hasher.update(&(value.len() as u32).to_le_bytes());
        self.hasher.update(value.as_bytes());
        self
    }

    /// Feed optional string.
    pub fn feed_option_str(&mut self, value: Option<&str>) -> &mut Self {
        match value {
            Some(s) => {
                self.feed_bool(true);
                self.feed_str(s);
            }
            None => {
                self.feed_bool(false);
            }
        }
        self
    }

    /// Feed a portal ID.
    pub fn feed_portal_id(&mut self, id: PortalId) -> &mut Self {
        self.feed_u64(id.raw())
    }

    /// Feed a zone ID.
    pub fn feed_zone_id(&mut self, id: ZoneId) -> &mut Self {
        self.feed_u64(id.raw())
    }

    /// Feed a Vec3 position.
    pub fn feed_vec3(&mut self, v: glam::Vec3) -> &mut Self {
        self.feed_f32(v.x);
        self.feed_f32(v.y);
        self.feed_f32(v.z)
    }

    /// Feed a portal's spatial configuration.
    pub fn feed_portal(&mut self, portal: &Portal) -> &mut Self {
        self.feed_portal_id(portal.id);
        self.feed_zone_id(portal.endpoint_a.zone);
        self.feed_zone_id(portal.endpoint_b.zone);
        self.feed_vec3(portal.endpoint_a.position);
        self.feed_vec3(portal.endpoint_a.forward);
        self.feed_vec3(portal.endpoint_a.up);
        self.feed_vec3(portal.endpoint_a.half_extents);
        self.feed_vec3(portal.endpoint_b.position);
        self.feed_vec3(portal.endpoint_b.forward);
        self.feed_vec3(portal.endpoint_b.up);
        self.feed_vec3(portal.endpoint_b.half_extents);
        self.feed_bool(portal.flags.active);
        self.feed_bool(portal.flags.bidirectional);
        self
    }

    /// Build the fingerprint.
    #[must_use]
    pub fn build(self) -> PortalFingerprint {
        PortalFingerprint {
            value: self.hasher.finalize(),
        }
    }

    /// Build and reset for reuse.
    #[must_use]
    pub fn finish(&mut self) -> PortalFingerprint {
        let value = self.hasher.clone().finalize();
        self.hasher = crc32fast::Hasher::new();
        PortalFingerprint { value }
    }
}

impl Default for PortalFingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Checksum for portal graph state verification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortalGraphChecksum {
    /// Structure checksum (topology only).
    pub structure: u32,
    /// Spatial checksum (positions and transforms).
    pub spatial: u32,
    /// Metadata checksum (names and tags).
    pub metadata: u32,
}

impl PortalGraphChecksum {
    /// Create a new checksum.
    #[must_use]
    pub const fn new(structure: u32, spatial: u32, metadata: u32) -> Self {
        Self {
            structure,
            spatial,
            metadata,
        }
    }

    /// Check if checksums fully match.
    #[must_use]
    pub const fn matches(&self, other: &Self) -> bool {
        self.structure == other.structure
            && self.spatial == other.spatial
            && self.metadata == other.metadata
    }

    /// Check if structure matches (ignores spatial and metadata).
    #[must_use]
    pub const fn structure_matches(&self, other: &Self) -> bool {
        self.structure == other.structure
    }

    /// Check if topology and spatial match (ignores metadata).
    #[must_use]
    pub const fn topology_matches(&self, other: &Self) -> bool {
        self.structure == other.structure && self.spatial == other.spatial
    }
}

impl std::fmt::Display for PortalGraphChecksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:08x}:{:08x}:{:08x}",
            self.structure, self.spatial, self.metadata
        )
    }
}

impl PortalGraph {
    /// Compute a fingerprint of the portal graph.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn fingerprint(&self) -> PortalFingerprint {
        let mut builder = PortalFingerprintBuilder::new();
        builder.feed_u32(self.portal_count() as u32);
        builder.feed_u32(self.zone_count() as u32);

        let mut portal_ids: Vec<_> = self.portal_ids().collect();
        portal_ids.sort();

        for portal_id in portal_ids {
            if let Some(portal) = self.portal(portal_id) {
                builder.feed_portal(portal);
            }
        }

        builder.build()
    }

    /// Compute a checksum of the portal graph.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn checksum(&self) -> PortalGraphChecksum {
        let mut portal_ids: Vec<_> = self.portal_ids().collect();
        portal_ids.sort();

        let mut zone_ids: Vec<_> = self.zone_ids().collect();
        zone_ids.sort();

        let mut struct_builder = PortalFingerprintBuilder::new();
        struct_builder.feed_u32(self.portal_count() as u32);
        struct_builder.feed_u32(self.zone_count() as u32);

        for portal_id in &portal_ids {
            if let Some(portal) = self.portal(*portal_id) {
                struct_builder.feed_portal_id(portal.id);
                struct_builder.feed_zone_id(portal.endpoint_a.zone);
                struct_builder.feed_zone_id(portal.endpoint_b.zone);
            }
        }
        let structure = struct_builder.build().value();

        let mut spatial_builder = PortalFingerprintBuilder::new();
        for portal_id in &portal_ids {
            if let Some(portal) = self.portal(*portal_id) {
                spatial_builder.feed_portal_id(portal.id);
                spatial_builder.feed_vec3(portal.endpoint_a.position);
                spatial_builder.feed_vec3(portal.endpoint_a.forward);
                spatial_builder.feed_vec3(portal.endpoint_a.up);
                spatial_builder.feed_vec3(portal.endpoint_a.half_extents);
                spatial_builder.feed_vec3(portal.endpoint_b.position);
                spatial_builder.feed_vec3(portal.endpoint_b.forward);
                spatial_builder.feed_vec3(portal.endpoint_b.up);
                spatial_builder.feed_vec3(portal.endpoint_b.half_extents);
            }
        }
        let spatial = spatial_builder.build().value();

        let mut meta_builder = PortalFingerprintBuilder::new();
        for portal_id in &portal_ids {
            if let Some(portal) = self.portal(*portal_id) {
                meta_builder.feed_portal_id(portal.id);
                meta_builder.feed_option_str(portal.name.as_deref());
            }
        }
        for zone_id in &zone_ids {
            if let Some(zone) = self.zone(*zone_id) {
                meta_builder.feed_zone_id(*zone_id);
                meta_builder.feed_option_str(zone.name.as_deref());
                meta_builder.feed_u32(zone.tags.len() as u32);
                for tag in &zone.tags {
                    meta_builder.feed_str(tag);
                }
            }
        }
        let metadata = meta_builder.build().value();

        PortalGraphChecksum::new(structure, spatial, metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::endpoint::PortalEndpoint;
    use crate::portal::graph::ZoneMetadata;
    use glam::Vec3;

    fn test_graph() -> PortalGraph {
        let mut graph = PortalGraph::new();
        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new().with_name("zone_a"));
        graph.add_zone(ZoneId::new(0, 1), ZoneMetadata::new().with_name("zone_b"));

        let portal = Portal::new(
            PortalId::new(0, 0),
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 3.0),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 1),
                Vec3::new(10.0, 0.0, 0.0),
                -Vec3::Z,
                Vec3::Y,
                2.0,
                3.0,
            ),
        );
        graph.add_portal(portal);
        graph
    }

    #[test]
    fn fingerprint_deterministic() {
        let graph1 = test_graph();
        let graph2 = test_graph();

        assert!(graph1.fingerprint().matches(&graph2.fingerprint()));
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let graph1 = test_graph();

        let mut graph2 = test_graph();
        graph2.add_zone(ZoneId::new(0, 2), ZoneMetadata::new());

        assert!(!graph1.fingerprint().matches(&graph2.fingerprint()));
    }

    #[test]
    fn checksum_structure_vs_metadata() {
        let graph1 = test_graph();

        let mut graph2 = test_graph();
        if let Some(zone) = graph2.zone_mut(ZoneId::new(0, 0)) {
            zone.name = Some("different_name".to_string());
        }

        let cs1 = graph1.checksum();
        let cs2 = graph2.checksum();

        assert!(cs1.structure_matches(&cs2));
        assert!(cs1.topology_matches(&cs2));
        assert!(!cs1.matches(&cs2));
    }

    #[test]
    fn fingerprint_display() {
        let fp = PortalFingerprint::from_raw(0xDEAD_BEEF);
        assert_eq!(format!("{fp}"), "deadbeef");
    }

    #[test]
    fn fingerprint_combine() {
        let fp1 = PortalFingerprint::from_raw(100);
        let fp2 = PortalFingerprint::from_raw(200);

        let combined1 = fp1.combine(&fp2);
        let combined2 = fp2.combine(&fp1);

        assert!(!combined1.matches(&combined2));
    }

    #[test]
    fn checksum_display() {
        let cs = PortalGraphChecksum::new(0xCAFE, 0xBABE, 0xDEAD);
        let display = format!("{cs}");
        assert!(display.contains("cafe"));
        assert!(display.contains("babe"));
        assert!(display.contains("dead"));
    }

    #[test]
    fn builder_feed_methods() {
        let mut builder = PortalFingerprintBuilder::new();
        builder
            .feed_u64(42)
            .feed_u32(100)
            .feed_i32(-50)
            .feed_u8(255)
            .feed_bool(true)
            .feed_f32(std::f32::consts::PI)
            .feed_str("test")
            .feed_option_str(Some("optional"));
        let fp = builder.build();

        assert_ne!(fp.value(), 0);
    }

    #[test]
    fn builder_finish_resets() {
        let mut builder = PortalFingerprintBuilder::new();
        builder.feed_u64(42);
        let fp1 = builder.finish();

        builder.feed_u64(42);
        let fp2 = builder.finish();

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn serde_roundtrip() {
        let fp = PortalFingerprint::from_raw(0xDEAD_BEEF);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: PortalFingerprint = serde_json::from_str(&json).unwrap();
        assert!(fp.matches(&recovered));

        let cs = PortalGraphChecksum::new(123, 456, 789);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: PortalGraphChecksum = serde_json::from_str(&json).unwrap();
        assert!(cs.matches(&recovered));
    }
}

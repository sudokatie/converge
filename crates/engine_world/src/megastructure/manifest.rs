//! Streaming manifests for megastructures.

use std::collections::BTreeMap;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::{
    anchor::StructureAnchor,
    bounds::ChunkBounds,
    slice::ChunkSlice,
    structure_id::MegastructureId,
    structure_kind::{StructureKind, StructureZone},
};

/// Streaming priority tier for loading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StreamingTier {
    /// Critical - must load immediately (anchor, player location).
    Critical,
    /// High - load soon (adjacent to loaded, visible).
    High,
    /// Normal - standard loading priority.
    Normal,
    /// Low - load when resources available.
    Low,
    /// Background - load opportunistically.
    Background,
}

impl StreamingTier {
    /// Convert to numeric priority (lower = higher priority).
    #[must_use]
    pub const fn as_priority(self) -> u16 {
        match self {
            Self::Critical => 0,
            Self::High => 100,
            Self::Normal => 500,
            Self::Low => 1000,
            Self::Background => 5000,
        }
    }
}

/// Entry in the streaming manifest for a single chunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Offset from structure anchor.
    pub offset: (i32, i32, i32),
    /// Zone classification.
    pub zone: StructureZone,
    /// Streaming tier.
    pub tier: StreamingTier,
    /// Estimated data size in bytes.
    pub size_estimate: u32,
    /// Dependencies (offsets of chunks that must load first).
    pub dependencies: Vec<(i32, i32, i32)>,
}

impl ManifestEntry {
    /// Create a new manifest entry.
    #[must_use]
    pub fn new(offset: (i32, i32, i32), zone: StructureZone) -> Self {
        Self {
            offset,
            zone,
            tier: StreamingTier::Normal,
            size_estimate: 0,
            dependencies: Vec::new(),
        }
    }

    /// Set the streaming tier.
    #[must_use]
    pub const fn with_tier(mut self, tier: StreamingTier) -> Self {
        self.tier = tier;
        self
    }

    /// Set the size estimate.
    #[must_use]
    pub const fn with_size(mut self, size: u32) -> Self {
        self.size_estimate = size;
        self
    }

    /// Add a dependency.
    #[must_use]
    pub fn with_dependency(mut self, dep: (i32, i32, i32)) -> Self {
        self.dependencies.push(dep);
        self
    }
}

/// Streaming manifest for a megastructure.
///
/// Contains all metadata needed to stream the structure's chunks
/// in the correct order with proper prioritization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingManifest {
    /// Structure identifier.
    structure_id: MegastructureId,
    /// Structure kind.
    kind: StructureKind,
    /// Anchor position.
    anchor: StructureAnchor,
    /// Chunk bounds.
    bounds: ChunkBounds,
    /// Per-chunk entries indexed by offset.
    entries: BTreeMap<(i32, i32, i32), ManifestEntry>,
    /// Version for cache invalidation.
    version: u32,
}

impl StreamingManifest {
    /// Create a new manifest for a structure.
    #[must_use]
    pub fn new(
        structure_id: MegastructureId,
        kind: StructureKind,
        anchor: StructureAnchor,
        bounds: ChunkBounds,
    ) -> Self {
        Self {
            structure_id,
            kind,
            anchor,
            bounds,
            entries: BTreeMap::new(),
            version: 1,
        }
    }

    /// Get the structure ID.
    #[must_use]
    pub const fn structure_id(&self) -> MegastructureId {
        self.structure_id
    }

    /// Get the structure kind.
    #[must_use]
    pub const fn kind(&self) -> StructureKind {
        self.kind
    }

    /// Get the anchor.
    #[must_use]
    pub const fn anchor(&self) -> &StructureAnchor {
        &self.anchor
    }

    /// Get the bounds.
    #[must_use]
    pub const fn bounds(&self) -> &ChunkBounds {
        &self.bounds
    }

    /// Get the manifest version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Increment the version.
    pub fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// Add or update an entry.
    pub fn insert(&mut self, entry: ManifestEntry) {
        self.entries.insert(entry.offset, entry);
    }

    /// Get an entry by offset.
    #[must_use]
    pub fn get(&self, offset: (i32, i32, i32)) -> Option<&ManifestEntry> {
        self.entries.get(&offset)
    }

    /// Get all entries.
    pub fn entries(&self) -> impl Iterator<Item = &ManifestEntry> {
        self.entries.values()
    }

    /// Get entries sorted by streaming priority.
    #[must_use]
    pub fn by_priority(&self) -> Vec<&ManifestEntry> {
        let mut sorted: Vec<_> = self.entries.values().collect();
        sorted.sort_by_key(|e| (e.tier.as_priority(), e.offset));
        sorted
    }

    /// Get entries in a specific tier.
    pub fn in_tier(&self, tier: StreamingTier) -> impl Iterator<Item = &ManifestEntry> {
        self.entries.values().filter(move |e| e.tier == tier)
    }

    /// Get entries in a specific zone.
    pub fn in_zone(&self, zone: StructureZone) -> impl Iterator<Item = &ManifestEntry> {
        self.entries.values().filter(move |e| e.zone == zone)
    }

    /// Count entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total estimated size in bytes.
    #[must_use]
    pub fn total_size_estimate(&self) -> u64 {
        self.entries
            .values()
            .map(|e| u64::from(e.size_estimate))
            .sum()
    }

    /// Build manifest entries from slice data.
    pub fn from_slices<'a>(
        structure_id: MegastructureId,
        kind: StructureKind,
        anchor: StructureAnchor,
        bounds: ChunkBounds,
        slices: impl Iterator<Item = &'a ChunkSlice>,
    ) -> Self {
        let mut manifest = Self::new(structure_id, kind, anchor, bounds);

        for slice in slices {
            let tier = zone_to_tier(slice.zone());
            let entry = ManifestEntry::new(slice.offset(), slice.zone()).with_tier(tier);
            manifest.insert(entry);
        }

        manifest
    }

    /// Get chunks that are ready to load (dependencies satisfied).
    pub fn ready_to_load<'a>(
        &'a self,
        loaded: &'a impl Fn((i32, i32, i32)) -> bool,
    ) -> impl Iterator<Item = &'a ManifestEntry> {
        self.entries
            .values()
            .filter(move |e| !loaded(e.offset) && e.dependencies.iter().all(|&dep| loaded(dep)))
    }
}

/// Map zone to default streaming tier.
fn zone_to_tier(zone: StructureZone) -> StreamingTier {
    match zone {
        StructureZone::Interior => StreamingTier::High,
        StructureZone::Hull | StructureZone::Wall => StreamingTier::Normal,
        StructureZone::Exterior => StreamingTier::Low,
    }
}

/// Query for finding structures to stream.
#[derive(Clone, Debug, Default)]
pub struct StreamingQuery {
    /// Observer chunk positions.
    pub observers: Vec<ChunkPos>,
    /// Maximum distance to consider (in chunks).
    pub max_distance: i32,
    /// Maximum structures to return.
    pub limit: usize,
    /// Filter by structure kind.
    pub kind_filter: Option<StructureKind>,
}

impl StreamingQuery {
    /// Create a new query centered on an observer.
    #[must_use]
    pub fn from_observer(observer: ChunkPos, max_distance: i32) -> Self {
        Self {
            observers: vec![observer],
            max_distance,
            limit: 100,
            kind_filter: None,
        }
    }

    /// Add an observer.
    #[must_use]
    pub fn with_observer(mut self, observer: ChunkPos) -> Self {
        self.observers.push(observer);
        self
    }

    /// Set the result limit.
    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Filter by structure kind.
    #[must_use]
    pub const fn with_kind(mut self, kind: StructureKind) -> Self {
        self.kind_filter = Some(kind);
        self
    }

    /// Check if a structure should be included based on distance.
    #[must_use]
    pub fn in_range(&self, anchor_chunk: ChunkPos) -> bool {
        if self.observers.is_empty() {
            return false;
        }

        self.observers
            .iter()
            .any(|obs| obs.chebyshev_distance(anchor_chunk) <= self.max_distance)
    }
}

#[cfg(test)]
mod tests {
    use glam::IVec3;

    use super::*;

    fn test_manifest() -> StreamingManifest {
        let id = MegastructureId::new(1, 1);
        let anchor = StructureAnchor::new(IVec3::ZERO);
        let bounds = ChunkBounds::new(IVec3::new(-1, -1, -1), IVec3::new(1, 1, 1));
        StreamingManifest::new(id, StructureKind::Station, anchor, bounds)
    }

    #[test]
    fn test_streaming_tier_priority() {
        assert!(StreamingTier::Critical.as_priority() < StreamingTier::High.as_priority());
        assert!(StreamingTier::High.as_priority() < StreamingTier::Normal.as_priority());
        assert!(StreamingTier::Normal.as_priority() < StreamingTier::Low.as_priority());
        assert!(StreamingTier::Low.as_priority() < StreamingTier::Background.as_priority());
    }

    #[test]
    fn test_manifest_entry_builder() {
        let entry = ManifestEntry::new((1, 2, 3), StructureZone::Interior)
            .with_tier(StreamingTier::High)
            .with_size(1024)
            .with_dependency((0, 0, 0));

        assert_eq!(entry.offset, (1, 2, 3));
        assert_eq!(entry.tier, StreamingTier::High);
        assert_eq!(entry.size_estimate, 1024);
        assert_eq!(entry.dependencies.len(), 1);
    }

    #[test]
    fn test_manifest_insert_get() {
        let mut manifest = test_manifest();
        let entry = ManifestEntry::new((0, 0, 0), StructureZone::Hull);
        manifest.insert(entry);

        assert_eq!(manifest.len(), 1);
        assert!(manifest.get((0, 0, 0)).is_some());
        assert!(manifest.get((1, 1, 1)).is_none());
    }

    #[test]
    fn test_manifest_by_priority() {
        let mut manifest = test_manifest();
        manifest.insert(
            ManifestEntry::new((0, 0, 0), StructureZone::Hull).with_tier(StreamingTier::Low),
        );
        manifest.insert(
            ManifestEntry::new((1, 0, 0), StructureZone::Interior)
                .with_tier(StreamingTier::Critical),
        );
        manifest.insert(
            ManifestEntry::new((2, 0, 0), StructureZone::Exterior)
                .with_tier(StreamingTier::Background),
        );

        let sorted = manifest.by_priority();
        assert_eq!(sorted[0].offset, (1, 0, 0));
        assert_eq!(sorted[1].offset, (0, 0, 0));
        assert_eq!(sorted[2].offset, (2, 0, 0));
    }

    #[test]
    fn test_manifest_in_tier() {
        let mut manifest = test_manifest();
        manifest.insert(
            ManifestEntry::new((0, 0, 0), StructureZone::Hull).with_tier(StreamingTier::Normal),
        );
        manifest.insert(
            ManifestEntry::new((1, 0, 0), StructureZone::Hull).with_tier(StreamingTier::Normal),
        );
        manifest.insert(
            ManifestEntry::new((2, 0, 0), StructureZone::Hull).with_tier(StreamingTier::High),
        );

        let normal: Vec<_> = manifest.in_tier(StreamingTier::Normal).collect();
        assert_eq!(normal.len(), 2);
    }

    #[test]
    fn test_manifest_total_size() {
        let mut manifest = test_manifest();
        manifest.insert(ManifestEntry::new((0, 0, 0), StructureZone::Hull).with_size(1000));
        manifest.insert(ManifestEntry::new((1, 0, 0), StructureZone::Hull).with_size(2000));

        assert_eq!(manifest.total_size_estimate(), 3000);
    }

    #[test]
    fn test_manifest_ready_to_load() {
        let mut manifest = test_manifest();
        manifest.insert(ManifestEntry::new((0, 0, 0), StructureZone::Hull));
        manifest
            .insert(ManifestEntry::new((1, 0, 0), StructureZone::Hull).with_dependency((0, 0, 0)));

        let loaded = |offset: (i32, i32, i32)| offset == (0, 0, 0);
        let ready: Vec<_> = manifest.ready_to_load(&loaded).collect();

        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].offset, (1, 0, 0));
    }

    #[test]
    fn test_manifest_version() {
        let mut manifest = test_manifest();
        assert_eq!(manifest.version(), 1);

        manifest.bump_version();
        assert_eq!(manifest.version(), 2);
    }

    #[test]
    fn test_streaming_query_in_range() {
        let query = StreamingQuery::from_observer(ChunkPos::new(0, 0, 0), 5);

        assert!(query.in_range(ChunkPos::new(0, 0, 0)));
        assert!(query.in_range(ChunkPos::new(3, 3, 3)));
        assert!(!query.in_range(ChunkPos::new(10, 0, 0)));
    }

    #[test]
    fn test_streaming_query_multiple_observers() {
        let query = StreamingQuery::from_observer(ChunkPos::new(0, 0, 0), 3)
            .with_observer(ChunkPos::new(20, 0, 0));

        assert!(query.in_range(ChunkPos::new(0, 0, 0)));
        assert!(query.in_range(ChunkPos::new(20, 0, 0)));
        assert!(!query.in_range(ChunkPos::new(10, 0, 0)));
    }

    #[test]
    fn test_serde_manifest_entry() {
        let entry = ManifestEntry::new((1, 2, 3), StructureZone::Interior)
            .with_tier(StreamingTier::High)
            .with_size(1024);

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: ManifestEntry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(entry.offset, deserialized.offset);
        assert_eq!(entry.tier, deserialized.tier);
        assert_eq!(entry.size_estimate, deserialized.size_estimate);
    }

    #[test]
    fn test_serde_manifest() {
        let mut manifest = test_manifest();
        manifest.insert(ManifestEntry::new((0, 0, 0), StructureZone::Hull));

        let serialized = bincode::serialize(&manifest).unwrap();
        let deserialized: StreamingManifest = bincode::deserialize(&serialized).unwrap();

        assert_eq!(manifest.structure_id(), deserialized.structure_id());
        assert_eq!(manifest.len(), deserialized.len());
    }
}

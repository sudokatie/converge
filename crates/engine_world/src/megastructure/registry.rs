//! Megastructure registry for managing large multi-chunk structures.

use std::collections::{BTreeMap, HashMap};

use engine_core::coords::ChunkPos;
use glam::IVec3;
use serde::{Deserialize, Serialize};

use super::{
    anchor::{AnchorMetadata, StructureAnchor},
    bounds::{ChunkBounds, ChunkMask},
    manifest::{StreamingManifest, StreamingQuery},
    slice::{ChunkSlice, SliceMap, SliceState},
    structure_id::MegastructureId,
    structure_kind::{StructureKind, StructureZone},
};

/// A complete megastructure definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Megastructure {
    /// Unique identifier.
    id: MegastructureId,
    /// Structure category.
    kind: StructureKind,
    /// Anchor point and orientation.
    anchor: StructureAnchor,
    /// Metadata (name, tags, etc).
    metadata: AnchorMetadata,
    /// Bounding box in chunk space.
    bounds: ChunkBounds,
    /// Chunks owned by this structure.
    owned_chunks: ChunkMask,
    /// Per-chunk slice tracking.
    slices: SliceMap,
}

impl Megastructure {
    /// Create a new megastructure.
    #[must_use]
    pub fn new(
        id: MegastructureId,
        kind: StructureKind,
        anchor: StructureAnchor,
        bounds: ChunkBounds,
    ) -> Self {
        Self {
            id,
            kind,
            anchor,
            metadata: AnchorMetadata::default(),
            bounds,
            owned_chunks: ChunkMask::new(),
            slices: SliceMap::new(),
        }
    }

    /// Get the structure ID.
    #[must_use]
    pub const fn id(&self) -> MegastructureId {
        self.id
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

    /// Get mutable anchor.
    pub fn anchor_mut(&mut self) -> &mut StructureAnchor {
        &mut self.anchor
    }

    /// Get metadata.
    #[must_use]
    pub const fn metadata(&self) -> &AnchorMetadata {
        &self.metadata
    }

    /// Get mutable metadata.
    pub fn metadata_mut(&mut self) -> &mut AnchorMetadata {
        &mut self.metadata
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, metadata: AnchorMetadata) {
        self.metadata = metadata;
    }

    /// Get the bounds.
    #[must_use]
    pub const fn bounds(&self) -> &ChunkBounds {
        &self.bounds
    }

    /// Set new bounds.
    pub fn set_bounds(&mut self, bounds: ChunkBounds) {
        self.bounds = bounds;
    }

    /// Get owned chunks.
    #[must_use]
    pub const fn owned_chunks(&self) -> &ChunkMask {
        &self.owned_chunks
    }

    /// Add a chunk to ownership.
    pub fn add_chunk(&mut self, pos: ChunkPos, zone: StructureZone) {
        if self.owned_chunks.insert(pos) {
            let offset = SliceMap::world_to_offset(pos, self.anchor.chunk());
            let slice = ChunkSlice::new(offset, zone);
            self.slices.insert(slice);
            self.bounds = self.bounds.expanded_to(pos);
        }
    }

    /// Remove a chunk from ownership.
    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        if self.owned_chunks.remove(pos) {
            let offset = SliceMap::world_to_offset(pos, self.anchor.chunk());
            self.slices.remove(offset);
        }
    }

    /// Check if a world chunk is owned by this structure.
    #[must_use]
    pub fn owns_chunk(&self, pos: ChunkPos) -> bool {
        self.owned_chunks.contains(pos)
    }

    /// Get the slice map.
    #[must_use]
    pub const fn slices(&self) -> &SliceMap {
        &self.slices
    }

    /// Get mutable slice map.
    pub fn slices_mut(&mut self) -> &mut SliceMap {
        &mut self.slices
    }

    /// Count owned chunks.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.owned_chunks.len()
    }

    /// Check if a chunk is within the bounding box.
    #[must_use]
    pub fn in_bounds(&self, pos: ChunkPos) -> bool {
        self.bounds.contains(pos)
    }

    /// Get the slice for a world chunk position.
    #[must_use]
    pub fn get_slice(&self, pos: ChunkPos) -> Option<&ChunkSlice> {
        let offset = SliceMap::world_to_offset(pos, self.anchor.chunk());
        self.slices.get(offset)
    }

    /// Get mutable slice for a world chunk position.
    pub fn get_slice_mut(&mut self, pos: ChunkPos) -> Option<&mut ChunkSlice> {
        let offset = SliceMap::world_to_offset(pos, self.anchor.chunk());
        self.slices.get_mut(offset)
    }

    /// Mark a slice as loaded.
    pub fn mark_loaded(&mut self, pos: ChunkPos) {
        if let Some(slice) = self.get_slice_mut(pos) {
            slice.set_state(SliceState::Loaded);
        }
    }

    /// Mark a slice as dirty.
    pub fn mark_dirty(&mut self, pos: ChunkPos) {
        if let Some(slice) = self.get_slice_mut(pos) {
            slice.mark_dirty();
            slice.bump_generation();
        }
    }

    /// Build a streaming manifest for this structure.
    #[must_use]
    pub fn build_manifest(&self) -> StreamingManifest {
        StreamingManifest::from_slices(
            self.id,
            self.kind,
            self.anchor,
            self.bounds,
            self.slices.iter(),
        )
    }

    /// Calculate distance from anchor to a point.
    #[must_use]
    pub fn distance_to(&self, pos: ChunkPos) -> i32 {
        self.anchor.chunk().chebyshev_distance(pos)
    }

    /// Iterate over world chunk positions owned by this structure.
    pub fn iter_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.owned_chunks.iter()
    }

    /// Get dirty chunk count.
    #[must_use]
    pub fn dirty_count(&self) -> usize {
        self.slices.dirty_count()
    }

    /// Get loaded chunk count.
    #[must_use]
    pub fn loaded_count(&self) -> usize {
        self.slices.count_by_state(SliceState::Loaded)
    }
}

/// ID generator for megastructures.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IdGenerator {
    seed: u32,
    next_sequence: u32,
}

impl IdGenerator {
    /// Create a new generator with the given seed.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            next_sequence: 0,
        }
    }

    /// Generate the next ID.
    pub fn generate(&mut self) -> MegastructureId {
        let id = MegastructureId::new(self.seed, self.next_sequence);
        self.next_sequence = self.next_sequence.wrapping_add(1);
        id
    }

    /// Get the current seed.
    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    /// Get the next sequence number without advancing.
    #[must_use]
    pub const fn peek_sequence(&self) -> u32 {
        self.next_sequence
    }
}

/// Registry managing all megastructures in a world.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MegastructureRegistry {
    /// All structures indexed by ID.
    structures: BTreeMap<MegastructureId, Megastructure>,
    /// Spatial index: chunk position -> structure ID.
    chunk_index: HashMap<(i32, i32, i32), MegastructureId>,
    /// ID generator.
    id_gen: IdGenerator,
}

impl MegastructureRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            structures: BTreeMap::new(),
            chunk_index: HashMap::new(),
            id_gen: IdGenerator::new(seed),
        }
    }

    /// Generate a new structure ID.
    pub fn generate_id(&mut self) -> MegastructureId {
        self.id_gen.generate()
    }

    /// Register a new megastructure.
    ///
    /// Returns `false` if any owned chunks conflict with existing structures.
    pub fn register(&mut self, structure: Megastructure) -> bool {
        for chunk in structure.iter_chunks() {
            let key = (chunk.x(), chunk.y(), chunk.z());
            if self.chunk_index.contains_key(&key) {
                return false;
            }
        }

        let id = structure.id();
        for chunk in structure.iter_chunks() {
            let key = (chunk.x(), chunk.y(), chunk.z());
            self.chunk_index.insert(key, id);
        }
        self.structures.insert(id, structure);

        true
    }

    /// Unregister a megastructure.
    pub fn unregister(&mut self, id: MegastructureId) -> Option<Megastructure> {
        if let Some(structure) = self.structures.remove(&id) {
            for chunk in structure.iter_chunks() {
                let key = (chunk.x(), chunk.y(), chunk.z());
                self.chunk_index.remove(&key);
            }
            Some(structure)
        } else {
            None
        }
    }

    /// Get a structure by ID.
    #[must_use]
    pub fn get(&self, id: MegastructureId) -> Option<&Megastructure> {
        self.structures.get(&id)
    }

    /// Get a mutable structure by ID.
    pub fn get_mut(&mut self, id: MegastructureId) -> Option<&mut Megastructure> {
        self.structures.get_mut(&id)
    }

    /// Find the structure owning a chunk position.
    #[must_use]
    pub fn find_at_chunk(&self, pos: ChunkPos) -> Option<&Megastructure> {
        let key = (pos.x(), pos.y(), pos.z());
        self.chunk_index
            .get(&key)
            .and_then(|&id| self.structures.get(&id))
    }

    /// Find the structure ID owning a chunk position.
    #[must_use]
    pub fn find_id_at_chunk(&self, pos: ChunkPos) -> Option<MegastructureId> {
        let key = (pos.x(), pos.y(), pos.z());
        self.chunk_index.get(&key).copied()
    }

    /// Check if a chunk is owned by any structure.
    #[must_use]
    pub fn is_chunk_owned(&self, pos: ChunkPos) -> bool {
        let key = (pos.x(), pos.y(), pos.z());
        self.chunk_index.contains_key(&key)
    }

    /// Count registered structures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.structures.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.structures.is_empty()
    }

    /// Iterate over all structures.
    pub fn iter(&self) -> impl Iterator<Item = &Megastructure> {
        self.structures.values()
    }

    /// Iterate over structure IDs.
    pub fn iter_ids(&self) -> impl Iterator<Item = MegastructureId> + '_ {
        self.structures.keys().copied()
    }

    /// Find structures by kind.
    pub fn by_kind(&self, kind: StructureKind) -> impl Iterator<Item = &Megastructure> {
        self.structures.values().filter(move |s| s.kind() == kind)
    }

    /// Find structures within range of observers.
    #[must_use]
    pub fn query(&self, query: &StreamingQuery) -> Vec<&Megastructure> {
        let mut results: Vec<_> = self
            .structures
            .values()
            .filter(|s| {
                query.in_range(s.anchor().chunk())
                    && query.kind_filter.is_none_or(|k| s.kind() == k)
            })
            .collect();

        results.sort_by_key(|s| {
            let distance = query
                .observers
                .iter()
                .map(|&obs| s.anchor().chunk().chebyshev_distance(obs))
                .min()
                .unwrap_or(i32::MAX);
            (s.kind().streaming_priority(), distance)
        });

        if results.len() > query.limit {
            results.truncate(query.limit);
        }

        results
    }

    /// Get total owned chunk count across all structures.
    #[must_use]
    pub fn total_chunks(&self) -> usize {
        self.chunk_index.len()
    }

    /// Update chunk ownership when a structure's anchor moves.
    ///
    /// This rebuilds the chunk index for the structure.
    pub fn update_chunk_index(&mut self, id: MegastructureId) {
        if let Some(structure) = self.structures.get(&id) {
            self.chunk_index
                .retain(|_, &mut structure_id| structure_id != id);

            for chunk in structure.iter_chunks() {
                let key = (chunk.x(), chunk.y(), chunk.z());
                self.chunk_index.insert(key, id);
            }
        }
    }

    /// Create a simple station structure.
    pub fn create_station(
        &mut self,
        anchor_pos: IVec3,
        size: IVec3,
        metadata: AnchorMetadata,
    ) -> MegastructureId {
        let id = self.generate_id();
        let anchor = StructureAnchor::new(anchor_pos);
        let anchor_chunk = anchor.chunk();

        let half = size / 2;
        let min = IVec3::new(
            anchor_chunk.x() - half.x,
            anchor_chunk.y() - half.y,
            anchor_chunk.z() - half.z,
        );
        let max = IVec3::new(
            anchor_chunk.x() + half.x,
            anchor_chunk.y() + half.y,
            anchor_chunk.z() + half.z,
        );
        let bounds = ChunkBounds::new(min, max);

        let mut structure = Megastructure::new(id, StructureKind::Station, anchor, bounds);
        structure.set_metadata(metadata);

        for chunk_pos in bounds.iter_chunks() {
            let is_surface = chunk_pos.x() == min.x
                || chunk_pos.x() == max.x
                || chunk_pos.y() == min.y
                || chunk_pos.y() == max.y
                || chunk_pos.z() == min.z
                || chunk_pos.z() == max.z;

            let zone = if is_surface {
                StructureZone::Hull
            } else {
                StructureZone::Interior
            };

            structure.add_chunk(chunk_pos, zone);
        }

        self.register(structure);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_structure() -> Megastructure {
        let id = MegastructureId::new(1, 1);
        let anchor = StructureAnchor::new(IVec3::ZERO);
        let bounds = ChunkBounds::new(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        Megastructure::new(id, StructureKind::Station, anchor, bounds)
    }

    #[test]
    fn test_megastructure_new() {
        let structure = test_structure();
        assert_eq!(structure.kind(), StructureKind::Station);
        assert_eq!(structure.chunk_count(), 0);
    }

    #[test]
    fn test_megastructure_add_chunk() {
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);
        structure.add_chunk(ChunkPos::new(1, 0, 0), StructureZone::Interior);

        assert_eq!(structure.chunk_count(), 2);
        assert!(structure.owns_chunk(ChunkPos::new(0, 0, 0)));
        assert!(structure.owns_chunk(ChunkPos::new(1, 0, 0)));
        assert!(!structure.owns_chunk(ChunkPos::new(5, 5, 5)));
    }

    #[test]
    fn test_megastructure_remove_chunk() {
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);
        structure.remove_chunk(ChunkPos::new(0, 0, 0));

        assert_eq!(structure.chunk_count(), 0);
    }

    #[test]
    fn test_megastructure_slice_access() {
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Interior);

        let slice = structure.get_slice(ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(slice.zone(), StructureZone::Interior);
    }

    #[test]
    fn test_megastructure_mark_loaded_dirty() {
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);

        structure.mark_loaded(ChunkPos::new(0, 0, 0));
        assert_eq!(structure.loaded_count(), 1);

        structure.mark_dirty(ChunkPos::new(0, 0, 0));
        assert_eq!(structure.dirty_count(), 1);
    }

    #[test]
    fn test_id_generator() {
        let mut generator = IdGenerator::new(42);

        let id1 = generator.generate();
        let id2 = generator.generate();

        assert_eq!(id1.seed(), 42);
        assert_eq!(id1.sequence(), 0);
        assert_eq!(id2.sequence(), 1);
    }

    #[test]
    fn test_registry_register_unregister() {
        let mut registry = MegastructureRegistry::new(1);
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);
        let id = structure.id();

        assert!(registry.register(structure));
        assert_eq!(registry.len(), 1);
        assert!(registry.get(id).is_some());

        let removed = registry.unregister(id);
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_conflict() {
        let mut registry = MegastructureRegistry::new(1);

        let mut s1 = Megastructure::new(
            MegastructureId::new(1, 1),
            StructureKind::Station,
            StructureAnchor::new(IVec3::ZERO),
            ChunkBounds::default(),
        );
        s1.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);

        let mut s2 = Megastructure::new(
            MegastructureId::new(1, 2),
            StructureKind::Station,
            StructureAnchor::new(IVec3::ZERO),
            ChunkBounds::default(),
        );
        s2.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);

        assert!(registry.register(s1));
        assert!(!registry.register(s2));
    }

    #[test]
    fn test_registry_find_at_chunk() {
        let mut registry = MegastructureRegistry::new(1);
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(5, 5, 5), StructureZone::Hull);
        let id = structure.id();
        registry.register(structure);

        let found = registry.find_at_chunk(ChunkPos::new(5, 5, 5));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), id);

        assert!(registry.find_at_chunk(ChunkPos::new(0, 0, 0)).is_none());
    }

    #[test]
    fn test_registry_query() {
        let mut registry = MegastructureRegistry::new(1);

        let mut s1 = Megastructure::new(
            MegastructureId::new(1, 1),
            StructureKind::Station,
            StructureAnchor::new(IVec3::new(0, 0, 0)),
            ChunkBounds::default(),
        );
        s1.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);

        let mut s2 = Megastructure::new(
            MegastructureId::new(1, 2),
            StructureKind::Station,
            StructureAnchor::new(IVec3::new(160, 0, 0)),
            ChunkBounds::default(),
        );
        s2.add_chunk(ChunkPos::new(10, 0, 0), StructureZone::Hull);

        registry.register(s1);
        registry.register(s2);

        let query = StreamingQuery::from_observer(ChunkPos::new(0, 0, 0), 5);
        let results = registry.query(&query);

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_registry_by_kind() {
        let mut registry = MegastructureRegistry::new(1);

        let mut station = Megastructure::new(
            MegastructureId::new(1, 1),
            StructureKind::Station,
            StructureAnchor::new(IVec3::ZERO),
            ChunkBounds::default(),
        );
        station.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);

        let mut titan = Megastructure::new(
            MegastructureId::new(1, 2),
            StructureKind::Titan,
            StructureAnchor::new(IVec3::new(100, 0, 0)),
            ChunkBounds::default(),
        );
        titan.add_chunk(ChunkPos::new(10, 0, 0), StructureZone::Hull);

        registry.register(station);
        registry.register(titan);

        let stations: Vec<_> = registry.by_kind(StructureKind::Station).collect();
        assert_eq!(stations.len(), 1);

        let titans: Vec<_> = registry.by_kind(StructureKind::Titan).collect();
        assert_eq!(titans.len(), 1);
    }

    #[test]
    fn test_registry_create_station() {
        let mut registry = MegastructureRegistry::new(42);
        let metadata = AnchorMetadata::named("Test Station");
        let id = registry.create_station(IVec3::new(32, 32, 32), IVec3::new(3, 2, 3), metadata);

        let structure = registry.get(id).unwrap();
        assert_eq!(structure.kind(), StructureKind::Station);
        assert!(structure.chunk_count() > 0);
        assert_eq!(structure.metadata().name.as_deref(), Some("Test Station"));
    }

    #[test]
    fn test_serde_megastructure() {
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);
        structure.set_metadata(AnchorMetadata::named("Test"));

        let serialized = bincode::serialize(&structure).unwrap();
        let deserialized: Megastructure = bincode::deserialize(&serialized).unwrap();

        assert_eq!(structure.id(), deserialized.id());
        assert_eq!(structure.kind(), deserialized.kind());
        assert_eq!(structure.chunk_count(), deserialized.chunk_count());
    }

    #[test]
    fn test_serde_registry() {
        let mut registry = MegastructureRegistry::new(1);
        let mut structure = test_structure();
        structure.add_chunk(ChunkPos::new(0, 0, 0), StructureZone::Hull);
        registry.register(structure);

        let serialized = bincode::serialize(&registry).unwrap();
        let deserialized: MegastructureRegistry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(registry.len(), deserialized.len());
    }

    #[test]
    fn test_iter_deterministic() {
        let mut registry = MegastructureRegistry::new(1);

        for i in 0_u32..5 {
            let idx = i32::try_from(i).expect("index fits in i32");
            let mut structure = Megastructure::new(
                MegastructureId::new(1, i),
                StructureKind::Station,
                StructureAnchor::new(IVec3::new(idx * 100, 0, 0)),
                ChunkBounds::default(),
            );
            structure.add_chunk(ChunkPos::new(idx * 10, 0, 0), StructureZone::Hull);
            registry.register(structure);
        }

        let ids1: Vec<_> = registry.iter_ids().collect();
        let ids2: Vec<_> = registry.iter_ids().collect();
        assert_eq!(ids1, ids2);
    }
}

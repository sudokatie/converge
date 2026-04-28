//! Per-chunk slice tracking for megastructures.

use std::collections::BTreeMap;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::structure_kind::StructureZone;

/// Load state of a chunk slice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SliceState {
    /// Not loaded, data not in memory.
    #[default]
    Unloaded,
    /// Currently being loaded/generated.
    Loading,
    /// Fully loaded and ready.
    Loaded,
    /// Loaded but marked for unload.
    PendingUnload,
}

impl SliceState {
    /// Check if the slice is available for reading.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Loaded | Self::PendingUnload)
    }

    /// Check if the slice is in a transitional state.
    #[must_use]
    pub const fn is_transitioning(self) -> bool {
        matches!(self, Self::Loading | Self::PendingUnload)
    }
}

/// Tracking information for a single chunk slice of a megastructure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkSlice {
    /// Position relative to structure anchor.
    offset: (i32, i32, i32),
    /// Zone classification.
    zone: StructureZone,
    /// Current load state.
    state: SliceState,
    /// Dirty flag for persistence.
    dirty: bool,
    /// Generation/version number for cache invalidation.
    generation: u32,
    /// Priority for streaming (lower = higher priority).
    priority: u16,
}

impl ChunkSlice {
    /// Create a new unloaded slice.
    #[must_use]
    pub fn new(offset: (i32, i32, i32), zone: StructureZone) -> Self {
        Self {
            offset,
            zone,
            state: SliceState::Unloaded,
            dirty: false,
            generation: 0,
            priority: u16::MAX,
        }
    }

    /// Get the offset from structure anchor.
    #[must_use]
    pub const fn offset(&self) -> (i32, i32, i32) {
        self.offset
    }

    /// Get the zone classification.
    #[must_use]
    pub const fn zone(&self) -> StructureZone {
        self.zone
    }

    /// Get the current load state.
    #[must_use]
    pub const fn state(&self) -> SliceState {
        self.state
    }

    /// Set the load state.
    pub fn set_state(&mut self, state: SliceState) {
        self.state = state;
    }

    /// Check if the slice is dirty (needs persistence).
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the slice as dirty.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear the dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Get the generation number.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Increment the generation number.
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Get the streaming priority.
    #[must_use]
    pub const fn priority(&self) -> u16 {
        self.priority
    }

    /// Set the streaming priority.
    pub fn set_priority(&mut self, priority: u16) {
        self.priority = priority;
    }

    /// Set the zone classification.
    pub fn set_zone(&mut self, zone: StructureZone) {
        self.zone = zone;
    }
}

/// Collection of chunk slices with tracking state.
///
/// Uses `BTreeMap` for deterministic iteration order based on offset.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SliceMap {
    slices: BTreeMap<(i32, i32, i32), ChunkSlice>,
}

impl SliceMap {
    /// Create a new empty slice map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a slice.
    ///
    /// Returns the previous slice if one existed at this offset.
    pub fn insert(&mut self, slice: ChunkSlice) -> Option<ChunkSlice> {
        self.slices.insert(slice.offset, slice)
    }

    /// Get a slice by offset.
    #[must_use]
    pub fn get(&self, offset: (i32, i32, i32)) -> Option<&ChunkSlice> {
        self.slices.get(&offset)
    }

    /// Get a mutable slice by offset.
    pub fn get_mut(&mut self, offset: (i32, i32, i32)) -> Option<&mut ChunkSlice> {
        self.slices.get_mut(&offset)
    }

    /// Remove a slice.
    pub fn remove(&mut self, offset: (i32, i32, i32)) -> Option<ChunkSlice> {
        self.slices.remove(&offset)
    }

    /// Check if an offset has a slice.
    #[must_use]
    pub fn contains(&self, offset: (i32, i32, i32)) -> bool {
        self.slices.contains_key(&offset)
    }

    /// Get the number of slices.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slices.len()
    }

    /// Check if the map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    /// Iterate over all slices in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &ChunkSlice> {
        self.slices.values()
    }

    /// Iterate over all slices mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ChunkSlice> {
        self.slices.values_mut()
    }

    /// Iterate over slices with their offsets.
    pub fn iter_with_offsets(&self) -> impl Iterator<Item = ((i32, i32, i32), &ChunkSlice)> {
        self.slices.iter().map(|(&k, v)| (k, v))
    }

    /// Get all loaded slices.
    pub fn loaded(&self) -> impl Iterator<Item = &ChunkSlice> {
        self.slices
            .values()
            .filter(|s| s.state() == SliceState::Loaded)
    }

    /// Get all dirty slices.
    pub fn dirty(&self) -> impl Iterator<Item = &ChunkSlice> {
        self.slices.values().filter(|s| s.is_dirty())
    }

    /// Count slices by state.
    #[must_use]
    pub fn count_by_state(&self, state: SliceState) -> usize {
        self.slices.values().filter(|s| s.state() == state).count()
    }

    /// Count dirty slices.
    #[must_use]
    pub fn dirty_count(&self) -> usize {
        self.slices.values().filter(|s| s.is_dirty()).count()
    }

    /// Get slices sorted by priority (for streaming).
    #[must_use]
    pub fn by_priority(&self) -> Vec<&ChunkSlice> {
        let mut slices: Vec<_> = self.slices.values().collect();
        slices.sort_by_key(|s| s.priority());
        slices
    }

    /// Clear all dirty flags.
    pub fn clear_all_dirty(&mut self) {
        for slice in self.slices.values_mut() {
            slice.clear_dirty();
        }
    }

    /// Set state for all slices matching a predicate.
    pub fn set_state_where<F>(&mut self, state: SliceState, predicate: F)
    where
        F: Fn(&ChunkSlice) -> bool,
    {
        for slice in self.slices.values_mut() {
            if predicate(slice) {
                slice.set_state(state);
            }
        }
    }

    /// Get slices within a zone.
    pub fn in_zone(&self, zone: StructureZone) -> impl Iterator<Item = &ChunkSlice> {
        self.slices.values().filter(move |s| s.zone() == zone)
    }

    /// Convert offset to world chunk position given an anchor chunk.
    #[must_use]
    pub fn offset_to_world(offset: (i32, i32, i32), anchor: ChunkPos) -> ChunkPos {
        ChunkPos::new(
            anchor.x() + offset.0,
            anchor.y() + offset.1,
            anchor.z() + offset.2,
        )
    }

    /// Convert world chunk position to offset given an anchor chunk.
    #[must_use]
    pub fn world_to_offset(pos: ChunkPos, anchor: ChunkPos) -> (i32, i32, i32) {
        (
            pos.x() - anchor.x(),
            pos.y() - anchor.y(),
            pos.z() - anchor.z(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_state_is_available() {
        assert!(!SliceState::Unloaded.is_available());
        assert!(!SliceState::Loading.is_available());
        assert!(SliceState::Loaded.is_available());
        assert!(SliceState::PendingUnload.is_available());
    }

    #[test]
    fn test_slice_state_is_transitioning() {
        assert!(!SliceState::Unloaded.is_transitioning());
        assert!(SliceState::Loading.is_transitioning());
        assert!(!SliceState::Loaded.is_transitioning());
        assert!(SliceState::PendingUnload.is_transitioning());
    }

    #[test]
    fn test_chunk_slice_new() {
        let slice = ChunkSlice::new((1, 2, 3), StructureZone::Interior);
        assert_eq!(slice.offset(), (1, 2, 3));
        assert_eq!(slice.zone(), StructureZone::Interior);
        assert_eq!(slice.state(), SliceState::Unloaded);
        assert!(!slice.is_dirty());
    }

    #[test]
    fn test_chunk_slice_dirty() {
        let mut slice = ChunkSlice::new((0, 0, 0), StructureZone::Hull);
        assert!(!slice.is_dirty());

        slice.mark_dirty();
        assert!(slice.is_dirty());

        slice.clear_dirty();
        assert!(!slice.is_dirty());
    }

    #[test]
    fn test_chunk_slice_generation() {
        let mut slice = ChunkSlice::new((0, 0, 0), StructureZone::Exterior);
        assert_eq!(slice.generation(), 0);

        slice.bump_generation();
        assert_eq!(slice.generation(), 1);

        slice.bump_generation();
        assert_eq!(slice.generation(), 2);
    }

    #[test]
    fn test_slice_map_insert_get() {
        let mut map = SliceMap::new();
        let slice = ChunkSlice::new((1, 2, 3), StructureZone::Interior);

        assert!(map.insert(slice.clone()).is_none());
        assert!(map.contains((1, 2, 3)));
        assert_eq!(map.get((1, 2, 3)).unwrap().zone(), StructureZone::Interior);
    }

    #[test]
    fn test_slice_map_remove() {
        let mut map = SliceMap::new();
        map.insert(ChunkSlice::new((0, 0, 0), StructureZone::Hull));

        let removed = map.remove((0, 0, 0));
        assert!(removed.is_some());
        assert!(map.is_empty());
    }

    #[test]
    fn test_slice_map_iter_deterministic() {
        let mut map = SliceMap::new();
        map.insert(ChunkSlice::new((2, 0, 0), StructureZone::Hull));
        map.insert(ChunkSlice::new((0, 0, 0), StructureZone::Hull));
        map.insert(ChunkSlice::new((1, 0, 0), StructureZone::Hull));

        let offsets: Vec<_> = map.iter().map(ChunkSlice::offset).collect();
        assert_eq!(offsets, vec![(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    }

    #[test]
    fn test_slice_map_loaded() {
        let mut map = SliceMap::new();
        let mut s1 = ChunkSlice::new((0, 0, 0), StructureZone::Hull);
        s1.set_state(SliceState::Loaded);
        let s2 = ChunkSlice::new((1, 0, 0), StructureZone::Hull);
        map.insert(s1);
        map.insert(s2);

        let loaded: Vec<_> = map.loaded().collect();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_slice_map_dirty() {
        let mut map = SliceMap::new();
        let mut s1 = ChunkSlice::new((0, 0, 0), StructureZone::Hull);
        s1.mark_dirty();
        let s2 = ChunkSlice::new((1, 0, 0), StructureZone::Hull);
        map.insert(s1);
        map.insert(s2);

        assert_eq!(map.dirty_count(), 1);

        map.clear_all_dirty();
        assert_eq!(map.dirty_count(), 0);
    }

    #[test]
    fn test_slice_map_by_priority() {
        let mut map = SliceMap::new();
        let mut s1 = ChunkSlice::new((0, 0, 0), StructureZone::Hull);
        s1.set_priority(100);
        let mut s2 = ChunkSlice::new((1, 0, 0), StructureZone::Hull);
        s2.set_priority(50);
        let mut s3 = ChunkSlice::new((2, 0, 0), StructureZone::Hull);
        s3.set_priority(200);
        map.insert(s1);
        map.insert(s2);
        map.insert(s3);

        let sorted = map.by_priority();
        assert_eq!(sorted[0].offset(), (1, 0, 0));
        assert_eq!(sorted[1].offset(), (0, 0, 0));
        assert_eq!(sorted[2].offset(), (2, 0, 0));
    }

    #[test]
    fn test_slice_map_in_zone() {
        let mut map = SliceMap::new();
        map.insert(ChunkSlice::new((0, 0, 0), StructureZone::Interior));
        map.insert(ChunkSlice::new((1, 0, 0), StructureZone::Hull));
        map.insert(ChunkSlice::new((2, 0, 0), StructureZone::Interior));

        let interior: Vec<_> = map.in_zone(StructureZone::Interior).collect();
        assert_eq!(interior.len(), 2);
    }

    #[test]
    fn test_offset_to_world() {
        let anchor = ChunkPos::new(10, 20, 30);
        let offset = (5, -3, 2);
        let world = SliceMap::offset_to_world(offset, anchor);
        assert_eq!(world, ChunkPos::new(15, 17, 32));
    }

    #[test]
    fn test_world_to_offset() {
        let anchor = ChunkPos::new(10, 20, 30);
        let world = ChunkPos::new(15, 17, 32);
        let offset = SliceMap::world_to_offset(world, anchor);
        assert_eq!(offset, (5, -3, 2));
    }

    #[test]
    fn test_serde_slice() {
        let mut slice = ChunkSlice::new((1, 2, 3), StructureZone::Interior);
        slice.set_state(SliceState::Loaded);
        slice.mark_dirty();
        slice.set_priority(42);

        let serialized = bincode::serialize(&slice).unwrap();
        let deserialized: ChunkSlice = bincode::deserialize(&serialized).unwrap();

        assert_eq!(slice.offset(), deserialized.offset());
        assert_eq!(slice.zone(), deserialized.zone());
        assert_eq!(slice.state(), deserialized.state());
        assert_eq!(slice.is_dirty(), deserialized.is_dirty());
        assert_eq!(slice.priority(), deserialized.priority());
    }

    #[test]
    fn test_serde_slice_map() {
        let mut map = SliceMap::new();
        map.insert(ChunkSlice::new((0, 0, 0), StructureZone::Hull));
        map.insert(ChunkSlice::new((1, 2, 3), StructureZone::Interior));

        let serialized = bincode::serialize(&map).unwrap();
        let deserialized: SliceMap = bincode::deserialize(&serialized).unwrap();

        assert_eq!(map.len(), deserialized.len());
        assert!(deserialized.contains((0, 0, 0)));
        assert!(deserialized.contains((1, 2, 3)));
    }
}

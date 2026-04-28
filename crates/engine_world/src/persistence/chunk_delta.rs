//! Chunk delta/overlay storage for efficient variant representation.
//!
//! Provides a compact overlay abstraction that stores only changed blocks
//! relative to a base chunk, enabling memory-efficient storage of chunk
//! variants for alternate dimensions, time-loop snapshots, and phased realities.

use std::collections::BTreeMap;

use engine_core::coords::LocalPos;
use serde::{Deserialize, Serialize};

use crate::chunk::{AIR, BlockId, CHUNK_VOLUME, Chunk};

/// Maximum valid block index (`CHUNK_VOLUME` - 1).
#[expect(
    clippy::cast_possible_truncation,
    reason = "CHUNK_VOLUME is 4096 which fits in u16"
)]
const MAX_INDEX: u16 = (CHUNK_VOLUME - 1) as u16;

/// Compact position index for delta storage.
///
/// Uses u16 to represent positions 0-4095 (16^3 - 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeltaIndex(u16);

impl DeltaIndex {
    /// Create a new delta index from a raw value.
    ///
    /// # Panics
    /// Panics if index > `MAX_INDEX`.
    #[must_use]
    pub fn new(index: u16) -> Self {
        debug_assert!(index <= MAX_INDEX, "index out of bounds");
        Self(index)
    }

    /// Create from a `LocalPos`.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "LocalPos indices are bounded by CHUNK_VOLUME (4096) which fits in u16"
    )]
    pub fn from_local_pos(pos: LocalPos) -> Self {
        Self(pos.to_index() as u16)
    }

    /// Convert to a `LocalPos`.
    #[must_use]
    pub fn to_local_pos(self) -> LocalPos {
        LocalPos::from_index(usize::from(self.0))
    }

    /// Get the raw index value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl From<LocalPos> for DeltaIndex {
    fn from(pos: LocalPos) -> Self {
        Self::from_local_pos(pos)
    }
}

impl From<DeltaIndex> for LocalPos {
    fn from(index: DeltaIndex) -> Self {
        index.to_local_pos()
    }
}

/// A chunk overlay storing only changed blocks relative to a base.
///
/// `ChunkDelta` provides a compact representation of chunk modifications,
/// storing only positions that differ from an implicit base chunk.
/// Uses `BTreeMap` for deterministic iteration order.
///
/// # Storage Semantics
///
/// - Entries in the map represent blocks that differ from the base chunk.
/// - To revert a position to base, remove it from the delta.
/// - Empty delta means the variant is identical to base.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDelta {
    changes: BTreeMap<DeltaIndex, BlockId>,
}

impl ChunkDelta {
    /// Create a new empty delta.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a delta with pre-allocated capacity hint.
    ///
    /// Note: `BTreeMap` doesn't pre-allocate, but this method
    /// exists for API consistency.
    #[must_use]
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    /// Get the block override at a position, if any.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> Option<BlockId> {
        self.changes.get(&DeltaIndex::from(pos)).copied()
    }

    /// Get the block at a position, falling back to base chunk.
    #[must_use]
    pub fn get_with_base(&self, pos: LocalPos, base: &Chunk) -> BlockId {
        self.get(pos).unwrap_or_else(|| base.get(pos))
    }

    /// Set a block override at a position.
    ///
    /// Returns the previous override if one existed.
    pub fn set(&mut self, pos: LocalPos, block: BlockId) -> Option<BlockId> {
        self.changes.insert(DeltaIndex::from(pos), block)
    }

    /// Set a block only if it differs from the base.
    ///
    /// This avoids storing redundant entries that match the base.
    /// Returns `true` if the delta was modified.
    pub fn set_if_different(&mut self, pos: LocalPos, block: BlockId, base: &Chunk) -> bool {
        let base_block = base.get(pos);
        let index = DeltaIndex::from(pos);

        if block == base_block {
            self.changes.remove(&index).is_some()
        } else {
            self.changes.insert(index, block);
            true
        }
    }

    /// Remove an override, reverting the position to base.
    ///
    /// Returns the removed override if one existed.
    pub fn remove(&mut self, pos: LocalPos) -> Option<BlockId> {
        self.changes.remove(&DeltaIndex::from(pos))
    }

    /// Check if a position has an override.
    #[must_use]
    pub fn has_override(&self, pos: LocalPos) -> bool {
        self.changes.contains_key(&DeltaIndex::from(pos))
    }

    /// Get the number of overrides.
    #[must_use]
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Check if the delta has no overrides.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Clear all overrides.
    pub fn clear(&mut self) {
        self.changes.clear();
    }

    /// Iterate over all overrides in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (LocalPos, BlockId)> + '_ {
        self.changes
            .iter()
            .map(|(&idx, &block)| (idx.to_local_pos(), block))
    }

    /// Iterate over override positions only.
    pub fn positions(&self) -> impl Iterator<Item = LocalPos> + '_ {
        self.changes.keys().map(|idx| idx.to_local_pos())
    }

    /// Materialize the delta into a full chunk given a base.
    ///
    /// Creates a new chunk by applying all overrides to a clone of the base.
    #[must_use]
    pub fn materialize(&self, base: &Chunk) -> Chunk {
        let mut result = base.clone();
        for (&idx, &block) in &self.changes {
            result.set(idx.to_local_pos(), block);
        }
        result
    }

    /// Compute the delta between two chunks.
    ///
    /// Returns a delta that, when applied to `base`, produces `target`.
    #[must_use]
    pub fn diff(base: &Chunk, target: &Chunk) -> Self {
        let mut delta = Self::new();

        for (pos, target_block) in target.iter() {
            let base_block = base.get(pos);
            if base_block != target_block {
                delta.changes.insert(DeltaIndex::from(pos), target_block);
            }
        }

        delta
    }

    /// Merge another delta into this one.
    ///
    /// Overrides from `other` take precedence over existing entries.
    pub fn merge(&mut self, other: &Self) {
        for (&idx, &block) in &other.changes {
            self.changes.insert(idx, block);
        }
    }

    /// Merge another delta, consuming it.
    pub fn merge_owned(&mut self, other: Self) {
        for (idx, block) in other.changes {
            self.changes.insert(idx, block);
        }
    }

    /// Retain only overrides matching a predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(LocalPos, BlockId) -> bool,
    {
        self.changes
            .retain(|&idx, &mut block| f(idx.to_local_pos(), block));
    }

    /// Remove overrides that match the base chunk.
    ///
    /// Useful after operations that may have created redundant entries.
    pub fn compact(&mut self, base: &Chunk) {
        self.changes.retain(|&idx, &mut block| {
            let pos = idx.to_local_pos();
            base.get(pos) != block
        });
    }

    /// Rebase the delta onto a new base chunk.
    ///
    /// Adjusts the delta so that applying it to `new_base` produces
    /// the same result as applying the original delta to `old_base`.
    pub fn rebase(&mut self, old_base: &Chunk, new_base: &Chunk) {
        let materialized = self.materialize(old_base);
        *self = Self::diff(new_base, &materialized);
    }

    /// Create a delta representing all non-air blocks in a chunk.
    ///
    /// Useful for converting a full chunk to delta form against an empty base.
    #[must_use]
    pub fn from_chunk_non_air(chunk: &Chunk) -> Self {
        let mut delta = Self::new();
        for (pos, block) in chunk.iter_non_air() {
            delta.changes.insert(DeltaIndex::from(pos), block);
        }
        delta
    }

    /// Estimate memory usage in bytes.
    ///
    /// Approximate: `BTreeMap` overhead + entries.
    #[must_use]
    pub fn memory_estimate(&self) -> usize {
        // BTreeMap node overhead (~40 bytes) + entry size (4 bytes per entry)
        const NODE_OVERHEAD: usize = 40;
        const ENTRY_SIZE: usize = 4; // u16 + BlockId(u16)

        NODE_OVERHEAD + self.changes.len() * ENTRY_SIZE
    }
}

/// Statistics about a chunk delta.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeltaStats {
    /// Number of block overrides.
    pub override_count: usize,
    /// Number of air block overrides.
    pub air_overrides: usize,
    /// Number of non-air block overrides.
    pub solid_overrides: usize,
}

impl ChunkDelta {
    /// Compute statistics about this delta.
    #[must_use]
    pub fn stats(&self) -> DeltaStats {
        let mut stats = DeltaStats {
            override_count: self.len(),
            ..Default::default()
        };

        for &block in self.changes.values() {
            if block == AIR {
                stats.air_overrides += 1;
            } else {
                stats.solid_overrides += 1;
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::STONE;

    fn test_chunk() -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(1, 0, 0), STONE);
        chunk.set(LocalPos::new(0, 1, 0), BlockId(100));
        chunk
    }

    #[test]
    fn test_delta_index_roundtrip() {
        let pos = LocalPos::new(5, 10, 15);
        let idx = DeltaIndex::from(pos);
        let back: LocalPos = idx.into();
        assert_eq!(pos, back);
    }

    #[test]
    fn test_delta_index_bounds() {
        let max_pos = LocalPos::new(15, 15, 15);
        let idx = DeltaIndex::from(max_pos);
        assert_eq!(idx.raw(), MAX_INDEX);
    }

    #[test]
    fn test_empty_delta() {
        let delta = ChunkDelta::new();
        assert!(delta.is_empty());
        assert_eq!(delta.len(), 0);
    }

    #[test]
    fn test_set_get() {
        let mut delta = ChunkDelta::new();
        let pos = LocalPos::new(5, 5, 5);

        assert!(delta.get(pos).is_none());

        delta.set(pos, STONE);
        assert_eq!(delta.get(pos), Some(STONE));
        assert!(delta.has_override(pos));
        assert_eq!(delta.len(), 1);
    }

    #[test]
    fn test_set_returns_previous() {
        let mut delta = ChunkDelta::new();
        let pos = LocalPos::new(0, 0, 0);

        let prev = delta.set(pos, STONE);
        assert!(prev.is_none());

        let prev = delta.set(pos, BlockId(50));
        assert_eq!(prev, Some(STONE));
    }

    #[test]
    fn test_remove() {
        let mut delta = ChunkDelta::new();
        let pos = LocalPos::new(3, 3, 3);

        delta.set(pos, STONE);
        assert!(delta.has_override(pos));

        let removed = delta.remove(pos);
        assert_eq!(removed, Some(STONE));
        assert!(!delta.has_override(pos));
        assert!(delta.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut delta = ChunkDelta::new();
        let removed = delta.remove(LocalPos::new(0, 0, 0));
        assert!(removed.is_none());
    }

    #[test]
    fn test_get_with_base() {
        let base = test_chunk();
        let mut delta = ChunkDelta::new();

        let pos_in_base = LocalPos::new(0, 0, 0);
        let pos_override = LocalPos::new(5, 5, 5);

        delta.set(pos_override, BlockId(200));

        assert_eq!(delta.get_with_base(pos_in_base, &base), STONE);
        assert_eq!(delta.get_with_base(pos_override, &base), BlockId(200));
        assert_eq!(delta.get_with_base(LocalPos::new(10, 10, 10), &base), AIR);
    }

    #[test]
    fn test_set_if_different() {
        let base = test_chunk();
        let mut delta = ChunkDelta::new();

        let pos = LocalPos::new(0, 0, 0);

        let modified = delta.set_if_different(pos, BlockId(50), &base);
        assert!(modified);
        assert_eq!(delta.get(pos), Some(BlockId(50)));

        let modified = delta.set_if_different(pos, STONE, &base);
        assert!(modified);
        assert!(!delta.has_override(pos));
    }

    #[test]
    fn test_set_if_different_no_change() {
        let base = Chunk::new();
        let mut delta = ChunkDelta::new();

        let modified = delta.set_if_different(LocalPos::new(0, 0, 0), AIR, &base);
        assert!(!modified);
        assert!(delta.is_empty());
    }

    #[test]
    fn test_materialize() {
        let base = test_chunk();
        let mut delta = ChunkDelta::new();

        delta.set(LocalPos::new(0, 0, 0), BlockId(999));
        delta.set(LocalPos::new(5, 5, 5), BlockId(888));

        let result = delta.materialize(&base);

        assert_eq!(result.get(LocalPos::new(0, 0, 0)), BlockId(999));
        assert_eq!(result.get(LocalPos::new(5, 5, 5)), BlockId(888));
        assert_eq!(result.get(LocalPos::new(1, 0, 0)), STONE);
        assert_eq!(result.get(LocalPos::new(0, 1, 0)), BlockId(100));
    }

    #[test]
    fn test_diff_identical() {
        let chunk = test_chunk();
        let delta = ChunkDelta::diff(&chunk, &chunk);
        assert!(delta.is_empty());
    }

    #[test]
    fn test_diff_single_change() {
        let base = Chunk::new();
        let mut target = Chunk::new();
        target.set(LocalPos::new(7, 7, 7), STONE);

        let delta = ChunkDelta::diff(&base, &target);

        assert_eq!(delta.len(), 1);
        assert_eq!(delta.get(LocalPos::new(7, 7, 7)), Some(STONE));
    }

    #[test]
    fn test_diff_roundtrip() {
        let base = test_chunk();
        let mut target = base.clone();
        target.set(LocalPos::new(0, 0, 0), BlockId(42));
        target.set(LocalPos::new(10, 10, 10), BlockId(43));
        target.set(LocalPos::new(1, 0, 0), AIR);

        let delta = ChunkDelta::diff(&base, &target);
        let reconstructed = delta.materialize(&base);

        for (pos, block) in target.iter() {
            assert_eq!(reconstructed.get(pos), block);
        }
    }

    #[test]
    fn test_merge() {
        let mut delta1 = ChunkDelta::new();
        delta1.set(LocalPos::new(0, 0, 0), STONE);
        delta1.set(LocalPos::new(1, 0, 0), BlockId(10));

        let mut delta2 = ChunkDelta::new();
        delta2.set(LocalPos::new(1, 0, 0), BlockId(20));
        delta2.set(LocalPos::new(2, 0, 0), BlockId(30));

        delta1.merge(&delta2);

        assert_eq!(delta1.len(), 3);
        assert_eq!(delta1.get(LocalPos::new(0, 0, 0)), Some(STONE));
        assert_eq!(delta1.get(LocalPos::new(1, 0, 0)), Some(BlockId(20)));
        assert_eq!(delta1.get(LocalPos::new(2, 0, 0)), Some(BlockId(30)));
    }

    #[test]
    fn test_merge_owned() {
        let mut delta1 = ChunkDelta::new();
        delta1.set(LocalPos::new(0, 0, 0), STONE);

        let mut delta2 = ChunkDelta::new();
        delta2.set(LocalPos::new(1, 0, 0), BlockId(50));

        delta1.merge_owned(delta2);

        assert_eq!(delta1.len(), 2);
    }

    #[test]
    fn test_retain() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(1, 0, 0), AIR);
        delta.set(LocalPos::new(2, 0, 0), BlockId(100));

        delta.retain(|_, block| block != AIR);

        assert_eq!(delta.len(), 2);
        assert!(!delta.has_override(LocalPos::new(1, 0, 0)));
    }

    #[test]
    fn test_compact() {
        let base = test_chunk();
        let mut delta = ChunkDelta::new();

        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(5, 5, 5), BlockId(50));

        delta.compact(&base);

        assert_eq!(delta.len(), 1);
        assert!(!delta.has_override(LocalPos::new(0, 0, 0)));
        assert!(delta.has_override(LocalPos::new(5, 5, 5)));
    }

    #[test]
    fn test_rebase() {
        let old_base = test_chunk();
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), BlockId(50));
        delta.set(LocalPos::new(5, 5, 5), BlockId(60));

        let target = delta.materialize(&old_base);

        let mut new_base = Chunk::new();
        new_base.set(LocalPos::new(0, 0, 0), BlockId(50));

        delta.rebase(&old_base, &new_base);

        let reconstructed = delta.materialize(&new_base);

        for (pos, block) in target.iter() {
            assert_eq!(reconstructed.get(pos), block);
        }
    }

    #[test]
    fn test_from_chunk_non_air() {
        let chunk = test_chunk();
        let delta = ChunkDelta::from_chunk_non_air(&chunk);

        assert_eq!(delta.len(), 3);
        assert_eq!(delta.get(LocalPos::new(0, 0, 0)), Some(STONE));
        assert_eq!(delta.get(LocalPos::new(1, 0, 0)), Some(STONE));
        assert_eq!(delta.get(LocalPos::new(0, 1, 0)), Some(BlockId(100)));
    }

    #[test]
    fn test_iter_deterministic() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(10, 0, 0), BlockId(10));
        delta.set(LocalPos::new(5, 0, 0), BlockId(5));
        delta.set(LocalPos::new(15, 0, 0), BlockId(15));

        let indices: Vec<_> = delta.iter().map(|(pos, _)| pos.x()).collect();
        assert_eq!(indices, vec![5, 10, 15]);
    }

    #[test]
    fn test_positions() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(1, 1, 1), STONE);

        let positions: Vec<_> = delta.positions().collect();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(1, 0, 0), STONE);

        delta.clear();

        assert!(delta.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), AIR);
        delta.set(LocalPos::new(1, 0, 0), STONE);
        delta.set(LocalPos::new(2, 0, 0), BlockId(100));

        let stats = delta.stats();
        assert_eq!(stats.override_count, 3);
        assert_eq!(stats.air_overrides, 1);
        assert_eq!(stats.solid_overrides, 2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(5, 5, 5), BlockId(42));

        let serialized = bincode::serialize(&delta).unwrap();
        let deserialized: ChunkDelta = bincode::deserialize(&serialized).unwrap();

        assert_eq!(delta, deserialized);
    }

    #[test]
    fn test_memory_estimate() {
        let delta = ChunkDelta::new();
        let empty_size = delta.memory_estimate();

        let mut delta = ChunkDelta::new();
        for i in 0..100 {
            delta.set(LocalPos::from_index(i), STONE);
        }
        let populated_size = delta.memory_estimate();

        assert!(populated_size > empty_size);
    }

    #[test]
    fn test_empty_base_materialize() {
        let base = Chunk::new();
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(7, 7, 7), STONE);

        let result = delta.materialize(&base);

        assert_eq!(result.non_air_count(), 1);
        assert_eq!(result.get(LocalPos::new(7, 7, 7)), STONE);
    }

    #[test]
    fn test_full_override() {
        let base = Chunk::filled(STONE);
        let mut delta = ChunkDelta::new();

        for x in 0..16u32 {
            delta.set(LocalPos::new(x, 0, 0), AIR);
        }

        let result = delta.materialize(&base);
        assert_eq!(result.get(LocalPos::new(0, 0, 0)), AIR);
        assert_eq!(result.get(LocalPos::new(0, 1, 0)), STONE);
    }
}

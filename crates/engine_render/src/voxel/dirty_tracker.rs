//! Chunk dirty flag tracking for mesh cache invalidation.
//!
//! Implements spec 3.3.3: chunk mesh caching with dirty flag.
//! When a block changes, the affected chunk is marked dirty
//! and its mesh will be regenerated on the next frame.

use engine_core::coords::ChunkPos;
use std::collections::HashSet;

/// Manages dirty flags for chunk meshes.
#[derive(Debug, Clone)]
pub struct ChunkDirtyTracker {
    /// Set of dirty chunk positions.
    dirty: HashSet<ChunkPos>,
    /// Whether tracking is enabled.
    enabled: bool,
}

impl Default for ChunkDirtyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkDirtyTracker {
    /// Create a new dirty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dirty: HashSet::new(),
            enabled: true,
        }
    }

    /// Enable or disable dirty tracking.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if tracking is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Mark a chunk as dirty (needs mesh regeneration).
    pub fn mark_dirty(&mut self, chunk_pos: ChunkPos) {
        if self.enabled {
            self.dirty.insert(chunk_pos);
        }
    }

    /// Mark a chunk as clean (mesh is up to date).
    pub fn mark_clean(&mut self, chunk_pos: &ChunkPos) {
        self.dirty.remove(chunk_pos);
    }

    /// Check if a chunk is dirty.
    #[must_use]
    pub fn is_dirty(&self, chunk_pos: &ChunkPos) -> bool {
        self.dirty.contains(chunk_pos)
    }

    /// Get all dirty chunk positions.
    #[must_use]
    pub fn dirty_chunks(&self) -> &HashSet<ChunkPos> {
        &self.dirty
    }

    /// Drain all dirty chunk positions.
    pub fn drain_dirty(&mut self) -> HashSet<ChunkPos> {
        std::mem::take(&mut self.dirty)
    }

    /// Number of dirty chunks.
    #[must_use]
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }

    /// Check if any chunks are dirty.
    #[must_use]
    pub fn has_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Clear all dirty flags.
    pub fn clear(&mut self) {
        self.dirty.clear();
    }

    /// Mark chunks near a block position as dirty.
    ///
    /// When a block changes, the chunk it's in and possibly
    /// neighboring chunks need mesh updates (if the block
    /// is on a chunk boundary).
    pub fn mark_block_changed(&mut self, block_pos: &engine_core::coords::WorldPos) {
        let chunk_pos = block_pos.to_chunk_pos();
        self.mark_dirty(chunk_pos);

        // Check if block is on a chunk boundary - if so, mark neighbor
        let local = block_pos.to_local_pos();
        if local.0.x == 0 {
            self.mark_dirty(ChunkPos(glam::IVec3::new(chunk_pos.0.x - 1, chunk_pos.0.y, chunk_pos.0.z)));
        }
        if local.0.x == 15 {
            self.mark_dirty(ChunkPos(glam::IVec3::new(chunk_pos.0.x + 1, chunk_pos.0.y, chunk_pos.0.z)));
        }
        if local.0.y == 0 {
            self.mark_dirty(ChunkPos(glam::IVec3::new(chunk_pos.0.x, chunk_pos.0.y - 1, chunk_pos.0.z)));
        }
        if local.0.y == 15 {
            self.mark_dirty(ChunkPos(glam::IVec3::new(chunk_pos.0.x, chunk_pos.0.y + 1, chunk_pos.0.z)));
        }
        if local.0.z == 0 {
            self.mark_dirty(ChunkPos(glam::IVec3::new(chunk_pos.0.x, chunk_pos.0.y, chunk_pos.0.z - 1)));
        }
        if local.0.z == 15 {
            self.mark_dirty(ChunkPos(glam::IVec3::new(chunk_pos.0.x, chunk_pos.0.y, chunk_pos.0.z + 1)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    fn chunk(x: i32, y: i32, z: i32) -> ChunkPos {
        ChunkPos(IVec3::new(x, y, z))
    }

    #[test]
    fn test_new_tracker() {
        let tracker = ChunkDirtyTracker::new();
        assert!(!tracker.has_dirty());
        assert!(tracker.is_enabled());
    }

    #[test]
    fn test_mark_dirty() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.mark_dirty(chunk(0, 0, 0));
        assert!(tracker.is_dirty(&chunk(0, 0, 0)));
        assert!(!tracker.is_dirty(&chunk(1, 0, 0)));
    }

    #[test]
    fn test_mark_clean() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.mark_dirty(chunk(0, 0, 0));
        tracker.mark_clean(&chunk(0, 0, 0));
        assert!(!tracker.is_dirty(&chunk(0, 0, 0)));
    }

    #[test]
    fn test_dirty_count() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.mark_dirty(chunk(0, 0, 0));
        tracker.mark_dirty(chunk(1, 0, 0));
        assert_eq!(tracker.dirty_count(), 2);
    }

    #[test]
    fn test_drain_dirty() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.mark_dirty(chunk(0, 0, 0));
        let drained = tracker.drain_dirty();
        assert_eq!(drained.len(), 1);
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn test_clear() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.mark_dirty(chunk(0, 0, 0));
        tracker.mark_dirty(chunk(1, 0, 0));
        tracker.clear();
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn test_disabled_ignores_marks() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.set_enabled(false);
        tracker.mark_dirty(chunk(0, 0, 0));
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn test_mark_block_changed() {
        let mut tracker = ChunkDirtyTracker::new();
        // Block at chunk center - only marks one chunk
        let block_pos = engine_core::coords::WorldPos::new(8, 8, 8);
        tracker.mark_block_changed(&block_pos);
        assert!(tracker.is_dirty(&chunk(0, 0, 0)));
        assert_eq!(tracker.dirty_count(), 1);
    }

    #[test]
    fn test_mark_block_on_boundary() {
        let mut tracker = ChunkDirtyTracker::new();
        // Block at x=0 (chunk boundary) - marks two chunks
        let block_pos = engine_core::coords::WorldPos::new(16, 8, 8);
        tracker.mark_block_changed(&block_pos);
        assert!(tracker.is_dirty(&chunk(1, 0, 0)));
        assert!(tracker.is_dirty(&chunk(0, 0, 0)));
    }

    #[test]
    fn test_mark_block_corner() {
        let mut tracker = ChunkDirtyTracker::new();
        // Block at corner (0,0,0 local) - marks up to 4 chunks
        let block_pos = engine_core::coords::WorldPos::new(0, 0, 0);
        tracker.mark_block_changed(&block_pos);
        assert!(tracker.dirty_count() >= 1);
    }

    #[test]
    fn test_duplicate_marks_count_once() {
        let mut tracker = ChunkDirtyTracker::new();
        tracker.mark_dirty(chunk(0, 0, 0));
        tracker.mark_dirty(chunk(0, 0, 0));
        assert_eq!(tracker.dirty_count(), 1);
    }
}

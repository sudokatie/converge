//! Neighbor dependency tracking for chunk meshing.

use engine_core::coords::ChunkPos;
use glam::IVec3;
use std::collections::{HashMap, HashSet};

/// The 6 cardinal neighbor offsets.
pub const NEIGHBOR_OFFSETS: [IVec3; 6] = [
    IVec3::new(-1, 0, 0),
    IVec3::new(1, 0, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, 0, -1),
    IVec3::new(0, 0, 1),
];

/// Tracks which chunks are waiting for neighbors before meshing.
#[derive(Debug, Default)]
pub struct NeighborTracker {
    /// Chunks waiting for neighbors: maps chunk -> set of missing neighbor positions.
    waiting: HashMap<ChunkPos, HashSet<ChunkPos>>,
    /// Reverse lookup: which chunks are waiting for a given chunk.
    dependents: HashMap<ChunkPos, HashSet<ChunkPos>>,
}

impl NeighborTracker {
    /// Create a new neighbor tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            waiting: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Get the 6 neighbor positions for a chunk.
    #[must_use]
    pub fn neighbors(pos: ChunkPos) -> [ChunkPos; 6] {
        NEIGHBOR_OFFSETS.map(|offset| ChunkPos(pos.0 + offset))
    }

    /// Register a chunk as waiting for its neighbors.
    /// Returns true if all neighbors are already ready (empty wait set).
    pub fn start_waiting(&mut self, pos: ChunkPos, ready_chunks: &HashSet<ChunkPos>) -> bool {
        let neighbors = Self::neighbors(pos);
        let missing: HashSet<ChunkPos> = neighbors
            .into_iter()
            .filter(|n| !ready_chunks.contains(n))
            .collect();

        if missing.is_empty() {
            return true;
        }

        // Register reverse dependencies
        for neighbor in &missing {
            self.dependents
                .entry(*neighbor)
                .or_default()
                .insert(pos);
        }

        self.waiting.insert(pos, missing);
        false
    }

    /// Notify that a chunk has become ready.
    /// Returns list of chunks that now have all neighbors ready.
    pub fn chunk_ready(&mut self, pos: ChunkPos) -> Vec<ChunkPos> {
        let mut newly_ready = Vec::new();

        // Get all chunks waiting for this one
        if let Some(dependents) = self.dependents.remove(&pos) {
            for dependent in dependents {
                if let Some(waiting_for) = self.waiting.get_mut(&dependent) {
                    waiting_for.remove(&pos);
                    if waiting_for.is_empty() {
                        self.waiting.remove(&dependent);
                        newly_ready.push(dependent);
                    }
                }
            }
        }

        newly_ready
    }

    /// Remove a chunk from tracking (when unloaded).
    pub fn remove(&mut self, pos: ChunkPos) {
        // Remove from waiting
        if let Some(missing) = self.waiting.remove(&pos) {
            for neighbor in missing {
                if let Some(deps) = self.dependents.get_mut(&neighbor) {
                    deps.remove(&pos);
                    if deps.is_empty() {
                        self.dependents.remove(&neighbor);
                    }
                }
            }
        }

        // Remove from dependents (as a neighbor)
        self.dependents.remove(&pos);
    }

    /// Check if a chunk is waiting for neighbors.
    #[must_use]
    pub fn is_waiting(&self, pos: &ChunkPos) -> bool {
        self.waiting.contains_key(pos)
    }

    /// Number of chunks waiting for neighbors.
    #[must_use]
    pub fn waiting_count(&self) -> usize {
        self.waiting.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_neighbors_ready_returns_true() {
        let mut tracker = NeighborTracker::new();
        let center = ChunkPos(IVec3::ZERO);

        // Mark all neighbors as ready
        let ready: HashSet<ChunkPos> = NeighborTracker::neighbors(center)
            .into_iter()
            .collect();

        assert!(tracker.start_waiting(center, &ready));
        assert!(!tracker.is_waiting(&center));
    }

    #[test]
    fn missing_neighbors_blocks() {
        let mut tracker = NeighborTracker::new();
        let center = ChunkPos(IVec3::ZERO);
        let ready = HashSet::new();

        assert!(!tracker.start_waiting(center, &ready));
        assert!(tracker.is_waiting(&center));
    }

    #[test]
    fn chunk_ready_unblocks_dependents() {
        let mut tracker = NeighborTracker::new();
        let center = ChunkPos(IVec3::ZERO);
        let neighbors = NeighborTracker::neighbors(center);

        // Mark all but one neighbor as ready
        let ready: HashSet<ChunkPos> = neighbors[1..].iter().copied().collect();
        assert!(!tracker.start_waiting(center, &ready));

        // Now mark the last neighbor as ready
        let newly_ready = tracker.chunk_ready(neighbors[0]);
        assert_eq!(newly_ready.len(), 1);
        assert_eq!(newly_ready[0], center);
        assert!(!tracker.is_waiting(&center));
    }
}

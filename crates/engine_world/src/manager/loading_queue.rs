//! Priority queue for chunk loading with spiral pattern.

use engine_core::coords::ChunkPos;
use glam::IVec3;
use std::collections::BinaryHeap;

/// Entry in the loading queue with priority based on distance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadEntry {
    pub pos: ChunkPos,
    /// Negative distance squared (so closer chunks have higher priority).
    priority: i32,
}

impl Ord for LoadEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority (less negative) comes first
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for LoadEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority queue for loading chunks in spiral order from player position.
#[derive(Debug, Default)]
pub struct LoadingQueue {
    queue: BinaryHeap<LoadEntry>,
}

impl LoadingQueue {
    /// Create a new empty loading queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }

    /// Clear the queue and populate with chunks in spiral order around center.
    pub fn rebuild(&mut self, center: ChunkPos, view_distance: i32) {
        self.queue.clear();

        let center_pos = center.0;

        // Generate spiral pattern
        for y in -view_distance..=view_distance {
            for x in -view_distance..=view_distance {
                for z in -view_distance..=view_distance {
                    let offset = IVec3::new(x, y, z);
                    let chunk_pos = ChunkPos(center_pos + offset);

                    // Calculate distance squared (negative for min-heap behavior)
                    let dist_sq = x * x + y * y + z * z;

                    // Skip if outside spherical view distance
                    if dist_sq > view_distance * view_distance {
                        continue;
                    }

                    self.queue.push(LoadEntry {
                        pos: chunk_pos,
                        priority: -dist_sq,
                    });
                }
            }
        }
    }

    /// Pop the next chunk to load (closest to center).
    pub fn pop(&mut self) -> Option<ChunkPos> {
        self.queue.pop().map(|e| e.pos)
    }

    /// Check if the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Number of chunks remaining to load.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spiral_order_closest_first() {
        let mut queue = LoadingQueue::new();
        queue.rebuild(ChunkPos(IVec3::ZERO), 2);

        // First chunk should be at origin (distance 0)
        let first = queue.pop().unwrap();
        assert_eq!(first.0, IVec3::ZERO);

        // Next chunks should be adjacent (distance 1)
        let mut dist_one_count = 0;
        for _ in 0..6 {
            if let Some(pos) = queue.pop() {
                let dist_sq = pos.0.x * pos.0.x + pos.0.y * pos.0.y + pos.0.z * pos.0.z;
                if dist_sq == 1 {
                    dist_one_count += 1;
                }
            }
        }
        assert_eq!(dist_one_count, 6);
    }

    #[test]
    fn respects_view_distance() {
        let mut queue = LoadingQueue::new();
        queue.rebuild(ChunkPos(IVec3::ZERO), 3);

        // Check all entries are within spherical distance
        while let Some(pos) = queue.pop() {
            let dist_sq = pos.0.x * pos.0.x + pos.0.y * pos.0.y + pos.0.z * pos.0.z;
            assert!(dist_sq <= 9, "chunk {:?} outside view distance", pos);
        }
    }
}

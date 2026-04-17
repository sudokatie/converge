//! Light invalidation and recalculation on block changes.
//!
//! Implements spec 3.4.3: recalculate light on block changes.
//! When a block is placed or removed, nearby light values are
//! invalidated and recalculated via BFS.

use engine_core::coords::WorldPos;
use std::collections::VecDeque;

/// Radius of light invalidation around a block change.
pub const LIGHT_INVALIDATION_RADIUS: i32 = 16;

/// A pending light update.
#[derive(Debug, Clone, Copy)]
pub struct LightUpdate {
    /// Position that needs light recalculation.
    pub position: WorldPos,
    /// Whether this was caused by a block removal (increases light).
    pub was_removal: bool,
}

/// Manages light invalidation and update scheduling.
#[derive(Debug, Clone)]
pub struct LightUpdateManager {
    /// Pending light updates.
    pending: VecDeque<LightUpdate>,
    /// Whether light updates are enabled.
    enabled: bool,
}

impl Default for LightUpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LightUpdateManager {
    /// Create a new light update manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            enabled: true,
        }
    }

    /// Enable or disable light updates.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if light updates are enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Called when a block is changed at a position.
    ///
    /// Schedules light recalculation for nearby blocks.
    pub fn on_block_changed(&mut self, position: WorldPos, was_removal: bool) {
        if !self.enabled {
            return;
        }

        self.pending.push_back(LightUpdate {
            position,
            was_removal,
        });
    }

    /// Called when a block is placed.
    pub fn on_block_placed(&mut self, position: WorldPos) {
        self.on_block_changed(position, false);
    }

    /// Called when a block is removed.
    pub fn on_block_removed(&mut self, position: WorldPos) {
        self.on_block_changed(position, true);
    }

    /// Get the next pending light update.
    pub fn next_update(&mut self) -> Option<LightUpdate> {
        self.pending.pop_front()
    }

    /// Get all pending updates, clearing the queue.
    pub fn drain_updates(&mut self) -> Vec<LightUpdate> {
        self.pending.drain(..).collect()
    }

    /// Number of pending light updates.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if there are pending updates.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Clear all pending updates.
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Process a batch of light updates (max count).
    ///
    /// Returns the number of updates processed.
    pub fn process_batch<F>(&mut self, max_count: usize, mut processor: F) -> usize
    where
        F: FnMut(LightUpdate),
    {
        let count = self.pending_count().min(max_count);
        for _ in 0..count {
            if let Some(update) = self.next_update() {
                processor(update);
            }
        }
        count
    }
}

/// Determines which positions need light recalculation after a block change.
///
/// Returns positions in a cube around the change point, within
/// the invalidation radius.
#[must_use]
pub fn affected_positions(center: WorldPos, radius: i32) -> Vec<WorldPos> {
    let mut positions = Vec::new();

    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                let pos = WorldPos::new(center.x() + dx, center.y() + dy, center.z() + dz);
                positions.push(pos);
            }
        }
    }

    positions
}

/// Determines if a position is within the invalidation radius of a change.
#[must_use]
pub fn is_affected(change_pos: WorldPos, check_pos: WorldPos, radius: i32) -> bool {
    let dx = (check_pos.x() - change_pos.x()).abs();
    let dy = (check_pos.y() - change_pos.y()).abs();
    let dz = (check_pos.z() - change_pos.z()).abs();

    dx <= radius && dy <= radius && dz <= radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let mgr = LightUpdateManager::new();
        assert!(mgr.is_enabled());
        assert!(!mgr.has_pending());
    }

    #[test]
    fn test_on_block_placed() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_placed(WorldPos::new(5, 64, 10));
        assert!(mgr.has_pending());
        assert_eq!(mgr.pending_count(), 1);
    }

    #[test]
    fn test_on_block_removed() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_removed(WorldPos::new(5, 64, 10));
        let update = mgr.next_update().unwrap();
        assert!(update.was_removal);
    }

    #[test]
    fn test_placed_not_removal() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_placed(WorldPos::new(5, 64, 10));
        let update = mgr.next_update().unwrap();
        assert!(!update.was_removal);
    }

    #[test]
    fn test_disabled_ignores_changes() {
        let mut mgr = LightUpdateManager::new();
        mgr.set_enabled(false);
        mgr.on_block_placed(WorldPos::new(5, 64, 10));
        assert!(!mgr.has_pending());
    }

    #[test]
    fn test_drain_updates() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_placed(WorldPos::new(1, 0, 0));
        mgr.on_block_placed(WorldPos::new(2, 0, 0));

        let updates = mgr.drain_updates();
        assert_eq!(updates.len(), 2);
        assert!(!mgr.has_pending());
    }

    #[test]
    fn test_next_update_fifo() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_placed(WorldPos::new(1, 0, 0));
        mgr.on_block_placed(WorldPos::new(2, 0, 0));

        let first = mgr.next_update().unwrap();
        assert_eq!(first.position, WorldPos::new(1, 0, 0));
    }

    #[test]
    fn test_clear() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_placed(WorldPos::new(1, 0, 0));
        mgr.clear();
        assert!(!mgr.has_pending());
    }

    #[test]
    fn test_affected_positions_count() {
        let positions = affected_positions(WorldPos::new(0, 0, 0), 1);
        assert_eq!(positions.len(), 27); // 3x3x3 cube
    }

    #[test]
    fn test_affected_positions_radius_zero() {
        let positions = affected_positions(WorldPos::new(0, 0, 0), 0);
        assert_eq!(positions.len(), 1);
    }

    #[test]
    fn test_is_affected() {
        let change = WorldPos::new(0, 0, 0);
        assert!(is_affected(change, WorldPos::new(5, 0, 0), 5));
        assert!(!is_affected(change, WorldPos::new(6, 0, 0), 5));
    }

    #[test]
    fn test_process_batch() {
        let mut mgr = LightUpdateManager::new();
        mgr.on_block_placed(WorldPos::new(1, 0, 0));
        mgr.on_block_placed(WorldPos::new(2, 0, 0));
        mgr.on_block_placed(WorldPos::new(3, 0, 0));

        let mut processed = 0;
        let count = mgr.process_batch(2, |_| processed += 1);
        assert_eq!(count, 2);
        assert_eq!(mgr.pending_count(), 1);
    }
}

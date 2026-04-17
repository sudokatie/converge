//! Player spawn point management.
//!
//! Tracks spawn points per player and provides the world spawn
//! for initial login and respawn on death.

use engine_core::coords::WorldPos;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A spawn point with optional metadata.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpawnPoint {
    /// World position of the spawn.
    pub position: WorldPos,
    /// Whether this is a personal spawn (bed) vs world spawn.
    pub personal: bool,
}

impl SpawnPoint {
    /// Create a world spawn point.
    #[must_use]
    pub fn world_spawn(position: WorldPos) -> Self {
        Self {
            position,
            personal: false,
        }
    }

    /// Create a personal spawn point (e.g., from a bed).
    #[must_use]
    pub fn personal(position: WorldPos) -> Self {
        Self {
            position,
            personal: true,
        }
    }
}

/// Manages spawn points for all players.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnManager {
    /// The default world spawn point.
    world_spawn: SpawnPoint,
    /// Per-player spawn points (player ID -> spawn).
    personal_spawns: HashMap<u64, SpawnPoint>,
}

impl SpawnManager {
    /// Create a new spawn manager with a default world spawn.
    #[must_use]
    pub fn new(world_spawn: WorldPos) -> Self {
        Self {
            world_spawn: SpawnPoint::world_spawn(world_spawn),
            personal_spawns: HashMap::new(),
        }
    }

    /// Get the spawn point for a player.
    ///
    /// Returns the player's personal spawn if set, otherwise the world spawn.
    #[must_use]
    pub fn get_spawn(&self, player_id: u64) -> SpawnPoint {
        self.personal_spawns
            .get(&player_id)
            .copied()
            .unwrap_or(self.world_spawn)
    }

    /// Get the world spawn point.
    #[must_use]
    pub fn world_spawn(&self) -> SpawnPoint {
        self.world_spawn
    }

    /// Set the world spawn point.
    pub fn set_world_spawn(&mut self, position: WorldPos) {
        self.world_spawn = SpawnPoint::world_spawn(position);
    }

    /// Set a personal spawn point for a player.
    pub fn set_player_spawn(&mut self, player_id: u64, position: WorldPos) {
        self.personal_spawns
            .insert(player_id, SpawnPoint::personal(position));
    }

    /// Clear a player's personal spawn point.
    ///
    /// Returns true if a spawn was removed.
    pub fn clear_player_spawn(&mut self, player_id: u64) -> bool {
        self.personal_spawns.remove(&player_id).is_some()
    }

    /// Find a safe spawn position near the requested point.
    ///
    /// Searches downward for a solid surface, then checks that
    /// the two blocks above are clear (player needs 2 blocks of space).
    #[must_use]
    pub fn find_safe_spawn<F>(&self, start: WorldPos, is_solid: F) -> WorldPos
    where
        F: Fn(WorldPos) -> bool,
    {
        // Search down from start position for a solid surface
        let mut y = start.y();
        while y > -64 {
            let below = WorldPos::new(start.x(), y - 1, start.z());
            let feet = WorldPos::new(start.x(), y, start.z());
            let head = WorldPos::new(start.x(), y + 1, start.z());

            if is_solid(below) && !is_solid(feet) && !is_solid(head) {
                return feet;
            }
            y -= 1;
        }

        // Fallback: return start position if no safe spot found
        start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_world_spawn() {
        let mgr = SpawnManager::new(WorldPos::new(0, 64, 0));
        let spawn = mgr.world_spawn();
        assert_eq!(spawn.position, WorldPos::new(0, 64, 0));
        assert!(!spawn.personal);
    }

    #[test]
    fn test_player_spawn_overrides_world() {
        let mut mgr = SpawnManager::new(WorldPos::new(0, 64, 0));
        mgr.set_player_spawn(1, WorldPos::new(100, 70, 200));

        let spawn = mgr.get_spawn(1);
        assert_eq!(spawn.position, WorldPos::new(100, 70, 200));
        assert!(spawn.personal);
    }

    #[test]
    fn test_world_spawn_as_fallback() {
        let mgr = SpawnManager::new(WorldPos::new(0, 64, 0));
        let spawn = mgr.get_spawn(999); // Unknown player
        assert_eq!(spawn.position, WorldPos::new(0, 64, 0));
        assert!(!spawn.personal);
    }

    #[test]
    fn test_clear_player_spawn() {
        let mut mgr = SpawnManager::new(WorldPos::new(0, 64, 0));
        mgr.set_player_spawn(1, WorldPos::new(50, 70, 50));

        assert!(mgr.clear_player_spawn(1));
        assert_eq!(mgr.get_spawn(1).position, WorldPos::new(0, 64, 0));
    }

    #[test]
    fn test_clear_nonexistent_spawn() {
        let mut mgr = SpawnManager::new(WorldPos::new(0, 64, 0));
        assert!(!mgr.clear_player_spawn(999));
    }

    #[test]
    fn test_set_world_spawn() {
        let mut mgr = SpawnManager::new(WorldPos::new(0, 64, 0));
        mgr.set_world_spawn(WorldPos::new(10, 80, 10));

        assert_eq!(mgr.world_spawn().position, WorldPos::new(10, 80, 10));
    }

    #[test]
    fn test_find_safe_spawn() {
        let mgr = SpawnManager::new(WorldPos::new(0, 64, 0));

        // Solid ground at y=60, air above
        let is_solid = |pos: WorldPos| pos.y() == 60;

        let safe = mgr.find_safe_spawn(WorldPos::new(0, 70, 0), is_solid);
        assert_eq!(safe, WorldPos::new(0, 61, 0), "Should find feet position above solid ground");
    }

    #[test]
    fn test_find_safe_spawn_no_ground() {
        let mgr = SpawnManager::new(WorldPos::new(0, 64, 0));

        // No solid blocks anywhere
        let is_solid = |_pos: WorldPos| false;

        let safe = mgr.find_safe_spawn(WorldPos::new(0, 70, 0), is_solid);
        assert_eq!(safe, WorldPos::new(0, 70, 0), "Should fallback to start position");
    }
}

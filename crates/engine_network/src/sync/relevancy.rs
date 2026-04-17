//! Entity relevancy system for network optimization.
//!
//! Implements spec 7.3.1: distant entities get position-only updates,
//! nearby entities get full state updates.

use glam::Vec3;

/// Full update distance threshold (blocks).
pub const FULL_UPDATE_DISTANCE: f32 = 32.0;

/// Position-only update distance threshold (blocks).
pub const POSITION_ONLY_DISTANCE: f32 = 64.0;

/// Maximum relevance distance (beyond this, entity is irrelevant).
pub const MAX_RELEVANCE_DISTANCE: f32 = 128.0;

/// Entity update detail level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateLevel {
    /// Full component state: position, rotation, velocity, health, etc.
    Full,
    /// Position and rotation only.
    PositionOnly,
    /// Entity is too far, no updates needed.
    Irrelevant,
}

impl UpdateLevel {
    /// Determine update level from distance.
    #[must_use]
    pub fn from_distance(distance: f32) -> Self {
        if distance <= FULL_UPDATE_DISTANCE {
            UpdateLevel::Full
        } else if distance <= POSITION_ONLY_DISTANCE {
            UpdateLevel::PositionOnly
        } else if distance <= MAX_RELEVANCE_DISTANCE {
            UpdateLevel::PositionOnly // Still relevant but minimal
        } else {
            UpdateLevel::Irrelevant
        }
    }

    /// Check if this level includes position data.
    #[must_use]
    pub fn includes_position(&self) -> bool {
        matches!(self, UpdateLevel::Full | UpdateLevel::PositionOnly)
    }

    /// Check if this level includes full state.
    #[must_use]
    pub fn includes_full_state(&self) -> bool {
        matches!(self, UpdateLevel::Full)
    }
}

/// Relevancy check result for an entity.
#[derive(Debug, Clone)]
pub struct RelevancyResult {
    /// Entity ID.
    pub entity_id: u64,
    /// Current update level.
    pub level: UpdateLevel,
    /// Distance from the viewer.
    pub distance: f32,
    /// Whether the update level changed since last check.
    pub changed: bool,
}

/// Manages entity relevancy for a single client.
#[derive(Debug, Clone)]
pub struct EntityRelevancyManager {
    /// Viewer position.
    viewer_pos: Vec3,
    /// Tracked entity states (entity_id -> last known UpdateLevel).
    entity_levels: std::collections::HashMap<u64, UpdateLevel>,
    /// Whether relevancy is enabled.
    enabled: bool,
}

impl Default for EntityRelevancyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityRelevancyManager {
    /// Create a new relevancy manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            viewer_pos: Vec3::ZERO,
            entity_levels: std::collections::HashMap::new(),
            enabled: true,
        }
    }

    /// Set the viewer position.
    pub fn set_viewer_position(&mut self, pos: Vec3) {
        self.viewer_pos = pos;
    }

    /// Enable or disable relevancy.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if an entity should receive updates.
    #[must_use]
    pub fn should_update(&self, entity_id: u64, entity_pos: Vec3) -> bool {
        if !self.enabled {
            return true;
        }
        let distance = entity_pos.distance(self.viewer_pos);
        distance <= MAX_RELEVANCE_DISTANCE
    }

    /// Get the update level for an entity.
    #[must_use]
    pub fn update_level(&self, entity_pos: Vec3) -> UpdateLevel {
        if !self.enabled {
            return UpdateLevel::Full;
        }
        let distance = entity_pos.distance(self.viewer_pos);
        UpdateLevel::from_distance(distance)
    }

    /// Check all entities and return those whose relevancy changed.
    pub fn update_relevancy(
        &mut self,
        entities: &[(u64, Vec3)],
    ) -> Vec<RelevancyResult> {
        if !self.enabled {
            return Vec::new();
        }

        let mut results = Vec::new();

        for (id, pos) in entities {
            let distance = pos.distance(self.viewer_pos);
            let new_level = UpdateLevel::from_distance(distance);
            let old_level = self.entity_levels.get(id).copied();

            let changed = old_level.map_or(true, |old| old != new_level);

            self.entity_levels.insert(*id, new_level);

            if changed {
                results.push(RelevancyResult {
                    entity_id: *id,
                    level: new_level,
                    distance,
                    changed,
                });
            }
        }

        results
    }

    /// Get the current update level for a tracked entity.
    #[must_use]
    pub fn get_level(&self, entity_id: u64) -> Option<UpdateLevel> {
        self.entity_levels.get(&entity_id).copied()
    }

    /// Remove an entity from tracking.
    pub fn remove(&mut self, entity_id: u64) {
        self.entity_levels.remove(&entity_id);
    }

    /// Clear all tracked entities.
    pub fn clear(&mut self) {
        self.entity_levels.clear();
    }

    /// Count entities at each update level.
    #[must_use]
    pub fn count_by_level(&self) -> (usize, usize, usize) {
        let mut full = 0;
        let mut position_only = 0;
        let mut irrelevant = 0;

        for level in self.entity_levels.values() {
            match level {
                UpdateLevel::Full => full += 1,
                UpdateLevel::PositionOnly => position_only += 1,
                UpdateLevel::Irrelevant => irrelevant += 1,
            }
        }

        (full, position_only, irrelevant)
    }

    /// Total tracked entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entity_levels.len()
    }

    /// Check if no entities tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entity_levels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_level_from_distance() {
        assert_eq!(UpdateLevel::from_distance(10.0), UpdateLevel::Full);
        assert_eq!(UpdateLevel::from_distance(32.0), UpdateLevel::Full);
        assert_eq!(UpdateLevel::from_distance(50.0), UpdateLevel::PositionOnly);
        assert_eq!(UpdateLevel::from_distance(64.0), UpdateLevel::PositionOnly);
        assert_eq!(UpdateLevel::from_distance(100.0), UpdateLevel::PositionOnly);
        assert_eq!(UpdateLevel::from_distance(200.0), UpdateLevel::Irrelevant);
    }

    #[test]
    fn test_update_level_includes() {
        assert!(UpdateLevel::Full.includes_position());
        assert!(UpdateLevel::Full.includes_full_state());
        assert!(UpdateLevel::PositionOnly.includes_position());
        assert!(!UpdateLevel::PositionOnly.includes_full_state());
        assert!(!UpdateLevel::Irrelevant.includes_position());
    }

    #[test]
    fn test_should_update() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_viewer_position(Vec3::ZERO);

        assert!(mgr.should_update(1, Vec3::new(50.0, 0.0, 0.0)));
        assert!(!mgr.should_update(1, Vec3::new(200.0, 0.0, 0.0)));
    }

    #[test]
    fn test_should_update_disabled() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_enabled(false);
        assert!(mgr.should_update(1, Vec3::new(200.0, 0.0, 0.0)));
    }

    #[test]
    fn test_update_level_method() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_viewer_position(Vec3::ZERO);

        assert_eq!(mgr.update_level(Vec3::new(10.0, 0.0, 0.0)), UpdateLevel::Full);
        assert_eq!(mgr.update_level(Vec3::new(50.0, 0.0, 0.0)), UpdateLevel::PositionOnly);
    }

    #[test]
    fn test_relevancy_changes_detected() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_viewer_position(Vec3::ZERO);

        let entities = vec![(1u64, Vec3::new(50.0, 0.0, 0.0))];
        let results = mgr.update_relevancy(&entities);

        assert_eq!(results.len(), 1);
        assert!(results[0].changed);
    }

    #[test]
    fn test_no_change_on_same_level() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_viewer_position(Vec3::ZERO);

        let entities = vec![(1u64, Vec3::new(50.0, 0.0, 0.0))];
        mgr.update_relevancy(&entities);

        // Same position, same level
        let results = mgr.update_relevancy(&entities);
        assert!(results.is_empty());
    }

    #[test]
    fn test_change_on_movement() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_viewer_position(Vec3::ZERO);

        mgr.update_relevancy(&[(1u64, Vec3::new(10.0, 0.0, 0.0))]);

        // Move entity from Full to PositionOnly range
        let results = mgr.update_relevancy(&[(1u64, Vec3::new(50.0, 0.0, 0.0))]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, UpdateLevel::PositionOnly);
    }

    #[test]
    fn test_count_by_level() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.set_viewer_position(Vec3::ZERO);

        mgr.update_relevancy(&[
            (1u64, Vec3::new(10.0, 0.0, 0.0)),   // Full
            (2u64, Vec3::new(50.0, 0.0, 0.0)),   // PositionOnly
            (3u64, Vec3::new(200.0, 0.0, 0.0)),  // Irrelevant
        ]);

        let (full, pos, irrelevant) = mgr.count_by_level();
        assert_eq!(full, 1);
        assert_eq!(pos, 1);
        assert_eq!(irrelevant, 1);
    }

    #[test]
    fn test_remove_entity() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.entity_levels.insert(42, UpdateLevel::Full);
        mgr.remove(42);
        assert!(mgr.is_empty());
    }

    #[test]
    fn test_get_level() {
        let mut mgr = EntityRelevancyManager::new();
        mgr.entity_levels.insert(1, UpdateLevel::Full);
        assert_eq!(mgr.get_level(1), Some(UpdateLevel::Full));
        assert_eq!(mgr.get_level(999), None);
    }
}

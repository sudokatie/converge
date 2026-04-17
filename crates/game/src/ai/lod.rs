//! AI level-of-detail system.
//!
//! Implements spec 8.4: full AI within 32 blocks, simplified AI
//! at 32-64 blocks, hibernation beyond 64 blocks.

use glam::Vec3;
use hecs::World;

/// Full AI distance threshold (blocks).
pub const FULL_AI_DISTANCE: f32 = 32.0;

/// Simplified AI distance threshold (blocks).
pub const SIMPLIFIED_AI_DISTANCE: f32 = 64.0;

/// AI detail level for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiLodLevel {
    /// Full behavior tree, pathfinding, and animations.
    Full,
    /// Reduced pathfinding, basic movement only.
    Simplified,
    /// No updates, entity is frozen.
    Hibernated,
}

impl AiLodLevel {
    /// Determine LOD level based on distance from player.
    #[must_use]
    pub fn from_distance(distance: f32) -> Self {
        if distance <= FULL_AI_DISTANCE {
            AiLodLevel::Full
        } else if distance <= SIMPLIFIED_AI_DISTANCE {
            AiLodLevel::Simplified
        } else {
            AiLodLevel::Hibernated
        }
    }

    /// Whether this LOD level allows pathfinding.
    #[must_use]
    pub fn allows_pathfinding(&self) -> bool {
        matches!(self, AiLodLevel::Full)
    }

    /// Whether this LOD level allows any AI updates.
    #[must_use]
    pub fn allows_updates(&self) -> bool {
        matches!(self, AiLodLevel::Full | AiLodLevel::Simplified)
    }

    /// Maximum update frequency for this LOD level.
    #[must_use]
    pub fn update_interval_secs(&self) -> f32 {
        match self {
            AiLodLevel::Full => 0.0, // Every frame
            AiLodLevel::Simplified => 1.0, // Once per second
            AiLodLevel::Hibernated => f32::INFINITY, // Never
        }
    }
}

/// AI LOD state for a single entity.
#[derive(Debug, Clone)]
pub struct AiLodState {
    /// Current LOD level.
    pub level: AiLodLevel,
    /// Distance from player.
    pub distance: f32,
    /// Time since last AI update.
    pub time_since_update: f32,
}

impl Default for AiLodState {
    fn default() -> Self {
        Self {
            level: AiLodLevel::Hibernated,
            distance: f32::INFINITY,
            time_since_update: 0.0,
        }
    }
}

impl AiLodState {
    /// Create a new LOD state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the LOD level based on distance.
    pub fn update_distance(&mut self, entity_pos: Vec3, player_pos: Vec3) {
        self.distance = entity_pos.distance(player_pos);
        let new_level = AiLodLevel::from_distance(self.distance);

        if new_level != self.level {
            self.level = new_level;
            self.time_since_update = 0.0; // Reset on level change
        }
    }

    /// Check if this entity should be updated this frame.
    #[must_use]
    pub fn should_update(&self, dt: f32) -> bool {
        let interval = self.level.update_interval_secs();
        self.time_since_update + dt >= interval
    }

    /// Mark that an update was performed.
    pub fn mark_updated(&mut self) {
        self.time_since_update = 0.0;
    }

    /// Advance time since last update.
    pub fn tick(&mut self, dt: f32) {
        self.time_since_update += dt;
    }
}

/// Manages AI LOD for all entities.
#[derive(Debug, Clone)]
pub struct AiLodManager {
    /// LOD states indexed by entity.
    states: std::collections::HashMap<u64, AiLodState>,
    /// Whether LOD is enabled.
    enabled: bool,
}

impl Default for AiLodManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AiLodManager {
    /// Create a new AI LOD manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            states: std::collections::HashMap::new(),
            enabled: true,
        }
    }

    /// Enable or disable LOD.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if LOD is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update LOD for all tracked entities.
    pub fn update_all(&mut self, player_pos: Vec3, positions: &[(u64, Vec3)]) {
        if !self.enabled {
            return;
        }

        for (id, pos) in positions {
            let state = self.states.entry(*id).or_default();
            state.update_distance(*pos, player_pos);
        }
    }

    /// Check if an entity should update its AI.
    #[must_use]
    pub fn should_update(&self, entity_id: u64, dt: f32) -> bool {
        if !self.enabled {
            return true; // Always update when LOD disabled
        }
        self.states
            .get(&entity_id)
            .map_or(true, |s| s.should_update(dt))
    }

    /// Mark an entity as updated.
    pub fn mark_updated(&mut self, entity_id: u64) {
        if let Some(state) = self.states.get_mut(&entity_id) {
            state.mark_updated();
        }
    }

    /// Tick all LOD states.
    pub fn tick(&mut self, dt: f32) {
        for state in self.states.values_mut() {
            state.tick(dt);
        }
    }

    /// Get the LOD level for an entity.
    #[must_use]
    pub fn level(&self, entity_id: u64) -> AiLodLevel {
        self.states
            .get(&entity_id)
            .map_or(AiLodLevel::Hibernated, |s| s.level)
    }

    /// Remove an entity from tracking.
    pub fn remove(&mut self, entity_id: u64) {
        self.states.remove(&entity_id);
    }

    /// Clear all tracked entities.
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Count entities at each LOD level.
    #[must_use]
    pub fn count_by_level(&self) -> (usize, usize, usize) {
        let mut full = 0;
        let mut simplified = 0;
        let mut hibernated = 0;

        for state in self.states.values() {
            match state.level {
                AiLodLevel::Full => full += 1,
                AiLodLevel::Simplified => simplified += 1,
                AiLodLevel::Hibernated => hibernated += 1,
            }
        }

        (full, simplified, hibernated)
    }

    /// Total tracked entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Check if no entities tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_from_distance() {
        assert_eq!(AiLodLevel::from_distance(10.0), AiLodLevel::Full);
        assert_eq!(AiLodLevel::from_distance(32.0), AiLodLevel::Full);
        assert_eq!(AiLodLevel::from_distance(50.0), AiLodLevel::Simplified);
        assert_eq!(AiLodLevel::from_distance(64.0), AiLodLevel::Simplified);
        assert_eq!(AiLodLevel::from_distance(100.0), AiLodLevel::Hibernated);
    }

    #[test]
    fn test_lod_allows_pathfinding() {
        assert!(AiLodLevel::Full.allows_pathfinding());
        assert!(!AiLodLevel::Simplified.allows_pathfinding());
        assert!(!AiLodLevel::Hibernated.allows_pathfinding());
    }

    #[test]
    fn test_lod_allows_updates() {
        assert!(AiLodLevel::Full.allows_updates());
        assert!(AiLodLevel::Simplified.allows_updates());
        assert!(!AiLodLevel::Hibernated.allows_updates());
    }

    #[test]
    fn test_lod_update_intervals() {
        assert_eq!(AiLodLevel::Full.update_interval_secs(), 0.0);
        assert_eq!(AiLodLevel::Simplified.update_interval_secs(), 1.0);
        assert!(AiLodLevel::Hibernated.update_interval_secs().is_infinite());
    }

    #[test]
    fn test_lod_state_update_distance() {
        let mut state = AiLodState::new();
        let player = Vec3::ZERO;

        state.update_distance(Vec3::new(20.0, 0.0, 0.0), player);
        assert_eq!(state.level, AiLodLevel::Full);

        state.update_distance(Vec3::new(50.0, 0.0, 0.0), player);
        assert_eq!(state.level, AiLodLevel::Simplified);

        state.update_distance(Vec3::new(100.0, 0.0, 0.0), player);
        assert_eq!(state.level, AiLodLevel::Hibernated);
    }

    #[test]
    fn test_lod_state_should_update() {
        let mut state = AiLodState::new();
        state.level = AiLodLevel::Simplified;

        // Just updated, shouldn't need update yet
        assert!(!state.should_update(0.5));

        // After 1 second, should update
        state.time_since_update = 0.5;
        assert!(state.should_update(0.5));
    }

    #[test]
    fn test_full_lod_always_updates() {
        let mut state = AiLodState::new();
        state.level = AiLodLevel::Full;
        assert!(state.should_update(0.0));
    }

    #[test]
    fn test_hibernated_never_updates() {
        let mut state = AiLodState::new();
        state.level = AiLodLevel::Hibernated;
        assert!(!state.should_update(100.0));
    }

    #[test]
    fn test_lod_manager_update_all() {
        let mut manager = AiLodManager::new();
        let player = Vec3::ZERO;
        let positions = vec![
            (1u64, Vec3::new(20.0, 0.0, 0.0)),
            (2u64, Vec3::new(50.0, 0.0, 0.0)),
            (3u64, Vec3::new(100.0, 0.0, 0.0)),
        ];

        manager.update_all(player, &positions);

        assert_eq!(manager.level(1), AiLodLevel::Full);
        assert_eq!(manager.level(2), AiLodLevel::Simplified);
        assert_eq!(manager.level(3), AiLodLevel::Hibernated);
    }

    #[test]
    fn test_lod_manager_count_by_level() {
        let mut manager = AiLodManager::new();
        let player = Vec3::ZERO;
        let positions = vec![
            (1u64, Vec3::new(20.0, 0.0, 0.0)),
            (2u64, Vec3::new(50.0, 0.0, 0.0)),
            (3u64, Vec3::new(100.0, 0.0, 0.0)),
            (4u64, Vec3::new(30.0, 0.0, 0.0)),
        ];

        manager.update_all(player, &positions);

        let (full, simplified, hibernated) = manager.count_by_level();
        assert_eq!(full, 2);
        assert_eq!(simplified, 1);
        assert_eq!(hibernated, 1);
    }

    #[test]
    fn test_lod_disabled_always_updates() {
        let mut manager = AiLodManager::new();
        manager.set_enabled(false);
        assert!(manager.should_update(1, 0.0));
    }

    #[test]
    fn test_remove_entity() {
        let mut manager = AiLodManager::new();
        manager.states.insert(42, AiLodState::new());
        manager.remove(42);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut manager = AiLodManager::new();
        manager.states.insert(1, AiLodState::new());
        manager.states.insert(2, AiLodState::new());
        manager.clear();
        assert!(manager.is_empty());
    }
}

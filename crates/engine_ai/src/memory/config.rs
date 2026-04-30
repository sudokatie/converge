//! Configuration types for creature memory.

use serde::{Deserialize, Serialize};

/// Configuration for memory strength decay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Decay rate per tick (multiplier, e.g., 0.99 = 1% decay per tick).
    pub decay_rate: f32,
    /// Minimum strength before memory is forgotten.
    pub forget_threshold: f32,
    /// Ticks of inactivity before decay accelerates.
    pub staleness_threshold: u64,
    /// Accelerated decay rate when stale.
    pub stale_decay_rate: f32,
}

impl DecayConfig {
    #[must_use]
    pub fn new(decay_rate: f32, forget_threshold: f32) -> Self {
        Self {
            decay_rate: decay_rate.clamp(0.0, 1.0),
            forget_threshold: forget_threshold.clamp(0.0, 1.0),
            staleness_threshold: 600,
            stale_decay_rate: decay_rate * 0.9,
        }
    }

    #[must_use]
    pub fn with_staleness(mut self, threshold: u64, accelerated_rate: f32) -> Self {
        self.staleness_threshold = threshold;
        self.stale_decay_rate = accelerated_rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn rapid() -> Self {
        Self {
            decay_rate: 0.95,
            forget_threshold: 0.1,
            staleness_threshold: 60,
            stale_decay_rate: 0.85,
        }
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            decay_rate: 0.995,
            forget_threshold: 0.05,
            staleness_threshold: 300,
            stale_decay_rate: 0.98,
        }
    }

    #[must_use]
    pub fn persistent() -> Self {
        Self {
            decay_rate: 0.999,
            forget_threshold: 0.01,
            staleness_threshold: 1200,
            stale_decay_rate: 0.995,
        }
    }

    #[must_use]
    pub fn calculate_decay(&self, staleness: u64) -> f32 {
        if staleness > self.staleness_threshold {
            self.stale_decay_rate
        } else {
            self.decay_rate
        }
    }
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Configuration for the memory store.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryStoreConfig {
    /// Maximum total memories to retain.
    pub max_memories: usize,
    /// Maximum danger zone memories.
    pub max_danger_zones: usize,
    /// Maximum food source memories.
    pub max_food_sources: usize,
    /// Maximum player trace memories.
    pub max_player_traces: usize,
    /// Decay configuration for danger zones.
    pub danger_decay: DecayConfig,
    /// Decay configuration for food sources.
    pub food_decay: DecayConfig,
    /// Decay configuration for player traces.
    pub player_decay: DecayConfig,
    /// Interval between prune operations (ticks).
    pub prune_interval: u64,
    /// Whether to merge nearby memories of the same type.
    pub enable_merge: bool,
    /// Distance threshold for merging memories.
    pub merge_distance: f32,
}

impl MemoryStoreConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_memories(mut self, max: usize) -> Self {
        self.max_memories = max;
        self
    }

    #[must_use]
    pub fn with_max_danger_zones(mut self, max: usize) -> Self {
        self.max_danger_zones = max;
        self
    }

    #[must_use]
    pub fn with_max_food_sources(mut self, max: usize) -> Self {
        self.max_food_sources = max;
        self
    }

    #[must_use]
    pub fn with_max_player_traces(mut self, max: usize) -> Self {
        self.max_player_traces = max;
        self
    }

    #[must_use]
    pub fn with_danger_decay(mut self, config: DecayConfig) -> Self {
        self.danger_decay = config;
        self
    }

    #[must_use]
    pub fn with_food_decay(mut self, config: DecayConfig) -> Self {
        self.food_decay = config;
        self
    }

    #[must_use]
    pub fn with_player_decay(mut self, config: DecayConfig) -> Self {
        self.player_decay = config;
        self
    }

    #[must_use]
    pub fn with_merge(mut self, enabled: bool, distance: f32) -> Self {
        self.enable_merge = enabled;
        self.merge_distance = distance.max(0.0);
        self
    }

    #[must_use]
    pub fn minimal() -> Self {
        Self {
            max_memories: 30,
            max_danger_zones: 10,
            max_food_sources: 10,
            max_player_traces: 10,
            danger_decay: DecayConfig::rapid(),
            food_decay: DecayConfig::standard(),
            player_decay: DecayConfig::rapid(),
            prune_interval: 30,
            enable_merge: false,
            merge_distance: 5.0,
        }
    }

    #[must_use]
    pub fn predator() -> Self {
        Self {
            max_memories: 100,
            max_danger_zones: 20,
            max_food_sources: 30,
            max_player_traces: 50,
            danger_decay: DecayConfig::standard(),
            food_decay: DecayConfig::persistent(),
            player_decay: DecayConfig::persistent(),
            prune_interval: 60,
            enable_merge: true,
            merge_distance: 10.0,
        }
    }

    #[must_use]
    pub fn prey() -> Self {
        Self {
            max_memories: 80,
            max_danger_zones: 40,
            max_food_sources: 20,
            max_player_traces: 20,
            danger_decay: DecayConfig::persistent(),
            food_decay: DecayConfig::standard(),
            player_decay: DecayConfig::standard(),
            prune_interval: 60,
            enable_merge: true,
            merge_distance: 8.0,
        }
    }
}

impl Default for MemoryStoreConfig {
    fn default() -> Self {
        Self {
            max_memories: 50,
            max_danger_zones: 15,
            max_food_sources: 20,
            max_player_traces: 15,
            danger_decay: DecayConfig::standard(),
            food_decay: DecayConfig::standard(),
            player_decay: DecayConfig::standard(),
            prune_interval: 60,
            enable_merge: false,
            merge_distance: 5.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_config_new() {
        let config = DecayConfig::new(0.98, 0.1);
        assert!((config.decay_rate - 0.98).abs() < f32::EPSILON);
        assert!((config.forget_threshold - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_decay_config_clamp() {
        let config = DecayConfig::new(1.5, -0.5);
        assert!((config.decay_rate - 1.0).abs() < f32::EPSILON);
        assert!(config.forget_threshold.abs() < f32::EPSILON);
    }

    #[test]
    fn test_decay_config_presets() {
        let rapid = DecayConfig::rapid();
        let standard = DecayConfig::standard();
        let persistent = DecayConfig::persistent();

        assert!(rapid.decay_rate < standard.decay_rate);
        assert!(standard.decay_rate < persistent.decay_rate);
    }

    #[test]
    fn test_decay_config_staleness() {
        let config = DecayConfig::standard();
        let fresh = config.calculate_decay(100);
        let stale = config.calculate_decay(500);

        assert!((fresh - config.decay_rate).abs() < f32::EPSILON);
        assert!((stale - config.stale_decay_rate).abs() < f32::EPSILON);
    }

    #[test]
    fn test_memory_store_config_default() {
        let config = MemoryStoreConfig::default();
        assert_eq!(config.max_memories, 50);
        assert!(!config.enable_merge);
    }

    #[test]
    fn test_memory_store_config_builder() {
        let config = MemoryStoreConfig::new()
            .with_max_memories(100)
            .with_max_danger_zones(30)
            .with_merge(false, 10.0);

        assert_eq!(config.max_memories, 100);
        assert_eq!(config.max_danger_zones, 30);
        assert!(!config.enable_merge);
        assert!((config.merge_distance - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_memory_store_config_presets() {
        let minimal = MemoryStoreConfig::minimal();
        let predator = MemoryStoreConfig::predator();
        let prey = MemoryStoreConfig::prey();

        assert!(minimal.max_memories < predator.max_memories);
        assert!(prey.max_danger_zones > prey.max_food_sources);
        assert!(predator.max_player_traces > prey.max_player_traces);
    }

    #[test]
    fn test_decay_config_serde() {
        let config = DecayConfig::persistent();
        let json = serde_json::to_string(&config).unwrap();
        let restored: DecayConfig = serde_json::from_str(&json).unwrap();

        assert!((restored.decay_rate - config.decay_rate).abs() < f32::EPSILON);
        assert!((restored.forget_threshold - config.forget_threshold).abs() < f32::EPSILON);
    }

    #[test]
    fn test_memory_store_config_serde() {
        let config = MemoryStoreConfig::predator();
        let json = serde_json::to_string(&config).unwrap();
        let restored: MemoryStoreConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.max_memories, config.max_memories);
        assert_eq!(restored.enable_merge, config.enable_merge);
    }
}

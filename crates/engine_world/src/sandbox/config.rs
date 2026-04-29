//! Configuration for the scenario sandbox.

use serde::{Deserialize, Serialize};

use crate::environment::{HazardKind, PropagationConfig};

/// Configuration for sandbox behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Seed for deterministic simulation.
    pub seed: u64,

    /// Whether to auto-create chunks when spawning at unloaded positions.
    pub auto_create_chunks: bool,

    /// Maximum chunks to keep loaded (0 = unlimited).
    pub max_chunks: usize,

    /// Whether to record simulation history for replay.
    pub record_history: bool,

    /// Maximum history entries to keep (0 = unlimited).
    pub max_history: usize,

    /// Hazard propagation configs per kind.
    pub hazard_configs: [PropagationConfig; HazardKind::COUNT],

    /// Default delta time for simulation steps.
    pub default_dt: f32,

    /// Whether to enable boundary propagation between chunks.
    pub enable_boundary_propagation: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            auto_create_chunks: true,
            max_chunks: 256,
            record_history: true,
            max_history: 10000,
            hazard_configs: std::array::from_fn(|i| PropagationConfig::new(HazardKind::ALL[i])),
            default_dt: 0.1,
            enable_boundary_propagation: true,
        }
    }
}

impl SandboxConfig {
    /// Create a new config with given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Create a minimal config for fast testing (no history, limited chunks).
    #[must_use]
    pub fn minimal(seed: u64) -> Self {
        Self {
            seed,
            auto_create_chunks: true,
            max_chunks: 64,
            record_history: false,
            max_history: 0,
            hazard_configs: std::array::from_fn(|i| PropagationConfig::new(HazardKind::ALL[i])),
            default_dt: 0.1,
            enable_boundary_propagation: true,
        }
    }

    /// Create a config optimized for deterministic replay.
    #[must_use]
    pub fn replay(seed: u64) -> Self {
        Self {
            seed,
            auto_create_chunks: true,
            max_chunks: 0,
            record_history: true,
            max_history: 0,
            hazard_configs: std::array::from_fn(|i| PropagationConfig::new(HazardKind::ALL[i])),
            default_dt: 0.1,
            enable_boundary_propagation: true,
        }
    }

    /// Set the hazard config for a specific kind.
    pub fn set_hazard_config(&mut self, kind: HazardKind, config: PropagationConfig) {
        self.hazard_configs[kind.as_index()] = config;
    }

    /// Get the hazard config for a specific kind.
    #[must_use]
    pub fn hazard_config(&self, kind: HazardKind) -> &PropagationConfig {
        &self.hazard_configs[kind.as_index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = SandboxConfig::default();
        assert_eq!(config.seed, 0);
        assert!(config.auto_create_chunks);
        assert!(config.record_history);
        assert!(config.enable_boundary_propagation);
    }

    #[test]
    fn minimal_config() {
        let config = SandboxConfig::minimal(42);
        assert_eq!(config.seed, 42);
        assert!(!config.record_history);
        assert_eq!(config.max_chunks, 64);
    }

    #[test]
    fn replay_config() {
        let config = SandboxConfig::replay(123);
        assert_eq!(config.seed, 123);
        assert!(config.record_history);
        assert_eq!(config.max_chunks, 0);
        assert_eq!(config.max_history, 0);
    }

    #[test]
    fn set_hazard_config() {
        let mut config = SandboxConfig::new(0);
        let fire_config = config.hazard_config(HazardKind::Fire).clone();

        let mut new_config = fire_config;
        new_config.spread.rate = 2.0;
        config.set_hazard_config(HazardKind::Fire, new_config.clone());

        assert!((config.hazard_config(HazardKind::Fire).spread.rate - 2.0).abs() < 0.001);
    }

    #[test]
    fn serde_round_trip() {
        let config = SandboxConfig::new(999);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: SandboxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.seed, config.seed);
    }
}

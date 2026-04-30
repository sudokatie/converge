//! Configuration for region graph generation.

use serde::{Deserialize, Serialize};

use super::gate::ProgressionTier;
use super::region_kind::RegionKind;

/// Configuration for region graph generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "config struct with boolean options"
)]
pub struct RegionGraphConfig {
    /// Random seed for generation.
    pub seed: u64,
    /// Total number of regions to generate.
    pub region_count: u32,
    /// Minimum connections per region.
    pub min_connections: u32,
    /// Maximum connections per region.
    pub max_connections: u32,
    /// Probability of creating loop edges (0.0-1.0).
    pub loop_probability: f32,
    /// Number of progression tiers.
    pub tier_count: u8,
    /// Regions per tier (approximate).
    pub regions_per_tier: u32,
    /// Probability of dead ends (0.0-1.0).
    pub dead_end_probability: f32,
    /// Probability of branch points (0.0-1.0).
    pub branch_probability: f32,
    /// Whether to generate hazard regions.
    pub enable_hazards: bool,
    /// Probability of hazard regions (0.0-1.0).
    pub hazard_probability: f32,
    /// Whether to generate resource regions.
    pub enable_resources: bool,
    /// Probability of resource regions (0.0-1.0).
    pub resource_probability: f32,
    /// Region kind weights for generation.
    pub kind_weights: KindWeights,
    /// Whether to enforce connectivity (all regions reachable).
    pub enforce_connectivity: bool,
    /// Whether to create a clear critical path.
    pub create_critical_path: bool,
    /// Minimum critical path length.
    pub min_critical_path_length: u32,
}

impl RegionGraphConfig {
    /// Create a new config with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            region_count: 20,
            min_connections: 1,
            max_connections: 4,
            loop_probability: 0.2,
            tier_count: 3,
            regions_per_tier: 7,
            dead_end_probability: 0.15,
            branch_probability: 0.25,
            enable_hazards: true,
            hazard_probability: 0.1,
            enable_resources: true,
            resource_probability: 0.2,
            kind_weights: KindWeights::default(),
            enforce_connectivity: true,
            create_critical_path: true,
            min_critical_path_length: 5,
        }
    }

    /// Create a small graph config.
    #[must_use]
    pub fn small(seed: u64) -> Self {
        Self::new(seed)
            .with_region_count(10)
            .with_tiers(2, 5)
            .with_critical_path(3)
    }

    /// Create a medium graph config.
    #[must_use]
    pub fn medium(seed: u64) -> Self {
        Self::new(seed)
            .with_region_count(30)
            .with_tiers(4, 8)
            .with_critical_path(8)
    }

    /// Create a large graph config.
    #[must_use]
    pub fn large(seed: u64) -> Self {
        Self::new(seed)
            .with_region_count(60)
            .with_tiers(6, 10)
            .with_critical_path(15)
    }

    /// Set the region count.
    #[must_use]
    pub fn with_region_count(mut self, count: u32) -> Self {
        self.region_count = count.max(2);
        self
    }

    /// Set connection limits.
    #[must_use]
    pub fn with_connections(mut self, min: u32, max: u32) -> Self {
        self.min_connections = min.max(1);
        self.max_connections = max.max(self.min_connections);
        self
    }

    /// Set loop probability.
    #[must_use]
    pub fn with_loop_probability(mut self, prob: f32) -> Self {
        self.loop_probability = prob.clamp(0.0, 1.0);
        self
    }

    /// Set tier configuration.
    #[must_use]
    pub fn with_tiers(mut self, count: u8, regions_per: u32) -> Self {
        self.tier_count = count.max(1).min(ProgressionTier::MAX.value());
        self.regions_per_tier = regions_per.max(1);
        self
    }

    /// Set dead end probability.
    #[must_use]
    pub fn with_dead_end_probability(mut self, prob: f32) -> Self {
        self.dead_end_probability = prob.clamp(0.0, 0.5);
        self
    }

    /// Set branch probability.
    #[must_use]
    pub fn with_branch_probability(mut self, prob: f32) -> Self {
        self.branch_probability = prob.clamp(0.0, 0.5);
        self
    }

    /// Enable or disable hazards.
    #[must_use]
    pub fn with_hazards(mut self, enabled: bool, probability: f32) -> Self {
        self.enable_hazards = enabled;
        self.hazard_probability = probability.clamp(0.0, 0.5);
        self
    }

    /// Enable or disable resources.
    #[must_use]
    pub fn with_resources(mut self, enabled: bool, probability: f32) -> Self {
        self.enable_resources = enabled;
        self.resource_probability = probability.clamp(0.0, 0.5);
        self
    }

    /// Set kind weights.
    #[must_use]
    pub fn with_kind_weights(mut self, weights: KindWeights) -> Self {
        self.kind_weights = weights;
        self
    }

    /// Set connectivity enforcement.
    #[must_use]
    pub fn with_connectivity(mut self, enforce: bool) -> Self {
        self.enforce_connectivity = enforce;
        self
    }

    /// Set critical path parameters.
    #[must_use]
    pub fn with_critical_path(mut self, min_length: u32) -> Self {
        self.create_critical_path = true;
        self.min_critical_path_length = min_length.max(2);
        self
    }

    /// Disable critical path generation.
    #[must_use]
    pub fn without_critical_path(mut self) -> Self {
        self.create_critical_path = false;
        self
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.region_count < 2 {
            return Err(ConfigError::TooFewRegions);
        }
        if self.max_connections < self.min_connections {
            return Err(ConfigError::InvalidConnectionRange);
        }
        if self.tier_count == 0 {
            return Err(ConfigError::NoTiers);
        }
        if self.create_critical_path && self.min_critical_path_length > self.region_count {
            return Err(ConfigError::CriticalPathTooLong);
        }
        Ok(())
    }
}

impl Default for RegionGraphConfig {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Weights for region kind selection during generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KindWeights {
    pub generic: u32,
    pub station: u32,
    pub trench: u32,
    pub cave: u32,
    pub sphere: u32,
    pub colony: u32,
    pub hub: u32,
}

impl KindWeights {
    /// Create uniform weights.
    #[must_use]
    pub fn uniform() -> Self {
        Self {
            generic: 10,
            station: 10,
            trench: 10,
            cave: 10,
            sphere: 10,
            colony: 10,
            hub: 10,
        }
    }

    /// Create weights favoring stations and colonies.
    #[must_use]
    pub fn settlement_heavy() -> Self {
        Self {
            generic: 5,
            station: 25,
            trench: 5,
            cave: 10,
            sphere: 5,
            colony: 25,
            hub: 15,
        }
    }

    /// Create weights favoring natural formations.
    #[must_use]
    pub fn natural() -> Self {
        Self {
            generic: 10,
            station: 5,
            trench: 25,
            cave: 25,
            sphere: 15,
            colony: 5,
            hub: 10,
        }
    }

    /// Get total weight.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.generic + self.station + self.trench + self.cave + self.sphere + self.colony + self.hub
    }

    /// Select a kind based on a random value (0..total).
    #[must_use]
    pub fn select(&self, value: u32) -> RegionKind {
        let value = value % self.total();
        let mut cumulative = 0;

        cumulative += self.generic;
        if value < cumulative {
            return RegionKind::Generic;
        }
        cumulative += self.station;
        if value < cumulative {
            return RegionKind::Station;
        }
        cumulative += self.trench;
        if value < cumulative {
            return RegionKind::Trench;
        }
        cumulative += self.cave;
        if value < cumulative {
            return RegionKind::Cave;
        }
        cumulative += self.sphere;
        if value < cumulative {
            return RegionKind::Sphere;
        }
        cumulative += self.colony;
        if value < cumulative {
            return RegionKind::Colony;
        }

        RegionKind::Hub
    }
}

impl Default for KindWeights {
    fn default() -> Self {
        Self {
            generic: 20,
            station: 15,
            trench: 10,
            cave: 15,
            sphere: 5,
            colony: 10,
            hub: 10,
        }
    }
}

/// Configuration validation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Too few regions specified.
    TooFewRegions,
    /// Invalid connection range.
    InvalidConnectionRange,
    /// No tiers specified.
    NoTiers,
    /// Critical path longer than region count.
    CriticalPathTooLong,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewRegions => write!(f, "region count must be at least 2"),
            Self::InvalidConnectionRange => {
                write!(f, "max_connections must be >= min_connections")
            }
            Self::NoTiers => write!(f, "tier count must be at least 1"),
            Self::CriticalPathTooLong => {
                write!(f, "critical path length exceeds region count")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = RegionGraphConfig::new(42);
        assert_eq!(config.seed, 42);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_presets() {
        let small = RegionGraphConfig::small(1);
        assert!(small.validate().is_ok());
        assert_eq!(small.region_count, 10);

        let medium = RegionGraphConfig::medium(1);
        assert!(medium.validate().is_ok());
        assert_eq!(medium.region_count, 30);

        let large = RegionGraphConfig::large(1);
        assert!(large.validate().is_ok());
        assert_eq!(large.region_count, 60);
    }

    #[test]
    fn config_validation() {
        let mut bad_regions = RegionGraphConfig::new(1);
        bad_regions.region_count = 1;
        assert_eq!(bad_regions.validate(), Err(ConfigError::TooFewRegions));

        let bad_path = RegionGraphConfig::new(1)
            .with_region_count(5)
            .with_critical_path(10);
        assert_eq!(bad_path.validate(), Err(ConfigError::CriticalPathTooLong));
    }

    #[test]
    fn kind_weights_selection() {
        let weights = KindWeights::uniform();
        let total = weights.total();
        assert_eq!(total, 70);

        let kind = weights.select(0);
        assert_eq!(kind, RegionKind::Generic);

        let kind = weights.select(15);
        assert_eq!(kind, RegionKind::Station);
    }

    #[test]
    fn serde_roundtrip() {
        let config = RegionGraphConfig::medium(12345)
            .with_hazards(true, 0.2)
            .with_resources(true, 0.3);

        let json = serde_json::to_string(&config).unwrap();
        let recovered: RegionGraphConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }
}

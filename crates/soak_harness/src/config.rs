//! Configuration for soak test runs.

use serde::{Deserialize, Serialize};

/// Region setup specification for soak tests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionSetup {
    /// Grid dimensions (x, y, z) for region spawning.
    pub grid_size: [i32; 3],
    /// Center offset for the region grid.
    pub center: [i32; 3],
    /// Whether to spawn hazards in each region.
    pub spawn_hazards: bool,
    /// Initial hazard intensity (0.0 to 1.0).
    pub hazard_intensity: f32,
}

impl Default for RegionSetup {
    fn default() -> Self {
        Self {
            grid_size: [3, 3, 1],
            center: [0, 0, 0],
            spawn_hazards: true,
            hazard_intensity: 0.8,
        }
    }
}

impl RegionSetup {
    /// Single-chunk region for minimal tests.
    #[must_use]
    pub fn single() -> Self {
        Self {
            grid_size: [1, 1, 1],
            center: [0, 0, 0],
            spawn_hazards: true,
            hazard_intensity: 1.0,
        }
    }

    /// Small 3x3 region grid.
    #[must_use]
    pub fn small() -> Self {
        Self::default()
    }

    /// Medium 5x5 region grid.
    #[must_use]
    pub fn medium() -> Self {
        Self {
            grid_size: [5, 5, 2],
            center: [0, 0, 0],
            spawn_hazards: true,
            hazard_intensity: 0.6,
        }
    }

    /// Large 8x8 region grid for stress testing.
    #[must_use]
    pub fn large() -> Self {
        Self {
            grid_size: [8, 8, 3],
            center: [0, 0, 0],
            spawn_hazards: true,
            hazard_intensity: 0.5,
        }
    }

    /// Total number of regions in this setup.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn region_count(&self) -> usize {
        (self.grid_size[0] * self.grid_size[1] * self.grid_size[2]) as usize
    }
}

/// Configuration for a soak test run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoakConfig {
    /// Seed for deterministic simulation.
    pub seed: u64,
    /// Total ticks to simulate.
    pub tick_count: u64,
    /// Maximum wall-clock duration in seconds (0 = unlimited).
    pub max_duration_secs: u64,
    /// Delta time per simulation tick.
    pub dt: f32,
    /// Region setup specification.
    pub regions: RegionSetup,
    /// Tick interval for checkpoint reports (0 = none).
    pub checkpoint_interval: u64,
    /// Whether to enable invariant checking.
    pub check_invariants: bool,
    /// Stop on first invariant violation.
    pub fail_fast: bool,
    /// Maximum invariant violations before aborting.
    pub max_violations: usize,
    /// Enable verbose logging.
    pub verbose: bool,
}

impl Default for SoakConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            tick_count: 1000,
            max_duration_secs: 0,
            dt: 0.1,
            regions: RegionSetup::default(),
            checkpoint_interval: 100,
            check_invariants: true,
            fail_fast: false,
            max_violations: 100,
            verbose: false,
        }
    }
}

impl SoakConfig {
    /// Create a config with a specific seed and tick count.
    #[must_use]
    pub fn new(seed: u64, tick_count: u64) -> Self {
        Self {
            seed,
            tick_count,
            ..Default::default()
        }
    }

    /// Quick smoke test configuration (100 ticks, single region).
    #[must_use]
    pub fn smoke() -> Self {
        Self {
            seed: 0,
            tick_count: 100,
            max_duration_secs: 10,
            dt: 0.1,
            regions: RegionSetup::single(),
            checkpoint_interval: 50,
            check_invariants: true,
            fail_fast: true,
            max_violations: 1,
            verbose: false,
        }
    }

    /// Short soak configuration (1000 ticks).
    #[must_use]
    pub fn short() -> Self {
        Self {
            seed: 42,
            tick_count: 1000,
            max_duration_secs: 60,
            dt: 0.1,
            regions: RegionSetup::small(),
            checkpoint_interval: 100,
            check_invariants: true,
            fail_fast: false,
            max_violations: 10,
            verbose: false,
        }
    }

    /// Medium soak configuration (10,000 ticks).
    #[must_use]
    pub fn medium() -> Self {
        Self {
            seed: 42,
            tick_count: 10_000,
            max_duration_secs: 300,
            dt: 0.1,
            regions: RegionSetup::medium(),
            checkpoint_interval: 1000,
            check_invariants: true,
            fail_fast: false,
            max_violations: 50,
            verbose: false,
        }
    }

    /// Long overnight soak configuration (100,000 ticks).
    #[must_use]
    pub fn overnight() -> Self {
        Self {
            seed: 42,
            tick_count: 100_000,
            max_duration_secs: 28800,
            dt: 0.1,
            regions: RegionSetup::large(),
            checkpoint_interval: 10_000,
            check_invariants: true,
            fail_fast: false,
            max_violations: 100,
            verbose: false,
        }
    }

    /// Validate the configuration.
    ///
    /// # Errors
    /// Returns an error if any configuration value is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.tick_count == 0 {
            return Err(ConfigError::InvalidTickCount);
        }
        if self.dt <= 0.0 || self.dt > 10.0 {
            return Err(ConfigError::InvalidDt);
        }
        if self.regions.region_count() == 0 {
            return Err(ConfigError::NoRegions);
        }
        if self.regions.hazard_intensity < 0.0 || self.regions.hazard_intensity > 1.0 {
            return Err(ConfigError::InvalidHazardIntensity);
        }
        Ok(())
    }

    /// Parse config from JSON string.
    ///
    /// # Errors
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize config to JSON string.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Configuration validation error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("tick_count must be greater than 0")]
    InvalidTickCount,
    #[error("dt must be between 0 and 10")]
    InvalidDt,
    #[error("at least one region must be configured")]
    NoRegions,
    #[error("hazard_intensity must be between 0.0 and 1.0")]
    InvalidHazardIntensity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_valid() {
        let config = SoakConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn smoke_config_valid() {
        let config = SoakConfig::smoke();
        assert!(config.validate().is_ok());
        assert_eq!(config.tick_count, 100);
        assert!(config.fail_fast);
    }

    #[test]
    fn short_config_valid() {
        let config = SoakConfig::short();
        assert!(config.validate().is_ok());
        assert_eq!(config.tick_count, 1000);
    }

    #[test]
    fn medium_config_valid() {
        let config = SoakConfig::medium();
        assert!(config.validate().is_ok());
        assert_eq!(config.tick_count, 10_000);
    }

    #[test]
    fn overnight_config_valid() {
        let config = SoakConfig::overnight();
        assert!(config.validate().is_ok());
        assert_eq!(config.tick_count, 100_000);
    }

    #[test]
    fn invalid_tick_count() {
        let config = SoakConfig {
            tick_count: 0,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidTickCount)
        ));
    }

    #[test]
    fn invalid_dt() {
        let config = SoakConfig {
            dt: -1.0,
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(ConfigError::InvalidDt)));
    }

    #[test]
    fn invalid_hazard_intensity() {
        let config = SoakConfig {
            regions: RegionSetup {
                hazard_intensity: 2.0,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidHazardIntensity)
        ));
    }

    #[test]
    fn region_count() {
        assert_eq!(RegionSetup::single().region_count(), 1);
        assert_eq!(RegionSetup::small().region_count(), 9);
        assert_eq!(RegionSetup::medium().region_count(), 50);
        assert_eq!(RegionSetup::large().region_count(), 192);
    }

    #[test]
    fn json_round_trip() {
        let config = SoakConfig::short();
        let json = config.to_json().unwrap();
        let recovered = SoakConfig::from_json(&json).unwrap();
        assert_eq!(recovered.seed, config.seed);
        assert_eq!(recovered.tick_count, config.tick_count);
    }
}

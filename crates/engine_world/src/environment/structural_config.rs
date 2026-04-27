//! Configuration for structural integrity simulation.

use serde::{Deserialize, Serialize};

/// Configuration for support propagation through the structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SupportPropagationConfig {
    /// Maximum distance support can propagate from foundation.
    pub max_propagation_distance: u8,
    /// Load transfer efficiency for vertical connections (0.0 to 1.0).
    pub vertical_efficiency: f32,
    /// Load transfer efficiency for horizontal connections (0.0 to 1.0).
    pub horizontal_efficiency: f32,
    /// Load transfer efficiency for diagonal connections (0.0 to 1.0).
    pub diagonal_efficiency: f32,
    /// Whether to include edge neighbors in support calculations.
    pub include_edge_neighbors: bool,
    /// Whether to include corner neighbors in support calculations.
    pub include_corner_neighbors: bool,
}

impl SupportPropagationConfig {
    /// Default configuration for solid structures.
    pub const SOLID: Self = Self {
        max_propagation_distance: 32,
        vertical_efficiency: 0.95,
        horizontal_efficiency: 0.7,
        diagonal_efficiency: 0.5,
        include_edge_neighbors: true,
        include_corner_neighbors: false,
    };

    /// Configuration for rigid structures with longer spans.
    pub const RIGID: Self = Self {
        max_propagation_distance: 64,
        vertical_efficiency: 0.98,
        horizontal_efficiency: 0.85,
        diagonal_efficiency: 0.7,
        include_edge_neighbors: true,
        include_corner_neighbors: true,
    };

    /// Configuration for weak/fragile structures.
    pub const FRAGILE: Self = Self {
        max_propagation_distance: 12,
        vertical_efficiency: 0.8,
        horizontal_efficiency: 0.4,
        diagonal_efficiency: 0.2,
        include_edge_neighbors: false,
        include_corner_neighbors: false,
    };
}

impl Default for SupportPropagationConfig {
    fn default() -> Self {
        Self::SOLID
    }
}

/// Configuration for load distribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadConfig {
    /// Base load per solid cell (normalized 0.0 to 1.0).
    pub base_cell_load: f32,
    /// Gravity multiplier for downward load transfer.
    pub gravity_factor: f32,
    /// Load accumulation rate per simulation step.
    pub accumulation_rate: f32,
    /// Load distribution rate to neighbors.
    pub distribution_rate: f32,
}

impl LoadConfig {
    /// Default load configuration.
    pub const DEFAULT: Self = Self {
        base_cell_load: 0.05,
        gravity_factor: 1.5,
        accumulation_rate: 0.2,
        distribution_rate: 0.3,
    };

    /// Light materials configuration.
    pub const LIGHT: Self = Self {
        base_cell_load: 0.02,
        gravity_factor: 1.2,
        accumulation_rate: 0.15,
        distribution_rate: 0.25,
    };

    /// Heavy materials configuration.
    pub const HEAVY: Self = Self {
        base_cell_load: 0.1,
        gravity_factor: 2.0,
        accumulation_rate: 0.3,
        distribution_rate: 0.4,
    };
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Configuration for stability checks and failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StabilityConfig {
    /// Stress threshold for warning (cells becoming overstressed).
    pub warning_threshold: f32,
    /// Stress threshold for structural failure.
    pub failure_threshold: f32,
    /// Integrity damage per tick when overstressed.
    pub overstress_damage_rate: f32,
    /// Minimum integrity before automatic collapse.
    pub collapse_integrity_threshold: f32,
    /// Whether unsupported cells collapse immediately.
    pub instant_unsupported_collapse: bool,
    /// Grace period (in seconds) before unsupported collapse.
    pub unsupported_grace_period: f32,
}

impl StabilityConfig {
    /// Default stability configuration.
    pub const DEFAULT: Self = Self {
        warning_threshold: 0.7,
        failure_threshold: 1.0,
        overstress_damage_rate: 0.1,
        collapse_integrity_threshold: 0.1,
        instant_unsupported_collapse: false,
        unsupported_grace_period: 0.5,
    };

    /// Strict stability (collapses quickly).
    pub const STRICT: Self = Self {
        warning_threshold: 0.5,
        failure_threshold: 0.9,
        overstress_damage_rate: 0.2,
        collapse_integrity_threshold: 0.2,
        instant_unsupported_collapse: true,
        unsupported_grace_period: 0.0,
    };

    /// Lenient stability (allows more stress).
    pub const LENIENT: Self = Self {
        warning_threshold: 0.85,
        failure_threshold: 1.0,
        overstress_damage_rate: 0.05,
        collapse_integrity_threshold: 0.05,
        instant_unsupported_collapse: false,
        unsupported_grace_period: 2.0,
    };
}

impl Default for StabilityConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Combined configuration for structural integrity simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructuralConfig {
    /// Support propagation settings.
    pub propagation: SupportPropagationConfig,
    /// Load distribution settings.
    pub load: LoadConfig,
    /// Stability and failure settings.
    pub stability: StabilityConfig,
    /// Enable decompression events from pressure differentials.
    pub decompression_enabled: bool,
    /// Pressure differential threshold for decompression damage.
    pub decompression_threshold: f32,
    /// Enable cave-in cascades.
    pub cavein_enabled: bool,
    /// Maximum cells affected by a single cave-in cascade.
    pub max_cascade_size: u32,
}

impl StructuralConfig {
    /// Default structural configuration.
    pub const DEFAULT: Self = Self {
        propagation: SupportPropagationConfig::SOLID,
        load: LoadConfig::DEFAULT,
        stability: StabilityConfig::DEFAULT,
        decompression_enabled: true,
        decompression_threshold: 0.5,
        cavein_enabled: true,
        max_cascade_size: 256,
    };

    /// Configuration for space/vacuum environments.
    pub const SPACE: Self = Self {
        propagation: SupportPropagationConfig::RIGID,
        load: LoadConfig::LIGHT,
        stability: StabilityConfig::STRICT,
        decompression_enabled: true,
        decompression_threshold: 0.3,
        cavein_enabled: false,
        max_cascade_size: 64,
    };

    /// Configuration for underground/cave environments.
    pub const UNDERGROUND: Self = Self {
        propagation: SupportPropagationConfig::SOLID,
        load: LoadConfig::HEAVY,
        stability: StabilityConfig::DEFAULT,
        decompression_enabled: false,
        decompression_threshold: 1.0,
        cavein_enabled: true,
        max_cascade_size: 512,
    };

    /// Create a new config with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::DEFAULT
    }

    /// Check if configuration is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.propagation.vertical_efficiency >= 0.0
            && self.propagation.vertical_efficiency <= 1.0
            && self.propagation.horizontal_efficiency >= 0.0
            && self.propagation.horizontal_efficiency <= 1.0
            && self.stability.warning_threshold < self.stability.failure_threshold
            && self.load.base_cell_load >= 0.0
    }
}

impl Default for StructuralConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_propagation_presets() {
        assert!(SupportPropagationConfig::SOLID.include_edge_neighbors);
        assert!(!SupportPropagationConfig::SOLID.include_corner_neighbors);

        assert!(SupportPropagationConfig::RIGID.include_corner_neighbors);
        assert_eq!(SupportPropagationConfig::RIGID.max_propagation_distance, 64);

        assert!(!SupportPropagationConfig::FRAGILE.include_edge_neighbors);
        assert!(SupportPropagationConfig::FRAGILE.diagonal_efficiency < 0.3);
    }

    #[test]
    fn load_config_presets() {
        assert!(LoadConfig::LIGHT.base_cell_load < LoadConfig::HEAVY.base_cell_load);
        assert!(LoadConfig::LIGHT.gravity_factor < LoadConfig::HEAVY.gravity_factor);
    }

    #[test]
    fn stability_config_presets() {
        assert!(
            StabilityConfig::STRICT.warning_threshold < StabilityConfig::DEFAULT.warning_threshold
        );
        assert!(StabilityConfig::STRICT.instant_unsupported_collapse);
        assert!(!StabilityConfig::LENIENT.instant_unsupported_collapse);
    }

    #[test]
    fn structural_config_presets() {
        let default = StructuralConfig::DEFAULT;
        assert!(default.decompression_enabled);
        assert!(default.cavein_enabled);

        let space = StructuralConfig::SPACE;
        assert!(!space.cavein_enabled);
        assert!(space.decompression_threshold < default.decompression_threshold);

        let underground = StructuralConfig::UNDERGROUND;
        assert!(!underground.decompression_enabled);
        assert!(underground.max_cascade_size > default.max_cascade_size);
    }

    #[test]
    fn config_is_valid() {
        assert!(StructuralConfig::DEFAULT.is_valid());
        assert!(StructuralConfig::SPACE.is_valid());
        assert!(StructuralConfig::UNDERGROUND.is_valid());
    }

    #[test]
    fn config_invalid_cases() {
        let mut config = StructuralConfig::DEFAULT;
        config.propagation.vertical_efficiency = -0.5;
        assert!(!config.is_valid());

        let mut config2 = StructuralConfig::DEFAULT;
        config2.stability.warning_threshold = 1.5;
        config2.stability.failure_threshold = 1.0;
        assert!(!config2.is_valid());
    }

    #[test]
    fn defaults() {
        assert_eq!(
            SupportPropagationConfig::default(),
            SupportPropagationConfig::SOLID
        );
        assert_eq!(LoadConfig::default(), LoadConfig::DEFAULT);
        assert_eq!(StabilityConfig::default(), StabilityConfig::DEFAULT);
        assert_eq!(StructuralConfig::default(), StructuralConfig::DEFAULT);
    }

    #[test]
    fn serde_support_propagation() {
        let config = SupportPropagationConfig::RIGID;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: SupportPropagationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_load_config() {
        let config = LoadConfig::HEAVY;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: LoadConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_stability_config() {
        let config = StabilityConfig::STRICT;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: StabilityConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_structural_config() {
        let config = StructuralConfig::SPACE;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: StructuralConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }
}

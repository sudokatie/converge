//! Configuration for topology generation.

use serde::{Deserialize, Serialize};

use super::kind::TopologyKind;

/// Configuration for topology generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyConfig {
    /// Random seed for generation.
    pub seed: u64,
    /// Kind of topology to generate.
    pub kind: TopologyKind,
    /// Number of nodes to generate.
    pub node_count: u32,
    /// Minimum segment width.
    pub min_width: f32,
    /// Maximum segment width.
    pub max_width: f32,
    /// Minimum segment height.
    pub min_height: f32,
    /// Maximum segment height.
    pub max_height: f32,
    /// Probability of branching (0.0-1.0).
    pub branch_probability: f32,
    /// Probability of loop connections (0.0-1.0).
    pub loop_probability: f32,
    /// Whether to generate hazard annotations.
    pub enable_hazards: bool,
    /// Probability of hazard annotations (0.0-1.0).
    pub hazard_probability: f32,
    /// Whether to generate resource annotations.
    pub enable_resources: bool,
    /// Probability of resource annotations (0.0-1.0).
    pub resource_probability: f32,
    /// Whether to generate mission hooks.
    pub enable_mission_hooks: bool,
    /// Probability of mission hooks (0.0-1.0).
    pub mission_hook_probability: f32,
    /// Maximum depth from entry (0 = unlimited).
    pub max_depth: u32,
    /// Cell size for spatial sampling.
    pub cell_size: f32,
}

impl TopologyConfig {
    /// Create a new config with the given seed and kind.
    #[must_use]
    pub fn new(seed: u64, kind: TopologyKind) -> Self {
        let (min_w, max_w) = kind.default_width_range();
        let (min_h, max_h) = kind.default_height_range();

        Self {
            seed,
            kind,
            node_count: kind.default_node_count(),
            min_width: min_w,
            max_width: max_w,
            min_height: min_h,
            max_height: max_h,
            branch_probability: 0.3,
            loop_probability: 0.15,
            enable_hazards: true,
            hazard_probability: 0.1,
            enable_resources: true,
            resource_probability: 0.15,
            enable_mission_hooks: true,
            mission_hook_probability: 0.05,
            max_depth: 0,
            cell_size: 4.0,
        }
    }

    /// Create a small topology config.
    #[must_use]
    pub fn small(seed: u64, kind: TopologyKind) -> Self {
        Self::new(seed, kind).with_node_count(8)
    }

    /// Create a medium topology config.
    #[must_use]
    pub fn medium(seed: u64, kind: TopologyKind) -> Self {
        Self::new(seed, kind).with_node_count(20)
    }

    /// Create a large topology config.
    #[must_use]
    pub fn large(seed: u64, kind: TopologyKind) -> Self {
        Self::new(seed, kind).with_node_count(50)
    }

    /// Set node count.
    #[must_use]
    pub fn with_node_count(mut self, count: u32) -> Self {
        self.node_count = count;
        self
    }

    /// Set width range.
    #[must_use]
    pub fn with_width_range(mut self, min: f32, max: f32) -> Self {
        self.min_width = min.max(1.0);
        self.max_width = max.max(self.min_width);
        self
    }

    /// Set height range.
    #[must_use]
    pub fn with_height_range(mut self, min: f32, max: f32) -> Self {
        self.min_height = min.max(1.0);
        self.max_height = max.max(self.min_height);
        self
    }

    /// Set branch probability.
    #[must_use]
    pub fn with_branch_probability(mut self, prob: f32) -> Self {
        self.branch_probability = prob.clamp(0.0, 1.0);
        self
    }

    /// Set loop probability.
    #[must_use]
    pub fn with_loop_probability(mut self, prob: f32) -> Self {
        self.loop_probability = prob.clamp(0.0, 0.5);
        self
    }

    /// Enable or disable hazards.
    #[must_use]
    pub fn with_hazards(mut self, enabled: bool, probability: f32) -> Self {
        self.enable_hazards = enabled;
        self.hazard_probability = probability.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable resources.
    #[must_use]
    pub fn with_resources(mut self, enabled: bool, probability: f32) -> Self {
        self.enable_resources = enabled;
        self.resource_probability = probability.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable mission hooks.
    #[must_use]
    pub fn with_mission_hooks(mut self, enabled: bool, probability: f32) -> Self {
        self.enable_mission_hooks = enabled;
        self.mission_hook_probability = probability.clamp(0.0, 1.0);
        self
    }

    /// Set maximum depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set cell size for spatial sampling.
    #[must_use]
    pub fn with_cell_size(mut self, size: f32) -> Self {
        self.cell_size = size;
        self
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.node_count < 2 {
            return Err(ConfigError::TooFewNodes);
        }
        if self.min_width > self.max_width {
            return Err(ConfigError::InvalidWidthRange);
        }
        if self.min_height > self.max_height {
            return Err(ConfigError::InvalidHeightRange);
        }
        if self.cell_size < 0.5 {
            return Err(ConfigError::CellSizeTooSmall);
        }
        Ok(())
    }
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self::new(0, TopologyKind::default())
    }
}

/// Configuration validation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Too few nodes specified.
    TooFewNodes,
    /// Invalid width range (min > max).
    InvalidWidthRange,
    /// Invalid height range (min > max).
    InvalidHeightRange,
    /// Cell size too small.
    CellSizeTooSmall,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewNodes => write!(f, "node count must be at least 2"),
            Self::InvalidWidthRange => write!(f, "min_width must be <= max_width"),
            Self::InvalidHeightRange => write!(f, "min_height must be <= max_height"),
            Self::CellSizeTooSmall => write!(f, "cell_size must be at least 0.5"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = TopologyConfig::new(42, TopologyKind::Trench);
        assert_eq!(config.seed, 42);
        assert_eq!(config.kind, TopologyKind::Trench);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_presets() {
        let small = TopologyConfig::small(1, TopologyKind::IceTunnel);
        assert!(small.validate().is_ok());
        assert_eq!(small.node_count, 8);

        let medium = TopologyConfig::medium(1, TopologyKind::StationDeck);
        assert!(medium.validate().is_ok());
        assert_eq!(medium.node_count, 20);

        let large = TopologyConfig::large(1, TopologyKind::HollowSphere);
        assert!(large.validate().is_ok());
        assert_eq!(large.node_count, 50);
    }

    #[test]
    fn config_validation() {
        let bad_nodes = TopologyConfig::new(1, TopologyKind::Trench).with_node_count(1);
        assert_eq!(bad_nodes.validate(), Err(ConfigError::TooFewNodes));

        let mut bad_width = TopologyConfig::new(1, TopologyKind::Trench);
        bad_width.min_width = 10.0;
        bad_width.max_width = 5.0;
        assert_eq!(bad_width.validate(), Err(ConfigError::InvalidWidthRange));

        let mut bad_height = TopologyConfig::new(1, TopologyKind::Trench);
        bad_height.min_height = 10.0;
        bad_height.max_height = 5.0;
        assert_eq!(bad_height.validate(), Err(ConfigError::InvalidHeightRange));

        let bad_cell = TopologyConfig::new(1, TopologyKind::Trench).with_cell_size(0.1);
        assert_eq!(bad_cell.validate(), Err(ConfigError::CellSizeTooSmall));
    }

    #[test]
    fn serde_roundtrip() {
        let config = TopologyConfig::medium(12345, TopologyKind::IceTunnel)
            .with_hazards(true, 0.2)
            .with_resources(true, 0.3);

        let json = serde_json::to_string(&config).unwrap();
        let recovered: TopologyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn builder_methods_valid() {
        let config = TopologyConfig::new(1, TopologyKind::Trench)
            .with_node_count(30)
            .with_width_range(5.0, 15.0)
            .with_height_range(10.0, 40.0)
            .with_branch_probability(0.4)
            .with_loop_probability(0.2)
            .with_hazards(true, 0.15)
            .with_resources(true, 0.25)
            .with_mission_hooks(true, 0.1)
            .with_max_depth(10)
            .with_cell_size(2.0);

        assert!(config.validate().is_ok());
        assert_eq!(config.node_count, 30);
        assert!((config.min_width - 5.0).abs() < f32::EPSILON);
        assert!((config.max_width - 15.0).abs() < f32::EPSILON);
    }
}

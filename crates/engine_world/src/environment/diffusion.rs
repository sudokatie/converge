//! Diffusion and advection configuration for environmental fields.

use serde::{Deserialize, Serialize};

use super::FieldChannel;

/// Configuration for field diffusion behavior.
///
/// Diffusion causes field values to spread to neighboring cells over time,
/// moving toward equilibrium.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiffusionConfig {
    /// Diffusion rate (0.0 = no diffusion, 1.0 = instant equalization).
    /// Typical values: 0.01-0.2.
    pub rate: f32,

    /// Whether diffusion is enabled for this channel.
    pub enabled: bool,

    /// Coefficient for diagonal neighbors (typically lower than face neighbors).
    /// 0.0 = only face neighbors, 1.0 = equal weight.
    pub diagonal_factor: f32,

    /// Whether to clamp values after diffusion step.
    pub clamp_after: bool,
}

impl DiffusionConfig {
    /// No diffusion.
    pub const NONE: Self = Self {
        rate: 0.0,
        enabled: false,
        diagonal_factor: 0.0,
        clamp_after: false,
    };

    /// Slow diffusion (gases in still air).
    pub const SLOW: Self = Self {
        rate: 0.02,
        enabled: true,
        diagonal_factor: 0.5,
        clamp_after: true,
    };

    /// Medium diffusion (heat, humidity).
    pub const MEDIUM: Self = Self {
        rate: 0.1,
        enabled: true,
        diagonal_factor: 0.7,
        clamp_after: true,
    };

    /// Fast diffusion (explosions, rapid spread).
    pub const FAST: Self = Self {
        rate: 0.25,
        enabled: true,
        diagonal_factor: 0.7,
        clamp_after: true,
    };

    /// Create a custom diffusion configuration.
    #[must_use]
    pub const fn new(rate: f32, diagonal_factor: f32) -> Self {
        Self {
            rate,
            enabled: true,
            diagonal_factor,
            clamp_after: true,
        }
    }

    /// Check if diffusion would have any effect.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && self.rate > 0.0
    }
}

impl Default for DiffusionConfig {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// Configuration for field advection behavior.
///
/// Advection moves field values along a velocity field, simulating
/// transport by wind, water currents, etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvectionConfig {
    /// Base advection speed multiplier.
    pub speed: f32,

    /// Whether advection is enabled for this channel.
    pub enabled: bool,

    /// Dissipation per step (0.0 = no loss, 1.0 = instant decay).
    pub dissipation: f32,
}

impl AdvectionConfig {
    /// No advection.
    pub const NONE: Self = Self {
        speed: 0.0,
        enabled: false,
        dissipation: 0.0,
    };

    /// Light advection (gentle breeze).
    pub const LIGHT: Self = Self {
        speed: 0.5,
        enabled: true,
        dissipation: 0.01,
    };

    /// Strong advection (wind, water flow).
    pub const STRONG: Self = Self {
        speed: 1.0,
        enabled: true,
        dissipation: 0.02,
    };

    /// Create a custom advection configuration.
    #[must_use]
    pub const fn new(speed: f32, dissipation: f32) -> Self {
        Self {
            speed,
            enabled: true,
            dissipation,
        }
    }

    /// Check if advection would have any effect.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && self.speed > 0.0
    }
}

impl Default for AdvectionConfig {
    fn default() -> Self {
        Self::NONE
    }
}

/// Complete simulation parameters for a field channel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldSimConfig {
    /// The field channel this config applies to.
    pub channel: FieldChannel,

    /// Diffusion settings.
    pub diffusion: DiffusionConfig,

    /// Advection settings.
    pub advection: AdvectionConfig,

    /// Decay rate per second (for fields that naturally decay).
    /// 0.0 = no decay, 1.0 = full decay in 1 second.
    pub decay_rate: f32,

    /// Growth rate per second (for fields that naturally grow).
    pub growth_rate: f32,

    /// Target value for decay/growth (fields decay toward this value).
    pub equilibrium: f32,
}

impl FieldSimConfig {
    /// Create a new simulation config for a channel.
    #[must_use]
    pub fn new(channel: FieldChannel) -> Self {
        let (diffusion, advection, decay_rate, growth_rate) = match channel {
            FieldChannel::Temperature => {
                (DiffusionConfig::MEDIUM, AdvectionConfig::LIGHT, 0.0, 0.0)
            }
            FieldChannel::Oxygen => (DiffusionConfig::SLOW, AdvectionConfig::LIGHT, 0.0, 0.0),
            FieldChannel::Pressure => (DiffusionConfig::FAST, AdvectionConfig::NONE, 0.0, 0.0),
            FieldChannel::Radiation => (DiffusionConfig::NONE, AdvectionConfig::NONE, 0.1, 0.0),
            FieldChannel::Toxicity => (DiffusionConfig::SLOW, AdvectionConfig::LIGHT, 0.05, 0.0),
            FieldChannel::Humidity => (DiffusionConfig::MEDIUM, AdvectionConfig::LIGHT, 0.0, 0.0),
            FieldChannel::Corruption => (DiffusionConfig::SLOW, AdvectionConfig::NONE, 0.0, 0.01),
            FieldChannel::SporeDensity => {
                (DiffusionConfig::MEDIUM, AdvectionConfig::STRONG, 0.1, 0.0)
            }
        };

        Self {
            channel,
            diffusion,
            advection,
            decay_rate,
            growth_rate,
            equilibrium: channel.default_value(),
        }
    }

    /// Create configs for all channels with default settings.
    ///
    /// # Panics
    ///
    /// Panics if internal channel indexing is inconsistent (should never happen).
    #[must_use]
    pub fn all_defaults() -> [Self; FieldChannel::COUNT] {
        std::array::from_fn(|i| {
            let channel = FieldChannel::from_index(i).expect("valid index");
            Self::new(channel)
        })
    }

    /// Check if any simulation is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.diffusion.is_active()
            || self.advection.is_active()
            || self.decay_rate > 0.0
            || self.growth_rate > 0.0
    }
}

/// Parameters for a single diffusion step.
#[derive(Clone, Debug)]
pub struct DiffusionStep {
    /// Delta time for this step.
    pub dt: f32,

    /// The diffusion configuration to use.
    pub config: DiffusionConfig,
}

impl DiffusionStep {
    /// Create a new diffusion step.
    #[must_use]
    pub const fn new(dt: f32, config: DiffusionConfig) -> Self {
        Self { dt, config }
    }

    /// Calculate the effective diffusion rate for this step.
    #[must_use]
    pub fn effective_rate(&self) -> f32 {
        (self.config.rate * self.dt).min(1.0)
    }
}

/// Result of applying a simulation step.
#[derive(Clone, Debug, Default)]
pub struct SimStepResult {
    /// Number of cells that changed.
    pub cells_changed: u32,

    /// Total amount of value transferred.
    pub total_transfer: f32,

    /// Maximum change in any single cell.
    pub max_change: f32,
}

impl SimStepResult {
    /// Merge another result into this one.
    pub fn merge(&mut self, other: &Self) {
        self.cells_changed += other.cells_changed;
        self.total_transfer += other.total_transfer;
        self.max_change = self.max_change.max(other.max_change);
    }

    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.cells_changed > 0
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "tests check exact constructor return values"
)]
mod tests {
    use super::*;

    #[test]
    fn test_diffusion_config_none() {
        let config = DiffusionConfig::NONE;
        assert!(!config.is_active());
        assert_eq!(config.rate, 0.0);
    }

    #[test]
    fn test_diffusion_config_active() {
        let config = DiffusionConfig::MEDIUM;
        assert!(config.is_active());
        assert!(config.rate > 0.0);
    }

    #[test]
    fn test_diffusion_step_effective_rate() {
        let config = DiffusionConfig::new(0.5, 0.7);
        let step = DiffusionStep::new(0.1, config);
        assert!((step.effective_rate() - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_diffusion_step_clamped() {
        let config = DiffusionConfig::new(10.0, 0.7);
        let step = DiffusionStep::new(1.0, config);
        assert_eq!(step.effective_rate(), 1.0); // Clamped
    }

    #[test]
    fn test_advection_config_none() {
        let config = AdvectionConfig::NONE;
        assert!(!config.is_active());
    }

    #[test]
    fn test_advection_config_active() {
        let config = AdvectionConfig::STRONG;
        assert!(config.is_active());
    }

    #[test]
    fn test_field_sim_config_defaults() {
        let configs = FieldSimConfig::all_defaults();
        assert_eq!(configs.len(), FieldChannel::COUNT);

        // Temperature should diffuse
        let temp_config = &configs[FieldChannel::Temperature.as_index()];
        assert!(temp_config.diffusion.is_active());

        // Radiation should decay
        let rad_config = &configs[FieldChannel::Radiation.as_index()];
        assert!(rad_config.decay_rate > 0.0);
    }

    #[test]
    fn test_field_sim_config_is_active() {
        let config = FieldSimConfig::new(FieldChannel::Temperature);
        assert!(config.is_active());

        // Make an inactive config
        let mut inactive = FieldSimConfig::new(FieldChannel::Temperature);
        inactive.diffusion = DiffusionConfig::NONE;
        inactive.advection = AdvectionConfig::NONE;
        inactive.decay_rate = 0.0;
        inactive.growth_rate = 0.0;
        assert!(!inactive.is_active());
    }

    #[test]
    fn test_sim_step_result_merge() {
        let mut a = SimStepResult {
            cells_changed: 10,
            total_transfer: 5.0,
            max_change: 0.5,
        };
        let b = SimStepResult {
            cells_changed: 5,
            total_transfer: 3.0,
            max_change: 0.8,
        };
        a.merge(&b);
        assert_eq!(a.cells_changed, 15);
        assert!((a.total_transfer - 8.0).abs() < 0.001);
        assert!((a.max_change - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_sim_step_result_has_changes() {
        let no_changes = SimStepResult::default();
        assert!(!no_changes.has_changes());

        let has_changes = SimStepResult {
            cells_changed: 1,
            ..Default::default()
        };
        assert!(has_changes.has_changes());
    }
}

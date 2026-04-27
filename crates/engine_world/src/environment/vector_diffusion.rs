//! Simulation configuration for vector environmental fields.

use serde::{Deserialize, Serialize};

use super::VectorFieldChannel;

/// Configuration for vector field advection/transport.
///
/// Controls how vector fields propagate through space.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorAdvectionConfig {
    /// Advection speed multiplier.
    pub speed: f32,

    /// Whether advection is enabled.
    pub enabled: bool,

    /// Magnitude dissipation per step (0.0 = no loss, 1.0 = instant decay).
    pub dissipation: f32,
}

impl VectorAdvectionConfig {
    /// No advection.
    pub const NONE: Self = Self {
        speed: 0.0,
        enabled: false,
        dissipation: 0.0,
    };

    /// Slow advection for static-ish fields.
    pub const SLOW: Self = Self {
        speed: 0.25,
        enabled: true,
        dissipation: 0.01,
    };

    /// Medium advection for flowing fields.
    pub const MEDIUM: Self = Self {
        speed: 0.5,
        enabled: true,
        dissipation: 0.02,
    };

    /// Fast advection for turbulent fields.
    pub const FAST: Self = Self {
        speed: 1.0,
        enabled: true,
        dissipation: 0.05,
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

impl Default for VectorAdvectionConfig {
    fn default() -> Self {
        Self::NONE
    }
}

/// Configuration for vector field decay.
///
/// Controls how vector magnitudes decay toward equilibrium over time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorDecayConfig {
    /// Decay rate per second (0.0 = no decay, 1.0 = instant).
    pub rate: f32,

    /// Whether decay is enabled.
    pub enabled: bool,

    /// Whether to preserve direction during decay (only magnitude decreases).
    pub preserve_direction: bool,
}

impl VectorDecayConfig {
    /// No decay.
    pub const NONE: Self = Self {
        rate: 0.0,
        enabled: false,
        preserve_direction: true,
    };

    /// Slow decay for persistent effects.
    pub const SLOW: Self = Self {
        rate: 0.05,
        enabled: true,
        preserve_direction: true,
    };

    /// Medium decay.
    pub const MEDIUM: Self = Self {
        rate: 0.15,
        enabled: true,
        preserve_direction: true,
    };

    /// Fast decay for transient effects.
    pub const FAST: Self = Self {
        rate: 0.4,
        enabled: true,
        preserve_direction: true,
    };

    /// Create a custom decay configuration.
    #[must_use]
    pub const fn new(rate: f32, preserve_direction: bool) -> Self {
        Self {
            rate,
            enabled: true,
            preserve_direction,
        }
    }

    /// Check if decay would have any effect.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && self.rate > 0.0
    }
}

impl Default for VectorDecayConfig {
    fn default() -> Self {
        Self::NONE
    }
}

/// Configuration for vector field smoothing.
///
/// Controls local averaging/blending between neighboring cells.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorSmoothingConfig {
    /// Smoothing strength (0.0 = none, 1.0 = full neighbor average).
    pub strength: f32,

    /// Whether smoothing is enabled.
    pub enabled: bool,

    /// Whether to normalize result vectors after smoothing.
    pub normalize_after: bool,
}

impl VectorSmoothingConfig {
    /// No smoothing.
    pub const NONE: Self = Self {
        strength: 0.0,
        enabled: false,
        normalize_after: false,
    };

    /// Light smoothing for subtle blending.
    pub const LIGHT: Self = Self {
        strength: 0.1,
        enabled: true,
        normalize_after: false,
    };

    /// Medium smoothing.
    pub const MEDIUM: Self = Self {
        strength: 0.25,
        enabled: true,
        normalize_after: false,
    };

    /// Strong smoothing for coherent flow fields.
    pub const STRONG: Self = Self {
        strength: 0.5,
        enabled: true,
        normalize_after: false,
    };

    /// Create a custom smoothing configuration.
    #[must_use]
    pub const fn new(strength: f32, normalize_after: bool) -> Self {
        Self {
            strength,
            enabled: true,
            normalize_after,
        }
    }

    /// Check if smoothing would have any effect.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && self.strength > 0.0
    }
}

impl Default for VectorSmoothingConfig {
    fn default() -> Self {
        Self::NONE
    }
}

/// Complete simulation parameters for a vector field channel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorFieldSimConfig {
    /// The vector field channel this config applies to.
    pub channel: VectorFieldChannel,

    /// Advection settings.
    pub advection: VectorAdvectionConfig,

    /// Decay settings.
    pub decay: VectorDecayConfig,

    /// Smoothing settings.
    pub smoothing: VectorSmoothingConfig,

    /// Whether to clamp to max magnitude after simulation step.
    pub clamp_magnitude: bool,
}

impl VectorFieldSimConfig {
    /// Create a new simulation config for a channel with sensible defaults.
    #[must_use]
    pub fn new(channel: VectorFieldChannel) -> Self {
        let (advection, decay, smoothing) = match channel {
            VectorFieldChannel::Wind => (
                VectorAdvectionConfig::MEDIUM,
                VectorDecayConfig::SLOW,
                VectorSmoothingConfig::LIGHT,
            ),
            VectorFieldChannel::WaterCurrent => (
                VectorAdvectionConfig::SLOW,
                VectorDecayConfig::SLOW,
                VectorSmoothingConfig::MEDIUM,
            ),
            VectorFieldChannel::PressureGradient => (
                VectorAdvectionConfig::NONE,
                VectorDecayConfig::MEDIUM,
                VectorSmoothingConfig::STRONG,
            ),
            VectorFieldChannel::GravityOverride => (
                VectorAdvectionConfig::NONE,
                VectorDecayConfig::NONE,
                VectorSmoothingConfig::NONE,
            ),
            VectorFieldChannel::HazardSpread => (
                VectorAdvectionConfig::SLOW,
                VectorDecayConfig::MEDIUM,
                VectorSmoothingConfig::LIGHT,
            ),
        };

        Self {
            channel,
            advection,
            decay,
            smoothing,
            clamp_magnitude: channel.max_magnitude().is_some(),
        }
    }

    /// Create configs for all channels with default settings.
    #[must_use]
    pub fn all_defaults() -> [Self; VectorFieldChannel::COUNT] {
        std::array::from_fn(|i| {
            let channel = VectorFieldChannel::from_index(i).expect("valid index");
            Self::new(channel)
        })
    }

    /// Check if any simulation is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.advection.is_active() || self.decay.is_active() || self.smoothing.is_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_advection_config_none() {
        let config = VectorAdvectionConfig::NONE;
        assert!(!config.is_active());
        assert_eq!(config.speed, 0.0);
    }

    #[test]
    fn test_vector_advection_config_active() {
        let config = VectorAdvectionConfig::MEDIUM;
        assert!(config.is_active());
        assert!(config.speed > 0.0);
    }

    #[test]
    fn test_vector_decay_config_none() {
        let config = VectorDecayConfig::NONE;
        assert!(!config.is_active());
    }

    #[test]
    fn test_vector_decay_config_active() {
        let config = VectorDecayConfig::FAST;
        assert!(config.is_active());
        assert!(config.rate > 0.0);
    }

    #[test]
    fn test_vector_smoothing_config_none() {
        let config = VectorSmoothingConfig::NONE;
        assert!(!config.is_active());
    }

    #[test]
    fn test_vector_smoothing_config_active() {
        let config = VectorSmoothingConfig::STRONG;
        assert!(config.is_active());
        assert!(config.strength > 0.0);
    }

    #[test]
    fn test_vector_field_sim_config_defaults() {
        let configs = VectorFieldSimConfig::all_defaults();
        assert_eq!(configs.len(), VectorFieldChannel::COUNT);

        let wind_config = &configs[VectorFieldChannel::Wind.as_index()];
        assert!(wind_config.advection.is_active());
        assert!(wind_config.decay.is_active());

        let gravity_config = &configs[VectorFieldChannel::GravityOverride.as_index()];
        assert!(!gravity_config.is_active());
    }

    #[test]
    fn test_vector_field_sim_config_is_active() {
        let config = VectorFieldSimConfig::new(VectorFieldChannel::Wind);
        assert!(config.is_active());

        let mut inactive = VectorFieldSimConfig::new(VectorFieldChannel::Wind);
        inactive.advection = VectorAdvectionConfig::NONE;
        inactive.decay = VectorDecayConfig::NONE;
        inactive.smoothing = VectorSmoothingConfig::NONE;
        assert!(!inactive.is_active());
    }

    #[test]
    fn test_gravity_override_static() {
        let config = VectorFieldSimConfig::new(VectorFieldChannel::GravityOverride);
        assert!(!config.advection.is_active());
        assert!(!config.decay.is_active());
        assert!(!config.smoothing.is_active());
    }
}

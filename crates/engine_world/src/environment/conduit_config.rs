//! Configuration for conduit network simulation.

use serde::{Deserialize, Serialize};

use super::ConduitKind;

/// Configuration for flow distribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowConfig {
    /// Base transfer rate multiplier.
    pub rate_multiplier: f32,
    /// Loss factor per segment (0.0 to 1.0).
    pub loss_per_segment: f32,
    /// Whether to use priority-based distribution.
    pub use_priority: bool,
    /// Minimum transfer amount to process.
    pub min_transfer: f32,
}

impl FlowConfig {
    /// Standard flow configuration.
    pub const STANDARD: Self = Self {
        rate_multiplier: 1.0,
        loss_per_segment: 0.01,
        use_priority: true,
        min_transfer: 0.001,
    };

    /// Fast flow with minimal loss.
    pub const FAST: Self = Self {
        rate_multiplier: 2.0,
        loss_per_segment: 0.005,
        use_priority: true,
        min_transfer: 0.0001,
    };

    /// Slow flow with higher loss (degraded network).
    pub const DEGRADED: Self = Self {
        rate_multiplier: 0.5,
        loss_per_segment: 0.05,
        use_priority: true,
        min_transfer: 0.01,
    };

    /// Check if configuration is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.rate_multiplier > 0.0
            && self.loss_per_segment >= 0.0
            && self.loss_per_segment <= 1.0
            && self.min_transfer >= 0.0
    }
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Configuration for heat transfer behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeatTransferConfig {
    /// Conductivity multiplier.
    pub conductivity: f32,
    /// Ambient temperature for heat loss.
    pub ambient_temperature: f32,
    /// Heat loss rate to environment (per second).
    pub ambient_loss_rate: f32,
    /// Minimum temperature difference for transfer.
    pub min_delta: f32,
}

impl HeatTransferConfig {
    /// Standard heat transfer.
    pub const STANDARD: Self = Self {
        conductivity: 1.0,
        ambient_temperature: 20.0,
        ambient_loss_rate: 0.01,
        min_delta: 0.1,
    };

    /// Insulated (minimal ambient loss).
    pub const INSULATED: Self = Self {
        conductivity: 1.0,
        ambient_temperature: 20.0,
        ambient_loss_rate: 0.001,
        min_delta: 0.1,
    };

    /// Exposed (high ambient loss).
    pub const EXPOSED: Self = Self {
        conductivity: 1.0,
        ambient_temperature: 20.0,
        ambient_loss_rate: 0.1,
        min_delta: 0.1,
    };
}

impl Default for HeatTransferConfig {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Configuration for fluid pressure behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureConfig {
    /// Pressure equalization rate (0.0 to 1.0).
    pub equalization_rate: f32,
    /// Pressure loss per segment.
    pub friction_loss: f32,
    /// Gravity effect multiplier.
    pub gravity_factor: f32,
}

impl PressureConfig {
    /// Standard pressure behavior.
    pub const STANDARD: Self = Self {
        equalization_rate: 0.5,
        friction_loss: 0.02,
        gravity_factor: 1.0,
    };

    /// High-pressure system.
    pub const HIGH_PRESSURE: Self = Self {
        equalization_rate: 0.8,
        friction_loss: 0.01,
        gravity_factor: 0.5,
    };

    /// Low-pressure/gravity-fed system.
    pub const GRAVITY_FED: Self = Self {
        equalization_rate: 0.2,
        friction_loss: 0.05,
        gravity_factor: 2.0,
    };
}

impl Default for PressureConfig {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Complete configuration for conduit network simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConduitNetworkConfig {
    /// Flow distribution settings.
    pub flow: FlowConfig,
    /// Heat transfer settings (for Heat conduits).
    pub heat: HeatTransferConfig,
    /// Pressure settings (for Fluid conduits).
    pub pressure: PressureConfig,
    /// Whether simulation is enabled.
    pub enabled: bool,
    /// Maximum iterations for network solver.
    pub max_iterations: u32,
}

impl ConduitNetworkConfig {
    /// Standard configuration.
    pub const STANDARD: Self = Self {
        flow: FlowConfig::STANDARD,
        heat: HeatTransferConfig::STANDARD,
        pressure: PressureConfig::STANDARD,
        enabled: true,
        max_iterations: 10,
    };

    /// Create default configuration for a specific conduit kind.
    #[must_use]
    pub fn for_kind(kind: ConduitKind) -> Self {
        match kind {
            ConduitKind::Power => Self {
                flow: FlowConfig::FAST,
                heat: HeatTransferConfig::STANDARD,
                pressure: PressureConfig::STANDARD,
                enabled: true,
                max_iterations: 5,
            },
            ConduitKind::Heat => Self {
                flow: FlowConfig::STANDARD,
                heat: HeatTransferConfig::STANDARD,
                pressure: PressureConfig::STANDARD,
                enabled: true,
                max_iterations: 10,
            },
            ConduitKind::Fluid => Self {
                flow: FlowConfig::STANDARD,
                heat: HeatTransferConfig::STANDARD,
                pressure: PressureConfig::STANDARD,
                enabled: true,
                max_iterations: 15,
            },
            ConduitKind::Signal => Self {
                flow: FlowConfig::FAST,
                heat: HeatTransferConfig::STANDARD,
                pressure: PressureConfig::STANDARD,
                enabled: true,
                max_iterations: 3,
            },
        }
    }

    /// Check if configuration is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.flow.is_valid() && self.max_iterations > 0
    }
}

impl Default for ConduitNetworkConfig {
    fn default() -> Self {
        Self::STANDARD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_config_valid() {
        assert!(FlowConfig::STANDARD.is_valid());
        assert!(FlowConfig::FAST.is_valid());
        assert!(FlowConfig::DEGRADED.is_valid());
    }

    #[test]
    fn flow_config_invalid() {
        let invalid = FlowConfig {
            rate_multiplier: -1.0,
            ..FlowConfig::STANDARD
        };
        assert!(!invalid.is_valid());

        let invalid = FlowConfig {
            loss_per_segment: 1.5,
            ..FlowConfig::STANDARD
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn network_config_for_kind() {
        for kind in ConduitKind::ALL {
            let config = ConduitNetworkConfig::for_kind(kind);
            assert!(config.is_valid());
            assert!(config.enabled);
        }
    }

    #[test]
    fn network_config_valid() {
        assert!(ConduitNetworkConfig::STANDARD.is_valid());
    }

    #[test]
    fn serde_flow_config() {
        let config = FlowConfig::FAST;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: FlowConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_heat_config() {
        let config = HeatTransferConfig::INSULATED;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: HeatTransferConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_pressure_config() {
        let config = PressureConfig::HIGH_PRESSURE;
        let json = serde_json::to_string(&config).unwrap();
        let recovered: PressureConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn serde_network_config() {
        let config = ConduitNetworkConfig::for_kind(ConduitKind::Fluid);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: ConduitNetworkConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);
    }
}

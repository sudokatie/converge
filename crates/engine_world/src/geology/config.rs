//! Configuration types for geological simulation.

use serde::{Deserialize, Serialize};

/// Configuration for thermal simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThermalConfig {
    /// Base ambient temperature at surface (Celsius).
    pub surface_temperature: f32,
    /// Temperature gradient per unit depth (C/unit).
    pub geothermal_gradient: f32,
    /// Thermal diffusivity coefficient.
    pub diffusivity: f32,
    /// Heat loss rate to surface per tick.
    pub surface_cooling_rate: f32,
    /// Magma temperature threshold (above this is molten).
    pub magma_threshold: f32,
}

impl Default for ThermalConfig {
    fn default() -> Self {
        Self {
            surface_temperature: 15.0,
            geothermal_gradient: 0.03,
            diffusivity: 0.01,
            surface_cooling_rate: 0.001,
            magma_threshold: 700.0,
        }
    }
}

impl ThermalConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_surface_temperature(mut self, temp: f32) -> Self {
        self.surface_temperature = temp.clamp(-50.0, 100.0);
        self
    }

    #[must_use]
    pub fn with_geothermal_gradient(mut self, gradient: f32) -> Self {
        self.geothermal_gradient = gradient.clamp(0.001, 0.1);
        self
    }

    #[must_use]
    pub fn with_diffusivity(mut self, diff: f32) -> Self {
        self.diffusivity = diff.clamp(0.001, 0.1);
        self
    }

    #[must_use]
    pub fn temperature_at_depth(&self, depth: f32) -> f32 {
        self.surface_temperature + self.geothermal_gradient * depth
    }
}

/// Configuration for magma simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MagmaConfig {
    /// Base magma temperature (Celsius).
    pub base_temperature: f32,
    /// Pressure buildup rate per tick when blocked.
    pub pressure_buildup_rate: f32,
    /// Pressure release threshold for eruption.
    pub eruption_threshold: f32,
    /// Flow rate coefficient for magma movement.
    pub flow_rate: f32,
    /// Cooling rate when exposed to cooler material.
    pub cooling_rate: f32,
    /// Viscosity affects flow speed (higher = slower).
    pub viscosity: f32,
}

impl Default for MagmaConfig {
    fn default() -> Self {
        Self {
            base_temperature: 1200.0,
            pressure_buildup_rate: 0.05,
            eruption_threshold: 100.0,
            flow_rate: 0.1,
            cooling_rate: 0.02,
            viscosity: 0.5,
        }
    }
}

impl MagmaConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_base_temperature(mut self, temp: f32) -> Self {
        self.base_temperature = temp.clamp(700.0, 2000.0);
        self
    }

    #[must_use]
    pub fn with_pressure_buildup_rate(mut self, rate: f32) -> Self {
        self.pressure_buildup_rate = rate.clamp(0.001, 1.0);
        self
    }

    #[must_use]
    pub fn with_eruption_threshold(mut self, threshold: f32) -> Self {
        self.eruption_threshold = threshold.clamp(10.0, 1000.0);
        self
    }

    #[must_use]
    pub fn with_flow_rate(mut self, rate: f32) -> Self {
        self.flow_rate = rate.clamp(0.01, 1.0);
        self
    }

    #[must_use]
    pub fn with_viscosity(mut self, visc: f32) -> Self {
        self.viscosity = visc.clamp(0.1, 2.0);
        self
    }

    #[must_use]
    pub fn effective_flow_rate(&self) -> f32 {
        self.flow_rate / self.viscosity
    }
}

/// Configuration for fault line simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaultConfig {
    /// Stress accumulation rate per tick.
    pub stress_rate: f32,
    /// Threshold for minor slip events.
    pub minor_slip_threshold: f32,
    /// Threshold for major slip (earthquake).
    pub major_slip_threshold: f32,
    /// Stress release factor on slip (0-1).
    pub slip_release_factor: f32,
    /// Decay rate for accumulated stress.
    pub stress_decay_rate: f32,
    /// Probability multiplier for aftershocks.
    pub aftershock_probability: f32,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            stress_rate: 0.01,
            minor_slip_threshold: 50.0,
            major_slip_threshold: 100.0,
            slip_release_factor: 0.8,
            stress_decay_rate: 0.001,
            aftershock_probability: 0.3,
        }
    }
}

impl FaultConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_stress_rate(mut self, rate: f32) -> Self {
        self.stress_rate = rate.clamp(0.001, 1.0);
        self
    }

    #[must_use]
    pub fn with_minor_slip_threshold(mut self, threshold: f32) -> Self {
        self.minor_slip_threshold = threshold.clamp(10.0, 500.0);
        self
    }

    #[must_use]
    pub fn with_major_slip_threshold(mut self, threshold: f32) -> Self {
        self.major_slip_threshold = threshold.clamp(self.minor_slip_threshold, 1000.0);
        self
    }

    #[must_use]
    pub fn with_slip_release_factor(mut self, factor: f32) -> Self {
        self.slip_release_factor = factor.clamp(0.1, 1.0);
        self
    }

    #[must_use]
    pub fn seismic_moment(&self, stress: f32, slip_amount: f32) -> f32 {
        stress * slip_amount * 1e6
    }

    #[must_use]
    pub fn magnitude_from_moment(&self, moment: f32) -> f32 {
        if moment <= 0.0 {
            return 0.0;
        }
        (moment.log10() - 9.1) / 1.5
    }
}

/// Configuration for crystal growth simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrystalGrowthConfig {
    /// Base growth rate per tick.
    pub base_growth_rate: f32,
    /// Optimal temperature for crystal growth.
    pub optimal_temperature: f32,
    /// Temperature tolerance range.
    pub temperature_tolerance: f32,
    /// Optimal pressure for crystal growth.
    pub optimal_pressure: f32,
    /// Pressure tolerance range.
    pub pressure_tolerance: f32,
    /// Decay rate when conditions are poor.
    pub degradation_rate: f32,
}

impl Default for CrystalGrowthConfig {
    fn default() -> Self {
        Self {
            base_growth_rate: 0.001,
            optimal_temperature: 300.0,
            temperature_tolerance: 100.0,
            optimal_pressure: 50.0,
            pressure_tolerance: 30.0,
            degradation_rate: 0.0001,
        }
    }
}

impl CrystalGrowthConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_base_growth_rate(mut self, rate: f32) -> Self {
        self.base_growth_rate = rate.clamp(0.0001, 0.1);
        self
    }

    #[must_use]
    pub fn with_optimal_temperature(mut self, temp: f32) -> Self {
        self.optimal_temperature = temp.clamp(50.0, 1000.0);
        self
    }

    #[must_use]
    pub fn with_optimal_pressure(mut self, pressure: f32) -> Self {
        self.optimal_pressure = pressure.clamp(1.0, 200.0);
        self
    }

    #[must_use]
    pub fn growth_factor(&self, temperature: f32, pressure: f32) -> f32 {
        let temp_diff = (temperature - self.optimal_temperature).abs();
        let temp_factor = (1.0 - temp_diff / self.temperature_tolerance).max(0.0);

        let pressure_diff = (pressure - self.optimal_pressure).abs();
        let pressure_factor = (1.0 - pressure_diff / self.pressure_tolerance).max(0.0);

        temp_factor * pressure_factor
    }
}

/// Master configuration for geological simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeologyConfig {
    /// Thermal simulation configuration.
    pub thermal: ThermalConfig,
    /// Magma simulation configuration.
    pub magma: MagmaConfig,
    /// Fault simulation configuration.
    pub fault: FaultConfig,
    /// Crystal growth configuration.
    pub crystal: CrystalGrowthConfig,
    /// Simulation tick interval.
    pub tick_interval: u64,
    /// Maximum depth for simulation.
    pub max_depth: f32,
    /// Pressure multiplier per unit depth.
    pub depth_pressure_coefficient: f32,
}

impl Default for GeologyConfig {
    fn default() -> Self {
        Self {
            thermal: ThermalConfig::default(),
            magma: MagmaConfig::default(),
            fault: FaultConfig::default(),
            crystal: CrystalGrowthConfig::default(),
            tick_interval: 10,
            max_depth: 1000.0,
            depth_pressure_coefficient: 0.1,
        }
    }
}

impl GeologyConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_thermal(mut self, config: ThermalConfig) -> Self {
        self.thermal = config;
        self
    }

    #[must_use]
    pub fn with_magma(mut self, config: MagmaConfig) -> Self {
        self.magma = config;
        self
    }

    #[must_use]
    pub fn with_fault(mut self, config: FaultConfig) -> Self {
        self.fault = config;
        self
    }

    #[must_use]
    pub fn with_crystal(mut self, config: CrystalGrowthConfig) -> Self {
        self.crystal = config;
        self
    }

    #[must_use]
    pub fn with_tick_interval(mut self, interval: u64) -> Self {
        self.tick_interval = interval.max(1);
        self
    }

    #[must_use]
    pub fn with_max_depth(mut self, depth: f32) -> Self {
        self.max_depth = depth.clamp(100.0, 10000.0);
        self
    }

    #[must_use]
    pub fn pressure_at_depth(&self, depth: f32) -> f32 {
        depth.max(0.0) * self.depth_pressure_coefficient
    }

    /// Validates the geology configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration value is invalid:
    /// - `ConfigError::Depth` if max depth is non-positive
    /// - `ConfigError::PressureCoefficient` if pressure coefficient is non-positive
    /// - `ConfigError::FaultThresholds` if major slip threshold is less than minor
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_depth <= 0.0 {
            return Err(ConfigError::Depth);
        }
        if self.depth_pressure_coefficient <= 0.0 {
            return Err(ConfigError::PressureCoefficient);
        }
        if self.fault.major_slip_threshold < self.fault.minor_slip_threshold {
            return Err(ConfigError::FaultThresholds);
        }
        Ok(())
    }
}

/// Configuration validation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Max depth is non-positive.
    Depth,
    /// Pressure coefficient is non-positive.
    PressureCoefficient,
    /// Major slip threshold is less than minor.
    FaultThresholds,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Depth => write!(f, "invalid max depth"),
            Self::PressureCoefficient => write!(f, "invalid pressure coefficient"),
            Self::FaultThresholds => write!(f, "major slip threshold must be >= minor"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_config_defaults() {
        let config = ThermalConfig::new();
        assert!((config.surface_temperature - 15.0).abs() < f32::EPSILON);
        assert!(config.geothermal_gradient > 0.0);
    }

    #[test]
    fn thermal_temperature_at_depth() {
        let config = ThermalConfig::new().with_geothermal_gradient(0.03);
        let temp = config.temperature_at_depth(100.0);
        assert!((temp - 18.0).abs() < 0.01);
    }

    #[test]
    fn magma_config_effective_flow() {
        let config = MagmaConfig::new().with_flow_rate(0.2).with_viscosity(0.5);
        assert!((config.effective_flow_rate() - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn fault_config_magnitude() {
        let config = FaultConfig::new();
        let moment = config.seismic_moment(1000.0, 10.0);
        assert!(moment > 0.0);
        let mag = config.magnitude_from_moment(moment);
        assert!(mag > 0.0);
    }

    #[test]
    fn crystal_growth_factor() {
        let config = CrystalGrowthConfig::new()
            .with_optimal_temperature(300.0)
            .with_optimal_pressure(50.0);

        let optimal = config.growth_factor(300.0, 50.0);
        assert!((optimal - 1.0).abs() < f32::EPSILON);

        let suboptimal = config.growth_factor(400.0, 80.0);
        assert!(suboptimal < optimal);
        assert!(suboptimal >= 0.0);
    }

    #[test]
    fn geology_config_pressure_at_depth() {
        let config = GeologyConfig::new();
        let p0 = config.pressure_at_depth(0.0);
        let p100 = config.pressure_at_depth(100.0);
        assert!((p0 - 0.0).abs() < f32::EPSILON);
        assert!(p100 > p0);
    }

    #[test]
    fn geology_config_validation() {
        let config = GeologyConfig::new();
        assert!(config.validate().is_ok());

        let mut bad = GeologyConfig::new();
        bad.max_depth = -10.0;
        assert!(matches!(bad.validate(), Err(ConfigError::Depth)));

        let mut bad_fault = GeologyConfig::new();
        bad_fault.fault.major_slip_threshold = 10.0;
        bad_fault.fault.minor_slip_threshold = 100.0;
        assert!(matches!(
            bad_fault.validate(),
            Err(ConfigError::FaultThresholds)
        ));
    }

    #[test]
    fn serde_thermal_config() {
        let config = ThermalConfig::new().with_surface_temperature(25.0);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: ThermalConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_magma_config() {
        let config = MagmaConfig::new().with_base_temperature(1000.0);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: MagmaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_fault_config() {
        let config = FaultConfig::new().with_stress_rate(0.05);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: FaultConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_crystal_config() {
        let config = CrystalGrowthConfig::new().with_base_growth_rate(0.002);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: CrystalGrowthConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_geology_config() {
        let config = GeologyConfig::new()
            .with_max_depth(500.0)
            .with_tick_interval(5);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: GeologyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }
}

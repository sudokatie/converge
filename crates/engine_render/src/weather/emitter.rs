//! Particle emitter configuration.
//!
//! Defines how particles are spawned and configured.

use super::curve::{ColorOverTime, OverTimeCurve};
use super::shape::SpawnShape;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Simulation space for particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum SimulationSpace {
    /// Particles move in local emitter space.
    Local = 0,
    /// Particles move in world space.
    #[default]
    World = 1,
}

impl SimulationSpace {
    /// All simulation spaces.
    pub const ALL: [Self; 2] = [Self::Local, Self::World];
}

/// Emission mode for the emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum EmissionMode {
    /// Continuous emission at a steady rate.
    #[default]
    Continuous = 0,
    /// Emit all particles in a burst.
    Burst = 1,
    /// Emit in repeating bursts.
    BurstRepeat = 2,
}

impl EmissionMode {
    /// All emission modes.
    pub const ALL: [Self; 3] = [Self::Continuous, Self::BurstRepeat, Self::Burst];
}

/// Velocity mode for initial particle velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum VelocityMode {
    /// Fixed direction.
    #[default]
    Directional = 0,
    /// Radial from spawn point.
    Radial = 1,
    /// Random within bounds.
    Random = 2,
    /// Tangential (perpendicular to radial).
    Tangential = 3,
}

impl VelocityMode {
    /// All velocity modes.
    pub const ALL: [Self; 4] = [
        Self::Directional,
        Self::Radial,
        Self::Random,
        Self::Tangential,
    ];
}

/// Range of values for randomization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValueRange {
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
}

impl Default for ValueRange {
    fn default() -> Self {
        Self { min: 1.0, max: 1.0 }
    }
}

impl ValueRange {
    /// Create a constant (non-random) range.
    #[must_use]
    pub fn constant(value: f32) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    /// Create a range between min and max.
    #[must_use]
    pub fn range(min: f32, max: f32) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// Sample a value from this range deterministically using a normalized factor (0-1).
    #[must_use]
    pub fn sample(&self, factor: f32) -> f32 {
        let factor = factor.clamp(0.0, 1.0);
        self.min + (self.max - self.min) * factor
    }

    /// Get the midpoint of the range.
    #[must_use]
    pub fn midpoint(&self) -> f32 {
        f32::midpoint(self.min, self.max)
    }

    /// Get the span of the range.
    #[must_use]
    pub fn span(&self) -> f32 {
        self.max - self.min
    }

    /// Check if this is a constant (no variation).
    #[must_use]
    pub fn is_constant(&self) -> bool {
        (self.max - self.min).abs() < 0.0001
    }
}

/// Configuration for a particle emitter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterConfig {
    /// Name/identifier for this emitter.
    pub name: String,
    /// Spawn shape.
    pub spawn_shape: SpawnShape,
    /// Particles per second (continuous) or count (burst).
    pub spawn_rate: f32,
    /// Maximum particles alive at once.
    pub max_particles: u32,
    /// Emission mode.
    pub emission_mode: EmissionMode,
    /// Burst count (for burst modes).
    pub burst_count: u32,
    /// Burst interval (for repeating burst).
    pub burst_interval: f32,
    /// Particle lifetime range.
    pub lifetime: ValueRange,
    /// Initial velocity mode.
    pub velocity_mode: VelocityMode,
    /// Initial velocity direction (for directional mode).
    pub velocity_direction: Vec3,
    /// Initial speed range.
    pub speed: ValueRange,
    /// Spread angle in radians (cone of uncertainty).
    pub spread_angle: f32,
    /// Initial size range.
    pub size: ValueRange,
    /// Size over lifetime curve.
    pub size_over_lifetime: OverTimeCurve,
    /// Initial rotation range (radians).
    pub rotation: ValueRange,
    /// Angular velocity range (radians/sec).
    pub angular_velocity: ValueRange,
    /// Color over lifetime.
    pub color_over_lifetime: ColorOverTime,
    /// Gravity multiplier.
    pub gravity_multiplier: f32,
    /// Drag coefficient (0 = no drag, 1 = high drag).
    pub drag: f32,
    /// Simulation space.
    pub simulation_space: SimulationSpace,
    /// Whether the emitter loops.
    pub looping: bool,
    /// Emitter duration (0 = infinite for continuous).
    pub duration: f32,
    /// Start delay before emission begins.
    pub start_delay: f32,
    /// Seed for deterministic randomization.
    pub seed: u32,
    /// Whether the emitter is enabled.
    pub enabled: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            name: String::from("Emitter"),
            spawn_shape: SpawnShape::point(Vec3::ZERO),
            spawn_rate: 100.0,
            max_particles: 1000,
            emission_mode: EmissionMode::Continuous,
            burst_count: 50,
            burst_interval: 1.0,
            lifetime: ValueRange::range(1.0, 3.0),
            velocity_mode: VelocityMode::Directional,
            velocity_direction: Vec3::Y,
            speed: ValueRange::range(1.0, 3.0),
            spread_angle: 0.2,
            size: ValueRange::range(0.02, 0.05),
            size_over_lifetime: OverTimeCurve::linear(1.0, 0.0),
            rotation: ValueRange::range(0.0, std::f32::consts::TAU),
            angular_velocity: ValueRange::range(-1.0, 1.0),
            color_over_lifetime: ColorOverTime::default(),
            gravity_multiplier: 1.0,
            drag: 0.0,
            simulation_space: SimulationSpace::World,
            looping: true,
            duration: 0.0,
            start_delay: 0.0,
            seed: 0,
            enabled: true,
        }
    }
}

impl EmitterConfig {
    /// Create a basic rain emitter.
    #[must_use]
    pub fn rain(spawn_area: Vec3) -> Self {
        Self {
            name: String::from("Rain"),
            spawn_shape: SpawnShape::box_volume(Vec3::new(0.0, spawn_area.y, 0.0), spawn_area),
            spawn_rate: 500.0,
            max_particles: 5000,
            lifetime: ValueRange::range(0.5, 1.5),
            velocity_mode: VelocityMode::Directional,
            velocity_direction: Vec3::NEG_Y,
            speed: ValueRange::range(10.0, 15.0),
            spread_angle: 0.1,
            size: ValueRange::range(0.01, 0.02),
            size_over_lifetime: OverTimeCurve::constant(1.0),
            color_over_lifetime: ColorOverTime::fade_out(0.7, 0.8, 0.9),
            gravity_multiplier: 1.0,
            drag: 0.01,
            ..Default::default()
        }
    }

    /// Create a basic snow emitter.
    #[must_use]
    pub fn snow(spawn_area: Vec3) -> Self {
        Self {
            name: String::from("Snow"),
            spawn_shape: SpawnShape::box_volume(Vec3::new(0.0, spawn_area.y, 0.0), spawn_area),
            spawn_rate: 150.0,
            max_particles: 3000,
            lifetime: ValueRange::range(4.0, 8.0),
            velocity_mode: VelocityMode::Directional,
            velocity_direction: Vec3::NEG_Y,
            speed: ValueRange::range(0.5, 2.0),
            spread_angle: 0.5,
            size: ValueRange::range(0.02, 0.05),
            size_over_lifetime: OverTimeCurve::constant(1.0),
            angular_velocity: ValueRange::range(-2.0, 2.0),
            color_over_lifetime: ColorOverTime::fade_out(1.0, 1.0, 1.0),
            gravity_multiplier: 0.3,
            drag: 0.2,
            ..Default::default()
        }
    }

    /// Create a dust/debris emitter.
    #[must_use]
    pub fn dust(center: Vec3, radius: f32) -> Self {
        Self {
            name: String::from("Dust"),
            spawn_shape: SpawnShape::sphere(center, radius),
            spawn_rate: 50.0,
            max_particles: 500,
            lifetime: ValueRange::range(3.0, 8.0),
            velocity_mode: VelocityMode::Random,
            speed: ValueRange::range(0.1, 0.5),
            spread_angle: std::f32::consts::PI,
            size: ValueRange::range(0.005, 0.015),
            size_over_lifetime: OverTimeCurve::from_preset(
                1.0,
                0.0,
                super::curve::CurvePreset::EaseOut,
            ),
            color_over_lifetime: ColorOverTime::fade_out(0.8, 0.7, 0.5),
            gravity_multiplier: 0.05,
            drag: 0.3,
            ..Default::default()
        }
    }

    /// Create a bubble emitter (underwater).
    #[must_use]
    pub fn bubbles(center: Vec3, radius: f32) -> Self {
        Self {
            name: String::from("Bubbles"),
            spawn_shape: SpawnShape::disc(center, radius),
            spawn_rate: 30.0,
            max_particles: 300,
            lifetime: ValueRange::range(2.0, 5.0),
            velocity_mode: VelocityMode::Directional,
            velocity_direction: Vec3::Y,
            speed: ValueRange::range(0.5, 1.5),
            spread_angle: 0.3,
            size: ValueRange::range(0.01, 0.03),
            size_over_lifetime: OverTimeCurve::from_preset(
                0.5,
                1.5,
                super::curve::CurvePreset::EaseOut,
            ),
            color_over_lifetime: ColorOverTime::fade_out(0.6, 0.8, 1.0),
            gravity_multiplier: -0.1,
            drag: 0.1,
            ..Default::default()
        }
    }

    /// Create a burst emitter (explosion/impact).
    #[must_use]
    pub fn burst(center: Vec3, count: u32) -> Self {
        Self {
            name: String::from("Burst"),
            spawn_shape: SpawnShape::point(center),
            spawn_rate: 0.0,
            max_particles: count,
            emission_mode: EmissionMode::Burst,
            burst_count: count,
            lifetime: ValueRange::range(0.5, 2.0),
            velocity_mode: VelocityMode::Radial,
            speed: ValueRange::range(3.0, 8.0),
            spread_angle: 0.0,
            size: ValueRange::range(0.02, 0.08),
            size_over_lifetime: OverTimeCurve::linear(1.0, 0.0),
            color_over_lifetime: ColorOverTime::gradient(
                (1.0, 0.8, 0.3, 1.0),
                (0.3, 0.1, 0.0, 0.0),
            ),
            gravity_multiplier: 0.5,
            drag: 0.1,
            looping: false,
            ..Default::default()
        }
    }

    /// Set spawn shape.
    #[must_use]
    pub fn with_spawn_shape(mut self, shape: SpawnShape) -> Self {
        self.spawn_shape = shape;
        self
    }

    /// Set spawn rate.
    #[must_use]
    pub fn with_spawn_rate(mut self, rate: f32) -> Self {
        self.spawn_rate = rate.max(0.0);
        self
    }

    /// Set max particles.
    #[must_use]
    pub fn with_max_particles(mut self, max: u32) -> Self {
        self.max_particles = max;
        self
    }

    /// Set lifetime range.
    #[must_use]
    pub fn with_lifetime(mut self, min: f32, max: f32) -> Self {
        self.lifetime = ValueRange::range(min.max(0.01), max.max(0.01));
        self
    }

    /// Set velocity direction.
    #[must_use]
    pub fn with_velocity(mut self, direction: Vec3, speed_min: f32, speed_max: f32) -> Self {
        self.velocity_mode = VelocityMode::Directional;
        self.velocity_direction = direction.normalize_or_zero();
        self.speed = ValueRange::range(speed_min, speed_max);
        self
    }

    /// Set gravity multiplier.
    #[must_use]
    pub fn with_gravity(mut self, multiplier: f32) -> Self {
        self.gravity_multiplier = multiplier;
        self
    }

    /// Set seed for deterministic behavior.
    #[must_use]
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Set enabled state.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Estimate particles per frame at given delta time.
    #[must_use]
    pub fn particles_per_frame(&self, delta_time: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        match self.emission_mode {
            EmissionMode::Continuous => self.spawn_rate * delta_time,
            EmissionMode::Burst | EmissionMode::BurstRepeat => 0.0,
        }
    }

    /// Estimate maximum active particles based on spawn rate and lifetime.
    #[must_use]
    pub fn estimated_max_active(&self) -> u32 {
        if !self.enabled {
            return 0;
        }
        let avg_lifetime = self.lifetime.midpoint();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to non-negative, bounded by max_particles which fits in u32"
        )]
        let estimated = (self.spawn_rate * avg_lifetime).max(0.0) as u32;
        estimated.min(self.max_particles)
    }

    /// Check if configuration is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.spawn_rate >= 0.0
            && self.max_particles > 0
            && self.lifetime.min > 0.0
            && self.lifetime.max >= self.lifetime.min
            && self.speed.max >= self.speed.min
            && self.size.min > 0.0
            && self.size.max >= self.size.min
            && self.spawn_shape.is_valid()
    }

    /// Clamp values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.spawn_rate = self.spawn_rate.max(0.0);
        self.max_particles = self.max_particles.max(1);
        self.lifetime.min = self.lifetime.min.max(0.01);
        self.lifetime.max = self.lifetime.max.max(self.lifetime.min);
        self.speed.min = self.speed.min.max(0.0);
        self.speed.max = self.speed.max.max(self.speed.min);
        self.size.min = self.size.min.max(0.001);
        self.size.max = self.size.max.max(self.size.min);
        self.spread_angle = self.spread_angle.clamp(0.0, std::f32::consts::PI);
        self.drag = self.drag.clamp(0.0, 1.0);
        self.burst_interval = self.burst_interval.max(0.01);
        self.spawn_shape = self.spawn_shape.clamped();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_value_range_constant() {
        let range = ValueRange::constant(0.5);
        assert!(range.is_constant());
        assert_relative_eq!(range.sample(0.0), 0.5, epsilon = 0.001);
        assert_relative_eq!(range.sample(1.0), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_value_range_sample() {
        let range = ValueRange::range(0.0, 10.0);
        assert_relative_eq!(range.sample(0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(range.sample(0.5), 5.0, epsilon = 0.001);
        assert_relative_eq!(range.sample(1.0), 10.0, epsilon = 0.001);
    }

    #[test]
    fn test_value_range_midpoint() {
        let range = ValueRange::range(2.0, 8.0);
        assert_relative_eq!(range.midpoint(), 5.0, epsilon = 0.001);
    }

    #[test]
    fn test_emitter_default_valid() {
        let config = EmitterConfig::default();
        assert!(config.is_valid());
    }

    #[test]
    fn test_emitter_rain_valid() {
        let config = EmitterConfig::rain(Vec3::new(50.0, 100.0, 50.0));
        assert!(config.is_valid());
        assert_eq!(config.velocity_direction, Vec3::NEG_Y);
    }

    #[test]
    fn test_emitter_snow_valid() {
        let config = EmitterConfig::snow(Vec3::new(50.0, 100.0, 50.0));
        assert!(config.is_valid());
        assert!(config.gravity_multiplier < 1.0);
    }

    #[test]
    fn test_emitter_burst_valid() {
        let config = EmitterConfig::burst(Vec3::ZERO, 100);
        assert!(config.is_valid());
        assert_eq!(config.emission_mode, EmissionMode::Burst);
        assert_eq!(config.burst_count, 100);
        assert!(!config.looping);
    }

    #[test]
    fn test_emitter_estimated_max() {
        let config = EmitterConfig::default()
            .with_spawn_rate(100.0)
            .with_lifetime(2.0, 2.0);

        let estimated = config.estimated_max_active();
        assert_eq!(estimated, 200);
    }

    #[test]
    fn test_emitter_particles_per_frame() {
        let config = EmitterConfig::default().with_spawn_rate(60.0);

        let ppf = config.particles_per_frame(1.0 / 60.0);
        assert_relative_eq!(ppf, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_emitter_disabled() {
        let config = EmitterConfig::default().with_enabled(false);

        assert_relative_eq!(config.particles_per_frame(0.016), 0.0, epsilon = 0.001);
        assert_eq!(config.estimated_max_active(), 0);
    }

    #[test]
    fn test_emitter_clamped() {
        let config = EmitterConfig {
            spawn_rate: -10.0,
            max_particles: 0,
            lifetime: ValueRange::range(-1.0, 0.0),
            ..Default::default()
        }
        .clamped();

        assert!(config.is_valid());
        assert!(config.spawn_rate >= 0.0);
        assert!(config.max_particles >= 1);
        assert!(config.lifetime.min > 0.0);
    }

    #[test]
    fn test_emitter_builders() {
        use approx::assert_relative_eq;
        let config = EmitterConfig::default()
            .with_spawn_shape(SpawnShape::sphere(Vec3::ZERO, 5.0))
            .with_spawn_rate(200.0)
            .with_max_particles(500)
            .with_gravity(0.5)
            .with_seed(42);

        assert_relative_eq!(config.spawn_rate, 200.0, epsilon = 0.001);
        assert_eq!(config.max_particles, 500);
        assert_relative_eq!(config.gravity_multiplier, 0.5, epsilon = 0.001);
        assert_eq!(config.seed, 42);
    }

    #[test]
    fn test_simulation_space() {
        for space in SimulationSpace::ALL {
            assert_eq!(space as u8, space as u8);
        }
    }

    #[test]
    fn test_velocity_mode() {
        for mode in VelocityMode::ALL {
            assert_eq!(mode as u8, mode as u8);
        }
    }

    #[test]
    fn test_bubbles_rises() {
        let config = EmitterConfig::bubbles(Vec3::ZERO, 5.0);
        assert!(config.gravity_multiplier < 0.0, "bubbles should rise");
        assert_eq!(config.velocity_direction, Vec3::Y);
    }
}

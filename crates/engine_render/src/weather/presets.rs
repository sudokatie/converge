//! Preset weather configurations.
//!
//! Ready-to-use configurations for common weather scenarios.

use super::curve::{ColorOverTime, CurvePreset, OverTimeCurve};
use super::effect::{WeatherEffect, WeatherKind};
use super::emitter::{EmissionMode, EmitterConfig, SimulationSpace, ValueRange, VelocityMode};
use super::shape::SpawnShape;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Preset configurations for common weather scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WeatherPreset {
    /// Light rain.
    LightRain = 0,
    /// Heavy rain/storm.
    HeavyRain = 1,
    /// Light snowfall.
    LightSnow = 2,
    /// Blizzard.
    Blizzard = 3,
    /// Underwater ambient.
    UnderwaterAmbient = 4,
    /// Deep ocean.
    DeepOcean = 5,
    /// Desert dust.
    DesertDust = 6,
    /// Dust storm.
    DustStorm = 7,
    /// Forest spores.
    ForestSpores = 8,
    /// Volcanic ash.
    VolcanicAsh = 9,
    /// Ember storm.
    EmberStorm = 10,
    /// Space debris.
    SpaceDebris = 11,
    /// Light fog.
    LightFog = 12,
    /// Dense fog.
    DenseFog = 13,
}

impl WeatherPreset {
    /// All presets.
    pub const ALL: [Self; 14] = [
        Self::LightRain,
        Self::HeavyRain,
        Self::LightSnow,
        Self::Blizzard,
        Self::UnderwaterAmbient,
        Self::DeepOcean,
        Self::DesertDust,
        Self::DustStorm,
        Self::ForestSpores,
        Self::VolcanicAsh,
        Self::EmberStorm,
        Self::SpaceDebris,
        Self::LightFog,
        Self::DenseFog,
    ];

    /// Get the weather kind for this preset.
    #[must_use]
    pub fn kind(self) -> WeatherKind {
        match self {
            Self::LightRain | Self::HeavyRain => WeatherKind::Rain,
            Self::LightSnow | Self::Blizzard => WeatherKind::Snow,
            Self::UnderwaterAmbient | Self::DeepOcean => WeatherKind::Underwater,
            Self::DesertDust | Self::DustStorm => WeatherKind::Dust,
            Self::ForestSpores => WeatherKind::Spores,
            Self::VolcanicAsh | Self::EmberStorm => WeatherKind::Ash,
            Self::SpaceDebris => WeatherKind::Vacuum,
            Self::LightFog | Self::DenseFog => WeatherKind::Fog,
        }
    }

    /// Get the weather effect for this preset.
    #[must_use]
    pub fn effect(self) -> WeatherEffect {
        match self {
            Self::LightRain => WeatherEffect::rain().with_intensity(0.4),
            Self::HeavyRain => WeatherEffect::rain()
                .with_intensity(1.0)
                .with_wind(Vec3::new(2.0, 0.0, 0.5)),
            Self::LightSnow => WeatherEffect::snow().with_intensity(0.3),
            Self::Blizzard => WeatherEffect::snow()
                .with_intensity(1.0)
                .with_wind(Vec3::new(4.0, 0.0, 1.0))
                .with_turbulence(0.8),
            Self::UnderwaterAmbient => WeatherEffect::underwater().with_intensity(0.3),
            Self::DeepOcean => WeatherEffect::underwater()
                .with_intensity(0.5)
                .with_color(Vec3::new(0.3, 0.5, 0.8))
                .with_opacity(0.6),
            Self::DesertDust => WeatherEffect::dust().with_intensity(0.3),
            Self::DustStorm => WeatherEffect::dust()
                .with_intensity(1.0)
                .with_wind(Vec3::new(5.0, 0.0, 2.0))
                .with_turbulence(0.9),
            Self::ForestSpores => WeatherEffect::spores(),
            Self::VolcanicAsh => WeatherEffect::ash()
                .with_intensity(0.6)
                .with_color(Vec3::new(0.2, 0.2, 0.2)),
            Self::EmberStorm => WeatherEffect::ash()
                .with_intensity(0.8)
                .with_color(Vec3::new(1.0, 0.5, 0.2))
                .with_wind(Vec3::new(2.0, 1.0, 0.5)),
            Self::SpaceDebris => WeatherEffect::vacuum(),
            Self::LightFog => WeatherEffect::fog().with_intensity(0.4),
            Self::DenseFog => WeatherEffect::fog().with_intensity(0.9).with_opacity(0.6),
        }
    }

    /// Create an emitter config for this preset with the given spawn bounds.
    #[must_use]
    pub fn emitter(self, spawn_bounds: Vec3) -> EmitterConfig {
        match self {
            Self::LightRain => rain_emitter(spawn_bounds, 0.4),
            Self::HeavyRain => rain_emitter(spawn_bounds, 1.0),
            Self::LightSnow => snow_emitter(spawn_bounds, 0.3),
            Self::Blizzard => snow_emitter(spawn_bounds, 1.0),
            Self::UnderwaterAmbient => underwater_emitter(spawn_bounds, 0.3),
            Self::DeepOcean => underwater_emitter(spawn_bounds, 0.5),
            Self::DesertDust => dust_emitter(spawn_bounds, 0.3),
            Self::DustStorm => dust_emitter(spawn_bounds, 1.0),
            Self::ForestSpores => spore_emitter(spawn_bounds, 0.5),
            Self::VolcanicAsh => ash_emitter(spawn_bounds, 0.6),
            Self::EmberStorm => ember_emitter(spawn_bounds, 0.8),
            Self::SpaceDebris => debris_emitter(spawn_bounds, 0.3),
            Self::LightFog => fog_emitter(spawn_bounds, 0.4),
            Self::DenseFog => fog_emitter(spawn_bounds, 0.9),
        }
    }

    /// Get a descriptive name for this preset.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::LightRain => "Light Rain",
            Self::HeavyRain => "Heavy Rain",
            Self::LightSnow => "Light Snow",
            Self::Blizzard => "Blizzard",
            Self::UnderwaterAmbient => "Underwater Ambient",
            Self::DeepOcean => "Deep Ocean",
            Self::DesertDust => "Desert Dust",
            Self::DustStorm => "Dust Storm",
            Self::ForestSpores => "Forest Spores",
            Self::VolcanicAsh => "Volcanic Ash",
            Self::EmberStorm => "Ember Storm",
            Self::SpaceDebris => "Space Debris",
            Self::LightFog => "Light Fog",
            Self::DenseFog => "Dense Fog",
        }
    }

    /// Get a description of this preset.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::LightRain => "Gentle rain with minimal wind",
            Self::HeavyRain => "Intense rainfall with wind",
            Self::LightSnow => "Gentle snowfall with light drift",
            Self::Blizzard => "Heavy snow with strong wind and turbulence",
            Self::UnderwaterAmbient => "Subtle underwater particle effects",
            Self::DeepOcean => "Dense underwater particles with low visibility",
            Self::DesertDust => "Light dust particles in air",
            Self::DustStorm => "Intense sandstorm conditions",
            Self::ForestSpores => "Floating spores and pollen",
            Self::VolcanicAsh => "Falling volcanic ash",
            Self::EmberStorm => "Rising embers with glowing particles",
            Self::SpaceDebris => "Zero-gravity floating debris",
            Self::LightFog => "Subtle atmospheric fog",
            Self::DenseFog => "Heavy fog with reduced visibility",
        }
    }

    /// Whether this preset is indoor-appropriate.
    #[must_use]
    pub fn is_indoor_appropriate(self) -> bool {
        matches!(
            self,
            Self::LightFog | Self::ForestSpores | Self::VolcanicAsh
        )
    }

    /// Get the recommended max particle count for this preset.
    #[must_use]
    pub fn recommended_max_particles(self) -> u32 {
        match self {
            Self::LightRain | Self::LightSnow | Self::LightFog => 2000,
            Self::HeavyRain | Self::Blizzard | Self::DustStorm | Self::DenseFog => 5000,
            Self::UnderwaterAmbient | Self::ForestSpores => 500,
            Self::DeepOcean | Self::DesertDust => 1000,
            Self::VolcanicAsh | Self::EmberStorm => 1500,
            Self::SpaceDebris => 300,
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "clamped to valid u32 range"
)]
fn saturating_particle_count(base: f32, intensity: f32) -> u32 {
    (base * intensity).clamp(0.0, u32::MAX as f32) as u32
}

fn rain_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Rain"),
        spawn_shape: SpawnShape::box_volume(Vec3::new(0.0, bounds.y, 0.0), bounds),
        spawn_rate: 500.0 * intensity,
        max_particles: saturating_particle_count(5000.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(0.5, 1.5),
        velocity_mode: VelocityMode::Directional,
        velocity_direction: Vec3::new(-0.1, -1.0, 0.0).normalize(),
        speed: ValueRange::range(10.0, 15.0),
        spread_angle: 0.05,
        size: ValueRange::range(0.01, 0.02),
        size_over_lifetime: OverTimeCurve::constant(1.0),
        color_over_lifetime: ColorOverTime::fade_out(0.7, 0.8, 0.9),
        gravity_multiplier: 1.0,
        drag: 0.02,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn snow_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Snow"),
        spawn_shape: SpawnShape::box_volume(Vec3::new(0.0, bounds.y, 0.0), bounds),
        spawn_rate: 200.0 * intensity,
        max_particles: saturating_particle_count(3000.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(4.0, 8.0),
        velocity_mode: VelocityMode::Directional,
        velocity_direction: Vec3::NEG_Y,
        speed: ValueRange::range(0.5, 2.0),
        spread_angle: 0.5,
        size: ValueRange::range(0.02, 0.06),
        size_over_lifetime: OverTimeCurve::constant(1.0),
        rotation: ValueRange::range(0.0, std::f32::consts::TAU),
        angular_velocity: ValueRange::range(-3.0, 3.0),
        color_over_lifetime: ColorOverTime::fade_out(1.0, 1.0, 1.0),
        gravity_multiplier: 0.3,
        drag: 0.15,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn underwater_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Underwater"),
        spawn_shape: SpawnShape::box_volume(Vec3::ZERO, bounds),
        spawn_rate: 100.0 * intensity,
        max_particles: saturating_particle_count(1000.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(3.0, 6.0),
        velocity_mode: VelocityMode::Random,
        speed: ValueRange::range(0.1, 0.3),
        spread_angle: std::f32::consts::PI,
        size: ValueRange::range(0.005, 0.02),
        size_over_lifetime: OverTimeCurve::from_preset(0.5, 1.0, CurvePreset::EaseOut),
        color_over_lifetime: ColorOverTime::fade_out(0.6, 0.8, 1.0),
        gravity_multiplier: -0.1,
        drag: 0.2,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn dust_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Dust"),
        spawn_shape: SpawnShape::box_volume(Vec3::ZERO, bounds),
        spawn_rate: 150.0 * intensity,
        max_particles: saturating_particle_count(2000.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(4.0, 10.0),
        velocity_mode: VelocityMode::Random,
        speed: ValueRange::range(0.2, 1.0),
        spread_angle: std::f32::consts::PI,
        size: ValueRange::range(0.005, 0.015),
        size_over_lifetime: OverTimeCurve::from_preset(1.0, 0.0, CurvePreset::EaseOut),
        color_over_lifetime: ColorOverTime::fade_out(0.8, 0.7, 0.5),
        gravity_multiplier: 0.05,
        drag: 0.3,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn spore_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Spores"),
        spawn_shape: SpawnShape::box_volume(Vec3::ZERO, bounds),
        spawn_rate: 50.0 * intensity,
        max_particles: saturating_particle_count(500.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(5.0, 12.0),
        velocity_mode: VelocityMode::Random,
        speed: ValueRange::range(0.05, 0.2),
        spread_angle: std::f32::consts::PI,
        size: ValueRange::range(0.003, 0.008),
        size_over_lifetime: OverTimeCurve::from_preset(0.5, 1.5, CurvePreset::Pulse),
        color_over_lifetime: ColorOverTime::fade_out(0.9, 1.0, 0.6),
        gravity_multiplier: 0.02,
        drag: 0.4,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn ash_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Ash"),
        spawn_shape: SpawnShape::box_volume(Vec3::new(0.0, bounds.y, 0.0), bounds),
        spawn_rate: 100.0 * intensity,
        max_particles: saturating_particle_count(1500.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(3.0, 7.0),
        velocity_mode: VelocityMode::Directional,
        velocity_direction: Vec3::NEG_Y,
        speed: ValueRange::range(0.5, 2.0),
        spread_angle: 0.4,
        size: ValueRange::range(0.01, 0.03),
        size_over_lifetime: OverTimeCurve::constant(1.0),
        rotation: ValueRange::range(0.0, std::f32::consts::TAU),
        angular_velocity: ValueRange::range(-2.0, 2.0),
        color_over_lifetime: ColorOverTime::fade_out(0.3, 0.3, 0.3),
        gravity_multiplier: 0.15,
        drag: 0.2,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn ember_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Embers"),
        spawn_shape: SpawnShape::box_volume(Vec3::new(0.0, -bounds.y * 0.5, 0.0), bounds),
        spawn_rate: 80.0 * intensity,
        max_particles: saturating_particle_count(1000.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(2.0, 5.0),
        velocity_mode: VelocityMode::Directional,
        velocity_direction: Vec3::Y,
        speed: ValueRange::range(1.0, 3.0),
        spread_angle: 0.3,
        size: ValueRange::range(0.005, 0.015),
        size_over_lifetime: OverTimeCurve::from_preset(1.0, 0.0, CurvePreset::EaseIn),
        color_over_lifetime: ColorOverTime::gradient((1.0, 0.6, 0.2, 1.0), (0.8, 0.2, 0.0, 0.0)),
        gravity_multiplier: -0.05,
        drag: 0.15,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn debris_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Debris"),
        spawn_shape: SpawnShape::box_volume(Vec3::ZERO, bounds),
        spawn_rate: 20.0 * intensity,
        max_particles: saturating_particle_count(300.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(10.0, 25.0),
        velocity_mode: VelocityMode::Random,
        speed: ValueRange::range(0.01, 0.1),
        spread_angle: std::f32::consts::PI,
        size: ValueRange::range(0.01, 0.04),
        size_over_lifetime: OverTimeCurve::constant(1.0),
        rotation: ValueRange::range(0.0, std::f32::consts::TAU),
        angular_velocity: ValueRange::range(-0.5, 0.5),
        color_over_lifetime: ColorOverTime::fade_out(0.5, 0.5, 0.6),
        gravity_multiplier: 0.0,
        drag: 0.0,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

fn fog_emitter(bounds: Vec3, intensity: f32) -> EmitterConfig {
    EmitterConfig {
        name: String::from("Fog"),
        spawn_shape: SpawnShape::box_volume(Vec3::ZERO, bounds),
        spawn_rate: 80.0 * intensity,
        max_particles: saturating_particle_count(1500.0, intensity),
        emission_mode: EmissionMode::Continuous,
        lifetime: ValueRange::range(4.0, 10.0),
        velocity_mode: VelocityMode::Random,
        speed: ValueRange::range(0.05, 0.2),
        spread_angle: std::f32::consts::PI,
        size: ValueRange::range(0.1, 0.3),
        size_over_lifetime: OverTimeCurve::from_preset(0.5, 1.5, CurvePreset::EaseInOut),
        color_over_lifetime: ColorOverTime::fade_out(0.9, 0.9, 0.95),
        gravity_multiplier: 0.0,
        drag: 0.1,
        simulation_space: SimulationSpace::World,
        ..Default::default()
    }
}

/// Create a full weather configuration from a preset.
#[must_use]
pub fn create_from_preset(
    preset: WeatherPreset,
    spawn_bounds: Vec3,
) -> (WeatherEffect, EmitterConfig) {
    (preset.effect(), preset.emitter(spawn_bounds))
}

/// Create multiple weather layers for complex weather.
#[must_use]
pub fn create_layered(
    presets: &[WeatherPreset],
    spawn_bounds: Vec3,
) -> Vec<(WeatherEffect, EmitterConfig)> {
    presets
        .iter()
        .map(|&preset| create_from_preset(preset, spawn_bounds))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_produce_valid_effects() {
        for preset in WeatherPreset::ALL {
            let effect = preset.effect();
            assert!(effect.is_valid(), "{preset:?} should produce valid effect");
        }
    }

    #[test]
    fn test_all_presets_produce_valid_emitters() {
        let bounds = Vec3::new(50.0, 100.0, 50.0);
        for preset in WeatherPreset::ALL {
            let emitter = preset.emitter(bounds);
            assert!(
                emitter.is_valid(),
                "{preset:?} should produce valid emitter"
            );
        }
    }

    #[test]
    fn test_preset_kinds() {
        assert_eq!(WeatherPreset::LightRain.kind(), WeatherKind::Rain);
        assert_eq!(WeatherPreset::Blizzard.kind(), WeatherKind::Snow);
        assert_eq!(WeatherPreset::DeepOcean.kind(), WeatherKind::Underwater);
        assert_eq!(WeatherPreset::SpaceDebris.kind(), WeatherKind::Vacuum);
    }

    #[test]
    fn test_preset_names() {
        for preset in WeatherPreset::ALL {
            assert!(!preset.name().is_empty());
            assert!(!preset.description().is_empty());
        }
    }

    #[test]
    fn test_create_from_preset() {
        let bounds = Vec3::new(50.0, 100.0, 50.0);
        let (effect, emitter) = create_from_preset(WeatherPreset::HeavyRain, bounds);

        assert_eq!(effect.kind, WeatherKind::Rain);
        assert!(effect.is_valid());
        assert!(emitter.is_valid());
    }

    #[test]
    fn test_create_layered() {
        let bounds = Vec3::new(50.0, 100.0, 50.0);
        let presets = [WeatherPreset::LightRain, WeatherPreset::LightFog];
        let layers = create_layered(&presets, bounds);

        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].0.kind, WeatherKind::Rain);
        assert_eq!(layers[1].0.kind, WeatherKind::Fog);
    }

    #[test]
    fn test_heavy_vs_light_intensity() {
        let bounds = Vec3::new(50.0, 100.0, 50.0);

        let light = WeatherPreset::LightRain.emitter(bounds);
        let heavy = WeatherPreset::HeavyRain.emitter(bounds);

        assert!(
            heavy.spawn_rate > light.spawn_rate,
            "heavy should spawn more particles"
        );
    }

    #[test]
    fn test_underwater_gravity_negative() {
        let bounds = Vec3::new(20.0, 20.0, 20.0);
        let emitter = WeatherPreset::UnderwaterAmbient.emitter(bounds);

        assert!(emitter.gravity_multiplier < 0.0, "underwater should rise");
    }

    #[test]
    fn test_space_no_gravity() {
        let bounds = Vec3::new(20.0, 20.0, 20.0);
        let emitter = WeatherPreset::SpaceDebris.emitter(bounds);

        assert!(
            emitter.gravity_multiplier.abs() < 0.01,
            "space should have no gravity"
        );
    }

    #[test]
    fn test_indoor_appropriate() {
        assert!(WeatherPreset::LightFog.is_indoor_appropriate());
        assert!(!WeatherPreset::HeavyRain.is_indoor_appropriate());
    }

    #[test]
    fn test_recommended_particles() {
        for preset in WeatherPreset::ALL {
            let max = preset.recommended_max_particles();
            assert!(max > 0);
            assert!(max <= 10000);
        }
    }
}

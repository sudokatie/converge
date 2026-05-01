//! GPU-friendly uniform structures for weather effects.
//!
//! These structures are designed to be directly uploaded to GPU buffers
//! with proper alignment and layout for shader access.

use super::effect::{WeatherEffect, WeatherKind};
use super::emitter::{EmitterConfig, SimulationSpace, VelocityMode};
use super::shape::{SpawnShape, SpawnShapeKind};
use bytemuck::{Pod, Zeroable};

/// GPU-friendly weather effect uniform.
///
/// 64 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct WeatherEffectUniform {
    /// Wind XYZ + intensity W.
    pub wind_intensity: [f32; 4],
    /// Color RGB + opacity A.
    pub color_opacity: [f32; 4],
    /// Gravity, turbulence, `turbulence_freq`, `particle_size`.
    pub dynamics: [f32; 4],
    /// Size variation, kind, active, padding.
    pub params: [f32; 4],
}

impl From<WeatherEffect> for WeatherEffectUniform {
    fn from(effect: WeatherEffect) -> Self {
        Self {
            wind_intensity: [
                effect.wind.x,
                effect.wind.y,
                effect.wind.z,
                effect.intensity,
            ],
            color_opacity: [
                effect.color.x,
                effect.color.y,
                effect.color.z,
                effect.opacity,
            ],
            dynamics: [
                effect.gravity,
                effect.turbulence,
                effect.turbulence_frequency,
                effect.particle_size,
            ],
            params: [
                effect.size_variation,
                f32::from(effect.kind as u8),
                if effect.active { 1.0 } else { 0.0 },
                0.0,
            ],
        }
    }
}

impl Default for WeatherEffectUniform {
    fn default() -> Self {
        WeatherEffect::default().into()
    }
}

/// GPU-friendly spawn shape uniform.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpawnShapeUniform {
    /// Center XYZ + shape kind W.
    pub center_kind: [f32; 4],
    /// Extents XYZ + padding.
    pub extents: [f32; 4],
    /// Secondary params XYZ + padding.
    pub secondary: [f32; 4],
}

impl From<SpawnShape> for SpawnShapeUniform {
    fn from(shape: SpawnShape) -> Self {
        Self {
            center_kind: [
                shape.center.x,
                shape.center.y,
                shape.center.z,
                f32::from(shape.kind as u8),
            ],
            extents: [shape.extents.x, shape.extents.y, shape.extents.z, 0.0],
            secondary: [shape.secondary.x, shape.secondary.y, shape.secondary.z, 0.0],
        }
    }
}

impl Default for SpawnShapeUniform {
    fn default() -> Self {
        SpawnShape::default().into()
    }
}

/// GPU-friendly emitter config uniform.
///
/// 96 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EmitterConfigUniform {
    /// Spawn shape.
    pub shape: SpawnShapeUniform,
    /// Velocity direction XYZ + speed min W.
    pub velocity_speed: [f32; 4],
    /// Speed max, spread angle, lifetime min, lifetime max.
    pub lifetime_params: [f32; 4],
    /// Size min, size max, gravity mult, drag.
    pub size_params: [f32; 4],
    /// Spawn rate, max particles, velocity mode, simulation space.
    pub emission_params: [u32; 4],
}

impl From<&EmitterConfig> for EmitterConfigUniform {
    fn from(config: &EmitterConfig) -> Self {
        Self {
            shape: config.spawn_shape.into(),
            velocity_speed: [
                config.velocity_direction.x,
                config.velocity_direction.y,
                config.velocity_direction.z,
                config.speed.min,
            ],
            lifetime_params: [
                config.speed.max,
                config.spread_angle,
                config.lifetime.min,
                config.lifetime.max,
            ],
            size_params: [
                config.size.min,
                config.size.max,
                config.gravity_multiplier,
                config.drag,
            ],
            emission_params: [
                config.spawn_rate.to_bits(),
                config.max_particles,
                config.velocity_mode as u32,
                config.simulation_space as u32,
            ],
        }
    }
}

impl Default for EmitterConfigUniform {
    fn default() -> Self {
        (&EmitterConfig::default()).into()
    }
}

/// GPU-friendly particle instance.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct ParticleInstance {
    /// Position XYZ + size W.
    pub position_size: [f32; 4],
    /// Velocity XYZ + rotation W.
    pub velocity_rotation: [f32; 4],
    /// Color RGBA.
    pub color: [f32; 4],
}

impl ParticleInstance {
    /// Create a new particle instance.
    #[must_use]
    pub fn new(
        position: [f32; 3],
        size: f32,
        velocity: [f32; 3],
        rotation: f32,
        color: [f32; 4],
    ) -> Self {
        Self {
            position_size: [position[0], position[1], position[2], size],
            velocity_rotation: [velocity[0], velocity[1], velocity[2], rotation],
            color,
        }
    }
}

/// Batch of particle instances for GPU upload.
#[derive(Debug, Clone)]
pub struct ParticleBatch {
    /// Instance data.
    pub instances: Vec<ParticleInstance>,
    /// Maximum capacity.
    pub capacity: usize,
}

impl ParticleBatch {
    /// Create a new batch with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Add an instance if there's room.
    pub fn push(&mut self, instance: ParticleInstance) -> bool {
        if self.instances.len() < self.capacity {
            self.instances.push(instance);
            true
        } else {
            false
        }
    }

    /// Get raw byte data for GPU upload.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.instances)
    }

    /// Number of active instances.
    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Whether the batch is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.instances.len() >= self.capacity
    }
}

impl Default for ParticleBatch {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Utility to convert enum values to GPU-compatible formats.
pub mod convert {
    use super::{SimulationSpace, SpawnShapeKind, VelocityMode, WeatherKind};

    /// Convert weather kind to u32.
    #[must_use]
    pub fn kind_to_u32(kind: WeatherKind) -> u32 {
        kind as u32
    }

    /// Convert spawn shape kind to u32.
    #[must_use]
    pub fn shape_to_u32(shape: SpawnShapeKind) -> u32 {
        shape as u32
    }

    /// Convert velocity mode to u32.
    #[must_use]
    pub fn velocity_mode_to_u32(mode: VelocityMode) -> u32 {
        mode as u32
    }

    /// Convert simulation space to u32.
    #[must_use]
    pub fn simulation_space_to_u32(space: SimulationSpace) -> u32 {
        space as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn test_weather_uniform_conversion() {
        let effect = WeatherEffect::rain()
            .with_intensity(0.8)
            .with_wind(Vec3::new(1.0, 0.0, 0.5));
        let uniform: WeatherEffectUniform = effect.into();

        assert_relative_eq!(uniform.wind_intensity[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.wind_intensity[3], 0.8, epsilon = 0.001);
        assert_relative_eq!(
            uniform.params[1],
            f32::from(WeatherKind::Rain as u8),
            epsilon = 0.001
        );
    }

    #[test]
    fn test_shape_uniform_conversion() {
        let shape = SpawnShape::sphere(Vec3::new(1.0, 2.0, 3.0), 5.0);
        let uniform: SpawnShapeUniform = shape.into();

        assert_relative_eq!(uniform.center_kind[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.center_kind[1], 2.0, epsilon = 0.001);
        assert_relative_eq!(uniform.center_kind[2], 3.0, epsilon = 0.001);
        assert_relative_eq!(uniform.extents[0], 5.0, epsilon = 0.001);
    }

    #[test]
    fn test_emitter_uniform_conversion() {
        let config = EmitterConfig::rain(Vec3::new(50.0, 100.0, 50.0));
        let uniform: EmitterConfigUniform = (&config).into();

        assert_relative_eq!(
            uniform.lifetime_params[2],
            config.lifetime.min,
            epsilon = 0.001
        );
        assert_relative_eq!(
            uniform.lifetime_params[3],
            config.lifetime.max,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_particle_instance_creation() {
        let instance = ParticleInstance::new(
            [1.0, 2.0, 3.0],
            0.5,
            [0.0, -1.0, 0.0],
            0.0,
            [1.0, 1.0, 1.0, 1.0],
        );

        assert_relative_eq!(instance.position_size[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(instance.position_size[3], 0.5, epsilon = 0.001);
        assert_relative_eq!(instance.velocity_rotation[1], -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_weather_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<WeatherEffectUniform>() % 16,
            0,
            "weather uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_shape_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<SpawnShapeUniform>() % 16,
            0,
            "shape uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_emitter_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<EmitterConfigUniform>() % 16,
            0,
            "emitter uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_particle_instance_size_aligned() {
        assert_eq!(
            std::mem::size_of::<ParticleInstance>() % 16,
            0,
            "particle instance should be 16-byte aligned"
        );
    }

    #[test]
    fn test_batch_operations() {
        let mut batch = ParticleBatch::new(2);
        assert!(batch.is_empty());

        let instance = ParticleInstance::default();
        assert!(batch.push(instance));
        assert!(batch.push(instance));
        assert!(!batch.push(instance));

        assert!(batch.is_full());
        assert_eq!(batch.len(), 2);

        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn test_batch_as_bytes() {
        let mut batch = ParticleBatch::new(4);
        batch.push(ParticleInstance::default());

        let bytes = batch.as_bytes();
        assert_eq!(bytes.len(), std::mem::size_of::<ParticleInstance>());
    }

    #[test]
    fn test_convert_utilities() {
        assert_eq!(convert::kind_to_u32(WeatherKind::Rain), 0);
        assert_eq!(convert::kind_to_u32(WeatherKind::Snow), 1);
        assert_eq!(convert::shape_to_u32(SpawnShapeKind::Point), 0);
        assert_eq!(convert::shape_to_u32(SpawnShapeKind::Sphere), 1);
        assert_eq!(convert::velocity_mode_to_u32(VelocityMode::Directional), 0);
        assert_eq!(convert::simulation_space_to_u32(SimulationSpace::World), 1);
    }

    #[test]
    fn test_default_uniforms() {
        let weather = WeatherEffectUniform::default();
        let shape = SpawnShapeUniform::default();
        let emitter = EmitterConfigUniform::default();
        let particle = ParticleInstance::default();

        assert_relative_eq!(weather.params[2], 1.0, epsilon = 0.001);
        assert_relative_eq!(
            shape.center_kind[3],
            f32::from(SpawnShapeKind::Point as u8),
            epsilon = 0.001
        );
        assert!(emitter.emission_params[1] > 0);
        assert_relative_eq!(particle.position_size[0], 0.0, epsilon = 0.001);
    }
}

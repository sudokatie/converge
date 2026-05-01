//! Deterministic sampling for particle systems.
//!
//! Provides reproducible random-like values for particle spawning
//! and behavior. The same inputs always produce the same outputs.

use super::emitter::{EmitterConfig, ValueRange, VelocityMode};
use super::shape::{SpawnShape, SpawnShapeKind};
use glam::Vec3;
use std::f32::consts::TAU;

const HASH_PRIME: u32 = 374_761_393;
const HASH_MUL_A: u32 = 0x85eb_ca6b;
const HASH_MUL_B: u32 = 0xc2b2_ae35;

/// Deterministic particle sampler.
///
/// Generates reproducible values for particle systems.
/// The same seed and particle index always produce the same results.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleSampler {
    /// Base seed for all sampling.
    pub seed: u32,
}

impl ParticleSampler {
    /// Create a new sampler with the given seed.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Sample a normalized value (0-1) for a particle property.
    #[must_use]
    pub fn sample_unit(&self, particle_id: u32, property_id: u32) -> f32 {
        let hash = self.hash(particle_id, property_id);
        hash_to_unit(hash)
    }

    /// Sample a value from a range.
    #[must_use]
    pub fn sample_range(&self, particle_id: u32, property_id: u32, range: &ValueRange) -> f32 {
        let t = self.sample_unit(particle_id, property_id);
        range.sample(t)
    }

    /// Sample a 3D point within a spawn shape.
    #[must_use]
    pub fn sample_spawn_position(&self, particle_id: u32, shape: &SpawnShape) -> Vec3 {
        match shape.kind {
            SpawnShapeKind::Point => shape.center,
            SpawnShapeKind::Sphere => {
                let u = self.sample_unit(particle_id, 100);
                let v = self.sample_unit(particle_id, 101);
                let w = self.sample_unit(particle_id, 102);
                let radius = shape.extents.x * w.cbrt();
                let theta = TAU * u;
                let phi = (2.0 * v - 1.0).acos();
                shape.center + spherical_to_cartesian(radius, theta, phi)
            }
            SpawnShapeKind::SphereShell => {
                let u = self.sample_unit(particle_id, 100);
                let v = self.sample_unit(particle_id, 101);
                let w = self.sample_unit(particle_id, 102);
                let inner_ratio = shape.secondary.x;
                let radius = shape.extents.x
                    * (inner_ratio.powi(3) + w * (1.0 - inner_ratio.powi(3))).cbrt();
                let theta = TAU * u;
                let phi = (2.0 * v - 1.0).acos();
                shape.center + spherical_to_cartesian(radius, theta, phi)
            }
            SpawnShapeKind::Box => {
                let x = self.sample_unit(particle_id, 100) * 2.0 - 1.0;
                let y = self.sample_unit(particle_id, 101) * 2.0 - 1.0;
                let z = self.sample_unit(particle_id, 102) * 2.0 - 1.0;
                shape.center
                    + Vec3::new(
                        x * shape.extents.x,
                        y * shape.extents.y,
                        z * shape.extents.z,
                    )
            }
            SpawnShapeKind::BoxShell => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "value is in [0, 6)"
                )]
                let face = (self.sample_unit(particle_id, 100) * 6.0) as u32;
                let u = self.sample_unit(particle_id, 101) * 2.0 - 1.0;
                let v = self.sample_unit(particle_id, 102) * 2.0 - 1.0;
                let offset = match face {
                    0 => Vec3::new(1.0, u, v),
                    1 => Vec3::new(-1.0, u, v),
                    2 => Vec3::new(u, 1.0, v),
                    3 => Vec3::new(u, -1.0, v),
                    4 => Vec3::new(u, v, 1.0),
                    _ => Vec3::new(u, v, -1.0),
                };
                shape.center + offset * shape.extents
            }
            SpawnShapeKind::Cylinder => {
                let angle = self.sample_unit(particle_id, 100) * TAU;
                let r = self.sample_unit(particle_id, 101).sqrt() * shape.extents.x;
                let y = (self.sample_unit(particle_id, 102) * 2.0 - 1.0) * shape.extents.y;
                shape.center + Vec3::new(r * angle.cos(), y, r * angle.sin())
            }
            SpawnShapeKind::Cone => {
                let t = self.sample_unit(particle_id, 100);
                let angle_var = self.sample_unit(particle_id, 101) * TAU;
                let dist = t * shape.extents.y;
                let max_radius = dist * shape.extents.x.tan();
                let r = self.sample_unit(particle_id, 102).sqrt() * max_radius;
                let dir = shape.secondary;
                let (tangent, bitangent) = compute_tangent_frame(dir);
                shape.center
                    + dir * dist
                    + tangent * (r * angle_var.cos())
                    + bitangent * (r * angle_var.sin())
            }
            SpawnShapeKind::Disc => {
                let angle = self.sample_unit(particle_id, 100) * TAU;
                let r = self.sample_unit(particle_id, 101).sqrt() * shape.extents.x;
                let normal = shape.secondary;
                let (tangent, bitangent) = compute_tangent_frame(normal);
                shape.center + tangent * (r * angle.cos()) + bitangent * (r * angle.sin())
            }
            SpawnShapeKind::Line => {
                let t = self.sample_unit(particle_id, 100);
                shape.center + shape.extents * t
            }
        }
    }

    /// Sample initial velocity for a particle.
    #[must_use]
    pub fn sample_velocity(
        &self,
        particle_id: u32,
        config: &EmitterConfig,
        spawn_pos: Vec3,
    ) -> Vec3 {
        let speed = self.sample_range(particle_id, 200, &config.speed);

        match config.velocity_mode {
            VelocityMode::Directional => {
                let base_dir = config.velocity_direction.normalize_or_zero();
                let spread_dir = self.apply_spread(particle_id, base_dir, config.spread_angle);
                spread_dir * speed
            }
            VelocityMode::Radial => {
                let center = config.spawn_shape.center;
                let dir = (spawn_pos - center).normalize_or(Vec3::Y);
                let spread_dir = self.apply_spread(particle_id, dir, config.spread_angle);
                spread_dir * speed
            }
            VelocityMode::Random => {
                let u = self.sample_unit(particle_id, 201);
                let v = self.sample_unit(particle_id, 202);
                let theta = TAU * u;
                let phi = (2.0 * v - 1.0).acos();
                spherical_to_cartesian(speed, theta, phi)
            }
            VelocityMode::Tangential => {
                let center = config.spawn_shape.center;
                let radial = (spawn_pos - center).normalize_or(Vec3::Y);
                let up = Vec3::Y;
                let tangent = radial.cross(up).normalize_or(Vec3::X);
                let spread_dir = self.apply_spread(particle_id, tangent, config.spread_angle);
                spread_dir * speed
            }
        }
    }

    /// Sample initial particle properties.
    #[must_use]
    pub fn sample_particle_properties(
        &self,
        particle_id: u32,
        config: &EmitterConfig,
    ) -> SampledParticle {
        let position = self.sample_spawn_position(particle_id, &config.spawn_shape);
        let velocity = self.sample_velocity(particle_id, config, position);
        let lifetime = self.sample_range(particle_id, 300, &config.lifetime);
        let size = self.sample_range(particle_id, 301, &config.size);
        let rotation = self.sample_range(particle_id, 302, &config.rotation);
        let angular_velocity = self.sample_range(particle_id, 303, &config.angular_velocity);

        SampledParticle {
            position,
            velocity,
            lifetime,
            size,
            rotation,
            angular_velocity,
        }
    }

    fn apply_spread(self, particle_id: u32, direction: Vec3, spread_angle: f32) -> Vec3 {
        if spread_angle < 0.0001 {
            return direction;
        }

        let u = self.sample_unit(particle_id, 210);
        let v = self.sample_unit(particle_id, 211);

        let theta = TAU * u;
        let phi = spread_angle * v.sqrt();

        let (tangent, bitangent) = compute_tangent_frame(direction);
        let offset = tangent * (phi.sin() * theta.cos()) + bitangent * (phi.sin() * theta.sin());
        (direction * phi.cos() + offset).normalize_or(direction)
    }

    fn hash(self, particle_id: u32, property_id: u32) -> u32 {
        let mut n = self.seed;
        n = n.wrapping_add(particle_id.wrapping_mul(HASH_PRIME));
        n = n.wrapping_add(property_id.wrapping_mul(127));
        n = n.wrapping_mul(HASH_MUL_A);
        n ^= n >> 13;
        n = n.wrapping_mul(HASH_MUL_B);
        n ^= n >> 16;
        n
    }
}

/// Sampled particle initial state.
#[derive(Debug, Clone, Copy)]
pub struct SampledParticle {
    /// Initial position.
    pub position: Vec3,
    /// Initial velocity.
    pub velocity: Vec3,
    /// Lifetime in seconds.
    pub lifetime: f32,
    /// Initial size.
    pub size: f32,
    /// Initial rotation (radians).
    pub rotation: f32,
    /// Angular velocity (radians/sec).
    pub angular_velocity: f32,
}

impl Default for SampledParticle {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::Y,
            lifetime: 1.0,
            size: 0.02,
            rotation: 0.0,
            angular_velocity: 0.0,
        }
    }
}

fn hash_to_unit(hash: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "masked to 23 bits; fits in f32 mantissa"
    )]
    let result = (hash & 0x7F_FFFF) as f32 / 0x7F_FFFF_u32 as f32;
    result
}

fn spherical_to_cartesian(radius: f32, theta: f32, phi: f32) -> Vec3 {
    Vec3::new(
        radius * phi.sin() * theta.cos(),
        radius * phi.cos(),
        radius * phi.sin() * theta.sin(),
    )
}

fn compute_tangent_frame(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.y.abs() < 0.999 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let tangent = normal.cross(up).normalize_or(Vec3::X);
    let bitangent = normal.cross(tangent);
    (tangent, bitangent)
}

/// Hash a 3D position for deterministic variation.
#[must_use]
pub fn position_hash(pos: Vec3, seed: u32) -> f32 {
    let xi = pos.x.to_bits();
    let yi = pos.y.to_bits();
    let zi = pos.z.to_bits();
    let mut n = seed
        .wrapping_add(xi)
        .wrapping_add(yi.wrapping_mul(57))
        .wrapping_add(zi.wrapping_mul(113));
    n = n.wrapping_mul(HASH_MUL_A);
    n ^= n >> 13;
    n = n.wrapping_mul(HASH_MUL_B);
    n ^= n >> 16;
    hash_to_unit(n)
}

/// Sample turbulence at a position.
#[must_use]
pub fn sample_turbulence(pos: Vec3, time: f32, frequency: f32, seed: u32) -> Vec3 {
    let fx = pos.x * frequency + time * 0.5;
    let fy = pos.y * frequency + time * 0.3;
    let fz = pos.z * frequency + time * 0.4;

    let nx = position_hash(Vec3::new(fx, fy, fz), seed) * 2.0 - 1.0;
    let ny = position_hash(Vec3::new(fx + 100.0, fy + 100.0, fz + 100.0), seed) * 2.0 - 1.0;
    let nz = position_hash(Vec3::new(fx + 200.0, fy + 200.0, fz + 200.0), seed) * 2.0 - 1.0;

    Vec3::new(nx, ny, nz)
}

/// Plan spawns for a time step (deterministic).
#[must_use]
pub fn plan_spawns(
    config: &EmitterConfig,
    start_id: u32,
    delta_time: f32,
    accumulated: f32,
) -> SpawnPlan {
    let to_spawn = config.spawn_rate * delta_time + accumulated;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "spawn rate and delta_time are non-negative"
    )]
    let count = to_spawn.max(0.0).floor() as u32;
    #[expect(clippy::cast_precision_loss, reason = "count is small in practice")]
    let remaining = to_spawn - count as f32;

    SpawnPlan {
        count,
        start_id,
        accumulated: remaining,
    }
}

/// Result of spawn planning.
#[derive(Debug, Clone, Copy)]
pub struct SpawnPlan {
    /// Number of particles to spawn.
    pub count: u32,
    /// Starting particle ID.
    pub start_id: u32,
    /// Remaining fractional particles for next frame.
    pub accumulated: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sampler_determinism() {
        let sampler = ParticleSampler::new(42);

        let v1 = sampler.sample_unit(0, 0);
        let v2 = sampler.sample_unit(0, 0);

        assert_relative_eq!(v1, v2, epsilon = 0.0001);
    }

    #[test]
    fn test_sampler_different_particles() {
        let sampler = ParticleSampler::new(42);

        let v1 = sampler.sample_unit(0, 0);
        let v2 = sampler.sample_unit(1, 0);

        assert!((v1 - v2).abs() > 0.001, "different particles should differ");
    }

    #[test]
    fn test_sampler_different_properties() {
        let sampler = ParticleSampler::new(42);

        let v1 = sampler.sample_unit(0, 0);
        let v2 = sampler.sample_unit(0, 1);

        assert!(
            (v1 - v2).abs() > 0.001,
            "different properties should differ"
        );
    }

    #[test]
    fn test_sampler_unit_range() {
        let sampler = ParticleSampler::new(0);

        for i in 0..100 {
            let v = sampler.sample_unit(i, 0);
            assert!((0.0..=1.0).contains(&v), "value should be in [0, 1]");
        }
    }

    #[test]
    fn test_sample_range() {
        let sampler = ParticleSampler::new(42);
        let range = ValueRange::range(5.0, 10.0);

        for i in 0..100 {
            let v = sampler.sample_range(i, 0, &range);
            assert!((5.0..=10.0).contains(&v));
        }
    }

    #[test]
    fn test_spawn_position_point() {
        let sampler = ParticleSampler::new(0);
        let shape = SpawnShape::point(Vec3::new(1.0, 2.0, 3.0));

        let pos = sampler.sample_spawn_position(0, &shape);
        assert_eq!(pos, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_spawn_position_sphere_in_bounds() {
        let sampler = ParticleSampler::new(0);
        let shape = SpawnShape::sphere(Vec3::ZERO, 5.0);

        for i in 0..100 {
            let pos = sampler.sample_spawn_position(i, &shape);
            assert!(pos.length() <= 5.01, "particle at {pos:?} outside sphere");
        }
    }

    #[test]
    fn test_spawn_position_box_in_bounds() {
        let sampler = ParticleSampler::new(0);
        let shape = SpawnShape::box_volume(Vec3::ZERO, Vec3::new(2.0, 3.0, 4.0));

        for i in 0..100 {
            let pos = sampler.sample_spawn_position(i, &shape);
            assert!(pos.x.abs() <= 2.01);
            assert!(pos.y.abs() <= 3.01);
            assert!(pos.z.abs() <= 4.01);
        }
    }

    #[test]
    fn test_spawn_position_line() {
        let sampler = ParticleSampler::new(0);
        let shape = SpawnShape::line(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));

        for i in 0..100 {
            let pos = sampler.sample_spawn_position(i, &shape);
            assert!(pos.x >= 0.0 && pos.x <= 10.0);
            assert_relative_eq!(pos.y, 0.0, epsilon = 0.001);
            assert_relative_eq!(pos.z, 0.0, epsilon = 0.001);
        }
    }

    #[test]
    fn test_velocity_directional() {
        let sampler = ParticleSampler::new(0);
        let config = EmitterConfig::default().with_velocity(Vec3::NEG_Y, 5.0, 5.0);

        for i in 0..100 {
            let pos = Vec3::ZERO;
            let vel = sampler.sample_velocity(i, &config, pos);
            assert!(vel.y < 0.0, "velocity should point down");
        }
    }

    #[test]
    fn test_velocity_radial() {
        let sampler = ParticleSampler::new(0);
        let config = EmitterConfig {
            velocity_mode: VelocityMode::Radial,
            spread_angle: 0.0,
            speed: ValueRange::constant(5.0),
            ..Default::default()
        };

        let spawn_pos = Vec3::new(10.0, 0.0, 0.0);
        let vel = sampler.sample_velocity(0, &config, spawn_pos);

        assert!(vel.x > 0.0, "radial velocity should point outward");
    }

    #[test]
    fn test_sampled_particle() {
        let sampler = ParticleSampler::new(42);
        let config = EmitterConfig::default();

        let particle = sampler.sample_particle_properties(0, &config);

        assert!(particle.lifetime > 0.0);
        assert!(particle.size > 0.0);
    }

    #[test]
    fn test_position_hash_determinism() {
        let h1 = position_hash(Vec3::new(1.5, 2.5, 3.5), 42);
        let h2 = position_hash(Vec3::new(1.5, 2.5, 3.5), 42);

        assert_relative_eq!(h1, h2, epsilon = 0.0001);
    }

    #[test]
    fn test_position_hash_range() {
        for i in 0..100 {
            #[expect(
                clippy::cast_precision_loss,
                reason = "small values, precision loss acceptable"
            )]
            let pos = Vec3::new(i as f32, i as f32 * 0.5, i as f32 * 0.3);
            let h = position_hash(pos, 0);
            assert!((0.0..=1.0).contains(&h));
        }
    }

    #[test]
    fn test_turbulence_bounded() {
        for i in 0..50 {
            #[expect(
                clippy::cast_precision_loss,
                reason = "small values, precision loss acceptable"
            )]
            let pos = Vec3::new(i as f32, 0.0, 0.0);
            let turb = sample_turbulence(pos, 0.0, 1.0, 0);
            assert!(turb.x >= -1.0 && turb.x <= 1.0);
            assert!(turb.y >= -1.0 && turb.y <= 1.0);
            assert!(turb.z >= -1.0 && turb.z <= 1.0);
        }
    }

    #[test]
    fn test_plan_spawns() {
        let config = EmitterConfig::default().with_spawn_rate(60.0);

        let plan = plan_spawns(&config, 0, 1.0 / 60.0, 0.0);
        assert_eq!(plan.count, 1);
        assert_relative_eq!(plan.accumulated, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_plan_spawns_accumulation() {
        let config = EmitterConfig::default().with_spawn_rate(30.0);

        let plan1 = plan_spawns(&config, 0, 1.0 / 60.0, 0.0);
        assert_eq!(plan1.count, 0);
        assert!(plan1.accumulated > 0.0);

        let plan2 = plan_spawns(&config, 0, 1.0 / 60.0, plan1.accumulated);
        assert_eq!(plan2.count, 1);
    }

    #[test]
    fn test_spawn_cylinder_in_bounds() {
        let sampler = ParticleSampler::new(0);
        let shape = SpawnShape::cylinder(Vec3::ZERO, 3.0, 5.0);

        for i in 0..100 {
            let pos = sampler.sample_spawn_position(i, &shape);
            let horizontal = Vec3::new(pos.x, 0.0, pos.z).length();
            assert!(horizontal <= 3.01, "horizontal distance {horizontal} > 3.0");
            assert!(pos.y.abs() <= 5.01, "vertical {:.2} out of bounds", pos.y);
        }
    }

    #[test]
    fn test_spawn_disc_flat() {
        let sampler = ParticleSampler::new(0);
        let shape = SpawnShape::disc(Vec3::ZERO, 5.0);

        for i in 0..100 {
            let pos = sampler.sample_spawn_position(i, &shape);
            assert_relative_eq!(pos.y, 0.0, epsilon = 0.01);
            let horizontal = Vec3::new(pos.x, 0.0, pos.z).length();
            assert!(horizontal <= 5.01);
        }
    }
}

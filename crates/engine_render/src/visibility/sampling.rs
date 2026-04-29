//! Deterministic sampling helpers for visibility effects.
//!
//! Provides reproducible noise and falloff calculations for use
//! in both CPU preview and GPU rendering.

use std::f32::consts::TAU;

const HASH_PRIME: u32 = 374_761_393;
const HASH_MUL_A: u32 = 0x85eb_ca6b;
const HASH_MUL_B: u32 = 0xc2b2_ae35;

/// Falloff curve type for visibility attenuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VisibilityFalloff {
    /// Linear falloff (constant rate).
    #[default]
    Linear = 0,
    /// Smooth falloff (ease in-out).
    Smooth = 1,
    /// Exponential falloff (realistic fog).
    Exponential = 2,
    /// Squared exponential (dense fog).
    ExponentialSquared = 3,
    /// Step function (sharp boundary).
    Step = 4,
}

impl VisibilityFalloff {
    /// All falloff types.
    pub const ALL: [Self; 5] = [
        Self::Linear,
        Self::Smooth,
        Self::Exponential,
        Self::ExponentialSquared,
        Self::Step,
    ];

    /// Evaluate the falloff at a normalized distance (0 = no visibility, 1 = full visibility).
    #[must_use]
    pub fn evaluate(self, distance: f32, range: f32) -> f32 {
        if range <= 0.0 {
            return 0.0;
        }
        let t = (distance / range).clamp(0.0, 1.0);
        match self {
            Self::Linear => 1.0 - t,
            Self::Smooth => {
                let t2 = t * t;
                let t3 = t2 * t;
                1.0 - (3.0 * t2 - 2.0 * t3)
            }
            Self::Exponential => (-t * 3.0).exp(),
            Self::ExponentialSquared => (-t * t * 4.0).exp(),
            Self::Step => {
                if t < 0.9 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Deterministic visibility sampler.
///
/// Provides reproducible noise values for visibility effects.
/// The same inputs always produce the same outputs.
#[derive(Debug, Clone, Copy)]
pub struct VisibilitySampler {
    /// Seed for deterministic variation.
    pub seed: u32,
    /// Base frequency of the noise pattern.
    pub frequency: f32,
    /// Number of octaves for multi-scale noise.
    pub octaves: u32,
    /// Amplitude decay per octave (persistence).
    pub persistence: f32,
    /// Frequency multiplier per octave (lacunarity).
    pub lacunarity: f32,
}

impl Default for VisibilitySampler {
    fn default() -> Self {
        Self {
            seed: 0,
            frequency: 2.0,
            octaves: 3,
            persistence: 0.5,
            lacunarity: 2.0,
        }
    }
}

impl VisibilitySampler {
    /// Create a new sampler with the given seed.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Set the frequency.
    #[must_use]
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.max(0.01);
        self
    }

    /// Set the number of octaves.
    #[must_use]
    pub fn with_octaves(mut self, octaves: u32) -> Self {
        self.octaves = octaves.clamp(1, 8);
        self
    }

    /// Set the persistence.
    #[must_use]
    pub fn with_persistence(mut self, persistence: f32) -> Self {
        self.persistence = persistence.clamp(0.0, 1.0);
        self
    }

    /// Set the lacunarity.
    #[must_use]
    pub fn with_lacunarity(mut self, lacunarity: f32) -> Self {
        self.lacunarity = lacunarity.max(1.0);
        self
    }

    /// Sample 2D noise at a position.
    #[must_use]
    pub fn sample_2d(&self, x: f32, y: f32) -> f32 {
        self.sample_fbm_2d(x * self.frequency, y * self.frequency)
    }

    /// Sample 3D noise at a position.
    #[must_use]
    pub fn sample_3d(&self, x: f32, y: f32, z: f32) -> f32 {
        self.sample_fbm_3d(x * self.frequency, y * self.frequency, z * self.frequency)
    }

    /// Sample visibility density at a position with time variation.
    #[must_use]
    pub fn sample_density(&self, x: f32, y: f32, z: f32, time: f32) -> f32 {
        let base = self.sample_3d(x, y, z);
        let temporal = self.sample_2d(x + time * 0.1, z + time * 0.07);
        (base + temporal * 0.3).clamp(0.0, 1.0)
    }

    /// Sample visibility with directional flow (for smoke/fog).
    #[must_use]
    pub fn sample_flow(&self, x: f32, y: f32, z: f32, flow_dir: (f32, f32, f32), time: f32) -> f32 {
        let (fx, fy, fz) = flow_dir;
        let offset = time * 0.5;
        let sx = x - fx * offset;
        let sy = y - fy * offset;
        let sz = z - fz * offset;
        self.sample_3d(sx, sy, sz)
    }

    fn sample_fbm_2d(&self, x: f32, y: f32) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for i in 0..self.octaves {
            total += self.hash_2d(x * frequency, y * frequency, i) * amplitude;
            max_value += amplitude;
            amplitude *= self.persistence;
            frequency *= self.lacunarity;
        }

        total / max_value
    }

    fn sample_fbm_3d(&self, x: f32, y: f32, z: f32) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for i in 0..self.octaves {
            total += self.hash_3d(x * frequency, y * frequency, z * frequency, i) * amplitude;
            max_value += amplitude;
            amplitude *= self.persistence;
            frequency *= self.lacunarity;
        }

        total / max_value
    }

    fn hash_2d(&self, x: f32, y: f32, octave: u32) -> f32 {
        let n = self.seed.wrapping_add(octave).wrapping_mul(HASH_PRIME);
        let xi = float_to_grid(x);
        let yi = float_to_grid(y);
        let tx = x.fract();
        let ty = y.fract();

        let n00 = hash_u32(n.wrapping_add(xi).wrapping_add(yi.wrapping_mul(57)));
        let n10 = hash_u32(
            n.wrapping_add(xi.wrapping_add(1))
                .wrapping_add(yi.wrapping_mul(57)),
        );
        let n01 = hash_u32(
            n.wrapping_add(xi)
                .wrapping_add(yi.wrapping_add(1).wrapping_mul(57)),
        );
        let n11 = hash_u32(
            n.wrapping_add(xi.wrapping_add(1))
                .wrapping_add(yi.wrapping_add(1).wrapping_mul(57)),
        );

        let v00 = u32_to_unit(n00);
        let v10 = u32_to_unit(n10);
        let v01 = u32_to_unit(n01);
        let v11 = u32_to_unit(n11);

        let ix0 = smoothstep_lerp(v00, v10, tx);
        let ix1 = smoothstep_lerp(v01, v11, tx);

        smoothstep_lerp(ix0, ix1, ty)
    }

    fn hash_3d(&self, x: f32, y: f32, z: f32, octave: u32) -> f32 {
        let n = self.seed.wrapping_add(octave).wrapping_mul(HASH_PRIME);
        let xi = float_to_grid(x);
        let yi = float_to_grid(y);
        let zi = float_to_grid(z);
        let tx = x.fract();
        let ty = y.fract();
        let tz = z.fract();

        let corner = |dx: u32, dy: u32, dz: u32| {
            let hash = n
                .wrapping_add(xi.wrapping_add(dx))
                .wrapping_add(yi.wrapping_add(dy).wrapping_mul(57))
                .wrapping_add(zi.wrapping_add(dz).wrapping_mul(113));
            u32_to_unit(hash_u32(hash))
        };

        let v000 = corner(0, 0, 0);
        let v100 = corner(1, 0, 0);
        let v010 = corner(0, 1, 0);
        let v110 = corner(1, 1, 0);
        let v001 = corner(0, 0, 1);
        let v101 = corner(1, 0, 1);
        let v011 = corner(0, 1, 1);
        let v111 = corner(1, 1, 1);

        let ix00 = smoothstep_lerp(v000, v100, tx);
        let ix10 = smoothstep_lerp(v010, v110, tx);
        let ix01 = smoothstep_lerp(v001, v101, tx);
        let ix11 = smoothstep_lerp(v011, v111, tx);

        let iy0 = smoothstep_lerp(ix00, ix10, ty);
        let iy1 = smoothstep_lerp(ix01, ix11, ty);

        smoothstep_lerp(iy0, iy1, tz)
    }
}

fn float_to_grid(v: f32) -> u32 {
    v.floor().to_bits()
}

fn hash_u32(mut n: u32) -> u32 {
    n = n.wrapping_mul(HASH_MUL_A);
    n ^= n >> 13;
    n = n.wrapping_mul(HASH_MUL_B);
    n ^= n >> 16;
    n
}

#[expect(
    clippy::cast_precision_loss,
    reason = "masked to 23 bits; fits in f32 mantissa"
)]
fn u32_to_unit(n: u32) -> f32 {
    (n & 0x7F_FFFF) as f32 / 0x7F_FFFF_u32 as f32
}

fn smoothstep_lerp(a: f32, b: f32, t: f32) -> f32 {
    let t = t * t * (3.0 - 2.0 * t);
    a + (b - a) * t
}

/// Compute visibility factor based on distance and fog density.
#[must_use]
pub fn visibility_from_distance(distance: f32, density: f32) -> f32 {
    if density <= 0.0 {
        return 1.0;
    }
    (-distance * density).exp()
}

/// Compute visibility with squared exponential falloff (realistic thick fog).
#[must_use]
pub fn visibility_squared_exp(distance: f32, density: f32) -> f32 {
    if density <= 0.0 {
        return 1.0;
    }
    let d = distance * density;
    (-d * d).exp()
}

/// Compute contrast enhancement factor for bioluminescence.
#[must_use]
pub fn bioluminescent_factor(base_visibility: f32, contrast: f32, emissive_strength: f32) -> f32 {
    let darkness = 1.0 - base_visibility;
    let glow_boost = darkness * contrast * emissive_strength;
    (base_visibility + glow_boost).clamp(0.0, 1.0)
}

/// Compute depth-based visibility (closer objects more visible).
#[must_use]
pub fn depth_visibility(depth: f32, near: f32, far: f32) -> f32 {
    if depth <= near {
        1.0
    } else if depth >= far {
        0.0
    } else {
        let t = (depth - near) / (far - near);
        1.0 - t * t
    }
}

/// Hash a position to a deterministic value (0-1).
#[must_use]
pub fn position_hash_3d(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let xi = x.to_bits();
    let yi = y.to_bits();
    let zi = z.to_bits();
    let mut n = seed
        .wrapping_add(xi)
        .wrapping_add(yi.wrapping_mul(57))
        .wrapping_add(zi.wrapping_mul(113));
    n = n.wrapping_mul(HASH_MUL_A);
    n ^= n >> 13;
    n = n.wrapping_mul(HASH_MUL_B);
    n ^= n >> 16;
    u32_to_unit(n)
}

/// Compute pulsing effect for bioluminescent sources.
#[must_use]
pub fn bioluminescent_pulse(time: f32, frequency: f32, phase: f32) -> f32 {
    let wave = ((time * frequency + phase) * TAU).sin();
    (wave + 1.0) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_falloff_linear() {
        assert_relative_eq!(
            VisibilityFalloff::Linear.evaluate(0.0, 10.0),
            1.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            VisibilityFalloff::Linear.evaluate(5.0, 10.0),
            0.5,
            epsilon = 0.001
        );
        assert_relative_eq!(
            VisibilityFalloff::Linear.evaluate(10.0, 10.0),
            0.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_falloff_smooth() {
        let smooth = VisibilityFalloff::Smooth;
        assert_relative_eq!(smooth.evaluate(0.0, 10.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(smooth.evaluate(10.0, 10.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(smooth.evaluate(5.0, 10.0), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_falloff_exponential() {
        let exp = VisibilityFalloff::Exponential;
        assert_relative_eq!(exp.evaluate(0.0, 10.0), 1.0, epsilon = 0.001);
        assert!(exp.evaluate(5.0, 10.0) > 0.0);
        assert!(exp.evaluate(10.0, 10.0) < 0.1);
    }

    #[test]
    fn test_falloff_step() {
        let step = VisibilityFalloff::Step;
        assert_relative_eq!(step.evaluate(5.0, 10.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(step.evaluate(9.5, 10.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_falloff_zero_range() {
        for falloff in VisibilityFalloff::ALL {
            assert_relative_eq!(falloff.evaluate(5.0, 0.0), 0.0, epsilon = 0.001);
        }
    }

    #[test]
    fn test_sampler_determinism() {
        let sampler = VisibilitySampler::new(42);

        let v1 = sampler.sample_2d(1.5, 2.5);
        let v2 = sampler.sample_2d(1.5, 2.5);

        assert_relative_eq!(v1, v2, epsilon = 0.0001);
    }

    #[test]
    fn test_sampler_seed_variation() {
        let sampler1 = VisibilitySampler::new(42);
        let sampler2 = VisibilitySampler::new(123);

        let v1 = sampler1.sample_2d(1.0, 1.0);
        let v2 = sampler2.sample_2d(1.0, 1.0);

        assert!(
            (v1 - v2).abs() > 0.01,
            "different seeds should produce different values"
        );
    }

    #[test]
    fn test_sampler_range() {
        let sampler = VisibilitySampler::new(0);

        for i in 0i16..100 {
            let x = f32::from(i) * 0.1;
            let y = f32::from(i) * 0.13;
            let v = sampler.sample_2d(x, y);
            assert!((0.0..=1.0).contains(&v), "noise should be in [0, 1]");
        }
    }

    #[test]
    fn test_sampler_3d() {
        let sampler = VisibilitySampler::new(0);
        let v1 = sampler.sample_3d(1.0, 2.0, 3.0);
        let v2 = sampler.sample_3d(1.0, 2.0, 3.0);
        assert_relative_eq!(v1, v2, epsilon = 0.0001);
        assert!((0.0..=1.0).contains(&v1));
    }

    #[test]
    fn test_sample_density() {
        let sampler = VisibilitySampler::new(0);
        let d1 = sampler.sample_density(1.0, 2.0, 3.0, 0.0);
        let d2 = sampler.sample_density(1.0, 2.0, 3.0, 1.0);

        assert!((0.0..=1.0).contains(&d1));
        assert!((0.0..=1.0).contains(&d2));
    }

    #[test]
    fn test_sample_flow() {
        let sampler = VisibilitySampler::new(0);
        let f1 = sampler.sample_flow(0.0, 0.0, 0.0, (0.0, 1.0, 0.0), 0.0);
        let f2 = sampler.sample_flow(0.0, 0.0, 0.0, (0.0, 1.0, 0.0), 1.0);

        assert!((0.0..=1.0).contains(&f1));
        assert!((0.0..=1.0).contains(&f2));
    }

    #[test]
    fn test_visibility_from_distance() {
        assert_relative_eq!(visibility_from_distance(0.0, 0.1), 1.0, epsilon = 0.001);
        assert!(visibility_from_distance(10.0, 0.1) < 1.0);
        assert!(visibility_from_distance(10.0, 0.1) > 0.0);
    }

    #[test]
    fn test_visibility_from_distance_zero_density() {
        assert_relative_eq!(visibility_from_distance(100.0, 0.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_visibility_squared_exp() {
        assert_relative_eq!(visibility_squared_exp(0.0, 0.1), 1.0, epsilon = 0.001);
        assert!(visibility_squared_exp(5.0, 0.1) < 1.0);
    }

    #[test]
    fn test_bioluminescent_factor() {
        let base = 0.2;
        let enhanced = bioluminescent_factor(base, 2.0, 0.5);
        assert!(enhanced > base, "bioluminescence should enhance visibility");
        assert!(enhanced <= 1.0);
    }

    #[test]
    fn test_depth_visibility() {
        assert_relative_eq!(depth_visibility(5.0, 10.0, 100.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(depth_visibility(10.0, 10.0, 100.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(depth_visibility(100.0, 10.0, 100.0), 0.0, epsilon = 0.001);
        assert!(depth_visibility(50.0, 10.0, 100.0) > 0.0);
        assert!(depth_visibility(50.0, 10.0, 100.0) < 1.0);
    }

    #[test]
    fn test_position_hash_3d_determinism() {
        let h1 = position_hash_3d(1.5, 2.5, 3.5, 42);
        let h2 = position_hash_3d(1.5, 2.5, 3.5, 42);
        assert_relative_eq!(h1, h2, epsilon = 0.0001);
    }

    #[test]
    fn test_position_hash_3d_range() {
        for i in 0u16..100 {
            let h = position_hash_3d(
                f32::from(i) * 0.1,
                f32::from(i) * 0.17,
                f32::from(i) * 0.23,
                u32::from(i),
            );
            assert!((0.0..=1.0).contains(&h));
        }
    }

    #[test]
    fn test_bioluminescent_pulse() {
        let p1 = bioluminescent_pulse(0.0, 1.0, 0.0);
        let p2 = bioluminescent_pulse(0.25, 1.0, 0.0);

        assert!((0.0..=1.0).contains(&p1));
        assert!((0.0..=1.0).contains(&p2));
    }

    #[test]
    fn test_sampler_octaves() {
        let single = VisibilitySampler::new(0).with_octaves(1);
        let multi = VisibilitySampler::new(0).with_octaves(4);

        let v1 = single.sample_2d(0.5, 0.5);
        let v2 = multi.sample_2d(0.5, 0.5);

        assert!((0.0..=1.0).contains(&v1));
        assert!((0.0..=1.0).contains(&v2));
    }
}

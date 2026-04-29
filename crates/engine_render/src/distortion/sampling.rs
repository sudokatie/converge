//! Deterministic sampling helpers for distortion effects.
//!
//! Provides reproducible noise and falloff calculations for use
//! in both CPU preview and GPU rendering.

use std::f32::consts::TAU;

const HASH_PRIME: u32 = 374_761_393;
const HASH_MUL_A: u32 = 0x85eb_ca6b;
const HASH_MUL_B: u32 = 0xc2b2_ae35;

/// Falloff curve type for spatial attenuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FalloffCurve {
    /// Linear falloff (constant rate).
    #[default]
    Linear = 0,
    /// Smooth falloff (ease in-out).
    Smooth = 1,
    /// Quadratic falloff (fast start, slow end).
    Quadratic = 2,
    /// Exponential falloff (sharp boundary).
    Exponential = 3,
    /// Inverse square (physically-based).
    InverseSquare = 4,
}

impl FalloffCurve {
    /// All falloff curves.
    pub const ALL: [Self; 5] = [
        Self::Linear,
        Self::Smooth,
        Self::Quadratic,
        Self::Exponential,
        Self::InverseSquare,
    ];

    /// Evaluate the falloff at a normalized distance (0 = center, 1 = edge).
    #[must_use]
    pub fn evaluate(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => 1.0 - t,
            Self::Smooth => {
                let t2 = t * t;
                let t3 = t2 * t;
                1.0 - (3.0 * t2 - 2.0 * t3)
            }
            Self::Quadratic => (1.0 - t) * (1.0 - t),
            Self::Exponential => (-t * 4.0).exp(),
            Self::InverseSquare => 1.0 / (1.0 + t * t * 4.0),
        }
    }
}

/// Deterministic distortion sampler.
///
/// Provides reproducible noise values for distortion effects.
/// The same inputs always produce the same outputs.
#[derive(Debug, Clone, Copy)]
pub struct DistortionSampler {
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

impl Default for DistortionSampler {
    fn default() -> Self {
        Self {
            seed: 0,
            frequency: 4.0,
            octaves: 3,
            persistence: 0.5,
            lacunarity: 2.0,
        }
    }
}

impl DistortionSampler {
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

    /// Sample 1D noise at a position.
    #[must_use]
    pub fn sample_1d(&self, x: f32) -> f32 {
        self.sample_fbm_1d(x * self.frequency)
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

    /// Sample 2D distortion offset (returns x, y displacement).
    #[must_use]
    pub fn sample_offset_2d(&self, x: f32, y: f32, time: f32) -> (f32, f32) {
        let fx = x * self.frequency;
        let fy = y * self.frequency;

        let offset_x = self.sample_fbm_2d(fx + time * 0.5, fy + 100.0);
        let offset_y = self.sample_fbm_2d(fx + 200.0, fy + time * 0.5);

        (offset_x, offset_y)
    }

    /// Sample radial distortion (returns magnitude, angle).
    #[must_use]
    pub fn sample_radial(&self, x: f32, y: f32, cx: f32, cy: f32, time: f32) -> (f32, f32) {
        let dx = x - cx;
        let dy = y - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);

        let wave = ((dist * self.frequency - time) * TAU).sin();
        let noise = self.sample_2d(x, y);

        let magnitude = wave * (1.0 + noise * 0.3);
        let angle_offset = noise * 0.2;

        (magnitude, angle + angle_offset)
    }

    fn sample_fbm_1d(&self, x: f32) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for i in 0..self.octaves {
            total += self.hash_1d(x * frequency, i) * amplitude;
            max_value += amplitude;
            amplitude *= self.persistence;
            frequency *= self.lacunarity;
        }

        total / max_value
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

    #[expect(
        clippy::many_single_char_names,
        reason = "conventional variable names for hash interpolation"
    )]
    fn hash_1d(&self, x: f32, octave: u32) -> f32 {
        let n = self.seed.wrapping_add(octave).wrapping_mul(HASH_PRIME);
        let xi = float_to_grid(x);
        let t = x.fract();

        let a = hash_u32(n.wrapping_add(xi));
        let b = hash_u32(n.wrapping_add(xi.wrapping_add(1)));

        let ta = u32_to_unit(a);
        let tb = u32_to_unit(b);

        smoothstep_lerp(ta, tb, t)
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

/// Compute linear falloff between inner and outer radius.
#[must_use]
pub fn linear_falloff(distance: f32, inner_radius: f32, outer_radius: f32) -> f32 {
    if distance <= inner_radius {
        1.0
    } else if distance >= outer_radius {
        0.0
    } else {
        1.0 - (distance - inner_radius) / (outer_radius - inner_radius)
    }
}

/// Compute smooth (cubic) falloff between inner and outer radius.
#[must_use]
pub fn smooth_falloff(distance: f32, inner_radius: f32, outer_radius: f32) -> f32 {
    let t = linear_falloff(distance, inner_radius, outer_radius);
    t * t * (3.0 - 2.0 * t)
}

/// Compute exponential falloff with a given decay rate.
#[must_use]
pub fn exponential_falloff(distance: f32, decay: f32) -> f32 {
    (-distance * decay).exp()
}

/// Compute sinusoidal wave with phase.
#[must_use]
pub fn sine_wave(position: f32, frequency: f32, phase: f32, time: f32, speed: f32) -> f32 {
    ((position * frequency + phase + time * speed) * TAU).sin()
}

/// Compute radial wave pattern.
#[must_use]
pub fn radial_wave(x: f32, y: f32, cx: f32, cy: f32, frequency: f32, time: f32, speed: f32) -> f32 {
    let dx = x - cx;
    let dy = y - cy;
    let dist = (dx * dx + dy * dy).sqrt();
    ((dist * frequency - time * speed) * TAU).sin()
}

/// Compute spiral pattern.
#[must_use]
pub fn spiral_wave(x: f32, y: f32, cx: f32, cy: f32, arms: f32, tightness: f32, time: f32) -> f32 {
    let dx = x - cx;
    let dy = y - cy;
    let angle = dy.atan2(dx);
    let dist = (dx * dx + dy * dy).sqrt();
    ((angle * arms + dist * tightness - time) % TAU).sin()
}

/// Hash a position to a deterministic value (0-1).
#[must_use]
pub fn position_hash(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.to_bits();
    let yi = y.to_bits();
    let mut n = seed.wrapping_add(xi).wrapping_add(yi.wrapping_mul(57));
    n = n.wrapping_mul(HASH_MUL_A);
    n ^= n >> 13;
    n = n.wrapping_mul(HASH_MUL_B);
    n ^= n >> 16;
    #[expect(
        clippy::cast_precision_loss,
        reason = "masked to 23 bits; fits in f32 mantissa"
    )]
    let result = (n & 0x7F_FFFF) as f32 / 0x7F_FFFF_u32 as f32;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::PI;

    #[test]
    fn test_falloff_curve_linear() {
        assert_relative_eq!(FalloffCurve::Linear.evaluate(0.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(FalloffCurve::Linear.evaluate(0.5), 0.5, epsilon = 0.001);
        assert_relative_eq!(FalloffCurve::Linear.evaluate(1.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_falloff_curve_smooth() {
        let smooth = FalloffCurve::Smooth;
        assert_relative_eq!(smooth.evaluate(0.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(smooth.evaluate(1.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(smooth.evaluate(0.5), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_falloff_curve_clamping() {
        for curve in FalloffCurve::ALL {
            assert!(curve.evaluate(-1.0) <= 1.0);
            assert!(curve.evaluate(2.0) >= 0.0);
        }
    }

    #[test]
    fn test_sampler_determinism() {
        let sampler = DistortionSampler::new(42);

        let v1 = sampler.sample_2d(1.5, 2.5);
        let v2 = sampler.sample_2d(1.5, 2.5);

        assert_relative_eq!(v1, v2, epsilon = 0.0001);
    }

    #[test]
    fn test_sampler_seed_variation() {
        let sampler1 = DistortionSampler::new(42);
        let sampler2 = DistortionSampler::new(123);

        let v1 = sampler1.sample_2d(1.0, 1.0);
        let v2 = sampler2.sample_2d(1.0, 1.0);

        assert!(
            (v1 - v2).abs() > 0.01,
            "different seeds should produce different values"
        );
    }

    #[test]
    fn test_sampler_range() {
        let sampler = DistortionSampler::new(0);

        for i in 0i16..100 {
            let x = f32::from(i) * 0.1;
            let y = f32::from(i) * 0.13;
            let v = sampler.sample_2d(x, y);
            assert!((0.0..=1.0).contains(&v), "noise should be in [0, 1]");
        }
    }

    #[test]
    fn test_sampler_offset_2d() {
        let sampler = DistortionSampler::new(0);
        let (ox, oy) = sampler.sample_offset_2d(0.5, 0.5, 0.0);

        assert!(ox.abs() <= 1.0);
        assert!(oy.abs() <= 1.0);
    }

    #[test]
    fn test_sampler_radial() {
        let sampler = DistortionSampler::new(0);
        let (mag, _angle) = sampler.sample_radial(1.0, 0.0, 0.0, 0.0, 0.0);

        assert!(mag.abs() <= 2.0, "radial magnitude should be bounded");
    }

    #[test]
    fn test_linear_falloff_inside() {
        assert_relative_eq!(linear_falloff(0.0, 5.0, 10.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(linear_falloff(3.0, 5.0, 10.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_linear_falloff_outside() {
        assert_relative_eq!(linear_falloff(15.0, 5.0, 10.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_linear_falloff_transition() {
        assert_relative_eq!(linear_falloff(7.5, 5.0, 10.0), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_smooth_falloff() {
        let s = smooth_falloff(7.5, 5.0, 10.0);
        assert!(s > 0.0 && s < 1.0);
        assert_relative_eq!(smooth_falloff(0.0, 5.0, 10.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(smooth_falloff(15.0, 5.0, 10.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_exponential_falloff() {
        assert_relative_eq!(exponential_falloff(0.0, 1.0), 1.0, epsilon = 0.001);
        assert!(exponential_falloff(1.0, 1.0) < 1.0);
        assert!(exponential_falloff(5.0, 1.0) < 0.01);
    }

    #[test]
    fn test_sine_wave() {
        let wave = sine_wave(0.0, 1.0, 0.0, 0.0, 1.0);
        assert_relative_eq!(wave, 0.0, epsilon = 0.001);

        let wave = sine_wave(0.25, 1.0, 0.0, 0.0, 1.0);
        assert_relative_eq!(wave, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_radial_wave() {
        let wave = radial_wave(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0);
        assert_relative_eq!(wave, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_spiral_wave_bounded() {
        for i in 0i16..20 {
            let angle = f32::from(i) * PI / 10.0;
            let x = angle.cos();
            let y = angle.sin();
            let wave = spiral_wave(x, y, 0.0, 0.0, 3.0, 1.0, 0.0);
            assert!((-1.0..=1.0).contains(&wave));
        }
    }

    #[test]
    fn test_position_hash_determinism() {
        let h1 = position_hash(1.5, 2.5, 42);
        let h2 = position_hash(1.5, 2.5, 42);
        assert_relative_eq!(h1, h2, epsilon = 0.0001);
    }

    #[test]
    fn test_position_hash_range() {
        for i in 0u16..100 {
            let h = position_hash(f32::from(i) * 0.1, f32::from(i) * 0.17, u32::from(i));
            assert!((0.0..=1.0).contains(&h));
        }
    }

    #[test]
    fn test_sampler_1d() {
        let sampler = DistortionSampler::new(0);
        let v1 = sampler.sample_1d(0.0);
        let v2 = sampler.sample_1d(0.0);
        assert_relative_eq!(v1, v2, epsilon = 0.0001);
        assert!((0.0..=1.0).contains(&v1));
    }

    #[test]
    fn test_sampler_3d() {
        let sampler = DistortionSampler::new(0);
        let v1 = sampler.sample_3d(1.0, 2.0, 3.0);
        let v2 = sampler.sample_3d(1.0, 2.0, 3.0);
        assert_relative_eq!(v1, v2, epsilon = 0.0001);
        assert!((0.0..=1.0).contains(&v1));
    }

    #[test]
    fn test_sampler_octaves() {
        let single = DistortionSampler::new(0).with_octaves(1);
        let multi = DistortionSampler::new(0).with_octaves(4);

        let v1 = single.sample_2d(0.5, 0.5);
        let v2 = multi.sample_2d(0.5, 0.5);

        assert!((0.0..=1.0).contains(&v1));
        assert!((0.0..=1.0).contains(&v2));
    }
}

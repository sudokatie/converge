//! Light shaft (god ray) configuration.
//!
//! Light shafts appear when volumetric media scatters directional light,
//! creating visible beams through fog, water, or dust.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Configuration for light shaft rendering.
#[derive(Debug, Clone, Copy)]
pub struct LightShaftConfig {
    /// Number of ray marching samples (higher = better quality, slower).
    pub sample_count: u32,
    /// Maximum ray distance in world units.
    pub max_distance: f32,
    /// Intensity multiplier for shaft brightness.
    pub intensity: f32,
    /// Decay factor per sample step.
    pub decay: f32,
    /// Exposure for HDR tone mapping.
    pub exposure: f32,
}

impl Default for LightShaftConfig {
    fn default() -> Self {
        Self {
            sample_count: 64,
            max_distance: 100.0,
            intensity: 1.0,
            decay: 0.96,
            exposure: 1.0,
        }
    }
}

impl LightShaftConfig {
    /// Create config optimized for underwater caustics.
    #[must_use]
    pub fn underwater() -> Self {
        Self {
            sample_count: 48,
            max_distance: 50.0,
            intensity: 0.8,
            decay: 0.94,
            exposure: 1.2,
        }
    }

    /// Create config for interior dust motes.
    #[must_use]
    pub fn interior_dust() -> Self {
        Self {
            sample_count: 32,
            max_distance: 30.0,
            intensity: 0.5,
            decay: 0.98,
            exposure: 0.8,
        }
    }

    /// Create config for exterior sun shafts.
    #[must_use]
    pub fn exterior_sun() -> Self {
        Self {
            sample_count: 96,
            max_distance: 200.0,
            intensity: 1.2,
            decay: 0.97,
            exposure: 1.0,
        }
    }

    /// Scale quality settings (0.0 = minimum, 1.0 = maximum).
    #[must_use]
    pub fn with_quality(mut self, quality: f32) -> Self {
        let quality = quality.clamp(0.0, 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "quality is clamped to [0,1], result is always valid u32"
        )]
        let sample_count = (16.0 + quality * 112.0) as u32;
        self.sample_count = sample_count;
        self.max_distance *= 0.5 + quality * 0.5;
        self
    }
}

/// A single light shaft instance.
#[derive(Debug, Clone, Copy)]
pub struct LightShaft {
    /// World position of light source.
    pub position: Vec3,
    /// Direction of light (normalized).
    pub direction: Vec3,
    /// Light color (linear RGB).
    pub color: Vec3,
    /// Angular radius of the light source in radians.
    pub angular_radius: f32,
    /// Configuration for rendering.
    pub config: LightShaftConfig,
}

impl Default for LightShaft {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 100.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
            color: Vec3::splat(1.0),
            angular_radius: 0.01,
            config: LightShaftConfig::default(),
        }
    }
}

impl LightShaft {
    /// Create a light shaft from a directional light source.
    #[must_use]
    pub fn from_directional(direction: Vec3, color: Vec3) -> Self {
        Self {
            position: Vec3::ZERO,
            direction: direction.normalize(),
            color,
            angular_radius: 0.02,
            config: LightShaftConfig::exterior_sun(),
        }
    }

    /// Create a light shaft from a point light source.
    #[must_use]
    pub fn from_point(position: Vec3, color: Vec3, radius: f32) -> Self {
        Self {
            position,
            direction: Vec3::NEG_Y,
            color,
            angular_radius: radius,
            config: LightShaftConfig::interior_dust(),
        }
    }

    /// Check if this shaft is from a directional (infinite) light.
    #[must_use]
    pub fn is_directional(&self) -> bool {
        self.position == Vec3::ZERO
    }
}

/// GPU-friendly light shaft uniform.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightShaftUniform {
    /// Position (XYZ) + angular radius (W).
    pub position_radius: [f32; 4],
    /// Direction (XYZ) + intensity (W).
    pub direction_intensity: [f32; 4],
    /// Color (RGB) + decay (A).
    pub color_decay: [f32; 4],
    /// Sample count, max distance, exposure, padding.
    pub params: [f32; 4],
}

impl From<LightShaft> for LightShaftUniform {
    fn from(shaft: LightShaft) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "sample_count is small; precision loss acceptable"
        )]
        let sample_count = shaft.config.sample_count as f32;

        Self {
            position_radius: [
                shaft.position.x,
                shaft.position.y,
                shaft.position.z,
                shaft.angular_radius,
            ],
            direction_intensity: [
                shaft.direction.x,
                shaft.direction.y,
                shaft.direction.z,
                shaft.config.intensity,
            ],
            color_decay: [
                shaft.color.x,
                shaft.color.y,
                shaft.color.z,
                shaft.config.decay,
            ],
            params: [
                sample_count,
                shaft.config.max_distance,
                shaft.config.exposure,
                0.0,
            ],
        }
    }
}

impl Default for LightShaftUniform {
    fn default() -> Self {
        LightShaft::default().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_config_valid() {
        let config = LightShaftConfig::default();
        assert!(config.sample_count >= 16);
        assert!(config.max_distance > 0.0);
        assert!(config.decay > 0.0 && config.decay < 1.0);
    }

    #[test]
    fn test_quality_scaling() {
        let low = LightShaftConfig::default().with_quality(0.0);
        let high = LightShaftConfig::default().with_quality(1.0);

        assert!(low.sample_count < high.sample_count);
        assert!(low.max_distance < high.max_distance);
    }

    #[test]
    fn test_directional_shaft() {
        let shaft = LightShaft::from_directional(Vec3::new(0.5, -1.0, 0.3), Vec3::ONE);
        assert!(shaft.is_directional());
        assert_relative_eq!(shaft.direction.length(), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_point_shaft() {
        let shaft = LightShaft::from_point(Vec3::new(10.0, 5.0, 10.0), Vec3::ONE, 0.5);
        assert!(!shaft.is_directional());
        assert_relative_eq!(shaft.angular_radius, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_uniform_conversion() {
        let shaft = LightShaft::from_directional(Vec3::NEG_Y, Vec3::new(1.0, 0.9, 0.8));
        let uniform: LightShaftUniform = shaft.into();

        assert_relative_eq!(uniform.direction_intensity[1], -1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.color_decay[0], 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<LightShaftUniform>() % 16,
            0,
            "uniform should be 16-byte aligned for GPU"
        );
    }

    #[test]
    fn test_preset_configs_valid() {
        for config in [
            LightShaftConfig::underwater(),
            LightShaftConfig::interior_dust(),
            LightShaftConfig::exterior_sun(),
        ] {
            assert!(config.sample_count >= 16);
            assert!(config.decay > 0.9 && config.decay < 1.0);
            assert!(config.intensity > 0.0);
        }
    }
}

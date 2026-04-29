//! Screen-space visibility overlay configuration.
//!
//! Controls how visibility effects are composited onto the final image,
//! including quality settings, blend modes, and temporal behavior.

use glam::Vec3;
use std::f32::consts::TAU;

/// Quality/budget level for visibility rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VisibilityQuality {
    /// Minimal samples, fastest.
    Low = 0,
    /// Balanced quality and performance.
    #[default]
    Medium = 1,
    /// High sample count, best quality.
    High = 2,
    /// Maximum quality, expensive.
    Ultra = 3,
}

impl VisibilityQuality {
    /// All quality levels.
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// Get sample count for this quality level.
    #[must_use]
    pub fn sample_count(self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 8,
            Self::High => 16,
            Self::Ultra => 32,
        }
    }

    /// Get resolution divisor (1 = full, 2 = half, etc.).
    #[must_use]
    pub fn resolution_divisor(self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 2,
            Self::High | Self::Ultra => 1,
        }
    }
}

/// Blend mode for compositing visibility with the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VisibilityBlendMode {
    /// Simple multiplicative darkening.
    #[default]
    Darken = 0,
    /// Additive brightening (for whiteout).
    Brighten = 1,
    /// Color tint overlay.
    ColorTint = 2,
    /// Contrast enhancement (for bioluminescence).
    ContrastBoost = 3,
    /// Fog-style depth blending.
    DepthFog = 4,
}

impl VisibilityBlendMode {
    /// All blend modes.
    pub const ALL: [Self; 5] = [
        Self::Darken,
        Self::Brighten,
        Self::ColorTint,
        Self::ContrastBoost,
        Self::DepthFog,
    ];
}

/// Screen-space visibility configuration.
#[derive(Debug, Clone, Copy)]
pub struct ScreenVisibility {
    /// Overall effect strength (0.0 to 1.0).
    pub strength: f32,
    /// Color shift/tint (linear RGB).
    pub color_shift: Vec3,
    /// Vignette strength (0.0 = none, 1.0 = heavy).
    pub vignette_strength: f32,
    /// Vignette radius (normalized, 0.0 = full screen).
    pub vignette_radius: f32,
    /// Noise scale for procedural variation.
    pub noise_scale: f32,
    /// Noise animation speed.
    pub noise_speed: f32,
    /// Center position for radial effects (normalized 0-1).
    pub center: (f32, f32),
    /// Temporal phase offset (0.0 to TAU).
    pub phase: f32,
    /// Quality/budget setting.
    pub quality: VisibilityQuality,
    /// Blend mode for compositing.
    pub blend_mode: VisibilityBlendMode,
    /// Contrast adjustment (-1.0 to 1.0).
    pub contrast_adjust: f32,
    /// Whether this visibility effect is enabled.
    pub enabled: bool,
}

impl Default for ScreenVisibility {
    fn default() -> Self {
        Self {
            strength: 0.5,
            color_shift: Vec3::ONE,
            vignette_strength: 0.3,
            vignette_radius: 0.7,
            noise_scale: 0.1,
            noise_speed: 0.2,
            center: (0.5, 0.5),
            phase: 0.0,
            quality: VisibilityQuality::Medium,
            blend_mode: VisibilityBlendMode::Darken,
            contrast_adjust: 0.0,
            enabled: true,
        }
    }
}

impl ScreenVisibility {
    /// Create a darkness screen visibility.
    #[must_use]
    pub fn darkness() -> Self {
        Self {
            strength: 1.0,
            color_shift: Vec3::ZERO,
            vignette_strength: 0.0,
            vignette_radius: 0.0,
            noise_scale: 0.0,
            noise_speed: 0.0,
            blend_mode: VisibilityBlendMode::Darken,
            contrast_adjust: 0.0,
            ..Default::default()
        }
    }

    /// Create a whiteout screen visibility.
    #[must_use]
    pub fn whiteout() -> Self {
        Self {
            strength: 0.95,
            color_shift: Vec3::new(0.98, 0.98, 1.0),
            vignette_strength: 0.1,
            vignette_radius: 0.9,
            noise_scale: 0.2,
            noise_speed: 0.3,
            blend_mode: VisibilityBlendMode::Brighten,
            contrast_adjust: -0.3,
            ..Default::default()
        }
    }

    /// Create a murk screen visibility.
    #[must_use]
    pub fn murk() -> Self {
        Self {
            strength: 0.7,
            color_shift: Vec3::new(0.3, 0.35, 0.25),
            vignette_strength: 0.4,
            vignette_radius: 0.6,
            noise_scale: 0.3,
            noise_speed: 0.1,
            blend_mode: VisibilityBlendMode::ColorTint,
            contrast_adjust: -0.2,
            ..Default::default()
        }
    }

    /// Create a smoke screen visibility.
    #[must_use]
    pub fn smoke() -> Self {
        Self {
            strength: 0.6,
            color_shift: Vec3::new(0.4, 0.4, 0.45),
            vignette_strength: 0.5,
            vignette_radius: 0.5,
            noise_scale: 0.5,
            noise_speed: 0.8,
            blend_mode: VisibilityBlendMode::DepthFog,
            contrast_adjust: -0.15,
            ..Default::default()
        }
    }

    /// Create a bioluminescent screen visibility.
    #[must_use]
    pub fn bioluminescent() -> Self {
        Self {
            strength: 0.85,
            color_shift: Vec3::new(0.05, 0.02, 0.08),
            vignette_strength: 0.6,
            vignette_radius: 0.4,
            noise_scale: 0.15,
            noise_speed: 0.2,
            blend_mode: VisibilityBlendMode::ContrastBoost,
            contrast_adjust: 0.5,
            ..Default::default()
        }
    }

    /// Set strength.
    #[must_use]
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Set color shift.
    #[must_use]
    pub fn with_color_shift(mut self, color: Vec3) -> Self {
        self.color_shift = color;
        self
    }

    /// Set vignette strength.
    #[must_use]
    pub fn with_vignette_strength(mut self, strength: f32) -> Self {
        self.vignette_strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Set vignette radius.
    #[must_use]
    pub fn with_vignette_radius(mut self, radius: f32) -> Self {
        self.vignette_radius = radius.clamp(0.0, 1.0);
        self
    }

    /// Set noise scale.
    #[must_use]
    pub fn with_noise_scale(mut self, scale: f32) -> Self {
        self.noise_scale = scale.clamp(0.0, 1.0);
        self
    }

    /// Set noise speed.
    #[must_use]
    pub fn with_noise_speed(mut self, speed: f32) -> Self {
        self.noise_speed = speed.max(0.0);
        self
    }

    /// Set center position.
    #[must_use]
    pub fn with_center(mut self, x: f32, y: f32) -> Self {
        self.center = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        self
    }

    /// Set phase offset.
    #[must_use]
    pub fn with_phase(mut self, phase: f32) -> Self {
        self.phase = phase % TAU;
        self
    }

    /// Set quality level.
    #[must_use]
    pub fn with_quality(mut self, quality: VisibilityQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Set blend mode.
    #[must_use]
    pub fn with_blend_mode(mut self, mode: VisibilityBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set contrast adjustment.
    #[must_use]
    pub fn with_contrast_adjust(mut self, adjust: f32) -> Self {
        self.contrast_adjust = adjust.clamp(-1.0, 1.0);
        self
    }

    /// Enable or disable.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Compute vignette factor at a screen position.
    #[must_use]
    pub fn compute_vignette(&self, uv: (f32, f32)) -> f32 {
        if self.vignette_strength <= 0.0 {
            return 1.0;
        }

        let (u, v) = uv;
        let (cx, cy) = self.center;
        let dx = u - cx;
        let dy = v - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= self.vignette_radius {
            1.0
        } else {
            let falloff_range = 1.0 - self.vignette_radius;
            if falloff_range <= 0.0 {
                1.0 - self.vignette_strength
            } else {
                let t = ((dist - self.vignette_radius) / falloff_range).min(1.0);
                1.0 - self.vignette_strength * t * t
            }
        }
    }

    /// Compute noise value at a screen position and time.
    #[must_use]
    pub fn compute_noise(&self, uv: (f32, f32), time: f32) -> f32 {
        if self.noise_scale <= 0.0 {
            return 0.0;
        }

        let (u, v) = uv;
        let phase = time * self.noise_speed + self.phase;
        let noise_x = (u * 10.0 + phase).sin() * (v * 7.3).cos();
        let noise_y = (v * 8.5 + phase * 0.7).cos() * (u * 11.2).sin();
        let combined = (noise_x + noise_y) * 0.5;
        combined * self.noise_scale
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.strength = self.strength.clamp(0.0, 1.0);
        self.vignette_strength = self.vignette_strength.clamp(0.0, 1.0);
        self.vignette_radius = self.vignette_radius.clamp(0.0, 1.0);
        self.noise_scale = self.noise_scale.clamp(0.0, 1.0);
        self.noise_speed = self.noise_speed.max(0.0);
        self.center.0 = self.center.0.clamp(0.0, 1.0);
        self.center.1 = self.center.1.clamp(0.0, 1.0);
        self.phase = self.phase.rem_euclid(TAU);
        self.contrast_adjust = self.contrast_adjust.clamp(-1.0, 1.0);
        self
    }

    /// Check if values are valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.strength >= 0.0
            && self.strength <= 1.0
            && self.vignette_strength >= 0.0
            && self.vignette_strength <= 1.0
            && self.vignette_radius >= 0.0
            && self.vignette_radius <= 1.0
            && self.noise_scale >= 0.0
            && self.noise_scale <= 1.0
            && self.noise_speed >= 0.0
            && self.center.0 >= 0.0
            && self.center.0 <= 1.0
            && self.center.1 >= 0.0
            && self.center.1 <= 1.0
            && self.contrast_adjust >= -1.0
            && self.contrast_adjust <= 1.0
    }

    /// Interpolate between two configurations.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            strength: self.strength + (other.strength - self.strength) * t,
            color_shift: self.color_shift.lerp(other.color_shift, t),
            vignette_strength: self.vignette_strength
                + (other.vignette_strength - self.vignette_strength) * t,
            vignette_radius: self.vignette_radius
                + (other.vignette_radius - self.vignette_radius) * t,
            noise_scale: self.noise_scale + (other.noise_scale - self.noise_scale) * t,
            noise_speed: self.noise_speed + (other.noise_speed - self.noise_speed) * t,
            center: (
                self.center.0 + (other.center.0 - self.center.0) * t,
                self.center.1 + (other.center.1 - self.center.1) * t,
            ),
            phase: self.phase + (other.phase - self.phase) * t,
            quality: if t < 0.5 { self.quality } else { other.quality },
            blend_mode: if t < 0.5 {
                self.blend_mode
            } else {
                other.blend_mode
            },
            contrast_adjust: self.contrast_adjust
                + (other.contrast_adjust - self.contrast_adjust) * t,
            enabled: if t < 0.5 { self.enabled } else { other.enabled },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_quality_sample_counts() {
        assert!(VisibilityQuality::Low.sample_count() < VisibilityQuality::Ultra.sample_count());
        assert!(VisibilityQuality::Medium.sample_count() >= 8);
    }

    #[test]
    fn test_quality_resolution() {
        assert!(
            VisibilityQuality::Low.resolution_divisor()
                > VisibilityQuality::High.resolution_divisor()
        );
    }

    #[test]
    fn test_darkness_preset() {
        let screen = ScreenVisibility::darkness();
        assert_relative_eq!(screen.strength, 1.0, epsilon = 0.001);
        assert_eq!(screen.blend_mode, VisibilityBlendMode::Darken);
        assert!(screen.is_valid());
    }

    #[test]
    fn test_whiteout_preset() {
        let screen = ScreenVisibility::whiteout();
        assert!(screen.strength > 0.9);
        assert_eq!(screen.blend_mode, VisibilityBlendMode::Brighten);
        assert!(screen.is_valid());
    }

    #[test]
    fn test_bioluminescent_preset() {
        let screen = ScreenVisibility::bioluminescent();
        assert_eq!(screen.blend_mode, VisibilityBlendMode::ContrastBoost);
        assert!(screen.contrast_adjust > 0.0);
        assert!(screen.is_valid());
    }

    #[test]
    fn test_compute_vignette_center() {
        let screen = ScreenVisibility::default().with_vignette_strength(0.5);
        let vignette = screen.compute_vignette((0.5, 0.5));
        assert_relative_eq!(vignette, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_compute_vignette_edge() {
        let screen = ScreenVisibility::default()
            .with_vignette_strength(0.8)
            .with_vignette_radius(0.3);
        let vignette = screen.compute_vignette((0.0, 0.0));
        assert!(vignette < 1.0);
        assert!(vignette > 0.0);
    }

    #[test]
    fn test_compute_vignette_no_vignette() {
        let screen = ScreenVisibility::default().with_vignette_strength(0.0);
        let vignette = screen.compute_vignette((0.0, 0.0));
        assert_relative_eq!(vignette, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_compute_noise_deterministic() {
        let screen = ScreenVisibility::default().with_noise_scale(0.5);
        let n1 = screen.compute_noise((0.3, 0.7), 1.0);
        let n2 = screen.compute_noise((0.3, 0.7), 1.0);
        assert_relative_eq!(n1, n2, epsilon = 0.0001);
    }

    #[test]
    fn test_compute_noise_bounded() {
        let screen = ScreenVisibility::default().with_noise_scale(1.0);
        let samples: [f32; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        for s in samples {
            let u = s * 0.1;
            let v = s * 0.13;
            let noise = screen.compute_noise((u, v), s);
            assert!(noise.abs() <= 1.0);
        }
    }

    #[test]
    fn test_compute_noise_zero_scale() {
        let screen = ScreenVisibility::default().with_noise_scale(0.0);
        let noise = screen.compute_noise((0.5, 0.5), 1.0);
        assert_relative_eq!(noise, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = ScreenVisibility::darkness();
        let b = ScreenVisibility::whiteout();

        let at_a = a.lerp(b, 0.0);
        assert_relative_eq!(at_a.strength, a.strength, epsilon = 0.001);

        let at_b = a.lerp(b, 1.0);
        assert_relative_eq!(at_b.strength, b.strength, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = ScreenVisibility::default().with_strength(0.2);
        let b = ScreenVisibility::default().with_strength(0.8);

        let mid = a.lerp(b, 0.5);
        assert_relative_eq!(mid.strength, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_clamping() {
        let screen = ScreenVisibility {
            strength: 2.0,
            vignette_strength: -0.5,
            noise_scale: 5.0,
            center: (2.0, -1.0),
            contrast_adjust: 3.0,
            ..Default::default()
        }
        .clamped();

        assert!(screen.is_valid());
        assert_relative_eq!(screen.strength, 1.0, epsilon = 0.001);
        assert_relative_eq!(screen.vignette_strength, 0.0, epsilon = 0.001);
        assert_relative_eq!(screen.noise_scale, 1.0, epsilon = 0.001);
        assert_relative_eq!(screen.center.0, 1.0, epsilon = 0.001);
        assert_relative_eq!(screen.center.1, 0.0, epsilon = 0.001);
        assert_relative_eq!(screen.contrast_adjust, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_builder_chain() {
        let screen = ScreenVisibility::default()
            .with_strength(0.7)
            .with_noise_scale(0.4)
            .with_quality(VisibilityQuality::High)
            .with_blend_mode(VisibilityBlendMode::ColorTint);

        assert_relative_eq!(screen.strength, 0.7, epsilon = 0.001);
        assert_relative_eq!(screen.noise_scale, 0.4, epsilon = 0.001);
        assert_eq!(screen.quality, VisibilityQuality::High);
        assert_eq!(screen.blend_mode, VisibilityBlendMode::ColorTint);
    }

    #[test]
    fn test_all_presets_valid() {
        for screen in [
            ScreenVisibility::darkness(),
            ScreenVisibility::whiteout(),
            ScreenVisibility::murk(),
            ScreenVisibility::smoke(),
            ScreenVisibility::bioluminescent(),
        ] {
            assert!(screen.is_valid(), "preset should be valid");
            assert!(screen.enabled, "preset should be enabled");
        }
    }
}

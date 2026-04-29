//! Post-processing effect definitions.
//!
//! Each effect kind represents a distinct post-processing operation
//! that can be applied per-environment or globally.

use glam::Vec3;

/// Kind of post-processing effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PostEffectKind {
    #[default]
    /// Color grading/correction.
    ColorGrade = 0,
    /// Bloom/glow extraction.
    Bloom = 1,
    /// Tone mapping (HDR to LDR).
    ToneMap = 2,
    /// Vignette darkening.
    Vignette = 3,
    /// Chromatic aberration.
    ChromaticAberration = 4,
    /// Film grain noise.
    FilmGrain = 5,
    /// Depth of field blur.
    DepthOfField = 6,
    /// Motion blur.
    MotionBlur = 7,
}

impl PostEffectKind {
    /// All effect kinds in order.
    pub const ALL: [Self; 8] = [
        Self::ColorGrade,
        Self::Bloom,
        Self::ToneMap,
        Self::Vignette,
        Self::ChromaticAberration,
        Self::FilmGrain,
        Self::DepthOfField,
        Self::MotionBlur,
    ];

    /// Default execution order for this effect (lower runs first).
    #[must_use]
    pub fn default_order(self) -> u8 {
        match self {
            Self::DepthOfField => 10,
            Self::MotionBlur => 20,
            Self::Bloom => 30,
            Self::ToneMap => 40,
            Self::ColorGrade => 50,
            Self::ChromaticAberration => 60,
            Self::Vignette => 70,
            Self::FilmGrain => 80,
        }
    }

    /// Whether this effect requires depth buffer access.
    #[must_use]
    pub fn requires_depth(self) -> bool {
        matches!(self, Self::DepthOfField | Self::MotionBlur)
    }

    /// Whether this effect is typically full-screen only.
    #[must_use]
    pub fn is_global(self) -> bool {
        matches!(self, Self::ToneMap | Self::MotionBlur)
    }
}

/// Configuration for a post-processing effect.
#[derive(Debug, Clone, Copy)]
pub struct PostEffect {
    /// Effect type.
    pub kind: PostEffectKind,
    /// Effect intensity (0.0 = off, 1.0 = full).
    pub intensity: f32,
    /// Primary color parameter (usage varies by effect).
    pub color: Vec3,
    /// Threshold for bloom/glow extraction.
    pub threshold: f32,
    /// Radius/size parameter (blur radius, grain size, etc.).
    pub radius: f32,
    /// Secondary parameter (varies by effect).
    pub secondary: f32,
    /// Whether this effect is currently active.
    pub active: bool,
}

impl PostEffect {
    /// Create an effect with default parameters for the given kind.
    #[must_use]
    pub fn from_kind(kind: PostEffectKind) -> Self {
        match kind {
            PostEffectKind::ColorGrade => Self::color_grade(),
            PostEffectKind::Bloom => Self::bloom(),
            PostEffectKind::ToneMap => Self::tone_map(),
            PostEffectKind::Vignette => Self::vignette(),
            PostEffectKind::ChromaticAberration => Self::chromatic_aberration(),
            PostEffectKind::FilmGrain => Self::film_grain(),
            PostEffectKind::DepthOfField => Self::depth_of_field(),
            PostEffectKind::MotionBlur => Self::motion_blur(),
        }
    }

    /// Color grading effect.
    #[must_use]
    pub fn color_grade() -> Self {
        Self {
            kind: PostEffectKind::ColorGrade,
            intensity: 1.0,
            color: Vec3::ONE,
            threshold: 0.0,
            radius: 0.0,
            secondary: 1.0,
            active: true,
        }
    }

    /// Bloom effect with default threshold.
    #[must_use]
    pub fn bloom() -> Self {
        Self {
            kind: PostEffectKind::Bloom,
            intensity: 0.5,
            color: Vec3::ONE,
            threshold: 0.8,
            radius: 4.0,
            secondary: 0.0,
            active: true,
        }
    }

    /// Tone mapping effect.
    #[must_use]
    pub fn tone_map() -> Self {
        Self {
            kind: PostEffectKind::ToneMap,
            intensity: 1.0,
            color: Vec3::ONE,
            threshold: 0.0,
            radius: 0.0,
            secondary: 2.2,
            active: true,
        }
    }

    /// Vignette effect.
    #[must_use]
    pub fn vignette() -> Self {
        Self {
            kind: PostEffectKind::Vignette,
            intensity: 0.4,
            color: Vec3::ZERO,
            threshold: 0.0,
            radius: 0.7,
            secondary: 0.3,
            active: true,
        }
    }

    /// Chromatic aberration effect.
    #[must_use]
    pub fn chromatic_aberration() -> Self {
        Self {
            kind: PostEffectKind::ChromaticAberration,
            intensity: 0.3,
            color: Vec3::new(1.0, 0.0, -1.0),
            threshold: 0.0,
            radius: 0.02,
            secondary: 0.0,
            active: true,
        }
    }

    /// Film grain effect.
    #[must_use]
    pub fn film_grain() -> Self {
        Self {
            kind: PostEffectKind::FilmGrain,
            intensity: 0.15,
            color: Vec3::ONE,
            threshold: 0.0,
            radius: 1.0,
            secondary: 24.0,
            active: true,
        }
    }

    /// Depth of field effect.
    #[must_use]
    pub fn depth_of_field() -> Self {
        Self {
            kind: PostEffectKind::DepthOfField,
            intensity: 1.0,
            color: Vec3::ONE,
            threshold: 10.0,
            radius: 5.0,
            secondary: 2.0,
            active: true,
        }
    }

    /// Motion blur effect.
    #[must_use]
    pub fn motion_blur() -> Self {
        Self {
            kind: PostEffectKind::MotionBlur,
            intensity: 0.5,
            color: Vec3::ONE,
            threshold: 0.0,
            radius: 8.0,
            secondary: 0.0,
            active: true,
        }
    }

    /// Set intensity.
    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set color parameter.
    #[must_use]
    pub fn with_color(mut self, color: Vec3) -> Self {
        self.color = color;
        self
    }

    /// Set threshold.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.max(0.0);
        self
    }

    /// Set radius.
    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    /// Set secondary parameter.
    #[must_use]
    pub fn with_secondary(mut self, secondary: f32) -> Self {
        self.secondary = secondary;
        self
    }

    /// Set active state.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Interpolate between two effects.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            kind: if t < 0.5 { self.kind } else { other.kind },
            intensity: self.intensity + (other.intensity - self.intensity) * t,
            color: self.color.lerp(other.color, t),
            threshold: self.threshold + (other.threshold - self.threshold) * t,
            radius: self.radius + (other.radius - self.radius) * t,
            secondary: self.secondary + (other.secondary - self.secondary) * t,
            active: if t < 0.5 { self.active } else { other.active },
        }
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.intensity = self.intensity.clamp(0.0, 1.0);
        self.threshold = self.threshold.max(0.0);
        self.radius = self.radius.max(0.0);
        self
    }

    /// Check if values are within valid ranges.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.intensity >= 0.0
            && self.intensity <= 1.0
            && self.threshold >= 0.0
            && self.radius >= 0.0
    }

    /// Compute effective intensity considering active state.
    #[must_use]
    pub fn effective_intensity(&self) -> f32 {
        if self.active { self.intensity } else { 0.0 }
    }
}

impl Default for PostEffect {
    fn default() -> Self {
        Self::color_grade()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_all_kinds_have_constructors() {
        for kind in PostEffectKind::ALL {
            let effect = PostEffect::from_kind(kind);
            assert_eq!(effect.kind, kind);
            assert!(effect.is_valid());
        }
    }

    #[test]
    fn test_bloom_threshold() {
        let effect = PostEffect::bloom();
        assert!(effect.threshold > 0.0, "bloom should have threshold");
        assert!(effect.intensity > 0.0);
    }

    #[test]
    fn test_vignette_params() {
        let effect = PostEffect::vignette();
        assert!(effect.radius > 0.0, "vignette should have radius");
        assert!(effect.intensity > 0.0);
    }

    #[test]
    fn test_depth_requires_depth() {
        assert!(PostEffectKind::DepthOfField.requires_depth());
        assert!(PostEffectKind::MotionBlur.requires_depth());
        assert!(!PostEffectKind::Bloom.requires_depth());
    }

    #[test]
    fn test_global_effects() {
        assert!(PostEffectKind::ToneMap.is_global());
        assert!(PostEffectKind::MotionBlur.is_global());
        assert!(!PostEffectKind::Vignette.is_global());
    }

    #[test]
    fn test_default_order() {
        assert!(
            PostEffectKind::DepthOfField.default_order() < PostEffectKind::Bloom.default_order()
        );
        assert!(PostEffectKind::Bloom.default_order() < PostEffectKind::ToneMap.default_order());
        assert!(
            PostEffectKind::ToneMap.default_order() < PostEffectKind::FilmGrain.default_order()
        );
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = PostEffect::bloom();
        let b = PostEffect::vignette();

        let at_a = a.lerp(b, 0.0);
        assert_relative_eq!(at_a.intensity, a.intensity, epsilon = 0.001);

        let at_b = a.lerp(b, 1.0);
        assert_relative_eq!(at_b.intensity, b.intensity, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = PostEffect::bloom().with_intensity(0.2);
        let b = PostEffect::bloom().with_intensity(0.8);

        let mid = a.lerp(b, 0.5);
        assert_relative_eq!(mid.intensity, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_clamping() {
        let effect = PostEffect::bloom()
            .with_intensity(2.0)
            .with_threshold(-1.0)
            .with_radius(-5.0)
            .clamped();

        assert!(effect.is_valid());
        assert_relative_eq!(effect.intensity, 1.0, epsilon = 0.001);
        assert_relative_eq!(effect.threshold, 0.0, epsilon = 0.001);
        assert_relative_eq!(effect.radius, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_effective_intensity_inactive() {
        let effect = PostEffect::bloom().with_active(false);
        assert_relative_eq!(effect.effective_intensity(), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_effective_intensity_active() {
        let effect = PostEffect::bloom().with_intensity(0.7);
        assert_relative_eq!(effect.effective_intensity(), 0.7, epsilon = 0.001);
    }

    #[test]
    fn test_builder_chain() {
        let effect = PostEffect::bloom()
            .with_intensity(0.8)
            .with_threshold(0.9)
            .with_radius(6.0)
            .with_color(Vec3::new(1.0, 0.9, 0.8));

        assert_relative_eq!(effect.intensity, 0.8, epsilon = 0.001);
        assert_relative_eq!(effect.threshold, 0.9, epsilon = 0.001);
        assert_relative_eq!(effect.radius, 6.0, epsilon = 0.001);
    }
}

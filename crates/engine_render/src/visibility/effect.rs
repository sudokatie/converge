//! Visibility effect definitions.
//!
//! Each effect kind has distinct visual properties: obscurance level,
//! color tint, contrast behavior, and animation parameters.

use glam::Vec3;

/// Kind of visibility effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VisibilityKind {
    /// Total darkness with no ambient light.
    Darkness = 0,
    /// Blinding white conditions (snow, fog, bright light).
    Whiteout = 1,
    /// Murky conditions (underwater sediment, swamp).
    Murk = 2,
    /// Smoke obscurance with volumetric behavior.
    Smoke = 3,
    /// Bioluminescent contrast (glowing sources in darkness).
    BioluminescentContrast = 4,
}

impl VisibilityKind {
    /// All visibility kinds in order.
    pub const ALL: [Self; 5] = [
        Self::Darkness,
        Self::Whiteout,
        Self::Murk,
        Self::Smoke,
        Self::BioluminescentContrast,
    ];

    /// Get the default obscurance for this kind.
    #[must_use]
    pub fn default_obscurance(self) -> f32 {
        match self {
            Self::Darkness => 1.0,
            Self::Whiteout => 0.95,
            Self::Murk => 0.7,
            Self::Smoke => 0.6,
            Self::BioluminescentContrast => 0.85,
        }
    }

    /// Whether this effect type affects color perception.
    #[must_use]
    pub fn affects_color(self) -> bool {
        matches!(
            self,
            Self::Whiteout | Self::Murk | Self::BioluminescentContrast
        )
    }

    /// Whether this effect has emissive sources.
    #[must_use]
    pub fn has_emissive(self) -> bool {
        matches!(self, Self::BioluminescentContrast)
    }
}

/// Configuration for a visibility effect.
#[derive(Debug, Clone, Copy)]
pub struct VisibilityEffect {
    /// Effect type.
    pub kind: VisibilityKind,
    /// Base tint color (linear RGB).
    pub color: Vec3,
    /// Obscurance level (0.0 = clear, 1.0 = fully obscured).
    pub obscurance: f32,
    /// Contrast multiplier (affects how light sources stand out).
    pub contrast: f32,
    /// Visibility range in world units (0 = no visibility).
    pub visibility_range: f32,
    /// Animation speed multiplier for dynamic effects.
    pub animation_speed: f32,
    /// Noise intensity for procedural variation.
    pub noise_intensity: f32,
    /// Whether the effect is currently active.
    pub active: bool,
}

impl VisibilityEffect {
    /// Create an effect with default parameters for the given kind.
    #[must_use]
    pub fn from_kind(kind: VisibilityKind) -> Self {
        match kind {
            VisibilityKind::Darkness => Self::darkness(),
            VisibilityKind::Whiteout => Self::whiteout(),
            VisibilityKind::Murk => Self::murk(),
            VisibilityKind::Smoke => Self::smoke(),
            VisibilityKind::BioluminescentContrast => Self::bioluminescent_contrast(),
        }
    }

    /// Total darkness effect.
    #[must_use]
    pub fn darkness() -> Self {
        Self {
            kind: VisibilityKind::Darkness,
            color: Vec3::ZERO,
            obscurance: 1.0,
            contrast: 0.0,
            visibility_range: 0.0,
            animation_speed: 0.0,
            noise_intensity: 0.0,
            active: true,
        }
    }

    /// Whiteout/blinding conditions effect.
    #[must_use]
    pub fn whiteout() -> Self {
        Self {
            kind: VisibilityKind::Whiteout,
            color: Vec3::new(0.98, 0.98, 1.0),
            obscurance: 0.95,
            contrast: 0.1,
            visibility_range: 5.0,
            animation_speed: 0.3,
            noise_intensity: 0.2,
            active: true,
        }
    }

    /// Murky underwater/swamp effect.
    #[must_use]
    pub fn murk() -> Self {
        Self {
            kind: VisibilityKind::Murk,
            color: Vec3::new(0.3, 0.35, 0.25),
            obscurance: 0.7,
            contrast: 0.3,
            visibility_range: 15.0,
            animation_speed: 0.1,
            noise_intensity: 0.4,
            active: true,
        }
    }

    /// Smoke cloud effect.
    #[must_use]
    pub fn smoke() -> Self {
        Self {
            kind: VisibilityKind::Smoke,
            color: Vec3::new(0.4, 0.4, 0.45),
            obscurance: 0.6,
            contrast: 0.2,
            visibility_range: 10.0,
            animation_speed: 0.8,
            noise_intensity: 0.6,
            active: true,
        }
    }

    /// Bioluminescent contrast effect (glowing in darkness).
    #[must_use]
    pub fn bioluminescent_contrast() -> Self {
        Self {
            kind: VisibilityKind::BioluminescentContrast,
            color: Vec3::new(0.05, 0.02, 0.08),
            obscurance: 0.85,
            contrast: 2.5,
            visibility_range: 3.0,
            animation_speed: 0.2,
            noise_intensity: 0.15,
            active: true,
        }
    }

    /// Set obscurance level.
    #[must_use]
    pub fn with_obscurance(mut self, obscurance: f32) -> Self {
        self.obscurance = obscurance.clamp(0.0, 1.0);
        self
    }

    /// Set contrast multiplier.
    #[must_use]
    pub fn with_contrast(mut self, contrast: f32) -> Self {
        self.contrast = contrast.clamp(0.0, 10.0);
        self
    }

    /// Set visibility range.
    #[must_use]
    pub fn with_visibility_range(mut self, range: f32) -> Self {
        self.visibility_range = range.max(0.0);
        self
    }

    /// Set animation speed.
    #[must_use]
    pub fn with_animation_speed(mut self, speed: f32) -> Self {
        self.animation_speed = speed.max(0.0);
        self
    }

    /// Set noise intensity.
    #[must_use]
    pub fn with_noise_intensity(mut self, intensity: f32) -> Self {
        self.noise_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set active state.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Set tint color.
    #[must_use]
    pub fn with_color(mut self, color: Vec3) -> Self {
        self.color = color;
        self
    }

    /// Interpolate between two effects.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            kind: if t < 0.5 { self.kind } else { other.kind },
            color: self.color.lerp(other.color, t),
            obscurance: self.obscurance + (other.obscurance - self.obscurance) * t,
            contrast: self.contrast + (other.contrast - self.contrast) * t,
            visibility_range: self.visibility_range
                + (other.visibility_range - self.visibility_range) * t,
            animation_speed: self.animation_speed
                + (other.animation_speed - self.animation_speed) * t,
            noise_intensity: self.noise_intensity
                + (other.noise_intensity - self.noise_intensity) * t,
            active: if t < 0.5 { self.active } else { other.active },
        }
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.obscurance = self.obscurance.clamp(0.0, 1.0);
        self.contrast = self.contrast.clamp(0.0, 10.0);
        self.visibility_range = self.visibility_range.max(0.0);
        self.animation_speed = self.animation_speed.max(0.0);
        self.noise_intensity = self.noise_intensity.clamp(0.0, 1.0);
        self
    }

    /// Check if values are within valid ranges.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.obscurance >= 0.0
            && self.obscurance <= 1.0
            && self.contrast >= 0.0
            && self.contrast <= 10.0
            && self.visibility_range >= 0.0
            && self.animation_speed >= 0.0
            && self.noise_intensity >= 0.0
            && self.noise_intensity <= 1.0
    }

    /// Compute effective obscurance at a given time.
    #[must_use]
    pub fn obscurance_at_time(&self, time: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let variation = if self.noise_intensity > 0.0 && self.animation_speed > 0.0 {
            let phase = time * self.animation_speed;
            self.noise_intensity * 0.1 * phase.sin()
        } else {
            0.0
        };
        (self.obscurance + variation).clamp(0.0, 1.0)
    }
}

impl Default for VisibilityEffect {
    fn default() -> Self {
        Self::smoke()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_all_kinds_have_constructors() {
        for kind in VisibilityKind::ALL {
            let effect = VisibilityEffect::from_kind(kind);
            assert_eq!(effect.kind, kind);
            assert!(effect.is_valid());
        }
    }

    #[test]
    fn test_darkness_is_total() {
        let effect = VisibilityEffect::darkness();
        assert_relative_eq!(effect.obscurance, 1.0, epsilon = 0.001);
        assert_relative_eq!(effect.visibility_range, 0.0, epsilon = 0.001);
        assert_relative_eq!(effect.contrast, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_whiteout_is_bright() {
        let effect = VisibilityEffect::whiteout();
        assert!(effect.color.x > 0.9, "whiteout should be bright");
        assert!(effect.obscurance > 0.9, "whiteout should heavily obscure");
    }

    #[test]
    fn test_bioluminescent_high_contrast() {
        let effect = VisibilityEffect::bioluminescent_contrast();
        assert!(
            effect.contrast > 1.0,
            "bioluminescent should have high contrast"
        );
        assert!(VisibilityKind::BioluminescentContrast.has_emissive());
    }

    #[test]
    fn test_smoke_animated() {
        let effect = VisibilityEffect::smoke();
        assert!(effect.animation_speed > 0.0, "smoke should be animated");
        assert!(effect.noise_intensity > 0.0, "smoke should have noise");
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = VisibilityEffect::darkness();
        let b = VisibilityEffect::whiteout();

        let at_a = a.lerp(b, 0.0);
        assert_relative_eq!(at_a.obscurance, a.obscurance, epsilon = 0.001);

        let at_b = a.lerp(b, 1.0);
        assert_relative_eq!(at_b.obscurance, b.obscurance, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = VisibilityEffect::smoke();
        let b = VisibilityEffect::murk();

        let mid = a.lerp(b, 0.5);
        let expected = f32::midpoint(a.obscurance, b.obscurance);
        assert_relative_eq!(mid.obscurance, expected, epsilon = 0.001);
    }

    #[test]
    fn test_clamping() {
        let effect = VisibilityEffect::smoke()
            .with_obscurance(2.0)
            .with_contrast(-1.0)
            .with_noise_intensity(5.0)
            .clamped();

        assert!(effect.is_valid());
        assert_relative_eq!(effect.obscurance, 1.0, epsilon = 0.001);
        assert_relative_eq!(effect.contrast, 0.0, epsilon = 0.001);
        assert_relative_eq!(effect.noise_intensity, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_obscurance_at_time_inactive() {
        let effect = VisibilityEffect::smoke().with_active(false);
        assert_relative_eq!(effect.obscurance_at_time(1.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_obscurance_at_time_variation() {
        let effect = VisibilityEffect::smoke();
        let o1 = effect.obscurance_at_time(0.0);
        let o2 = effect.obscurance_at_time(1.0);

        assert!(o1 > 0.0);
        assert!(o2 > 0.0);
        assert!(
            (o1 - o2).abs() < effect.obscurance,
            "variation should be subtle"
        );
    }

    #[test]
    fn test_kind_properties() {
        assert!(VisibilityKind::Whiteout.affects_color());
        assert!(VisibilityKind::Murk.affects_color());
        assert!(!VisibilityKind::Darkness.affects_color());

        assert!(VisibilityKind::BioluminescentContrast.has_emissive());
        assert!(!VisibilityKind::Smoke.has_emissive());
    }

    #[test]
    fn test_default_obscurance() {
        for kind in VisibilityKind::ALL {
            let obs = kind.default_obscurance();
            assert!((0.0..=1.0).contains(&obs));
        }
    }
}

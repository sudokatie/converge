//! Preset visibility configurations.
//!
//! Ready-to-use visibility effect configurations for common scenarios.

use super::{
    ScreenVisibility, VisibilityEffect, VisibilityKind, VisibilityQuality, VisibilityRegion,
};
use glam::Vec3;

/// Preset configurations for common visibility scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VisibilityPreset {
    /// Cave darkness with no ambient light.
    CaveDarkness = 0,
    /// Blizzard whiteout conditions.
    BlizzardWhiteout = 1,
    /// Underwater murk/sediment.
    UnderwaterMurk = 2,
    /// Fire/explosion smoke.
    FireSmoke = 3,
    /// Deep sea bioluminescence.
    DeepSeaBioluminescence = 4,
    /// Sandstorm visibility.
    Sandstorm = 5,
    /// Volcanic ash cloud.
    VolcanicAsh = 6,
    /// Swamp fog.
    SwampFog = 7,
}

impl VisibilityPreset {
    /// All presets.
    pub const ALL: [Self; 8] = [
        Self::CaveDarkness,
        Self::BlizzardWhiteout,
        Self::UnderwaterMurk,
        Self::FireSmoke,
        Self::DeepSeaBioluminescence,
        Self::Sandstorm,
        Self::VolcanicAsh,
        Self::SwampFog,
    ];

    /// Get the visibility effect for this preset.
    #[must_use]
    pub fn effect(self) -> VisibilityEffect {
        match self {
            Self::CaveDarkness => VisibilityEffect::darkness(),
            Self::BlizzardWhiteout => VisibilityEffect::whiteout()
                .with_noise_intensity(0.3)
                .with_animation_speed(0.5),
            Self::UnderwaterMurk => VisibilityEffect::murk()
                .with_visibility_range(12.0)
                .with_color(Vec3::new(0.25, 0.35, 0.3)),
            Self::FireSmoke => VisibilityEffect::smoke()
                .with_obscurance(0.75)
                .with_animation_speed(1.2)
                .with_color(Vec3::new(0.3, 0.3, 0.32)),
            Self::DeepSeaBioluminescence => VisibilityEffect::bioluminescent_contrast()
                .with_contrast(3.0)
                .with_color(Vec3::new(0.02, 0.01, 0.05)),
            Self::Sandstorm => VisibilityEffect::from_kind(VisibilityKind::Murk)
                .with_obscurance(0.8)
                .with_color(Vec3::new(0.6, 0.5, 0.35))
                .with_noise_intensity(0.5)
                .with_animation_speed(1.5),
            Self::VolcanicAsh => VisibilityEffect::smoke()
                .with_obscurance(0.85)
                .with_color(Vec3::new(0.25, 0.22, 0.2))
                .with_noise_intensity(0.4),
            Self::SwampFog => VisibilityEffect::murk()
                .with_obscurance(0.5)
                .with_visibility_range(20.0)
                .with_color(Vec3::new(0.35, 0.4, 0.3))
                .with_animation_speed(0.05),
        }
    }

    /// Get a default region for this preset.
    #[must_use]
    pub fn default_region(self, center: Vec3) -> VisibilityRegion {
        match self {
            Self::CaveDarkness => {
                VisibilityRegion::new_box(center, Vec3::new(50.0, 20.0, 50.0)).with_falloff(5.0)
            }
            Self::BlizzardWhiteout => {
                VisibilityRegion::new_half_space(center.y + 100.0).with_falloff(10.0)
            }
            Self::UnderwaterMurk => VisibilityRegion::new_half_space(center.y)
                .with_falloff(8.0)
                .with_gradient_strength(0.3)
                .with_gradient_direction(Vec3::NEG_Y),
            Self::FireSmoke => VisibilityRegion::new_sphere(center, 15.0)
                .with_falloff(5.0)
                .with_gradient_strength(0.5)
                .with_gradient_direction(Vec3::Y),
            Self::DeepSeaBioluminescence => VisibilityRegion::new_half_space(center.y - 50.0)
                .with_falloff(15.0)
                .with_gradient_strength(0.4),
            Self::Sandstorm => {
                VisibilityRegion::new_cylinder(center, 100.0, 30.0).with_falloff(20.0)
            }
            Self::VolcanicAsh => VisibilityRegion::new_sphere(center, 50.0)
                .with_falloff(15.0)
                .with_gradient_strength(0.6)
                .with_gradient_direction(Vec3::Y),
            Self::SwampFog => VisibilityRegion::new_half_space(center.y + 5.0)
                .with_falloff(3.0)
                .with_gradient_strength(0.8)
                .with_gradient_direction(Vec3::NEG_Y),
        }
    }

    /// Get screen visibility settings for this preset.
    #[must_use]
    pub fn screen_settings(self) -> ScreenVisibility {
        match self {
            Self::CaveDarkness => ScreenVisibility::darkness(),
            Self::BlizzardWhiteout => ScreenVisibility::whiteout(),
            Self::UnderwaterMurk => ScreenVisibility::murk()
                .with_vignette_strength(0.4)
                .with_noise_scale(0.3),
            Self::FireSmoke => ScreenVisibility::smoke()
                .with_vignette_strength(0.5)
                .with_quality(VisibilityQuality::High),
            Self::DeepSeaBioluminescence => ScreenVisibility::bioluminescent()
                .with_vignette_strength(0.7)
                .with_quality(VisibilityQuality::High),
            Self::Sandstorm => ScreenVisibility::default()
                .with_strength(0.8)
                .with_color_shift(Vec3::new(0.6, 0.5, 0.35))
                .with_noise_scale(0.6)
                .with_noise_speed(1.5),
            Self::VolcanicAsh => ScreenVisibility::smoke()
                .with_strength(0.85)
                .with_color_shift(Vec3::new(0.25, 0.22, 0.2)),
            Self::SwampFog => ScreenVisibility::murk()
                .with_strength(0.5)
                .with_color_shift(Vec3::new(0.35, 0.4, 0.3))
                .with_vignette_strength(0.2),
        }
    }

    /// Get the recommended quality level for this preset.
    #[must_use]
    pub fn recommended_quality(self) -> VisibilityQuality {
        match self {
            Self::CaveDarkness | Self::SwampFog => VisibilityQuality::Low,
            Self::BlizzardWhiteout | Self::UnderwaterMurk | Self::Sandstorm => {
                VisibilityQuality::Medium
            }
            Self::FireSmoke | Self::VolcanicAsh | Self::DeepSeaBioluminescence => {
                VisibilityQuality::High
            }
        }
    }

    /// Get the visibility kind for this preset.
    #[must_use]
    pub fn kind(self) -> VisibilityKind {
        match self {
            Self::CaveDarkness => VisibilityKind::Darkness,
            Self::BlizzardWhiteout => VisibilityKind::Whiteout,
            Self::UnderwaterMurk | Self::SwampFog | Self::Sandstorm => VisibilityKind::Murk,
            Self::FireSmoke | Self::VolcanicAsh => VisibilityKind::Smoke,
            Self::DeepSeaBioluminescence => VisibilityKind::BioluminescentContrast,
        }
    }

    /// Whether this preset uses animated noise.
    #[must_use]
    pub fn is_animated(self) -> bool {
        !matches!(self, Self::CaveDarkness)
    }

    /// Whether this preset has gradient density.
    #[must_use]
    pub fn has_gradient(self) -> bool {
        matches!(
            self,
            Self::UnderwaterMurk
                | Self::FireSmoke
                | Self::DeepSeaBioluminescence
                | Self::VolcanicAsh
                | Self::SwampFog
        )
    }

    /// Get a descriptive name for the preset.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::CaveDarkness => "Cave Darkness",
            Self::BlizzardWhiteout => "Blizzard Whiteout",
            Self::UnderwaterMurk => "Underwater Murk",
            Self::FireSmoke => "Fire Smoke",
            Self::DeepSeaBioluminescence => "Deep Sea Bioluminescence",
            Self::Sandstorm => "Sandstorm",
            Self::VolcanicAsh => "Volcanic Ash",
            Self::SwampFog => "Swamp Fog",
        }
    }
}

/// Create a full visibility configuration from a preset.
#[must_use]
pub fn create_from_preset(
    preset: VisibilityPreset,
    center: Vec3,
) -> (VisibilityEffect, VisibilityRegion, ScreenVisibility) {
    (
        preset.effect(),
        preset.default_region(center),
        preset.screen_settings(),
    )
}

/// Create multiple visibility layers for complex effects.
#[must_use]
pub fn create_layered(
    presets: &[VisibilityPreset],
    center: Vec3,
) -> Vec<(VisibilityEffect, VisibilityRegion, ScreenVisibility)> {
    presets
        .iter()
        .enumerate()
        .map(|(i, &preset)| {
            let (effect, mut region, screen) = create_from_preset(preset, center);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                reason = "preset count is small; priority fits in i32"
            )]
            let priority = (presets.len() - i) as i32;
            region = region.with_priority(priority);
            (effect, region, screen)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_all_presets_produce_valid_effects() {
        for preset in VisibilityPreset::ALL {
            let effect = preset.effect();
            assert!(effect.is_valid(), "{preset:?} should produce valid effect");
        }
    }

    #[test]
    fn test_all_presets_produce_valid_regions() {
        for preset in VisibilityPreset::ALL {
            let region = preset.default_region(Vec3::ZERO);
            assert!(
                region.falloff >= 0.0,
                "{preset:?} should have valid falloff"
            );
        }
    }

    #[test]
    fn test_all_presets_produce_valid_screens() {
        for preset in VisibilityPreset::ALL {
            let screen = preset.screen_settings();
            assert!(screen.is_valid(), "{preset:?} should produce valid screen");
        }
    }

    #[test]
    fn test_preset_kinds() {
        assert_eq!(
            VisibilityPreset::CaveDarkness.kind(),
            VisibilityKind::Darkness
        );
        assert_eq!(
            VisibilityPreset::BlizzardWhiteout.kind(),
            VisibilityKind::Whiteout
        );
        assert_eq!(
            VisibilityPreset::UnderwaterMurk.kind(),
            VisibilityKind::Murk
        );
        assert_eq!(VisibilityPreset::FireSmoke.kind(), VisibilityKind::Smoke);
        assert_eq!(
            VisibilityPreset::DeepSeaBioluminescence.kind(),
            VisibilityKind::BioluminescentContrast
        );
    }

    #[test]
    fn test_animated_presets() {
        assert!(!VisibilityPreset::CaveDarkness.is_animated());
        assert!(VisibilityPreset::FireSmoke.is_animated());
        assert!(VisibilityPreset::Sandstorm.is_animated());
    }

    #[test]
    fn test_gradient_presets() {
        assert!(!VisibilityPreset::CaveDarkness.has_gradient());
        assert!(VisibilityPreset::UnderwaterMurk.has_gradient());
        assert!(VisibilityPreset::FireSmoke.has_gradient());
    }

    #[test]
    fn test_preset_names() {
        for preset in VisibilityPreset::ALL {
            assert!(!preset.name().is_empty());
        }
    }

    #[test]
    fn test_create_from_preset() {
        let (effect, region, screen) =
            create_from_preset(VisibilityPreset::CaveDarkness, Vec3::new(10.0, 5.0, 10.0));

        assert_eq!(effect.kind, VisibilityKind::Darkness);
        assert_relative_eq!(region.center.x, 10.0, epsilon = 0.001);
        assert!(screen.enabled);
    }

    #[test]
    fn test_create_layered() {
        let presets = [VisibilityPreset::CaveDarkness, VisibilityPreset::FireSmoke];
        let layers = create_layered(&presets, Vec3::ZERO);

        assert_eq!(layers.len(), 2);
        assert!(layers[0].1.priority > layers[1].1.priority);
    }

    #[test]
    fn test_recommended_quality() {
        assert!(matches!(
            VisibilityPreset::DeepSeaBioluminescence.recommended_quality(),
            VisibilityQuality::High | VisibilityQuality::Ultra
        ));
        assert!(matches!(
            VisibilityPreset::CaveDarkness.recommended_quality(),
            VisibilityQuality::Low | VisibilityQuality::Medium
        ));
    }

    #[test]
    fn test_fire_smoke_has_upward_gradient() {
        let region = VisibilityPreset::FireSmoke.default_region(Vec3::ZERO);
        assert!(region.has_gradient());
        assert!(region.gradient_direction.y > 0.0, "smoke rises");
    }

    #[test]
    fn test_underwater_murk_has_downward_gradient() {
        let region = VisibilityPreset::UnderwaterMurk.default_region(Vec3::ZERO);
        assert!(region.has_gradient());
        assert!(region.gradient_direction.y < 0.0, "murk settles downward");
    }
}

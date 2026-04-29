//! Preset distortion configurations.
//!
//! Ready-to-use distortion effect configurations for common scenarios.

use super::{
    BlendMode, DistortionEffect, DistortionKind, DistortionQuality, DistortionRegion,
    FlowDirection, ScreenDistortion,
};
use glam::Vec3;

/// Preset configurations for common distortion scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DistortionPreset {
    /// Heat shimmer rising from hot surfaces.
    HeatShimmer = 0,
    /// Explosion shockwave.
    ExplosionWave = 1,
    /// Reactor radiation leak.
    RadiationLeak = 2,
    /// Dimensional rift/tear.
    DimensionalRift = 3,
    /// Engine exhaust.
    EngineExhaust = 4,
    /// Lava pool heat.
    LavaHeat = 5,
    /// Force field impact.
    ForceFieldImpact = 6,
    /// Teleporter activation.
    TeleporterActive = 7,
}

impl DistortionPreset {
    /// All presets.
    pub const ALL: [Self; 8] = [
        Self::HeatShimmer,
        Self::ExplosionWave,
        Self::RadiationLeak,
        Self::DimensionalRift,
        Self::EngineExhaust,
        Self::LavaHeat,
        Self::ForceFieldImpact,
        Self::TeleporterActive,
    ];

    /// Get the distortion effect for this preset.
    #[must_use]
    pub fn effect(self) -> DistortionEffect {
        match self {
            Self::HeatShimmer => DistortionEffect::heat_shimmer(),
            Self::ExplosionWave => DistortionEffect::pressure_wave()
                .with_strength(1.0)
                .with_frequency(2.0),
            Self::RadiationLeak => DistortionEffect::radiation_warp()
                .with_strength(0.6)
                .with_frequency(3.0),
            Self::DimensionalRift => DistortionEffect::fracture_event()
                .with_strength(1.0)
                .with_frequency(8.0),
            Self::EngineExhaust => DistortionEffect::heat_shimmer()
                .with_strength(0.5)
                .with_frequency(12.0)
                .with_animation_speed(2.0),
            Self::LavaHeat => DistortionEffect::heat_shimmer()
                .with_strength(0.7)
                .with_frequency(4.0)
                .with_animation_speed(0.5),
            Self::ForceFieldImpact => DistortionEffect::pressure_wave()
                .with_strength(0.6)
                .with_frequency(6.0)
                .with_animation_speed(4.0),
            Self::TeleporterActive => DistortionEffect::fracture_event()
                .with_strength(0.8)
                .with_frequency(10.0)
                .with_animation_speed(3.0),
        }
    }

    /// Get a default region for this preset.
    #[must_use]
    pub fn default_region(self, center: Vec3) -> DistortionRegion {
        match self {
            Self::HeatShimmer => {
                DistortionRegion::new_box(center, Vec3::new(5.0, 2.0, 5.0)).with_falloff(1.0)
            }
            Self::ExplosionWave => {
                DistortionRegion::new_expanding_sphere(center, 1.0, 30.0).with_falloff(3.0)
            }
            Self::RadiationLeak => DistortionRegion::new_sphere(center, 15.0).with_falloff(5.0),
            Self::DimensionalRift => DistortionRegion::new_sphere(center, 8.0).with_falloff(2.0),
            Self::EngineExhaust => {
                DistortionRegion::new_cone(center, Vec3::NEG_Z, 0.3, 10.0).with_falloff(1.0)
            }
            Self::LavaHeat => DistortionRegion::new_half_space(center.y).with_falloff(3.0),
            Self::ForceFieldImpact => DistortionRegion::new_sphere(center, 5.0).with_falloff(2.0),
            Self::TeleporterActive => {
                DistortionRegion::new_cylinder(center, 2.0, 3.0).with_falloff(1.0)
            }
        }
    }

    /// Get screen distortion settings for this preset.
    #[must_use]
    pub fn screen_settings(self) -> ScreenDistortion {
        match self {
            Self::HeatShimmer => ScreenDistortion::heat_shimmer(),
            Self::ExplosionWave => ScreenDistortion::pressure_wave(),
            Self::RadiationLeak => ScreenDistortion::radiation_warp(),
            Self::DimensionalRift => ScreenDistortion::fracture_event(),
            Self::EngineExhaust => ScreenDistortion::heat_shimmer()
                .with_strength(0.4)
                .with_frequency(10.0)
                .with_max_displacement(6.0),
            Self::LavaHeat => ScreenDistortion::heat_shimmer()
                .with_strength(0.6)
                .with_frequency(3.0)
                .with_flow_speed(0.3),
            Self::ForceFieldImpact => ScreenDistortion::pressure_wave()
                .with_strength(0.5)
                .with_blend_mode(BlendMode::ChromaticAberration),
            Self::TeleporterActive => ScreenDistortion::fracture_event()
                .with_flow_direction(FlowDirection::RadialIn)
                .with_quality(DistortionQuality::High),
        }
    }

    /// Get the recommended quality level for this preset.
    #[must_use]
    pub fn recommended_quality(self) -> DistortionQuality {
        match self {
            Self::HeatShimmer | Self::LavaHeat | Self::EngineExhaust | Self::RadiationLeak => {
                DistortionQuality::Medium
            }
            Self::ExplosionWave
            | Self::ForceFieldImpact
            | Self::DimensionalRift
            | Self::TeleporterActive => DistortionQuality::High,
        }
    }

    /// Get the distortion kind for this preset.
    #[must_use]
    pub fn kind(self) -> DistortionKind {
        match self {
            Self::HeatShimmer | Self::EngineExhaust | Self::LavaHeat => DistortionKind::HeatShimmer,
            Self::ExplosionWave | Self::ForceFieldImpact => DistortionKind::PressureWave,
            Self::RadiationLeak => DistortionKind::RadiationWarp,
            Self::DimensionalRift | Self::TeleporterActive => DistortionKind::FractureEvent,
        }
    }

    /// Whether this preset typically uses animated/expanding regions.
    #[must_use]
    pub fn is_animated(self) -> bool {
        matches!(self, Self::ExplosionWave | Self::ForceFieldImpact)
    }

    /// Get a descriptive name for the preset.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::HeatShimmer => "Heat Shimmer",
            Self::ExplosionWave => "Explosion Wave",
            Self::RadiationLeak => "Radiation Leak",
            Self::DimensionalRift => "Dimensional Rift",
            Self::EngineExhaust => "Engine Exhaust",
            Self::LavaHeat => "Lava Heat",
            Self::ForceFieldImpact => "Force Field Impact",
            Self::TeleporterActive => "Teleporter Active",
        }
    }
}

/// Create a full distortion configuration from a preset.
#[must_use]
pub fn create_from_preset(
    preset: DistortionPreset,
    center: Vec3,
) -> (DistortionEffect, DistortionRegion, ScreenDistortion) {
    (
        preset.effect(),
        preset.default_region(center),
        preset.screen_settings(),
    )
}

/// Create multiple distortion layers for complex effects.
#[must_use]
pub fn create_layered(
    presets: &[DistortionPreset],
    center: Vec3,
) -> Vec<(DistortionEffect, DistortionRegion, ScreenDistortion)> {
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
        for preset in DistortionPreset::ALL {
            let effect = preset.effect();
            assert!(effect.is_valid(), "{preset:?} should produce valid effect");
        }
    }

    #[test]
    fn test_all_presets_produce_valid_regions() {
        for preset in DistortionPreset::ALL {
            let region = preset.default_region(Vec3::ZERO);
            assert!(
                region.falloff >= 0.0,
                "{preset:?} should have valid falloff"
            );
        }
    }

    #[test]
    fn test_all_presets_produce_valid_screens() {
        for preset in DistortionPreset::ALL {
            let screen = preset.screen_settings();
            assert!(screen.is_valid(), "{preset:?} should produce valid screen");
        }
    }

    #[test]
    fn test_preset_kinds() {
        assert_eq!(
            DistortionPreset::HeatShimmer.kind(),
            DistortionKind::HeatShimmer
        );
        assert_eq!(
            DistortionPreset::ExplosionWave.kind(),
            DistortionKind::PressureWave
        );
        assert_eq!(
            DistortionPreset::RadiationLeak.kind(),
            DistortionKind::RadiationWarp
        );
        assert_eq!(
            DistortionPreset::DimensionalRift.kind(),
            DistortionKind::FractureEvent
        );
    }

    #[test]
    fn test_animated_presets() {
        assert!(DistortionPreset::ExplosionWave.is_animated());
        assert!(!DistortionPreset::HeatShimmer.is_animated());
    }

    #[test]
    fn test_preset_names() {
        for preset in DistortionPreset::ALL {
            assert!(!preset.name().is_empty());
        }
    }

    #[test]
    fn test_create_from_preset() {
        let (effect, region, screen) =
            create_from_preset(DistortionPreset::HeatShimmer, Vec3::new(10.0, 5.0, 10.0));

        assert_eq!(effect.kind, DistortionKind::HeatShimmer);
        assert_relative_eq!(region.center.x, 10.0, epsilon = 0.001);
        assert!(screen.enabled);
    }

    #[test]
    fn test_create_layered() {
        let presets = [
            DistortionPreset::HeatShimmer,
            DistortionPreset::RadiationLeak,
        ];
        let layers = create_layered(&presets, Vec3::ZERO);

        assert_eq!(layers.len(), 2);
        assert!(layers[0].1.priority > layers[1].1.priority);
    }

    #[test]
    fn test_explosion_expanding_region() {
        let region = DistortionPreset::ExplosionWave.default_region(Vec3::ZERO);
        assert!(region.is_animated());
        assert!(region.expansion_rate > 0.0);
    }

    #[test]
    fn test_recommended_quality() {
        assert!(matches!(
            DistortionPreset::DimensionalRift.recommended_quality(),
            DistortionQuality::High | DistortionQuality::Ultra
        ));
    }
}

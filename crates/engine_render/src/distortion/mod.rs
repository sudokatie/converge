//! Screen-space and world-space distortion effects.
//!
//! Provides CPU-side primitives for rendering visual distortions:
//! heat shimmer, pressure waves, radiation warping, and fracture events.
//! These types configure the GPU distortion rendering passes.

mod effect;
mod presets;
mod region;
mod sampling;
mod screen;
mod uniform;

pub use effect::{DistortionEffect, DistortionKind};
pub use presets::{DistortionPreset, create_from_preset, create_layered};
pub use region::{DistortionRegion, DistortionShape};
pub use sampling::{
    DistortionSampler, FalloffCurve, exponential_falloff, linear_falloff, position_hash,
    radial_wave, sine_wave, smooth_falloff, spiral_wave,
};
pub use screen::{BlendMode, DistortionQuality, FlowDirection, ScreenDistortion};
pub use uniform::{
    DistortionBatch, DistortionInstanceUniform, DistortionRegionUniform, ScreenDistortionUniform,
    convert,
};

use std::hash::{Hash, Hasher};

/// Compute a stable fingerprint for a distortion configuration.
#[must_use]
pub fn compute_fingerprint(effect: &DistortionEffect, region: &DistortionRegion) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    effect.kind.hash(&mut hasher);
    region.shape.hash(&mut hasher);
    region.priority.hash(&mut hasher);
    hash_f32(effect.strength, &mut hasher);
    hash_f32(effect.frequency, &mut hasher);
    hash_f32(region.falloff, &mut hasher);
    hasher.finish()
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

/// Sort distortion effects by priority (higher first), then by kind.
pub fn sort_by_priority(effects: &mut [(DistortionEffect, DistortionRegion)]) {
    effects.sort_by(|a, b| {
        b.1.priority
            .cmp(&a.1.priority)
            .then_with(|| (a.0.kind as u8).cmp(&(b.0.kind as u8)))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_fingerprint_determinism() {
        let effect = DistortionEffect::heat_shimmer();
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);

        let fp1 = compute_fingerprint(&effect, &region);
        let fp2 = compute_fingerprint(&effect, &region);

        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_fingerprint_sensitivity() {
        let effect1 = DistortionEffect::heat_shimmer();
        let effect2 = DistortionEffect::pressure_wave();
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);

        let fp1 = compute_fingerprint(&effect1, &region);
        let fp2 = compute_fingerprint(&effect2, &region);

        assert_ne!(
            fp1, fp2,
            "different effects should have different fingerprints"
        );
    }

    #[test]
    fn test_sort_by_priority() {
        let low = (
            DistortionEffect::heat_shimmer(),
            DistortionRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(0),
        );
        let high = (
            DistortionEffect::pressure_wave(),
            DistortionRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(10),
        );
        let mid = (
            DistortionEffect::radiation_warp(),
            DistortionRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(5),
        );

        let mut effects = vec![low, high, mid];
        sort_by_priority(&mut effects);

        assert_eq!(effects[0].1.priority, 10);
        assert_eq!(effects[1].1.priority, 5);
        assert_eq!(effects[2].1.priority, 0);
    }
}

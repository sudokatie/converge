//! Visibility effect rendering primitives.
//!
//! Provides CPU-side primitives for rendering visibility effects:
//! darkness, whiteout, murk, smoke, and bioluminescent contrast.
//! These types configure the GPU visibility rendering passes.

mod effect;
mod presets;
mod region;
mod sampling;
mod uniform;
mod viewport;

pub use effect::{VisibilityEffect, VisibilityKind};
pub use presets::{VisibilityPreset, create_from_preset, create_layered};
pub use region::{VisibilityRegion, VisibilityShape};
pub use sampling::{
    VisibilityFalloff, VisibilitySampler, bioluminescent_factor, bioluminescent_pulse,
    depth_visibility, position_hash_3d, visibility_from_distance, visibility_squared_exp,
};
pub use uniform::{
    ScreenVisibilityUniform, VisibilityBatch, VisibilityEffectUniform, VisibilityInstanceUniform,
    VisibilityRegionUniform, convert,
};
pub use viewport::{ScreenVisibility, VisibilityBlendMode, VisibilityQuality};

use std::hash::{Hash, Hasher};

/// Compute a stable fingerprint for a visibility configuration.
#[must_use]
pub fn compute_fingerprint(effect: &VisibilityEffect, region: &VisibilityRegion) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    effect.kind.hash(&mut hasher);
    region.shape.hash(&mut hasher);
    region.priority.hash(&mut hasher);
    hash_f32(effect.obscurance, &mut hasher);
    hash_f32(effect.contrast, &mut hasher);
    hash_f32(region.falloff, &mut hasher);
    hasher.finish()
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

/// Sort visibility effects by priority (higher first), then by kind.
pub fn sort_by_priority(effects: &mut [(VisibilityEffect, VisibilityRegion)]) {
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
        let effect = VisibilityEffect::smoke();
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);

        let fp1 = compute_fingerprint(&effect, &region);
        let fp2 = compute_fingerprint(&effect, &region);

        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_fingerprint_sensitivity() {
        let effect1 = VisibilityEffect::smoke();
        let effect2 = VisibilityEffect::darkness();
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);

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
            VisibilityEffect::smoke(),
            VisibilityRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(0),
        );
        let high = (
            VisibilityEffect::darkness(),
            VisibilityRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(10),
        );
        let mid = (
            VisibilityEffect::murk(),
            VisibilityRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(5),
        );

        let mut effects = vec![low, high, mid];
        sort_by_priority(&mut effects);

        assert_eq!(effects[0].1.priority, 10);
        assert_eq!(effects[1].1.priority, 5);
        assert_eq!(effects[2].1.priority, 0);
    }
}

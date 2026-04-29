//! Multi-environment post-processing stack with region-aware blending.
//!
//! Provides CPU-side primitives for managing post-processing effects
//! across multiple environments with spatial blending. These types
//! configure the GPU post-processing rendering passes.
//!
//! # Architecture
//!
//! The post-processing system is organized around:
//!
//! - **Effects**: Individual post-processing operations (bloom, color grading, etc.)
//! - **Regions**: Spatial bounds where effects apply (spheres, boxes, global)
//! - **Layers**: Combinations of effects and regions with blend modes
//! - **Stacks**: Collections of layers per effect type for evaluation
//!
//! # Example
//!
//! ```ignore
//! use engine_render::post_process::*;
//!
//! // Create a bloom layer for an underground cave
//! let cave_bloom = PostLayer::new(
//!     PostEffect::bloom().with_intensity(0.3).with_threshold(0.5),
//!     PostRegion::new_sphere(cave_center, 50.0)
//!         .with_priority(priorities::ENVIRONMENT)
//!         .with_environment(CAVE_ENV_ID),
//! );
//!
//! // Evaluate at camera position
//! let effective_bloom = stack.evaluate(camera_pos);
//! ```

mod blend;
mod effect;
mod layer;
mod region;
mod sampling;
mod uniform;

pub use blend::{BlendWeights, PostBlendMode, RegionWeight, priorities};
pub use effect::{PostEffect, PostEffectKind};
pub use layer::{Environment, PostLayer, PostLayerStack};
pub use region::{PostRegion, PostRegionShape};
pub use sampling::{PostCameraState, PostSampler, frame_jitter, halton_jitter, position_hash};
pub use uniform::{
    PostBatch, PostEffectUniform, PostGlobalUniform, PostInstanceUniform, PostRegionUniform,
    convert,
};

use std::hash::{Hash, Hasher};

/// Compute a stable fingerprint for a post-processing configuration.
///
/// The fingerprint is deterministic and can be used to detect changes
/// in the post-processing setup for caching and invalidation.
#[must_use]
pub fn compute_fingerprint(effect: &PostEffect, region: &PostRegion) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    effect.kind.hash(&mut hasher);
    region.shape.hash(&mut hasher);
    region.priority.hash(&mut hasher);
    region.environment_id.hash(&mut hasher);
    hash_f32(effect.intensity, &mut hasher);
    hash_f32(effect.threshold, &mut hasher);
    hash_f32(effect.radius, &mut hasher);
    hash_f32(region.falloff, &mut hasher);
    hasher.finish()
}

/// Compute a fingerprint for a layer.
#[must_use]
pub fn compute_layer_fingerprint(layer: &PostLayer) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let base = compute_fingerprint(&layer.effect, &layer.region);
    base.hash(&mut hasher);
    layer.blend_mode.hash(&mut hasher);
    layer.enabled.hash(&mut hasher);
    hasher.finish()
}

/// Compute a fingerprint for an entire stack.
#[must_use]
pub fn compute_stack_fingerprint(stack: &PostLayerStack) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (stack.kind() as u8).hash(&mut hasher);
    stack.len().hash(&mut hasher);
    for layer in stack.layers() {
        compute_layer_fingerprint(layer).hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

/// Sort layers by priority (higher first), then by effect kind.
pub fn sort_by_priority(layers: &mut [(PostEffect, PostRegion)]) {
    layers.sort_by(|a, b| {
        b.1.priority
            .cmp(&a.1.priority)
            .then_with(|| (a.0.kind as u8).cmp(&(b.0.kind as u8)))
    });
}

/// Sort layers by execution order (for proper effect ordering).
pub fn sort_by_execution_order(layers: &mut [PostLayer]) {
    layers.sort_by(|a, b| {
        a.effect
            .kind
            .default_order()
            .cmp(&b.effect.kind.default_order())
            .then_with(|| b.region.priority.cmp(&a.region.priority))
    });
}

/// Filter to only active layers.
#[must_use]
pub fn filter_active(layers: &[PostLayer]) -> Vec<&PostLayer> {
    layers
        .iter()
        .filter(|l| l.enabled && l.effect.active)
        .collect()
}

/// Group layers by environment ID.
#[must_use]
pub fn group_by_environment(
    layers: &[PostLayer],
) -> std::collections::HashMap<u32, Vec<&PostLayer>> {
    let mut groups = std::collections::HashMap::new();
    for layer in layers {
        groups
            .entry(layer.environment_id())
            .or_insert_with(Vec::new)
            .push(layer);
    }
    groups
}

/// Group layers by effect kind.
#[must_use]
pub fn group_by_kind(
    layers: &[PostLayer],
) -> std::collections::HashMap<PostEffectKind, Vec<&PostLayer>> {
    let mut groups = std::collections::HashMap::new();
    for layer in layers {
        groups
            .entry(layer.kind())
            .or_insert_with(Vec::new)
            .push(layer);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_fingerprint_determinism() {
        let effect = PostEffect::bloom();
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);

        let fp1 = compute_fingerprint(&effect, &region);
        let fp2 = compute_fingerprint(&effect, &region);

        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_fingerprint_sensitivity() {
        let effect1 = PostEffect::bloom();
        let effect2 = PostEffect::vignette();
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);

        let fp1 = compute_fingerprint(&effect1, &region);
        let fp2 = compute_fingerprint(&effect2, &region);

        assert_ne!(
            fp1, fp2,
            "different effects should have different fingerprints"
        );
    }

    #[test]
    fn test_fingerprint_region_sensitivity() {
        let effect = PostEffect::bloom();
        let region1 = PostRegion::new_sphere(Vec3::ZERO, 10.0);
        let region2 = PostRegion::new_sphere(Vec3::ZERO, 20.0);

        let fp1 = compute_fingerprint(&effect, &region1);
        let fp2 = compute_fingerprint(&effect, &region2);

        assert_ne!(
            fp1, fp2,
            "different regions should have different fingerprints"
        );
    }

    #[test]
    fn test_layer_fingerprint() {
        let layer1 = PostLayer::global(PostEffect::bloom());
        let layer2 = PostLayer::global(PostEffect::bloom()).with_enabled(false);

        let fp1 = compute_layer_fingerprint(&layer1);
        let fp2 = compute_layer_fingerprint(&layer2);

        assert_ne!(fp1, fp2, "enabled state should affect fingerprint");
    }

    #[test]
    fn test_stack_fingerprint() {
        let mut stack1 = PostLayerStack::new(PostEffectKind::Bloom);
        stack1.push(PostLayer::global(PostEffect::bloom()));

        let mut stack2 = PostLayerStack::new(PostEffectKind::Bloom);
        stack2.push(PostLayer::global(PostEffect::bloom()));
        stack2.push(PostLayer::global(PostEffect::bloom()));

        let fp1 = compute_stack_fingerprint(&stack1);
        let fp2 = compute_stack_fingerprint(&stack2);

        assert_ne!(
            fp1, fp2,
            "different stacks should have different fingerprints"
        );
    }

    #[test]
    fn test_sort_by_priority() {
        let low = (
            PostEffect::bloom(),
            PostRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(0),
        );
        let high = (
            PostEffect::vignette(),
            PostRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(10),
        );
        let mid = (
            PostEffect::tone_map(),
            PostRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(5),
        );

        let mut layers = vec![low, high, mid];
        sort_by_priority(&mut layers);

        assert_eq!(layers[0].1.priority, 10);
        assert_eq!(layers[1].1.priority, 5);
        assert_eq!(layers[2].1.priority, 0);
    }

    #[test]
    fn test_sort_by_execution_order() {
        let bloom = PostLayer::global(PostEffect::bloom());
        let grain = PostLayer::global(PostEffect::film_grain());
        let dof = PostLayer::global(PostEffect::depth_of_field());

        let mut layers = vec![bloom.clone(), grain.clone(), dof.clone()];
        sort_by_execution_order(&mut layers);

        assert_eq!(layers[0].kind(), PostEffectKind::DepthOfField);
        assert_eq!(layers[1].kind(), PostEffectKind::Bloom);
        assert_eq!(layers[2].kind(), PostEffectKind::FilmGrain);
    }

    #[test]
    fn test_filter_active() {
        let active = PostLayer::global(PostEffect::bloom());
        let disabled = PostLayer::global(PostEffect::bloom()).with_enabled(false);
        let inactive_effect = PostLayer::global(PostEffect::bloom().with_active(false));

        let layers = vec![active, disabled, inactive_effect];
        let filtered = filter_active(&layers);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_group_by_environment() {
        let env1_layer = PostLayer::new(
            PostEffect::bloom(),
            PostRegion::new_sphere(Vec3::ZERO, 10.0).with_environment(1),
        );
        let env2_layer = PostLayer::new(
            PostEffect::bloom(),
            PostRegion::new_sphere(Vec3::ZERO, 10.0).with_environment(2),
        );
        let env1_layer2 = PostLayer::new(
            PostEffect::vignette(),
            PostRegion::new_sphere(Vec3::ZERO, 10.0).with_environment(1),
        );

        let layers = vec![env1_layer, env2_layer, env1_layer2];
        let groups = group_by_environment(&layers);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get(&1).unwrap().len(), 2);
        assert_eq!(groups.get(&2).unwrap().len(), 1);
    }

    #[test]
    fn test_group_by_kind() {
        let bloom1 = PostLayer::global(PostEffect::bloom());
        let bloom2 = PostLayer::global(PostEffect::bloom());
        let vignette = PostLayer::global(PostEffect::vignette());

        let layers = vec![bloom1, bloom2, vignette];
        let groups = group_by_kind(&layers);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get(&PostEffectKind::Bloom).unwrap().len(), 2);
        assert_eq!(groups.get(&PostEffectKind::Vignette).unwrap().len(), 1);
    }

    #[test]
    fn test_all_effect_kinds_constructible() {
        for kind in PostEffectKind::ALL {
            let effect = PostEffect::from_kind(kind);
            assert_eq!(effect.kind, kind);
            assert!(effect.is_valid());
        }
    }

    #[test]
    fn test_all_region_shapes_constructible() {
        let sphere = PostRegion::new_sphere(Vec3::ZERO, 10.0);
        assert_eq!(sphere.shape, PostRegionShape::Sphere);

        let box_region = PostRegion::new_box(Vec3::ZERO, Vec3::splat(5.0));
        assert_eq!(box_region.shape, PostRegionShape::Box);

        let cylinder = PostRegion::new_cylinder(Vec3::ZERO, 5.0, 10.0);
        assert_eq!(cylinder.shape, PostRegionShape::Cylinder);

        let half_space = PostRegion::new_half_space(64.0);
        assert_eq!(half_space.shape, PostRegionShape::HalfSpace);

        let global = PostRegion::global();
        assert_eq!(global.shape, PostRegionShape::Global);
    }

    #[test]
    fn test_blend_modes_comprehensive() {
        for mode in PostBlendMode::ALL {
            let result = mode.blend(0.5, 0.8, 0.5);
            assert!(
                (0.0..=1.0).contains(&result),
                "blend result should be in valid range for {mode:?}",
            );
        }
    }

    #[test]
    fn test_priorities_ordering() {
        const { assert!(priorities::BACKGROUND < priorities::DEFAULT) };
        const { assert!(priorities::DEFAULT < priorities::ENVIRONMENT) };
        const { assert!(priorities::ENVIRONMENT < priorities::LOCAL) };
        const { assert!(priorities::LOCAL < priorities::PLAYER) };
        const { assert!(priorities::PLAYER < priorities::OVERLAY) };
    }

    #[test]
    fn test_camera_sampling_integration() {
        let camera = PostCameraState::default();
        let sampler = PostSampler::new(42).with_time(1.0);

        let region = PostRegion::new_sphere(Vec3::new(0.0, 0.0, -10.0), 5.0);
        let effect = PostEffect::bloom();
        let layer = PostLayer::new(effect, region);

        assert!(camera.is_in_front(Vec3::new(0.0, 0.0, -10.0)));
        assert!(layer.affects(Vec3::new(0.0, 0.0, -10.0)));

        let noise = sampler.sample_screen_noise(0.5, 0.5);
        assert!((0.0..=1.0).contains(&noise));
    }

    #[test]
    fn test_batch_conversion() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0).with_priority(5);
        let effect = PostEffect::bloom().with_intensity(0.7);
        let blend = PostBlendMode::Additive;

        let mut batch = PostBatch::new(PostEffectKind::Bloom, 8);
        assert!(batch.add(region, effect, blend));

        let bytes = batch.as_bytes();
        assert!(!bytes.is_empty());
        assert_eq!(bytes.len() % 16, 0, "batch bytes should be 16-byte aligned");
    }

    #[test]
    fn test_uniform_alignment() {
        assert_eq!(std::mem::size_of::<PostRegionUniform>(), 32);
        assert_eq!(std::mem::size_of::<PostEffectUniform>(), 32);
        assert_eq!(std::mem::size_of::<PostInstanceUniform>(), 80);
        assert_eq!(std::mem::size_of::<PostGlobalUniform>(), 64);
    }
}

//! Post-processing layers combining effects with regions.
//!
//! A layer binds a post-processing effect to a region, creating a
//! spatially-bounded visual treatment.

use super::{PostBlendMode, PostEffect, PostEffectKind, PostRegion};
use glam::Vec3;

/// A post-processing layer combining effect and region.
#[derive(Debug, Clone)]
pub struct PostLayer {
    /// The effect to apply.
    pub effect: PostEffect,
    /// The region where this effect applies.
    pub region: PostRegion,
    /// How this layer blends with others.
    pub blend_mode: PostBlendMode,
    /// Layer name for debugging.
    pub name: String,
    /// Whether this layer is currently enabled.
    pub enabled: bool,
}

impl PostLayer {
    /// Create a new post-processing layer.
    #[must_use]
    pub fn new(effect: PostEffect, region: PostRegion) -> Self {
        Self {
            effect,
            region,
            blend_mode: PostBlendMode::Weighted,
            name: String::new(),
            enabled: true,
        }
    }

    /// Create a global layer (applies to entire screen).
    #[must_use]
    pub fn global(effect: PostEffect) -> Self {
        Self::new(effect, PostRegion::global())
    }

    /// Set blend mode.
    #[must_use]
    pub fn with_blend_mode(mut self, mode: PostBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set layer name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set enabled state.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Compute the effective intensity at a world position.
    #[must_use]
    pub fn intensity_at(&self, position: Vec3) -> f32 {
        if !self.enabled || !self.effect.active {
            return 0.0;
        }
        self.effect.intensity * self.region.blend_factor(position)
    }

    /// Check if this layer affects a given position.
    #[must_use]
    pub fn affects(&self, position: Vec3) -> bool {
        self.enabled && self.effect.active && self.region.blend_factor(position) > 0.001
    }

    /// Get the effect kind.
    #[must_use]
    pub fn kind(&self) -> PostEffectKind {
        self.effect.kind
    }

    /// Get the priority.
    #[must_use]
    pub fn priority(&self) -> i32 {
        self.region.priority
    }

    /// Get the environment ID.
    #[must_use]
    pub fn environment_id(&self) -> u32 {
        self.region.environment_id
    }
}

impl Default for PostLayer {
    fn default() -> Self {
        Self::new(PostEffect::default(), PostRegion::default())
    }
}

/// A stack of post-processing layers for a single effect kind.
#[derive(Debug, Clone, Default)]
pub struct PostLayerStack {
    /// Layers in this stack, ordered by priority.
    layers: Vec<PostLayer>,
    /// Effect kind this stack handles.
    kind: PostEffectKind,
}

impl PostLayerStack {
    /// Create an empty stack for the given effect kind.
    #[must_use]
    pub fn new(kind: PostEffectKind) -> Self {
        Self {
            layers: Vec::new(),
            kind,
        }
    }

    /// Create with capacity.
    #[must_use]
    pub fn with_capacity(kind: PostEffectKind, capacity: usize) -> Self {
        Self {
            layers: Vec::with_capacity(capacity),
            kind,
        }
    }

    /// Get the effect kind.
    #[must_use]
    pub fn kind(&self) -> PostEffectKind {
        self.kind
    }

    /// Add a layer to the stack.
    pub fn push(&mut self, layer: PostLayer) {
        debug_assert_eq!(
            layer.effect.kind, self.kind,
            "layer effect kind must match stack kind"
        );
        self.layers.push(layer);
        self.sort_by_priority();
    }

    /// Remove a layer by index.
    pub fn remove(&mut self, index: usize) -> Option<PostLayer> {
        if index < self.layers.len() {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Clear all layers.
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Get layers sorted by priority (highest first).
    #[must_use]
    pub fn layers(&self) -> &[PostLayer] {
        &self.layers
    }

    /// Get mutable access to layers.
    pub fn layers_mut(&mut self) -> &mut [PostLayer] {
        &mut self.layers
    }

    /// Number of layers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Get all layers affecting a position.
    #[must_use]
    pub fn affecting(&self, position: Vec3) -> Vec<(usize, &PostLayer, f32)> {
        self.layers
            .iter()
            .enumerate()
            .filter_map(|(i, layer)| {
                let factor = layer.region.blend_factor(position);
                if factor > 0.001 && layer.enabled && layer.effect.active {
                    Some((i, layer, factor))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Compute blended effect at a position.
    #[must_use]
    pub fn evaluate(&self, position: Vec3) -> Option<PostEffect> {
        let affecting = self.affecting(position);
        if affecting.is_empty() {
            return None;
        }

        if affecting.len() == 1 {
            let (_, layer, factor) = affecting[0];
            return Some(layer.effect.with_intensity(layer.effect.intensity * factor));
        }

        let mut result = affecting[0].1.effect;
        let mut total_weight = affecting[0].2;

        for (_, layer, factor) in affecting.iter().skip(1) {
            let normalized_factor = *factor / (total_weight + factor);
            result = result.lerp(layer.effect, normalized_factor);
            total_weight += factor;
        }

        if total_weight > 0.0 {
            result = result.with_intensity(result.intensity * total_weight.min(1.0));
        }

        Some(result)
    }

    fn sort_by_priority(&mut self) {
        self.layers.sort_by(|a, b| {
            b.region
                .priority
                .cmp(&a.region.priority)
                .then_with(|| (a.effect.kind as u8).cmp(&(b.effect.kind as u8)))
        });
    }
}

/// Environment identifier with metadata.
#[derive(Debug, Clone)]
pub struct Environment {
    /// Unique identifier.
    pub id: u32,
    /// Environment name.
    pub name: String,
    /// Base priority offset for all effects in this environment.
    pub priority_offset: i32,
    /// Whether this environment is currently active.
    pub active: bool,
}

impl Environment {
    /// Create a new environment.
    #[must_use]
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            priority_offset: 0,
            active: true,
        }
    }

    /// Set priority offset.
    #[must_use]
    pub fn with_priority_offset(mut self, offset: i32) -> Self {
        self.priority_offset = offset;
        self
    }

    /// Set active state.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_layer_creation() {
        let effect = PostEffect::bloom();
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);
        let layer = PostLayer::new(effect, region);

        assert_eq!(layer.kind(), PostEffectKind::Bloom);
        assert!(layer.enabled);
    }

    #[test]
    fn test_layer_global() {
        let layer = PostLayer::global(PostEffect::tone_map());
        assert!(layer.region.is_global());
        assert_relative_eq!(
            layer.intensity_at(Vec3::new(1000.0, 1000.0, 1000.0)),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_layer_intensity_at() {
        let effect = PostEffect::bloom().with_intensity(0.8);
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0).with_falloff(0.0);
        let layer = PostLayer::new(effect, region);

        assert_relative_eq!(layer.intensity_at(Vec3::ZERO), 0.8, epsilon = 0.001);
        assert_relative_eq!(
            layer.intensity_at(Vec3::new(20.0, 0.0, 0.0)),
            0.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_layer_disabled() {
        let layer = PostLayer::global(PostEffect::bloom()).with_enabled(false);
        assert_relative_eq!(layer.intensity_at(Vec3::ZERO), 0.0, epsilon = 0.001);
        assert!(!layer.affects(Vec3::ZERO));
    }

    #[test]
    fn test_layer_effect_inactive() {
        let effect = PostEffect::bloom().with_active(false);
        let layer = PostLayer::global(effect);
        assert_relative_eq!(layer.intensity_at(Vec3::ZERO), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_layer_stack_creation() {
        let stack = PostLayerStack::new(PostEffectKind::Bloom);
        assert_eq!(stack.kind(), PostEffectKind::Bloom);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_layer_stack_push() {
        let mut stack = PostLayerStack::new(PostEffectKind::Bloom);
        stack.push(PostLayer::global(PostEffect::bloom()));
        stack.push(PostLayer::new(
            PostEffect::bloom(),
            PostRegion::new_sphere(Vec3::ZERO, 10.0).with_priority(10),
        ));

        assert_eq!(stack.len(), 2);
        assert_eq!(stack.layers()[0].priority(), 10);
    }

    #[test]
    fn test_layer_stack_affecting() {
        let mut stack = PostLayerStack::new(PostEffectKind::Bloom);
        stack.push(PostLayer::global(PostEffect::bloom()));
        stack.push(PostLayer::new(
            PostEffect::bloom(),
            PostRegion::new_sphere(Vec3::new(100.0, 0.0, 0.0), 5.0),
        ));

        let affecting = stack.affecting(Vec3::ZERO);
        assert_eq!(affecting.len(), 1);

        let affecting = stack.affecting(Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(affecting.len(), 2);
    }

    #[test]
    fn test_layer_stack_evaluate_single() {
        let mut stack = PostLayerStack::new(PostEffectKind::Bloom);
        stack.push(PostLayer::global(PostEffect::bloom().with_intensity(0.7)));

        let result = stack.evaluate(Vec3::ZERO).unwrap();
        assert_relative_eq!(result.intensity, 0.7, epsilon = 0.001);
    }

    #[test]
    fn test_layer_stack_evaluate_multiple() {
        let mut stack = PostLayerStack::new(PostEffectKind::Bloom);
        stack.push(PostLayer::global(PostEffect::bloom().with_intensity(0.3)));
        stack.push(PostLayer::new(
            PostEffect::bloom().with_intensity(0.9),
            PostRegion::new_sphere(Vec3::ZERO, 10.0).with_priority(10),
        ));

        let result = stack.evaluate(Vec3::ZERO).unwrap();
        assert!(result.intensity > 0.3);
    }

    #[test]
    fn test_layer_stack_evaluate_empty() {
        let stack = PostLayerStack::new(PostEffectKind::Bloom);
        assert!(stack.evaluate(Vec3::ZERO).is_none());
    }

    #[test]
    fn test_layer_stack_remove() {
        let mut stack = PostLayerStack::new(PostEffectKind::Bloom);
        stack.push(PostLayer::global(PostEffect::bloom()).with_name("test"));
        assert_eq!(stack.len(), 1);

        let removed = stack.remove(0).unwrap();
        assert_eq!(removed.name, "test");
        assert!(stack.is_empty());
    }

    #[test]
    fn test_layer_stack_clear() {
        let mut stack = PostLayerStack::new(PostEffectKind::Bloom);
        stack.push(PostLayer::global(PostEffect::bloom()));
        stack.push(PostLayer::global(PostEffect::bloom()));
        assert_eq!(stack.len(), 2);

        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_environment_creation() {
        let env = Environment::new(1, "cave")
            .with_priority_offset(100)
            .with_active(true);

        assert_eq!(env.id, 1);
        assert_eq!(env.name, "cave");
        assert_eq!(env.priority_offset, 100);
        assert!(env.active);
    }

    #[test]
    fn test_layer_with_name() {
        let layer = PostLayer::global(PostEffect::bloom()).with_name("main_bloom");
        assert_eq!(layer.name, "main_bloom");
    }

    #[test]
    fn test_layer_environment_id() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0).with_environment(42);
        let layer = PostLayer::new(PostEffect::bloom(), region);
        assert_eq!(layer.environment_id(), 42);
    }
}

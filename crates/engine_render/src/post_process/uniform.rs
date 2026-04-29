//! GPU-friendly uniform structures for post-processing.
//!
//! These structures are designed to be directly uploaded to GPU buffers
//! with proper alignment and layout for shader access.

use super::{PostBlendMode, PostEffect, PostEffectKind, PostRegion, PostRegionShape};
use bytemuck::{Pod, Zeroable};

/// GPU-friendly post-processing region uniform.
///
/// 32 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PostRegionUniform {
    /// Center position (XYZ) + falloff (W).
    pub center_falloff: [f32; 4],
    /// Extents (XYZ) + shape (W as f32).
    pub extents_shape: [f32; 4],
}

impl From<PostRegion> for PostRegionUniform {
    fn from(region: PostRegion) -> Self {
        Self {
            center_falloff: [
                region.center.x,
                region.center.y,
                region.center.z,
                region.falloff,
            ],
            extents_shape: [
                region.extents.x,
                region.extents.y,
                region.extents.z,
                f32::from(region.shape as u8),
            ],
        }
    }
}

impl Default for PostRegionUniform {
    fn default() -> Self {
        PostRegion::default().into()
    }
}

/// GPU-friendly post-processing effect uniform.
///
/// 32 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PostEffectUniform {
    /// Color (RGB) + intensity (A).
    pub color_intensity: [f32; 4],
    /// Threshold, radius, secondary, kind (as f32).
    pub params: [f32; 4],
}

impl From<PostEffect> for PostEffectUniform {
    fn from(effect: PostEffect) -> Self {
        Self {
            color_intensity: [
                effect.color.x,
                effect.color.y,
                effect.color.z,
                if effect.active { effect.intensity } else { 0.0 },
            ],
            params: [
                effect.threshold,
                effect.radius,
                effect.secondary,
                f32::from(effect.kind as u8),
            ],
        }
    }
}

impl Default for PostEffectUniform {
    fn default() -> Self {
        PostEffect::default().into()
    }
}

/// Combined post-processing instance for GPU instanced rendering.
///
/// 80 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PostInstanceUniform {
    /// Region data.
    pub region: PostRegionUniform,
    /// Effect data.
    pub effect: PostEffectUniform,
    /// Priority (as f32), environment ID (as f32), blend mode (as f32), padding.
    pub meta: [f32; 4],
}

impl PostInstanceUniform {
    /// Create a new instance uniform.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "small metadata values")]
    pub fn new(region: PostRegion, effect: PostEffect, blend_mode: PostBlendMode) -> Self {
        Self {
            region: region.into(),
            effect: effect.into(),
            meta: [
                region.priority as f32,
                region.environment_id as f32,
                f32::from(blend_mode as u8),
                0.0,
            ],
        }
    }
}

impl Default for PostInstanceUniform {
    fn default() -> Self {
        Self::new(
            PostRegion::default(),
            PostEffect::default(),
            PostBlendMode::default(),
        )
    }
}

/// Batch of post-processing instances for GPU upload.
#[derive(Debug, Clone)]
pub struct PostBatch {
    /// Instance data.
    pub instances: Vec<PostInstanceUniform>,
    /// Maximum number of active instances.
    pub max_instances: usize,
    /// Effect kind this batch handles.
    pub kind: PostEffectKind,
}

impl PostBatch {
    /// Create a new batch with the given capacity.
    #[must_use]
    pub fn new(kind: PostEffectKind, capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            max_instances: capacity,
            kind,
        }
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Add an instance if there's room.
    pub fn push(&mut self, instance: PostInstanceUniform) -> bool {
        if self.instances.len() < self.max_instances {
            self.instances.push(instance);
            true
        } else {
            false
        }
    }

    /// Add from region, effect, and blend mode.
    pub fn add(
        &mut self,
        region: PostRegion,
        effect: PostEffect,
        blend_mode: PostBlendMode,
    ) -> bool {
        self.push(PostInstanceUniform::new(region, effect, blend_mode))
    }

    /// Get the raw byte data for GPU upload.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.instances)
    }

    /// Number of active instances.
    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Sort instances by priority (descending).
    pub fn sort_by_priority(&mut self) {
        self.instances.sort_by(|a, b| {
            b.meta[0]
                .partial_cmp(&a.meta[0])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

impl Default for PostBatch {
    fn default() -> Self {
        Self::new(PostEffectKind::ColorGrade, 64)
    }
}

/// Global post-processing parameters uniform.
///
/// 64 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PostGlobalUniform {
    /// Camera position (XYZ) + time (W).
    pub camera_time: [f32; 4],
    /// Viewport size (XY) + aspect ratio (Z) + padding.
    pub viewport: [f32; 4],
    /// Near plane, far plane, exposure, gamma.
    pub render_params: [f32; 4],
    /// Frame index (as f32), jitter X, jitter Y, padding.
    pub temporal: [f32; 4],
}

impl PostGlobalUniform {
    /// Create a new global uniform.
    #[must_use]
    pub fn new() -> Self {
        Self {
            camera_time: [0.0, 0.0, 0.0, 0.0],
            viewport: [1920.0, 1080.0, 16.0 / 9.0, 0.0],
            render_params: [0.1, 1000.0, 1.0, 2.2],
            temporal: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Set camera position.
    #[must_use]
    pub fn with_camera(mut self, x: f32, y: f32, z: f32) -> Self {
        self.camera_time[0] = x;
        self.camera_time[1] = y;
        self.camera_time[2] = z;
        self
    }

    /// Set time.
    #[must_use]
    pub fn with_time(mut self, time: f32) -> Self {
        self.camera_time[3] = time;
        self
    }

    /// Set viewport size.
    #[must_use]
    pub fn with_viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport[0] = width;
        self.viewport[1] = height;
        self.viewport[2] = if height > 0.0 { width / height } else { 1.0 };
        self
    }

    /// Set near/far planes.
    #[must_use]
    pub fn with_planes(mut self, near: f32, far: f32) -> Self {
        self.render_params[0] = near;
        self.render_params[1] = far;
        self
    }

    /// Set exposure.
    #[must_use]
    pub fn with_exposure(mut self, exposure: f32) -> Self {
        self.render_params[2] = exposure;
        self
    }

    /// Set gamma.
    #[must_use]
    pub fn with_gamma(mut self, gamma: f32) -> Self {
        self.render_params[3] = gamma;
        self
    }

    /// Set frame index and jitter.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "frame index is small")]
    pub fn with_temporal(mut self, frame: u32, jitter_x: f32, jitter_y: f32) -> Self {
        self.temporal[0] = frame as f32;
        self.temporal[1] = jitter_x;
        self.temporal[2] = jitter_y;
        self
    }
}

impl Default for PostGlobalUniform {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility to convert enum values to GPU-compatible formats.
pub mod convert {
    use super::{PostBlendMode, PostEffectKind, PostRegionShape};

    /// Convert effect kind to u32 for shader.
    #[must_use]
    pub fn kind_to_u32(kind: PostEffectKind) -> u32 {
        kind as u32
    }

    /// Convert region shape to u32 for shader.
    #[must_use]
    pub fn shape_to_u32(shape: PostRegionShape) -> u32 {
        shape as u32
    }

    /// Convert blend mode to u32 for shader.
    #[must_use]
    pub fn blend_to_u32(blend: PostBlendMode) -> u32 {
        blend as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn test_region_uniform_conversion() {
        let region = PostRegion::new_sphere(Vec3::new(1.0, 2.0, 3.0), 5.0)
            .with_falloff(1.5)
            .with_priority(10);
        let uniform: PostRegionUniform = region.into();

        assert_relative_eq!(uniform.center_falloff[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.center_falloff[3], 1.5, epsilon = 0.001);
        assert_relative_eq!(uniform.extents_shape[0], 5.0, epsilon = 0.001);
        assert_relative_eq!(
            uniform.extents_shape[3],
            f32::from(PostRegionShape::Sphere as u8),
            epsilon = 0.001
        );
    }

    #[test]
    fn test_effect_uniform_conversion() {
        let effect = PostEffect::bloom().with_intensity(0.8).with_threshold(0.9);
        let uniform: PostEffectUniform = effect.into();

        assert_relative_eq!(uniform.color_intensity[3], 0.8, epsilon = 0.001);
        assert_relative_eq!(uniform.params[0], 0.9, epsilon = 0.001);
        assert_relative_eq!(
            uniform.params[3],
            f32::from(PostEffectKind::Bloom as u8),
            epsilon = 0.001
        );
    }

    #[test]
    fn test_effect_uniform_inactive() {
        let effect = PostEffect::bloom().with_active(false);
        let uniform: PostEffectUniform = effect.into();
        assert_relative_eq!(uniform.color_intensity[3], 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_instance_uniform_creation() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0).with_priority(5);
        let effect = PostEffect::bloom();
        let blend = PostBlendMode::Additive;

        let instance = PostInstanceUniform::new(region, effect, blend);

        assert_relative_eq!(instance.region.extents_shape[0], 10.0, epsilon = 0.001);
        assert_relative_eq!(instance.meta[0], 5.0, epsilon = 0.001);
        assert_relative_eq!(
            instance.meta[2],
            f32::from(PostBlendMode::Additive as u8),
            epsilon = 0.001
        );
    }

    #[test]
    fn test_region_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<PostRegionUniform>() % 16,
            0,
            "region uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_effect_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<PostEffectUniform>() % 16,
            0,
            "effect uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_instance_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<PostInstanceUniform>() % 16,
            0,
            "instance uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_global_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<PostGlobalUniform>() % 16,
            0,
            "global uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_batch_operations() {
        let mut batch = PostBatch::new(PostEffectKind::Bloom, 2);
        assert!(batch.is_empty());

        let instance = PostInstanceUniform::default();
        assert!(batch.push(instance));
        assert!(batch.push(instance));
        assert!(!batch.push(instance));

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());

        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn test_batch_add() {
        let mut batch = PostBatch::new(PostEffectKind::Bloom, 4);
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);
        let effect = PostEffect::bloom();

        assert!(batch.add(region, effect, PostBlendMode::Weighted));
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn test_batch_as_bytes() {
        let mut batch = PostBatch::new(PostEffectKind::Bloom, 4);
        batch.push(PostInstanceUniform::default());

        let bytes = batch.as_bytes();
        assert_eq!(bytes.len(), std::mem::size_of::<PostInstanceUniform>());
    }

    #[test]
    fn test_batch_sort_by_priority() {
        let mut batch = PostBatch::new(PostEffectKind::Bloom, 4);

        let low = PostRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(1);
        let high = PostRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(10);
        let mid = PostRegion::new_sphere(Vec3::ZERO, 5.0).with_priority(5);

        batch.add(low, PostEffect::bloom(), PostBlendMode::Weighted);
        batch.add(high, PostEffect::bloom(), PostBlendMode::Weighted);
        batch.add(mid, PostEffect::bloom(), PostBlendMode::Weighted);

        batch.sort_by_priority();

        assert_relative_eq!(batch.instances[0].meta[0], 10.0, epsilon = 0.001);
        assert_relative_eq!(batch.instances[1].meta[0], 5.0, epsilon = 0.001);
        assert_relative_eq!(batch.instances[2].meta[0], 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_global_uniform_builder() {
        let global = PostGlobalUniform::new()
            .with_camera(1.0, 2.0, 3.0)
            .with_time(10.0)
            .with_viewport(1920.0, 1080.0)
            .with_planes(0.1, 500.0)
            .with_exposure(1.5)
            .with_gamma(2.4)
            .with_temporal(42, 0.1, -0.2);

        assert_relative_eq!(global.camera_time[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(global.camera_time[3], 10.0, epsilon = 0.001);
        assert_relative_eq!(global.viewport[0], 1920.0, epsilon = 0.001);
        assert_relative_eq!(global.viewport[2], 16.0 / 9.0, epsilon = 0.01);
        assert_relative_eq!(global.render_params[0], 0.1, epsilon = 0.001);
        assert_relative_eq!(global.render_params[2], 1.5, epsilon = 0.001);
        assert_relative_eq!(global.render_params[3], 2.4, epsilon = 0.001);
        assert_relative_eq!(global.temporal[0], 42.0, epsilon = 0.001);
        assert_relative_eq!(global.temporal[1], 0.1, epsilon = 0.001);
    }

    #[test]
    fn test_convert_utilities() {
        assert_eq!(convert::kind_to_u32(PostEffectKind::ColorGrade), 0);
        assert_eq!(convert::kind_to_u32(PostEffectKind::Bloom), 1);
        assert_eq!(convert::shape_to_u32(PostRegionShape::Sphere), 1);
        assert_eq!(convert::blend_to_u32(PostBlendMode::Weighted), 1);
    }

    #[test]
    fn test_default_uniforms() {
        let region = PostRegionUniform::default();
        let effect = PostEffectUniform::default();
        let instance = PostInstanceUniform::default();
        let global = PostGlobalUniform::default();

        assert_relative_eq!(
            region.extents_shape[3],
            f32::from(PostRegionShape::Sphere as u8),
            epsilon = 0.001
        );
        assert_relative_eq!(
            effect.params[3],
            f32::from(PostEffectKind::ColorGrade as u8),
            epsilon = 0.001
        );
        assert_relative_eq!(
            instance.meta[2],
            f32::from(PostBlendMode::Weighted as u8),
            epsilon = 0.001
        );
        assert_relative_eq!(global.render_params[3], 2.2, epsilon = 0.001);
    }
}

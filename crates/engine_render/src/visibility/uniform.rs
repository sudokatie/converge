//! GPU-friendly uniform structures for visibility effects.
//!
//! These structures are designed to be directly uploaded to GPU buffers
//! with proper alignment and layout for shader access.

use super::{
    ScreenVisibility, VisibilityBlendMode, VisibilityEffect, VisibilityKind, VisibilityQuality,
    VisibilityRegion, VisibilityShape,
};
use bytemuck::{Pod, Zeroable};

/// GPU-friendly visibility region uniform.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VisibilityRegionUniform {
    /// Center position (XYZ) + falloff (W).
    pub center_falloff: [f32; 4],
    /// Extents (XYZ) + shape (W as f32).
    pub extents_shape: [f32; 4],
    /// Gradient direction (XYZ) + gradient strength (W).
    pub gradient: [f32; 4],
}

impl From<VisibilityRegion> for VisibilityRegionUniform {
    fn from(region: VisibilityRegion) -> Self {
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
            gradient: [
                region.gradient_direction.x,
                region.gradient_direction.y,
                region.gradient_direction.z,
                region.gradient_strength,
            ],
        }
    }
}

impl Default for VisibilityRegionUniform {
    fn default() -> Self {
        VisibilityRegion::default().into()
    }
}

/// GPU-friendly visibility effect uniform.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VisibilityEffectUniform {
    /// Effect color (RGB) + obscurance (A).
    pub color_obscurance: [f32; 4],
    /// Contrast, visibility range, animation speed, noise intensity.
    pub params: [f32; 4],
    /// Kind (as u32), active (as u32), padding.
    pub flags: [u32; 4],
}

impl From<VisibilityEffect> for VisibilityEffectUniform {
    fn from(effect: VisibilityEffect) -> Self {
        Self {
            color_obscurance: [
                effect.color.x,
                effect.color.y,
                effect.color.z,
                effect.obscurance,
            ],
            params: [
                effect.contrast,
                effect.visibility_range,
                effect.animation_speed,
                effect.noise_intensity,
            ],
            flags: [effect.kind as u32, u32::from(effect.active), 0, 0],
        }
    }
}

impl Default for VisibilityEffectUniform {
    fn default() -> Self {
        VisibilityEffect::default().into()
    }
}

/// GPU-friendly screen visibility uniform.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ScreenVisibilityUniform {
    /// Strength, vignette strength, vignette radius, noise scale.
    pub params_a: [f32; 4],
    /// Noise speed, center (XY), phase.
    pub params_b: [f32; 4],
    /// Color shift (RGB) + contrast adjust.
    pub color_contrast: [f32; 4],
    /// Quality (as u32), blend mode (as u32), enabled (as u32), padding.
    pub flags: [u32; 4],
}

impl From<ScreenVisibility> for ScreenVisibilityUniform {
    fn from(screen: ScreenVisibility) -> Self {
        Self {
            params_a: [
                screen.strength,
                screen.vignette_strength,
                screen.vignette_radius,
                screen.noise_scale,
            ],
            params_b: [
                screen.noise_speed,
                screen.center.0,
                screen.center.1,
                screen.phase,
            ],
            color_contrast: [
                screen.color_shift.x,
                screen.color_shift.y,
                screen.color_shift.z,
                screen.contrast_adjust,
            ],
            flags: [
                screen.quality as u32,
                screen.blend_mode as u32,
                u32::from(screen.enabled),
                0,
            ],
        }
    }
}

impl Default for ScreenVisibilityUniform {
    fn default() -> Self {
        ScreenVisibility::default().into()
    }
}

/// Combined visibility instance for GPU instanced rendering.
///
/// 144 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct VisibilityInstanceUniform {
    /// Region data.
    pub region: VisibilityRegionUniform,
    /// Effect data.
    pub effect: VisibilityEffectUniform,
    /// Screen overlay data.
    pub screen: ScreenVisibilityUniform,
}

impl VisibilityInstanceUniform {
    /// Create a new instance uniform.
    #[must_use]
    pub fn new(
        region: VisibilityRegion,
        effect: VisibilityEffect,
        screen: ScreenVisibility,
    ) -> Self {
        Self {
            region: region.into(),
            effect: effect.into(),
            screen: screen.into(),
        }
    }
}

/// Batch of visibility instances for GPU upload.
#[derive(Debug, Clone)]
pub struct VisibilityBatch {
    /// Instance data.
    pub instances: Vec<VisibilityInstanceUniform>,
    /// Maximum number of active instances.
    pub max_instances: usize,
}

impl VisibilityBatch {
    /// Create a new batch with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            max_instances: capacity,
        }
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Add an instance if there's room.
    pub fn push(&mut self, instance: VisibilityInstanceUniform) -> bool {
        if self.instances.len() < self.max_instances {
            self.instances.push(instance);
            true
        } else {
            false
        }
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
}

impl Default for VisibilityBatch {
    fn default() -> Self {
        Self::new(64)
    }
}

/// Utility to convert enum values to GPU-compatible formats.
pub mod convert {
    use super::{VisibilityBlendMode, VisibilityKind, VisibilityQuality, VisibilityShape};

    /// Convert visibility kind to u32 for shader.
    #[must_use]
    pub fn kind_to_u32(kind: VisibilityKind) -> u32 {
        kind as u32
    }

    /// Convert visibility shape to u32 for shader.
    #[must_use]
    pub fn shape_to_u32(shape: VisibilityShape) -> u32 {
        shape as u32
    }

    /// Convert blend mode to u32 for shader.
    #[must_use]
    pub fn blend_to_u32(blend: VisibilityBlendMode) -> u32 {
        blend as u32
    }

    /// Convert quality to u32 for shader.
    #[must_use]
    pub fn quality_to_u32(quality: VisibilityQuality) -> u32 {
        quality as u32
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ScreenVisibility, ScreenVisibilityUniform, VisibilityBatch, VisibilityBlendMode,
        VisibilityEffect, VisibilityEffectUniform, VisibilityInstanceUniform, VisibilityKind,
        VisibilityQuality, VisibilityRegion, VisibilityRegionUniform, VisibilityShape, convert,
    };
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn test_region_uniform_conversion() {
        let region = VisibilityRegion::new_sphere(Vec3::new(1.0, 2.0, 3.0), 5.0)
            .with_falloff(1.5)
            .with_priority(10)
            .with_gradient_strength(0.5)
            .with_gradient_direction(Vec3::Y);
        let uniform: VisibilityRegionUniform = region.into();

        assert_relative_eq!(uniform.center_falloff[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.center_falloff[3], 1.5, epsilon = 0.001);
        assert_relative_eq!(uniform.extents_shape[0], 5.0, epsilon = 0.001);
        assert_relative_eq!(
            uniform.extents_shape[3],
            f32::from(VisibilityShape::Sphere as u8),
            epsilon = 0.001
        );
        assert_relative_eq!(uniform.gradient[1], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.gradient[3], 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_effect_uniform_conversion() {
        let effect = VisibilityEffect::murk()
            .with_obscurance(0.8)
            .with_contrast(1.5);
        let uniform: VisibilityEffectUniform = effect.into();

        assert_relative_eq!(uniform.color_obscurance[3], 0.8, epsilon = 0.001);
        assert_relative_eq!(uniform.params[0], 1.5, epsilon = 0.001);
        assert_eq!(uniform.flags[0], VisibilityKind::Murk as u32);
        assert_eq!(uniform.flags[1], 1);
    }

    #[test]
    fn test_screen_uniform_conversion() {
        let screen = ScreenVisibility::smoke()
            .with_strength(0.9)
            .with_center(0.3, 0.7);
        let uniform: ScreenVisibilityUniform = screen.into();

        assert_relative_eq!(uniform.params_a[0], 0.9, epsilon = 0.001);
        assert_relative_eq!(uniform.params_b[1], 0.3, epsilon = 0.001);
        assert_relative_eq!(uniform.params_b[2], 0.7, epsilon = 0.001);
    }

    #[test]
    fn test_instance_uniform_creation() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);
        let effect = VisibilityEffect::smoke();
        let screen = ScreenVisibility::default();

        let instance = VisibilityInstanceUniform::new(region, effect, screen);

        assert_relative_eq!(instance.region.extents_shape[0], 10.0, epsilon = 0.001);
        assert_eq!(instance.effect.flags[0], VisibilityKind::Smoke as u32);
    }

    #[test]
    fn test_region_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<VisibilityRegionUniform>() % 16,
            0,
            "region uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_effect_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<VisibilityEffectUniform>() % 16,
            0,
            "effect uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_screen_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<ScreenVisibilityUniform>() % 16,
            0,
            "screen uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_instance_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<VisibilityInstanceUniform>() % 16,
            0,
            "instance uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_batch_operations() {
        let mut batch = VisibilityBatch::new(2);
        assert!(batch.is_empty());

        let instance = VisibilityInstanceUniform::default();
        assert!(batch.push(instance));
        assert!(batch.push(instance));
        assert!(!batch.push(instance));

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());

        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn test_batch_as_bytes() {
        let mut batch = VisibilityBatch::new(4);
        batch.push(VisibilityInstanceUniform::default());

        let bytes = batch.as_bytes();
        assert_eq!(
            bytes.len(),
            std::mem::size_of::<VisibilityInstanceUniform>()
        );
    }

    #[test]
    fn test_convert_utilities() {
        assert_eq!(convert::kind_to_u32(VisibilityKind::Darkness), 0);
        assert_eq!(convert::kind_to_u32(VisibilityKind::Smoke), 3);
        assert_eq!(convert::shape_to_u32(VisibilityShape::Sphere), 1);
        assert_eq!(convert::blend_to_u32(VisibilityBlendMode::Darken), 0);
        assert_eq!(convert::quality_to_u32(VisibilityQuality::Medium), 1);
    }

    #[test]
    fn test_gradient_in_region_uniform() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0)
            .with_gradient_direction(Vec3::new(0.0, 1.0, 0.0))
            .with_gradient_strength(0.8);
        let uniform: VisibilityRegionUniform = region.into();

        assert_relative_eq!(uniform.gradient[1], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.gradient[3], 0.8, epsilon = 0.001);
    }

    #[test]
    fn test_default_uniforms() {
        let region = VisibilityRegionUniform::default();
        let effect = VisibilityEffectUniform::default();
        let screen = ScreenVisibilityUniform::default();
        let instance = VisibilityInstanceUniform::default();

        assert_relative_eq!(
            region.extents_shape[3],
            f32::from(VisibilityShape::Sphere as u8),
            epsilon = 0.0001
        );
        assert_eq!(effect.flags[0], VisibilityKind::Smoke as u32);
        assert_eq!(screen.flags[2], 1);
        assert_eq!(instance.effect.flags[0], VisibilityKind::Smoke as u32);
    }
}

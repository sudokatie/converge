//! GPU-friendly uniform structures for distortion effects.
//!
//! These structures are designed to be directly uploaded to GPU buffers
//! with proper alignment and layout for shader access.

use super::{
    BlendMode, DistortionEffect, DistortionKind, DistortionQuality, DistortionRegion,
    DistortionShape, FlowDirection, ScreenDistortion,
};
use bytemuck::{Pod, Zeroable};

/// GPU-friendly distortion region uniform.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DistortionRegionUniform {
    /// Center position (XYZ) + falloff (W).
    pub center_falloff: [f32; 4],
    /// Extents (XYZ) + shape (W as f32).
    pub extents_shape: [f32; 4],
    /// Expansion rate, creation time, priority (as f32), padding.
    pub dynamics: [f32; 4],
}

impl From<DistortionRegion> for DistortionRegionUniform {
    fn from(region: DistortionRegion) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "priority is small; precision loss acceptable"
        )]
        let priority = region.priority as f32;

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
            dynamics: [region.expansion_rate, region.creation_time, priority, 0.0],
        }
    }
}

impl Default for DistortionRegionUniform {
    fn default() -> Self {
        DistortionRegion::default().into()
    }
}

/// GPU-friendly distortion effect uniform.
///
/// 32 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DistortionEffectUniform {
    /// Strength, frequency, animation speed, phase.
    pub params: [f32; 4],
    /// Flow direction (XYZ) + secondary amplitude (W).
    pub flow_secondary: [f32; 4],
    /// Kind (as u32), active (as u32), padding.
    pub flags: [u32; 4],
}

impl From<DistortionEffect> for DistortionEffectUniform {
    fn from(effect: DistortionEffect) -> Self {
        Self {
            params: [
                effect.strength,
                effect.frequency,
                effect.animation_speed,
                effect.phase,
            ],
            flow_secondary: [
                effect.flow_direction.x,
                effect.flow_direction.y,
                effect.flow_direction.z,
                effect.secondary_amplitude,
            ],
            flags: [effect.kind as u32, u32::from(effect.active), 0, 0],
        }
    }
}

impl Default for DistortionEffectUniform {
    fn default() -> Self {
        DistortionEffect::default().into()
    }
}

/// GPU-friendly screen distortion uniform.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ScreenDistortionUniform {
    /// Strength, frequency, flow speed, edge falloff.
    pub params_a: [f32; 4],
    /// Center (XY), phase, max displacement.
    pub params_b: [f32; 4],
    /// Flow direction (as u32), quality (as u32), blend mode (as u32), enabled (as u32).
    pub flags: [u32; 4],
}

impl From<ScreenDistortion> for ScreenDistortionUniform {
    fn from(screen: ScreenDistortion) -> Self {
        Self {
            params_a: [
                screen.strength,
                screen.frequency,
                screen.flow_speed,
                screen.edge_falloff,
            ],
            params_b: [
                screen.center.0,
                screen.center.1,
                screen.phase,
                screen.max_displacement,
            ],
            flags: [
                screen.flow_direction as u32,
                screen.quality as u32,
                screen.blend_mode as u32,
                u32::from(screen.enabled),
            ],
        }
    }
}

impl Default for ScreenDistortionUniform {
    fn default() -> Self {
        ScreenDistortion::default().into()
    }
}

/// Combined distortion instance for GPU instanced rendering.
///
/// 128 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct DistortionInstanceUniform {
    /// Region data.
    pub region: DistortionRegionUniform,
    /// Effect data.
    pub effect: DistortionEffectUniform,
    /// Screen overlay data.
    pub screen: ScreenDistortionUniform,
}

impl DistortionInstanceUniform {
    /// Create a new instance uniform.
    #[must_use]
    pub fn new(
        region: DistortionRegion,
        effect: DistortionEffect,
        screen: ScreenDistortion,
    ) -> Self {
        Self {
            region: region.into(),
            effect: effect.into(),
            screen: screen.into(),
        }
    }
}

/// Batch of distortion instances for GPU upload.
#[derive(Debug, Clone)]
pub struct DistortionBatch {
    /// Instance data.
    pub instances: Vec<DistortionInstanceUniform>,
    /// Maximum number of active instances.
    pub max_instances: usize,
}

impl DistortionBatch {
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
    pub fn push(&mut self, instance: DistortionInstanceUniform) -> bool {
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

impl Default for DistortionBatch {
    fn default() -> Self {
        Self::new(64)
    }
}

/// Utility to convert enum values to GPU-compatible formats.
pub mod convert {
    use super::{BlendMode, DistortionKind, DistortionQuality, DistortionShape, FlowDirection};

    /// Convert distortion kind to u32 for shader.
    #[must_use]
    pub fn kind_to_u32(kind: DistortionKind) -> u32 {
        kind as u32
    }

    /// Convert distortion shape to u32 for shader.
    #[must_use]
    pub fn shape_to_u32(shape: DistortionShape) -> u32 {
        shape as u32
    }

    /// Convert flow direction to u32 for shader.
    #[must_use]
    pub fn flow_to_u32(flow: FlowDirection) -> u32 {
        flow as u32
    }

    /// Convert blend mode to u32 for shader.
    #[must_use]
    pub fn blend_to_u32(blend: BlendMode) -> u32 {
        blend as u32
    }

    /// Convert quality to u32 for shader.
    #[must_use]
    pub fn quality_to_u32(quality: DistortionQuality) -> u32 {
        quality as u32
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlendMode, DistortionBatch, DistortionEffect, DistortionEffectUniform,
        DistortionInstanceUniform, DistortionKind, DistortionQuality, DistortionRegion,
        DistortionRegionUniform, DistortionShape, FlowDirection, ScreenDistortion,
        ScreenDistortionUniform, convert,
    };
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn test_region_uniform_conversion() {
        let region = DistortionRegion::new_sphere(Vec3::new(1.0, 2.0, 3.0), 5.0)
            .with_falloff(1.5)
            .with_priority(10);
        let uniform: DistortionRegionUniform = region.into();

        assert_relative_eq!(uniform.center_falloff[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.center_falloff[3], 1.5, epsilon = 0.001);
        assert_relative_eq!(uniform.extents_shape[0], 5.0, epsilon = 0.001);
        assert_relative_eq!(
            uniform.extents_shape[3],
            f32::from(DistortionShape::Sphere as u8),
            epsilon = 0.001
        );
        assert_relative_eq!(uniform.dynamics[2], 10.0, epsilon = 0.001);
    }

    #[test]
    fn test_effect_uniform_conversion() {
        let effect = DistortionEffect::heat_shimmer()
            .with_strength(0.7)
            .with_frequency(5.0);
        let uniform: DistortionEffectUniform = effect.into();

        assert_relative_eq!(uniform.params[0], 0.7, epsilon = 0.001);
        assert_relative_eq!(uniform.params[1], 5.0, epsilon = 0.001);
        assert_eq!(uniform.flags[0], DistortionKind::HeatShimmer as u32);
        assert_eq!(uniform.flags[1], 1);
    }

    #[test]
    fn test_screen_uniform_conversion() {
        let screen = ScreenDistortion::pressure_wave()
            .with_strength(0.9)
            .with_center(0.3, 0.7);
        let uniform: ScreenDistortionUniform = screen.into();

        assert_relative_eq!(uniform.params_a[0], 0.9, epsilon = 0.001);
        assert_relative_eq!(uniform.params_b[0], 0.3, epsilon = 0.001);
        assert_relative_eq!(uniform.params_b[1], 0.7, epsilon = 0.001);
    }

    #[test]
    fn test_instance_uniform_creation() {
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);
        let effect = DistortionEffect::heat_shimmer();
        let screen = ScreenDistortion::default();

        let instance = DistortionInstanceUniform::new(region, effect, screen);

        assert_relative_eq!(instance.region.extents_shape[0], 10.0, epsilon = 0.001);
        assert_eq!(instance.effect.flags[0], DistortionKind::HeatShimmer as u32);
    }

    #[test]
    fn test_region_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<DistortionRegionUniform>() % 16,
            0,
            "region uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_effect_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<DistortionEffectUniform>() % 16,
            0,
            "effect uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_screen_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<ScreenDistortionUniform>() % 16,
            0,
            "screen uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_instance_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<DistortionInstanceUniform>() % 16,
            0,
            "instance uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_batch_operations() {
        let mut batch = DistortionBatch::new(2);
        assert!(batch.is_empty());

        let instance = DistortionInstanceUniform::default();
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
        let mut batch = DistortionBatch::new(4);
        batch.push(DistortionInstanceUniform::default());

        let bytes = batch.as_bytes();
        assert_eq!(
            bytes.len(),
            std::mem::size_of::<DistortionInstanceUniform>()
        );
    }

    #[test]
    fn test_convert_utilities() {
        assert_eq!(convert::kind_to_u32(DistortionKind::HeatShimmer), 0);
        assert_eq!(convert::kind_to_u32(DistortionKind::PressureWave), 1);
        assert_eq!(convert::shape_to_u32(DistortionShape::Sphere), 1);
        assert_eq!(convert::flow_to_u32(FlowDirection::Up), 1);
        assert_eq!(convert::blend_to_u32(BlendMode::Offset), 0);
        assert_eq!(convert::quality_to_u32(DistortionQuality::Medium), 1);
    }

    #[test]
    fn test_expanding_region_dynamics() {
        let region =
            DistortionRegion::new_expanding_sphere(Vec3::ZERO, 5.0, 20.0).with_creation_time(1.5);
        let uniform: DistortionRegionUniform = region.into();

        assert_relative_eq!(uniform.dynamics[0], 20.0, epsilon = 0.001);
        assert_relative_eq!(uniform.dynamics[1], 1.5, epsilon = 0.001);
    }

    #[test]
    fn test_default_uniforms() {
        let region = DistortionRegionUniform::default();
        let effect = DistortionEffectUniform::default();
        let screen = ScreenDistortionUniform::default();
        let instance = DistortionInstanceUniform::default();

        assert_relative_eq!(
            region.extents_shape[3],
            f32::from(DistortionShape::Sphere as u8),
            epsilon = 0.0001
        );
        assert_eq!(effect.flags[0], DistortionKind::HeatShimmer as u32);
        assert_eq!(screen.flags[3], 1);
        assert_eq!(instance.effect.flags[0], DistortionKind::HeatShimmer as u32);
    }
}

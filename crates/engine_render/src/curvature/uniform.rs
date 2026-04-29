//! GPU-friendly uniform structures for curvature rendering.
//!
//! These structures are designed to be directly uploaded to GPU buffers
//! with proper alignment and layout for shader access.

use super::{CurvatureBody, CurvatureBodyKind, HorizonConfig, HorizonModel, HorizonQuality};
use bytemuck::{Pod, Zeroable};

/// GPU-friendly curvature body uniform.
///
/// 80 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CurvatureBodyUniform {
    /// Center position (XYZ) + radius (W).
    pub center_radius: [f32; 4],
    /// Axis direction (XYZ) + `half_length` (W).
    pub axis_length: [f32; 4],
    /// Velocity (XYZ) + `angular_velocity` (W).
    pub velocity_omega: [f32; 4],
    /// Surface gravity, kind (as f32), active (as f32), padding.
    pub params: [f32; 4],
    /// Reserved for future use.
    pub reserved: [f32; 4],
}

impl From<CurvatureBody> for CurvatureBodyUniform {
    fn from(body: CurvatureBody) -> Self {
        Self {
            center_radius: [body.center.x, body.center.y, body.center.z, body.radius],
            axis_length: [body.axis.x, body.axis.y, body.axis.z, body.half_length],
            velocity_omega: [
                body.velocity.x,
                body.velocity.y,
                body.velocity.z,
                body.angular_velocity,
            ],
            params: [
                body.surface_gravity,
                f32::from(body.kind as u8),
                if body.active { 1.0 } else { 0.0 },
                0.0,
            ],
            reserved: [0.0; 4],
        }
    }
}

impl Default for CurvatureBodyUniform {
    fn default() -> Self {
        CurvatureBody::default().into()
    }
}

/// GPU-friendly horizon model uniform.
///
/// 64 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HorizonModelUniform {
    /// Camera position (XYZ) + camera height (W).
    pub camera_height: [f32; 4],
    /// Horizon center direction (XYZ) + horizon distance (W).
    pub direction_distance: [f32; 4],
    /// Horizon dip angle, effective radius, `above_surface` (as f32), padding.
    pub params: [f32; 4],
    /// Reserved for future use.
    pub reserved: [f32; 4],
}

impl From<HorizonModel> for HorizonModelUniform {
    fn from(model: HorizonModel) -> Self {
        Self {
            camera_height: [
                model.camera_position.x,
                model.camera_position.y,
                model.camera_position.z,
                model.camera_height,
            ],
            direction_distance: [
                model.horizon_center_dir.x,
                model.horizon_center_dir.y,
                model.horizon_center_dir.z,
                model.horizon_distance,
            ],
            params: [
                model.horizon_dip_angle,
                model.effective_radius,
                if model.above_surface { 1.0 } else { 0.0 },
                0.0,
            ],
            reserved: [0.0; 4],
        }
    }
}

impl Default for HorizonModelUniform {
    fn default() -> Self {
        Self {
            camera_height: [0.0, 0.0, 0.0, 0.0],
            direction_distance: [0.0, 1.0, 0.0, 0.0],
            params: [0.0, 1000.0, 1.0, 0.0],
            reserved: [0.0; 4],
        }
    }
}

/// GPU-friendly horizon config uniform.
///
/// 32 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HorizonConfigUniform {
    /// Max distance, fade start, fade end, blend angle.
    pub fade_params: [f32; 4],
    /// Quality (as f32), `atmospheric_scattering` (as f32), enabled (as f32), `debug_line_width`.
    pub flags: [f32; 4],
}

impl From<HorizonConfig> for HorizonConfigUniform {
    fn from(config: HorizonConfig) -> Self {
        Self {
            fade_params: [
                config.max_distance,
                config.fade_start,
                config.fade_end,
                config.blend_angle.to_radians(),
            ],
            flags: [
                f32::from(config.quality as u8),
                if config.atmospheric_scattering {
                    1.0
                } else {
                    0.0
                },
                if config.enabled { 1.0 } else { 0.0 },
                config.debug_line_width,
            ],
        }
    }
}

impl Default for HorizonConfigUniform {
    fn default() -> Self {
        HorizonConfig::default().into()
    }
}

/// Combined curvature instance for GPU rendering.
///
/// 176 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct CurvatureInstanceUniform {
    /// Body data.
    pub body: CurvatureBodyUniform,
    /// Horizon model data.
    pub horizon: HorizonModelUniform,
    /// Horizon config data.
    pub config: HorizonConfigUniform,
}

impl CurvatureInstanceUniform {
    /// Create a new instance uniform.
    #[must_use]
    pub fn new(body: CurvatureBody, model: HorizonModel, config: HorizonConfig) -> Self {
        Self {
            body: body.into(),
            horizon: model.into(),
            config: config.into(),
        }
    }
}

/// Batch of curvature instances for GPU upload.
#[derive(Debug, Clone)]
pub struct CurvatureBatch {
    /// Instance data.
    pub instances: Vec<CurvatureInstanceUniform>,
    /// Maximum number of active instances.
    pub max_instances: usize,
}

impl CurvatureBatch {
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
    pub fn push(&mut self, instance: CurvatureInstanceUniform) -> bool {
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

impl Default for CurvatureBatch {
    fn default() -> Self {
        Self::new(16)
    }
}

/// Fog parameters adjusted for curvature.
///
/// 32 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CurvatureFogUniform {
    /// Base density, horizon density boost, near plane, far plane.
    pub density_planes: [f32; 4],
    /// Fog color (RGB) + inscatter strength.
    pub color_inscatter: [f32; 4],
}

impl CurvatureFogUniform {
    /// Create fog parameters for a horizon.
    #[must_use]
    pub fn from_horizon(
        model: &HorizonModel,
        base_density: f32,
        horizon_boost: f32,
        color: [f32; 3],
    ) -> Self {
        let near = 1.0;
        let far = model.horizon_distance.min(100_000.0);
        let inscatter = if model.above_surface { 0.1 } else { 0.0 };

        Self {
            density_planes: [base_density, horizon_boost, near, far],
            color_inscatter: [color[0], color[1], color[2], inscatter],
        }
    }
}

impl Default for CurvatureFogUniform {
    fn default() -> Self {
        Self {
            density_planes: [0.0001, 0.001, 1.0, 10000.0],
            color_inscatter: [0.7, 0.8, 0.9, 0.1],
        }
    }
}

/// Clip plane parameters for horizon-aware rendering.
///
/// 16 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CurvatureClipUniform {
    /// Near clip, far clip, horizon clip, padding.
    pub planes: [f32; 4],
}

impl CurvatureClipUniform {
    /// Create clip parameters from horizon model.
    #[must_use]
    pub fn from_horizon(model: &HorizonModel, config: &HorizonConfig) -> Self {
        let near = 0.1;
        let far = config.max_distance;
        let horizon = model.horizon_distance * config.fade_end;

        Self {
            planes: [near, far, horizon.min(far), 0.0],
        }
    }
}

impl Default for CurvatureClipUniform {
    fn default() -> Self {
        Self {
            planes: [0.1, 100_000.0, 50_000.0, 0.0],
        }
    }
}

/// Utility to convert enum values to GPU-compatible formats.
pub mod convert {
    use super::{CurvatureBodyKind, HorizonQuality};

    /// Convert body kind to u32 for shader.
    #[must_use]
    pub fn body_kind_to_u32(kind: CurvatureBodyKind) -> u32 {
        kind as u32
    }

    /// Convert horizon quality to u32 for shader.
    #[must_use]
    pub fn quality_to_u32(quality: HorizonQuality) -> u32 {
        quality as u32
    }

    /// Convert bool to shader-compatible f32.
    #[must_use]
    pub fn bool_to_f32(value: bool) -> f32 {
        if value { 1.0 } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn test_body_uniform_conversion() {
        let body = CurvatureBody::planetary_sphere(Vec3::new(1.0, 2.0, 3.0), 1000.0)
            .with_surface_gravity(9.81);
        let uniform: CurvatureBodyUniform = body.into();

        assert_relative_eq!(uniform.center_radius[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.center_radius[3], 1000.0, epsilon = 0.001);
        assert_relative_eq!(uniform.params[0], 9.81, epsilon = 0.001);
        assert_relative_eq!(
            uniform.params[1],
            f32::from(CurvatureBodyKind::PlanetarySphere as u8),
            epsilon = 0.001
        );
    }

    #[test]
    fn test_horizon_model_uniform_conversion() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let uniform: HorizonModelUniform = model.into();

        assert_relative_eq!(uniform.camera_height[0], 1100.0, epsilon = 0.001);
        assert_relative_eq!(uniform.camera_height[3], 100.0, epsilon = 1.0);
        assert!(uniform.direction_distance[3] > 0.0);
        assert_relative_eq!(uniform.params[2], 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_horizon_config_uniform_conversion() {
        let config = HorizonConfig::default()
            .with_quality(HorizonQuality::High)
            .with_max_distance(50_000.0)
            .with_atmospheric_scattering(true);
        let uniform: HorizonConfigUniform = config.into();

        assert_relative_eq!(uniform.fade_params[0], 50_000.0, epsilon = 0.1);
        assert_relative_eq!(
            uniform.flags[0],
            f32::from(HorizonQuality::High as u8),
            epsilon = 0.001
        );
        assert_relative_eq!(uniform.flags[1], 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_instance_uniform_creation() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let config = HorizonConfig::default();

        let instance = CurvatureInstanceUniform::new(body, model, config);

        assert_relative_eq!(instance.body.center_radius[3], 1000.0, epsilon = 0.001);
        assert!(instance.horizon.direction_distance[3] > 0.0);
    }

    #[test]
    fn test_body_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<CurvatureBodyUniform>() % 16,
            0,
            "body uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_horizon_model_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<HorizonModelUniform>() % 16,
            0,
            "horizon model uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_horizon_config_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<HorizonConfigUniform>() % 16,
            0,
            "horizon config uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_instance_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<CurvatureInstanceUniform>() % 16,
            0,
            "instance uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_fog_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<CurvatureFogUniform>() % 16,
            0,
            "fog uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_clip_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<CurvatureClipUniform>() % 16,
            0,
            "clip uniform should be 16-byte aligned"
        );
    }

    #[test]
    fn test_batch_operations() {
        let mut batch = CurvatureBatch::new(2);
        assert!(batch.is_empty());

        let instance = CurvatureInstanceUniform::default();
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
        let mut batch = CurvatureBatch::new(4);
        batch.push(CurvatureInstanceUniform::default());

        let bytes = batch.as_bytes();
        assert_eq!(bytes.len(), std::mem::size_of::<CurvatureInstanceUniform>());
    }

    #[test]
    fn test_fog_uniform_from_horizon() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        let fog = CurvatureFogUniform::from_horizon(&model, 0.001, 0.01, [0.7, 0.8, 0.9]);

        assert_relative_eq!(fog.density_planes[0], 0.001, epsilon = 0.0001);
        assert_relative_eq!(fog.density_planes[1], 0.01, epsilon = 0.0001);
        assert!(fog.density_planes[3] > 0.0);
    }

    #[test]
    fn test_clip_uniform_from_horizon() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let config = HorizonConfig::default();

        let clip = CurvatureClipUniform::from_horizon(&model, &config);

        assert!(clip.planes[0] < clip.planes[1]);
        assert!(clip.planes[2] <= clip.planes[1]);
    }

    #[test]
    fn test_convert_utilities() {
        assert_eq!(
            convert::body_kind_to_u32(CurvatureBodyKind::PlanetarySphere),
            0
        );
        assert_eq!(convert::body_kind_to_u32(CurvatureBodyKind::Cylinder), 2);
        assert_eq!(convert::quality_to_u32(HorizonQuality::Medium), 1);
        assert_relative_eq!(convert::bool_to_f32(true), 1.0, epsilon = 0.001);
        assert_relative_eq!(convert::bool_to_f32(false), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_default_uniforms() {
        let body = CurvatureBodyUniform::default();
        let horizon = HorizonModelUniform::default();
        let config = HorizonConfigUniform::default();
        let fog = CurvatureFogUniform::default();
        let clip = CurvatureClipUniform::default();
        let instance = CurvatureInstanceUniform::default();

        assert!(body.center_radius[3] > 0.0);
        assert!(horizon.params[1] > 0.0);
        assert!(config.fade_params[0] > 0.0);
        assert!(fog.density_planes[3] > 0.0);
        assert!(clip.planes[1] > clip.planes[0]);
        assert!(instance.body.center_radius[3] > 0.0);
    }
}

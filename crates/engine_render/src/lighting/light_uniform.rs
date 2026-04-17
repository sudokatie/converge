//! Light uniform buffer for GPU upload.
//!
//! Combines directional and point light data into a single uniform buffer
//! that can be bound to the voxel rendering pipeline.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use super::directional::DirectionalLight;
use super::point_light::{PointLightGpuData, PointLightManager, MAX_POINT_LIGHTS};

/// GPU-friendly directional light data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DirectionalLightGpuData {
    /// Normalized light direction.
    pub direction: [f32; 3],
    /// Light color (linear RGB, pre-multiplied by intensity).
    pub color: [f32; 3],
    /// Shadow map view-projection matrix (row-major).
    pub light_view_proj: [[f32; 4]; 4],
}

impl From<&DirectionalLight> for DirectionalLightGpuData {
    fn from(light: &DirectionalLight) -> Self {
        Self {
            direction: light.direction.to_array(),
            color: light.color.to_array(),
            light_view_proj: light.light_view_proj.to_cols_array_2d(),
        }
    }
}

/// Combined light uniform data for the shader.
///
/// Matches the expected layout in WGSL:
/// ```wgsl
/// struct LightUniform {
///     dir_direction: vec3<f32>,
///     dir_color: vec3<f32>,
///     dir_light_view_proj: mat4x4<f32>,
///     ambient_color: vec3<f32>,
///     ambient_intensity: f32,
///     num_point_lights: u32,
///     _padding: [u32; 3],
///     point_lights: array<PointLightData, 64>,
/// }
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightUniform {
    /// Directional light direction.
    pub dir_direction: [f32; 3],
    /// Directional light color.
    pub dir_color: [f32; 3],
    /// Directional light shadow view-projection matrix.
    pub dir_light_view_proj: [[f32; 4]; 4],
    /// Ambient light color.
    pub ambient_color: [f32; 3],
    /// Ambient light intensity.
    pub ambient_intensity: f32,
    /// Number of active point lights.
    pub num_point_lights: u32,
    /// Padding for alignment.
    pub _padding0: [u32; 3],
    /// Point light data (max 64).
    pub point_lights: [PointLightGpuData; MAX_POINT_LIGHTS],
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            dir_direction: [0.3, 1.0, 0.2],
            dir_color: [1.0; 3],
            dir_light_view_proj: [[0.0; 4]; 4],
            ambient_color: [0.15, 0.15, 0.2],
            ambient_intensity: 0.3,
            num_point_lights: 0,
            _padding0: [0; 3],
            point_lights: [PointLightGpuData::zeroed(); MAX_POINT_LIGHTS],
        }
    }
}

impl LightUniform {
    /// Create a new light uniform from directional light and point light manager.
    pub fn new(directional: &DirectionalLight, point_manager: &PointLightManager) -> Self {
        let mut uniform = Self {
            dir_direction: directional.direction.to_array(),
            dir_color: directional.color.to_array(),
            dir_light_view_proj: directional.light_view_proj.to_cols_array_2d(),
            ..Default::default()
        };

        // Set ambient based on sun elevation
        let sun_up = directional.direction.y.max(0.0);
        uniform.ambient_intensity = 0.1 + 0.3 * sun_up;
        uniform.ambient_color = if sun_up > 0.0 {
            [0.6, 0.65, 0.8] // Daytime ambient (sky blue)
        } else {
            [0.05, 0.05, 0.1] // Nighttime ambient (dark blue)
        };

        // Copy active point lights
        let active_data = point_manager.active_light_data();
        uniform.num_point_lights = active_data.len() as u32;
        for (i, light_data) in active_data.into_iter().enumerate() {
            if i < MAX_POINT_LIGHTS {
                uniform.point_lights[i] = light_data;
            }
        }

        uniform
    }

    /// Update from new light data.
    pub fn update(
        &mut self,
        directional: &DirectionalLight,
        point_manager: &PointLightManager,
    ) {
        self.dir_direction = directional.direction.to_array();
        self.dir_color = directional.color.to_array();
        self.dir_light_view_proj = directional.light_view_proj.to_cols_array_2d();

        // Update ambient
        let sun_up = directional.direction.y.max(0.0);
        self.ambient_intensity = 0.1 + 0.3 * sun_up;
        self.ambient_color = if sun_up > 0.0 {
            [0.6, 0.65, 0.8]
        } else {
            [0.05, 0.05, 0.1]
        };

        // Update point lights
        let active_data = point_manager.active_light_data();
        self.num_point_lights = active_data.len() as u32;

        // Clear all slots first
        for slot in &mut self.point_lights {
            *slot = PointLightGpuData::zeroed();
        }

        for (i, light_data) in active_data.into_iter().enumerate() {
            if i < MAX_POINT_LIGHTS {
                self.point_lights[i] = light_data;
            }
        }
    }
}

/// Manages the GPU buffer for light uniform data.
pub struct LightUniformBuffer {
    /// Current light uniform data.
    data: LightUniform,
    /// Whether the buffer needs to be re-uploaded.
    dirty: bool,
}

impl Default for LightUniformBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LightUniformBuffer {
    /// Create a new light uniform buffer with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: LightUniform::default(),
            dirty: true,
        }
    }

    /// Update the buffer with current light data.
    pub fn update(
        &mut self,
        directional: &DirectionalLight,
        point_manager: &PointLightManager,
    ) {
        self.data.update(directional, point_manager);
        self.dirty = true;
    }

    /// Get the raw bytes for GPU upload.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(std::slice::from_ref(&self.data))
    }

    /// Check if the buffer needs re-uploading.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the buffer as uploaded.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Get a reference to the current uniform data.
    #[must_use]
    pub fn data(&self) -> &LightUniform {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lighting::directional::DirectionalLight;
    use crate::lighting::point_light::PointLight;

    #[test]
    fn test_default_light_uniform() {
        let uniform = LightUniform::default();
        assert_eq!(uniform.num_point_lights, 0);
        assert!(uniform.ambient_intensity > 0.0);
    }

    #[test]
    fn test_light_uniform_from_lights() {
        let directional = DirectionalLight::new(
            Vec3::new(0.3, 1.0, 0.2).normalize(),
            Vec3::new(1.0, 0.9, 0.8),
        );
        let mut point_manager = PointLightManager::new();
        point_manager.add(PointLight::torch(Vec3::new(10.0, 5.0, 10.0)));
        point_manager.update_active(Vec3::ZERO);

        let uniform = LightUniform::new(&directional, &point_manager);

        // Direction should be normalized
        let dir_len = (uniform.dir_direction[0].powi(2)
            + uniform.dir_direction[1].powi(2)
            + uniform.dir_direction[2].powi(2))
        .sqrt();
        assert!(dir_len > 0.99 && dir_len < 1.01, "Direction should be normalized");

        // Should have one active point light
        assert_eq!(uniform.num_point_lights, 1);
    }

    #[test]
    fn test_light_uniform_update() {
        let mut uniform = LightUniform::default();
        let directional = DirectionalLight::default();
        let point_manager = PointLightManager::new();

        uniform.update(&directional, &point_manager);

        assert_eq!(uniform.num_point_lights, 0);
    }

    #[test]
    fn test_light_uniform_buffer_dirty_tracking() {
        let mut buffer = LightUniformBuffer::new();

        // New buffer should be dirty
        assert!(buffer.is_dirty());

        // Mark clean
        buffer.mark_clean();
        assert!(!buffer.is_dirty());

        // Update should make it dirty again
        let directional = DirectionalLight::default();
        let point_manager = PointLightManager::new();
        buffer.update(&directional, &point_manager);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn test_light_uniform_buffer_as_bytes() {
        let buffer = LightUniformBuffer::new();
        let bytes = buffer.as_bytes();

        // Should be non-empty and correctly sized
        assert!(!bytes.is_empty());
        assert_eq!(bytes.len(), std::mem::size_of::<LightUniform>());
    }

    #[test]
    fn test_nighttime_ambient() {
        let mut directional = DirectionalLight::default();
        directional.update_from_time(0.0); // Midnight

        let point_manager = PointLightManager::new();
        let uniform = LightUniform::new(&directional, &point_manager);

        // Nighttime ambient should be dim
        assert!(uniform.ambient_intensity < 0.2);
        // Nighttime ambient should be blueish
        assert!(uniform.ambient_color[2] > uniform.ambient_color[0]);
    }

    #[test]
    fn test_daytime_ambient() {
        let mut directional = DirectionalLight::default();
        directional.update_from_time(0.5); // Noon

        let point_manager = PointLightManager::new();
        let uniform = LightUniform::new(&directional, &point_manager);

        // Daytime ambient should be brighter
        assert!(uniform.ambient_intensity > 0.3);
    }
}

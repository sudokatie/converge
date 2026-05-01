//! GPU-friendly uniform structures for photo mode.
//!
//! Provides aligned data structures suitable for GPU shader uniforms
//! with conversion from CPU-side types.

use super::framing::CompositionGuide;
use super::sampling::BokehShape;
use super::settings::{PhotoFilter, PhotoSettings};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// GPU uniform for photo mode settings.
///
/// 32 bytes, 4-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PhotoSettingsUniform {
    /// Exposure multiplier.
    pub exposure: f32,
    /// Blur radius from aperture.
    pub blur_radius: f32,
    /// Focus distance.
    pub focus_distance: f32,
    /// FOV in radians.
    pub fov_radians: f32,
    /// Roll in radians.
    pub roll_radians: f32,
    /// Filter type index.
    pub filter: u32,
    /// Time frozen flag.
    pub time_frozen: u32,
    /// UI visible flag.
    pub ui_visible: u32,
}

impl PhotoSettingsUniform {
    /// Create from photo settings.
    #[must_use]
    pub fn from_settings(settings: &PhotoSettings) -> Self {
        Self {
            exposure: settings.exposure_multiplier(),
            blur_radius: settings.blur_radius(),
            focus_distance: settings.focus_distance,
            fov_radians: settings.fov.to_radians(),
            roll_radians: settings.roll.to_radians(),
            filter: settings.filter as u32,
            time_frozen: u32::from(settings.time_frozen),
            ui_visible: u32::from(settings.ui_visible),
        }
    }

    /// Get as raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// GPU uniform for DOF parameters.
///
/// 32 bytes, 4-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct DofUniform {
    /// Focus distance in world units.
    pub focus_distance: f32,
    /// Aperture f-stop.
    pub aperture: f32,
    /// Maximum `CoC` radius in pixels.
    pub max_coc: f32,
    /// Bokeh shape (0=circle, 1=hex, 2=oct, 3=square).
    pub bokeh_shape: u32,
    /// Near blur start distance.
    pub near_start: f32,
    /// Near blur end distance.
    pub near_end: f32,
    /// Far blur start distance.
    pub far_start: f32,
    /// Far blur end distance.
    pub far_end: f32,
}

impl DofUniform {
    /// Create from photo settings.
    #[must_use]
    pub fn from_settings(settings: &PhotoSettings, bokeh: BokehShape) -> Self {
        let focus = settings.focus_distance;
        let dof_range = focus * 0.1 * (settings.aperture / 5.6);

        Self {
            focus_distance: focus,
            aperture: settings.aperture,
            max_coc: 30.0,
            bokeh_shape: bokeh as u32,
            near_start: (focus - dof_range * 2.0).max(0.1),
            near_end: (focus - dof_range).max(0.1),
            far_start: focus + dof_range,
            far_end: focus + dof_range * 2.0,
        }
    }

    /// Get as raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// GPU uniform for filter parameters.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct FilterUniform {
    /// Filter type index.
    pub filter_type: u32,
    /// Filter intensity (0-1).
    pub intensity: f32,
    /// Saturation adjustment.
    pub saturation: f32,
    /// Contrast adjustment.
    pub contrast: f32,
    /// Color tint RGBA.
    pub tint: [f32; 4],
    /// Vignette intensity.
    pub vignette: f32,
    /// Film grain intensity.
    pub grain: f32,
    /// Padding for GPU alignment.
    pub pad: [f32; 2],
}

impl Default for FilterUniform {
    fn default() -> Self {
        Self {
            filter_type: 0,
            intensity: 1.0,
            saturation: 1.0,
            contrast: 1.0,
            tint: [1.0, 1.0, 1.0, 1.0],
            vignette: 0.0,
            grain: 0.0,
            pad: [0.0; 2],
        }
    }
}

impl FilterUniform {
    /// Create from photo filter.
    #[must_use]
    pub fn from_filter(filter: PhotoFilter) -> Self {
        let (saturation, contrast, tint, vignette, grain) = match filter {
            PhotoFilter::None => (1.0, 1.0, [1.0, 1.0, 1.0, 1.0], 0.0, 0.0),
            PhotoFilter::BlackAndWhite => (0.0, 1.1, [1.0, 1.0, 1.0, 1.0], 0.2, 0.0),
            PhotoFilter::Sepia => (0.3, 1.0, [1.2, 1.0, 0.8, 1.0], 0.1, 0.0),
            PhotoFilter::Vintage => (0.8, 0.9, [1.1, 1.0, 0.9, 1.0], 0.3, 0.1),
            PhotoFilter::HighContrast => (1.2, 1.5, [1.0, 1.0, 1.0, 1.0], 0.1, 0.0),
            PhotoFilter::Cool => (1.0, 1.0, [0.9, 1.0, 1.1, 1.0], 0.0, 0.0),
            PhotoFilter::Warm => (1.0, 1.0, [1.1, 1.0, 0.9, 1.0], 0.0, 0.0),
            PhotoFilter::Cinematic => (0.9, 1.1, [1.0, 0.95, 0.9, 1.0], 0.15, 0.02),
            PhotoFilter::Neon => (1.5, 1.3, [1.0, 0.9, 1.1, 1.0], 0.1, 0.0),
            PhotoFilter::BleachBypass => (0.5, 1.4, [1.0, 1.0, 1.0, 1.0], 0.2, 0.03),
            PhotoFilter::Noir => (0.0, 1.4, [1.0, 1.0, 1.0, 1.0], 0.4, 0.05),
            PhotoFilter::Dream => (0.9, 0.8, [1.05, 1.0, 1.1, 1.0], 0.1, 0.0),
        };

        Self {
            filter_type: filter as u32,
            intensity: 1.0,
            saturation,
            contrast,
            tint,
            vignette,
            grain,
            pad: [0.0; 2],
        }
    }

    /// Get as raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// GPU uniform for composition guide rendering.
///
/// 32 bytes, 4-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct CompositionGuideUniform {
    /// Guide type index.
    pub guide_type: u32,
    /// Line thickness.
    pub line_thickness: f32,
    /// Power point size.
    pub point_size: f32,
    /// Opacity.
    pub opacity: f32,
    /// Line color RGBA.
    pub line_color: [f32; 4],
}

impl Default for CompositionGuideUniform {
    fn default() -> Self {
        Self {
            guide_type: 0,
            line_thickness: 1.0,
            point_size: 4.0,
            opacity: 0.7,
            line_color: [1.0, 1.0, 1.0, 0.5],
        }
    }
}

impl CompositionGuideUniform {
    /// Create from composition guide.
    #[must_use]
    pub fn from_guide(guide: CompositionGuide) -> Self {
        Self {
            guide_type: guide as u32,
            line_thickness: 1.0,
            point_size: 4.0,
            opacity: 0.7,
            line_color: [1.0, 1.0, 1.0, 0.5],
        }
    }

    /// Set line color.
    #[must_use]
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.line_color = [r, g, b, a];
        self
    }

    /// Get as raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// GPU uniform for camera path playback.
///
/// 112 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct CameraPathUniform {
    /// Camera position XYZ + W=1.
    pub position: [f32; 4],
    /// Camera forward direction XYZ + W=0.
    pub forward: [f32; 4],
    /// Camera up direction XYZ + W=0.
    pub up: [f32; 4],
    /// Camera right direction XYZ + W=0.
    pub right: [f32; 4],
    /// FOV, near, far, aspect.
    pub projection_params: [f32; 4],
    /// Velocity XYZ for motion blur + W=0.
    pub velocity: [f32; 4],
    /// Time, progress, speed, flags.
    pub playback: [f32; 4],
}

impl Default for CameraPathUniform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            forward: [0.0, 0.0, -1.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            right: [1.0, 0.0, 0.0, 0.0],
            projection_params: [1.22, 0.1, 1000.0, 1.78],
            velocity: [0.0; 4],
            playback: [0.0; 4],
        }
    }
}

impl CameraPathUniform {
    /// Create from interpolated camera state.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "GPU uniform constructor: all parameters map directly to uniform fields"
    )]
    pub fn from_camera(
        position: Vec3,
        forward: Vec3,
        up: Vec3,
        fov: f32,
        near: f32,
        far: f32,
        aspect: f32,
        velocity: Vec3,
        time: f32,
        progress: f32,
        speed: f32,
    ) -> Self {
        let right = up.cross(forward).normalize();

        Self {
            position: [position.x, position.y, position.z, 1.0],
            forward: [forward.x, forward.y, forward.z, 0.0],
            up: [up.x, up.y, up.z, 0.0],
            right: [right.x, right.y, right.z, 0.0],
            projection_params: [fov.to_radians(), near, far, aspect],
            velocity: [velocity.x, velocity.y, velocity.z, 0.0],
            playback: [time, progress, speed, 0.0],
        }
    }

    /// Get as raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// Batch of photo mode instances for instanced rendering.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PhotoBatch {
    instances: Vec<PhotoInstanceUniform>,
    capacity: usize,
}

impl PhotoBatch {
    /// Create new batch with capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Add an instance.
    pub fn add(&mut self, instance: PhotoInstanceUniform) -> bool {
        if self.instances.len() < self.capacity {
            self.instances.push(instance);
            true
        } else {
            false
        }
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Number of instances.
    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Get as raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        if self.instances.is_empty() {
            &[]
        } else {
            bytemuck::cast_slice(&self.instances)
        }
    }
}

/// Individual photo mode instance data.
///
/// 48 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct PhotoInstanceUniform {
    /// Position XYZ + scale W.
    pub position_scale: [f32; 4],
    /// Rotation quaternion XYZW.
    pub rotation: [f32; 4],
    /// Filter and settings indices.
    pub indices: [f32; 4],
}

impl Default for PhotoInstanceUniform {
    fn default() -> Self {
        Self {
            position_scale: [0.0, 0.0, 0.0, 1.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            indices: [0.0; 4],
        }
    }
}

/// Convert photo settings to GPU uniforms.
#[must_use]
pub fn convert(
    settings: &PhotoSettings,
    bokeh: BokehShape,
) -> (PhotoSettingsUniform, DofUniform, FilterUniform) {
    let settings_uniform = PhotoSettingsUniform::from_settings(settings);
    let dof_uniform = DofUniform::from_settings(settings, bokeh);
    let filter_uniform = FilterUniform::from_filter(settings.filter);

    (settings_uniform, dof_uniform, filter_uniform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_settings_uniform_conversion() {
        let settings = PhotoSettings::default().with_exposure(2.0).with_fov(90.0);

        let uniform = PhotoSettingsUniform::from_settings(&settings);

        assert_relative_eq!(uniform.exposure, 4.0, epsilon = 0.01);
        assert_relative_eq!(uniform.fov_radians, 90.0_f32.to_radians(), epsilon = 0.001);
    }

    #[test]
    fn test_settings_uniform_alignment() {
        assert_eq!(std::mem::size_of::<PhotoSettingsUniform>(), 32);
        assert_eq!(std::mem::align_of::<PhotoSettingsUniform>(), 4);
    }

    #[test]
    fn test_dof_uniform() {
        let settings = PhotoSettings::default()
            .with_focus_distance(10.0)
            .with_aperture(2.8);

        let uniform = DofUniform::from_settings(&settings, BokehShape::Hexagon);

        assert_relative_eq!(uniform.focus_distance, 10.0);
        assert_relative_eq!(uniform.aperture, 2.8);
        assert_eq!(uniform.bokeh_shape, BokehShape::Hexagon as u32);
    }

    #[test]
    fn test_dof_uniform_alignment() {
        assert_eq!(std::mem::size_of::<DofUniform>(), 32);
    }

    #[test]
    fn test_filter_uniform_all_filters() {
        for filter in PhotoFilter::ALL {
            let uniform = FilterUniform::from_filter(filter);
            assert_eq!(uniform.filter_type, filter as u32);
            assert!(uniform.intensity > 0.0);
        }
    }

    #[test]
    fn test_filter_uniform_alignment() {
        assert_eq!(std::mem::size_of::<FilterUniform>(), 48);
        assert_eq!(std::mem::size_of::<FilterUniform>() % 16, 0);
    }

    #[test]
    fn test_composition_guide_uniform() {
        let uniform = CompositionGuideUniform::from_guide(CompositionGuide::RuleOfThirds);
        assert_eq!(uniform.guide_type, CompositionGuide::RuleOfThirds as u32);
    }

    #[test]
    fn test_composition_guide_uniform_alignment() {
        assert_eq!(std::mem::size_of::<CompositionGuideUniform>(), 32);
    }

    #[test]
    fn test_camera_path_uniform() {
        let uniform = CameraPathUniform::from_camera(
            Vec3::new(0.0, 5.0, 10.0),
            Vec3::NEG_Z,
            Vec3::Y,
            70.0,
            0.1,
            1000.0,
            16.0 / 9.0,
            Vec3::X,
            1.5,
            0.5,
            1.0,
        );

        assert_relative_eq!(uniform.position[0], 0.0);
        assert_relative_eq!(uniform.position[1], 5.0);
        assert_relative_eq!(uniform.playback[0], 1.5);
        assert_relative_eq!(uniform.playback[1], 0.5);
    }

    #[test]
    fn test_camera_path_uniform_alignment() {
        assert_eq!(std::mem::size_of::<CameraPathUniform>(), 112);
        assert_eq!(std::mem::size_of::<CameraPathUniform>() % 16, 0);
    }

    #[test]
    fn test_photo_batch() {
        let mut batch = PhotoBatch::new(4);
        assert!(batch.is_empty());

        let instance = PhotoInstanceUniform {
            position_scale: [1.0, 2.0, 3.0, 1.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            indices: [0.0; 4],
        };

        assert!(batch.add(instance));
        assert_eq!(batch.len(), 1);

        batch.add(instance);
        batch.add(instance);
        batch.add(instance);
        assert!(!batch.add(instance));

        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn test_convert_function() {
        let settings = PhotoSettings::default().with_filter(PhotoFilter::Cinematic);

        let (settings_u, dof_u, filter_u) = convert(&settings, BokehShape::Circle);

        assert_eq!(filter_u.filter_type, PhotoFilter::Cinematic as u32);
        assert!(dof_u.focus_distance > 0.0);
        assert!(settings_u.exposure > 0.0);
    }

    #[test]
    fn test_as_bytes() {
        let settings = PhotoSettingsUniform::from_settings(&PhotoSettings::default());
        let bytes = settings.as_bytes();
        assert_eq!(bytes.len(), std::mem::size_of::<PhotoSettingsUniform>());
    }

    #[test]
    fn test_instance_uniform_alignment() {
        assert_eq!(std::mem::size_of::<PhotoInstanceUniform>(), 48);
        assert_eq!(std::mem::size_of::<PhotoInstanceUniform>() % 16, 0);
    }
}

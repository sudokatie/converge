//! Horizon model and configuration.
//!
//! Provides CPU-side primitives for computing horizon geometry,
//! visibility, and occlusion for curved world rendering.

use super::CurvatureBody;
use glam::Vec3;
use std::f32::consts::FRAC_PI_2;

/// Horizon rendering quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum HorizonQuality {
    /// Minimal samples, fast approximation.
    Low = 0,
    /// Balanced quality and performance.
    #[default]
    Medium = 1,
    /// High sample count for smooth horizon.
    High = 2,
    /// Maximum quality with sub-pixel precision.
    Ultra = 3,
}

impl HorizonQuality {
    /// All quality levels.
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// Get horizon sample count for this quality.
    #[must_use]
    pub fn sample_count(self) -> u32 {
        match self {
            Self::Low => 16,
            Self::Medium => 32,
            Self::High => 64,
            Self::Ultra => 128,
        }
    }
}

/// Configuration for horizon rendering.
#[derive(Debug, Clone, Copy)]
pub struct HorizonConfig {
    /// Quality level for horizon sampling.
    pub quality: HorizonQuality,
    /// Maximum horizon distance (for clipping).
    pub max_distance: f32,
    /// Atmospheric fade start distance (fraction of horizon).
    pub fade_start: f32,
    /// Atmospheric fade end distance (fraction of horizon).
    pub fade_end: f32,
    /// Sky/ground blend width at horizon (degrees).
    pub blend_angle: f32,
    /// Enable atmospheric scattering at horizon.
    pub atmospheric_scattering: bool,
    /// Horizon line thickness for debug rendering.
    pub debug_line_width: f32,
    /// Whether horizon effects are enabled.
    pub enabled: bool,
}

impl Default for HorizonConfig {
    fn default() -> Self {
        Self {
            quality: HorizonQuality::Medium,
            max_distance: 100_000.0,
            fade_start: 0.7,
            fade_end: 0.95,
            blend_angle: 0.5,
            atmospheric_scattering: true,
            debug_line_width: 2.0,
            enabled: true,
        }
    }
}

impl HorizonConfig {
    /// Set quality level.
    #[must_use]
    pub fn with_quality(mut self, quality: HorizonQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Set maximum horizon distance.
    #[must_use]
    pub fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance.max(0.0);
        self
    }

    /// Set fade parameters.
    #[must_use]
    pub fn with_fade(mut self, start: f32, end: f32) -> Self {
        self.fade_start = start.clamp(0.0, 1.0);
        self.fade_end = end.clamp(0.0, 1.0).max(self.fade_start);
        self
    }

    /// Set blend angle in degrees.
    #[must_use]
    pub fn with_blend_angle(mut self, degrees: f32) -> Self {
        self.blend_angle = degrees.clamp(0.0, 10.0);
        self
    }

    /// Enable or disable atmospheric scattering.
    #[must_use]
    pub fn with_atmospheric_scattering(mut self, enabled: bool) -> Self {
        self.atmospheric_scattering = enabled;
        self
    }

    /// Enable or disable horizon effects.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Computed horizon geometry for a camera position.
#[derive(Debug, Clone, Copy)]
pub struct HorizonModel {
    /// Camera position in world space.
    pub camera_position: Vec3,
    /// Camera height above surface.
    pub camera_height: f32,
    /// Distance to geometric horizon.
    pub horizon_distance: f32,
    /// Angle below camera forward to horizon (radians, positive = below).
    pub horizon_dip_angle: f32,
    /// Direction to horizon center (if applicable).
    pub horizon_center_dir: Vec3,
    /// Body radius at camera position.
    pub effective_radius: f32,
    /// Whether camera is above the surface.
    pub above_surface: bool,
}

impl HorizonModel {
    /// Compute horizon model for a camera on a curved body.
    #[must_use]
    pub fn compute(body: &CurvatureBody, camera_position: Vec3) -> Self {
        let height = body.height_above_surface(camera_position);
        let above_surface = height >= 0.0;
        let effective_height = height.abs().max(0.1);
        let effective_radius = body.radius;

        let horizon_distance = if above_surface {
            Self::compute_horizon_distance(effective_radius, effective_height)
        } else {
            0.0
        };

        let horizon_dip_angle = if above_surface && horizon_distance > 0.0 {
            Self::compute_dip_angle(effective_radius, effective_height)
        } else {
            0.0
        };

        let up = body.surface_up(camera_position);

        Self {
            camera_position,
            camera_height: height,
            horizon_distance,
            horizon_dip_angle,
            horizon_center_dir: up,
            effective_radius,
            above_surface,
        }
    }

    /// Compute geometric horizon distance.
    fn compute_horizon_distance(radius: f32, height: f32) -> f32 {
        let r_plus_h = radius + height;
        (r_plus_h * r_plus_h - radius * radius).sqrt()
    }

    /// Compute horizon dip angle (radians below horizontal, positive value).
    fn compute_dip_angle(radius: f32, height: f32) -> f32 {
        let r_plus_h = radius + height;
        if r_plus_h > radius && radius > 0.0 {
            (radius / r_plus_h).acos()
        } else {
            FRAC_PI_2
        }
    }

    /// Check if a direction is below the horizon.
    #[must_use]
    pub fn is_below_horizon(&self, direction: Vec3) -> bool {
        if !self.above_surface {
            return false;
        }
        let up = self.horizon_center_dir;
        let cos_angle = direction.normalize().dot(up);
        let horizon_cos = (FRAC_PI_2 - self.horizon_dip_angle + FRAC_PI_2).cos();
        cos_angle < horizon_cos
    }

    /// Compute visibility factor for a direction (1.0 = fully visible, 0.0 = below horizon).
    #[must_use]
    pub fn visibility_factor(&self, direction: Vec3, blend_angle_rad: f32) -> f32 {
        if !self.above_surface {
            return 1.0;
        }

        let up = self.horizon_center_dir;
        let dir_normalized = direction.normalize();
        let cos_angle = dir_normalized.dot(up);
        let angle_from_up = cos_angle.clamp(-1.0, 1.0).acos();

        let horizon_angle = FRAC_PI_2 + (FRAC_PI_2 - self.horizon_dip_angle);

        if angle_from_up <= horizon_angle - blend_angle_rad {
            1.0
        } else if angle_from_up >= horizon_angle + blend_angle_rad {
            0.0
        } else {
            let t = (angle_from_up - (horizon_angle - blend_angle_rad)) / (2.0 * blend_angle_rad);
            1.0 - t
        }
    }

    /// Compute occlusion factor for a point in world space.
    #[must_use]
    pub fn occlusion_factor(&self, body: &CurvatureBody, world_point: Vec3) -> f32 {
        let to_point = world_point - self.camera_position;
        let distance = to_point.length();

        if distance < 0.001 {
            return 1.0;
        }

        let direction = to_point / distance;
        let point_height = body.height_above_surface(world_point);

        if !self.above_surface || point_height < 0.0 {
            return 0.0;
        }

        if distance > self.horizon_distance {
            return 0.0;
        }

        self.visibility_factor(direction, 0.01)
    }
}

/// Compute horizon visibility for a point relative to camera.
#[must_use]
pub fn horizon_visibility(
    body: &CurvatureBody,
    camera_position: Vec3,
    target_position: Vec3,
) -> f32 {
    let model = HorizonModel::compute(body, camera_position);
    model.occlusion_factor(body, target_position)
}

/// Compute atmospheric fade factor based on distance to horizon.
#[must_use]
pub fn atmospheric_fade(distance: f32, horizon_distance: f32, config: &HorizonConfig) -> f32 {
    if horizon_distance <= 0.0 || !config.enabled {
        return 1.0;
    }

    let normalized = distance / horizon_distance;
    if normalized <= config.fade_start {
        1.0
    } else if normalized >= config.fade_end {
        0.0
    } else {
        let t = (normalized - config.fade_start) / (config.fade_end - config.fade_start);
        1.0 - t * t
    }
}

/// Compute fog density multiplier based on horizon geometry.
#[must_use]
pub fn horizon_fog_density(
    distance: f32,
    horizon_distance: f32,
    base_density: f32,
    horizon_boost: f32,
) -> f32 {
    if horizon_distance <= 0.0 {
        return base_density;
    }

    let normalized = (distance / horizon_distance).clamp(0.0, 1.0);
    let boost = normalized * normalized * horizon_boost;
    base_density + boost
}

/// Compute clip plane distance for horizon-aware rendering.
#[must_use]
pub fn horizon_clip_distance(horizon_distance: f32, config: &HorizonConfig) -> f32 {
    let base = horizon_distance * config.fade_end;
    base.min(config.max_distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::PI;

    fn earth_like_body() -> CurvatureBody {
        CurvatureBody::planetary_sphere(Vec3::ZERO, 6_371_000.0)
    }

    #[test]
    fn test_horizon_distance_at_sea_level() {
        let body = earth_like_body();
        let camera = Vec3::new(6_371_002.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(model.horizon_distance > 4000.0);
        assert!(model.horizon_distance < 6000.0);
    }

    #[test]
    fn test_horizon_distance_at_altitude() {
        let body = earth_like_body();
        let low = Vec3::new(6_371_010.0, 0.0, 0.0);
        let high = Vec3::new(6_371_100.0, 0.0, 0.0);

        let model_low = HorizonModel::compute(&body, low);
        let model_high = HorizonModel::compute(&body, high);

        assert!(
            model_high.horizon_distance > model_low.horizon_distance,
            "higher altitude should have farther horizon"
        );
    }

    #[test]
    fn test_horizon_dip_angle_positive() {
        let body = earth_like_body();
        let camera = Vec3::new(6_372_000.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(model.horizon_dip_angle > 0.0);
        assert!(model.horizon_dip_angle < PI / 4.0);
    }

    #[test]
    fn test_visibility_factor_above_horizon() {
        let body = earth_like_body();
        let camera = Vec3::new(6_372_000.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        let up_dir = Vec3::new(1.0, 0.0, 0.0);
        let visibility = model.visibility_factor(up_dir, 0.01);
        assert_relative_eq!(visibility, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_visibility_factor_below_horizon() {
        let body = earth_like_body();
        let camera = Vec3::new(6_372_000.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        let down_dir = Vec3::new(-1.0, 0.0, 0.0);
        let visibility = model.visibility_factor(down_dir, 0.01);
        assert_relative_eq!(visibility, 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_is_below_horizon() {
        let body = earth_like_body();
        let camera = Vec3::new(6_372_000.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(!model.is_below_horizon(Vec3::new(1.0, 0.0, 0.0)));
        assert!(model.is_below_horizon(Vec3::new(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_atmospheric_fade() {
        let config = HorizonConfig::default();

        assert_relative_eq!(atmospheric_fade(0.0, 1000.0, &config), 1.0, epsilon = 0.01);
        assert_relative_eq!(
            atmospheric_fade(500.0, 1000.0, &config),
            1.0,
            epsilon = 0.01
        );
        assert!(atmospheric_fade(850.0, 1000.0, &config) < 1.0);
        assert_relative_eq!(
            atmospheric_fade(1000.0, 1000.0, &config),
            0.0,
            epsilon = 0.1
        );
    }

    #[test]
    fn test_horizon_fog_density() {
        let base = 0.001;
        let boost = 0.01;
        let horizon = 10000.0;

        let near = horizon_fog_density(1000.0, horizon, base, boost);
        let far = horizon_fog_density(9000.0, horizon, base, boost);

        assert!(near < far, "fog should be denser near horizon");
        assert!(near >= base);
    }

    #[test]
    fn test_horizon_clip_distance() {
        let config = HorizonConfig::default().with_max_distance(50_000.0);
        let horizon = 100_000.0;
        let clip = horizon_clip_distance(horizon, &config);

        assert!(clip <= config.max_distance);
        assert!(clip <= horizon * config.fade_end);
    }

    #[test]
    fn test_interior_sphere_horizon() {
        let body = CurvatureBody::interior_sphere(Vec3::ZERO, 10_000.0);
        let camera = Vec3::new(9_000.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(model.above_surface);
        assert!(model.horizon_distance > 0.0);
    }

    #[test]
    fn test_below_surface_no_horizon() {
        let body = earth_like_body();
        let camera = Vec3::new(6_370_000.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(!model.above_surface);
        assert_relative_eq!(model.horizon_distance, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_quality_sample_counts() {
        assert!(HorizonQuality::Low.sample_count() < HorizonQuality::Ultra.sample_count());
        for quality in HorizonQuality::ALL {
            assert!(quality.sample_count() >= 8);
        }
    }

    #[test]
    fn test_config_builders() {
        let config = HorizonConfig::default()
            .with_quality(HorizonQuality::High)
            .with_max_distance(200_000.0)
            .with_fade(0.5, 0.9)
            .with_blend_angle(1.0)
            .with_atmospheric_scattering(false)
            .with_enabled(true);

        assert_eq!(config.quality, HorizonQuality::High);
        assert_relative_eq!(config.max_distance, 200_000.0, epsilon = 0.1);
        assert_relative_eq!(config.fade_start, 0.5, epsilon = 0.001);
        assert!(!config.atmospheric_scattering);
    }

    #[test]
    fn test_horizon_visibility_helper() {
        let body = earth_like_body();
        let camera = Vec3::new(6_372_000.0, 0.0, 0.0);
        let visible_point = Vec3::new(6_372_000.0, 1000.0, 0.0);
        let hidden_point = Vec3::new(6_370_000.0, 0.0, 1000.0);

        let vis1 = horizon_visibility(&body, camera, visible_point);
        let vis2 = horizon_visibility(&body, camera, hidden_point);

        assert!(vis1 > vis2);
    }
}

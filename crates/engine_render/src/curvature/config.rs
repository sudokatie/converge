//! Curvature-aware rendering configuration.
//!
//! Provides configuration for fog, clip planes, and fade effects
//! that account for curved world geometry.

use super::{CurvatureBody, HorizonConfig, HorizonModel};
use glam::Vec3;

/// Curvature-aware fog configuration.
#[derive(Debug, Clone, Copy)]
pub struct CurvatureFogConfig {
    /// Base fog density (exponential coefficient).
    pub base_density: f32,
    /// Additional density boost at horizon.
    pub horizon_boost: f32,
    /// Fog color (linear RGB).
    pub color: Vec3,
    /// Inscattering strength (atmospheric glow).
    pub inscatter_strength: f32,
    /// Height-based density falloff exponent.
    pub height_falloff: f32,
    /// Minimum fog distance.
    pub min_distance: f32,
    /// Whether fog is enabled.
    pub enabled: bool,
}

impl Default for CurvatureFogConfig {
    fn default() -> Self {
        Self {
            base_density: 0.0001,
            horizon_boost: 0.001,
            color: Vec3::new(0.7, 0.8, 0.95),
            inscatter_strength: 0.1,
            height_falloff: 0.0001,
            min_distance: 10.0,
            enabled: true,
        }
    }
}

impl CurvatureFogConfig {
    /// Create fog config for a clear atmosphere.
    #[must_use]
    pub fn clear() -> Self {
        Self {
            base_density: 0.00005,
            horizon_boost: 0.0005,
            color: Vec3::new(0.6, 0.75, 0.95),
            inscatter_strength: 0.05,
            height_falloff: 0.0002,
            ..Default::default()
        }
    }

    /// Create fog config for a hazy atmosphere.
    #[must_use]
    pub fn hazy() -> Self {
        Self {
            base_density: 0.0003,
            horizon_boost: 0.003,
            color: Vec3::new(0.75, 0.8, 0.85),
            inscatter_strength: 0.15,
            height_falloff: 0.00005,
            ..Default::default()
        }
    }

    /// Create fog config for thick fog conditions.
    #[must_use]
    pub fn thick() -> Self {
        Self {
            base_density: 0.001,
            horizon_boost: 0.01,
            color: Vec3::new(0.85, 0.85, 0.85),
            inscatter_strength: 0.3,
            height_falloff: 0.0,
            min_distance: 1.0,
            ..Default::default()
        }
    }

    /// Create fog config for a vacuum (no fog).
    #[must_use]
    pub fn vacuum() -> Self {
        Self {
            base_density: 0.0,
            horizon_boost: 0.0,
            color: Vec3::ZERO,
            inscatter_strength: 0.0,
            height_falloff: 0.0,
            enabled: false,
            ..Default::default()
        }
    }

    /// Set base density.
    #[must_use]
    pub fn with_base_density(mut self, density: f32) -> Self {
        self.base_density = density.max(0.0);
        self
    }

    /// Set horizon boost.
    #[must_use]
    pub fn with_horizon_boost(mut self, boost: f32) -> Self {
        self.horizon_boost = boost.max(0.0);
        self
    }

    /// Set fog color.
    #[must_use]
    pub fn with_color(mut self, color: Vec3) -> Self {
        self.color = color;
        self
    }

    /// Set inscatter strength.
    #[must_use]
    pub fn with_inscatter_strength(mut self, strength: f32) -> Self {
        self.inscatter_strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Set height falloff.
    #[must_use]
    pub fn with_height_falloff(mut self, falloff: f32) -> Self {
        self.height_falloff = falloff.max(0.0);
        self
    }

    /// Enable or disable fog.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Compute fog density at a given height above surface.
    #[must_use]
    pub fn density_at_height(&self, height: f32) -> f32 {
        if !self.enabled || height < 0.0 {
            return 0.0;
        }
        self.base_density * (-height * self.height_falloff).exp()
    }

    /// Compute fog density for a ray segment considering curvature.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "samples capped at reasonable values; fits in f32 mantissa"
    )]
    pub fn density_for_ray(
        &self,
        body: &CurvatureBody,
        start: Vec3,
        end: Vec3,
        samples: u32,
    ) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let dir = end - start;
        let length = dir.length();
        if length < 0.001 {
            return 0.0;
        }

        let step = 1.0 / samples as f32;
        let mut total_density = 0.0;

        for i in 0..samples {
            let t = (i as f32 + 0.5) * step;
            let point = start + dir * t;
            let height = body.height_above_surface(point).max(0.0);
            total_density += self.density_at_height(height);
        }

        (total_density * length * step).min(10.0)
    }

    /// Compute fog transmittance (0 = fully fogged, 1 = clear).
    #[must_use]
    pub fn transmittance(&self, optical_depth: f32) -> f32 {
        (-optical_depth).exp()
    }

    /// Compute fog color contribution.
    #[must_use]
    pub fn fog_contribution(&self, optical_depth: f32) -> Vec3 {
        let alpha = 1.0 - self.transmittance(optical_depth);
        self.color * alpha
    }
}

/// Curvature-aware clip plane configuration.
#[derive(Debug, Clone, Copy)]
pub struct CurvatureClipConfig {
    /// Near clip plane distance.
    pub near: f32,
    /// Far clip plane distance.
    pub far: f32,
    /// Whether to use horizon-based far clip.
    pub horizon_clip: bool,
    /// Multiplier for horizon distance when computing far clip.
    pub horizon_multiplier: f32,
    /// Minimum far clip distance.
    pub min_far: f32,
    /// Maximum far clip distance.
    pub max_far: f32,
}

impl Default for CurvatureClipConfig {
    fn default() -> Self {
        Self {
            near: 0.1,
            far: 100_000.0,
            horizon_clip: true,
            horizon_multiplier: 1.1,
            min_far: 1000.0,
            max_far: 1_000_000.0,
        }
    }
}

impl CurvatureClipConfig {
    /// Create clip config for close-range rendering.
    #[must_use]
    pub fn close_range() -> Self {
        Self {
            near: 0.01,
            far: 10_000.0,
            horizon_clip: false,
            min_far: 100.0,
            max_far: 50_000.0,
            ..Default::default()
        }
    }

    /// Create clip config for planetary-scale rendering.
    #[must_use]
    pub fn planetary() -> Self {
        Self {
            near: 1.0,
            far: 500_000.0,
            horizon_clip: true,
            horizon_multiplier: 1.2,
            min_far: 10_000.0,
            max_far: 2_000_000.0,
        }
    }

    /// Set near clip plane.
    #[must_use]
    pub fn with_near(mut self, near: f32) -> Self {
        self.near = near.max(0.001);
        self
    }

    /// Set far clip plane.
    #[must_use]
    pub fn with_far(mut self, far: f32) -> Self {
        self.far = far.max(self.near * 10.0);
        self
    }

    /// Enable or disable horizon-based clipping.
    #[must_use]
    pub fn with_horizon_clip(mut self, enabled: bool) -> Self {
        self.horizon_clip = enabled;
        self
    }

    /// Compute effective far clip plane.
    #[must_use]
    pub fn effective_far(&self, model: &HorizonModel) -> f32 {
        if self.horizon_clip && model.above_surface && model.horizon_distance > 0.0 {
            let horizon_far = model.horizon_distance * self.horizon_multiplier;
            horizon_far.clamp(self.min_far, self.max_far)
        } else {
            self.far.clamp(self.min_far, self.max_far)
        }
    }

    /// Compute near and far clip planes.
    #[must_use]
    pub fn compute_planes(&self, model: &HorizonModel) -> (f32, f32) {
        (self.near, self.effective_far(model))
    }
}

/// Curvature-aware fade configuration.
#[derive(Debug, Clone, Copy)]
pub struct CurvatureFadeConfig {
    /// Distance at which fade begins (fraction of horizon).
    pub start: f32,
    /// Distance at which fade completes (fraction of horizon).
    pub end: f32,
    /// Fade curve power (1.0 = linear, 2.0 = quadratic).
    pub power: f32,
    /// Additional fade at horizon edge.
    pub horizon_edge_fade: f32,
    /// Whether fade is enabled.
    pub enabled: bool,
}

impl Default for CurvatureFadeConfig {
    fn default() -> Self {
        Self {
            start: 0.7,
            end: 0.95,
            power: 2.0,
            horizon_edge_fade: 0.1,
            enabled: true,
        }
    }
}

impl CurvatureFadeConfig {
    /// Create fade config for gentle fade.
    #[must_use]
    pub fn gentle() -> Self {
        Self {
            start: 0.5,
            end: 0.9,
            power: 1.5,
            horizon_edge_fade: 0.05,
            ..Default::default()
        }
    }

    /// Create fade config for sharp fade.
    #[must_use]
    pub fn sharp() -> Self {
        Self {
            start: 0.85,
            end: 0.98,
            power: 3.0,
            horizon_edge_fade: 0.2,
            ..Default::default()
        }
    }

    /// Set fade range.
    #[must_use]
    pub fn with_range(mut self, start: f32, end: f32) -> Self {
        self.start = start.clamp(0.0, 1.0);
        self.end = end.clamp(0.0, 1.0).max(self.start);
        self
    }

    /// Set fade power.
    #[must_use]
    pub fn with_power(mut self, power: f32) -> Self {
        self.power = power.clamp(0.5, 5.0);
        self
    }

    /// Enable or disable fade.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Compute fade factor for a distance.
    #[must_use]
    pub fn compute_fade(&self, distance: f32, horizon_distance: f32) -> f32 {
        if !self.enabled || horizon_distance <= 0.0 {
            return 1.0;
        }

        let normalized = distance / horizon_distance;

        if normalized <= self.start {
            1.0
        } else if normalized >= self.end {
            0.0
        } else {
            let t = (normalized - self.start) / (self.end - self.start);
            1.0 - t.powf(self.power)
        }
    }

    /// Compute fade with horizon edge adjustment.
    #[must_use]
    pub fn compute_fade_with_edge(
        &self,
        distance: f32,
        horizon_distance: f32,
        angle_from_horizon: f32,
    ) -> f32 {
        let base_fade = self.compute_fade(distance, horizon_distance);

        if self.horizon_edge_fade > 0.0 && angle_from_horizon.abs() < 0.1 {
            let edge_factor = 1.0 - (angle_from_horizon.abs() / 0.1);
            base_fade * (1.0 - self.horizon_edge_fade * edge_factor)
        } else {
            base_fade
        }
    }
}

/// Combined curvature rendering configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct CurvatureRenderConfig {
    /// Fog configuration.
    pub fog: CurvatureFogConfig,
    /// Clip plane configuration.
    pub clip: CurvatureClipConfig,
    /// Fade configuration.
    pub fade: CurvatureFadeConfig,
    /// Horizon configuration.
    pub horizon: HorizonConfig,
}

impl CurvatureRenderConfig {
    /// Create config for Earth-like planet.
    #[must_use]
    pub fn earth_like() -> Self {
        Self {
            fog: CurvatureFogConfig::clear(),
            clip: CurvatureClipConfig::planetary(),
            fade: CurvatureFadeConfig::default(),
            horizon: HorizonConfig::default(),
        }
    }

    /// Create config for interior habitat.
    #[must_use]
    pub fn interior_habitat() -> Self {
        Self {
            fog: CurvatureFogConfig::hazy(),
            clip: CurvatureClipConfig::close_range(),
            fade: CurvatureFadeConfig::gentle(),
            horizon: HorizonConfig::default()
                .with_max_distance(50_000.0)
                .with_atmospheric_scattering(true),
        }
    }

    /// Create config for space/vacuum.
    #[must_use]
    pub fn space() -> Self {
        Self {
            fog: CurvatureFogConfig::vacuum(),
            clip: CurvatureClipConfig::planetary(),
            fade: CurvatureFadeConfig::default().with_enabled(false),
            horizon: HorizonConfig::default().with_atmospheric_scattering(false),
        }
    }

    /// Set fog config.
    #[must_use]
    pub fn with_fog(mut self, fog: CurvatureFogConfig) -> Self {
        self.fog = fog;
        self
    }

    /// Set clip config.
    #[must_use]
    pub fn with_clip(mut self, clip: CurvatureClipConfig) -> Self {
        self.clip = clip;
        self
    }

    /// Set fade config.
    #[must_use]
    pub fn with_fade(mut self, fade: CurvatureFadeConfig) -> Self {
        self.fade = fade;
        self
    }

    /// Set horizon config.
    #[must_use]
    pub fn with_horizon(mut self, horizon: HorizonConfig) -> Self {
        self.horizon = horizon;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_fog_density_at_height() {
        let fog = CurvatureFogConfig::default();

        let at_surface = fog.density_at_height(0.0);
        let at_altitude = fog.density_at_height(10_000.0);

        assert!(at_surface > 0.0);
        assert!(at_altitude < at_surface);
    }

    #[test]
    fn test_fog_density_below_surface() {
        let fog = CurvatureFogConfig::default();
        let below = fog.density_at_height(-100.0);
        assert_relative_eq!(below, 0.0, epsilon = 0.0001);
    }

    #[test]
    fn test_fog_disabled() {
        let fog = CurvatureFogConfig::vacuum();
        assert_relative_eq!(fog.density_at_height(0.0), 0.0, epsilon = 0.0001);
    }

    #[test]
    fn test_fog_transmittance() {
        let fog = CurvatureFogConfig::default();

        assert_relative_eq!(fog.transmittance(0.0), 1.0, epsilon = 0.001);
        assert!(fog.transmittance(1.0) < 1.0);
        assert!(fog.transmittance(10.0) < 0.01);
    }

    #[test]
    fn test_fog_contribution() {
        let fog = CurvatureFogConfig::default();
        let contrib = fog.fog_contribution(1.0);

        assert!(contrib.x > 0.0);
        assert!(contrib.y > 0.0);
        assert!(contrib.z > 0.0);
    }

    #[test]
    fn test_fog_ray_density() {
        let fog = CurvatureFogConfig::default();
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);

        let start = Vec3::new(1100.0, 0.0, 0.0);
        let end = Vec3::new(1100.0, 100.0, 0.0);

        let density = fog.density_for_ray(&body, start, end, 10);
        assert!(density > 0.0);
    }

    #[test]
    fn test_clip_effective_far() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let clip = CurvatureClipConfig::default();

        let far = clip.effective_far(&model);
        assert!(far > model.horizon_distance);
        assert!(far <= clip.max_far);
    }

    #[test]
    fn test_clip_no_horizon_clip() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let clip = CurvatureClipConfig::default().with_horizon_clip(false);

        let far = clip.effective_far(&model);
        assert_relative_eq!(
            far,
            clip.far.clamp(clip.min_far, clip.max_far),
            epsilon = 0.1
        );
    }

    #[test]
    fn test_fade_compute() {
        let fade = CurvatureFadeConfig::default();
        let horizon_dist = 10_000.0;

        assert_relative_eq!(fade.compute_fade(0.0, horizon_dist), 1.0, epsilon = 0.001);
        assert_relative_eq!(
            fade.compute_fade(5000.0, horizon_dist),
            1.0,
            epsilon = 0.001
        );
        assert!(fade.compute_fade(8500.0, horizon_dist) < 1.0);
        assert_relative_eq!(
            fade.compute_fade(10_000.0, horizon_dist),
            0.0,
            epsilon = 0.1
        );
    }

    #[test]
    fn test_fade_disabled() {
        let fade = CurvatureFadeConfig::default().with_enabled(false);
        assert_relative_eq!(fade.compute_fade(9000.0, 10_000.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_fade_with_edge() {
        let fade = CurvatureFadeConfig::default();
        let base = fade.compute_fade(8000.0, 10_000.0);
        let with_edge = fade.compute_fade_with_edge(8000.0, 10_000.0, 0.0);

        assert!(with_edge <= base);
    }

    #[test]
    fn test_render_config_presets() {
        let earth = CurvatureRenderConfig::earth_like();
        assert!(earth.fog.enabled);
        assert!(earth.horizon.atmospheric_scattering);

        let interior = CurvatureRenderConfig::interior_habitat();
        assert!(interior.fog.enabled);

        let space = CurvatureRenderConfig::space();
        assert!(!space.fog.enabled);
        assert!(!space.horizon.atmospheric_scattering);
    }

    #[test]
    fn test_fog_presets() {
        let clear = CurvatureFogConfig::clear();
        let hazy = CurvatureFogConfig::hazy();
        let thick = CurvatureFogConfig::thick();

        assert!(clear.base_density < hazy.base_density);
        assert!(hazy.base_density < thick.base_density);
    }

    #[test]
    fn test_clip_presets() {
        let close = CurvatureClipConfig::close_range();
        let planetary = CurvatureClipConfig::planetary();

        assert!(close.far < planetary.far);
        assert!(!close.horizon_clip);
        assert!(planetary.horizon_clip);
    }

    #[test]
    fn test_fade_presets() {
        let gentle = CurvatureFadeConfig::gentle();
        let sharp = CurvatureFadeConfig::sharp();

        assert!(gentle.start < sharp.start);
        assert!(gentle.power < sharp.power);
    }

    #[test]
    fn test_config_builders() {
        let config = CurvatureRenderConfig::default()
            .with_fog(CurvatureFogConfig::hazy())
            .with_clip(CurvatureClipConfig::planetary())
            .with_fade(CurvatureFadeConfig::sharp());

        assert_relative_eq!(config.fog.base_density, 0.0003, epsilon = 0.0001);
        assert!(config.clip.horizon_clip);
        assert_relative_eq!(config.fade.power, 3.0, epsilon = 0.001);
    }
}

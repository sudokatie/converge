//! Curvature rendering primitives for large-body horizon effects.
//!
//! Provides CPU-side primitives for rendering curved worlds and large bodies:
//! planetary spheres, interior spheres (Dyson-like), cylinders/rings,
//! and large moving bodies. These types configure horizon-aware rendering
//! without implementing a full GPU pass.

mod body;
mod config;
mod horizon;
mod sampling;
mod uniform;

pub use body::{CurvatureBody, CurvatureBodyKind};
pub use config::{
    CurvatureClipConfig, CurvatureFadeConfig, CurvatureFogConfig, CurvatureRenderConfig,
};
pub use horizon::{
    HorizonConfig, HorizonModel, HorizonQuality, atmospheric_fade, horizon_clip_distance,
    horizon_fog_density, horizon_visibility,
};
pub use sampling::{
    CurvatureSampler, angular_separation, compute_tangent_frame, curvature_distance_correction,
    flat_to_curved_direction, flat_to_curved_position, great_circle_distance, line_of_sight,
    position_hash, surface_forward, surface_normal,
};
pub use uniform::{
    CurvatureBatch, CurvatureBodyUniform, CurvatureClipUniform, CurvatureFogUniform,
    CurvatureInstanceUniform, HorizonConfigUniform, HorizonModelUniform, convert,
};

use std::hash::{Hash, Hasher};

/// Compute a stable fingerprint for a curvature configuration.
#[must_use]
pub fn compute_fingerprint(body: &CurvatureBody, config: &HorizonConfig) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    body.kind.hash(&mut hasher);
    hash_f32(body.radius, &mut hasher);
    hash_f32(body.center.x, &mut hasher);
    hash_f32(body.center.y, &mut hasher);
    hash_f32(body.center.z, &mut hasher);
    config.quality.hash(&mut hasher);
    hash_f32(config.max_distance, &mut hasher);
    hash_f32(config.fade_start, &mut hasher);
    hash_f32(config.fade_end, &mut hasher);
    hasher.finish()
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

/// Sort curvature bodies by distance to camera (closest first).
pub fn sort_by_distance(
    bodies: &mut [(CurvatureBody, HorizonConfig)],
    camera_position: glam::Vec3,
) {
    bodies.sort_by(|a, b| {
        let dist_a = (a.0.center - camera_position).length_squared();
        let dist_b = (b.0.center - camera_position).length_squared();
        dist_a
            .partial_cmp(&dist_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Sort curvature bodies by radius (largest first).
pub fn sort_by_radius(bodies: &mut [(CurvatureBody, HorizonConfig)]) {
    bodies.sort_by(|a, b| {
        b.0.radius
            .partial_cmp(&a.0.radius)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Filter active bodies from a list.
#[must_use]
pub fn filter_active(
    bodies: &[(CurvatureBody, HorizonConfig)],
) -> Vec<(CurvatureBody, HorizonConfig)> {
    bodies
        .iter()
        .filter(|(body, config)| body.active && config.enabled)
        .copied()
        .collect()
}

/// Find the dominant body affecting a camera position.
#[must_use]
pub fn find_dominant_body(
    bodies: &[(CurvatureBody, HorizonConfig)],
    camera_position: glam::Vec3,
) -> Option<(CurvatureBody, HorizonConfig)> {
    let active = filter_active(bodies);
    if active.is_empty() {
        return None;
    }

    let mut best: Option<(f32, (CurvatureBody, HorizonConfig))> = None;

    for (body, config) in active {
        let height = body.height_above_surface(camera_position);
        if height >= 0.0 {
            let score = body.radius / (height + 1.0);
            let dominated = best.as_ref().is_none_or(|b| score > b.0);
            if dominated {
                best = Some((score, (body, config)));
            }
        }
    }

    best.map(|(_, pair)| pair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn test_fingerprint_determinism() {
        let body = CurvatureBody::planetary_sphere(Vec3::new(1.0, 2.0, 3.0), 1000.0);
        let config = HorizonConfig::default();

        let fp1 = compute_fingerprint(&body, &config);
        let fp2 = compute_fingerprint(&body, &config);

        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
    }

    #[test]
    fn test_fingerprint_sensitivity() {
        let body1 = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let body2 = CurvatureBody::interior_sphere(Vec3::ZERO, 1000.0);
        let config = HorizonConfig::default();

        let fp1 = compute_fingerprint(&body1, &config);
        let fp2 = compute_fingerprint(&body2, &config);

        assert_ne!(
            fp1, fp2,
            "different body kinds should have different fingerprints"
        );
    }

    #[test]
    fn test_fingerprint_radius_sensitivity() {
        let body1 = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let body2 = CurvatureBody::planetary_sphere(Vec3::ZERO, 2000.0);
        let config = HorizonConfig::default();

        let fp1 = compute_fingerprint(&body1, &config);
        let fp2 = compute_fingerprint(&body2, &config);

        assert_ne!(
            fp1, fp2,
            "different radii should have different fingerprints"
        );
    }

    #[test]
    fn test_sort_by_distance() {
        let config = HorizonConfig::default();
        let camera = Vec3::ZERO;

        let near = (
            CurvatureBody::planetary_sphere(Vec3::new(100.0, 0.0, 0.0), 50.0),
            config,
        );
        let far = (
            CurvatureBody::planetary_sphere(Vec3::new(1000.0, 0.0, 0.0), 50.0),
            config,
        );
        let mid = (
            CurvatureBody::planetary_sphere(Vec3::new(500.0, 0.0, 0.0), 50.0),
            config,
        );

        let mut bodies = vec![far, near, mid];
        sort_by_distance(&mut bodies, camera);

        assert_relative_eq!(bodies[0].0.center.x, 100.0, epsilon = 0.1);
        assert_relative_eq!(bodies[1].0.center.x, 500.0, epsilon = 0.1);
        assert_relative_eq!(bodies[2].0.center.x, 1000.0, epsilon = 0.1);
    }

    #[test]
    fn test_sort_by_radius() {
        let config = HorizonConfig::default();

        let small = (CurvatureBody::planetary_sphere(Vec3::ZERO, 100.0), config);
        let large = (CurvatureBody::planetary_sphere(Vec3::ZERO, 10000.0), config);
        let mid = (CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0), config);

        let mut bodies = vec![small, large, mid];
        sort_by_radius(&mut bodies);

        assert_relative_eq!(bodies[0].0.radius, 10000.0, epsilon = 0.1);
        assert_relative_eq!(bodies[1].0.radius, 1000.0, epsilon = 0.1);
        assert_relative_eq!(bodies[2].0.radius, 100.0, epsilon = 0.1);
    }

    #[test]
    fn test_filter_active() {
        let config = HorizonConfig::default();
        let disabled_config = HorizonConfig::default().with_enabled(false);

        let active = (CurvatureBody::planetary_sphere(Vec3::ZERO, 100.0), config);
        let inactive_body = (
            CurvatureBody::planetary_sphere(Vec3::ZERO, 200.0).with_active(false),
            config,
        );
        let inactive_config = (
            CurvatureBody::planetary_sphere(Vec3::ZERO, 300.0),
            disabled_config,
        );

        let bodies = vec![active, inactive_body, inactive_config];
        let filtered = filter_active(&bodies);

        assert_eq!(filtered.len(), 1);
        assert_relative_eq!(filtered[0].0.radius, 100.0, epsilon = 0.1);
    }

    #[test]
    fn test_find_dominant_body() {
        let config = HorizonConfig::default();

        let planet = (
            CurvatureBody::planetary_sphere(Vec3::ZERO, 6_371_000.0),
            config,
        );
        let moon = (
            CurvatureBody::planetary_sphere(Vec3::new(384_400_000.0, 0.0, 0.0), 1_737_000.0),
            config,
        );

        let bodies = vec![planet, moon];
        let camera = Vec3::new(6_371_100.0, 0.0, 0.0);

        let dominant = find_dominant_body(&bodies, camera);
        assert!(dominant.is_some());
        assert_relative_eq!(dominant.unwrap().0.radius, 6_371_000.0, epsilon = 1.0);
    }

    #[test]
    fn test_find_dominant_body_no_active() {
        let config = HorizonConfig::default();
        let inactive = (
            CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0).with_active(false),
            config,
        );

        let bodies = vec![inactive];
        let camera = Vec3::new(1100.0, 0.0, 0.0);

        let dominant = find_dominant_body(&bodies, camera);
        assert!(dominant.is_none());
    }

    #[test]
    fn test_find_dominant_body_below_surface() {
        let config = HorizonConfig::default();
        let body = (CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0), config);

        let bodies = vec![body];
        let camera = Vec3::new(500.0, 0.0, 0.0);

        let dominant = find_dominant_body(&bodies, camera);
        assert!(dominant.is_none());
    }

    #[test]
    fn test_integration_full_workflow() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 6_371_000.0);
        let config = HorizonConfig::default();
        let render_config = CurvatureRenderConfig::earth_like();

        let camera = Vec3::new(6_371_002.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(model.above_surface);
        assert!(model.horizon_distance > 4000.0);

        let visibility = horizon_visibility(&body, camera, Vec3::new(6_371_002.0, 1000.0, 0.0));
        assert!(visibility > 0.0);

        let fade = atmospheric_fade(
            model.horizon_distance * 0.5,
            model.horizon_distance,
            &config,
        );
        assert!(fade > 0.0);

        let (near, far) = render_config.clip.compute_planes(&model);
        assert!(near < far);

        let _uniform = CurvatureInstanceUniform::new(body, model, config);
        let fingerprint = compute_fingerprint(&body, &config);
        assert!(fingerprint != 0);
    }

    #[test]
    fn test_integration_interior_habitat() {
        let body = CurvatureBody::cylinder(Vec3::ZERO, 500.0, 5000.0, Vec3::Y);
        let _config = HorizonConfig::default();
        let render_config = CurvatureRenderConfig::interior_habitat();

        let camera = Vec3::new(400.0, 100.0, 0.0);
        let model = HorizonModel::compute(&body, camera);

        assert!(model.above_surface);

        let up = body.surface_up(camera);
        assert!(up.x < 0.0);

        let (tangent1, tangent2, normal) = compute_tangent_frame(up);
        assert_relative_eq!(tangent1.dot(tangent2).abs(), 0.0, epsilon = 0.001);
        assert_relative_eq!(tangent1.dot(normal).abs(), 0.0, epsilon = 0.001);

        let fog_density = render_config
            .fog
            .density_at_height(body.height_above_surface(camera));
        assert!(fog_density > 0.0);
    }

    #[test]
    fn test_sampler_integration() {
        let body = CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0);
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let sampler = CurvatureSampler::new(42).with_ring_samples(16);

        let points = sampler.sample_horizon_ring(&model, &body);
        assert_eq!(points.len(), 16);

        for point in &points {
            let height = body.height_above_surface(*point);
            assert!(height.abs() < 50.0, "horizon points should be near surface");
        }
    }

    #[test]
    fn test_uniform_round_trip() {
        let body = CurvatureBody::planetary_sphere(Vec3::new(1.0, 2.0, 3.0), 1000.0)
            .with_surface_gravity(9.81)
            .with_angular_velocity(0.01);
        let camera = Vec3::new(1100.0, 2.0, 3.0);
        let model = HorizonModel::compute(&body, camera);
        let config = HorizonConfig::default().with_quality(HorizonQuality::High);

        let body_uniform: CurvatureBodyUniform = body.into();
        let model_uniform: HorizonModelUniform = model.into();
        let config_uniform: HorizonConfigUniform = config.into();

        assert_relative_eq!(body_uniform.center_radius[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(body_uniform.center_radius[3], 1000.0, epsilon = 0.001);
        assert!(model_uniform.direction_distance[3] > 0.0);
        assert_relative_eq!(
            config_uniform.flags[0],
            f32::from(HorizonQuality::High as u8),
            epsilon = 0.001
        );
    }
}

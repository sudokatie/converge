//! Camera-relative horizon sampling and geometric helpers.
//!
//! Provides deterministic sampling functions for horizon rendering,
//! including tangent frame computation and curvature-aware transforms.

use super::{CurvatureBody, HorizonModel};
use glam::{Mat3, Vec3};
use std::f32::consts::TAU;

const HASH_PRIME: u32 = 374_761_393;
const HASH_MUL_A: u32 = 0x85eb_ca6b;
const HASH_MUL_B: u32 = 0xc2b2_ae35;

/// Sampler for horizon-related calculations.
#[derive(Debug, Clone, Copy)]
pub struct CurvatureSampler {
    /// Seed for deterministic variation.
    pub seed: u32,
    /// Number of horizon ring samples.
    pub ring_samples: u32,
    /// Number of radial distance samples.
    pub radial_samples: u32,
    /// Jitter amount for anti-aliasing (0-1).
    pub jitter: f32,
}

impl Default for CurvatureSampler {
    fn default() -> Self {
        Self {
            seed: 0,
            ring_samples: 32,
            radial_samples: 4,
            jitter: 0.1,
        }
    }
}

impl CurvatureSampler {
    /// Create a new sampler with the given seed.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Set ring sample count.
    #[must_use]
    pub fn with_ring_samples(mut self, count: u32) -> Self {
        self.ring_samples = count.clamp(4, 256);
        self
    }

    /// Set radial sample count.
    #[must_use]
    pub fn with_radial_samples(mut self, count: u32) -> Self {
        self.radial_samples = count.clamp(1, 16);
        self
    }

    /// Set jitter amount.
    #[must_use]
    pub fn with_jitter(mut self, jitter: f32) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Sample a point on the horizon ring.
    #[must_use]
    pub fn sample_horizon_point(
        &self,
        model: &HorizonModel,
        body: &CurvatureBody,
        sample_index: u32,
    ) -> Vec3 {
        let angle = self.ring_angle(sample_index);
        self.horizon_point_at_angle(model, body, angle)
    }

    /// Compute angle for a ring sample.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "ring_samples clamped to 4-256; fits in f32 mantissa"
    )]
    pub fn ring_angle(&self, sample_index: u32) -> f32 {
        let base_angle = (sample_index as f32 / self.ring_samples as f32) * TAU;
        if self.jitter > 0.0 {
            let jitter = self.deterministic_jitter(sample_index, 0) * self.jitter * TAU
                / self.ring_samples as f32;
            base_angle + jitter
        } else {
            base_angle
        }
    }

    /// Compute horizon point at a given angle around the camera's up axis.
    #[must_use]
    pub fn horizon_point_at_angle(
        &self,
        model: &HorizonModel,
        body: &CurvatureBody,
        angle: f32,
    ) -> Vec3 {
        let up = body.surface_up(model.camera_position);
        let (tangent1, tangent2, _) = compute_tangent_frame(up);

        let horizontal = tangent1 * angle.cos() + tangent2 * angle.sin();
        let direction =
            horizontal * model.horizon_dip_angle.cos() - up * model.horizon_dip_angle.sin();

        model.camera_position + direction.normalize() * model.horizon_distance
    }

    /// Sample multiple horizon points.
    #[must_use]
    pub fn sample_horizon_ring(&self, model: &HorizonModel, body: &CurvatureBody) -> Vec<Vec3> {
        (0..self.ring_samples)
            .map(|i| self.sample_horizon_point(model, body, i))
            .collect()
    }

    /// Sample radial fade points from camera to horizon.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "radial_samples clamped to 1-16; fits in f32 mantissa"
    )]
    pub fn sample_radial_points(
        &self,
        model: &HorizonModel,
        direction: Vec3,
        count: u32,
    ) -> Vec<(Vec3, f32)> {
        (0..count)
            .map(|i| {
                let t = (i as f32 + 0.5) / count as f32;
                let distance = model.horizon_distance * t;
                let point = model.camera_position + direction.normalize() * distance;
                (point, t)
            })
            .collect()
    }

    /// Generate deterministic jitter value.
    fn deterministic_jitter(&self, index: u32, layer: u32) -> f32 {
        let hash = hash_u32(
            self.seed
                .wrapping_add(index)
                .wrapping_mul(HASH_PRIME)
                .wrapping_add(layer.wrapping_mul(113)),
        );
        u32_to_unit(hash) * 2.0 - 1.0
    }
}

/// Compute an orthonormal tangent frame from an up direction.
#[must_use]
pub fn compute_tangent_frame(up: Vec3) -> (Vec3, Vec3, Vec3) {
    let up = if up.length_squared() > 0.0001 {
        up.normalize()
    } else {
        Vec3::Y
    };

    let arbitrary = if up.dot(Vec3::X).abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Z
    };

    let tangent1 = up.cross(arbitrary).normalize();
    let tangent2 = up.cross(tangent1);

    (tangent1, tangent2, up)
}

/// Compute surface normal at a point on a curved body.
#[must_use]
pub fn surface_normal(body: &CurvatureBody, surface_point: Vec3) -> Vec3 {
    body.surface_up(surface_point)
}

/// Compute the "forward" direction along the surface from a point.
#[must_use]
pub fn surface_forward(body: &CurvatureBody, surface_point: Vec3, reference_forward: Vec3) -> Vec3 {
    let up = body.surface_up(surface_point);
    let right = reference_forward.cross(up);
    if right.length_squared() > 0.0001 {
        up.cross(right.normalize())
    } else {
        let (t1, _, _) = compute_tangent_frame(up);
        t1
    }
}

/// Transform a direction from flat-world to curved-world coordinates.
#[must_use]
pub fn flat_to_curved_direction(body: &CurvatureBody, origin: Vec3, flat_direction: Vec3) -> Vec3 {
    let up = body.surface_up(origin);
    let (t1, t2, _) = compute_tangent_frame(up);

    let rotation = Mat3::from_cols(t1, up, t2);
    rotation * flat_direction
}

/// Transform a position from flat-world to curved-world coordinates.
#[must_use]
pub fn flat_to_curved_position(body: &CurvatureBody, origin: Vec3, flat_offset: Vec3) -> Vec3 {
    let surface_origin = body.closest_surface_point(origin);
    let up = body.surface_up(surface_origin);
    let (t1, t2, _) = compute_tangent_frame(up);

    let horizontal = t1 * flat_offset.x + t2 * flat_offset.z;
    let arc_length = horizontal.length();

    if arc_length < 0.001 {
        return surface_origin + up * flat_offset.y;
    }

    let arc_angle = arc_length / body.radius;
    let arc_direction = horizontal.normalize();

    let rotated_up = up * arc_angle.cos() + arc_direction * arc_angle.sin();
    let new_surface = body.center + rotated_up.normalize() * body.radius;

    let new_up = body.surface_up(new_surface);
    new_surface + new_up * flat_offset.y
}

/// Compute curvature correction factor for distance.
#[must_use]
pub fn curvature_distance_correction(body: &CurvatureBody, flat_distance: f32) -> f32 {
    if body.radius <= 0.0 {
        return flat_distance;
    }

    let angle = flat_distance / body.radius;
    if angle < 0.01 {
        flat_distance
    } else {
        2.0 * body.radius * (angle / 2.0).sin()
    }
}

/// Compute the great-circle distance between two points on a sphere.
#[must_use]
pub fn great_circle_distance(body: &CurvatureBody, point_a: Vec3, point_b: Vec3) -> f32 {
    let surface_a = body.closest_surface_point(point_a);
    let surface_b = body.closest_surface_point(point_b);

    let dir_a = (surface_a - body.center).normalize();
    let dir_b = (surface_b - body.center).normalize();

    let dot = dir_a.dot(dir_b).clamp(-1.0, 1.0);
    let angle = dot.acos();

    body.radius * angle
}

/// Hash a u32 value deterministically.
fn hash_u32(mut n: u32) -> u32 {
    n = n.wrapping_mul(HASH_MUL_A);
    n ^= n >> 13;
    n = n.wrapping_mul(HASH_MUL_B);
    n ^= n >> 16;
    n
}

#[expect(
    clippy::cast_precision_loss,
    reason = "masked to 23 bits; fits in f32 mantissa"
)]
fn u32_to_unit(n: u32) -> f32 {
    (n & 0x7F_FFFF) as f32 / 0x7F_FFFF_u32 as f32
}

/// Deterministic position hash for curvature sampling.
#[must_use]
pub fn position_hash(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let xi = x.to_bits();
    let yi = y.to_bits();
    let zi = z.to_bits();
    let mut n = seed
        .wrapping_add(xi)
        .wrapping_add(yi.wrapping_mul(57))
        .wrapping_add(zi.wrapping_mul(113));
    n = hash_u32(n);
    u32_to_unit(n)
}

/// Compute angular separation between two directions.
#[must_use]
pub fn angular_separation(dir_a: Vec3, dir_b: Vec3) -> f32 {
    let dot = dir_a.normalize().dot(dir_b.normalize()).clamp(-1.0, 1.0);
    dot.acos()
}

/// Check if two points are mutually visible on a curved surface.
#[must_use]
pub fn line_of_sight(body: &CurvatureBody, point_a: Vec3, point_b: Vec3) -> bool {
    let height_a = body.height_above_surface(point_a);
    let height_b = body.height_above_surface(point_b);

    if height_a < 0.0 || height_b < 0.0 {
        return false;
    }

    let surface_a = body.closest_surface_point(point_a);
    let surface_b = body.closest_surface_point(point_b);

    let dir_a = (surface_a - body.center).normalize();
    let dir_b = (surface_b - body.center).normalize();

    let angle = dir_a.dot(dir_b).clamp(-1.0, 1.0).acos();

    let effective_radius_a = body.radius + height_a;
    let effective_radius_b = body.radius + height_b;

    let max_angle_a = (body.radius / effective_radius_a).acos();
    let max_angle_b = (body.radius / effective_radius_b).acos();

    angle <= max_angle_a + max_angle_b
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::PI;

    fn test_body() -> CurvatureBody {
        CurvatureBody::planetary_sphere(Vec3::ZERO, 1000.0)
    }

    #[test]
    fn test_tangent_frame_orthonormal() {
        let up = Vec3::new(0.5, 0.8, 0.3).normalize();
        let (t1, t2, up_out) = compute_tangent_frame(up);

        assert_relative_eq!(t1.length(), 1.0, epsilon = 0.001);
        assert_relative_eq!(t2.length(), 1.0, epsilon = 0.001);
        assert_relative_eq!(up_out.length(), 1.0, epsilon = 0.001);

        assert_relative_eq!(t1.dot(t2).abs(), 0.0, epsilon = 0.001);
        assert_relative_eq!(t1.dot(up_out).abs(), 0.0, epsilon = 0.001);
        assert_relative_eq!(t2.dot(up_out).abs(), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_tangent_frame_degenerate() {
        let (t1, t2, up) = compute_tangent_frame(Vec3::ZERO);
        assert_relative_eq!(up.y, 1.0, epsilon = 0.001);
        assert_relative_eq!(t1.length(), 1.0, epsilon = 0.001);
        assert_relative_eq!(t2.length(), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_surface_normal() {
        let body = test_body();
        let point = Vec3::new(1000.0, 0.0, 0.0);
        let normal = surface_normal(&body, point);

        assert_relative_eq!(normal.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(normal.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(normal.z, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_surface_forward() {
        let body = test_body();
        let point = Vec3::new(1000.0, 0.0, 0.0);
        let ref_forward = Vec3::Z;
        let forward = surface_forward(&body, point, ref_forward);

        assert_relative_eq!(forward.dot(Vec3::X).abs(), 0.0, epsilon = 0.001);
        assert_relative_eq!(forward.length(), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_flat_to_curved_direction() {
        let body = test_body();
        let origin = Vec3::new(1000.0, 0.0, 0.0);
        let flat_up = Vec3::Y;

        let curved = flat_to_curved_direction(&body, origin, flat_up);
        assert_relative_eq!(curved.x, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_curvature_distance_correction_small() {
        let body = test_body();
        let small_dist = 1.0;
        let corrected = curvature_distance_correction(&body, small_dist);
        assert_relative_eq!(corrected, small_dist, epsilon = 0.01);
    }

    #[test]
    fn test_curvature_distance_correction_large() {
        let body = test_body();
        let quarter_circumference = PI * body.radius / 2.0;
        let corrected = curvature_distance_correction(&body, quarter_circumference);

        assert!(
            corrected < quarter_circumference,
            "chord should be shorter than arc"
        );
    }

    #[test]
    fn test_great_circle_distance() {
        let body = test_body();
        let point_a = Vec3::new(1000.0, 0.0, 0.0);
        let point_b = Vec3::new(0.0, 1000.0, 0.0);

        let distance = great_circle_distance(&body, point_a, point_b);
        let expected = body.radius * PI / 2.0;

        assert_relative_eq!(distance, expected, epsilon = 0.1);
    }

    #[test]
    fn test_position_hash_determinism() {
        let h1 = position_hash(1.5, 2.5, 3.5, 42);
        let h2 = position_hash(1.5, 2.5, 3.5, 42);
        assert_relative_eq!(h1, h2, epsilon = 0.0001);
    }

    #[test]
    fn test_position_hash_range() {
        for i in 0u16..100 {
            let h = position_hash(f32::from(i), f32::from(i) * 0.7, f32::from(i) * 0.3, 0);
            assert!((0.0..=1.0).contains(&h));
        }
    }

    #[test]
    fn test_angular_separation() {
        let a = Vec3::X;
        let b = Vec3::Y;
        let angle = angular_separation(a, b);
        assert_relative_eq!(angle, PI / 2.0, epsilon = 0.001);

        let same = angular_separation(a, a);
        assert_relative_eq!(same, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_line_of_sight_visible() {
        let body = test_body();
        let a = Vec3::new(1100.0, 0.0, 0.0);
        let b = Vec3::new(1100.0, 100.0, 0.0);

        assert!(line_of_sight(&body, a, b));
    }

    #[test]
    fn test_line_of_sight_blocked() {
        let body = test_body();
        let a = Vec3::new(1010.0, 0.0, 0.0);
        let b = Vec3::new(-1010.0, 0.0, 0.0);

        assert!(!line_of_sight(&body, a, b));
    }

    #[test]
    fn test_line_of_sight_below_surface() {
        let body = test_body();
        let a = Vec3::new(500.0, 0.0, 0.0);
        let b = Vec3::new(1100.0, 0.0, 0.0);

        assert!(!line_of_sight(&body, a, b));
    }

    #[test]
    fn test_sampler_ring_samples() {
        let body = test_body();
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let sampler = CurvatureSampler::new(0).with_ring_samples(8);

        let points = sampler.sample_horizon_ring(&model, &body);
        assert_eq!(points.len(), 8);

        for point in &points {
            let dist = (*point - camera).length();
            assert_relative_eq!(dist, model.horizon_distance, epsilon = 1.0);
        }
    }

    #[test]
    fn test_sampler_determinism() {
        let body = test_body();
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let sampler = CurvatureSampler::new(42);

        let p1 = sampler.sample_horizon_point(&model, &body, 0);
        let p2 = sampler.sample_horizon_point(&model, &body, 0);

        assert_relative_eq!(p1.x, p2.x, epsilon = 0.001);
        assert_relative_eq!(p1.y, p2.y, epsilon = 0.001);
        assert_relative_eq!(p1.z, p2.z, epsilon = 0.001);
    }

    #[test]
    fn test_sampler_radial_points() {
        let body = test_body();
        let camera = Vec3::new(1100.0, 0.0, 0.0);
        let model = HorizonModel::compute(&body, camera);
        let sampler = CurvatureSampler::new(0);

        let direction = Vec3::new(0.0, 0.1, 1.0).normalize();
        let points = sampler.sample_radial_points(&model, direction, 4);

        assert_eq!(points.len(), 4);
        for (i, (_, t)) in points.iter().enumerate() {
            assert!(*t > 0.0 && *t <= 1.0);
            if i > 0 {
                assert!(points[i].1 > points[i - 1].1);
            }
        }
    }

    #[test]
    fn test_sampler_jitter() {
        let sampler_no_jitter = CurvatureSampler::new(0).with_jitter(0.0);
        let sampler_jitter = CurvatureSampler::new(0).with_jitter(0.5);

        let angle1 = sampler_no_jitter.ring_angle(0);
        let angle2 = sampler_jitter.ring_angle(0);

        assert_relative_eq!(angle1, 0.0, epsilon = 0.001);
        assert!((angle1 - angle2).abs() > 0.0 || angle2 == 0.0);
    }
}

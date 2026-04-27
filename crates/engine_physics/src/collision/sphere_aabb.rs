//! Sphere-AABB collision detection for projectiles.

use super::ccd;
use engine_core::math::{Aabb, Sphere};
use glam::Vec3;

/// Contact information from sphere-AABB collision.
#[derive(Clone, Copy, Debug)]
pub struct SphereAabbContact {
    /// Contact normal (points from AABB toward sphere center).
    pub normal: Vec3,
    /// Penetration depth (positive when overlapping).
    pub penetration: f32,
    /// Contact point on AABB surface.
    pub point: Vec3,
}

/// Test if a sphere intersects an AABB.
///
/// Returns contact information if they overlap, including the normal
/// pointing from the AABB toward the sphere (the direction to push
/// the sphere to resolve the collision).
#[must_use]
#[expect(
    clippy::similar_names,
    reason = "axis-aligned distances use x/y/z prefixes"
)]
pub fn sphere_aabb_intersection(sphere: &Sphere, aabb: &Aabb) -> Option<SphereAabbContact> {
    // Find the closest point on the AABB to the sphere center
    let closest = Vec3::new(
        sphere.center.x.clamp(aabb.min.x, aabb.max.x),
        sphere.center.y.clamp(aabb.min.y, aabb.max.y),
        sphere.center.z.clamp(aabb.min.z, aabb.max.z),
    );

    let diff = sphere.center - closest;
    let distance_sq = diff.length_squared();
    let radius_sq = sphere.radius * sphere.radius;

    if distance_sq > radius_sq {
        return None;
    }

    let distance = distance_sq.sqrt();

    // Calculate contact normal and penetration
    let (normal, penetration) = if distance > 0.0001 {
        // Sphere center is outside AABB
        (diff / distance, sphere.radius - distance)
    } else {
        // Sphere center is inside AABB - find axis of least penetration
        let center = aabb.center();
        let half = aabb.half_extents();
        let local = sphere.center - center;

        // Distance to each face
        let (dx_pos, dx_neg) = (half.x - local.x, half.x + local.x);
        let (dy_pos, dy_neg) = (half.y - local.y, half.y + local.y);
        let (dz_pos, dz_neg) = (half.z - local.z, half.z + local.z);

        // Find minimum penetration axis
        let min_dist = dx_pos
            .min(dx_neg)
            .min(dy_pos)
            .min(dy_neg)
            .min(dz_pos)
            .min(dz_neg);

        let normal = if (min_dist - dx_pos).abs() < 0.0001 {
            Vec3::X
        } else if (min_dist - dx_neg).abs() < 0.0001 {
            Vec3::NEG_X
        } else if (min_dist - dy_pos).abs() < 0.0001 {
            Vec3::Y
        } else if (min_dist - dy_neg).abs() < 0.0001 {
            Vec3::NEG_Y
        } else if (min_dist - dz_pos).abs() < 0.0001 {
            Vec3::Z
        } else {
            Vec3::NEG_Z
        };

        (normal, sphere.radius + min_dist)
    };

    Some(SphereAabbContact {
        normal,
        penetration,
        point: closest,
    })
}

/// Test if a moving sphere will intersect an AABB.
///
/// Uses swept sphere test for continuous collision detection.
/// Returns the time of first contact (0.0 to 1.0) if collision occurs.
#[must_use]
pub fn sphere_aabb_sweep(sphere: &Sphere, velocity: Vec3, aabb: &Aabb) -> Option<f32> {
    ccd::swept_sphere_aabb(sphere, velocity, aabb).map(|hit| hit.time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_inside_aabb() {
        let sphere = Sphere::new(Vec3::new(0.5, 0.5, 0.5), 0.2);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let contact = sphere_aabb_intersection(&sphere, &aabb);
        assert!(contact.is_some());
    }

    #[test]
    fn test_sphere_touching_aabb_face() {
        let sphere = Sphere::new(Vec3::new(1.5, 0.5, 0.5), 0.5);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let contact = sphere_aabb_intersection(&sphere, &aabb);
        assert!(contact.is_some());

        let contact = contact.unwrap();
        assert!(contact.normal.x > 0.9, "Normal should point +X");
        assert!(contact.penetration >= 0.0);
    }

    #[test]
    fn test_sphere_not_touching_aabb() {
        let sphere = Sphere::new(Vec3::new(3.0, 0.5, 0.5), 0.5);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let contact = sphere_aabb_intersection(&sphere, &aabb);
        assert!(contact.is_none());
    }

    #[test]
    fn test_sphere_touching_aabb_corner() {
        // Sphere at corner of AABB
        let sphere = Sphere::new(Vec3::new(1.3, 1.3, 1.3), 0.6);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let contact = sphere_aabb_intersection(&sphere, &aabb);
        assert!(contact.is_some());
    }

    #[test]
    fn test_sweep_hit() {
        let sphere = Sphere::new(Vec3::new(-1.0, 0.5, 0.5), 0.2);
        let velocity = Vec3::new(2.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_some());
        assert!(t.unwrap() > 0.0 && t.unwrap() < 1.0);
    }

    #[test]
    fn test_sweep_miss() {
        let sphere = Sphere::new(Vec3::new(-1.0, 5.0, 0.5), 0.2);
        let velocity = Vec3::new(2.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_none());
    }

    #[test]
    fn test_sweep_already_inside() {
        let sphere = Sphere::new(Vec3::new(0.5, 0.5, 0.5), 0.2);
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_some());
        assert!((t.unwrap() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_sweep_zero_velocity() {
        let sphere = Sphere::new(Vec3::new(-1.0, 0.5, 0.5), 0.2);
        let velocity = Vec3::ZERO;
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_none(), "Zero velocity, not overlapping = no hit");
    }

    #[test]
    fn test_sweep_zero_velocity_overlapping() {
        let sphere = Sphere::new(Vec3::new(0.5, 0.5, 0.5), 0.2);
        let velocity = Vec3::ZERO;
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_some(), "Zero velocity but overlapping = hit at t=0");
        assert!((t.unwrap() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_sweep_zero_axis_outside_slab() {
        let sphere = Sphere::new(Vec3::new(-1.0, 5.0, 0.5), 0.2);
        let velocity = Vec3::new(2.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_none(), "Outside Y slab with zero Y velocity = miss");
    }

    #[test]
    fn test_sweep_zero_axis_inside_slab() {
        let sphere = Sphere::new(Vec3::new(-1.0, 0.5, 0.5), 0.2);
        let velocity = Vec3::new(2.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let t = sphere_aabb_sweep(&sphere, velocity, &aabb);
        assert!(t.is_some(), "Inside Y/Z slabs with zero Y/Z velocity = hit");
        assert!(t.unwrap() > 0.0 && t.unwrap() < 1.0);
    }
}

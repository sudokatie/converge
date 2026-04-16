//! Axis-Aligned Bounding Box.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::Sphere;

/// Axis-Aligned Bounding Box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb {
    /// Create a new AABB from min and max corners.
    #[must_use]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        debug_assert!(min.x <= max.x && min.y <= max.y && min.z <= max.z);
        Self { min, max }
    }

    /// Create an AABB from center point and half-extents.
    #[must_use]
    pub fn from_center_half_extents(center: Vec3, half: Vec3) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Get the center point.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the half-extents.
    #[must_use]
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Get the full size (dimensions).
    #[must_use]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Check if a point is inside the AABB.
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Check if this AABB intersects another AABB.
    #[must_use]
    pub fn intersects_aabb(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Expand the AABB by a given amount in all directions.
    #[must_use]
    pub fn expand(&self, amount: f32) -> Aabb {
        Aabb {
            min: self.min - Vec3::splat(amount),
            max: self.max + Vec3::splat(amount),
        }
    }

    /// Merge two AABBs into one that contains both.
    #[must_use]
    pub fn merge(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Check if this AABB intersects a sphere.
    ///
    /// Uses closest point on AABB to sphere center approach.
    #[must_use]
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        // Find the closest point on the AABB to the sphere center
        let closest = Vec3::new(
            sphere.center.x.clamp(self.min.x, self.max.x),
            sphere.center.y.clamp(self.min.y, self.max.y),
            sphere.center.z.clamp(self.min.z, self.max.z),
        );

        // Check if that point is within the sphere's radius
        let distance_squared = closest.distance_squared(sphere.center);
        distance_squared <= sphere.radius * sphere.radius
    }

    /// Check if a ray intersects this AABB.
    ///
    /// Uses the slab method for ray-AABB intersection.
    /// Direction should be normalized or at least non-zero.
    #[must_use]
    pub fn intersects_ray(&self, origin: Vec3, direction: Vec3) -> bool {
        // Slab method: find intersection intervals for each axis
        let inv_dir = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);

        let t1 = (self.min.x - origin.x) * inv_dir.x;
        let t2 = (self.max.x - origin.x) * inv_dir.x;
        let t3 = (self.min.y - origin.y) * inv_dir.y;
        let t4 = (self.max.y - origin.y) * inv_dir.y;
        let t5 = (self.min.z - origin.z) * inv_dir.z;
        let t6 = (self.max.z - origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        // If tmax < 0, ray intersects AABB but entire AABB is behind origin
        // If tmin > tmax, ray doesn't intersect AABB
        tmax >= 0.0 && tmin <= tmax
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_new() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ONE);
    }

    #[test]
    fn test_from_center_half_extents() {
        let aabb = Aabb::from_center_half_extents(Vec3::splat(1.0), Vec3::splat(0.5));
        assert_relative_eq!(aabb.min.x, 0.5);
        assert_relative_eq!(aabb.max.x, 1.5);
    }

    #[test]
    fn test_center() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::splat(2.0));
        let center = aabb.center();
        assert_relative_eq!(center.x, 1.0);
        assert_relative_eq!(center.y, 1.0);
        assert_relative_eq!(center.z, 1.0);
    }

    #[test]
    fn test_contains_point() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        assert!(aabb.contains_point(Vec3::splat(0.5)));
        assert!(aabb.contains_point(Vec3::ZERO)); // On edge
        assert!(!aabb.contains_point(Vec3::splat(2.0)));
    }

    #[test]
    fn test_intersects_aabb_overlapping() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));
        assert!(a.intersects_aabb(&b));
        assert!(b.intersects_aabb(&a));
    }

    #[test]
    fn test_intersects_aabb_touching() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));
        assert!(a.intersects_aabb(&b)); // Touching counts as intersection
    }

    #[test]
    fn test_intersects_aabb_separate() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));
        assert!(!a.intersects_aabb(&b));
    }

    #[test]
    fn test_expand() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let expanded = aabb.expand(0.5);
        assert_relative_eq!(expanded.min.x, -0.5);
        assert_relative_eq!(expanded.max.x, 1.5);
    }

    #[test]
    fn test_merge() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));
        let merged = a.merge(&b);
        assert_eq!(merged.min, Vec3::ZERO);
        assert_eq!(merged.max, Vec3::splat(3.0));
    }

    #[test]
    fn test_intersects_sphere_inside() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::splat(2.0));
        let sphere = Sphere::new(Vec3::ONE, 0.5);
        assert!(aabb.intersects_sphere(&sphere));
    }

    #[test]
    fn test_intersects_sphere_touching() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        // Sphere touching the +X face
        let sphere = Sphere::new(Vec3::new(2.0, 0.5, 0.5), 1.0);
        assert!(aabb.intersects_sphere(&sphere));
    }

    #[test]
    fn test_intersects_sphere_outside() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let sphere = Sphere::new(Vec3::splat(5.0), 1.0);
        assert!(!aabb.intersects_sphere(&sphere));
    }

    #[test]
    fn test_intersects_ray_hit() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        // Ray from (-1, 0.5, 0.5) pointing +X
        let origin = Vec3::new(-1.0, 0.5, 0.5);
        let direction = Vec3::X;
        assert!(aabb.intersects_ray(origin, direction));
    }

    #[test]
    fn test_intersects_ray_miss() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        // Ray from (-1, 5, 0.5) pointing +X - misses above
        let origin = Vec3::new(-1.0, 5.0, 0.5);
        let direction = Vec3::X;
        assert!(!aabb.intersects_ray(origin, direction));
    }

    #[test]
    fn test_intersects_ray_inside() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        // Ray starting inside the AABB
        let origin = Vec3::splat(0.5);
        let direction = Vec3::X;
        assert!(aabb.intersects_ray(origin, direction));
    }

    #[test]
    fn test_intersects_ray_behind() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        // Ray pointing away from AABB
        let origin = Vec3::new(-1.0, 0.5, 0.5);
        let direction = Vec3::NEG_X;
        assert!(!aabb.intersects_ray(origin, direction));
    }
}

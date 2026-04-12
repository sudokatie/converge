//! Sphere primitive.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A sphere defined by center and radius.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sphere {
    /// Center point.
    pub center: Vec3,
    /// Radius.
    pub radius: f32,
}

impl Sphere {
    /// Create a new sphere.
    #[must_use]
    pub fn new(center: Vec3, radius: f32) -> Self {
        debug_assert!(radius >= 0.0);
        Self { center, radius }
    }

    /// Check if a point is inside or on the sphere.
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.center.distance_squared(point) <= self.radius * self.radius
    }

    /// Check if this sphere intersects another sphere.
    #[must_use]
    pub fn intersects_sphere(&self, other: &Sphere) -> bool {
        let combined_radius = self.radius + other.radius;
        self.center.distance_squared(other.center) <= combined_radius * combined_radius
    }

    /// Expand the sphere by a given amount.
    #[must_use]
    pub fn expand(&self, amount: f32) -> Sphere {
        Sphere {
            center: self.center,
            radius: self.radius + amount,
        }
    }
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            radius: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let s = Sphere::new(Vec3::ONE, 2.0);
        assert_eq!(s.center, Vec3::ONE);
        assert_eq!(s.radius, 2.0);
    }

    #[test]
    fn test_contains_point_inside() {
        let s = Sphere::new(Vec3::ZERO, 1.0);
        assert!(s.contains_point(Vec3::splat(0.3)));
    }

    #[test]
    fn test_contains_point_on_surface() {
        let s = Sphere::new(Vec3::ZERO, 1.0);
        assert!(s.contains_point(Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_contains_point_outside() {
        let s = Sphere::new(Vec3::ZERO, 1.0);
        assert!(!s.contains_point(Vec3::splat(2.0)));
    }

    #[test]
    fn test_intersects_sphere_overlapping() {
        let a = Sphere::new(Vec3::ZERO, 1.0);
        let b = Sphere::new(Vec3::new(1.0, 0.0, 0.0), 1.0);
        assert!(a.intersects_sphere(&b));
    }

    #[test]
    fn test_intersects_sphere_touching() {
        let a = Sphere::new(Vec3::ZERO, 1.0);
        let b = Sphere::new(Vec3::new(2.0, 0.0, 0.0), 1.0);
        assert!(a.intersects_sphere(&b)); // Touching counts
    }

    #[test]
    fn test_intersects_sphere_separate() {
        let a = Sphere::new(Vec3::ZERO, 1.0);
        let b = Sphere::new(Vec3::new(3.0, 0.0, 0.0), 1.0);
        assert!(!a.intersects_sphere(&b));
    }
}

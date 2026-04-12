//! Plane primitive.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A plane defined by normal and distance from origin.
///
/// The plane equation is: dot(normal, point) = distance
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    /// Plane normal (should be normalized).
    pub normal: Vec3,
    /// Distance from origin along the normal.
    pub distance: f32,
}

impl Plane {
    /// Create a new plane from normal and distance.
    #[must_use]
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self {
            normal: normal.normalize(),
            distance,
        }
    }

    /// Create a plane from a point on the plane and a normal.
    #[must_use]
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let normal = normal.normalize();
        let distance = normal.dot(point);
        Self { normal, distance }
    }

    /// Create a plane from three points (counter-clockwise winding).
    #[must_use]
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let normal = (b - a).cross(c - a).normalize();
        let distance = normal.dot(a);
        Self { normal, distance }
    }

    /// Get the signed distance from a point to the plane.
    ///
    /// Positive means in front (normal side), negative means behind.
    #[must_use]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) - self.distance
    }

    /// Check which side of the plane a point is on.
    #[must_use]
    pub fn classify_point(&self, point: Vec3, epsilon: f32) -> PlaneSide {
        let dist = self.signed_distance(point);
        if dist > epsilon {
            PlaneSide::Front
        } else if dist < -epsilon {
            PlaneSide::Back
        } else {
            PlaneSide::On
        }
    }
}

/// Which side of a plane a point is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneSide {
    /// In front of the plane (positive side of normal).
    Front,
    /// Behind the plane (negative side of normal).
    Back,
    /// On the plane (within epsilon).
    On,
}

impl Default for Plane {
    fn default() -> Self {
        Self {
            normal: Vec3::Y,
            distance: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_new() {
        let p = Plane::new(Vec3::Y, 5.0);
        assert_relative_eq!(p.normal.length(), 1.0, epsilon = 1e-6);
        assert_eq!(p.distance, 5.0);
    }

    #[test]
    fn test_from_point_normal() {
        let p = Plane::from_point_normal(Vec3::new(0.0, 5.0, 0.0), Vec3::Y);
        assert_relative_eq!(p.distance, 5.0);
    }

    #[test]
    fn test_signed_distance_front() {
        let p = Plane::new(Vec3::Y, 0.0);
        let dist = p.signed_distance(Vec3::new(0.0, 5.0, 0.0));
        assert_relative_eq!(dist, 5.0);
    }

    #[test]
    fn test_signed_distance_back() {
        let p = Plane::new(Vec3::Y, 0.0);
        let dist = p.signed_distance(Vec3::new(0.0, -5.0, 0.0));
        assert_relative_eq!(dist, -5.0);
    }

    #[test]
    fn test_classify_point() {
        let p = Plane::new(Vec3::Y, 0.0);
        assert_eq!(p.classify_point(Vec3::new(0.0, 1.0, 0.0), 0.001), PlaneSide::Front);
        assert_eq!(p.classify_point(Vec3::new(0.0, -1.0, 0.0), 0.001), PlaneSide::Back);
        assert_eq!(p.classify_point(Vec3::new(0.0, 0.0, 0.0), 0.001), PlaneSide::On);
    }
}

//! Ray primitive.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A ray with origin, direction, and maximum distance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ray {
    /// Ray origin.
    pub origin: Vec3,
    /// Ray direction (should be normalized).
    pub direction: Vec3,
    /// Maximum distance (t parameter).
    pub t_max: f32,
}

impl Ray {
    /// Create a new ray with unlimited distance.
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
            t_max: f32::INFINITY,
        }
    }

    /// Create a new ray with a maximum distance.
    #[must_use]
    pub fn with_max(origin: Vec3, direction: Vec3, t_max: f32) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
            t_max,
        }
    }

    /// Get a point along the ray at parameter t.
    #[must_use]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

impl Default for Ray {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            direction: Vec3::Z,
            t_max: f32::INFINITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_new_normalizes_direction() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0));
        assert_relative_eq!(ray.direction.length(), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_at() {
        let ray = Ray::new(Vec3::ZERO, Vec3::X);
        let point = ray.at(5.0);
        assert_relative_eq!(point.x, 5.0);
        assert_relative_eq!(point.y, 0.0);
    }

    #[test]
    fn test_with_max() {
        let ray = Ray::with_max(Vec3::ZERO, Vec3::X, 10.0);
        assert_eq!(ray.t_max, 10.0);
    }
}

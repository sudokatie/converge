//! Oriented Bounding Box.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Oriented Bounding Box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Obb {
    /// Center point.
    pub center: Vec3,
    /// Half-extents along local axes.
    pub half_extents: Vec3,
    /// Rotation (local to world).
    pub rotation: Quat,
}

impl Obb {
    /// Create a new OBB.
    #[must_use]
    pub fn new(center: Vec3, half_extents: Vec3, rotation: Quat) -> Self {
        Self {
            center,
            half_extents,
            rotation,
        }
    }

    /// Create an axis-aligned OBB (no rotation).
    #[must_use]
    pub fn axis_aligned(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            half_extents,
            rotation: Quat::IDENTITY,
        }
    }

    /// Get the three local axes in world space.
    #[must_use]
    pub fn axes(&self) -> [Vec3; 3] {
        [
            self.rotation * Vec3::X,
            self.rotation * Vec3::Y,
            self.rotation * Vec3::Z,
        ]
    }

    /// Get the 8 corners of the OBB in world space.
    #[must_use]
    pub fn corners(&self) -> [Vec3; 8] {
        let axes = self.axes();
        let x = axes[0] * self.half_extents.x;
        let y = axes[1] * self.half_extents.y;
        let z = axes[2] * self.half_extents.z;

        [
            self.center - x - y - z,
            self.center + x - y - z,
            self.center - x + y - z,
            self.center + x + y - z,
            self.center - x - y + z,
            self.center + x - y + z,
            self.center - x + y + z,
            self.center + x + y + z,
        ]
    }

    /// Transform a point from world space to local OBB space.
    #[must_use]
    pub fn world_to_local(&self, point: Vec3) -> Vec3 {
        self.rotation.inverse() * (point - self.center)
    }

    /// Check if a point is inside the OBB.
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        let local = self.world_to_local(point);
        local.x.abs() <= self.half_extents.x
            && local.y.abs() <= self.half_extents.y
            && local.z.abs() <= self.half_extents.z
    }
}

impl Default for Obb {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            half_extents: Vec3::ONE,
            rotation: Quat::IDENTITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_axis_aligned() {
        let obb = Obb::axis_aligned(Vec3::ZERO, Vec3::ONE);
        assert_eq!(obb.rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_contains_point_inside() {
        let obb = Obb::axis_aligned(Vec3::ZERO, Vec3::ONE);
        assert!(obb.contains_point(Vec3::splat(0.5)));
    }

    #[test]
    fn test_contains_point_outside() {
        let obb = Obb::axis_aligned(Vec3::ZERO, Vec3::ONE);
        assert!(!obb.contains_point(Vec3::splat(2.0)));
    }

    #[test]
    fn test_contains_point_rotated() {
        let obb = Obb::new(
            Vec3::ZERO,
            Vec3::new(2.0, 0.5, 0.5),
            Quat::from_rotation_z(FRAC_PI_2),
        );
        // After 90 degree Z rotation, the long axis is now Y
        assert!(obb.contains_point(Vec3::new(0.0, 1.5, 0.0)));
        assert!(!obb.contains_point(Vec3::new(1.5, 0.0, 0.0)));
    }

    #[test]
    fn test_corners_count() {
        let obb = Obb::default();
        let corners = obb.corners();
        assert_eq!(corners.len(), 8);
    }
}

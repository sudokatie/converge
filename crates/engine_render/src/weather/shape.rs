//! Spawn shapes for particle emitters.
//!
//! Defines the volume from which particles are spawned.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Shape used for spawning particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum SpawnShapeKind {
    /// Single point.
    #[default]
    Point = 0,
    /// Spherical volume.
    Sphere = 1,
    /// Spherical shell (surface only).
    SphereShell = 2,
    /// Axis-aligned box.
    Box = 3,
    /// Box surface only.
    BoxShell = 4,
    /// Cylindrical volume (vertical axis).
    Cylinder = 5,
    /// Cone emanating from apex.
    Cone = 6,
    /// Disc (flat circle).
    Disc = 7,
    /// Line segment.
    Line = 8,
}

impl SpawnShapeKind {
    /// All spawn shape kinds.
    pub const ALL: [Self; 9] = [
        Self::Point,
        Self::Sphere,
        Self::SphereShell,
        Self::Box,
        Self::BoxShell,
        Self::Cylinder,
        Self::Cone,
        Self::Disc,
        Self::Line,
    ];
}

/// Configuration for a spawn shape.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpawnShape {
    /// Shape kind.
    pub kind: SpawnShapeKind,
    /// Center/origin position in local space.
    pub center: Vec3,
    /// Dimensions/extents depending on shape:
    /// - Sphere/SphereShell: x = radius
    /// - Box/BoxShell: half-extents
    /// - Cylinder: x = radius, y = half-height
    /// - Cone: x = angle (radians), y = length
    /// - Disc: x = radius
    /// - Line: direction * length
    pub extents: Vec3,
    /// Secondary parameter:
    /// - `SphereShell`: x = inner radius ratio (0-1)
    /// - `Cone`: x = apex radius (truncated cone)
    pub secondary: Vec3,
}

impl Default for SpawnShape {
    fn default() -> Self {
        Self::point(Vec3::ZERO)
    }
}

impl SpawnShape {
    /// Create a point spawn shape.
    #[must_use]
    pub fn point(position: Vec3) -> Self {
        Self {
            kind: SpawnShapeKind::Point,
            center: position,
            extents: Vec3::ZERO,
            secondary: Vec3::ZERO,
        }
    }

    /// Create a sphere spawn shape.
    #[must_use]
    pub fn sphere(center: Vec3, radius: f32) -> Self {
        Self {
            kind: SpawnShapeKind::Sphere,
            center,
            extents: Vec3::splat(radius),
            secondary: Vec3::ZERO,
        }
    }

    /// Create a sphere shell (surface) spawn shape.
    #[must_use]
    pub fn sphere_shell(center: Vec3, radius: f32, thickness: f32) -> Self {
        let inner_ratio = ((radius - thickness) / radius).max(0.0);
        Self {
            kind: SpawnShapeKind::SphereShell,
            center,
            extents: Vec3::splat(radius),
            secondary: Vec3::splat(inner_ratio),
        }
    }

    /// Create an axis-aligned box spawn shape.
    #[must_use]
    pub fn box_volume(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            kind: SpawnShapeKind::Box,
            center,
            extents: half_extents,
            secondary: Vec3::ZERO,
        }
    }

    /// Create a box shell (surface) spawn shape.
    #[must_use]
    pub fn box_shell(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            kind: SpawnShapeKind::BoxShell,
            center,
            extents: half_extents,
            secondary: Vec3::ZERO,
        }
    }

    /// Create a cylinder spawn shape (vertical axis).
    #[must_use]
    pub fn cylinder(center: Vec3, radius: f32, half_height: f32) -> Self {
        Self {
            kind: SpawnShapeKind::Cylinder,
            center,
            extents: Vec3::new(radius, half_height, radius),
            secondary: Vec3::ZERO,
        }
    }

    /// Create a cone spawn shape.
    #[must_use]
    pub fn cone(apex: Vec3, direction: Vec3, angle: f32, length: f32) -> Self {
        let dir = direction.normalize_or_zero();
        Self {
            kind: SpawnShapeKind::Cone,
            center: apex,
            extents: Vec3::new(angle, length, 0.0),
            secondary: dir,
        }
    }

    /// Create a disc spawn shape (flat circle, XZ plane by default).
    #[must_use]
    pub fn disc(center: Vec3, radius: f32) -> Self {
        Self {
            kind: SpawnShapeKind::Disc,
            center,
            extents: Vec3::new(radius, 0.0, radius),
            secondary: Vec3::Y,
        }
    }

    /// Create a disc with custom normal.
    #[must_use]
    pub fn disc_with_normal(center: Vec3, radius: f32, normal: Vec3) -> Self {
        Self {
            kind: SpawnShapeKind::Disc,
            center,
            extents: Vec3::new(radius, 0.0, radius),
            secondary: normal.normalize_or_zero(),
        }
    }

    /// Create a line segment spawn shape.
    #[must_use]
    pub fn line(start: Vec3, end: Vec3) -> Self {
        Self {
            kind: SpawnShapeKind::Line,
            center: start,
            extents: end - start,
            secondary: Vec3::ZERO,
        }
    }

    /// Compute axis-aligned bounding box for this shape.
    #[must_use]
    pub fn aabb(&self) -> (Vec3, Vec3) {
        match self.kind {
            SpawnShapeKind::Point => (self.center, self.center),
            SpawnShapeKind::Sphere | SpawnShapeKind::SphereShell => {
                let radius = self.extents.x;
                (
                    self.center - Vec3::splat(radius),
                    self.center + Vec3::splat(radius),
                )
            }
            SpawnShapeKind::Box | SpawnShapeKind::BoxShell => {
                (self.center - self.extents, self.center + self.extents)
            }
            SpawnShapeKind::Cylinder => {
                let radius = self.extents.x;
                let half_height = self.extents.y;
                (
                    self.center - Vec3::new(radius, half_height, radius),
                    self.center + Vec3::new(radius, half_height, radius),
                )
            }
            SpawnShapeKind::Cone => {
                let angle = self.extents.x;
                let length = self.extents.y;
                let max_radius = length * angle.tan();
                (
                    self.center - Vec3::splat(max_radius.max(length)),
                    self.center + Vec3::splat(max_radius.max(length)),
                )
            }
            SpawnShapeKind::Disc => {
                let radius = self.extents.x;
                (
                    self.center - Vec3::new(radius, 0.0, radius),
                    self.center + Vec3::new(radius, 0.0, radius),
                )
            }
            SpawnShapeKind::Line => {
                let end = self.center + self.extents;
                (self.center.min(end), self.center.max(end))
            }
        }
    }

    /// Compute the volume of this shape (0 for surface/line/point shapes).
    #[must_use]
    pub fn volume(&self) -> f32 {
        use std::f32::consts::PI;
        match self.kind {
            SpawnShapeKind::Point
            | SpawnShapeKind::Line
            | SpawnShapeKind::BoxShell
            | SpawnShapeKind::Disc => 0.0,
            SpawnShapeKind::Sphere => {
                let r = self.extents.x;
                (4.0 / 3.0) * PI * r * r * r
            }
            SpawnShapeKind::SphereShell => {
                let r_outer = self.extents.x;
                let r_inner = r_outer * self.secondary.x;
                (4.0 / 3.0) * PI * (r_outer.powi(3) - r_inner.powi(3))
            }
            SpawnShapeKind::Box => 8.0 * self.extents.x * self.extents.y * self.extents.z,
            SpawnShapeKind::Cylinder => {
                let r = self.extents.x;
                let h = self.extents.y * 2.0;
                PI * r * r * h
            }
            SpawnShapeKind::Cone => {
                let angle = self.extents.x;
                let length = self.extents.y;
                let r = length * angle.tan();
                (1.0 / 3.0) * PI * r * r * length
            }
        }
    }

    /// Check if values are valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self.kind {
            SpawnShapeKind::Point | SpawnShapeKind::Line => true,
            SpawnShapeKind::Sphere | SpawnShapeKind::SphereShell | SpawnShapeKind::Disc => {
                self.extents.x >= 0.0
            }
            SpawnShapeKind::Box | SpawnShapeKind::BoxShell => {
                self.extents.x >= 0.0 && self.extents.y >= 0.0 && self.extents.z >= 0.0
            }
            SpawnShapeKind::Cylinder => self.extents.x >= 0.0 && self.extents.y >= 0.0,
            SpawnShapeKind::Cone => self.extents.x > 0.0 && self.extents.y > 0.0,
        }
    }

    /// Clamp values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.extents = self.extents.max(Vec3::ZERO);
        if matches!(self.kind, SpawnShapeKind::SphereShell) {
            self.secondary.x = self.secondary.x.clamp(0.0, 1.0);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::PI;

    #[test]
    fn test_point_aabb() {
        let shape = SpawnShape::point(Vec3::new(1.0, 2.0, 3.0));
        let (min, max) = shape.aabb();
        assert_eq!(min, max);
        assert_eq!(min, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_sphere_aabb() {
        let shape = SpawnShape::sphere(Vec3::ZERO, 5.0);
        let (min, max) = shape.aabb();
        assert_relative_eq!(min.x, -5.0, epsilon = 0.001);
        assert_relative_eq!(max.x, 5.0, epsilon = 0.001);
    }

    #[test]
    fn test_sphere_volume() {
        let shape = SpawnShape::sphere(Vec3::ZERO, 1.0);
        let expected = (4.0 / 3.0) * PI;
        assert_relative_eq!(shape.volume(), expected, epsilon = 0.001);
    }

    #[test]
    fn test_box_volume() {
        let shape = SpawnShape::box_volume(Vec3::ZERO, Vec3::ONE);
        assert_relative_eq!(shape.volume(), 8.0, epsilon = 0.001);
    }

    #[test]
    fn test_cylinder_volume() {
        let shape = SpawnShape::cylinder(Vec3::ZERO, 1.0, 1.0);
        let expected = PI * 2.0;
        assert_relative_eq!(shape.volume(), expected, epsilon = 0.001);
    }

    #[test]
    fn test_line_aabb() {
        let shape = SpawnShape::line(Vec3::ZERO, Vec3::new(10.0, 5.0, 3.0));
        let (min, max) = shape.aabb();
        assert_relative_eq!(min.x, 0.0, epsilon = 0.001);
        assert_relative_eq!(max.x, 10.0, epsilon = 0.001);
    }

    #[test]
    fn test_sphere_shell() {
        let shape = SpawnShape::sphere_shell(Vec3::ZERO, 10.0, 2.0);
        assert_eq!(shape.kind, SpawnShapeKind::SphereShell);
        assert!(shape.volume() > 0.0);
        assert!(shape.volume() < SpawnShape::sphere(Vec3::ZERO, 10.0).volume());
    }

    #[test]
    fn test_disc_normal() {
        let shape = SpawnShape::disc_with_normal(Vec3::ZERO, 5.0, Vec3::X);
        assert_relative_eq!(shape.secondary.length(), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_point_volume_zero() {
        let shape = SpawnShape::point(Vec3::ZERO);
        assert_relative_eq!(shape.volume(), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_line_volume_zero() {
        let shape = SpawnShape::line(Vec3::ZERO, Vec3::X);
        assert_relative_eq!(shape.volume(), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_all_shapes_valid() {
        let shapes = [
            SpawnShape::point(Vec3::ZERO),
            SpawnShape::sphere(Vec3::ZERO, 5.0),
            SpawnShape::sphere_shell(Vec3::ZERO, 5.0, 1.0),
            SpawnShape::box_volume(Vec3::ZERO, Vec3::ONE),
            SpawnShape::box_shell(Vec3::ZERO, Vec3::ONE),
            SpawnShape::cylinder(Vec3::ZERO, 2.0, 3.0),
            SpawnShape::cone(Vec3::ZERO, Vec3::Y, 0.5, 5.0),
            SpawnShape::disc(Vec3::ZERO, 3.0),
            SpawnShape::line(Vec3::ZERO, Vec3::X * 10.0),
        ];

        for shape in shapes {
            assert!(shape.is_valid(), "{:?} should be valid", shape.kind);
        }
    }

    #[test]
    fn test_clamped() {
        let shape = SpawnShape {
            kind: SpawnShapeKind::Sphere,
            center: Vec3::ZERO,
            extents: Vec3::splat(-5.0),
            secondary: Vec3::ZERO,
        }
        .clamped();

        assert!(shape.is_valid());
        assert!(shape.extents.x >= 0.0);
    }

    #[test]
    fn test_cone_aabb_symmetric() {
        let shape = SpawnShape::cone(Vec3::ZERO, Vec3::Y, 0.5, 10.0);
        let (min, max) = shape.aabb();
        assert_relative_eq!(min.x, -max.x, epsilon = 0.001);
    }
}

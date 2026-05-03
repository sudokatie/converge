//! Volume shape definitions.

use engine_core::math::{Aabb, Sphere};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Shape of a physics volume.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum VolumeShape {
    /// Axis-aligned bounding box.
    Aabb(Aabb),
    /// Spherical volume.
    Sphere(Sphere),
}

impl VolumeShape {
    /// Creates an AABB-shaped volume.
    #[must_use]
    pub const fn aabb(aabb: Aabb) -> Self {
        Self::Aabb(aabb)
    }

    /// Creates a sphere-shaped volume.
    #[must_use]
    pub const fn sphere(sphere: Sphere) -> Self {
        Self::Sphere(sphere)
    }

    /// Creates an AABB from center and half-extents.
    #[must_use]
    pub fn aabb_centered(center: Vec3, half_extents: Vec3) -> Self {
        Self::Aabb(Aabb::from_center_half_extents(center, half_extents))
    }

    /// Creates a sphere from center and radius.
    #[must_use]
    pub fn sphere_centered(center: Vec3, radius: f32) -> Self {
        Self::Sphere(Sphere::new(center, radius))
    }

    /// Returns whether a point is inside this volume.
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        match self {
            Self::Aabb(aabb) => aabb.contains_point(point),
            Self::Sphere(sphere) => sphere.contains_point(point),
        }
    }

    /// Returns the center of the volume.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        match self {
            Self::Aabb(aabb) => aabb.center(),
            Self::Sphere(sphere) => sphere.center,
        }
    }

    /// Returns the bounding AABB of this shape.
    #[must_use]
    pub fn bounding_aabb(&self) -> Aabb {
        match self {
            Self::Aabb(aabb) => *aabb,
            Self::Sphere(sphere) => {
                let r = Vec3::splat(sphere.radius);
                Aabb::new(sphere.center - r, sphere.center + r)
            }
        }
    }

    /// Returns whether this volume intersects another volume.
    #[must_use]
    pub fn intersects(&self, other: &VolumeShape) -> bool {
        match (self, other) {
            (Self::Aabb(a), Self::Aabb(b)) => a.intersects_aabb(b),
            (Self::Sphere(a), Self::Sphere(b)) => a.intersects_sphere(b),
            (Self::Aabb(aabb), Self::Sphere(sphere)) | (Self::Sphere(sphere), Self::Aabb(aabb)) => {
                aabb.intersects_sphere(sphere)
            }
        }
    }

    /// Returns the signed distance to the volume surface (negative inside).
    #[must_use]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        match self {
            Self::Aabb(aabb) => {
                let center = aabb.center();
                let half = aabb.half_extents();
                let q = (point - center).abs() - half;
                let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
                let inside = q.x.max(q.y).max(q.z).min(0.0);
                outside + inside
            }
            Self::Sphere(sphere) => point.distance(sphere.center) - sphere.radius,
        }
    }

    /// Returns the penetration depth if inside (positive), zero otherwise.
    #[must_use]
    pub fn penetration_depth(&self, point: Vec3) -> f32 {
        (-self.signed_distance(point)).max(0.0)
    }
}

impl Default for VolumeShape {
    fn default() -> Self {
        Self::Aabb(Aabb::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn aabb_contains_point() {
        let shape = VolumeShape::aabb_centered(Vec3::ZERO, Vec3::ONE);
        assert!(shape.contains_point(Vec3::ZERO));
        assert!(shape.contains_point(Vec3::splat(0.5)));
        assert!(!shape.contains_point(Vec3::splat(2.0)));
    }

    #[test]
    fn sphere_contains_point() {
        let shape = VolumeShape::sphere_centered(Vec3::ZERO, 2.0);
        assert!(shape.contains_point(Vec3::ZERO));
        assert!(shape.contains_point(Vec3::X));
        assert!(!shape.contains_point(Vec3::splat(2.0)));
    }

    #[test]
    fn bounding_aabb() {
        let sphere = VolumeShape::sphere_centered(Vec3::new(5.0, 0.0, 0.0), 1.0);
        let bounds = sphere.bounding_aabb();
        assert_relative_eq!(bounds.min.x, 4.0);
        assert_relative_eq!(bounds.max.x, 6.0);
    }

    #[test]
    fn intersects_aabb_aabb() {
        let a = VolumeShape::aabb_centered(Vec3::ZERO, Vec3::ONE);
        let b = VolumeShape::aabb_centered(Vec3::new(1.5, 0.0, 0.0), Vec3::ONE);
        assert!(a.intersects(&b));

        let c = VolumeShape::aabb_centered(Vec3::new(5.0, 0.0, 0.0), Vec3::ONE);
        assert!(!a.intersects(&c));
    }

    #[test]
    fn intersects_sphere_sphere() {
        let a = VolumeShape::sphere_centered(Vec3::ZERO, 1.0);
        let b = VolumeShape::sphere_centered(Vec3::new(1.5, 0.0, 0.0), 1.0);
        assert!(a.intersects(&b));

        let c = VolumeShape::sphere_centered(Vec3::new(5.0, 0.0, 0.0), 1.0);
        assert!(!a.intersects(&c));
    }

    #[test]
    fn signed_distance_sphere() {
        let shape = VolumeShape::sphere_centered(Vec3::ZERO, 2.0);
        assert_relative_eq!(shape.signed_distance(Vec3::ZERO), -2.0);
        assert_relative_eq!(shape.signed_distance(Vec3::new(2.0, 0.0, 0.0)), 0.0);
        assert_relative_eq!(shape.signed_distance(Vec3::new(4.0, 0.0, 0.0)), 2.0);
    }

    #[test]
    fn penetration_depth() {
        let shape = VolumeShape::sphere_centered(Vec3::ZERO, 2.0);
        assert_relative_eq!(shape.penetration_depth(Vec3::ZERO), 2.0);
        assert_relative_eq!(shape.penetration_depth(Vec3::new(1.0, 0.0, 0.0)), 1.0);
        assert_relative_eq!(shape.penetration_depth(Vec3::new(4.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn shape_serialization() {
        let shape = VolumeShape::sphere_centered(Vec3::new(1.0, 2.0, 3.0), 5.0);
        let json = serde_json::to_string(&shape).unwrap();
        let recovered: VolumeShape = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.center(), shape.center());
    }
}

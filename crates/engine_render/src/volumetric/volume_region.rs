//! Volume region definitions for spatial fog bounds.
//!
//! Regions define where volumetric effects are active in world space.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Shape of a volume region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VolumeShape {
    /// Axis-aligned box.
    Box = 0,
    /// Sphere.
    Sphere = 1,
    /// Vertical cylinder.
    Cylinder = 2,
    /// Infinite half-space (below a plane).
    HalfSpace = 3,
}

/// A region in world space where volumetric effects are active.
#[derive(Debug, Clone, Copy)]
pub struct VolumeRegion {
    /// Center position in world coordinates.
    pub center: Vec3,
    /// Half-extents or radius depending on shape.
    pub extents: Vec3,
    /// Shape of the volume.
    pub shape: VolumeShape,
    /// Falloff distance at boundary (soft edge).
    pub falloff: f32,
    /// Priority for overlapping regions (higher wins).
    pub priority: i32,
}

impl Default for VolumeRegion {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            extents: Vec3::splat(10.0),
            shape: VolumeShape::Box,
            falloff: 2.0,
            priority: 0,
        }
    }
}

impl VolumeRegion {
    /// Create a box region.
    #[must_use]
    pub fn new_box(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            extents: half_extents,
            shape: VolumeShape::Box,
            falloff: 2.0,
            priority: 0,
        }
    }

    /// Create a sphere region.
    #[must_use]
    pub fn new_sphere(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            extents: Vec3::splat(radius),
            shape: VolumeShape::Sphere,
            falloff: radius * 0.1,
            priority: 0,
        }
    }

    /// Create a cylinder region (vertical axis).
    #[must_use]
    pub fn new_cylinder(center: Vec3, radius: f32, half_height: f32) -> Self {
        Self {
            center,
            extents: Vec3::new(radius, half_height, radius),
            shape: VolumeShape::Cylinder,
            falloff: 2.0,
            priority: 0,
        }
    }

    /// Create a half-space region (everything below plane).
    #[must_use]
    pub fn new_half_space(plane_y: f32) -> Self {
        Self {
            center: Vec3::new(0.0, plane_y, 0.0),
            extents: Vec3::ZERO,
            shape: VolumeShape::HalfSpace,
            falloff: 5.0,
            priority: 0,
        }
    }

    /// Set falloff distance.
    #[must_use]
    pub fn with_falloff(mut self, falloff: f32) -> Self {
        self.falloff = falloff.max(0.0);
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if a point is inside the region (ignoring falloff).
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        let local = point - self.center;

        match self.shape {
            VolumeShape::Box => {
                local.x.abs() <= self.extents.x
                    && local.y.abs() <= self.extents.y
                    && local.z.abs() <= self.extents.z
            }
            VolumeShape::Sphere => local.length() <= self.extents.x,
            VolumeShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                horizontal_dist <= self.extents.x && local.y.abs() <= self.extents.y
            }
            VolumeShape::HalfSpace => point.y <= self.center.y,
        }
    }

    /// Calculate blend factor for a point (0.0 = outside, 1.0 = inside).
    #[must_use]
    pub fn blend_factor(&self, point: Vec3) -> f32 {
        let distance = self.signed_distance(point);

        if distance <= 0.0 {
            1.0
        } else if self.falloff <= 0.0 {
            0.0
        } else {
            (1.0 - distance / self.falloff).max(0.0)
        }
    }

    /// Signed distance to the region boundary (negative = inside).
    #[must_use]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        let local = point - self.center;

        match self.shape {
            VolumeShape::Box => {
                let q = local.abs() - self.extents;
                let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
                let inside = q.x.max(q.y).max(q.z).min(0.0);
                outside + inside
            }
            VolumeShape::Sphere => local.length() - self.extents.x,
            VolumeShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                let d_radial = horizontal_dist - self.extents.x;
                let d_vertical = local.y.abs() - self.extents.y;
                d_radial.max(d_vertical)
            }
            VolumeShape::HalfSpace => point.y - self.center.y,
        }
    }

    /// Compute axis-aligned bounding box.
    #[must_use]
    pub fn aabb(&self) -> (Vec3, Vec3) {
        match self.shape {
            VolumeShape::Box | VolumeShape::Sphere | VolumeShape::Cylinder => {
                let padded_extents = self.extents + Vec3::splat(self.falloff);
                (self.center - padded_extents, self.center + padded_extents)
            }
            VolumeShape::HalfSpace => (
                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec3::new(f32::INFINITY, self.center.y + self.falloff, f32::INFINITY),
            ),
        }
    }
}

/// GPU-friendly volume region uniform.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumeRegionUniform {
    /// Center (XYZ) + falloff (W).
    pub center_falloff: [f32; 4],
    /// Extents (XYZ) + shape (W as f32).
    pub extents_shape: [f32; 4],
}

impl From<VolumeRegion> for VolumeRegionUniform {
    fn from(region: VolumeRegion) -> Self {
        let shape = f32::from(region.shape as u8);

        Self {
            center_falloff: [
                region.center.x,
                region.center.y,
                region.center.z,
                region.falloff,
            ],
            extents_shape: [region.extents.x, region.extents.y, region.extents.z, shape],
        }
    }
}

impl Default for VolumeRegionUniform {
    fn default() -> Self {
        VolumeRegion::default().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_box_contains() {
        let region = VolumeRegion::new_box(Vec3::ZERO, Vec3::splat(5.0));

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 4.0, 4.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_sphere_contains() {
        let region = VolumeRegion::new_sphere(Vec3::ZERO, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(5.0, 5.0, 0.0)));
        assert!(!region.contains(Vec3::new(10.0, 10.0, 0.0)));
    }

    #[test]
    fn test_cylinder_contains() {
        let region = VolumeRegion::new_cylinder(Vec3::ZERO, 5.0, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 8.0, 0.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 15.0, 0.0)));
    }

    #[test]
    fn test_half_space_contains() {
        let region = VolumeRegion::new_half_space(64.0);

        assert!(region.contains(Vec3::new(100.0, 63.0, -50.0)));
        assert!(region.contains(Vec3::new(0.0, 64.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 65.0, 0.0)));
    }

    #[test]
    fn test_blend_factor_inside() {
        let region = VolumeRegion::new_sphere(Vec3::ZERO, 10.0).with_falloff(2.0);

        assert_relative_eq!(region.blend_factor(Vec3::ZERO), 1.0, epsilon = 0.001);
        assert_relative_eq!(
            region.blend_factor(Vec3::new(5.0, 0.0, 0.0)),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_factor_falloff() {
        let region = VolumeRegion::new_sphere(Vec3::ZERO, 10.0).with_falloff(2.0);

        let at_boundary = region.blend_factor(Vec3::new(10.0, 0.0, 0.0));
        assert_relative_eq!(at_boundary, 1.0, epsilon = 0.001);

        let in_falloff = region.blend_factor(Vec3::new(11.0, 0.0, 0.0));
        assert!(in_falloff > 0.0 && in_falloff < 1.0);

        let outside = region.blend_factor(Vec3::new(15.0, 0.0, 0.0));
        assert_relative_eq!(outside, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_signed_distance_box() {
        let region = VolumeRegion::new_box(Vec3::ZERO, Vec3::splat(5.0));

        assert!(
            region.signed_distance(Vec3::ZERO) < 0.0,
            "center should be inside"
        );
        assert_relative_eq!(
            region.signed_distance(Vec3::new(5.0, 0.0, 0.0)),
            0.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            region.signed_distance(Vec3::new(7.0, 0.0, 0.0)),
            2.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_aabb() {
        let region =
            VolumeRegion::new_box(Vec3::new(10.0, 20.0, 30.0), Vec3::splat(5.0)).with_falloff(1.0);

        let (min, max) = region.aabb();
        assert_relative_eq!(min.x, 4.0, epsilon = 0.001);
        assert_relative_eq!(max.x, 16.0, epsilon = 0.001);
    }

    #[test]
    fn test_uniform_conversion() {
        let region = VolumeRegion::new_sphere(Vec3::new(1.0, 2.0, 3.0), 5.0);
        let uniform: VolumeRegionUniform = region.into();

        assert_relative_eq!(uniform.center_falloff[0], 1.0, epsilon = 0.001);
        assert_relative_eq!(uniform.extents_shape[0], 5.0, epsilon = 0.001);
        assert_relative_eq!(
            uniform.extents_shape[3],
            f32::from(VolumeShape::Sphere as u8),
            epsilon = 0.001
        );
    }

    #[test]
    fn test_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<VolumeRegionUniform>() % 16,
            0,
            "uniform should be 16-byte aligned for GPU"
        );
    }

    #[test]
    fn test_priority_builder() {
        let region = VolumeRegion::new_box(Vec3::ZERO, Vec3::ONE)
            .with_priority(10)
            .with_falloff(3.0);

        assert_eq!(region.priority, 10);
        assert_relative_eq!(region.falloff, 3.0, epsilon = 0.001);
    }
}

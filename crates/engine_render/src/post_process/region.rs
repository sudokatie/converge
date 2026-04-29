//! World-space post-processing region definitions.
//!
//! Regions define where post-processing effects are active in world space,
//! enabling environment-specific visual treatments.

use crate::volumetric::VolumeShape;
use glam::Vec3;

/// Shape of a post-processing region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PostRegionShape {
    /// Axis-aligned box.
    Box = 0,
    /// Sphere.
    Sphere = 1,
    /// Vertical cylinder.
    Cylinder = 2,
    /// Infinite half-space (below a plane).
    HalfSpace = 3,
    /// Global (affects entire screen).
    Global = 4,
}

impl PostRegionShape {
    /// All region shapes.
    pub const ALL: [Self; 5] = [
        Self::Box,
        Self::Sphere,
        Self::Cylinder,
        Self::HalfSpace,
        Self::Global,
    ];
}

impl From<VolumeShape> for PostRegionShape {
    fn from(shape: VolumeShape) -> Self {
        match shape {
            VolumeShape::Box => Self::Box,
            VolumeShape::Sphere => Self::Sphere,
            VolumeShape::Cylinder => Self::Cylinder,
            VolumeShape::HalfSpace => Self::HalfSpace,
        }
    }
}

/// A region in world space where post-processing effects apply.
#[derive(Debug, Clone, Copy)]
pub struct PostRegion {
    /// Center position in world coordinates.
    pub center: Vec3,
    /// Half-extents, radius, or size depending on shape.
    pub extents: Vec3,
    /// Shape of the region.
    pub shape: PostRegionShape,
    /// Falloff distance at boundary (soft edge).
    pub falloff: f32,
    /// Priority for overlapping regions (higher wins).
    pub priority: i32,
    /// Environment identifier for grouping related regions.
    pub environment_id: u32,
}

impl Default for PostRegion {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            extents: Vec3::splat(10.0),
            shape: PostRegionShape::Sphere,
            falloff: 2.0,
            priority: 0,
            environment_id: 0,
        }
    }
}

impl PostRegion {
    /// Create a global region (affects entire screen).
    #[must_use]
    pub fn global() -> Self {
        Self {
            center: Vec3::ZERO,
            extents: Vec3::ZERO,
            shape: PostRegionShape::Global,
            falloff: 0.0,
            priority: i32::MIN,
            environment_id: 0,
        }
    }

    /// Create a box region.
    #[must_use]
    pub fn new_box(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            extents: half_extents,
            shape: PostRegionShape::Box,
            ..Default::default()
        }
    }

    /// Create a sphere region.
    #[must_use]
    pub fn new_sphere(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            extents: Vec3::splat(radius),
            shape: PostRegionShape::Sphere,
            falloff: radius * 0.15,
            ..Default::default()
        }
    }

    /// Create a cylinder region (vertical axis).
    #[must_use]
    pub fn new_cylinder(center: Vec3, radius: f32, half_height: f32) -> Self {
        Self {
            center,
            extents: Vec3::new(radius, half_height, radius),
            shape: PostRegionShape::Cylinder,
            ..Default::default()
        }
    }

    /// Create a half-space region (everything below plane).
    #[must_use]
    pub fn new_half_space(plane_y: f32) -> Self {
        Self {
            center: Vec3::new(0.0, plane_y, 0.0),
            extents: Vec3::ZERO,
            shape: PostRegionShape::HalfSpace,
            falloff: 5.0,
            ..Default::default()
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

    /// Set environment identifier.
    #[must_use]
    pub fn with_environment(mut self, env_id: u32) -> Self {
        self.environment_id = env_id;
        self
    }

    /// Check if a point is inside the region (ignoring falloff).
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        if self.shape == PostRegionShape::Global {
            return true;
        }

        let local = point - self.center;

        match self.shape {
            PostRegionShape::Box => {
                local.x.abs() <= self.extents.x
                    && local.y.abs() <= self.extents.y
                    && local.z.abs() <= self.extents.z
            }
            PostRegionShape::Sphere => local.length() <= self.extents.x,
            PostRegionShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                horizontal_dist <= self.extents.x && local.y.abs() <= self.extents.y
            }
            PostRegionShape::HalfSpace => point.y <= self.center.y,
            PostRegionShape::Global => true,
        }
    }

    /// Calculate blend factor for a point (0.0 = outside, 1.0 = inside).
    #[must_use]
    pub fn blend_factor(&self, point: Vec3) -> f32 {
        if self.shape == PostRegionShape::Global {
            return 1.0;
        }

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
        if self.shape == PostRegionShape::Global {
            return f32::NEG_INFINITY;
        }

        let local = point - self.center;

        match self.shape {
            PostRegionShape::Box => {
                let q = local.abs() - self.extents;
                let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
                let inside = q.x.max(q.y).max(q.z).min(0.0);
                outside + inside
            }
            PostRegionShape::Sphere => local.length() - self.extents.x,
            PostRegionShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                let d_radial = horizontal_dist - self.extents.x;
                let d_vertical = local.y.abs() - self.extents.y;
                d_radial.max(d_vertical)
            }
            PostRegionShape::HalfSpace => point.y - self.center.y,
            PostRegionShape::Global => f32::NEG_INFINITY,
        }
    }

    /// Compute axis-aligned bounding box (includes falloff).
    #[must_use]
    pub fn aabb(&self) -> (Vec3, Vec3) {
        if self.shape == PostRegionShape::Global {
            return (Vec3::splat(f32::NEG_INFINITY), Vec3::splat(f32::INFINITY));
        }

        let padding = Vec3::splat(self.falloff);

        match self.shape {
            PostRegionShape::Box | PostRegionShape::Sphere | PostRegionShape::Cylinder => {
                let padded = self.extents + padding;
                (self.center - padded, self.center + padded)
            }
            PostRegionShape::HalfSpace => (
                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec3::new(f32::INFINITY, self.center.y + self.falloff, f32::INFINITY),
            ),
            PostRegionShape::Global => (Vec3::splat(f32::NEG_INFINITY), Vec3::splat(f32::INFINITY)),
        }
    }

    /// Check if this is a global region.
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.shape == PostRegionShape::Global
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.falloff = self.falloff.max(0.0);
        self.extents = self.extents.max(Vec3::ZERO);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_global_region() {
        let region = PostRegion::global();
        assert!(region.is_global());
        assert!(region.contains(Vec3::new(1000.0, -500.0, 999.0)));
        assert_relative_eq!(region.blend_factor(Vec3::ZERO), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_sphere_contains() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);
        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(5.0, 5.0, 0.0)));
        assert!(!region.contains(Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn test_box_contains() {
        let region = PostRegion::new_box(Vec3::ZERO, Vec3::splat(5.0));
        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 4.0, 4.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_cylinder_contains() {
        let region = PostRegion::new_cylinder(Vec3::ZERO, 5.0, 10.0);
        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 8.0, 0.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 15.0, 0.0)));
    }

    #[test]
    fn test_half_space_contains() {
        let region = PostRegion::new_half_space(64.0);
        assert!(region.contains(Vec3::new(100.0, 63.0, -50.0)));
        assert!(region.contains(Vec3::new(0.0, 64.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 65.0, 0.0)));
    }

    #[test]
    fn test_blend_factor_inside() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);
        assert_relative_eq!(region.blend_factor(Vec3::ZERO), 1.0, epsilon = 0.001);
        assert_relative_eq!(
            region.blend_factor(Vec3::new(5.0, 0.0, 0.0)),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_factor_falloff() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0).with_falloff(5.0);

        let at_edge = region.blend_factor(Vec3::new(10.0, 0.0, 0.0));
        assert_relative_eq!(at_edge, 1.0, epsilon = 0.001);

        let in_falloff = region.blend_factor(Vec3::new(12.5, 0.0, 0.0));
        assert!(in_falloff > 0.0 && in_falloff < 1.0);

        let outside = region.blend_factor(Vec3::new(20.0, 0.0, 0.0));
        assert_relative_eq!(outside, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_signed_distance_sphere() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0);

        assert!(region.signed_distance(Vec3::ZERO) < 0.0);
        assert_relative_eq!(
            region.signed_distance(Vec3::new(10.0, 0.0, 0.0)),
            0.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            region.signed_distance(Vec3::new(15.0, 0.0, 0.0)),
            5.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_aabb_sphere() {
        let region = PostRegion::new_sphere(Vec3::new(10.0, 20.0, 30.0), 5.0).with_falloff(1.0);

        let (min, max) = region.aabb();
        assert_relative_eq!(min.x, 4.0, epsilon = 0.001);
        assert_relative_eq!(max.x, 16.0, epsilon = 0.001);
    }

    #[test]
    fn test_environment_id() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 10.0).with_environment(42);
        assert_eq!(region.environment_id, 42);
    }

    #[test]
    fn test_priority_builder() {
        let region = PostRegion::new_sphere(Vec3::ZERO, 5.0)
            .with_priority(10)
            .with_falloff(3.0);

        assert_eq!(region.priority, 10);
        assert_relative_eq!(region.falloff, 3.0, epsilon = 0.001);
    }

    #[test]
    fn test_clamped() {
        let region = PostRegion {
            falloff: -5.0,
            extents: Vec3::new(-1.0, 5.0, 10.0),
            ..Default::default()
        }
        .clamped();

        assert!(region.falloff >= 0.0);
        assert!(region.extents.x >= 0.0);
    }

    #[test]
    fn test_volume_shape_conversion() {
        let shape: PostRegionShape = VolumeShape::Sphere.into();
        assert_eq!(shape, PostRegionShape::Sphere);

        let shape: PostRegionShape = VolumeShape::Box.into();
        assert_eq!(shape, PostRegionShape::Box);
    }
}

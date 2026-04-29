//! World-space distortion region definitions.
//!
//! Regions define where distortion effects are active in world space.
//! Compatible with the volumetric module's [`VolumeRegion`] for interoperability.

use crate::volumetric::VolumeShape;
use glam::Vec3;

/// Shape of a distortion region (mirrors [`VolumeShape`] for interop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DistortionShape {
    /// Axis-aligned box.
    Box = 0,
    /// Sphere.
    Sphere = 1,
    /// Vertical cylinder.
    Cylinder = 2,
    /// Infinite half-space (below a plane).
    HalfSpace = 3,
    /// Cone emanating from a point.
    Cone = 4,
}

impl DistortionShape {
    /// All distortion shapes.
    pub const ALL: [Self; 5] = [
        Self::Box,
        Self::Sphere,
        Self::Cylinder,
        Self::HalfSpace,
        Self::Cone,
    ];
}

impl From<VolumeShape> for DistortionShape {
    fn from(shape: VolumeShape) -> Self {
        match shape {
            VolumeShape::Box => Self::Box,
            VolumeShape::Sphere => Self::Sphere,
            VolumeShape::Cylinder => Self::Cylinder,
            VolumeShape::HalfSpace => Self::HalfSpace,
        }
    }
}

/// A region in world space where distortion effects are active.
#[derive(Debug, Clone, Copy)]
pub struct DistortionRegion {
    /// Center position in world coordinates.
    pub center: Vec3,
    /// Half-extents, radius, or cone parameters depending on shape.
    pub extents: Vec3,
    /// Shape of the region.
    pub shape: DistortionShape,
    /// Falloff distance at boundary (soft edge).
    pub falloff: f32,
    /// Priority for overlapping regions (higher wins).
    pub priority: i32,
    /// Expansion rate for animated regions (units/second).
    pub expansion_rate: f32,
    /// Time when the region was created (for animated regions).
    pub creation_time: f32,
}

impl Default for DistortionRegion {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            extents: Vec3::splat(10.0),
            shape: DistortionShape::Sphere,
            falloff: 2.0,
            priority: 0,
            expansion_rate: 0.0,
            creation_time: 0.0,
        }
    }
}

impl DistortionRegion {
    /// Create a box region.
    #[must_use]
    pub fn new_box(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            extents: half_extents,
            shape: DistortionShape::Box,
            ..Default::default()
        }
    }

    /// Create a sphere region.
    #[must_use]
    pub fn new_sphere(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            extents: Vec3::splat(radius),
            shape: DistortionShape::Sphere,
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
            shape: DistortionShape::Cylinder,
            ..Default::default()
        }
    }

    /// Create a half-space region (everything below plane).
    #[must_use]
    pub fn new_half_space(plane_y: f32) -> Self {
        Self {
            center: Vec3::new(0.0, plane_y, 0.0),
            extents: Vec3::ZERO,
            shape: DistortionShape::HalfSpace,
            falloff: 5.0,
            ..Default::default()
        }
    }

    /// Create a cone region emanating from a point.
    #[must_use]
    pub fn new_cone(apex: Vec3, direction: Vec3, angle_radians: f32, length: f32) -> Self {
        let dir = direction.normalize_or_zero();
        Self {
            center: apex,
            extents: Vec3::new(angle_radians, length, dir.dot(Vec3::Y)),
            shape: DistortionShape::Cone,
            falloff: length * 0.1,
            ..Default::default()
        }
    }

    /// Create an expanding pressure wave region.
    #[must_use]
    pub fn new_expanding_sphere(center: Vec3, initial_radius: f32, expansion_rate: f32) -> Self {
        Self {
            center,
            extents: Vec3::splat(initial_radius),
            shape: DistortionShape::Sphere,
            falloff: initial_radius * 0.2,
            expansion_rate,
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

    /// Set creation time for animated regions.
    #[must_use]
    pub fn with_creation_time(mut self, time: f32) -> Self {
        self.creation_time = time;
        self
    }

    /// Get effective radius at a given time (for expanding regions).
    #[must_use]
    pub fn effective_radius_at_time(&self, current_time: f32) -> f32 {
        let elapsed = (current_time - self.creation_time).max(0.0);
        self.extents.x + elapsed * self.expansion_rate
    }

    /// Check if a point is inside the region (ignoring falloff).
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        self.contains_at_time(point, self.creation_time)
    }

    /// Check if a point is inside the region at a given time.
    #[must_use]
    pub fn contains_at_time(&self, point: Vec3, current_time: f32) -> bool {
        let local = point - self.center;
        let effective_radius = self.effective_radius_at_time(current_time);

        match self.shape {
            DistortionShape::Box => {
                let eff_extents = self.extents
                    + Vec3::splat(
                        self.expansion_rate * (current_time - self.creation_time).max(0.0),
                    );
                local.x.abs() <= eff_extents.x
                    && local.y.abs() <= eff_extents.y
                    && local.z.abs() <= eff_extents.z
            }
            DistortionShape::Sphere => local.length() <= effective_radius,
            DistortionShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                horizontal_dist <= effective_radius && local.y.abs() <= self.extents.y
            }
            DistortionShape::HalfSpace => point.y <= self.center.y,
            DistortionShape::Cone => {
                let angle = self.extents.x;
                let length = self.extents.y;
                let dir = Vec3::new(
                    0.0,
                    self.extents.z,
                    (1.0 - self.extents.z * self.extents.z).sqrt(),
                );
                let dist_along = local.dot(dir);
                if dist_along < 0.0 || dist_along > length {
                    return false;
                }
                let perpendicular = local - dir * dist_along;
                let max_radius = dist_along * angle.tan();
                perpendicular.length() <= max_radius
            }
        }
    }

    /// Calculate blend factor for a point (0.0 = outside, 1.0 = inside).
    #[must_use]
    pub fn blend_factor(&self, point: Vec3) -> f32 {
        self.blend_factor_at_time(point, self.creation_time)
    }

    /// Calculate blend factor at a given time.
    #[must_use]
    pub fn blend_factor_at_time(&self, point: Vec3, current_time: f32) -> f32 {
        let distance = self.signed_distance_at_time(point, current_time);

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
        self.signed_distance_at_time(point, self.creation_time)
    }

    /// Signed distance at a given time.
    #[must_use]
    pub fn signed_distance_at_time(&self, point: Vec3, current_time: f32) -> f32 {
        let local = point - self.center;
        let elapsed = (current_time - self.creation_time).max(0.0);
        let expansion = self.expansion_rate * elapsed;

        match self.shape {
            DistortionShape::Box => {
                let eff_extents = self.extents + Vec3::splat(expansion);
                let q = local.abs() - eff_extents;
                let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
                let inside = q.x.max(q.y).max(q.z).min(0.0);
                outside + inside
            }
            DistortionShape::Sphere => local.length() - (self.extents.x + expansion),
            DistortionShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                let d_radial = horizontal_dist - (self.extents.x + expansion);
                let d_vertical = local.y.abs() - self.extents.y;
                d_radial.max(d_vertical)
            }
            DistortionShape::HalfSpace => point.y - self.center.y,
            DistortionShape::Cone => {
                let length = self.extents.y;
                let dir = Vec3::new(
                    0.0,
                    self.extents.z,
                    (1.0 - self.extents.z * self.extents.z).sqrt(),
                );
                let dist_along = local.dot(dir);
                if dist_along < 0.0 {
                    local.length()
                } else if dist_along > length {
                    (local - dir * length).length()
                } else {
                    let perpendicular = local - dir * dist_along;
                    let max_radius = dist_along * self.extents.x.tan();
                    perpendicular.length() - max_radius
                }
            }
        }
    }

    /// Compute axis-aligned bounding box (includes falloff).
    #[must_use]
    pub fn aabb(&self) -> (Vec3, Vec3) {
        self.aabb_at_time(self.creation_time)
    }

    /// Compute AABB at a given time.
    #[must_use]
    pub fn aabb_at_time(&self, current_time: f32) -> (Vec3, Vec3) {
        let elapsed = (current_time - self.creation_time).max(0.0);
        let expansion = Vec3::splat(self.expansion_rate * elapsed + self.falloff);

        match self.shape {
            DistortionShape::Box | DistortionShape::Sphere | DistortionShape::Cylinder => {
                let padded = self.extents + expansion;
                (self.center - padded, self.center + padded)
            }
            DistortionShape::HalfSpace => (
                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec3::new(f32::INFINITY, self.center.y + self.falloff, f32::INFINITY),
            ),
            DistortionShape::Cone => {
                let length = self.extents.y + self.falloff;
                let max_radius = length * self.extents.x.tan() + self.falloff;
                (
                    self.center - Vec3::splat(max_radius.max(length)),
                    self.center + Vec3::splat(max_radius.max(length)),
                )
            }
        }
    }

    /// Check if this is an animated (expanding) region.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.expansion_rate.abs() > 0.0001
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.falloff = self.falloff.max(0.0);
        self.expansion_rate = self.expansion_rate.max(0.0);
        self.extents = self.extents.max(Vec3::ZERO);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sphere_contains() {
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(5.0, 5.0, 0.0)));
        assert!(!region.contains(Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn test_box_contains() {
        let region = DistortionRegion::new_box(Vec3::ZERO, Vec3::splat(5.0));

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 4.0, 4.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_cylinder_contains() {
        let region = DistortionRegion::new_cylinder(Vec3::ZERO, 5.0, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 8.0, 0.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 15.0, 0.0)));
    }

    #[test]
    fn test_half_space_contains() {
        let region = DistortionRegion::new_half_space(64.0);

        assert!(region.contains(Vec3::new(100.0, 63.0, -50.0)));
        assert!(region.contains(Vec3::new(0.0, 64.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 65.0, 0.0)));
    }

    #[test]
    fn test_expanding_sphere() {
        let region =
            DistortionRegion::new_expanding_sphere(Vec3::ZERO, 5.0, 10.0).with_creation_time(0.0);

        assert!(region.contains_at_time(Vec3::new(4.0, 0.0, 0.0), 0.0));
        assert!(!region.contains_at_time(Vec3::new(12.0, 0.0, 0.0), 0.0));

        assert!(region.contains_at_time(Vec3::new(12.0, 0.0, 0.0), 1.0));
        assert!(!region.contains_at_time(Vec3::new(20.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn test_blend_factor_inside() {
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);

        assert_relative_eq!(region.blend_factor(Vec3::ZERO), 1.0, epsilon = 0.001);
        assert_relative_eq!(
            region.blend_factor(Vec3::new(5.0, 0.0, 0.0)),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_factor_falloff() {
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0).with_falloff(5.0);

        let at_edge = region.blend_factor(Vec3::new(10.0, 0.0, 0.0));
        assert_relative_eq!(at_edge, 1.0, epsilon = 0.001);

        let in_falloff = region.blend_factor(Vec3::new(12.5, 0.0, 0.0));
        assert!(in_falloff > 0.0 && in_falloff < 1.0);

        let outside = region.blend_factor(Vec3::new(20.0, 0.0, 0.0));
        assert_relative_eq!(outside, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_signed_distance_sphere() {
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);

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
        let region =
            DistortionRegion::new_sphere(Vec3::new(10.0, 20.0, 30.0), 5.0).with_falloff(1.0);

        let (min, max) = region.aabb();
        assert_relative_eq!(min.x, 4.0, epsilon = 0.001);
        assert_relative_eq!(max.x, 16.0, epsilon = 0.001);
    }

    #[test]
    fn test_aabb_expanding() {
        let region = DistortionRegion::new_expanding_sphere(Vec3::ZERO, 5.0, 10.0)
            .with_falloff(1.0)
            .with_creation_time(0.0);

        let (_min0, max0) = region.aabb_at_time(0.0);
        let (_min1, max1) = region.aabb_at_time(1.0);

        assert!(max1.x > max0.x, "AABB should expand over time");
    }

    #[test]
    fn test_is_animated() {
        let static_region = DistortionRegion::new_sphere(Vec3::ZERO, 10.0);
        let animated_region = DistortionRegion::new_expanding_sphere(Vec3::ZERO, 5.0, 10.0);

        assert!(!static_region.is_animated());
        assert!(animated_region.is_animated());
    }

    #[test]
    fn test_volume_shape_conversion() {
        let shape: DistortionShape = VolumeShape::Sphere.into();
        assert_eq!(shape, DistortionShape::Sphere);

        let shape: DistortionShape = VolumeShape::Box.into();
        assert_eq!(shape, DistortionShape::Box);
    }

    #[test]
    fn test_priority_builder() {
        let region = DistortionRegion::new_sphere(Vec3::ZERO, 5.0)
            .with_priority(10)
            .with_falloff(3.0);

        assert_eq!(region.priority, 10);
        assert_relative_eq!(region.falloff, 3.0, epsilon = 0.001);
    }

    #[test]
    fn test_clamped() {
        let region = DistortionRegion {
            falloff: -5.0,
            expansion_rate: -1.0,
            extents: Vec3::new(-1.0, 5.0, 10.0),
            ..Default::default()
        }
        .clamped();

        assert!(region.falloff >= 0.0);
        assert!(region.expansion_rate >= 0.0);
        assert!(region.extents.x >= 0.0);
    }
}

//! World-space visibility region definitions.
//!
//! Regions define where visibility effects are active in world space.
//! Compatible with volumetric and distortion modules for interoperability.

use crate::volumetric::VolumeShape;
use glam::Vec3;

/// Shape of a visibility region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VisibilityShape {
    /// Axis-aligned box.
    Box = 0,
    /// Sphere.
    Sphere = 1,
    /// Vertical cylinder.
    Cylinder = 2,
    /// Infinite half-space (below a plane).
    HalfSpace = 3,
    /// Capsule (cylinder with hemispherical caps).
    Capsule = 4,
}

impl VisibilityShape {
    /// All visibility shapes.
    pub const ALL: [Self; 5] = [
        Self::Box,
        Self::Sphere,
        Self::Cylinder,
        Self::HalfSpace,
        Self::Capsule,
    ];
}

impl From<VolumeShape> for VisibilityShape {
    fn from(shape: VolumeShape) -> Self {
        match shape {
            VolumeShape::Box => Self::Box,
            VolumeShape::Sphere => Self::Sphere,
            VolumeShape::Cylinder => Self::Cylinder,
            VolumeShape::HalfSpace => Self::HalfSpace,
        }
    }
}

/// A region in world space where visibility effects are active.
#[derive(Debug, Clone, Copy)]
pub struct VisibilityRegion {
    /// Center position in world coordinates.
    pub center: Vec3,
    /// Half-extents, radius, or capsule parameters depending on shape.
    pub extents: Vec3,
    /// Shape of the region.
    pub shape: VisibilityShape,
    /// Falloff distance at boundary (soft edge).
    pub falloff: f32,
    /// Priority for overlapping regions (higher wins).
    pub priority: i32,
    /// Density gradient direction (for volumetric variation).
    pub gradient_direction: Vec3,
    /// Gradient strength (0 = uniform, 1 = full gradient).
    pub gradient_strength: f32,
}

impl Default for VisibilityRegion {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            extents: Vec3::splat(10.0),
            shape: VisibilityShape::Sphere,
            falloff: 2.0,
            priority: 0,
            gradient_direction: Vec3::Y,
            gradient_strength: 0.0,
        }
    }
}

impl VisibilityRegion {
    /// Create a box region.
    #[must_use]
    pub fn new_box(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            extents: half_extents,
            shape: VisibilityShape::Box,
            ..Default::default()
        }
    }

    /// Create a sphere region.
    #[must_use]
    pub fn new_sphere(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            extents: Vec3::splat(radius),
            shape: VisibilityShape::Sphere,
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
            shape: VisibilityShape::Cylinder,
            ..Default::default()
        }
    }

    /// Create a half-space region (everything below plane).
    #[must_use]
    pub fn new_half_space(plane_y: f32) -> Self {
        Self {
            center: Vec3::new(0.0, plane_y, 0.0),
            extents: Vec3::ZERO,
            shape: VisibilityShape::HalfSpace,
            falloff: 5.0,
            gradient_direction: Vec3::NEG_Y,
            ..Default::default()
        }
    }

    /// Create a capsule region (vertical axis).
    #[must_use]
    pub fn new_capsule(center: Vec3, radius: f32, half_height: f32) -> Self {
        Self {
            center,
            extents: Vec3::new(radius, half_height, radius),
            shape: VisibilityShape::Capsule,
            falloff: radius * 0.2,
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

    /// Set gradient direction (will be normalized).
    #[must_use]
    pub fn with_gradient_direction(mut self, direction: Vec3) -> Self {
        self.gradient_direction = if direction.length_squared() > 0.0001 {
            direction.normalize()
        } else {
            Vec3::Y
        };
        self
    }

    /// Set gradient strength.
    #[must_use]
    pub fn with_gradient_strength(mut self, strength: f32) -> Self {
        self.gradient_strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Check if a point is inside the region (ignoring falloff).
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        let local = point - self.center;

        match self.shape {
            VisibilityShape::Box => {
                local.x.abs() <= self.extents.x
                    && local.y.abs() <= self.extents.y
                    && local.z.abs() <= self.extents.z
            }
            VisibilityShape::Sphere => local.length() <= self.extents.x,
            VisibilityShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                horizontal_dist <= self.extents.x && local.y.abs() <= self.extents.y
            }
            VisibilityShape::HalfSpace => point.y <= self.center.y,
            VisibilityShape::Capsule => {
                let radius = self.extents.x;
                let half_height = self.extents.y;
                if local.y.abs() <= half_height {
                    Vec3::new(local.x, 0.0, local.z).length() <= radius
                } else {
                    let cap_center_y = if local.y > 0.0 {
                        half_height
                    } else {
                        -half_height
                    };
                    let to_cap = local - Vec3::new(0.0, cap_center_y, 0.0);
                    to_cap.length() <= radius
                }
            }
        }
    }

    /// Calculate blend factor for a point (0.0 = outside, 1.0 = inside).
    #[must_use]
    pub fn blend_factor(&self, point: Vec3) -> f32 {
        let distance = self.signed_distance(point);
        let base_factor = if distance <= 0.0 {
            1.0
        } else if self.falloff <= 0.0 {
            0.0
        } else {
            (1.0 - distance / self.falloff).max(0.0)
        };

        if self.gradient_strength > 0.0 && base_factor > 0.0 {
            let local = point - self.center;
            let gradient_factor = local.dot(self.gradient_direction);
            let max_extent = self.extents.max_element().max(1.0);
            let normalized_gradient = (gradient_factor / max_extent + 1.0) * 0.5;
            let gradient_mod = 1.0 - self.gradient_strength * (1.0 - normalized_gradient);
            base_factor * gradient_mod.clamp(0.0, 1.0)
        } else {
            base_factor
        }
    }

    /// Signed distance to the region boundary (negative = inside).
    #[must_use]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        let local = point - self.center;

        match self.shape {
            VisibilityShape::Box => {
                let q = local.abs() - self.extents;
                let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
                let inside = q.x.max(q.y).max(q.z).min(0.0);
                outside + inside
            }
            VisibilityShape::Sphere => local.length() - self.extents.x,
            VisibilityShape::Cylinder => {
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                let d_radial = horizontal_dist - self.extents.x;
                let d_vertical = local.y.abs() - self.extents.y;
                d_radial.max(d_vertical)
            }
            VisibilityShape::HalfSpace => point.y - self.center.y,
            VisibilityShape::Capsule => {
                let radius = self.extents.x;
                let half_height = self.extents.y;
                let clamped_y = local.y.clamp(-half_height, half_height);
                let closest_axis_point = Vec3::new(0.0, clamped_y, 0.0);
                (local - closest_axis_point).length() - radius
            }
        }
    }

    /// Compute axis-aligned bounding box (includes falloff).
    #[must_use]
    pub fn aabb(&self) -> (Vec3, Vec3) {
        let padding = Vec3::splat(self.falloff);

        match self.shape {
            VisibilityShape::Box | VisibilityShape::Sphere | VisibilityShape::Cylinder => {
                let padded = self.extents + padding;
                (self.center - padded, self.center + padded)
            }
            VisibilityShape::HalfSpace => (
                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec3::new(f32::INFINITY, self.center.y + self.falloff, f32::INFINITY),
            ),
            VisibilityShape::Capsule => {
                let radius = self.extents.x;
                let half_height = self.extents.y;
                let max_extent = (radius + half_height).max(radius);
                let padded = Vec3::splat(max_extent) + padding;
                (self.center - padded, self.center + padded)
            }
        }
    }

    /// Check if this region has a density gradient.
    #[must_use]
    pub fn has_gradient(&self) -> bool {
        self.gradient_strength > 0.0001
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.falloff = self.falloff.max(0.0);
        self.extents = self.extents.max(Vec3::ZERO);
        self.gradient_strength = self.gradient_strength.clamp(0.0, 1.0);
        if self.gradient_direction.length_squared() > 0.0001 {
            self.gradient_direction = self.gradient_direction.normalize();
        } else {
            self.gradient_direction = Vec3::Y;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sphere_contains() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(5.0, 5.0, 0.0)));
        assert!(!region.contains(Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn test_box_contains() {
        let region = VisibilityRegion::new_box(Vec3::ZERO, Vec3::splat(5.0));

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 4.0, 4.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_cylinder_contains() {
        let region = VisibilityRegion::new_cylinder(Vec3::ZERO, 5.0, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(4.0, 8.0, 0.0)));
        assert!(!region.contains(Vec3::new(6.0, 0.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 15.0, 0.0)));
    }

    #[test]
    fn test_half_space_contains() {
        let region = VisibilityRegion::new_half_space(64.0);

        assert!(region.contains(Vec3::new(100.0, 63.0, -50.0)));
        assert!(region.contains(Vec3::new(0.0, 64.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 65.0, 0.0)));
    }

    #[test]
    fn test_capsule_contains() {
        let region = VisibilityRegion::new_capsule(Vec3::ZERO, 5.0, 10.0);

        assert!(region.contains(Vec3::ZERO));
        assert!(region.contains(Vec3::new(0.0, 10.0, 0.0)));
        assert!(region.contains(Vec3::new(0.0, 14.0, 0.0)));
        assert!(!region.contains(Vec3::new(0.0, 20.0, 0.0)));
        assert!(!region.contains(Vec3::new(8.0, 0.0, 0.0)));
    }

    #[test]
    fn test_blend_factor_inside() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);

        assert_relative_eq!(region.blend_factor(Vec3::ZERO), 1.0, epsilon = 0.001);
        assert_relative_eq!(
            region.blend_factor(Vec3::new(5.0, 0.0, 0.0)),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_blend_factor_falloff() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0).with_falloff(5.0);

        let at_edge = region.blend_factor(Vec3::new(10.0, 0.0, 0.0));
        assert_relative_eq!(at_edge, 1.0, epsilon = 0.001);

        let in_falloff = region.blend_factor(Vec3::new(12.5, 0.0, 0.0));
        assert!(in_falloff > 0.0 && in_falloff < 1.0);

        let outside = region.blend_factor(Vec3::new(20.0, 0.0, 0.0));
        assert_relative_eq!(outside, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_blend_factor_gradient() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0)
            .with_gradient_direction(Vec3::Y)
            .with_gradient_strength(1.0);

        let at_top = region.blend_factor(Vec3::new(0.0, 5.0, 0.0));
        let at_bottom = region.blend_factor(Vec3::new(0.0, -5.0, 0.0));

        assert!(at_top > at_bottom, "gradient should make top denser");
    }

    #[test]
    fn test_signed_distance_sphere() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);

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
    fn test_signed_distance_capsule() {
        let region = VisibilityRegion::new_capsule(Vec3::ZERO, 5.0, 10.0);

        assert!(region.signed_distance(Vec3::ZERO) < 0.0);
        assert_relative_eq!(
            region.signed_distance(Vec3::new(5.0, 0.0, 0.0)),
            0.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            region.signed_distance(Vec3::new(0.0, 15.0, 0.0)),
            0.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_aabb_sphere() {
        let region =
            VisibilityRegion::new_sphere(Vec3::new(10.0, 20.0, 30.0), 5.0).with_falloff(1.0);

        let (min, max) = region.aabb();
        assert_relative_eq!(min.x, 4.0, epsilon = 0.001);
        assert_relative_eq!(max.x, 16.0, epsilon = 0.001);
    }

    #[test]
    fn test_has_gradient() {
        let no_gradient = VisibilityRegion::new_sphere(Vec3::ZERO, 10.0);
        let with_gradient = no_gradient.with_gradient_strength(0.5);

        assert!(!no_gradient.has_gradient());
        assert!(with_gradient.has_gradient());
    }

    #[test]
    fn test_volume_shape_conversion() {
        let shape: VisibilityShape = VolumeShape::Sphere.into();
        assert_eq!(shape, VisibilityShape::Sphere);

        let shape: VisibilityShape = VolumeShape::Box.into();
        assert_eq!(shape, VisibilityShape::Box);
    }

    #[test]
    fn test_priority_builder() {
        let region = VisibilityRegion::new_sphere(Vec3::ZERO, 5.0)
            .with_priority(10)
            .with_falloff(3.0);

        assert_eq!(region.priority, 10);
        assert_relative_eq!(region.falloff, 3.0, epsilon = 0.001);
    }

    #[test]
    fn test_clamped() {
        let region = VisibilityRegion {
            falloff: -5.0,
            gradient_strength: 2.0,
            extents: Vec3::new(-1.0, 5.0, 10.0),
            gradient_direction: Vec3::ZERO,
            ..Default::default()
        }
        .clamped();

        assert!(region.falloff >= 0.0);
        assert!(region.gradient_strength <= 1.0);
        assert!(region.extents.x >= 0.0);
        assert_relative_eq!(region.gradient_direction.length(), 1.0, epsilon = 0.001);
    }
}

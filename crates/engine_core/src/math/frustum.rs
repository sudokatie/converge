//! View frustum for culling.

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

use super::{Aabb, Plane, Sphere};

/// Result of a containment test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Containment {
    /// Completely outside the frustum.
    Outside,
    /// Partially inside (intersecting the frustum boundary).
    Intersecting,
    /// Completely inside the frustum.
    Inside,
}

/// A view frustum defined by 6 planes.
///
/// Used for culling objects outside the camera's view.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Frustum {
    /// Left plane.
    pub left: Plane,
    /// Right plane.
    pub right: Plane,
    /// Bottom plane.
    pub bottom: Plane,
    /// Top plane.
    pub top: Plane,
    /// Near plane.
    pub near: Plane,
    /// Far plane.
    pub far: Plane,
}

impl Frustum {
    /// Extract frustum planes from a view-projection matrix.
    ///
    /// The matrix should be projection * view (clip space).
    #[must_use]
    pub fn from_view_projection(vp: Mat4) -> Self {
        let rows = [
            vp.row(0),
            vp.row(1),
            vp.row(2),
            vp.row(3),
        ];

        let extract_plane = |row_a: glam::Vec4, row_b: glam::Vec4, sign: f32| {
            let combined = row_a + row_b * sign;
            let normal = Vec3::new(combined.x, combined.y, combined.z);
            let length = normal.length();
            Plane {
                normal: normal / length,
                distance: -combined.w / length,
            }
        };

        Self {
            left: extract_plane(rows[3], rows[0], 1.0),
            right: extract_plane(rows[3], rows[0], -1.0),
            bottom: extract_plane(rows[3], rows[1], 1.0),
            top: extract_plane(rows[3], rows[1], -1.0),
            near: extract_plane(rows[3], rows[2], 1.0),
            far: extract_plane(rows[3], rows[2], -1.0),
        }
    }

    /// Get all 6 planes as an array.
    #[must_use]
    pub fn planes(&self) -> [&Plane; 6] {
        [
            &self.left,
            &self.right,
            &self.bottom,
            &self.top,
            &self.near,
            &self.far,
        ]
    }

    /// Test if a point is inside the frustum.
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in self.planes() {
            if plane.signed_distance(point) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if a sphere intersects the frustum.
    #[must_use]
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        for plane in self.planes() {
            if plane.signed_distance(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }

    /// Test if an AABB intersects the frustum.
    #[must_use]
    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        for plane in self.planes() {
            // Find the corner most aligned with the plane normal
            let p = Vec3::new(
                if plane.normal.x >= 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if plane.normal.y >= 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if plane.normal.z >= 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );

            if plane.signed_distance(p) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test sphere containment with full classification.
    ///
    /// Returns whether the sphere is completely outside, intersecting,
    /// or completely inside the frustum.
    #[must_use]
    pub fn contains_sphere(&self, sphere: &Sphere) -> Containment {
        let mut all_inside = true;

        for plane in self.planes() {
            let distance = plane.signed_distance(sphere.center);

            if distance < -sphere.radius {
                // Sphere is completely outside this plane
                return Containment::Outside;
            }

            if distance < sphere.radius {
                // Sphere intersects this plane
                all_inside = false;
            }
        }

        if all_inside {
            Containment::Inside
        } else {
            Containment::Intersecting
        }
    }

    /// Test AABB containment with full classification.
    ///
    /// Returns whether the AABB is completely outside, intersecting,
    /// or completely inside the frustum.
    #[must_use]
    pub fn contains_aabb(&self, aabb: &Aabb) -> Containment {
        let mut all_inside = true;

        for plane in self.planes() {
            // Positive vertex (most aligned with plane normal)
            let p_vertex = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );

            // Negative vertex (least aligned with plane normal)
            let n_vertex = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.min.x } else { aabb.max.x },
                if plane.normal.y >= 0.0 { aabb.min.y } else { aabb.max.y },
                if plane.normal.z >= 0.0 { aabb.min.z } else { aabb.max.z },
            );

            if plane.signed_distance(p_vertex) < 0.0 {
                // Positive vertex is outside, entire AABB is outside
                return Containment::Outside;
            }

            if plane.signed_distance(n_vertex) < 0.0 {
                // Negative vertex is outside, AABB straddles the plane
                all_inside = false;
            }
        }

        if all_inside {
            Containment::Inside
        } else {
            Containment::Intersecting
        }
    }
}

impl Default for Frustum {
    fn default() -> Self {
        // Default to a simple orthographic-like frustum
        Self {
            left: Plane::new(Vec3::X, -1.0),
            right: Plane::new(Vec3::NEG_X, -1.0),
            bottom: Plane::new(Vec3::Y, -1.0),
            top: Plane::new(Vec3::NEG_Y, -1.0),
            near: Plane::new(Vec3::NEG_Z, -0.1),
            far: Plane::new(Vec3::Z, -100.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    fn test_frustum() -> Frustum {
        let projection = Mat4::perspective_rh(FRAC_PI_4, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        Frustum::from_view_projection(projection * view)
    }

    #[test]
    fn test_contains_point_inside() {
        let frustum = test_frustum();
        // Point in front of camera, within view
        assert!(frustum.contains_point(Vec3::new(0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_contains_point_behind() {
        let frustum = test_frustum();
        // Point behind camera
        assert!(!frustum.contains_point(Vec3::new(0.0, 0.0, 10.0)));
    }

    #[test]
    fn test_contains_point_outside() {
        let frustum = test_frustum();
        // Point far to the side
        assert!(!frustum.contains_point(Vec3::new(100.0, 0.0, 0.0)));
    }

    #[test]
    fn test_intersects_sphere_inside() {
        let frustum = test_frustum();
        let sphere = Sphere::new(Vec3::ZERO, 1.0);
        assert!(frustum.intersects_sphere(&sphere));
    }

    #[test]
    fn test_intersects_sphere_partial() {
        let frustum = test_frustum();
        // Large sphere that partially overlaps
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, 10.0), 20.0);
        assert!(frustum.intersects_sphere(&sphere));
    }

    #[test]
    fn test_intersects_sphere_outside() {
        let frustum = test_frustum();
        let sphere = Sphere::new(Vec3::new(100.0, 0.0, 0.0), 1.0);
        assert!(!frustum.intersects_sphere(&sphere));
    }

    #[test]
    fn test_intersects_aabb_inside() {
        let frustum = test_frustum();
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        assert!(frustum.intersects_aabb(&aabb));
    }

    #[test]
    fn test_intersects_aabb_outside() {
        let frustum = test_frustum();
        let aabb = Aabb::new(Vec3::splat(100.0), Vec3::splat(101.0));
        assert!(!frustum.intersects_aabb(&aabb));
    }

    #[test]
    fn test_contains_sphere_inside() {
        let frustum = test_frustum();
        // Small sphere well within frustum
        let sphere = Sphere::new(Vec3::ZERO, 0.1);
        assert_eq!(frustum.contains_sphere(&sphere), Containment::Inside);
    }

    #[test]
    fn test_contains_sphere_intersecting() {
        let frustum = test_frustum();
        // Large sphere that straddles frustum boundary
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, 10.0), 20.0);
        assert_eq!(frustum.contains_sphere(&sphere), Containment::Intersecting);
    }

    #[test]
    fn test_contains_sphere_outside() {
        let frustum = test_frustum();
        let sphere = Sphere::new(Vec3::new(100.0, 0.0, 0.0), 1.0);
        assert_eq!(frustum.contains_sphere(&sphere), Containment::Outside);
    }

    #[test]
    fn test_contains_aabb_inside() {
        let frustum = test_frustum();
        // Small AABB well within frustum
        let aabb = Aabb::new(Vec3::splat(-0.1), Vec3::splat(0.1));
        assert_eq!(frustum.contains_aabb(&aabb), Containment::Inside);
    }

    #[test]
    fn test_contains_aabb_intersecting() {
        let frustum = test_frustum();
        // Large AABB that straddles frustum boundary
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -50.0), Vec3::new(1.0, 1.0, 50.0));
        assert_eq!(frustum.contains_aabb(&aabb), Containment::Intersecting);
    }

    #[test]
    fn test_contains_aabb_outside() {
        let frustum = test_frustum();
        let aabb = Aabb::new(Vec3::splat(100.0), Vec3::splat(101.0));
        assert_eq!(frustum.contains_aabb(&aabb), Containment::Outside);
    }
}

//! Frustum primitives for portal-aware culling.

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    #[must_use]
    pub fn new(normal: Vec3, distance: f32) -> Self {
        let len = normal.length();
        Self {
            normal: normal / len,
            distance: distance / len,
        }
    }

    #[must_use]
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self {
            normal: n,
            distance: -n.dot(point),
        }
    }

    #[must_use]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }

    #[must_use]
    pub fn is_in_front(&self, point: Vec3) -> bool {
        self.signed_distance(point) > 0.0
    }

    #[must_use]
    pub fn transform(&self, matrix: Mat4) -> Self {
        let point_on_plane = self.normal * -self.distance;
        let new_point = matrix.transform_point3(point_on_plane);
        let new_normal = matrix
            .inverse()
            .transpose()
            .transform_vector3(self.normal)
            .normalize();
        Self::from_point_normal(new_point, new_normal)
    }
}

impl Default for Plane {
    fn default() -> Self {
        Self {
            normal: Vec3::Y,
            distance: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frustum {
    pub planes: [Plane; 6],
    pub origin: Vec3,
    pub forward: Vec3,
}

impl Frustum {
    #[must_use]
    pub fn from_view_projection(view_projection: Mat4, origin: Vec3, forward: Vec3) -> Self {
        let m = view_projection;

        let left = Plane::new(
            Vec3::new(
                m.x_axis.w + m.x_axis.x,
                m.y_axis.w + m.y_axis.x,
                m.z_axis.w + m.z_axis.x,
            ),
            m.w_axis.w + m.w_axis.x,
        );
        let right = Plane::new(
            Vec3::new(
                m.x_axis.w - m.x_axis.x,
                m.y_axis.w - m.y_axis.x,
                m.z_axis.w - m.z_axis.x,
            ),
            m.w_axis.w - m.w_axis.x,
        );
        let bottom = Plane::new(
            Vec3::new(
                m.x_axis.w + m.x_axis.y,
                m.y_axis.w + m.y_axis.y,
                m.z_axis.w + m.z_axis.y,
            ),
            m.w_axis.w + m.w_axis.y,
        );
        let top = Plane::new(
            Vec3::new(
                m.x_axis.w - m.x_axis.y,
                m.y_axis.w - m.y_axis.y,
                m.z_axis.w - m.z_axis.y,
            ),
            m.w_axis.w - m.w_axis.y,
        );
        let near = Plane::new(
            Vec3::new(
                m.x_axis.w + m.x_axis.z,
                m.y_axis.w + m.y_axis.z,
                m.z_axis.w + m.z_axis.z,
            ),
            m.w_axis.w + m.w_axis.z,
        );
        let far = Plane::new(
            Vec3::new(
                m.x_axis.w - m.x_axis.z,
                m.y_axis.w - m.y_axis.z,
                m.z_axis.w - m.z_axis.z,
            ),
            m.w_axis.w - m.w_axis.z,
        );

        Self {
            planes: [left, right, bottom, top, near, far],
            origin,
            forward,
        }
    }

    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.planes.iter().all(|p| p.signed_distance(point) >= 0.0)
    }

    #[must_use]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        self.planes
            .iter()
            .all(|p| p.signed_distance(center) >= -radius)
    }

    #[must_use]
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.normal.x >= 0.0 { max.x } else { min.x },
                if plane.normal.y >= 0.0 { max.y } else { min.y },
                if plane.normal.z >= 0.0 { max.z } else { min.z },
            );
            if plane.signed_distance(p) < 0.0 {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn transform(&self, matrix: Mat4) -> Self {
        Self {
            planes: [
                self.planes[0].transform(matrix),
                self.planes[1].transform(matrix),
                self.planes[2].transform(matrix),
                self.planes[3].transform(matrix),
                self.planes[4].transform(matrix),
                self.planes[5].transform(matrix),
            ],
            origin: matrix.transform_point3(self.origin),
            forward: matrix.transform_vector3(self.forward).normalize(),
        }
    }

    #[must_use]
    pub fn clip_to_portal(&self, portal_corners: &[Vec3; 4], portal_normal: Vec3) -> Self {
        let mut planes = self.planes;

        let edge_normals = [
            (portal_corners[1] - portal_corners[0])
                .cross(self.origin - portal_corners[0])
                .normalize(),
            (portal_corners[2] - portal_corners[1])
                .cross(self.origin - portal_corners[1])
                .normalize(),
            (portal_corners[3] - portal_corners[2])
                .cross(self.origin - portal_corners[2])
                .normalize(),
            (portal_corners[0] - portal_corners[3])
                .cross(self.origin - portal_corners[3])
                .normalize(),
        ];

        for (i, (&corner, &edge_normal)) in
            portal_corners.iter().zip(edge_normals.iter()).enumerate()
        {
            if i < 4 && edge_normal.length_squared() > 0.0 {
                planes[i] = Plane::from_point_normal(corner, edge_normal);
            }
        }

        planes[4] = Plane::from_point_normal(portal_corners[0], portal_normal);

        Self {
            planes,
            origin: self.origin,
            forward: -portal_normal,
        }
    }
}

impl Default for Frustum {
    fn default() -> Self {
        Self {
            planes: [Plane::default(); 6],
            origin: Vec3::ZERO,
            forward: -Vec3::Z,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CullResult {
    Inside,
    Outside,
    Intersecting,
}

impl CullResult {
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        !matches!(self, Self::Outside)
    }
}

pub fn cull_sphere(frustum: &Frustum, center: Vec3, radius: f32) -> CullResult {
    let mut all_inside = true;

    for plane in &frustum.planes {
        let dist = plane.signed_distance(center);
        if dist < -radius {
            return CullResult::Outside;
        }
        if dist < radius {
            all_inside = false;
        }
    }

    if all_inside {
        CullResult::Inside
    } else {
        CullResult::Intersecting
    }
}

pub fn cull_aabb(frustum: &Frustum, min: Vec3, max: Vec3) -> CullResult {
    let mut all_inside = true;

    for plane in &frustum.planes {
        let p_vertex = Vec3::new(
            if plane.normal.x >= 0.0 { max.x } else { min.x },
            if plane.normal.y >= 0.0 { max.y } else { min.y },
            if plane.normal.z >= 0.0 { max.z } else { min.z },
        );
        let n_vertex = Vec3::new(
            if plane.normal.x >= 0.0 { min.x } else { max.x },
            if plane.normal.y >= 0.0 { min.y } else { max.y },
            if plane.normal.z >= 0.0 { min.z } else { max.z },
        );

        if plane.signed_distance(p_vertex) < 0.0 {
            return CullResult::Outside;
        }
        if plane.signed_distance(n_vertex) < 0.0 {
            all_inside = false;
        }
    }

    if all_inside {
        CullResult::Inside
    } else {
        CullResult::Intersecting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn identity_frustum() -> Frustum {
        let view = Mat4::look_at_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        Frustum::from_view_projection(proj * view, Vec3::ZERO, -Vec3::Z)
    }

    #[test]
    fn plane_signed_distance() {
        let plane = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        assert_relative_eq!(
            plane.signed_distance(Vec3::new(0.0, 5.0, 0.0)),
            5.0,
            epsilon = 1e-5
        );
        assert_relative_eq!(
            plane.signed_distance(Vec3::new(0.0, -3.0, 0.0)),
            -3.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn frustum_contains_point_inside() {
        let frustum = identity_frustum();
        assert!(frustum.contains_point(Vec3::new(0.0, 0.0, -10.0)));
    }

    #[test]
    fn frustum_excludes_point_behind() {
        let frustum = identity_frustum();
        assert!(!frustum.contains_point(Vec3::new(0.0, 0.0, 10.0)));
    }

    #[test]
    fn sphere_culling() {
        let frustum = identity_frustum();
        let inside = cull_sphere(&frustum, Vec3::new(0.0, 0.0, -10.0), 1.0);
        let outside = cull_sphere(&frustum, Vec3::new(0.0, 0.0, 200.0), 1.0);

        assert!(inside.is_visible());
        assert!(!outside.is_visible());
    }

    #[test]
    fn aabb_culling() {
        let frustum = identity_frustum();
        let inside = cull_aabb(
            &frustum,
            Vec3::new(-1.0, -1.0, -11.0),
            Vec3::new(1.0, 1.0, -9.0),
        );
        let outside = cull_aabb(
            &frustum,
            Vec3::new(-1.0, -1.0, 200.0),
            Vec3::new(1.0, 1.0, 202.0),
        );

        assert!(inside.is_visible());
        assert!(!outside.is_visible());
    }

    #[test]
    fn cull_result_is_visible() {
        assert!(CullResult::Inside.is_visible());
        assert!(CullResult::Intersecting.is_visible());
        assert!(!CullResult::Outside.is_visible());
    }

    #[test]
    fn plane_transform() {
        let plane = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        let translation = Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0));
        let transformed = plane.transform(translation);

        assert_relative_eq!(transformed.normal.y, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn serde_roundtrip() {
        let frustum = identity_frustum();
        let json = serde_json::to_string(&frustum).unwrap();
        let recovered: Frustum = serde_json::from_str(&json).unwrap();

        assert_relative_eq!(recovered.origin.x, frustum.origin.x, epsilon = 1e-5);
    }
}

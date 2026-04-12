//! Intersection tests between primitives.

use glam::Vec3;

use super::{Aabb, Plane, Ray, Sphere};

/// Result of an intersection test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intersection {
    /// Distance along ray to hit point (t parameter).
    pub t: f32,
    /// Hit point in world space.
    pub point: Vec3,
    /// Surface normal at hit point.
    pub normal: Vec3,
}

/// Test ray intersection with AABB.
///
/// Returns the intersection if hit within ray's t_max.
#[must_use]
pub fn ray_aabb(ray: &Ray, aabb: &Aabb) -> Option<Intersection> {
    let inv_dir = Vec3::new(
        1.0 / ray.direction.x,
        1.0 / ray.direction.y,
        1.0 / ray.direction.z,
    );

    let t1 = (aabb.min.x - ray.origin.x) * inv_dir.x;
    let t2 = (aabb.max.x - ray.origin.x) * inv_dir.x;
    let t3 = (aabb.min.y - ray.origin.y) * inv_dir.y;
    let t4 = (aabb.max.y - ray.origin.y) * inv_dir.y;
    let t5 = (aabb.min.z - ray.origin.z) * inv_dir.z;
    let t6 = (aabb.max.z - ray.origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax || tmin > ray.t_max {
        return None;
    }

    let t = if tmin < 0.0 { tmax } else { tmin };
    if t > ray.t_max {
        return None;
    }

    let point = ray.at(t);

    // Determine which face was hit for normal
    let center = aabb.center();
    let local = point - center;
    let half = aabb.half_extents();
    let bias = 1.0001; // Small bias to handle edge cases

    let normal = if (local.x / half.x).abs() * bias > (local.y / half.y).abs()
        && (local.x / half.x).abs() * bias > (local.z / half.z).abs()
    {
        Vec3::new(local.x.signum(), 0.0, 0.0)
    } else if (local.y / half.y).abs() * bias > (local.z / half.z).abs() {
        Vec3::new(0.0, local.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, local.z.signum())
    };

    Some(Intersection { t, point, normal })
}

/// Test ray intersection with sphere.
///
/// Returns the intersection if hit within ray's t_max.
#[must_use]
pub fn ray_sphere(ray: &Ray, sphere: &Sphere) -> Option<Intersection> {
    let oc = ray.origin - sphere.center;
    let a = ray.direction.dot(ray.direction);
    let b = 2.0 * oc.dot(ray.direction);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);

    let t = if t1 > 0.0 && t1 <= ray.t_max {
        t1
    } else if t2 > 0.0 && t2 <= ray.t_max {
        t2
    } else {
        return None;
    };

    let point = ray.at(t);
    let normal = (point - sphere.center).normalize();

    Some(Intersection { t, point, normal })
}

/// Test ray intersection with plane.
///
/// Returns the intersection if hit within ray's t_max.
#[must_use]
pub fn ray_plane(ray: &Ray, plane: &Plane) -> Option<Intersection> {
    let denom = plane.normal.dot(ray.direction);

    // Ray parallel to plane
    if denom.abs() < f32::EPSILON {
        return None;
    }

    let t = (plane.distance - plane.normal.dot(ray.origin)) / denom;

    if t < 0.0 || t > ray.t_max {
        return None;
    }

    let point = ray.at(t);
    let normal = if denom < 0.0 {
        plane.normal
    } else {
        -plane.normal
    };

    Some(Intersection { t, point, normal })
}

/// Test if AABB intersects sphere.
#[must_use]
pub fn aabb_sphere(aabb: &Aabb, sphere: &Sphere) -> bool {
    // Find closest point on AABB to sphere center
    let closest = Vec3::new(
        sphere.center.x.clamp(aabb.min.x, aabb.max.x),
        sphere.center.y.clamp(aabb.min.y, aabb.max.y),
        sphere.center.z.clamp(aabb.min.z, aabb.max.z),
    );

    let distance_sq = sphere.center.distance_squared(closest);
    distance_sq <= sphere.radius * sphere.radius
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_ray_aabb_hit_front() {
        let ray = Ray::new(Vec3::new(-5.0, 0.5, 0.5), Vec3::X);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let hit = ray_aabb(&ray, &aabb).expect("Should hit");
        assert_relative_eq!(hit.t, 5.0, epsilon = 0.001);
        assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn test_ray_aabb_miss() {
        let ray = Ray::new(Vec3::new(-5.0, 5.0, 5.0), Vec3::X);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        assert!(ray_aabb(&ray, &aabb).is_none());
    }

    #[test]
    fn test_ray_aabb_inside() {
        let ray = Ray::new(Vec3::splat(0.5), Vec3::X);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let hit = ray_aabb(&ray, &aabb).expect("Should hit exit");
        assert_relative_eq!(hit.t, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_ray_sphere_hit() {
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);
        let sphere = Sphere::new(Vec3::ZERO, 1.0);
        let hit = ray_sphere(&ray, &sphere).expect("Should hit");
        assert_relative_eq!(hit.t, 4.0, epsilon = 0.001);
        assert_relative_eq!(hit.normal.x, -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_ray_sphere_miss() {
        let ray = Ray::new(Vec3::new(-5.0, 5.0, 0.0), Vec3::X);
        let sphere = Sphere::new(Vec3::ZERO, 1.0);
        assert!(ray_sphere(&ray, &sphere).is_none());
    }

    #[test]
    fn test_ray_plane_hit() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y);
        let plane = Plane::new(Vec3::Y, 0.0);
        let hit = ray_plane(&ray, &plane).expect("Should hit");
        assert_relative_eq!(hit.t, 5.0, epsilon = 0.001);
    }

    #[test]
    fn test_ray_plane_parallel() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::X);
        let plane = Plane::new(Vec3::Y, 0.0);
        assert!(ray_plane(&ray, &plane).is_none());
    }

    #[test]
    fn test_aabb_sphere_intersecting() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let sphere = Sphere::new(Vec3::new(1.5, 0.5, 0.5), 1.0);
        assert!(aabb_sphere(&aabb, &sphere));
    }

    #[test]
    fn test_aabb_sphere_inside() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::splat(10.0));
        let sphere = Sphere::new(Vec3::splat(5.0), 1.0);
        assert!(aabb_sphere(&aabb, &sphere));
    }

    #[test]
    fn test_aabb_sphere_outside() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let sphere = Sphere::new(Vec3::splat(5.0), 1.0);
        assert!(!aabb_sphere(&aabb, &sphere));
    }
}

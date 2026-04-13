//! Capsule-AABB collision detection.

use engine_core::math::Aabb;
use glam::Vec3;

/// A capsule shape defined by two endpoints and a radius.
#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    /// Base point (bottom of capsule).
    pub base: Vec3,
    /// Tip point (top of capsule).
    pub tip: Vec3,
    /// Capsule radius.
    pub radius: f32,
}

impl Capsule {
    /// Create a new capsule.
    #[must_use]
    pub fn new(base: Vec3, tip: Vec3, radius: f32) -> Self {
        Self { base, tip, radius }
    }

    /// Create a capsule from center, height and radius.
    #[must_use]
    pub fn from_center(center: Vec3, height: f32, radius: f32) -> Self {
        let half_height = height / 2.0;
        Self {
            base: center - Vec3::Y * half_height,
            tip: center + Vec3::Y * half_height,
            radius,
        }
    }

    /// Get the center point.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        (self.base + self.tip) / 2.0
    }

    /// Get the height (distance between base and tip).
    #[must_use]
    pub fn height(&self) -> f32 {
        (self.tip - self.base).length()
    }

    /// Get the axis direction (normalized base to tip).
    #[must_use]
    pub fn axis(&self) -> Vec3 {
        (self.tip - self.base).normalize_or_zero()
    }
}

/// Contact information from collision detection.
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    /// Contact normal (points from B to A, i.e., push direction for A).
    pub normal: Vec3,
    /// Penetration depth (positive when overlapping).
    pub penetration: f32,
    /// Contact point on surface of A (capsule).
    pub point: Vec3,
}

/// Find the closest point on a line segment to a point.
fn closest_point_on_segment(a: Vec3, b: Vec3, point: Vec3) -> Vec3 {
    let ab = b - a;
    let t = (point - a).dot(ab) / ab.length_squared();
    a + ab * t.clamp(0.0, 1.0)
}

/// Find the closest point on an AABB to a point.
fn closest_point_on_aabb(aabb: &Aabb, point: Vec3) -> Vec3 {
    Vec3::new(
        point.x.clamp(aabb.min.x, aabb.max.x),
        point.y.clamp(aabb.min.y, aabb.max.y),
        point.z.clamp(aabb.min.z, aabb.max.z),
    )
}

/// Check if a capsule intersects an AABB.
///
/// Returns contact information if they overlap, including the normal
/// pointing from the AABB toward the capsule (the direction to push
/// the capsule to resolve the collision).
#[must_use]
pub fn capsule_aabb_intersection(capsule: &Capsule, aabb: &Aabb) -> Option<Contact> {
    // Find the closest point on the capsule's line segment to the AABB
    // by first finding the closest point on the segment to each AABB face,
    // then taking the minimum.

    // Sample several points along the capsule axis
    let samples = 5;
    let mut min_distance = f32::MAX;
    let mut closest_capsule_point = capsule.base;
    let mut closest_aabb_point = aabb.min;

    for i in 0..=samples {
        let t = i as f32 / samples as f32;
        let capsule_point = capsule.base.lerp(capsule.tip, t);
        let aabb_point = closest_point_on_aabb(aabb, capsule_point);
        let distance = (capsule_point - aabb_point).length();

        if distance < min_distance {
            min_distance = distance;
            closest_capsule_point = capsule_point;
            closest_aabb_point = aabb_point;
        }
    }

    // Refine: find exact closest point on segment to the current closest AABB point
    closest_capsule_point = closest_point_on_segment(capsule.base, capsule.tip, closest_aabb_point);
    closest_aabb_point = closest_point_on_aabb(aabb, closest_capsule_point);

    let diff = closest_capsule_point - closest_aabb_point;
    let distance = diff.length();

    // Check if capsule sphere at closest point overlaps AABB
    if distance < capsule.radius {
        let penetration = capsule.radius - distance;

        let normal = if distance > 0.0001 {
            diff.normalize()
        } else {
            // Capsule center is inside AABB, push out on Y axis (floor collision)
            Vec3::Y
        };

        let point = closest_capsule_point - normal * capsule.radius;

        Some(Contact {
            normal,
            penetration,
            point,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_collision_detected() {
        // Capsule standing on floor
        let capsule = Capsule::new(
            Vec3::new(0.0, 0.1, 0.0), // Base slightly below floor
            Vec3::new(0.0, 1.9, 0.0), // Tip at head height
            0.3,
        );

        // Floor AABB
        let floor = Aabb {
            min: Vec3::new(-10.0, -1.0, -10.0),
            max: Vec3::new(10.0, 0.0, 10.0),
        };

        let contact = capsule_aabb_intersection(&capsule, &floor);
        assert!(contact.is_some(), "Should detect floor collision");

        let contact = contact.unwrap();
        assert!(contact.normal.y > 0.9, "Normal should point up");
        assert!(contact.penetration > 0.0, "Should have penetration");
    }

    #[test]
    fn test_wall_collision_detected() {
        // Capsule next to wall
        let capsule = Capsule::new(
            Vec3::new(0.8, 0.0, 0.0), // Close to wall at x=1
            Vec3::new(0.8, 1.8, 0.0),
            0.3,
        );

        // Wall AABB
        let wall = Aabb {
            min: Vec3::new(1.0, -1.0, -10.0),
            max: Vec3::new(2.0, 3.0, 10.0),
        };

        let contact = capsule_aabb_intersection(&capsule, &wall);
        assert!(contact.is_some(), "Should detect wall collision");

        let contact = contact.unwrap();
        assert!(contact.normal.x < -0.9, "Normal should point away from wall (-X)");
    }

    #[test]
    fn test_no_collision_when_separated() {
        let capsule = Capsule::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 6.8, 0.0), 0.3);

        let floor = Aabb {
            min: Vec3::new(-10.0, -1.0, -10.0),
            max: Vec3::new(10.0, 0.0, 10.0),
        };

        let contact = capsule_aabb_intersection(&capsule, &floor);
        assert!(contact.is_none(), "Should not collide when separated");
    }

    #[test]
    fn test_penetration_depth_correct() {
        // Capsule with base exactly at floor level
        // The capsule's sphere at the base point touches y=0
        // So it should penetrate by the radius amount
        let capsule = Capsule::new(
            Vec3::new(0.0, 0.0, 0.0), // Base at floor level
            Vec3::new(0.0, 1.8, 0.0),
            0.3,
        );

        let floor = Aabb {
            min: Vec3::new(-10.0, -1.0, -10.0),
            max: Vec3::new(10.0, 0.0, 10.0),
        };

        let contact = capsule_aabb_intersection(&capsule, &floor);
        assert!(contact.is_some());

        let contact = contact.unwrap();
        // Penetration should be the radius (sphere at y=0 penetrates into floor by radius)
        assert!(
            contact.penetration > 0.0 && contact.penetration <= 0.35,
            "Penetration depth should be positive and around radius: got {}",
            contact.penetration
        );
    }

    #[test]
    fn test_capsule_from_center() {
        let capsule = Capsule::from_center(Vec3::new(0.0, 1.0, 0.0), 2.0, 0.3);
        assert!((capsule.base.y - 0.0).abs() < 0.001);
        assert!((capsule.tip.y - 2.0).abs() < 0.001);
        assert!((capsule.height() - 2.0).abs() < 0.001);
    }
}

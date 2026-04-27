//! Continuous Collision Detection for fast-moving objects.
//!
//! Provides swept sphere/AABB collision detection suitable for projectiles,
//! hazards, and high-velocity debris that might tunnel through thin geometry.

use engine_core::math::{Aabb, Sphere};
use glam::Vec3;

/// Result of a continuous collision detection query.
#[derive(Clone, Copy, Debug)]
pub struct CcdHit {
    /// Time of impact in [0, 1] range where 0 = start position, 1 = end position.
    pub time: f32,
    /// Contact normal pointing from the AABB surface toward the sphere.
    pub normal: Vec3,
    /// Contact point on the AABB surface at impact time.
    pub point: Vec3,
}

/// Extended hit result that includes the index of the collided object.
#[derive(Clone, Copy, Debug)]
pub struct CcdIndexedHit {
    /// The collision hit data.
    pub hit: CcdHit,
    /// Index of the AABB that was hit (in the input slice).
    pub index: usize,
}

/// Small epsilon for floating point comparisons.
const EPSILON: f32 = 1e-6;

/// Swept sphere vs AABB continuous collision detection.
///
/// Tests if a sphere moving along `velocity` will hit an AABB during the frame.
/// Returns the earliest hit with contact information.
///
/// # Arguments
/// * `sphere` - The sphere at its starting position
/// * `velocity` - The displacement vector for this frame (not normalized)
/// * `aabb` - The static AABB to test against
///
/// # Returns
/// `Some(CcdHit)` if collision occurs with time in [0, 1], `None` otherwise.
///
/// # Examples
/// ```
/// use engine_physics::collision::ccd::swept_sphere_aabb;
/// use engine_core::math::{Aabb, Sphere};
/// use glam::Vec3;
///
/// let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.25);
/// let velocity = Vec3::new(4.0, 0.0, 0.0);
/// let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);
///
/// if let Some(hit) = swept_sphere_aabb(&sphere, velocity, &aabb) {
///     println!("Hit at t={} with normal {:?}", hit.time, hit.normal);
/// }
/// ```
#[must_use]
pub fn swept_sphere_aabb(sphere: &Sphere, velocity: Vec3, aabb: &Aabb) -> Option<CcdHit> {
    // Handle zero velocity - just check static intersection
    let vel_len_sq = velocity.length_squared();
    if vel_len_sq < EPSILON * EPSILON {
        return check_initial_overlap(sphere, aabb);
    }

    // Expand AABB by sphere radius (Minkowski sum)
    let expanded = aabb.expand(sphere.radius);

    // Use slab method for ray-AABB intersection
    // Handle each axis separately to properly deal with zero velocity components
    let (t_enter, t_exit, hit_axis, hit_sign) =
        compute_slab_intersection(sphere.center, velocity, &expanded)?;

    // Check if intersection is within [0, 1] range
    if t_exit < 0.0 || t_enter > 1.0 {
        return None;
    }

    // If t_enter < 0, we started inside - handle initial overlap
    if t_enter < EPSILON {
        return check_initial_overlap(sphere, aabb);
    }

    // Compute hit point and normal
    let time = t_enter.clamp(0.0, 1.0);
    let sphere_center_at_hit = sphere.center + velocity * time;

    // Normal points from AABB toward sphere (outward from the face we hit)
    let normal = compute_face_normal(hit_axis, hit_sign);

    // Contact point is on the original (non-expanded) AABB surface
    let point = compute_contact_point(sphere_center_at_hit, sphere.radius, aabb, normal);

    Some(CcdHit {
        time,
        normal,
        point,
    })
}

/// Compute slab intersection for ray-AABB test.
/// Returns `(t_enter, t_exit, hit_axis, hit_sign)` or None if no intersection.
fn compute_slab_intersection(
    origin: Vec3,
    direction: Vec3,
    aabb: &Aabb,
) -> Option<(f32, f32, usize, f32)> {
    let mut t_enter = f32::NEG_INFINITY;
    let mut t_exit = f32::INFINITY;
    let mut hit_axis = 0usize;
    let mut hit_sign = -1.0f32;

    let origin_arr = [origin.x, origin.y, origin.z];
    let dir_arr = [direction.x, direction.y, direction.z];
    let min_arr = [aabb.min.x, aabb.min.y, aabb.min.z];
    let max_arr = [aabb.max.x, aabb.max.y, aabb.max.z];

    for i in 0..3 {
        if dir_arr[i].abs() < EPSILON {
            // Ray is parallel to slab - check if origin is inside
            if origin_arr[i] < min_arr[i] || origin_arr[i] > max_arr[i] {
                return None; // Outside slab, no intersection possible
            }
            // Inside slab, continue with other axes
        } else {
            let inv_d = 1.0 / dir_arr[i];
            let mut t1 = (min_arr[i] - origin_arr[i]) * inv_d;
            let mut t2 = (max_arr[i] - origin_arr[i]) * inv_d;

            // Track which face we're entering through
            let mut sign = -1.0f32;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                sign = 1.0;
            }

            if t1 > t_enter {
                t_enter = t1;
                hit_axis = i;
                hit_sign = sign;
            }
            t_exit = t_exit.min(t2);

            // Early exit if no overlap
            if t_enter > t_exit {
                return None;
            }
        }
    }

    Some((t_enter, t_exit, hit_axis, hit_sign))
}

/// Compute face normal from axis index and sign.
fn compute_face_normal(axis: usize, sign: f32) -> Vec3 {
    match axis {
        0 => Vec3::new(sign, 0.0, 0.0),
        1 => Vec3::new(0.0, sign, 0.0),
        _ => Vec3::new(0.0, 0.0, sign),
    }
}

/// Compute contact point on AABB surface.
fn compute_contact_point(sphere_center: Vec3, radius: f32, aabb: &Aabb, normal: Vec3) -> Vec3 {
    // Project sphere center onto AABB surface along normal
    let surface_point = sphere_center - normal * radius;

    // Clamp to AABB bounds
    Vec3::new(
        surface_point.x.clamp(aabb.min.x, aabb.max.x),
        surface_point.y.clamp(aabb.min.y, aabb.max.y),
        surface_point.z.clamp(aabb.min.z, aabb.max.z),
    )
}

/// Check for initial overlap when sphere starts inside or touching AABB.
fn check_initial_overlap(sphere: &Sphere, aabb: &Aabb) -> Option<CcdHit> {
    // Find closest point on AABB to sphere center
    let closest = Vec3::new(
        sphere.center.x.clamp(aabb.min.x, aabb.max.x),
        sphere.center.y.clamp(aabb.min.y, aabb.max.y),
        sphere.center.z.clamp(aabb.min.z, aabb.max.z),
    );

    let diff = sphere.center - closest;
    let dist_sq = diff.length_squared();

    if dist_sq > sphere.radius * sphere.radius {
        return None; // No overlap
    }

    let dist = dist_sq.sqrt();

    // Compute push-out normal
    let normal = if dist > EPSILON {
        diff / dist
    } else {
        // Sphere center is inside AABB - find minimum penetration axis
        find_minimum_penetration_axis(sphere.center, aabb)
    };

    Some(CcdHit {
        time: 0.0,
        normal,
        point: closest,
    })
}

/// Find the axis with minimum penetration when sphere center is inside AABB.
fn find_minimum_penetration_axis(point: Vec3, aabb: &Aabb) -> Vec3 {
    let to_min = point - aabb.min;
    let to_max = aabb.max - point;

    let distances = [
        (to_min.x, Vec3::NEG_X),
        (to_max.x, Vec3::X),
        (to_min.y, Vec3::NEG_Y),
        (to_max.y, Vec3::Y),
        (to_min.z, Vec3::NEG_Z),
        (to_max.z, Vec3::Z),
    ];

    distances
        .iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map_or(Vec3::Y, |(_, normal)| *normal)
}

/// Sweep a sphere against multiple AABBs and return the earliest hit.
///
/// # Arguments
/// * `sphere` - The sphere at its starting position
/// * `velocity` - The displacement vector for this frame
/// * `aabbs` - Slice of static AABBs to test against
///
/// # Returns
/// The earliest hit with its index in the input slice, or `None` if no hit.
///
/// # Examples
/// ```
/// use engine_physics::collision::ccd::swept_sphere_aabbs;
/// use engine_core::math::{Aabb, Sphere};
/// use glam::Vec3;
///
/// let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.25);
/// let velocity = Vec3::new(10.0, 0.0, 0.0);
/// let walls = vec![
///     Aabb::new(Vec3::ZERO, Vec3::ONE),
///     Aabb::new(Vec3::new(3.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0)),
/// ];
///
/// if let Some(result) = swept_sphere_aabbs(&sphere, velocity, &walls) {
///     println!("Hit wall {} at t={}", result.index, result.hit.time);
/// }
/// ```
#[must_use]
pub fn swept_sphere_aabbs(
    sphere: &Sphere,
    velocity: Vec3,
    aabbs: &[Aabb],
) -> Option<CcdIndexedHit> {
    let mut earliest: Option<CcdIndexedHit> = None;

    for (index, aabb) in aabbs.iter().enumerate() {
        if let Some(hit) = swept_sphere_aabb(sphere, velocity, aabb) {
            match &earliest {
                None => {
                    earliest = Some(CcdIndexedHit { hit, index });
                }
                Some(current) if hit.time < current.hit.time => {
                    earliest = Some(CcdIndexedHit { hit, index });
                }
                _ => {}
            }
        }
    }

    earliest
}

/// Collect all hits from sweeping a sphere against multiple AABBs.
///
/// Returns hits sorted by time of impact (earliest first).
///
/// # Arguments
/// * `sphere` - The sphere at its starting position
/// * `velocity` - The displacement vector for this frame
/// * `aabbs` - Slice of static AABBs to test against
///
/// # Returns
/// Vector of all hits sorted by time, earliest first.
#[must_use]
pub fn swept_sphere_aabbs_all(
    sphere: &Sphere,
    velocity: Vec3,
    aabbs: &[Aabb],
) -> Vec<CcdIndexedHit> {
    let mut hits: Vec<CcdIndexedHit> = aabbs
        .iter()
        .enumerate()
        .filter_map(|(index, aabb)| {
            swept_sphere_aabb(sphere, velocity, aabb).map(|hit| CcdIndexedHit { hit, index })
        })
        .collect();

    hits.sort_by(|a, b| {
        a.hit
            .time
            .partial_cmp(&b.hit.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    hits
}

/// Compute bounce velocity response.
///
/// Reflects the velocity about the collision normal with optional restitution.
///
/// # Arguments
/// * `velocity` - The incoming velocity
/// * `normal` - The collision normal (should be normalized)
/// * `restitution` - Coefficient of restitution (0 = no bounce, 1 = perfect bounce)
///
/// # Returns
/// The reflected velocity.
///
/// # Examples
/// ```
/// use engine_physics::collision::ccd::compute_bounce;
/// use glam::Vec3;
///
/// let velocity = Vec3::new(5.0, -3.0, 0.0);
/// let normal = Vec3::Y;
/// let bounced = compute_bounce(velocity, normal, 0.8);
/// // Y component is reflected and scaled by restitution
/// assert!(bounced.y > 0.0);
/// ```
#[must_use]
pub fn compute_bounce(velocity: Vec3, normal: Vec3, restitution: f32) -> Vec3 {
    let normal_velocity = velocity.dot(normal);

    // Only bounce if moving into the surface
    if normal_velocity >= 0.0 {
        return velocity;
    }

    // Reflect: v' = v - (1 + e) * (v · n) * n
    velocity - normal * normal_velocity * (1.0 + restitution)
}

/// Compute slide velocity response.
///
/// Removes the velocity component along the collision normal, allowing the
/// object to slide along the surface.
///
/// # Arguments
/// * `velocity` - The incoming velocity
/// * `normal` - The collision normal (should be normalized)
///
/// # Returns
/// The velocity with the normal component removed.
///
/// # Examples
/// ```
/// use engine_physics::collision::ccd::compute_slide;
/// use glam::Vec3;
///
/// let velocity = Vec3::new(5.0, -3.0, 0.0);
/// let normal = Vec3::Y;
/// let slide = compute_slide(velocity, normal);
/// // Y component is removed
/// assert!(slide.y.abs() < 0.001);
/// assert!((slide.x - 5.0).abs() < 0.001);
/// ```
#[must_use]
pub fn compute_slide(velocity: Vec3, normal: Vec3) -> Vec3 {
    let normal_velocity = velocity.dot(normal);

    // Only slide if moving into the surface
    if normal_velocity >= 0.0 {
        return velocity;
    }

    // Remove normal component: v' = v - (v · n) * n
    velocity - normal * normal_velocity
}

/// Compute slide velocity with friction.
///
/// Removes the normal velocity component and applies friction to the tangential
/// component.
///
/// # Arguments
/// * `velocity` - The incoming velocity
/// * `normal` - The collision normal (should be normalized)
/// * `friction` - Friction coefficient (0 = no friction, 1 = full stop)
///
/// # Returns
/// The sliding velocity with friction applied.
#[must_use]
pub fn compute_slide_with_friction(velocity: Vec3, normal: Vec3, friction: f32) -> Vec3 {
    let normal_velocity = velocity.dot(normal);

    // Only slide if moving into the surface
    if normal_velocity >= 0.0 {
        return velocity;
    }

    // Remove normal component
    let tangent_velocity = velocity - normal * normal_velocity;

    // Apply friction
    tangent_velocity * (1.0 - friction.clamp(0.0, 1.0))
}

/// Step a projectile through the scene with iterative collision resolution.
///
/// Performs multiple collision iterations to handle corner cases where the
/// projectile might hit multiple surfaces in quick succession.
///
/// # Arguments
/// * `sphere` - The sphere at its starting position
/// * `velocity` - The desired displacement for this frame
/// * `aabbs` - Slice of static AABBs to test against
/// * `restitution` - Bounce factor (0 = slide, 1 = perfect bounce)
/// * `max_iterations` - Maximum collision iterations (typically 3-5)
///
/// # Returns
/// Tuple of `(final_position, final_velocity)` after collision resolution.
#[must_use]
pub fn step_projectile(
    sphere: &Sphere,
    velocity: Vec3,
    aabbs: &[Aabb],
    restitution: f32,
    max_iterations: u32,
) -> (Vec3, Vec3) {
    let mut position = sphere.center;
    let mut remaining_velocity = velocity;
    let mut current_sphere = *sphere;

    for _ in 0..max_iterations {
        if remaining_velocity.length_squared() < EPSILON * EPSILON {
            break;
        }

        current_sphere.center = position;

        if let Some(result) = swept_sphere_aabbs(&current_sphere, remaining_velocity, aabbs) {
            let hit = result.hit;

            // Move to just before contact
            let safe_time = (hit.time - EPSILON).max(0.0);
            position += remaining_velocity * safe_time;

            // Compute response velocity
            let response = if restitution > EPSILON {
                compute_bounce(remaining_velocity, hit.normal, restitution)
            } else {
                compute_slide(remaining_velocity, hit.normal)
            };

            // Scale by remaining time
            remaining_velocity = response * (1.0 - hit.time).max(0.0);
        } else {
            // No collision, apply full remaining velocity
            position += remaining_velocity;
            break;
        }
    }

    // Compute final velocity from the response
    let final_velocity = if velocity.length_squared() > EPSILON * EPSILON {
        let original_speed = velocity.length();
        let final_direction = remaining_velocity.normalize_or_zero();
        final_direction * original_speed * (remaining_velocity.length() / velocity.length())
    } else {
        Vec3::ZERO
    };

    (position, final_velocity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========== Basic Hit Tests ==========

    #[test]
    fn ccd_fast_projectile_hits_wall() {
        // Fast projectile that would tunnel without CCD
        let sphere = Sphere::new(Vec3::new(-5.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(100.0, 0.0, 0.0); // Very fast
        let thin_wall = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.05, 1.0, 1.0));

        let hit = swept_sphere_aabb(&sphere, velocity, &thin_wall);
        assert!(hit.is_some(), "Should detect collision with thin wall");

        let hit = hit.unwrap();
        assert!(hit.time > 0.0 && hit.time < 1.0, "Hit should be mid-flight");
        assert!(hit.normal.x < -0.9, "Normal should point -X");
    }

    #[test]
    fn ccd_projectile_misses_wall() {
        let sphere = Sphere::new(Vec3::new(-5.0, 5.0, 0.5), 0.1);
        let velocity = Vec3::new(100.0, 0.0, 0.0);
        let wall = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &wall);
        assert!(hit.is_none(), "Should miss wall above");
    }

    #[test]
    fn ccd_hit_from_different_directions() {
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        // From -X
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.1);
        let hit = swept_sphere_aabb(&sphere, Vec3::new(4.0, 0.0, 0.0), &aabb).unwrap();
        assert!(hit.normal.x < -0.9, "From -X, normal should be -X");

        // From +X
        let sphere = Sphere::new(Vec3::new(3.0, 0.5, 0.5), 0.1);
        let hit = swept_sphere_aabb(&sphere, Vec3::new(-4.0, 0.0, 0.0), &aabb).unwrap();
        assert!(hit.normal.x > 0.9, "From +X, normal should be +X");

        // From -Y
        let sphere = Sphere::new(Vec3::new(0.5, -2.0, 0.5), 0.1);
        let hit = swept_sphere_aabb(&sphere, Vec3::new(0.0, 4.0, 0.0), &aabb).unwrap();
        assert!(hit.normal.y < -0.9, "From -Y, normal should be -Y");

        // From +Z
        let sphere = Sphere::new(Vec3::new(0.5, 0.5, 3.0), 0.1);
        let hit = swept_sphere_aabb(&sphere, Vec3::new(0.0, 0.0, -4.0), &aabb).unwrap();
        assert!(hit.normal.z > 0.9, "From +Z, normal should be +Z");
    }

    // ========== Zero Velocity Axis Tests ==========

    #[test]
    fn ccd_zero_velocity_on_two_axes() {
        // Moving only on X axis
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(4.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_some());
    }

    #[test]
    fn ccd_zero_velocity_outside_slab() {
        // Sphere outside AABB on Y, not moving on Y
        let sphere = Sphere::new(Vec3::new(-2.0, 5.0, 0.5), 0.1);
        let velocity = Vec3::new(4.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(
            hit.is_none(),
            "Should miss - outside Y slab with no Y velocity"
        );
    }

    #[test]
    fn ccd_completely_zero_velocity() {
        // No movement at all, sphere not touching
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::ZERO;
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_none(), "No movement, no overlap = no hit");
    }

    #[test]
    fn ccd_zero_velocity_but_overlapping() {
        // No movement, but sphere overlaps AABB
        let sphere = Sphere::new(Vec3::new(0.5, 0.5, 0.5), 0.1);
        let velocity = Vec3::ZERO;
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_some(), "No movement but overlapping = hit at t=0");
        assert!((hit.unwrap().time - 0.0).abs() < EPSILON);
    }

    // ========== Starting Inside Tests ==========

    #[test]
    fn ccd_starting_inside_aabb() {
        let sphere = Sphere::new(Vec3::new(0.5, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_some(), "Starting inside should report hit");
        assert!(
            (hit.unwrap().time - 0.0).abs() < EPSILON,
            "Hit time should be 0"
        );
    }

    #[test]
    fn ccd_starting_touching_surface() {
        // Sphere just touching the -X face
        let sphere = Sphere::new(Vec3::new(-0.1, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_some(), "Touching should report hit");
        assert!(hit.unwrap().time < 0.01, "Hit time should be near 0");
    }

    #[test]
    fn ccd_center_deep_inside_aabb() {
        // Sphere center deep inside AABB
        let sphere = Sphere::new(Vec3::new(0.2, 0.2, 0.2), 0.1);
        let velocity = Vec3::new(-1.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.time - 0.0).abs() < EPSILON);
        // Should push out the shortest direction
        assert!(hit.normal.length() > 0.9);
    }

    // ========== Time of Impact Range Tests ==========

    #[test]
    fn ccd_hit_at_end_of_frame() {
        // Sphere barely reaches the wall at t=1
        let sphere = Sphere::new(Vec3::new(-1.1, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_some());
        assert!(hit.unwrap().time <= 1.0, "Time should be at most 1.0");
    }

    #[test]
    fn ccd_hit_beyond_frame_rejected() {
        // Sphere won't reach wall this frame
        let sphere = Sphere::new(Vec3::new(-3.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(1.0, 0.0, 0.0); // Would take 2+ frames
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb);
        assert!(hit.is_none(), "Hit beyond t=1 should be rejected");
    }

    // ========== Contact Point Tests ==========

    #[test]
    fn ccd_contact_point_on_surface() {
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.2);
        let velocity = Vec3::new(4.0, 0.0, 0.0);
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb).unwrap();

        // Contact point should be on the AABB surface (x=0)
        assert!(
            (hit.point.x - 0.0).abs() < 0.01,
            "Contact X should be on AABB surface"
        );
        // Contact point Y and Z should be within AABB bounds
        assert!(hit.point.y >= 0.0 && hit.point.y <= 1.0);
        assert!(hit.point.z >= 0.0 && hit.point.z <= 1.0);
    }

    #[test]
    fn ccd_contact_normal_is_normalized() {
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.2);
        let velocity = Vec3::new(4.0, 0.5, 0.3); // Diagonal
        let aabb = Aabb::new(Vec3::ZERO, Vec3::ONE);

        let hit = swept_sphere_aabb(&sphere, velocity, &aabb).unwrap();
        assert_relative_eq!(hit.normal.length(), 1.0, epsilon = 0.001);
    }

    // ========== Earliest Hit Tests ==========

    #[test]
    fn ccd_earliest_hit_from_multiple_aabbs() {
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(10.0, 0.0, 0.0);

        let aabbs = vec![
            Aabb::new(Vec3::new(5.0, 0.0, 0.0), Vec3::new(6.0, 1.0, 1.0)), // Far
            Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)), // Near (hit first)
            Aabb::new(Vec3::new(3.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0)), // Middle
        ];

        let result = swept_sphere_aabbs(&sphere, velocity, &aabbs);
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!(result.index, 1, "Should hit the near wall (index 1) first");
    }

    #[test]
    fn ccd_all_hits_sorted_by_time() {
        let sphere = Sphere::new(Vec3::new(-2.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(20.0, 0.0, 0.0);

        let aabbs = vec![
            Aabb::new(Vec3::new(5.0, 0.0, 0.0), Vec3::new(5.1, 1.0, 1.0)),
            Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.1, 1.0, 1.0)),
            Aabb::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(10.1, 1.0, 1.0)),
        ];

        let hits = swept_sphere_aabbs_all(&sphere, velocity, &aabbs);

        assert_eq!(hits.len(), 3, "Should hit all 3 walls");

        // Verify sorted by time
        for i in 1..hits.len() {
            assert!(
                hits[i].hit.time >= hits[i - 1].hit.time,
                "Hits should be sorted by time"
            );
        }

        // First hit should be the closest wall (index 1)
        assert_eq!(hits[0].index, 1);
    }

    #[test]
    fn ccd_no_hits_returns_empty() {
        let sphere = Sphere::new(Vec3::new(-2.0, 5.0, 5.0), 0.1);
        let velocity = Vec3::new(10.0, 0.0, 0.0);

        let aabbs = vec![
            Aabb::new(Vec3::ZERO, Vec3::ONE),
            Aabb::new(Vec3::new(3.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0)),
        ];

        let result = swept_sphere_aabbs(&sphere, velocity, &aabbs);
        assert!(result.is_none());

        let all_hits = swept_sphere_aabbs_all(&sphere, velocity, &aabbs);
        assert!(all_hits.is_empty());
    }

    // ========== Bounce Response Tests ==========

    #[test]
    fn ccd_bounce_perfect_reflection() {
        let velocity = Vec3::new(1.0, -1.0, 0.0);
        let normal = Vec3::Y;

        let bounced = compute_bounce(velocity, normal, 1.0);

        assert_relative_eq!(bounced.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(bounced.y, 1.0, epsilon = 0.001); // Reflected
        assert_relative_eq!(bounced.z, 0.0, epsilon = 0.001);
    }

    #[test]
    fn ccd_bounce_with_restitution() {
        let velocity = Vec3::new(0.0, -10.0, 0.0);
        let normal = Vec3::Y;

        let bounced = compute_bounce(velocity, normal, 0.5);

        assert_relative_eq!(bounced.y, 5.0, epsilon = 0.001); // Half bounce
    }

    #[test]
    fn ccd_bounce_no_restitution() {
        let velocity = Vec3::new(0.0, -10.0, 0.0);
        let normal = Vec3::Y;

        let bounced = compute_bounce(velocity, normal, 0.0);

        assert_relative_eq!(bounced.y, 0.0, epsilon = 0.001); // No bounce = slide
    }

    #[test]
    fn ccd_bounce_moving_away_unchanged() {
        let velocity = Vec3::new(0.0, 5.0, 0.0); // Moving up, away from floor
        let normal = Vec3::Y;

        let bounced = compute_bounce(velocity, normal, 1.0);

        assert_relative_eq!(bounced.y, 5.0, epsilon = 0.001); // Unchanged
    }

    // ========== Slide Response Tests ==========

    #[test]
    fn ccd_slide_removes_normal_component() {
        let velocity = Vec3::new(5.0, -3.0, 2.0);
        let normal = Vec3::Y;

        let slide = compute_slide(velocity, normal);

        assert_relative_eq!(slide.x, 5.0, epsilon = 0.001);
        assert_relative_eq!(slide.y, 0.0, epsilon = 0.001); // Normal component removed
        assert_relative_eq!(slide.z, 2.0, epsilon = 0.001);
    }

    #[test]
    fn ccd_slide_diagonal_normal() {
        // Velocity moving into surface (negative dot with normal)
        let velocity = Vec3::new(-1.0, 0.0, 0.0);
        let normal = Vec3::new(1.0, 1.0, 0.0).normalize();

        let slide = compute_slide(velocity, normal);

        // Should slide along the surface, losing speed
        assert!(slide.length() < velocity.length());
        assert!(
            slide.dot(normal).abs() < 0.001,
            "Slide should be tangent to surface"
        );
    }

    #[test]
    fn ccd_slide_moving_away_unchanged() {
        let velocity = Vec3::new(0.0, 5.0, 0.0);
        let normal = Vec3::Y;

        let slide = compute_slide(velocity, normal);

        assert_eq!(slide, velocity);
    }

    #[test]
    fn ccd_slide_with_friction() {
        let velocity = Vec3::new(10.0, -5.0, 0.0);
        let normal = Vec3::Y;

        let slide_no_friction = compute_slide_with_friction(velocity, normal, 0.0);
        let slide_half_friction = compute_slide_with_friction(velocity, normal, 0.5);
        let slide_full_friction = compute_slide_with_friction(velocity, normal, 1.0);

        assert_relative_eq!(slide_no_friction.x, 10.0, epsilon = 0.001);
        assert_relative_eq!(slide_half_friction.x, 5.0, epsilon = 0.001);
        assert_relative_eq!(slide_full_friction.x, 0.0, epsilon = 0.001);
    }

    // ========== Step Projectile Tests ==========

    #[test]
    fn ccd_step_projectile_bounces() {
        let sphere = Sphere::new(Vec3::new(-1.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(2.0, 0.0, 0.0);
        let aabbs = vec![Aabb::new(Vec3::ZERO, Vec3::ONE)];

        let (_final_pos, final_vel) = step_projectile(&sphere, velocity, &aabbs, 1.0, 3);

        // Should have bounced back
        assert!(final_vel.x < 0.0, "Should bounce back in -X direction");
    }

    #[test]
    fn ccd_step_projectile_slides() {
        let sphere = Sphere::new(Vec3::new(-1.0, 0.5, 0.5), 0.1);
        let velocity = Vec3::new(2.0, 0.5, 0.0);
        let aabbs = vec![Aabb::new(Vec3::ZERO, Vec3::ONE)];

        let (_final_pos, final_vel) = step_projectile(&sphere, velocity, &aabbs, 0.0, 3);

        // Should slide along the surface (no X component into wall)
        assert!(
            final_vel.x.abs() < 0.5,
            "X velocity should be reduced after slide"
        );
    }

    #[test]
    fn ccd_step_projectile_no_collision() {
        let sphere = Sphere::new(Vec3::new(-5.0, 5.0, 5.0), 0.1);
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let aabbs = vec![Aabb::new(Vec3::ZERO, Vec3::ONE)];

        let (final_pos, _) = step_projectile(&sphere, velocity, &aabbs, 0.5, 3);

        // Should move full distance
        assert_relative_eq!(final_pos.x, -4.0, epsilon = 0.01);
    }
}

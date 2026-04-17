//! Herd behavior for passive animals.
//!
//! Implements spec 8.3.1: passive animals follow the nearest
//! same-type creature within 8 blocks.

use glam::Vec3;

/// Maximum distance for herd following (blocks).
pub const HERD_FOLLOW_DISTANCE: f32 = 8.0;

/// Minimum distance to maintain from herd leader (blocks).
pub const HERD_MIN_DISTANCE: f32 = 2.0;

/// Speed multiplier when following the herd.
pub const HERD_FOLLOW_SPEED: f32 = 0.8;

/// Herd behavior state for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HerdState {
    /// Not following anyone, wandering freely.
    Independent,
    /// Following another creature of the same type.
    Following,
}

/// Result of herd behavior calculation.
#[derive(Debug, Clone)]
pub struct HerdResult {
    /// New state.
    pub state: HerdState,
    /// Target position to move toward (if following).
    pub target_position: Option<Vec3>,
    /// Movement speed multiplier.
    pub speed_multiplier: f32,
}

/// Calculate herd behavior for a creature.
///
/// Given the creature's position, type, and nearby same-type positions,
/// determines whether the creature should follow the nearest one.
#[must_use]
pub fn calculate_herd_behavior(
    creature_pos: Vec3,
    same_type_positions: &[Vec3],
) -> HerdResult {
    // Find nearest same-type creature
    let nearest = same_type_positions
        .iter()
        .filter(|pos| pos.distance(creature_pos) > HERD_MIN_DISTANCE)
        .min_by(|a, b| {
            a.distance(creature_pos)
                .partial_cmp(&b.distance(creature_pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let Some(target) = nearest else {
        return HerdResult {
            state: HerdState::Independent,
            target_position: None,
            speed_multiplier: 1.0,
        };
    };

    let distance = target.distance(creature_pos);

    if distance <= HERD_FOLLOW_DISTANCE {
        HerdResult {
            state: HerdState::Following,
            target_position: Some(*target),
            speed_multiplier: HERD_FOLLOW_SPEED,
        }
    } else {
        HerdResult {
            state: HerdState::Independent,
            target_position: None,
            speed_multiplier: 1.0,
        }
    }
}

/// Find the herd leader for a group of same-type creatures.
///
/// The leader is the creature closest to the center of the group.
#[must_use]
pub fn find_herd_leader(positions: &[Vec3]) -> Option<Vec3> {
    if positions.is_empty() {
        return None;
    }

    // Calculate center of the group
    let center: Vec3 = positions.iter().sum::<Vec3>() / positions.len() as f32;

    // Find the creature closest to center
    positions
        .iter()
        .min_by(|a, b| {
            a.distance(center)
                .partial_cmp(&b.distance(center))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
}

/// Calculate cohesion force for herd movement.
///
/// Returns a direction vector pointing toward the group center,
/// with magnitude based on distance from center.
#[must_use]
pub fn cohesion_force(creature_pos: Vec3, group_positions: &[Vec3]) -> Vec3 {
    if group_positions.is_empty() {
        return Vec3::ZERO;
    }

    let center: Vec3 = group_positions.iter().sum::<Vec3>() / group_positions.len() as f32;
    let to_center = center - creature_pos;

    if to_center.length() < HERD_MIN_DISTANCE {
        Vec3::ZERO
    } else {
        to_center.normalize()
    }
}

/// Calculate separation force to avoid crowding.
///
/// Pushes creatures apart if they're too close.
#[must_use]
pub fn separation_force(creature_pos: Vec3, nearby_positions: &[Vec3], min_distance: f32) -> Vec3 {
    let mut force = Vec3::ZERO;

    for other_pos in nearby_positions {
        let diff = creature_pos - *other_pos;
        let distance = diff.length();
        if distance > 0.0 && distance < min_distance {
            // Stronger push when closer
            force += diff.normalize() / distance;
        }
    }

    force
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_independent_when_alone() {
        let result = calculate_herd_behavior(Vec3::new(0.0, 0.0, 0.0), &[]);
        assert_eq!(result.state, HerdState::Independent);
        assert!(result.target_position.is_none());
        assert_eq!(result.speed_multiplier, 1.0);
    }

    #[test]
    fn test_follow_nearby_same_type() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let nearby = vec![Vec3::new(5.0, 0.0, 0.0)];

        let result = calculate_herd_behavior(creature, &nearby);
        assert_eq!(result.state, HerdState::Following);
        assert_eq!(result.target_position, Some(Vec3::new(5.0, 0.0, 0.0)));
        assert!((result.speed_multiplier - HERD_FOLLOW_SPEED).abs() < 0.001);
    }

    #[test]
    fn test_independent_when_too_far() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let far = vec![Vec3::new(15.0, 0.0, 0.0)];

        let result = calculate_herd_behavior(creature, &far);
        assert_eq!(result.state, HerdState::Independent);
    }

    #[test]
    fn test_independent_when_too_close() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let too_close = vec![Vec3::new(1.0, 0.0, 0.0)]; // Less than HERD_MIN_DISTANCE

        let result = calculate_herd_behavior(creature, &too_close);
        assert_eq!(result.state, HerdState::Independent);
    }

    #[test]
    fn test_follows_nearest() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let nearby = vec![
            Vec3::new(3.0, 0.0, 0.0), // Nearest
            Vec3::new(6.0, 0.0, 0.0), // Farther
        ];

        let result = calculate_herd_behavior(creature, &nearby);
        assert_eq!(result.state, HerdState::Following);
        assert_eq!(result.target_position, Some(Vec3::new(3.0, 0.0, 0.0)));
    }

    #[test]
    fn test_find_herd_leader() {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0), // Closest to center
        ];

        let leader = find_herd_leader(&positions);
        assert_eq!(leader, Some(Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn test_find_herd_leader_empty() {
        assert!(find_herd_leader(&[]).is_none());
    }

    #[test]
    fn test_find_herd_leader_single() {
        let positions = vec![Vec3::new(5.0, 0.0, 5.0)];
        assert_eq!(find_herd_leader(&positions), Some(Vec3::new(5.0, 0.0, 5.0)));
    }

    #[test]
    fn test_cohesion_force() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let group = vec![Vec3::new(6.0, 0.0, 6.0)];

        let force = cohesion_force(creature, &group);
        assert!(force.length() > 0.0, "Should have cohesion force");
        // Force should point toward group
        assert!(force.x > 0.0 && force.z > 0.0);
    }

    #[test]
    fn test_cohesion_force_near_center() {
        let creature = Vec3::new(5.0, 0.0, 5.0);
        let group = vec![Vec3::new(5.5, 0.0, 5.5)]; // Very close

        let force = cohesion_force(creature, &group);
        // Too close, force should be zero or negligible
        assert!(force.length() < 0.1);
    }

    #[test]
    fn test_separation_force() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let nearby = vec![Vec3::new(1.0, 0.0, 0.0)]; // Too close

        let force = separation_force(creature, &nearby, 2.0);
        assert!(force.x < 0.0, "Should push away from nearby");
    }

    #[test]
    fn test_separation_force_far_enough() {
        let creature = Vec3::new(0.0, 0.0, 0.0);
        let nearby = vec![Vec3::new(10.0, 0.0, 0.0)]; // Far enough

        let force = separation_force(creature, &nearby, 2.0);
        assert!(force.length() < 0.01, "No separation when far enough");
    }

    #[test]
    fn test_herd_state_variants() {
        assert_ne!(HerdState::Independent, HerdState::Following);
    }
}

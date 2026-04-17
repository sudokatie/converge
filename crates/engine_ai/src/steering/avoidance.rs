//! Obstacle avoidance steering behavior - raycast ahead and steer around obstacles.

use crate::behavior::blackboard::Vec3;
use crate::pathfinding::astar::GridPos;
use super::{SteeringBehavior, SteeringOutput};

/// Result of a raycast for obstacle detection
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    /// Position where the ray hit
    pub position: Vec3,
    /// Normal of the surface hit
    pub normal: Vec3,
    /// Distance to the hit
    pub distance: f32,
}

/// Trait for checking obstacles in the world
pub trait ObstacleChecker {
    /// Cast a ray and return hit information if blocked
    fn raycast(&self, origin: &Vec3, direction: &Vec3, max_distance: f32) -> Option<RaycastHit>;

    /// Check if a position is blocked
    fn is_blocked(&self, pos: &GridPos) -> bool;
}

/// Obstacle avoidance behavior: raycast ahead and steer around obstacles
#[derive(Debug, Clone)]
pub struct ObstacleAvoidance {
    /// Look-ahead distance
    pub look_ahead: f32,
    /// Width of the character (for side feelers)
    pub character_width: f32,
    /// Maximum avoidance force
    pub max_avoidance_force: f32,
    /// Number of side feelers (rays)
    pub num_feelers: u32,
    /// Feeler spread angle (radians)
    pub feeler_spread: f32,
}

impl ObstacleAvoidance {
    pub fn new() -> Self {
        Self {
            look_ahead: 4.0,
            character_width: 0.6,
            max_avoidance_force: 8.0,
            num_feelers: 3, // center + 2 sides
            feeler_spread: std::f32::consts::FRAC_PI_4, // 45 degrees
        }
    }

    pub fn with_look_ahead(mut self, distance: f32) -> Self {
        self.look_ahead = distance;
        self
    }

    pub fn with_character_width(mut self, width: f32) -> Self {
        self.character_width = width;
        self
    }

    pub fn with_max_force(mut self, force: f32) -> Self {
        self.max_avoidance_force = force;
        self
    }

    /// Calculate avoidance steering with an obstacle checker
    pub fn calculate_with_checker<C: ObstacleChecker>(
        &self,
        position: &Vec3,
        velocity: &Vec3,
        checker: &C,
    ) -> SteeringOutput {
        let speed = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();

        if speed < 0.001 {
            return SteeringOutput::zero();
        }

        // Normalized velocity (facing direction)
        let facing_x = velocity.x / speed;
        let facing_z = velocity.z / speed;

        // Dynamic look-ahead based on speed
        let dynamic_look_ahead = self.look_ahead * (speed / 5.0).min(1.0).max(0.3);

        let mut total_avoidance = Vec3::new(0.0, 0.0, 0.0);
        let mut hit_count = 0;

        // Cast feeler rays
        for i in 0..self.num_feelers {
            // Calculate feeler angle offset
            let angle_offset = if self.num_feelers == 1 {
                0.0
            } else {
                let t = i as f32 / (self.num_feelers - 1) as f32;
                (t - 0.5) * 2.0 * self.feeler_spread
            };

            // Rotate facing direction by angle offset
            let cos_a = angle_offset.cos();
            let sin_a = angle_offset.sin();
            let feeler_x = facing_x * cos_a - facing_z * sin_a;
            let feeler_z = facing_x * sin_a + facing_z * cos_a;

            let direction = Vec3::new(feeler_x, 0.0, feeler_z);

            // Cast ray
            if let Some(hit) = checker.raycast(position, &direction, dynamic_look_ahead) {
                // Calculate avoidance force - stronger when closer
                let urgency = 1.0 - (hit.distance / dynamic_look_ahead);

                // Steer perpendicular to obstacle (use hit normal)
                let avoidance_x = hit.normal.x * urgency * self.max_avoidance_force;
                let avoidance_z = hit.normal.z * urgency * self.max_avoidance_force;

                total_avoidance.x += avoidance_x;
                total_avoidance.z += avoidance_z;
                hit_count += 1;
            }
        }

        if hit_count > 0 {
            // Average the avoidance forces
            total_avoidance.x /= hit_count as f32;
            total_avoidance.z /= hit_count as f32;

            SteeringOutput::new(total_avoidance, 0.0)
        } else {
            SteeringOutput::zero()
        }
    }
}

impl Default for ObstacleAvoidance {
    fn default() -> Self {
        Self::new()
    }
}

impl SteeringBehavior for ObstacleAvoidance {
    fn calculate(&self, _position: &Vec3, _velocity: &Vec3) -> SteeringOutput {
        // Without an obstacle checker, we can't detect obstacles
        // Use calculate_with_checker for actual avoidance
        SteeringOutput::zero()
    }
}

/// Simple obstacle checker for testing
pub struct SimpleObstacleChecker {
    obstacles: Vec<(Vec3, f32)>, // (center, radius)
}

impl SimpleObstacleChecker {
    pub fn new() -> Self {
        Self {
            obstacles: Vec::new(),
        }
    }

    pub fn add_obstacle(&mut self, center: Vec3, radius: f32) {
        self.obstacles.push((center, radius));
    }
}

impl Default for SimpleObstacleChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ObstacleChecker for SimpleObstacleChecker {
    fn raycast(&self, origin: &Vec3, direction: &Vec3, max_distance: f32) -> Option<RaycastHit> {
        let mut closest_hit: Option<RaycastHit> = None;
        let mut closest_dist = max_distance;

        for (center, radius) in &self.obstacles {
            // Ray-sphere intersection
            let oc_x = origin.x - center.x;
            let oc_z = origin.z - center.z;

            let a = direction.x * direction.x + direction.z * direction.z;
            let b = 2.0 * (oc_x * direction.x + oc_z * direction.z);
            let c = oc_x * oc_x + oc_z * oc_z - radius * radius;

            let discriminant = b * b - 4.0 * a * c;

            if discriminant >= 0.0 {
                let t = (-b - discriminant.sqrt()) / (2.0 * a);

                if t > 0.0 && t < closest_dist {
                    let hit_x = origin.x + direction.x * t;
                    let hit_z = origin.z + direction.z * t;

                    // Normal points from center to hit point
                    let nx = hit_x - center.x;
                    let nz = hit_z - center.z;
                    let n_len = (nx * nx + nz * nz).sqrt();

                    closest_dist = t;
                    closest_hit = Some(RaycastHit {
                        position: Vec3::new(hit_x, origin.y, hit_z),
                        normal: Vec3::new(nx / n_len, 0.0, nz / n_len),
                        distance: t,
                    });
                }
            }
        }

        closest_hit
    }

    fn is_blocked(&self, pos: &GridPos) -> bool {
        let pos_vec = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
        for (center, radius) in &self.obstacles {
            let dist = pos_vec.distance(center);
            if dist < *radius {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_obstacle() {
        let avoidance = ObstacleAvoidance::new();
        let checker = SimpleObstacleChecker::new();

        let output = avoidance.calculate_with_checker(
            &Vec3::new(0.0, 0.0, 0.0),
            &Vec3::new(1.0, 0.0, 0.0),
            &checker,
        );

        // No obstacles, no avoidance
        assert!(output.linear.x.abs() < 0.001);
        assert!(output.linear.z.abs() < 0.001);
    }

    #[test]
    fn test_obstacle_ahead() {
        let avoidance = ObstacleAvoidance::new().with_look_ahead(5.0);
        let mut checker = SimpleObstacleChecker::new();
        checker.add_obstacle(Vec3::new(3.0, 0.0, 0.0), 1.0);

        let output = avoidance.calculate_with_checker(
            &Vec3::new(0.0, 0.0, 0.0),
            &Vec3::new(5.0, 0.0, 0.0), // Moving toward obstacle
            &checker,
        );

        // Should produce avoidance force
        let magnitude = (output.linear.x * output.linear.x + output.linear.z * output.linear.z).sqrt();
        assert!(magnitude > 0.0);
    }

    #[test]
    fn test_obstacle_far_away() {
        let avoidance = ObstacleAvoidance::new().with_look_ahead(3.0);
        let mut checker = SimpleObstacleChecker::new();
        checker.add_obstacle(Vec3::new(10.0, 0.0, 0.0), 1.0);

        let output = avoidance.calculate_with_checker(
            &Vec3::new(0.0, 0.0, 0.0),
            &Vec3::new(1.0, 0.0, 0.0),
            &checker,
        );

        // Obstacle too far, no avoidance
        assert!(output.linear.x.abs() < 0.001);
        assert!(output.linear.z.abs() < 0.001);
    }
}

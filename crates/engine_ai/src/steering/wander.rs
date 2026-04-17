//! Wander steering behavior - random direction changes for natural movement.

use crate::behavior::blackboard::Vec3;
use super::{SteeringBehavior, SteeringOutput};

/// Wander behavior: smooth random direction changes
#[derive(Debug, Clone)]
pub struct Wander {
    /// Current wander angle (radians around Y axis)
    wander_angle: f32,
    /// Maximum angle change per update (radians)
    pub angle_change: f32,
    /// Distance ahead to project the wander circle
    pub circle_distance: f32,
    /// Radius of the wander circle
    pub circle_radius: f32,
    /// Maximum speed
    pub max_speed: f32,
    /// RNG seed for deterministic behavior
    seed: u64,
}

impl Wander {
    pub fn new() -> Self {
        Self {
            wander_angle: 0.0,
            angle_change: 0.5, // ~28 degrees max change
            circle_distance: 2.0,
            circle_radius: 1.0,
            max_speed: 3.0,
            seed: 12345,
        }
    }

    pub fn with_angle_change(mut self, angle: f32) -> Self {
        self.angle_change = angle;
        self
    }

    pub fn with_circle_distance(mut self, distance: f32) -> Self {
        self.circle_distance = distance;
        self
    }

    pub fn with_circle_radius(mut self, radius: f32) -> Self {
        self.circle_radius = radius;
        self
    }

    pub fn with_max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Simple pseudo-random number generator
    fn random(&mut self) -> f32 {
        // xorshift64
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        // Convert to 0..1 range
        (self.seed as f32) / (u64::MAX as f32)
    }

    /// Update wander angle with random displacement
    pub fn update(&mut self, dt: f32) {
        let random_val = self.random() * 2.0 - 1.0; // -1 to 1
        self.wander_angle += random_val * self.angle_change * dt * 10.0;

        // Keep angle in reasonable bounds
        if self.wander_angle > std::f32::consts::TAU {
            self.wander_angle -= std::f32::consts::TAU;
        }
        if self.wander_angle < 0.0 {
            self.wander_angle += std::f32::consts::TAU;
        }
    }

    /// Reset wander angle
    pub fn reset(&mut self) {
        self.wander_angle = 0.0;
    }
}

impl Default for Wander {
    fn default() -> Self {
        Self::new()
    }
}

impl SteeringBehavior for Wander {
    fn calculate(&self, position: &Vec3, velocity: &Vec3) -> SteeringOutput {
        // Get current facing direction from velocity, or use default
        let vel_magnitude = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();

        let (facing_x, facing_z) = if vel_magnitude > 0.001 {
            (velocity.x / vel_magnitude, velocity.z / vel_magnitude)
        } else {
            (1.0, 0.0) // Default facing +X
        };

        // Project circle center ahead of current position
        let circle_center_x = position.x + facing_x * self.circle_distance;
        let circle_center_z = position.z + facing_z * self.circle_distance;

        // Calculate displacement on the circle
        let displacement_x = self.wander_angle.cos() * self.circle_radius;
        let displacement_z = self.wander_angle.sin() * self.circle_radius;

        // Target point on circle
        let target_x = circle_center_x + displacement_x;
        let target_z = circle_center_z + displacement_z;

        // Direction to target
        let dx = target_x - position.x;
        let dz = target_z - position.z;
        let distance = (dx * dx + dz * dz).sqrt();

        if distance < 0.001 {
            return SteeringOutput::zero();
        }

        let nx = dx / distance;
        let nz = dz / distance;

        SteeringOutput::new(
            Vec3::new(nx * self.max_speed, 0.0, nz * self.max_speed),
            0.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wander_produces_movement() {
        let wander = Wander::new();
        let output = wander.calculate(
            &Vec3::new(0.0, 0.0, 0.0),
            &Vec3::new(1.0, 0.0, 0.0),
        );

        // Should produce some movement
        let magnitude = (output.linear.x * output.linear.x + output.linear.z * output.linear.z).sqrt();
        assert!(magnitude > 0.0);
    }

    #[test]
    fn test_wander_update_changes_direction() {
        let mut wander = Wander::new().with_seed(42);
        let initial_angle = wander.wander_angle;

        wander.update(0.1);

        // Angle should change after update
        assert!((wander.wander_angle - initial_angle).abs() > 0.0);
    }

    #[test]
    fn test_wander_deterministic() {
        let mut wander1 = Wander::new().with_seed(12345);
        let mut wander2 = Wander::new().with_seed(12345);

        wander1.update(0.1);
        wander2.update(0.1);

        // Same seed should produce same results
        assert!((wander1.wander_angle - wander2.wander_angle).abs() < 0.001);
    }
}

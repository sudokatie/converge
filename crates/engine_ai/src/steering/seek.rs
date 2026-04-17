//! Seek steering behavior - move toward a target position.

use crate::behavior::blackboard::Vec3;
use super::{SteeringBehavior, SteeringOutput};

/// Seek behavior: move toward a target position
#[derive(Debug, Clone)]
pub struct Seek {
    /// Target position to seek
    pub target: Vec3,
    /// Maximum speed
    pub max_speed: f32,
    /// Arrival radius - slow down when within this distance
    pub arrival_radius: f32,
    /// Stopping radius - stop when within this distance
    pub stop_radius: f32,
}

impl Seek {
    pub fn new(target: Vec3) -> Self {
        Self {
            target,
            max_speed: 5.0,
            arrival_radius: 3.0,
            stop_radius: 0.5,
        }
    }

    pub fn with_max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }

    pub fn with_arrival_radius(mut self, radius: f32) -> Self {
        self.arrival_radius = radius;
        self
    }

    pub fn with_stop_radius(mut self, radius: f32) -> Self {
        self.stop_radius = radius;
        self
    }

    /// Update the target position
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }
}

impl SteeringBehavior for Seek {
    fn calculate(&self, position: &Vec3, _velocity: &Vec3) -> SteeringOutput {
        // Direction to target
        let dx = self.target.x - position.x;
        let dy = self.target.y - position.y;
        let dz = self.target.z - position.z;

        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Already at target
        if distance < self.stop_radius {
            return SteeringOutput::zero();
        }

        // Normalize direction
        let nx = dx / distance;
        let ny = dy / distance;
        let nz = dz / distance;

        // Calculate speed with arrival slowdown
        let speed = if distance < self.arrival_radius {
            self.max_speed * (distance / self.arrival_radius)
        } else {
            self.max_speed
        };

        SteeringOutput::new(
            Vec3::new(nx * speed, ny * speed, nz * speed),
            0.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seek_toward_target() {
        let seek = Seek::new(Vec3::new(10.0, 0.0, 0.0));
        let output = seek.calculate(&Vec3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Should move in positive X direction
        assert!(output.linear.x > 0.0);
        assert!(output.linear.z.abs() < 0.001);
    }

    #[test]
    fn test_seek_at_target() {
        let seek = Seek::new(Vec3::new(0.0, 0.0, 0.0));
        let output = seek.calculate(&Vec3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Should not move
        assert!(output.linear.x.abs() < 0.001);
        assert!(output.linear.z.abs() < 0.001);
    }

    #[test]
    fn test_seek_arrival_slowdown() {
        let seek = Seek::new(Vec3::new(1.0, 0.0, 0.0))
            .with_arrival_radius(5.0)
            .with_max_speed(10.0);

        let output = seek.calculate(&Vec3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Within arrival radius, should be slower than max speed
        let speed = (output.linear.x * output.linear.x + output.linear.z * output.linear.z).sqrt();
        assert!(speed < 10.0);
    }
}

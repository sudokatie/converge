//! Flee steering behavior - move away from a target position.

use crate::behavior::blackboard::Vec3;
use super::{SteeringBehavior, SteeringOutput};

/// Flee behavior: move away from a target position
#[derive(Debug, Clone)]
pub struct Flee {
    /// Position to flee from
    pub threat: Vec3,
    /// Maximum speed
    pub max_speed: f32,
    /// Panic radius - flee at max speed within this distance
    pub panic_radius: f32,
    /// Safe radius - stop fleeing beyond this distance
    pub safe_radius: f32,
}

impl Flee {
    pub fn new(threat: Vec3) -> Self {
        Self {
            threat,
            max_speed: 6.0,
            panic_radius: 4.0,
            safe_radius: 16.0,
        }
    }

    pub fn with_max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }

    pub fn with_panic_radius(mut self, radius: f32) -> Self {
        self.panic_radius = radius;
        self
    }

    pub fn with_safe_radius(mut self, radius: f32) -> Self {
        self.safe_radius = radius;
        self
    }

    /// Update the threat position
    pub fn set_threat(&mut self, threat: Vec3) {
        self.threat = threat;
    }
}

impl SteeringBehavior for Flee {
    fn calculate(&self, position: &Vec3, _velocity: &Vec3) -> SteeringOutput {
        // Direction away from threat
        let dx = position.x - self.threat.x;
        let dy = position.y - self.threat.y;
        let dz = position.z - self.threat.z;

        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Beyond safe radius, no need to flee
        if distance > self.safe_radius {
            return SteeringOutput::zero();
        }

        // If at same position as threat, pick random direction
        if distance < 0.001 {
            return SteeringOutput::new(
                Vec3::new(self.max_speed, 0.0, 0.0),
                0.0,
            );
        }

        // Normalize direction
        let nx = dx / distance;
        let ny = dy / distance;
        let nz = dz / distance;

        // Calculate speed - faster when closer (within panic radius)
        let speed = if distance < self.panic_radius {
            self.max_speed
        } else {
            // Gradual slowdown between panic and safe radius
            let t = (self.safe_radius - distance) / (self.safe_radius - self.panic_radius);
            self.max_speed * t
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
    fn test_flee_from_threat() {
        let flee = Flee::new(Vec3::new(0.0, 0.0, 0.0));
        let output = flee.calculate(&Vec3::new(2.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Should move in positive X direction (away from origin)
        assert!(output.linear.x > 0.0);
    }

    #[test]
    fn test_flee_beyond_safe_radius() {
        let flee = Flee::new(Vec3::new(0.0, 0.0, 0.0)).with_safe_radius(10.0);
        let output = flee.calculate(&Vec3::new(20.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Should not move - beyond safe radius
        assert!(output.linear.x.abs() < 0.001);
    }

    #[test]
    fn test_flee_panic_speed() {
        let flee = Flee::new(Vec3::new(0.0, 0.0, 0.0))
            .with_panic_radius(5.0)
            .with_max_speed(10.0);

        let output = flee.calculate(&Vec3::new(2.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));

        // Within panic radius, should be at max speed
        let speed = (output.linear.x * output.linear.x + output.linear.z * output.linear.z).sqrt();
        assert!((speed - 10.0).abs() < 0.001);
    }
}

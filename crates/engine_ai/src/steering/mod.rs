//! Steering behaviors for AI navigation.
//!
//! Implements spec 8.2.2 - steering behaviors for smooth movement.

pub mod seek;
pub mod flee;
pub mod wander;
pub mod avoidance;
pub mod combined;

pub use seek::Seek;
pub use flee::Flee;
pub use wander::Wander;
pub use avoidance::ObstacleAvoidance;
pub use combined::{SteeringCombiner, SteeringWeight};

use crate::behavior::blackboard::Vec3;

/// Output of a steering behavior
#[derive(Debug, Clone, Copy, Default)]
pub struct SteeringOutput {
    /// Linear velocity/force to apply
    pub linear: Vec3,
    /// Angular velocity (rotation around Y axis)
    pub angular: f32,
}

impl SteeringOutput {
    pub fn new(linear: Vec3, angular: f32) -> Self {
        Self { linear, angular }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    /// Scale the steering output
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            linear: Vec3::new(
                self.linear.x * factor,
                self.linear.y * factor,
                self.linear.z * factor,
            ),
            angular: self.angular * factor,
        }
    }

    /// Add two steering outputs
    pub fn add(&self, other: &SteeringOutput) -> Self {
        Self {
            linear: Vec3::new(
                self.linear.x + other.linear.x,
                self.linear.y + other.linear.y,
                self.linear.z + other.linear.z,
            ),
            angular: self.angular + other.angular,
        }
    }

    /// Clamp the linear magnitude to a maximum
    pub fn clamp_linear(&self, max_speed: f32) -> Self {
        let magnitude = (self.linear.x * self.linear.x
            + self.linear.y * self.linear.y
            + self.linear.z * self.linear.z)
            .sqrt();

        if magnitude > max_speed && magnitude > 0.0 {
            let factor = max_speed / magnitude;
            Self {
                linear: Vec3::new(
                    self.linear.x * factor,
                    self.linear.y * factor,
                    self.linear.z * factor,
                ),
                angular: self.angular,
            }
        } else {
            *self
        }
    }
}

/// Trait for all steering behaviors
pub trait SteeringBehavior {
    /// Calculate steering output given current position and velocity
    fn calculate(&self, position: &Vec3, velocity: &Vec3) -> SteeringOutput;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steering_output_scale() {
        let output = SteeringOutput::new(Vec3::new(2.0, 0.0, 2.0), 1.0);
        let scaled = output.scale(0.5);
        assert!((scaled.linear.x - 1.0).abs() < 0.001);
        assert!((scaled.angular - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_steering_output_clamp() {
        let output = SteeringOutput::new(Vec3::new(10.0, 0.0, 0.0), 0.0);
        let clamped = output.clamp_linear(5.0);
        let magnitude = (clamped.linear.x * clamped.linear.x
            + clamped.linear.y * clamped.linear.y
            + clamped.linear.z * clamped.linear.z)
            .sqrt();
        assert!((magnitude - 5.0).abs() < 0.001);
    }
}

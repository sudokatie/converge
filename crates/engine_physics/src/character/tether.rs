//! Tether state and physics for constrained movement.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// State of an active tether connection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TetherState {
    /// World position of the tether anchor point.
    pub anchor: Vec3,
    /// Current tether length.
    pub length: f32,
    /// Target length (for reeling).
    pub target_length: f32,
    /// Whether the tether is taut (at max extension).
    pub taut: bool,
}

impl TetherState {
    /// Create a new tether attached to an anchor point.
    #[must_use]
    pub fn new(anchor: Vec3, length: f32) -> Self {
        Self {
            anchor,
            length,
            target_length: length,
            taut: false,
        }
    }

    /// Create a tether with the anchor at the given position relative to character.
    #[must_use]
    pub fn from_character_position(character_pos: Vec3, anchor: Vec3) -> Self {
        let length = (anchor - character_pos).length();
        Self::new(anchor, length)
    }

    /// Get the vector from anchor to a position.
    #[must_use]
    pub fn to_position(&self, pos: Vec3) -> Vec3 {
        pos - self.anchor
    }

    /// Get the distance from anchor to a position.
    #[must_use]
    pub fn distance_to(&self, pos: Vec3) -> f32 {
        self.to_position(pos).length()
    }

    /// Check if a position exceeds the tether length.
    #[must_use]
    pub fn exceeds_length(&self, pos: Vec3) -> bool {
        self.distance_to(pos) > self.length
    }

    /// Constrain a position to be within tether length.
    #[must_use]
    pub fn constrain_position(&self, pos: Vec3) -> Vec3 {
        let to_pos = self.to_position(pos);
        let distance = to_pos.length();
        if distance > self.length && distance > 0.0001 {
            self.anchor + to_pos * (self.length / distance)
        } else {
            pos
        }
    }

    /// Apply tether constraint force to velocity.
    #[must_use]
    pub fn constrain_velocity(&self, pos: Vec3, velocity: Vec3, stiffness: f32, dt: f32) -> Vec3 {
        let to_pos = self.to_position(pos);
        let distance = to_pos.length();

        if distance <= 0.0001 {
            return velocity;
        }

        let direction = to_pos / distance;

        if distance > self.length {
            let overshoot = distance - self.length;
            let radial_velocity = velocity.dot(direction);

            if radial_velocity > 0.0 {
                let correction = direction * (radial_velocity + overshoot * stiffness * dt);
                velocity - correction
            } else {
                let correction = direction * (overshoot * stiffness * dt);
                velocity - correction
            }
        } else {
            velocity
        }
    }

    /// Update target length for reeling.
    pub fn set_target_length(&mut self, target: f32, min_length: f32, max_length: f32) {
        self.target_length = target.clamp(min_length, max_length);
    }

    /// Reel the tether toward target length.
    pub fn reel(&mut self, reel_speed: f32, min_length: f32, max_length: f32, dt: f32) {
        let diff = self.target_length - self.length;
        let max_change = reel_speed * dt;

        if diff.abs() <= max_change {
            self.length = self.target_length;
        } else if diff > 0.0 {
            self.length += max_change;
        } else {
            self.length -= max_change;
        }

        self.length = self.length.clamp(min_length, max_length);
    }

    /// Extend tether by a delta amount.
    pub fn extend(&mut self, delta: f32, max_length: f32) {
        self.length = (self.length + delta).min(max_length);
        self.target_length = self.length;
    }

    /// Retract tether by a delta amount.
    pub fn retract(&mut self, delta: f32, min_length: f32) {
        self.length = (self.length - delta).max(min_length);
        self.target_length = self.length;
    }
}

impl Default for TetherState {
    fn default() -> Self {
        Self {
            anchor: Vec3::ZERO,
            length: 10.0,
            target_length: 10.0,
            taut: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tether_creation() {
        let tether = TetherState::new(Vec3::new(0.0, 10.0, 0.0), 5.0);
        assert_eq!(tether.anchor, Vec3::new(0.0, 10.0, 0.0));
        assert!((tether.length - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tether_from_character() {
        let tether = TetherState::from_character_position(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        );
        assert!((tether.length - 10.0).abs() < 0.001);
    }

    #[test]
    fn exceeds_length() {
        let tether = TetherState::new(Vec3::ZERO, 5.0);
        assert!(!tether.exceeds_length(Vec3::new(3.0, 0.0, 0.0)));
        assert!(tether.exceeds_length(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn constrain_position() {
        let tether = TetherState::new(Vec3::ZERO, 5.0);
        let constrained = tether.constrain_position(Vec3::new(10.0, 0.0, 0.0));
        assert!((constrained.x - 5.0).abs() < 0.001);
    }

    #[test]
    fn constrain_position_within_length() {
        let tether = TetherState::new(Vec3::ZERO, 10.0);
        let pos = Vec3::new(3.0, 0.0, 0.0);
        let constrained = tether.constrain_position(pos);
        assert_eq!(constrained, pos);
    }

    #[test]
    fn constrain_velocity_away() {
        let tether = TetherState::new(Vec3::ZERO, 5.0);
        let pos = Vec3::new(6.0, 0.0, 0.0);
        let vel = Vec3::new(10.0, 0.0, 0.0);
        let constrained = tether.constrain_velocity(pos, vel, 100.0, 0.016);
        assert!(constrained.x < vel.x);
    }

    #[test]
    fn constrain_velocity_within() {
        let tether = TetherState::new(Vec3::ZERO, 10.0);
        let pos = Vec3::new(5.0, 0.0, 0.0);
        let vel = Vec3::new(1.0, 2.0, 3.0);
        let constrained = tether.constrain_velocity(pos, vel, 100.0, 0.016);
        assert_eq!(constrained, vel);
    }

    #[test]
    fn reel_in() {
        let mut tether = TetherState::new(Vec3::ZERO, 10.0);
        tether.set_target_length(5.0, 1.0, 20.0);
        tether.reel(2.0, 1.0, 20.0, 1.0);
        assert!((tether.length - 8.0).abs() < 0.001);
    }

    #[test]
    fn reel_out() {
        let mut tether = TetherState::new(Vec3::ZERO, 5.0);
        tether.set_target_length(10.0, 1.0, 20.0);
        tether.reel(2.0, 1.0, 20.0, 1.0);
        assert!((tether.length - 7.0).abs() < 0.001);
    }

    #[test]
    fn extend_clamps_to_max() {
        let mut tether = TetherState::new(Vec3::ZERO, 18.0);
        tether.extend(5.0, 20.0);
        assert!((tether.length - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn retract_clamps_to_min() {
        let mut tether = TetherState::new(Vec3::ZERO, 3.0);
        tether.retract(5.0, 1.0);
        assert!((tether.length - 1.0).abs() < f32::EPSILON);
    }
}

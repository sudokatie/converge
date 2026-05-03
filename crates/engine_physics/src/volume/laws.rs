//! Physics laws configuration for volumes.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Custom physics parameters applied within a volume.
///
/// All fields are optional overrides. When `None`, the global/default
/// physics parameters are used for that property.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhysicsLaws {
    /// Gravity vector override.
    pub gravity: Option<Vec3>,
    /// Linear drag coefficient (velocity damping per second).
    pub drag: Option<f32>,
    /// Angular damping coefficient.
    pub angular_damping: Option<f32>,
    /// Buoyancy factor (0 = no buoyancy, 1 = neutral, >1 = floats up).
    pub buoyancy: Option<f32>,
    /// Terminal velocity cap (maximum speed).
    pub terminal_velocity: Option<f32>,
    /// Friction coefficient multiplier.
    pub friction: Option<f32>,
    /// Time scale for physics simulation (1.0 = normal, 0.5 = slow-mo).
    pub time_scale: Option<f32>,
    /// Restitution (bounciness) multiplier.
    pub restitution: Option<f32>,
}

impl PhysicsLaws {
    /// Creates empty physics laws (all defaults).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            gravity: None,
            drag: None,
            angular_damping: None,
            buoyancy: None,
            terminal_velocity: None,
            friction: None,
            time_scale: None,
            restitution: None,
        }
    }

    /// Creates physics laws for a low-gravity zone.
    #[must_use]
    pub fn low_gravity() -> Self {
        Self {
            gravity: Some(Vec3::new(0.0, -2.0, 0.0)),
            ..Default::default()
        }
    }

    /// Creates physics laws for zero gravity.
    #[must_use]
    pub fn zero_gravity() -> Self {
        Self {
            gravity: Some(Vec3::ZERO),
            drag: Some(0.1),
            ..Default::default()
        }
    }

    /// Creates physics laws for underwater environments.
    #[must_use]
    pub fn underwater() -> Self {
        Self {
            gravity: Some(Vec3::new(0.0, -4.0, 0.0)),
            drag: Some(2.0),
            angular_damping: Some(1.5),
            buoyancy: Some(0.8),
            terminal_velocity: Some(10.0),
            ..Default::default()
        }
    }

    /// Creates physics laws for high-drag environments (mud, quicksand).
    #[must_use]
    pub fn high_drag() -> Self {
        Self {
            drag: Some(5.0),
            angular_damping: Some(3.0),
            terminal_velocity: Some(3.0),
            friction: Some(2.0),
            ..Default::default()
        }
    }

    /// Creates physics laws for a time-slowed zone.
    #[must_use]
    pub fn slow_motion(scale: f32) -> Self {
        Self {
            time_scale: Some(scale),
            ..Default::default()
        }
    }

    /// Creates physics laws for a bouncy zone.
    #[must_use]
    pub fn bouncy() -> Self {
        Self {
            restitution: Some(1.5),
            ..Default::default()
        }
    }

    /// Builder: sets gravity override.
    #[must_use]
    pub const fn with_gravity(mut self, gravity: Vec3) -> Self {
        self.gravity = Some(gravity);
        self
    }

    /// Builder: sets drag coefficient.
    #[must_use]
    pub const fn with_drag(mut self, drag: f32) -> Self {
        self.drag = Some(drag);
        self
    }

    /// Builder: sets angular damping.
    #[must_use]
    pub const fn with_angular_damping(mut self, damping: f32) -> Self {
        self.angular_damping = Some(damping);
        self
    }

    /// Builder: sets buoyancy factor.
    #[must_use]
    pub const fn with_buoyancy(mut self, buoyancy: f32) -> Self {
        self.buoyancy = Some(buoyancy);
        self
    }

    /// Builder: sets terminal velocity.
    #[must_use]
    pub const fn with_terminal_velocity(mut self, velocity: f32) -> Self {
        self.terminal_velocity = Some(velocity);
        self
    }

    /// Builder: sets friction multiplier.
    #[must_use]
    pub const fn with_friction(mut self, friction: f32) -> Self {
        self.friction = Some(friction);
        self
    }

    /// Builder: sets time scale.
    #[must_use]
    pub const fn with_time_scale(mut self, scale: f32) -> Self {
        self.time_scale = Some(scale);
        self
    }

    /// Builder: sets restitution multiplier.
    #[must_use]
    pub const fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = Some(restitution);
        self
    }

    /// Returns whether any physics property is overridden.
    #[must_use]
    pub const fn has_overrides(&self) -> bool {
        self.gravity.is_some()
            || self.drag.is_some()
            || self.angular_damping.is_some()
            || self.buoyancy.is_some()
            || self.terminal_velocity.is_some()
            || self.friction.is_some()
            || self.time_scale.is_some()
            || self.restitution.is_some()
    }

    /// Blends two physics laws by weight (0 = self, 1 = other).
    #[must_use]
    pub fn blend(&self, other: &PhysicsLaws, weight: f32) -> PhysicsLaws {
        let blend_f32 = |a: Option<f32>, b: Option<f32>| -> Option<f32> {
            match (a, b) {
                (Some(va), Some(vb)) => Some(va + (vb - va) * weight),
                (Some(v), None) => Some(v * (1.0 - weight)),
                (None, Some(v)) => Some(v * weight),
                (None, None) => None,
            }
        };

        let blend_vec3 = |a: Option<Vec3>, b: Option<Vec3>| -> Option<Vec3> {
            match (a, b) {
                (Some(va), Some(vb)) => Some(va + (vb - va) * weight),
                (Some(v), None) => Some(v * (1.0 - weight)),
                (None, Some(v)) => Some(v * weight),
                (None, None) => None,
            }
        };

        PhysicsLaws {
            gravity: blend_vec3(self.gravity, other.gravity),
            drag: blend_f32(self.drag, other.drag),
            angular_damping: blend_f32(self.angular_damping, other.angular_damping),
            buoyancy: blend_f32(self.buoyancy, other.buoyancy),
            terminal_velocity: blend_f32(self.terminal_velocity, other.terminal_velocity),
            friction: blend_f32(self.friction, other.friction),
            time_scale: blend_f32(self.time_scale, other.time_scale),
            restitution: blend_f32(self.restitution, other.restitution),
        }
    }

    /// Merges another set of laws, using `other`'s values where present.
    #[must_use]
    pub fn merge(&self, other: &PhysicsLaws) -> PhysicsLaws {
        PhysicsLaws {
            gravity: other.gravity.or(self.gravity),
            drag: other.drag.or(self.drag),
            angular_damping: other.angular_damping.or(self.angular_damping),
            buoyancy: other.buoyancy.or(self.buoyancy),
            terminal_velocity: other.terminal_velocity.or(self.terminal_velocity),
            friction: other.friction.or(self.friction),
            time_scale: other.time_scale.or(self.time_scale),
            restitution: other.restitution.or(self.restitution),
        }
    }

    /// Applies default values for any unset properties.
    #[must_use]
    pub fn with_defaults(self, defaults: &PhysicsLaws) -> PhysicsLaws {
        PhysicsLaws {
            gravity: self.gravity.or(defaults.gravity),
            drag: self.drag.or(defaults.drag),
            angular_damping: self.angular_damping.or(defaults.angular_damping),
            buoyancy: self.buoyancy.or(defaults.buoyancy),
            terminal_velocity: self.terminal_velocity.or(defaults.terminal_velocity),
            friction: self.friction.or(defaults.friction),
            time_scale: self.time_scale.or(defaults.time_scale),
            restitution: self.restitution.or(defaults.restitution),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn assert_vec3_eq(a: Vec3, b: Vec3) {
        assert_relative_eq!(a.x, b.x, epsilon = 1e-6);
        assert_relative_eq!(a.y, b.y, epsilon = 1e-6);
        assert_relative_eq!(a.z, b.z, epsilon = 1e-6);
    }

    #[test]
    fn default_has_no_overrides() {
        let laws = PhysicsLaws::default();
        assert!(!laws.has_overrides());
    }

    #[test]
    fn preset_low_gravity() {
        let laws = PhysicsLaws::low_gravity();
        assert!(laws.has_overrides());
        assert!(laws.gravity.is_some());
        assert_relative_eq!(laws.gravity.unwrap().y, -2.0);
    }

    #[test]
    fn preset_underwater() {
        let laws = PhysicsLaws::underwater();
        assert!(laws.gravity.is_some());
        assert!(laws.drag.is_some());
        assert!(laws.buoyancy.is_some());
        assert_relative_eq!(laws.buoyancy.unwrap(), 0.8);
    }

    #[test]
    fn builder_chain() {
        let laws = PhysicsLaws::default()
            .with_gravity(Vec3::new(0.0, -5.0, 0.0))
            .with_drag(1.5)
            .with_time_scale(0.5);

        assert_vec3_eq(laws.gravity.unwrap(), Vec3::new(0.0, -5.0, 0.0));
        assert_relative_eq!(laws.drag.unwrap(), 1.5);
        assert_relative_eq!(laws.time_scale.unwrap(), 0.5);
    }

    #[test]
    fn blend_half() {
        let a = PhysicsLaws::default().with_drag(0.0);
        let b = PhysicsLaws::default().with_drag(2.0);
        let blended = a.blend(&b, 0.5);
        assert_relative_eq!(blended.drag.unwrap(), 1.0);
    }

    #[test]
    fn blend_gravity() {
        let a = PhysicsLaws::default().with_gravity(Vec3::new(0.0, -10.0, 0.0));
        let b = PhysicsLaws::default().with_gravity(Vec3::new(0.0, 0.0, 0.0));
        let blended = a.blend(&b, 0.5);
        assert_vec3_eq(blended.gravity.unwrap(), Vec3::new(0.0, -5.0, 0.0));
    }

    #[test]
    fn merge_laws() {
        let base = PhysicsLaws::default().with_gravity(Vec3::Y).with_drag(1.0);
        let override_laws = PhysicsLaws::default().with_drag(2.0);
        let merged = base.merge(&override_laws);

        assert_vec3_eq(merged.gravity.unwrap(), Vec3::Y);
        assert_relative_eq!(merged.drag.unwrap(), 2.0);
    }

    #[test]
    fn with_defaults() {
        let laws = PhysicsLaws::default().with_drag(1.0);
        let defaults = PhysicsLaws::default()
            .with_gravity(Vec3::NEG_Y * 9.8)
            .with_drag(0.5);
        let result = laws.with_defaults(&defaults);

        assert_vec3_eq(result.gravity.unwrap(), Vec3::NEG_Y * 9.8);
        assert_relative_eq!(result.drag.unwrap(), 1.0);
    }

    #[test]
    fn serialization() {
        let laws = PhysicsLaws::underwater();
        let json = serde_json::to_string(&laws).unwrap();
        let recovered: PhysicsLaws = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.buoyancy, laws.buoyancy);
        assert_eq!(recovered.drag, laws.drag);
    }

    #[test]
    fn bincode_serialization() {
        let laws = PhysicsLaws::high_drag();
        let bytes = bincode::serialize(&laws).unwrap();
        let recovered: PhysicsLaws = bincode::deserialize(&bytes).unwrap();
        assert_eq!(recovered.drag, laws.drag);
    }
}

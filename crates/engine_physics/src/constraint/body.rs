//! Body identifiers and state snapshots for constraint solving.
//!
//! Provides lightweight representations of rigid body state used during
//! constraint iteration without requiring full body references.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Unique identifier for a physics body.
///
/// Uses `u64` consistent with other entity identifiers in the engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyId(pub u64);

impl BodyId {
    /// Creates a new body identifier.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl From<u64> for BodyId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<BodyId> for u64 {
    fn from(id: BodyId) -> Self {
        id.0
    }
}

/// Snapshot of body state for constraint solving.
///
/// Contains position, orientation, velocities, and inverse mass/inertia
/// needed for positional and velocity corrections.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodySnapshot {
    /// World-space position of the body's center of mass.
    pub position: Vec3,
    /// Orientation as a unit quaternion.
    pub orientation: Quat,
    /// Linear velocity in world space.
    pub linear_velocity: Vec3,
    /// Angular velocity in world space (radians per second).
    pub angular_velocity: Vec3,
    /// Inverse mass (0 for static bodies).
    pub inv_mass: f32,
    /// Inverse inertia tensor diagonal (simplified).
    pub inv_inertia: Vec3,
}

impl Default for BodySnapshot {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            inv_mass: 1.0,
            inv_inertia: Vec3::ONE,
        }
    }
}

impl BodySnapshot {
    /// Creates a new body snapshot with the given position.
    #[must_use]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Creates a static body snapshot (infinite mass).
    #[must_use]
    pub fn new_static(position: Vec3) -> Self {
        Self {
            position,
            inv_mass: 0.0,
            inv_inertia: Vec3::ZERO,
            ..Default::default()
        }
    }

    /// Builder: sets position.
    #[must_use]
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Builder: sets orientation.
    #[must_use]
    pub fn with_orientation(mut self, orientation: Quat) -> Self {
        self.orientation = orientation;
        self
    }

    /// Builder: sets linear velocity.
    #[must_use]
    pub fn with_linear_velocity(mut self, velocity: Vec3) -> Self {
        self.linear_velocity = velocity;
        self
    }

    /// Builder: sets angular velocity.
    #[must_use]
    pub fn with_angular_velocity(mut self, velocity: Vec3) -> Self {
        self.angular_velocity = velocity;
        self
    }

    /// Builder: sets mass (converts to inverse mass internally).
    #[must_use]
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        self
    }

    /// Builder: sets inverse mass directly.
    #[must_use]
    pub fn with_inv_mass(mut self, inv_mass: f32) -> Self {
        self.inv_mass = inv_mass;
        self
    }

    /// Builder: sets inertia tensor diagonal (converts to inverse).
    #[must_use]
    pub fn with_inertia(mut self, inertia: Vec3) -> Self {
        self.inv_inertia = Vec3::new(
            if inertia.x > 0.0 {
                1.0 / inertia.x
            } else {
                0.0
            },
            if inertia.y > 0.0 {
                1.0 / inertia.y
            } else {
                0.0
            },
            if inertia.z > 0.0 {
                1.0 / inertia.z
            } else {
                0.0
            },
        );
        self
    }

    /// Returns whether this body is static (infinite mass).
    #[must_use]
    pub fn is_static(&self) -> bool {
        self.inv_mass == 0.0
    }

    /// Returns the mass, or `f32::INFINITY` for static bodies.
    #[must_use]
    pub fn mass(&self) -> f32 {
        if self.inv_mass > 0.0 {
            1.0 / self.inv_mass
        } else {
            f32::INFINITY
        }
    }

    /// Transforms a local-space point to world space.
    #[must_use]
    pub fn local_to_world(&self, local_point: Vec3) -> Vec3 {
        self.position + self.orientation * local_point
    }

    /// Transforms a world-space point to local space.
    #[must_use]
    pub fn world_to_local(&self, world_point: Vec3) -> Vec3 {
        self.orientation.inverse() * (world_point - self.position)
    }

    /// Computes the velocity at a world-space point on this body.
    #[must_use]
    pub fn velocity_at_point(&self, world_point: Vec3) -> Vec3 {
        let r = world_point - self.position;
        self.linear_velocity + self.angular_velocity.cross(r)
    }

    /// Applies a positional correction impulse at the given world point.
    pub fn apply_position_correction(&mut self, world_point: Vec3, impulse: Vec3) {
        self.position += impulse * self.inv_mass;
        let r = world_point - self.position;
        let angular_impulse = r.cross(impulse);
        let angular_correction = self.inv_inertia * angular_impulse;
        let delta_rotation = Quat::from_scaled_axis(angular_correction * 0.5);
        self.orientation = (delta_rotation * self.orientation).normalize();
    }

    /// Applies a velocity correction impulse at the given world point.
    pub fn apply_velocity_correction(&mut self, world_point: Vec3, impulse: Vec3) {
        self.linear_velocity += impulse * self.inv_mass;
        let r = world_point - self.position;
        self.angular_velocity += self.inv_inertia * r.cross(impulse);
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
    fn body_id_roundtrip() {
        let id = BodyId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(u64::from(id), 42);
        assert_eq!(BodyId::from(42u64), id);
    }

    #[test]
    fn body_id_serialization() {
        let id = BodyId::new(123);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: BodyId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, id);
    }

    #[test]
    fn static_body_has_zero_inv_mass() {
        let body = BodySnapshot::new_static(Vec3::ZERO);
        assert!(body.is_static());
        assert_relative_eq!(body.inv_mass, 0.0);
        assert!(body.mass().is_infinite());
    }

    #[test]
    fn dynamic_body_mass_conversion() {
        let body = BodySnapshot::new(Vec3::ZERO).with_mass(4.0);
        assert!(!body.is_static());
        assert_relative_eq!(body.inv_mass, 0.25);
        assert_relative_eq!(body.mass(), 4.0);
    }

    #[test]
    fn local_to_world_identity() {
        let body = BodySnapshot::new(Vec3::new(1.0, 2.0, 3.0));
        let local = Vec3::new(1.0, 0.0, 0.0);
        let world = body.local_to_world(local);
        assert_vec3_eq(world, Vec3::new(2.0, 2.0, 3.0));
    }

    #[test]
    fn local_to_world_with_rotation() {
        use std::f32::consts::FRAC_PI_2;
        let body = BodySnapshot::new(Vec3::ZERO).with_orientation(Quat::from_rotation_z(FRAC_PI_2));
        let local = Vec3::new(1.0, 0.0, 0.0);
        let world = body.local_to_world(local);
        assert_relative_eq!(world.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(world.y, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn velocity_at_point_linear_only() {
        let body = BodySnapshot::new(Vec3::ZERO).with_linear_velocity(Vec3::new(1.0, 0.0, 0.0));
        let vel = body.velocity_at_point(Vec3::new(0.0, 1.0, 0.0));
        assert_vec3_eq(vel, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn velocity_at_point_with_angular() {
        let body = BodySnapshot::new(Vec3::ZERO).with_angular_velocity(Vec3::new(0.0, 0.0, 1.0));
        let vel = body.velocity_at_point(Vec3::new(1.0, 0.0, 0.0));
        assert_relative_eq!(vel.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(vel.y, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn snapshot_serialization() {
        let body = BodySnapshot::new(Vec3::new(1.0, 2.0, 3.0))
            .with_mass(5.0)
            .with_linear_velocity(Vec3::X);
        let json = serde_json::to_string(&body).unwrap();
        let recovered: BodySnapshot = serde_json::from_str(&json).unwrap();
        assert_vec3_eq(recovered.position, body.position);
        assert_relative_eq!(recovered.inv_mass, body.inv_mass, epsilon = 1e-6);
    }
}

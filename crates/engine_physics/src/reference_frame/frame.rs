//! Reference frame representation for moving platforms and dynamic bodies.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// A reference frame representing a coordinate system that may be moving
/// relative to the world.
///
/// Used for platforms, vehicles, rotating structures, and any large dynamic
/// body that characters or objects can ride on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceFrame {
    /// Position of the frame's origin in world coordinates.
    pub origin: Vec3,
    /// Orientation of the frame (local-to-world rotation).
    pub orientation: Quat,
    /// Linear velocity in world coordinates.
    pub linear_velocity: Vec3,
    /// Angular velocity (axis-angle, magnitude is radians/second) in world coordinates.
    pub angular_velocity: Vec3,
    /// Linear acceleration in world coordinates.
    pub linear_acceleration: Vec3,
    /// Optional parent frame ID for nested frame hierarchies.
    pub parent: Option<u64>,
}

impl Default for ReferenceFrame {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl ReferenceFrame {
    /// A stationary frame at the world origin.
    pub const IDENTITY: Self = Self {
        origin: Vec3::ZERO,
        orientation: Quat::IDENTITY,
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
        linear_acceleration: Vec3::ZERO,
        parent: None,
    };

    /// Create a new frame at the given position.
    #[must_use]
    pub fn at_position(origin: Vec3) -> Self {
        Self {
            origin,
            ..Self::IDENTITY
        }
    }

    /// Create a frame with position and velocity.
    #[must_use]
    pub fn with_velocity(origin: Vec3, linear_velocity: Vec3) -> Self {
        Self {
            origin,
            linear_velocity,
            ..Self::IDENTITY
        }
    }

    /// Create a frame with full linear motion state.
    #[must_use]
    pub fn with_linear_motion(
        origin: Vec3,
        linear_velocity: Vec3,
        linear_acceleration: Vec3,
    ) -> Self {
        Self {
            origin,
            linear_velocity,
            linear_acceleration,
            ..Self::IDENTITY
        }
    }

    /// Create a rotating frame.
    #[must_use]
    pub fn rotating(origin: Vec3, orientation: Quat, angular_velocity: Vec3) -> Self {
        Self {
            origin,
            orientation,
            angular_velocity,
            ..Self::IDENTITY
        }
    }

    /// Builder: set the orientation.
    #[must_use]
    pub fn with_orientation(mut self, orientation: Quat) -> Self {
        self.orientation = orientation.normalize();
        self
    }

    /// Builder: set angular velocity.
    #[must_use]
    pub fn with_angular_velocity(mut self, angular_velocity: Vec3) -> Self {
        self.angular_velocity = angular_velocity;
        self
    }

    /// Builder: set linear acceleration.
    #[must_use]
    pub fn with_acceleration(mut self, linear_acceleration: Vec3) -> Self {
        self.linear_acceleration = linear_acceleration;
        self
    }

    /// Builder: set parent frame.
    #[must_use]
    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent = Some(parent_id);
        self
    }

    /// Check if this frame is stationary (no velocity or acceleration).
    #[must_use]
    pub fn is_stationary(&self) -> bool {
        self.linear_velocity.length_squared() < 1e-10
            && self.angular_velocity.length_squared() < 1e-10
            && self.linear_acceleration.length_squared() < 1e-10
    }

    /// Check if this frame has rotation (non-identity orientation or angular velocity).
    #[must_use]
    pub fn is_rotating(&self) -> bool {
        self.angular_velocity.length_squared() > 1e-10
            || (self.orientation - Quat::IDENTITY).length_squared() > 1e-10
    }

    /// Get the velocity at a specific point in the frame due to combined
    /// linear and angular motion.
    #[must_use]
    pub fn velocity_at_point(&self, world_point: Vec3) -> Vec3 {
        let r = world_point - self.origin;
        self.linear_velocity + self.angular_velocity.cross(r)
    }

    /// Transform a position from local frame coordinates to world coordinates.
    #[must_use]
    pub fn local_to_world_position(&self, local_pos: Vec3) -> Vec3 {
        self.origin + self.orientation * local_pos
    }

    /// Transform a position from world coordinates to local frame coordinates.
    #[must_use]
    pub fn world_to_local_position(&self, world_pos: Vec3) -> Vec3 {
        self.orientation.inverse() * (world_pos - self.origin)
    }

    /// Transform a direction from local frame to world coordinates.
    #[must_use]
    pub fn local_to_world_direction(&self, local_dir: Vec3) -> Vec3 {
        self.orientation * local_dir
    }

    /// Transform a direction from world to local frame coordinates.
    #[must_use]
    pub fn world_to_local_direction(&self, world_dir: Vec3) -> Vec3 {
        self.orientation.inverse() * world_dir
    }

    /// Transform a velocity from local frame coordinates to world coordinates.
    ///
    /// Accounts for both frame velocity and rotational effects at the given
    /// local position.
    #[must_use]
    pub fn local_to_world_velocity(&self, local_pos: Vec3, local_vel: Vec3) -> Vec3 {
        let world_pos = self.local_to_world_position(local_pos);
        let frame_vel = self.velocity_at_point(world_pos);
        let rotated_vel = self.orientation * local_vel;
        frame_vel + rotated_vel
    }

    /// Transform a velocity from world coordinates to local frame coordinates.
    ///
    /// Subtracts the frame's contribution at the given world position.
    #[must_use]
    pub fn world_to_local_velocity(&self, world_pos: Vec3, world_vel: Vec3) -> Vec3 {
        let frame_vel = self.velocity_at_point(world_pos);
        let relative_vel = world_vel - frame_vel;
        self.orientation.inverse() * relative_vel
    }

    /// Advance the frame by one time step using current velocities and accelerations.
    pub fn integrate(&mut self, dt: f32) {
        self.linear_velocity += self.linear_acceleration * dt;
        self.origin += self.linear_velocity * dt;

        if self.angular_velocity.length_squared() > 1e-10 {
            let angle = self.angular_velocity.length() * dt;
            let axis = self.angular_velocity.normalize_or_zero();
            let rotation = Quat::from_axis_angle(axis, angle);
            self.orientation = (rotation * self.orientation).normalize();
        }
    }

    /// Compute the pseudo-forces (fictitious forces) experienced at a point
    /// in this frame: centrifugal and linear acceleration.
    ///
    /// Returns a force vector that should be added to gravity when simulating
    /// objects in this frame.
    #[must_use]
    pub fn pseudo_force_at_point(&self, world_point: Vec3, mass: f32) -> Vec3 {
        let r = world_point - self.origin;

        let centrifugal = self.angular_velocity.cross(self.angular_velocity.cross(r));

        -mass * (self.linear_acceleration + centrifugal)
    }

    /// Compute the Coriolis force for an object moving within this frame.
    ///
    /// The Coriolis force is velocity-dependent and perpendicular to motion.
    #[must_use]
    pub fn coriolis_force(&self, relative_velocity: Vec3, mass: f32) -> Vec3 {
        -2.0 * mass * self.angular_velocity.cross(relative_velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-5;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    #[test]
    fn identity_frame() {
        let frame = ReferenceFrame::IDENTITY;
        assert!(frame.is_stationary());
        assert!(!frame.is_rotating());
        assert_eq!(frame.origin, Vec3::ZERO);
    }

    #[test]
    fn stationary_transforms() {
        let frame = ReferenceFrame::at_position(Vec3::new(10.0, 0.0, 0.0));

        let local = Vec3::new(1.0, 2.0, 3.0);
        let world = frame.local_to_world_position(local);
        assert!(approx_eq_vec3(world, Vec3::new(11.0, 2.0, 3.0)));

        let back = frame.world_to_local_position(world);
        assert!(approx_eq_vec3(back, local));
    }

    #[test]
    fn rotated_frame_transforms() {
        let frame = ReferenceFrame::at_position(Vec3::ZERO)
            .with_orientation(Quat::from_rotation_y(PI / 2.0));

        let local = Vec3::new(1.0, 0.0, 0.0);
        let world = frame.local_to_world_position(local);
        assert!(approx_eq_vec3(world, Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn velocity_at_point_linear() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        let vel = frame.velocity_at_point(Vec3::new(10.0, 0.0, 0.0));
        assert!(approx_eq_vec3(vel, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn velocity_at_point_rotating() {
        let frame = ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, 1.0, 0.0));
        let vel = frame.velocity_at_point(Vec3::new(1.0, 0.0, 0.0));
        assert!(approx_eq_vec3(vel, Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn local_to_world_velocity_moving_frame() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let local_vel = Vec3::new(0.0, 5.0, 0.0);
        let world_vel = frame.local_to_world_velocity(Vec3::ZERO, local_vel);
        assert!(approx_eq_vec3(world_vel, Vec3::new(10.0, 5.0, 0.0)));
    }

    #[test]
    fn world_to_local_velocity() {
        let frame = ReferenceFrame::with_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let world_vel = Vec3::new(10.0, 5.0, 0.0);
        let local_vel = frame.world_to_local_velocity(Vec3::ZERO, world_vel);
        assert!(approx_eq_vec3(local_vel, Vec3::new(0.0, 5.0, 0.0)));
    }

    #[test]
    fn integrate_linear() {
        let mut frame =
            ReferenceFrame::with_linear_motion(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);
        frame.integrate(1.0);
        assert!(approx_eq_vec3(frame.origin, Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn integrate_with_acceleration() {
        let mut frame =
            ReferenceFrame::with_linear_motion(Vec3::ZERO, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0));
        frame.integrate(1.0);
        assert!(approx_eq_vec3(
            frame.linear_velocity,
            Vec3::new(2.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn integrate_rotation() {
        let mut frame =
            ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, PI, 0.0));
        frame.integrate(1.0);

        let local_x = Vec3::X;
        let world_x = frame.orientation * local_x;
        assert!(approx_eq_vec3(world_x, Vec3::new(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn pseudo_forces_linear_acceleration() {
        let frame = ReferenceFrame::IDENTITY.with_acceleration(Vec3::new(5.0, 0.0, 0.0));
        let force = frame.pseudo_force_at_point(Vec3::ZERO, 1.0);
        assert!(approx_eq_vec3(force, Vec3::new(-5.0, 0.0, 0.0)));
    }

    #[test]
    fn pseudo_forces_centrifugal() {
        let frame = ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, 1.0, 0.0));
        let force = frame.pseudo_force_at_point(Vec3::new(1.0, 0.0, 0.0), 1.0);
        assert!(force.x > 0.0);
    }

    #[test]
    fn coriolis_force() {
        let frame = ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, 1.0, 0.0));
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let force = frame.coriolis_force(velocity, 1.0);
        assert!(approx_eq_vec3(force, Vec3::new(0.0, 0.0, 2.0)));
    }

    #[test]
    fn is_rotating_detection() {
        let stationary = ReferenceFrame::at_position(Vec3::X);
        assert!(!stationary.is_rotating());

        let rotating = ReferenceFrame::rotating(Vec3::ZERO, Quat::IDENTITY, Vec3::Y);
        assert!(rotating.is_rotating());

        let tilted =
            ReferenceFrame::at_position(Vec3::ZERO).with_orientation(Quat::from_rotation_x(0.1));
        assert!(tilted.is_rotating());
    }

    #[test]
    fn direction_transforms() {
        let frame = ReferenceFrame::at_position(Vec3::new(100.0, 0.0, 0.0))
            .with_orientation(Quat::from_rotation_y(PI / 2.0));

        let local_dir = Vec3::X;
        let world_dir = frame.local_to_world_direction(local_dir);
        assert!(approx_eq_vec3(world_dir, Vec3::new(0.0, 0.0, -1.0)));

        let back = frame.world_to_local_direction(world_dir);
        assert!(approx_eq_vec3(back, local_dir));
    }
}

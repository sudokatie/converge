//! Fixed (weld) joints that lock relative position and orientation.
//!
//! Used for rigid attachment between bodies or to world anchors.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::anchor::ConstraintEndpoint;
use super::body::BodySnapshot;
use super::config::SpringParams;

/// A constraint that maintains fixed relative position and orientation.
///
/// Can be softened with compliance for some give, but generally acts
/// as a rigid weld between the two anchor frames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixedConstraint {
    /// Unique constraint identifier.
    pub id: ConstraintId,
    /// First endpoint (anchor A).
    pub endpoint_a: ConstraintEndpoint,
    /// Second endpoint (anchor B).
    pub endpoint_b: ConstraintEndpoint,
    /// Target local offset from A to B in A's frame.
    pub local_offset: Vec3,
    /// Target relative orientation (B relative to A).
    pub local_orientation: Quat,
    /// Spring parameters for position compliance.
    pub position_spring: SpringParams,
    /// Spring parameters for orientation compliance.
    pub angular_spring: SpringParams,
}

impl FixedConstraint {
    /// Creates a fixed constraint with current positions as the target.
    #[must_use]
    pub fn new(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            local_offset: Vec3::ZERO,
            local_orientation: Quat::IDENTITY,
            position_spring: SpringParams::stiff(),
            angular_spring: SpringParams::stiff(),
        }
    }

    /// Creates a fixed constraint from body snapshots, capturing current offset.
    #[must_use]
    pub fn from_bodies(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Self {
        let pos_a = endpoint_a.world_position(body_a);
        let pos_b = endpoint_b.world_position(body_b);

        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        let orientation_b = body_b.map_or(Quat::IDENTITY, |b| b.orientation);

        let local_offset = orientation_a.inverse() * (pos_b - pos_a);
        let local_orientation = orientation_a.inverse() * orientation_b;

        Self {
            id,
            endpoint_a,
            endpoint_b,
            local_offset,
            local_orientation,
            position_spring: SpringParams::stiff(),
            angular_spring: SpringParams::stiff(),
        }
    }

    /// Builder: sets local position offset.
    #[must_use]
    pub fn with_local_offset(mut self, offset: Vec3) -> Self {
        self.local_offset = offset;
        self
    }

    /// Builder: sets local orientation offset.
    #[must_use]
    pub fn with_local_orientation(mut self, orientation: Quat) -> Self {
        self.local_orientation = orientation;
        self
    }

    /// Builder: sets position spring parameters.
    #[must_use]
    pub fn with_position_spring(mut self, spring: SpringParams) -> Self {
        self.position_spring = spring;
        self
    }

    /// Builder: sets angular spring parameters.
    #[must_use]
    pub fn with_angular_spring(mut self, spring: SpringParams) -> Self {
        self.angular_spring = spring;
        self
    }

    /// Computes the target world position for endpoint B given body A state.
    #[must_use]
    pub fn target_position_b(&self, body_a: Option<&BodySnapshot>) -> Vec3 {
        let pos_a = self.endpoint_a.world_position(body_a);
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        pos_a + orientation_a * self.local_offset
    }

    /// Computes the target world orientation for body B given body A state.
    #[must_use]
    pub fn target_orientation_b(&self, body_a: Option<&BodySnapshot>) -> Quat {
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        orientation_a * self.local_orientation
    }

    /// Computes position error (world space vector from B to target).
    #[must_use]
    pub fn position_error(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Vec3 {
        let target = self.target_position_b(body_a);
        let actual = self.endpoint_b.world_position(body_b);
        target - actual
    }

    /// Computes angular error as axis-angle vector.
    #[must_use]
    pub fn angular_error(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Vec3 {
        let target = self.target_orientation_b(body_a);
        let actual = body_b.map_or(Quat::IDENTITY, |b| b.orientation);
        let error_quat = target * actual.inverse();
        let (axis, angle) = error_quat.to_axis_angle();
        axis * angle
    }

    /// Solves position constraint, returns correction magnitude.
    #[must_use]
    pub fn solve_position(
        &self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> f32 {
        let error = self.position_error(Some(body_a), Some(body_b));
        let error_magnitude = error.length();

        if error_magnitude < 1e-6 {
            return 0.0;
        }

        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return 0.0;
        }

        let compliance = self.position_spring.compliance / (dt * dt);
        let effective_mass = 1.0 / (inv_mass_sum + compliance);

        let correction = error * effective_mass * damping;

        body_a.apply_position_correction(pos_a, -correction);
        body_b.apply_position_correction(pos_b, correction);

        error_magnitude
    }

    /// Solves angular constraint, returns correction magnitude.
    #[must_use]
    pub fn solve_angular(
        &self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> f32 {
        let error = self.angular_error(Some(body_a), Some(body_b));
        let error_magnitude = error.length();

        if error_magnitude < 1e-6 {
            return 0.0;
        }

        let inv_inertia_sum = body_a.inv_inertia + body_b.inv_inertia;
        let avg_inv_inertia = (inv_inertia_sum.x + inv_inertia_sum.y + inv_inertia_sum.z) / 3.0;

        if avg_inv_inertia < 1e-6 {
            return 0.0;
        }

        let compliance = self.angular_spring.compliance / (dt * dt);
        let effective_inertia = 1.0 / (avg_inv_inertia + compliance);

        let angular_correction = error * effective_inertia * damping * 0.5;

        let delta_a = Quat::from_scaled_axis(-angular_correction * body_a.inv_inertia.length());
        let delta_b = Quat::from_scaled_axis(angular_correction * body_b.inv_inertia.length());

        body_a.orientation = (delta_a * body_a.orientation).normalize();
        body_b.orientation = (delta_b * body_b.orientation).normalize();

        error_magnitude
    }

    /// Solves velocity constraint.
    pub fn solve_velocity(&self, body_a: &mut BodySnapshot, body_b: &mut BodySnapshot, _dt: f32) {
        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let vel_a = body_a.velocity_at_point(pos_a);
        let vel_b = body_b.velocity_at_point(pos_b);
        let relative_velocity = vel_b - vel_a;

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return;
        }

        let damping_factor = self.position_spring.damping.min(1.0);
        let correction = relative_velocity * damping_factor / inv_mass_sum;

        body_a.apply_velocity_correction(pos_a, correction);
        body_b.apply_velocity_correction(pos_b, -correction);
    }

    /// Computes the force magnitude being applied by this constraint.
    #[must_use]
    pub fn compute_force(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
        dt: f32,
    ) -> f32 {
        let error = self.position_error(body_a, body_b);
        let error_magnitude = error.length();

        let inv_mass_a = body_a.map_or(0.0, |b| b.inv_mass);
        let inv_mass_b = body_b.map_or(0.0, |b| b.inv_mass);
        let inv_mass_sum = inv_mass_a + inv_mass_b;

        if inv_mass_sum < 1e-6 || error_magnitude < 1e-6 {
            return 0.0;
        }

        let effective_mass = 1.0 / inv_mass_sum;
        let stiffness = self.position_spring.stiffness();

        if stiffness.is_finite() {
            error_magnitude * stiffness
        } else {
            error_magnitude * effective_mass / (dt * dt)
        }
    }

    /// Computes the torque magnitude being applied by this constraint.
    #[must_use]
    pub fn compute_torque(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
        dt: f32,
    ) -> f32 {
        let error = self.angular_error(body_a, body_b);
        let error_magnitude = error.length();

        if error_magnitude < 1e-6 {
            return 0.0;
        }

        let stiffness = self.angular_spring.stiffness();
        if stiffness.is_finite() {
            error_magnitude * stiffness
        } else {
            error_magnitude / (dt * dt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::body::BodyId;
    use approx::assert_relative_eq;

    fn assert_vec3_eq(a: Vec3, b: Vec3) {
        assert_relative_eq!(a.x, b.x, epsilon = 1e-6);
        assert_relative_eq!(a.y, b.y, epsilon = 1e-6);
        assert_relative_eq!(a.z, b.z, epsilon = 1e-6);
    }

    #[test]
    fn fixed_constraint_zero_offset() {
        let body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let body_b = BodySnapshot::new(Vec3::new(1.0, 0.0, 0.0)).with_mass(1.0);

        let constraint = FixedConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
        );

        let target = constraint.target_position_b(Some(&body_a));
        assert_vec3_eq(target, Vec3::ZERO);

        let error = constraint.position_error(Some(&body_a), Some(&body_b));
        assert_vec3_eq(error, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn fixed_constraint_with_offset() {
        let body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let body_b = BodySnapshot::new(Vec3::new(2.0, 0.0, 0.0)).with_mass(1.0);

        let constraint = FixedConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
        )
        .with_local_offset(Vec3::new(2.0, 0.0, 0.0));

        let error = constraint.position_error(Some(&body_a), Some(&body_b));
        assert_relative_eq!(error.length(), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn fixed_constraint_captures_offset() {
        let body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let body_b = BodySnapshot::new(Vec3::new(3.0, 0.0, 0.0)).with_mass(1.0);

        let constraint = FixedConstraint::from_bodies(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            Some(&body_a),
            Some(&body_b),
        );

        assert_vec3_eq(constraint.local_offset, Vec3::new(3.0, 0.0, 0.0));

        let error = constraint.position_error(Some(&body_a), Some(&body_b));
        assert_relative_eq!(error.length(), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn solve_position_reduces_error() {
        let mut body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let mut body_b = BodySnapshot::new(Vec3::new(2.0, 0.0, 0.0)).with_mass(1.0);

        let constraint = FixedConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
        )
        .with_local_offset(Vec3::new(1.0, 0.0, 0.0));

        let error_before = constraint
            .position_error(Some(&body_a), Some(&body_b))
            .length();
        let _ = constraint.solve_position(&mut body_a, &mut body_b, 1.0 / 60.0, 1.0);
        let error_after = constraint
            .position_error(Some(&body_a), Some(&body_b))
            .length();

        assert!(error_after < error_before);
    }

    #[test]
    fn fixed_constraint_serialization() {
        let constraint = FixedConstraint::new(
            ConstraintId::new(7),
            ConstraintEndpoint::world(Vec3::ZERO),
            ConstraintEndpoint::body(BodyId::new(1)),
        )
        .with_local_offset(Vec3::new(1.0, 2.0, 3.0))
        .with_position_spring(SpringParams::soft(0.01, 0.5));

        let json = serde_json::to_string(&constraint).unwrap();
        let recovered: FixedConstraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id, constraint.id);
        assert_vec3_eq(recovered.local_offset, constraint.local_offset);
        assert_relative_eq!(
            recovered.position_spring.compliance,
            constraint.position_spring.compliance,
            epsilon = 1e-6
        );
    }
}

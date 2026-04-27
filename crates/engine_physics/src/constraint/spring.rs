//! Spring constraints with configurable stiffness and damping.
//!
//! Provides soft distance constraints that apply force proportional to
//! displacement from rest length.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::anchor::ConstraintEndpoint;
use super::body::BodySnapshot;
use super::config::SpringParams;

/// A soft distance constraint that behaves like a spring.
///
/// Unlike hard distance constraints, springs allow deviation from rest length
/// and apply proportional restoring forces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpringConstraint {
    /// Unique constraint identifier.
    pub id: ConstraintId,
    /// First endpoint (anchor A).
    pub endpoint_a: ConstraintEndpoint,
    /// Second endpoint (anchor B).
    pub endpoint_b: ConstraintEndpoint,
    /// Rest length where spring exerts no force.
    pub rest_length: f32,
    /// Spring parameters (stiffness via compliance, damping).
    pub params: SpringParams,
}

impl SpringConstraint {
    /// Creates a spring constraint with default parameters.
    #[must_use]
    pub fn new(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        rest_length: f32,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            rest_length,
            params: SpringParams::from_stiffness(100.0, 1.0),
        }
    }

    /// Creates a spring from stiffness and damping values.
    #[must_use]
    pub fn with_stiffness(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        rest_length: f32,
        stiffness: f32,
        damping: f32,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            rest_length,
            params: SpringParams::from_stiffness(stiffness, damping),
        }
    }

    /// Builder: sets rest length.
    #[must_use]
    pub fn with_rest_length(mut self, length: f32) -> Self {
        self.rest_length = length;
        self
    }

    /// Builder: sets spring parameters.
    #[must_use]
    pub fn with_params(mut self, params: SpringParams) -> Self {
        self.params = params;
        self
    }

    /// Builder: sets stiffness (k).
    #[must_use]
    pub fn stiffness(mut self, k: f32) -> Self {
        self.params = SpringParams::from_stiffness(k, self.params.damping);
        self
    }

    /// Builder: sets damping (c).
    #[must_use]
    pub fn damping(mut self, c: f32) -> Self {
        self.params.damping = c;
        self
    }

    /// Computes current separation and direction.
    #[must_use]
    pub fn compute_separation(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> (f32, Vec3) {
        let pos_a = self.endpoint_a.world_position(body_a);
        let pos_b = self.endpoint_b.world_position(body_b);
        let delta = pos_b - pos_a;
        let distance = delta.length();
        let direction = if distance > 1e-6 {
            delta / distance
        } else {
            Vec3::Y
        };
        (distance, direction)
    }

    /// Computes the displacement from rest length (positive = stretched).
    #[must_use]
    pub fn displacement(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let (distance, _) = self.compute_separation(body_a, body_b);
        distance - self.rest_length
    }

    /// Computes the spring force magnitude (positive = pulling together).
    #[must_use]
    pub fn force_magnitude(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let displacement = self.displacement(body_a, body_b);
        displacement * self.params.stiffness()
    }

    /// Computes the spring force vector (from A toward B if stretched).
    #[must_use]
    pub fn force_vector(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Vec3 {
        let (distance, direction) = self.compute_separation(body_a, body_b);
        let displacement = distance - self.rest_length;
        direction * displacement * self.params.stiffness()
    }

    /// Computes the damping force based on relative velocity.
    #[must_use]
    pub fn damping_force(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Vec3 {
        let (_, direction) = self.compute_separation(body_a, body_b);

        let vel_a = self.endpoint_a.world_velocity(body_a);
        let vel_b = self.endpoint_b.world_velocity(body_b);
        let relative_velocity = vel_b - vel_a;
        let velocity_along_spring = relative_velocity.dot(direction);

        direction * velocity_along_spring * self.params.damping
    }

    /// Applies spring forces to both bodies.
    pub fn apply_forces(&self, body_a: &mut BodySnapshot, body_b: &mut BodySnapshot, dt: f32) {
        let spring_force = self.force_vector(Some(body_a), Some(body_b));
        let damping_force = self.damping_force(Some(body_a), Some(body_b));
        let total_force = spring_force + damping_force;

        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let impulse = total_force * dt;
        body_a.apply_velocity_correction(pos_a, impulse);
        body_b.apply_velocity_correction(pos_b, -impulse);
    }

    /// Solves the spring constraint using XPBD-style position correction.
    #[must_use]
    pub fn solve_position(
        &self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping_factor: f32,
    ) -> f32 {
        let (distance, direction) = self.compute_separation(Some(body_a), Some(body_b));
        let error = distance - self.rest_length;

        if error.abs() < 1e-6 {
            return 0.0;
        }

        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return 0.0;
        }

        let compliance = self.params.compliance / (dt * dt);
        let effective_mass = 1.0 / (inv_mass_sum + compliance);
        let correction = error * effective_mass * damping_factor;

        let impulse = direction * correction;
        body_a.apply_position_correction(pos_a, impulse);
        body_b.apply_position_correction(pos_b, -impulse);

        correction.abs()
    }

    /// Solves velocity damping.
    pub fn solve_velocity(&self, body_a: &mut BodySnapshot, body_b: &mut BodySnapshot, dt: f32) {
        let (_, direction) = self.compute_separation(Some(body_a), Some(body_b));
        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let vel_a = body_a.velocity_at_point(pos_a);
        let vel_b = body_b.velocity_at_point(pos_b);
        let relative_velocity = vel_b - vel_a;
        let normal_velocity = relative_velocity.dot(direction);

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return;
        }

        let damping_correction = normal_velocity * self.params.damping * dt / inv_mass_sum;
        let impulse = direction * damping_correction;

        body_a.apply_velocity_correction(pos_a, impulse);
        body_b.apply_velocity_correction(pos_b, -impulse);
    }

    /// Computes the potential energy stored in the spring.
    #[must_use]
    pub fn potential_energy(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let displacement = self.displacement(body_a, body_b);
        0.5 * self.params.stiffness() * displacement * displacement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::body::BodyId;
    use approx::assert_relative_eq;

    fn make_spring(rest_length: f32, stiffness: f32, damping: f32) -> SpringConstraint {
        SpringConstraint::with_stiffness(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            rest_length,
            stiffness,
            damping,
        )
    }

    #[test]
    fn spring_at_rest_no_force() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(5.0, 0.0, 0.0));
        let spring = make_spring(5.0, 100.0, 0.0);

        let force = spring.force_magnitude(Some(&body_a), Some(&body_b));
        assert_relative_eq!(force, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn spring_stretched_positive_force() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(6.0, 0.0, 0.0));
        let spring = make_spring(5.0, 100.0, 0.0);

        let force = spring.force_magnitude(Some(&body_a), Some(&body_b));
        assert_relative_eq!(force, 100.0, epsilon = 1e-6);

        let force_vec = spring.force_vector(Some(&body_a), Some(&body_b));
        assert!(force_vec.x > 0.0);
    }

    #[test]
    fn spring_compressed_negative_force() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(4.0, 0.0, 0.0));
        let spring = make_spring(5.0, 100.0, 0.0);

        let force = spring.force_magnitude(Some(&body_a), Some(&body_b));
        assert_relative_eq!(force, -100.0, epsilon = 1e-6);
    }

    #[test]
    fn damping_opposes_motion() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let mut body_b = BodySnapshot::new(Vec3::new(5.0, 0.0, 0.0));
        body_b.linear_velocity = Vec3::new(10.0, 0.0, 0.0);

        let spring = make_spring(5.0, 0.0, 10.0);
        let damping = spring.damping_force(Some(&body_a), Some(&body_b));

        assert!(damping.x > 0.0);
    }

    #[test]
    fn solve_position_reduces_displacement() {
        let mut body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let mut body_b = BodySnapshot::new(Vec3::new(7.0, 0.0, 0.0)).with_mass(1.0);

        let spring = make_spring(5.0, 100.0, 0.0);

        let displacement_before = spring.displacement(Some(&body_a), Some(&body_b)).abs();
        let _ = spring.solve_position(&mut body_a, &mut body_b, 1.0 / 60.0, 1.0);
        let displacement_after = spring.displacement(Some(&body_a), Some(&body_b)).abs();

        assert!(displacement_after < displacement_before);
    }

    #[test]
    fn potential_energy_at_rest() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(5.0, 0.0, 0.0));
        let spring = make_spring(5.0, 100.0, 0.0);

        let energy = spring.potential_energy(Some(&body_a), Some(&body_b));
        assert_relative_eq!(energy, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn potential_energy_when_stretched() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(7.0, 0.0, 0.0));
        let spring = make_spring(5.0, 100.0, 0.0);

        let energy = spring.potential_energy(Some(&body_a), Some(&body_b));
        assert_relative_eq!(energy, 200.0, epsilon = 1e-6);
    }

    #[test]
    fn spring_serialization() {
        let spring = make_spring(10.0, 200.0, 5.0);

        let json = serde_json::to_string(&spring).unwrap();
        let recovered: SpringConstraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id, spring.id);
        assert_relative_eq!(recovered.rest_length, 10.0);
        assert_relative_eq!(recovered.params.stiffness(), 200.0, epsilon = 1e-6);
        assert_relative_eq!(recovered.params.damping, 5.0);
    }
}

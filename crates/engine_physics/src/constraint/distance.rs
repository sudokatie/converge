//! Distance constraints for maintaining separation between anchor points.
//!
//! Supports exact distance constraints, rope/max-length constraints with
//! slack handling, and spring-like soft constraints.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::anchor::ConstraintEndpoint;
use super::body::BodySnapshot;
use super::config::SpringParams;

/// Mode for distance constraint enforcement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMode {
    /// Maintain exact distance (bidirectional constraint).
    #[default]
    Exact,
    /// Maximum distance only (rope behavior - allows slack).
    MaxLength,
    /// Minimum distance only (repulsion behavior).
    MinLength,
}

/// A constraint that maintains distance between two endpoints.
///
/// Can function as a rigid rod, a rope with slack, or a soft spring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistanceConstraint {
    /// Unique constraint identifier.
    pub id: ConstraintId,
    /// First endpoint (anchor A).
    pub endpoint_a: ConstraintEndpoint,
    /// Second endpoint (anchor B).
    pub endpoint_b: ConstraintEndpoint,
    /// Rest/target distance.
    pub rest_length: f32,
    /// Distance enforcement mode.
    pub mode: DistanceMode,
    /// Spring parameters (compliance, damping).
    pub spring: SpringParams,
    /// Whether the constraint is currently taut (for rope mode).
    pub taut: bool,
}

impl DistanceConstraint {
    /// Creates an exact distance constraint.
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
            mode: DistanceMode::Exact,
            spring: SpringParams::stiff(),
            taut: true,
        }
    }

    /// Creates a rope constraint (max length with slack).
    #[must_use]
    pub fn rope(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        max_length: f32,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            rest_length: max_length,
            mode: DistanceMode::MaxLength,
            spring: SpringParams::stiff(),
            taut: false,
        }
    }

    /// Creates a spring constraint with the given parameters.
    #[must_use]
    pub fn spring(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        rest_length: f32,
        spring_params: SpringParams,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            rest_length,
            mode: DistanceMode::Exact,
            spring: spring_params,
            taut: true,
        }
    }

    /// Builder: sets the distance mode.
    #[must_use]
    pub fn with_mode(mut self, mode: DistanceMode) -> Self {
        self.mode = mode;
        self
    }

    /// Builder: sets spring parameters.
    #[must_use]
    pub fn with_spring(mut self, spring: SpringParams) -> Self {
        self.spring = spring;
        self
    }

    /// Builder: sets rest length.
    #[must_use]
    pub fn with_rest_length(mut self, length: f32) -> Self {
        self.rest_length = length;
        self
    }

    /// Returns whether this is a rope-style constraint.
    #[must_use]
    pub fn is_rope(&self) -> bool {
        self.mode == DistanceMode::MaxLength
    }

    /// Returns whether the constraint is currently enforcing (taut for ropes).
    #[must_use]
    pub fn is_active(&self) -> bool {
        match self.mode {
            DistanceMode::Exact => true,
            DistanceMode::MaxLength | DistanceMode::MinLength => self.taut,
        }
    }

    /// Computes current distance and direction between endpoints.
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

    /// Computes the constraint error (positive = stretched, negative = compressed).
    #[must_use]
    pub fn compute_error(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let (distance, _) = self.compute_separation(body_a, body_b);
        match self.mode {
            DistanceMode::Exact => distance - self.rest_length,
            DistanceMode::MaxLength => (distance - self.rest_length).max(0.0),
            DistanceMode::MinLength => (self.rest_length - distance).max(0.0),
        }
    }

    /// Updates taut state based on current separation.
    pub fn update_taut_state(
        &mut self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> bool {
        let (distance, _) = self.compute_separation(body_a, body_b);
        let was_taut = self.taut;

        self.taut = match self.mode {
            DistanceMode::Exact => true,
            DistanceMode::MaxLength => distance >= self.rest_length - 1e-4,
            DistanceMode::MinLength => distance <= self.rest_length + 1e-4,
        };

        was_taut != self.taut
    }

    /// Computes tension magnitude (positive = pulling apart).
    #[must_use]
    pub fn compute_tension(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
        dt: f32,
    ) -> f32 {
        if !self.is_active() {
            return 0.0;
        }

        let error = self.compute_error(body_a, body_b);
        if error.abs() < 1e-6 {
            return 0.0;
        }

        let inv_mass_sum = self.endpoint_a.inv_mass(body_a) + self.endpoint_b.inv_mass(body_b);
        if inv_mass_sum < 1e-6 {
            return 0.0;
        }

        let effective_mass = 1.0 / inv_mass_sum;
        let stiffness = self.spring.stiffness();

        if stiffness.is_finite() {
            error * stiffness
        } else {
            error * effective_mass / (dt * dt)
        }
    }

    /// Solves position constraint, returns correction impulse magnitude.
    #[must_use]
    pub fn solve_position(
        &mut self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> f32 {
        self.update_taut_state(Some(body_a), Some(body_b));

        if !self.is_active() {
            return 0.0;
        }

        let (distance, direction) = self.compute_separation(Some(body_a), Some(body_b));
        let error = match self.mode {
            DistanceMode::Exact => distance - self.rest_length,
            DistanceMode::MaxLength => (distance - self.rest_length).max(0.0),
            DistanceMode::MinLength => (self.rest_length - distance).max(0.0),
        };

        if error.abs() < 1e-6 {
            return 0.0;
        }

        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return 0.0;
        }

        let compliance = self.spring.compliance / (dt * dt);
        let effective_mass = 1.0 / (inv_mass_sum + compliance);
        let correction = error * effective_mass * damping;

        let impulse_direction = match self.mode {
            DistanceMode::MinLength => -direction,
            _ => direction,
        };
        let impulse = impulse_direction * correction;

        body_a.apply_position_correction(pos_a, impulse);
        body_b.apply_position_correction(pos_b, -impulse);

        correction.abs()
    }

    /// Solves velocity constraint, returns correction impulse magnitude.
    #[must_use]
    pub fn solve_velocity(
        &self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
    ) -> f32 {
        if !self.is_active() {
            return 0.0;
        }

        let (_, direction) = self.compute_separation(Some(body_a), Some(body_b));
        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));

        let vel_a = body_a.velocity_at_point(pos_a);
        let vel_b = body_b.velocity_at_point(pos_b);
        let relative_velocity = vel_b - vel_a;
        let normal_velocity = relative_velocity.dot(direction);

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return 0.0;
        }

        let damping_impulse = self.spring.damping * normal_velocity * dt;
        let correction = (normal_velocity + damping_impulse) / inv_mass_sum;

        let impulse = direction * correction;
        body_a.apply_velocity_correction(pos_a, impulse);
        body_b.apply_velocity_correction(pos_b, -impulse);

        correction.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn make_bodies(pos_a: Vec3, pos_b: Vec3) -> (BodySnapshot, BodySnapshot) {
        (
            BodySnapshot::new(pos_a).with_mass(1.0),
            BodySnapshot::new(pos_b).with_mass(1.0),
        )
    }

    #[test]
    fn distance_constraint_error_exact() {
        let (body_a, body_b) = make_bodies(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        let constraint = DistanceConstraint::new(
            ConstraintId::new(1),
            body_a.position.into(),
            body_b.position.into(),
            4.0,
        );
        let error = constraint.compute_error(Some(&body_a), Some(&body_b));
        assert_relative_eq!(error, 1.0);
    }

    #[test]
    fn rope_constraint_slack() {
        let (body_a, body_b) = make_bodies(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0));
        let mut constraint = DistanceConstraint::rope(
            ConstraintId::new(1),
            body_a.position.into(),
            body_b.position.into(),
            5.0,
        );
        constraint.update_taut_state(Some(&body_a), Some(&body_b));
        assert!(!constraint.taut);
        let error = constraint.compute_error(Some(&body_a), Some(&body_b));
        assert_relative_eq!(error, 0.0);
    }

    #[test]
    fn rope_constraint_taut() {
        let (body_a, body_b) = make_bodies(Vec3::ZERO, Vec3::new(6.0, 0.0, 0.0));
        let mut constraint = DistanceConstraint::rope(
            ConstraintId::new(1),
            body_a.position.into(),
            body_b.position.into(),
            5.0,
        );
        constraint.update_taut_state(Some(&body_a), Some(&body_b));
        assert!(constraint.taut);
        let error = constraint.compute_error(Some(&body_a), Some(&body_b));
        assert_relative_eq!(error, 1.0);
    }

    #[test]
    fn solve_position_reduces_error() {
        let (mut body_a, mut body_b) = make_bodies(Vec3::ZERO, Vec3::new(6.0, 0.0, 0.0));
        let mut constraint = DistanceConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(super::super::body::BodyId::new(0)),
            ConstraintEndpoint::body(super::super::body::BodyId::new(1)),
            4.0,
        );

        let error_before = constraint.compute_error(Some(&body_a), Some(&body_b));
        let _ = constraint.solve_position(&mut body_a, &mut body_b, 1.0 / 60.0, 1.0);
        let error_after = constraint.compute_error(Some(&body_a), Some(&body_b));

        assert!(error_after.abs() < error_before.abs());
    }

    #[test]
    fn tension_zero_when_slack() {
        let (body_a, body_b) = make_bodies(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0));
        let mut constraint = DistanceConstraint::rope(
            ConstraintId::new(1),
            body_a.position.into(),
            body_b.position.into(),
            5.0,
        );
        constraint.update_taut_state(Some(&body_a), Some(&body_b));
        let tension = constraint.compute_tension(Some(&body_a), Some(&body_b), 1.0 / 60.0);
        assert_relative_eq!(tension, 0.0);
    }

    #[test]
    fn tension_positive_when_stretched() {
        let (body_a, body_b) = make_bodies(Vec3::ZERO, Vec3::new(6.0, 0.0, 0.0));
        let mut constraint = DistanceConstraint::rope(
            ConstraintId::new(1),
            ConstraintEndpoint::body(super::super::body::BodyId::new(0)),
            ConstraintEndpoint::body(super::super::body::BodyId::new(1)),
            5.0,
        );
        constraint.update_taut_state(Some(&body_a), Some(&body_b));
        let tension = constraint.compute_tension(Some(&body_a), Some(&body_b), 1.0 / 60.0);
        assert!(tension > 0.0);
    }

    #[test]
    fn spring_damping_reduces_velocity() {
        let (mut body_a, mut body_b) = make_bodies(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0));
        body_b.linear_velocity = Vec3::new(10.0, 0.0, 0.0);

        let constraint = DistanceConstraint::spring(
            ConstraintId::new(1),
            ConstraintEndpoint::body(0.into()),
            ConstraintEndpoint::body(1.into()),
            4.0,
            SpringParams::soft(0.01, 10.0),
        );

        let vel_before = body_b.linear_velocity.x;
        let _ = constraint.solve_velocity(&mut body_a, &mut body_b, 1.0 / 60.0);
        let vel_after = body_b.linear_velocity.x;

        assert!(vel_after.abs() < vel_before.abs());
    }

    #[test]
    fn distance_constraint_serialization() {
        let constraint = DistanceConstraint::rope(
            ConstraintId::new(42),
            Vec3::ZERO.into(),
            Vec3::X.into(),
            5.0,
        )
        .with_spring(SpringParams::soft(0.01, 0.5));

        let json = serde_json::to_string(&constraint).unwrap();
        let recovered: DistanceConstraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id, constraint.id);
        assert_eq!(recovered.mode, DistanceMode::MaxLength);
        assert_relative_eq!(recovered.rest_length, 5.0);
        assert_relative_eq!(recovered.spring.compliance, 0.01);
    }
}

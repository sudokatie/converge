//! Rope and tether helpers for suspended cargo and grappling mechanics.
//!
//! Provides specialized functionality for rope-like constraints including
//! slack handling, tension reporting, and attachment/detachment semantics.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::anchor::ConstraintEndpoint;
use super::body::{BodyId, BodySnapshot};
use super::config::{BreakParams, SpringParams};
use super::distance::DistanceConstraint;
use super::event::{BreakEvent, ConstraintEvent, RopeSlackEvent, RopeTautEvent, TensionEvent};

/// State of a rope constraint for tracking transitions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RopeState {
    /// Rope is shorter than max length.
    #[default]
    Slack,
    /// Rope is at max length and under tension.
    Taut,
    /// Rope has broken.
    Broken,
}

/// A rope constraint with full slack/taut handling and break detection.
///
/// Wraps a distance constraint with additional rope-specific behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RopeConstraint {
    /// Underlying distance constraint.
    pub inner: DistanceConstraint,
    /// Break parameters.
    pub break_params: BreakParams,
    /// Current state.
    pub state: RopeState,
    /// Last computed tension.
    tension: f32,
    /// Whether this rope was taut last frame (for transition detection).
    was_taut: bool,
}

impl RopeConstraint {
    /// Creates a new rope constraint.
    #[must_use]
    pub fn new(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        max_length: f32,
    ) -> Self {
        Self {
            inner: DistanceConstraint::rope(id, endpoint_a, endpoint_b, max_length),
            break_params: BreakParams::unbreakable(),
            state: RopeState::Slack,
            tension: 0.0,
            was_taut: false,
        }
    }

    /// Creates a breakable rope.
    #[must_use]
    pub fn breakable(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        max_length: f32,
        max_tension: f32,
    ) -> Self {
        Self {
            inner: DistanceConstraint::rope(id, endpoint_a, endpoint_b, max_length),
            break_params: BreakParams::with_max_force(max_tension),
            state: RopeState::Slack,
            tension: 0.0,
            was_taut: false,
        }
    }

    /// Builder: sets max rope length.
    #[must_use]
    pub fn with_max_length(mut self, length: f32) -> Self {
        self.inner.rest_length = length;
        self
    }

    /// Builder: sets spring parameters for soft rope behavior.
    #[must_use]
    pub fn with_spring(mut self, spring: SpringParams) -> Self {
        self.inner.spring = spring;
        self
    }

    /// Builder: sets break parameters.
    #[must_use]
    pub fn with_break_params(mut self, params: BreakParams) -> Self {
        self.break_params = params;
        self
    }

    /// Returns the constraint ID.
    #[must_use]
    pub fn id(&self) -> ConstraintId {
        self.inner.id
    }

    /// Returns the max rope length.
    #[must_use]
    pub fn max_length(&self) -> f32 {
        self.inner.rest_length
    }

    /// Sets a new max rope length.
    pub fn set_max_length(&mut self, length: f32) {
        self.inner.rest_length = length;
    }

    /// Returns whether the rope is currently taut.
    #[must_use]
    pub fn is_taut(&self) -> bool {
        self.state == RopeState::Taut
    }

    /// Returns whether the rope is slack.
    #[must_use]
    pub fn is_slack(&self) -> bool {
        self.state == RopeState::Slack
    }

    /// Returns whether the rope has broken.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.state == RopeState::Broken
    }

    /// Returns the current tension (0 if slack).
    #[must_use]
    pub fn tension(&self) -> f32 {
        self.tension
    }

    /// Computes current rope length.
    #[must_use]
    pub fn current_length(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let (distance, _) = self.inner.compute_separation(body_a, body_b);
        distance
    }

    /// Computes slack amount (positive if rope is shorter than max).
    #[must_use]
    pub fn slack_amount(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let length = self.current_length(body_a, body_b);
        (self.max_length() - length).max(0.0)
    }

    /// Updates rope state and returns any transition events.
    pub fn update_state(
        &mut self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
        dt: f32,
    ) -> Vec<ConstraintEvent> {
        let mut events = Vec::new();

        if self.state == RopeState::Broken {
            return events;
        }

        self.was_taut = self.inner.taut;
        self.inner.update_taut_state(body_a, body_b);

        let newly_taut = !self.was_taut && self.inner.taut;
        let newly_slack = self.was_taut && !self.inner.taut;

        if newly_taut {
            let (_, direction) = self.inner.compute_separation(body_a, body_b);
            let vel_a = self.inner.endpoint_a.world_velocity(body_a);
            let vel_b = self.inner.endpoint_b.world_velocity(body_b);
            let relative_velocity = (vel_b - vel_a).dot(direction);

            events.push(ConstraintEvent::RopeTaut(RopeTautEvent::new(
                self.inner.id,
                relative_velocity.abs(),
                self.current_length(body_a, body_b),
            )));
            self.state = RopeState::Taut;
        } else if newly_slack {
            events.push(ConstraintEvent::RopeSlack(RopeSlackEvent::new(
                self.inner.id,
                self.slack_amount(body_a, body_b),
            )));
            self.state = RopeState::Slack;
        } else if self.inner.taut {
            self.state = RopeState::Taut;
        } else {
            self.state = RopeState::Slack;
        }

        self.tension = if self.state == RopeState::Taut {
            self.inner.compute_tension(body_a, body_b, dt)
        } else {
            0.0
        };

        if self.break_params.should_break(self.tension, 0.0) {
            let (_, direction) = self.inner.compute_separation(body_a, body_b);
            events.push(ConstraintEvent::Broke(BreakEvent::from_force(
                self.inner.id,
                self.tension,
                direction,
            )));
            self.state = RopeState::Broken;
            self.tension = 0.0;
        }

        events
    }

    /// Solves the rope constraint and returns tension event if taut.
    pub fn solve(
        &mut self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> Option<TensionEvent> {
        if self.state == RopeState::Broken || self.state == RopeState::Slack {
            return None;
        }

        let _ = self.inner.solve_position(body_a, body_b, dt, damping);
        let _ = self.inner.solve_velocity(body_a, body_b, dt);

        self.tension = self.inner.compute_tension(Some(body_a), Some(body_b), dt);
        let (distance, direction) = self.inner.compute_separation(Some(body_a), Some(body_b));

        Some(TensionEvent::new(
            self.inner.id,
            self.tension,
            distance,
            self.max_length(),
            direction,
        ))
    }
}

/// Attachment point for suspending cargo from a rope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CargoAttachment {
    /// The body ID of the attached cargo.
    pub cargo_id: BodyId,
    /// Local offset on the cargo where the rope attaches.
    pub local_offset: Vec3,
    /// The rope constraint.
    pub rope: RopeConstraint,
}

impl CargoAttachment {
    /// Creates a cargo attachment.
    #[must_use]
    pub fn new(
        constraint_id: ConstraintId,
        anchor: ConstraintEndpoint,
        cargo_id: BodyId,
        local_offset: Vec3,
        rope_length: f32,
    ) -> Self {
        let cargo_endpoint = ConstraintEndpoint::body_offset(cargo_id, local_offset);
        Self {
            cargo_id,
            local_offset,
            rope: RopeConstraint::new(constraint_id, anchor, cargo_endpoint, rope_length),
        }
    }

    /// Builder: sets break tension.
    #[must_use]
    pub fn with_break_tension(mut self, max_tension: f32) -> Self {
        self.rope.break_params = BreakParams::with_max_force(max_tension);
        self
    }

    /// Returns whether the cargo is still attached (rope not broken).
    #[must_use]
    pub fn is_attached(&self) -> bool {
        !self.rope.is_broken()
    }

    /// Detaches the cargo, returning detachment velocity if rope was taut.
    #[must_use]
    pub fn detach(&mut self, cargo_body: &BodySnapshot) -> DetachResult {
        let was_taut = self.rope.is_taut();
        let tension = self.rope.tension();
        let world_pos = cargo_body.local_to_world(self.local_offset);
        let world_vel = cargo_body.velocity_at_point(world_pos);

        self.rope.state = RopeState::Broken;

        DetachResult {
            world_position: world_pos,
            world_velocity: world_vel,
            was_taut,
            final_tension: tension,
        }
    }
}

/// Result of detaching cargo from a rope.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetachResult {
    /// World position of the attachment point at detach time.
    pub world_position: Vec3,
    /// World velocity of the attachment point at detach time.
    pub world_velocity: Vec3,
    /// Whether the rope was taut when detached.
    pub was_taut: bool,
    /// Final tension before detachment.
    pub final_tension: f32,
}

/// Builder for creating rope/tether configurations.
#[derive(Clone, Debug, Default)]
pub struct RopeBuilder {
    max_length: f32,
    break_tension: Option<f32>,
    spring_compliance: f32,
    spring_damping: f32,
}

impl RopeBuilder {
    /// Creates a new rope builder with the given max length.
    #[must_use]
    pub fn new(max_length: f32) -> Self {
        Self {
            max_length,
            break_tension: None,
            spring_compliance: 0.0,
            spring_damping: 0.0,
        }
    }

    /// Sets the break tension.
    #[must_use]
    pub fn break_tension(mut self, tension: f32) -> Self {
        self.break_tension = Some(tension);
        self
    }

    /// Sets spring compliance for soft rope behavior.
    #[must_use]
    pub fn compliance(mut self, compliance: f32) -> Self {
        self.spring_compliance = compliance;
        self
    }

    /// Sets spring damping.
    #[must_use]
    pub fn damping(mut self, damping: f32) -> Self {
        self.spring_damping = damping;
        self
    }

    /// Builds a rope constraint between two endpoints.
    #[must_use]
    pub fn build(
        self,
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
    ) -> RopeConstraint {
        let mut rope = RopeConstraint::new(id, endpoint_a, endpoint_b, self.max_length);

        if let Some(tension) = self.break_tension {
            rope.break_params = BreakParams::with_max_force(tension);
        }

        rope.inner.spring = SpringParams::soft(self.spring_compliance, self.spring_damping);
        rope
    }

    /// Builds a cargo attachment.
    #[must_use]
    pub fn build_cargo_attachment(
        self,
        id: ConstraintId,
        anchor: ConstraintEndpoint,
        cargo_id: BodyId,
        local_offset: Vec3,
    ) -> CargoAttachment {
        let mut attachment =
            CargoAttachment::new(id, anchor, cargo_id, local_offset, self.max_length);

        if let Some(tension) = self.break_tension {
            attachment.rope.break_params = BreakParams::with_max_force(tension);
        }

        attachment.rope.inner.spring =
            SpringParams::soft(self.spring_compliance, self.spring_damping);
        attachment
    }
}

/// Validates and enforces max rope length, returning whether rope is taut.
#[must_use]
pub fn enforce_max_length(pos_a: Vec3, pos_b: &mut Vec3, max_length: f32) -> bool {
    let delta = *pos_b - pos_a;
    let distance = delta.length();

    if distance > max_length && distance > 1e-6 {
        let direction = delta / distance;
        *pos_b = pos_a + direction * max_length;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::body::BodyId;
    use approx::assert_relative_eq;

    fn make_rope(max_length: f32) -> RopeConstraint {
        RopeConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            max_length,
        )
    }

    #[test]
    fn rope_slack_when_short() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(3.0, 0.0, 0.0));
        let mut rope = make_rope(5.0);

        rope.update_state(Some(&body_a), Some(&body_b), 1.0 / 60.0);

        assert!(rope.is_slack());
        assert_relative_eq!(rope.tension(), 0.0);
        assert_relative_eq!(
            rope.slack_amount(Some(&body_a), Some(&body_b)),
            2.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn rope_taut_when_stretched() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(6.0, 0.0, 0.0));
        let mut rope = make_rope(5.0);

        rope.update_state(Some(&body_a), Some(&body_b), 1.0 / 60.0);

        assert!(rope.is_taut());
        assert!(rope.tension() > 0.0);
    }

    #[test]
    fn rope_breaks_at_threshold() {
        let body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let body_b = BodySnapshot::new(Vec3::new(10.0, 0.0, 0.0)).with_mass(1.0);
        let mut rope = RopeConstraint::breakable(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            5.0,
            10.0,
        );

        let events = rope.update_state(Some(&body_a), Some(&body_b), 1.0 / 60.0);

        assert!(rope.is_broken());
        assert!(events.iter().any(ConstraintEvent::is_break));
    }

    #[test]
    fn rope_taut_transition_event() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(3.0, 0.0, 0.0));
        let mut rope = make_rope(5.0);

        rope.update_state(Some(&body_a), Some(&body_b), 1.0 / 60.0);
        assert!(rope.is_slack());

        let body_b_stretched = BodySnapshot::new(Vec3::new(6.0, 0.0, 0.0));
        let events = rope.update_state(Some(&body_a), Some(&body_b_stretched), 1.0 / 60.0);

        assert!(rope.is_taut());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ConstraintEvent::RopeTaut(_)))
        );
    }

    #[test]
    fn enforce_max_length_clamps() {
        let anchor = Vec3::ZERO;
        let mut pos = Vec3::new(10.0, 0.0, 0.0);

        let was_clamped = enforce_max_length(anchor, &mut pos, 5.0);

        assert!(was_clamped);
        assert_relative_eq!(pos.length(), 5.0, epsilon = 1e-6);
    }

    #[test]
    fn enforce_max_length_preserves_direction() {
        let anchor = Vec3::ZERO;
        let mut pos = Vec3::new(10.0, 10.0, 0.0);

        let _ = enforce_max_length(anchor, &mut pos, 5.0);

        let direction = pos.normalize();
        let expected = Vec3::new(10.0, 10.0, 0.0).normalize();
        assert_relative_eq!(direction.x, expected.x, epsilon = 1e-6);
        assert_relative_eq!(direction.y, expected.y, epsilon = 1e-6);
        assert_relative_eq!(direction.z, expected.z, epsilon = 1e-6);
    }

    #[test]
    fn cargo_attachment_detach() {
        let cargo_body = BodySnapshot::new(Vec3::new(0.0, -5.0, 0.0))
            .with_linear_velocity(Vec3::new(1.0, 0.0, 0.0));

        let mut attachment = CargoAttachment::new(
            ConstraintId::new(1),
            Vec3::ZERO.into(),
            BodyId::new(1),
            Vec3::ZERO,
            5.0,
        );

        attachment.rope.state = RopeState::Taut;
        attachment.rope.tension = 100.0;

        let result = attachment.detach(&cargo_body);

        assert!(!attachment.is_attached());
        assert!(result.was_taut);
        assert_relative_eq!(result.final_tension, 100.0);
        assert_relative_eq!(result.world_velocity.x, 1.0);
    }

    #[test]
    fn rope_builder() {
        let rope = RopeBuilder::new(10.0)
            .break_tension(500.0)
            .compliance(0.001)
            .damping(0.5)
            .build(ConstraintId::new(1), Vec3::ZERO.into(), Vec3::X.into());

        assert_relative_eq!(rope.max_length(), 10.0);
        assert!(rope.break_params.is_breakable());
        assert_relative_eq!(rope.inner.spring.compliance, 0.001);
    }

    #[test]
    fn rope_serialization() {
        let rope = make_rope(15.0).with_break_params(BreakParams::with_max_force(200.0));

        let json = serde_json::to_string(&rope).unwrap();
        let recovered: RopeConstraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id(), rope.id());
        assert_relative_eq!(recovered.max_length(), 15.0);
        assert!(recovered.break_params.is_breakable());
    }
}

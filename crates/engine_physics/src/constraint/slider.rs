//! Slider (prismatic) constraints for linear motion along an axis.
//!
//! Used for elevators, pistons, and other linear actuators with
//! position limits and motor control.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::anchor::ConstraintEndpoint;
use super::body::BodySnapshot;
use super::config::{MotorParams, SpringParams};
use super::event::{LimitEvent, LimitType};

/// A constraint that allows linear motion along a single axis.
///
/// The slide axis is defined in the local frame of body A.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SliderConstraint {
    /// Unique constraint identifier.
    pub id: ConstraintId,
    /// First endpoint (anchor A).
    pub endpoint_a: ConstraintEndpoint,
    /// Second endpoint (anchor B).
    pub endpoint_b: ConstraintEndpoint,
    /// Slide axis in body A's local frame.
    pub axis_local: Vec3,
    /// Reference position along axis at creation.
    pub reference_position: f32,
    /// Lower position limit (None for unlimited).
    pub position_min: Option<f32>,
    /// Upper position limit (None for unlimited).
    pub position_max: Option<f32>,
    /// Spring parameters for off-axis constraint.
    pub position_spring: SpringParams,
    /// Spring parameters for position limits.
    pub limit_spring: SpringParams,
    /// Motor parameters.
    pub motor: MotorParams,
    /// Current position along axis.
    current_position: f32,
}

impl SliderConstraint {
    /// Creates a slider constraint with the given axis.
    #[must_use]
    pub fn new(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        axis_local: Vec3,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            axis_local: axis_local.normalize_or_zero(),
            reference_position: 0.0,
            position_min: None,
            position_max: None,
            position_spring: SpringParams::stiff(),
            limit_spring: SpringParams::stiff(),
            motor: MotorParams::disabled(),
            current_position: 0.0,
        }
    }

    /// Creates a slider from body states, setting current position as reference.
    #[must_use]
    pub fn from_bodies(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        axis_local: Vec3,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Self {
        let pos_a = endpoint_a.world_position(body_a);
        let pos_b = endpoint_b.world_position(body_b);
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);

        let world_axis = (orientation_a * axis_local).normalize_or_zero();
        let reference_position = (pos_b - pos_a).dot(world_axis);

        Self {
            id,
            endpoint_a,
            endpoint_b,
            axis_local: axis_local.normalize_or_zero(),
            reference_position,
            position_min: None,
            position_max: None,
            position_spring: SpringParams::stiff(),
            limit_spring: SpringParams::stiff(),
            motor: MotorParams::disabled(),
            current_position: reference_position,
        }
    }

    /// Builder: sets position limits.
    #[must_use]
    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        self.position_min = Some(min);
        self.position_max = Some(max);
        self
    }

    /// Builder: removes position limits.
    #[must_use]
    pub fn with_unlimited(mut self) -> Self {
        self.position_min = None;
        self.position_max = None;
        self
    }

    /// Builder: sets position spring parameters.
    #[must_use]
    pub fn with_position_spring(mut self, spring: SpringParams) -> Self {
        self.position_spring = spring;
        self
    }

    /// Builder: sets limit spring parameters.
    #[must_use]
    pub fn with_limit_spring(mut self, spring: SpringParams) -> Self {
        self.limit_spring = spring;
        self
    }

    /// Builder: sets motor parameters.
    #[must_use]
    pub fn with_motor(mut self, motor: MotorParams) -> Self {
        self.motor = motor;
        self
    }

    /// Returns whether this slider has position limits.
    #[must_use]
    pub fn has_limits(&self) -> bool {
        self.position_min.is_some() || self.position_max.is_some()
    }

    /// Returns the current position along the slide axis.
    #[must_use]
    pub fn current_position(&self) -> f32 {
        self.current_position
    }

    /// Computes the world-space slide axis.
    #[must_use]
    pub fn world_axis(&self, body_a: Option<&BodySnapshot>) -> Vec3 {
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        (orientation_a * self.axis_local).normalize_or_zero()
    }

    /// Computes the current position along the slide axis.
    #[must_use]
    pub fn compute_position(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let pos_a = self.endpoint_a.world_position(body_a);
        let pos_b = self.endpoint_b.world_position(body_b);
        let world_axis = self.world_axis(body_a);

        (pos_b - pos_a).dot(world_axis) - self.reference_position
    }

    /// Updates the cached current position.
    pub fn update_position(
        &mut self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) {
        self.current_position = self.compute_position(body_a, body_b);
    }

    /// Checks if a limit is violated and returns the limit event if so.
    #[must_use]
    pub fn check_limit(&self) -> Option<LimitEvent> {
        if let Some(min) = self.position_min
            && self.current_position < min
        {
            return Some(LimitEvent::new(
                self.id,
                LimitType::Lower,
                self.current_position,
                min,
            ));
        }
        if let Some(max) = self.position_max
            && self.current_position > max
        {
            return Some(LimitEvent::new(
                self.id,
                LimitType::Upper,
                self.current_position,
                max,
            ));
        }
        None
    }

    /// Computes the off-axis position error.
    #[must_use]
    pub fn off_axis_error(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Vec3 {
        let pos_a = self.endpoint_a.world_position(body_a);
        let pos_b = self.endpoint_b.world_position(body_b);
        let world_axis = self.world_axis(body_a);

        let delta = pos_b - pos_a;
        let on_axis = world_axis * delta.dot(world_axis);
        delta - on_axis - world_axis * self.reference_position
    }

    /// Solves off-axis position constraint (keeps B on the slide axis).
    #[must_use]
    pub fn solve_off_axis(
        &mut self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> f32 {
        let error = self.off_axis_error(Some(body_a), Some(body_b));
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

        body_a.apply_position_correction(pos_a, correction);
        body_b.apply_position_correction(pos_b, -correction);

        error_magnitude
    }

    /// Solves position limit constraint.
    #[must_use]
    pub fn solve_position_limit(
        &mut self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> f32 {
        self.update_position(Some(body_a), Some(body_b));

        let limit_error = if let Some(min) = self.position_min {
            if self.current_position < min {
                Some(min - self.current_position)
            } else {
                None
            }
        } else {
            None
        }
        .or_else(|| {
            self.position_max.and_then(|max| {
                if self.current_position > max {
                    Some(max - self.current_position)
                } else {
                    None
                }
            })
        });

        let Some(error) = limit_error else {
            return 0.0;
        };

        if error.abs() < 1e-6 {
            return 0.0;
        }

        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));
        let world_axis = self.world_axis(Some(body_a));

        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;
        if inv_mass_sum < 1e-6 {
            return 0.0;
        }

        let compliance = self.limit_spring.compliance / (dt * dt);
        let effective_mass = 1.0 / (inv_mass_sum + compliance);
        let correction = world_axis * error * effective_mass * damping;

        body_a.apply_position_correction(pos_a, -correction);
        body_b.apply_position_correction(pos_b, correction);

        error.abs()
    }

    /// Solves motor constraint.
    pub fn solve_motor(&mut self, body_a: &mut BodySnapshot, body_b: &mut BodySnapshot, dt: f32) {
        if !self.motor.is_enabled() {
            return;
        }

        let pos_a = self.endpoint_a.world_position(Some(body_a));
        let pos_b = self.endpoint_b.world_position(Some(body_b));
        let world_axis = self.world_axis(Some(body_a));

        let vel_a = body_a.velocity_at_point(pos_a);
        let vel_b = body_b.velocity_at_point(pos_b);
        let relative_velocity = (vel_b - vel_a).dot(world_axis);

        let target_vel = match self.motor.mode {
            super::config::MotorMode::Velocity => self.motor.target_velocity,
            super::config::MotorMode::Position => {
                let position_error = self.motor.target_position - self.current_position;
                position_error / dt
            }
            super::config::MotorMode::Disabled => return,
        };

        let velocity_error = target_vel - relative_velocity;
        let inv_mass_sum = body_a.inv_mass + body_b.inv_mass;

        if inv_mass_sum < 1e-6 {
            return;
        }

        let impulse = (velocity_error / inv_mass_sum)
            .clamp(-self.motor.max_force * dt, self.motor.max_force * dt);

        let impulse_vec = world_axis * impulse;
        body_a.apply_velocity_correction(pos_a, -impulse_vec);
        body_b.apply_velocity_correction(pos_b, impulse_vec);
    }

    /// Sets the motor to target a specific position (elevator helper).
    pub fn set_target_position(&mut self, position: f32, max_force: f32) {
        self.motor = MotorParams::position(position, max_force);
    }

    /// Sets the motor to target a specific velocity (elevator helper).
    pub fn set_target_velocity(&mut self, velocity: f32, max_force: f32) {
        self.motor = MotorParams::velocity(velocity, max_force);
    }

    /// Stops the motor.
    pub fn stop_motor(&mut self) {
        self.motor = MotorParams::disabled();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::body::BodyId;
    use approx::assert_relative_eq;

    fn make_slider() -> SliderConstraint {
        SliderConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            Vec3::Y,
        )
    }

    #[test]
    fn slider_position_along_axis() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(0.0, 5.0, 0.0));

        let slider = make_slider();
        let position = slider.compute_position(Some(&body_a), Some(&body_b));
        assert_relative_eq!(position, 5.0, epsilon = 1e-6);
    }

    #[test]
    fn slider_from_bodies_sets_reference() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(0.0, 3.0, 0.0));

        let slider = SliderConstraint::from_bodies(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            Vec3::Y,
            Some(&body_a),
            Some(&body_b),
        );

        let position = slider.compute_position(Some(&body_a), Some(&body_b));
        assert_relative_eq!(position, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn slider_limit_check() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(0.0, 10.0, 0.0));

        let mut slider = make_slider().with_limits(0.0, 5.0);
        slider.update_position(Some(&body_a), Some(&body_b));

        let limit_event = slider.check_limit();
        assert!(limit_event.is_some());
        assert_eq!(limit_event.unwrap().limit_type, LimitType::Upper);
    }

    #[test]
    fn slider_within_limits() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(0.0, 3.0, 0.0));

        let mut slider = make_slider().with_limits(0.0, 5.0);
        slider.update_position(Some(&body_a), Some(&body_b));

        assert!(slider.check_limit().is_none());
    }

    #[test]
    fn slider_off_axis_error() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::new(2.0, 3.0, 0.0));

        let slider = make_slider();
        let off_axis = slider.off_axis_error(Some(&body_a), Some(&body_b));

        assert_relative_eq!(off_axis.x, 2.0, epsilon = 1e-6);
        assert_relative_eq!(off_axis.y, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn solve_off_axis_reduces_error() {
        let mut body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let mut body_b = BodySnapshot::new(Vec3::new(2.0, 3.0, 0.0)).with_mass(1.0);

        let mut slider = make_slider();
        let error_before = slider.off_axis_error(Some(&body_a), Some(&body_b)).length();
        let _ = slider.solve_off_axis(&mut body_a, &mut body_b, 1.0 / 60.0, 1.0);
        let error_after = slider.off_axis_error(Some(&body_a), Some(&body_b)).length();

        assert!(error_after < error_before);
    }

    #[test]
    fn slider_motor_helpers() {
        let mut slider = make_slider();

        slider.set_target_position(5.0, 100.0);
        assert_eq!(slider.motor.mode, super::super::config::MotorMode::Position);
        assert_relative_eq!(slider.motor.target_position, 5.0);

        slider.set_target_velocity(2.0, 50.0);
        assert_eq!(slider.motor.mode, super::super::config::MotorMode::Velocity);
        assert_relative_eq!(slider.motor.target_velocity, 2.0);

        slider.stop_motor();
        assert_eq!(slider.motor.mode, super::super::config::MotorMode::Disabled);
    }

    #[test]
    fn slider_serialization() {
        let slider = make_slider()
            .with_limits(-10.0, 10.0)
            .with_motor(MotorParams::position(5.0, 100.0));

        let json = serde_json::to_string(&slider).unwrap();
        let recovered: SliderConstraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id, slider.id);
        assert_relative_eq!(recovered.position_min.unwrap(), -10.0, epsilon = 1e-6);
        assert_eq!(
            recovered.motor.mode,
            super::super::config::MotorMode::Position
        );
    }
}

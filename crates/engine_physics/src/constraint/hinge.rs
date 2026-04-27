//! Hinge (revolute) joints that allow rotation around a single axis.
//!
//! Supports angle limits, motors, and spring-like compliance.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::anchor::ConstraintEndpoint;
use super::body::BodySnapshot;
use super::config::{MotorParams, SpringParams};
use super::event::{LimitEvent, LimitType};

/// A joint that allows rotation around a single axis.
///
/// The hinge axis is defined in the local frame of body A.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HingeConstraint {
    /// Unique constraint identifier.
    pub id: ConstraintId,
    /// First endpoint (anchor A).
    pub endpoint_a: ConstraintEndpoint,
    /// Second endpoint (anchor B).
    pub endpoint_b: ConstraintEndpoint,
    /// Hinge axis in body A's local frame.
    pub axis_local: Vec3,
    /// Reference orientation (B relative to A at angle=0).
    pub reference_orientation: Quat,
    /// Lower angle limit (radians, None for unlimited).
    pub angle_min: Option<f32>,
    /// Upper angle limit (radians, None for unlimited).
    pub angle_max: Option<f32>,
    /// Spring parameters for position compliance.
    pub position_spring: SpringParams,
    /// Spring parameters for angle limits.
    pub limit_spring: SpringParams,
    /// Motor parameters.
    pub motor: MotorParams,
    /// Current angle (cached for efficiency).
    current_angle: f32,
}

impl HingeConstraint {
    /// Creates a hinge constraint with the given axis.
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
            reference_orientation: Quat::IDENTITY,
            angle_min: None,
            angle_max: None,
            position_spring: SpringParams::stiff(),
            limit_spring: SpringParams::stiff(),
            motor: MotorParams::disabled(),
            current_angle: 0.0,
        }
    }

    /// Creates a hinge from body states, capturing current orientation as reference.
    #[must_use]
    pub fn from_bodies(
        id: ConstraintId,
        endpoint_a: ConstraintEndpoint,
        endpoint_b: ConstraintEndpoint,
        axis_local: Vec3,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Self {
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        let orientation_b = body_b.map_or(Quat::IDENTITY, |b| b.orientation);
        let reference_orientation = orientation_a.inverse() * orientation_b;

        Self {
            id,
            endpoint_a,
            endpoint_b,
            axis_local: axis_local.normalize_or_zero(),
            reference_orientation,
            angle_min: None,
            angle_max: None,
            position_spring: SpringParams::stiff(),
            limit_spring: SpringParams::stiff(),
            motor: MotorParams::disabled(),
            current_angle: 0.0,
        }
    }

    /// Builder: sets angle limits.
    #[must_use]
    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        self.angle_min = Some(min);
        self.angle_max = Some(max);
        self
    }

    /// Builder: removes angle limits.
    #[must_use]
    pub fn with_unlimited(mut self) -> Self {
        self.angle_min = None;
        self.angle_max = None;
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

    /// Returns whether this hinge has angle limits.
    #[must_use]
    pub fn has_limits(&self) -> bool {
        self.angle_min.is_some() || self.angle_max.is_some()
    }

    /// Returns the current angle.
    #[must_use]
    pub fn current_angle(&self) -> f32 {
        self.current_angle
    }

    /// Computes the world-space hinge axis.
    #[must_use]
    pub fn world_axis(&self, body_a: Option<&BodySnapshot>) -> Vec3 {
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        (orientation_a * self.axis_local).normalize_or_zero()
    }

    /// Computes the current rotation angle around the hinge axis.
    #[must_use]
    pub fn compute_angle(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> f32 {
        let orientation_a = body_a.map_or(Quat::IDENTITY, |b| b.orientation);
        let orientation_b = body_b.map_or(Quat::IDENTITY, |b| b.orientation);

        let relative = orientation_a.inverse() * orientation_b;
        let delta = relative * self.reference_orientation.inverse();

        let (axis, angle) = delta.to_axis_angle();
        let sign = axis.dot(self.axis_local).signum();
        angle * sign
    }

    /// Updates the cached current angle.
    pub fn update_angle(&mut self, body_a: Option<&BodySnapshot>, body_b: Option<&BodySnapshot>) {
        self.current_angle = self.compute_angle(body_a, body_b);
    }

    /// Checks if a limit is violated and returns the limit event if so.
    #[must_use]
    pub fn check_limit(&self) -> Option<LimitEvent> {
        if let Some(min) = self.angle_min
            && self.current_angle < min
        {
            return Some(LimitEvent::new(
                self.id,
                LimitType::Lower,
                self.current_angle,
                min,
            ));
        }
        if let Some(max) = self.angle_max
            && self.current_angle > max
        {
            return Some(LimitEvent::new(
                self.id,
                LimitType::Upper,
                self.current_angle,
                max,
            ));
        }
        None
    }

    /// Computes position error for the pivot point.
    #[must_use]
    pub fn position_error(
        &self,
        body_a: Option<&BodySnapshot>,
        body_b: Option<&BodySnapshot>,
    ) -> Vec3 {
        let pos_a = self.endpoint_a.world_position(body_a);
        let pos_b = self.endpoint_b.world_position(body_b);
        pos_a - pos_b
    }

    /// Solves pivot position constraint.
    #[must_use]
    pub fn solve_position(
        &mut self,
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

    /// Solves angle limit constraint.
    #[must_use]
    pub fn solve_angle_limit(
        &mut self,
        body_a: &mut BodySnapshot,
        body_b: &mut BodySnapshot,
        dt: f32,
        damping: f32,
    ) -> f32 {
        self.update_angle(Some(body_a), Some(body_b));

        let limit_error = if let Some(min) = self.angle_min {
            if self.current_angle < min {
                Some(min - self.current_angle)
            } else {
                None
            }
        } else {
            None
        }
        .or_else(|| {
            self.angle_max.and_then(|max| {
                if self.current_angle > max {
                    Some(max - self.current_angle)
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

        let world_axis = self.world_axis(Some(body_a));
        let inv_inertia_a = body_a.inv_inertia.dot(world_axis.abs());
        let inv_inertia_b = body_b.inv_inertia.dot(world_axis.abs());
        let inv_inertia_sum = inv_inertia_a + inv_inertia_b;

        if inv_inertia_sum < 1e-6 {
            return 0.0;
        }

        let compliance = self.limit_spring.compliance / (dt * dt);
        let effective_inertia = 1.0 / (inv_inertia_sum + compliance);
        let angular_correction = error * effective_inertia * damping;

        let correction_vec = world_axis * angular_correction;
        body_a.angular_velocity -= correction_vec * body_a.inv_inertia.length();
        body_b.angular_velocity += correction_vec * body_b.inv_inertia.length();

        let delta_a = Quat::from_scaled_axis(-correction_vec * 0.5);
        let delta_b = Quat::from_scaled_axis(correction_vec * 0.5);
        body_a.orientation = (delta_a * body_a.orientation).normalize();
        body_b.orientation = (delta_b * body_b.orientation).normalize();

        error.abs()
    }

    /// Solves motor constraint.
    pub fn solve_motor(&self, body_a: &mut BodySnapshot, body_b: &mut BodySnapshot, dt: f32) {
        if !self.motor.is_enabled() {
            return;
        }

        let world_axis = self.world_axis(Some(body_a));
        let relative_angular_vel =
            (body_b.angular_velocity - body_a.angular_velocity).dot(world_axis);

        let target_vel = match self.motor.mode {
            super::config::MotorMode::Velocity => self.motor.target_velocity,
            super::config::MotorMode::Position => {
                let position_error = self.motor.target_position - self.current_angle;
                position_error / dt
            }
            super::config::MotorMode::Disabled => return,
        };

        let velocity_error = target_vel - relative_angular_vel;
        let inv_inertia_a = body_a.inv_inertia.dot(world_axis.abs());
        let inv_inertia_b = body_b.inv_inertia.dot(world_axis.abs());
        let inv_inertia_sum = inv_inertia_a + inv_inertia_b;

        if inv_inertia_sum < 1e-6 {
            return;
        }

        let impulse = (velocity_error / inv_inertia_sum)
            .clamp(-self.motor.max_force * dt, self.motor.max_force * dt);

        let angular_impulse = world_axis * impulse;
        body_a.angular_velocity -= angular_impulse * inv_inertia_a;
        body_b.angular_velocity += angular_impulse * inv_inertia_b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::body::BodyId;
    use approx::assert_relative_eq;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn make_hinge() -> HingeConstraint {
        HingeConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            Vec3::Z,
        )
    }

    #[test]
    fn hinge_angle_zero_at_reference() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::X);

        let hinge = HingeConstraint::from_bodies(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            Vec3::Z,
            Some(&body_a),
            Some(&body_b),
        );

        let angle = hinge.compute_angle(Some(&body_a), Some(&body_b));
        assert_relative_eq!(angle, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn hinge_angle_with_rotation() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::X).with_orientation(Quat::from_rotation_z(FRAC_PI_2));

        let hinge = make_hinge();
        let angle = hinge.compute_angle(Some(&body_a), Some(&body_b));
        assert_relative_eq!(angle, FRAC_PI_2, epsilon = 1e-4);
    }

    #[test]
    fn hinge_limit_check() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::X).with_orientation(Quat::from_rotation_z(PI * 0.75));

        let mut hinge = make_hinge().with_limits(-FRAC_PI_2, FRAC_PI_2);
        hinge.update_angle(Some(&body_a), Some(&body_b));

        let limit_event = hinge.check_limit();
        assert!(limit_event.is_some());
        assert_eq!(limit_event.unwrap().limit_type, LimitType::Upper);
    }

    #[test]
    fn hinge_within_limits() {
        let body_a = BodySnapshot::new(Vec3::ZERO);
        let body_b = BodySnapshot::new(Vec3::X).with_orientation(Quat::from_rotation_z(FRAC_PI_4));

        let mut hinge = make_hinge().with_limits(-FRAC_PI_2, FRAC_PI_2);
        hinge.update_angle(Some(&body_a), Some(&body_b));

        assert!(hinge.check_limit().is_none());
    }

    #[test]
    fn solve_position_reduces_error() {
        let mut body_a = BodySnapshot::new(Vec3::ZERO).with_mass(1.0);
        let mut body_b = BodySnapshot::new(Vec3::new(2.0, 0.0, 0.0)).with_mass(1.0);

        let mut hinge = make_hinge();
        let error_before = hinge.position_error(Some(&body_a), Some(&body_b)).length();
        let _ = hinge.solve_position(&mut body_a, &mut body_b, 1.0 / 60.0, 1.0);
        let error_after = hinge.position_error(Some(&body_a), Some(&body_b)).length();

        assert!(error_after < error_before);
    }

    #[test]
    fn hinge_serialization() {
        let hinge = make_hinge()
            .with_limits(-PI, PI)
            .with_motor(MotorParams::velocity(1.0, 10.0));

        let json = serde_json::to_string(&hinge).unwrap();
        let recovered: HingeConstraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id, hinge.id);
        assert_relative_eq!(recovered.angle_min.unwrap(), -PI, epsilon = 1e-6);
        assert_eq!(
            recovered.motor.mode,
            super::super::config::MotorMode::Velocity
        );
    }
}

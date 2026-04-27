//! Solver configuration and constraint parameters.
//!
//! Provides iteration settings, compliance/stiffness/damping parameters,
//! and break thresholds for constraint solving.

use serde::{Deserialize, Serialize};

/// Configuration for the constraint solver.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Number of position correction iterations.
    pub position_iterations: u32,
    /// Number of velocity correction iterations.
    pub velocity_iterations: u32,
    /// Global position correction factor (0-1, typically 0.2-0.8).
    pub position_damping: f32,
    /// Maximum position correction per iteration.
    pub max_position_correction: f32,
    /// Maximum velocity correction per iteration.
    pub max_velocity_correction: f32,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            position_iterations: 4,
            velocity_iterations: 2,
            position_damping: 0.5,
            max_position_correction: 0.2,
            max_velocity_correction: 10.0,
        }
    }
}

impl SolverConfig {
    /// Creates a solver config optimized for real-time simulation.
    #[must_use]
    pub fn realtime() -> Self {
        Self::default()
    }

    /// Creates a solver config with higher accuracy for critical constraints.
    #[must_use]
    pub fn high_accuracy() -> Self {
        Self {
            position_iterations: 8,
            velocity_iterations: 4,
            position_damping: 0.7,
            max_position_correction: 0.1,
            max_velocity_correction: 5.0,
        }
    }

    /// Builder: sets position iterations.
    #[must_use]
    pub const fn with_position_iterations(mut self, iterations: u32) -> Self {
        self.position_iterations = iterations;
        self
    }

    /// Builder: sets velocity iterations.
    #[must_use]
    pub const fn with_velocity_iterations(mut self, iterations: u32) -> Self {
        self.velocity_iterations = iterations;
        self
    }

    /// Builder: sets position damping.
    #[must_use]
    pub const fn with_position_damping(mut self, damping: f32) -> Self {
        self.position_damping = damping;
        self
    }
}

/// Spring-like constraint parameters with compliance, stiffness, and damping.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpringParams {
    /// Compliance (inverse stiffness). Higher values = softer constraint.
    pub compliance: f32,
    /// Damping coefficient for velocity correction.
    pub damping: f32,
}

impl Default for SpringParams {
    fn default() -> Self {
        Self {
            compliance: 0.0,
            damping: 0.0,
        }
    }
}

impl SpringParams {
    /// Creates stiff constraint parameters (hard constraint).
    #[must_use]
    pub const fn stiff() -> Self {
        Self {
            compliance: 0.0,
            damping: 0.0,
        }
    }

    /// Creates spring parameters from stiffness (k) and damping (c).
    #[must_use]
    pub fn from_stiffness(stiffness: f32, damping: f32) -> Self {
        Self {
            compliance: if stiffness > 0.0 {
                1.0 / stiffness
            } else {
                0.0
            },
            damping,
        }
    }

    /// Creates soft constraint parameters.
    #[must_use]
    pub const fn soft(compliance: f32, damping: f32) -> Self {
        Self {
            compliance,
            damping,
        }
    }

    /// Builder: sets compliance.
    #[must_use]
    pub const fn with_compliance(mut self, compliance: f32) -> Self {
        self.compliance = compliance;
        self
    }

    /// Builder: sets damping.
    #[must_use]
    pub const fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Returns the effective stiffness (inverse of compliance).
    #[must_use]
    pub fn stiffness(&self) -> f32 {
        if self.compliance > 0.0 {
            1.0 / self.compliance
        } else {
            f32::INFINITY
        }
    }

    /// Returns whether this is a hard constraint (zero compliance).
    #[must_use]
    pub fn is_hard(&self) -> bool {
        self.compliance == 0.0
    }
}

/// Parameters for breakable constraints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BreakParams {
    /// Maximum force before breaking (Newtons). `None` = unbreakable.
    pub max_force: Option<f32>,
    /// Maximum torque before breaking (Newton-meters). `None` = unbreakable.
    pub max_torque: Option<f32>,
}

impl BreakParams {
    /// Creates unbreakable parameters.
    #[must_use]
    pub const fn unbreakable() -> Self {
        Self {
            max_force: None,
            max_torque: None,
        }
    }

    /// Creates parameters with a force threshold.
    #[must_use]
    pub const fn with_max_force(max_force: f32) -> Self {
        Self {
            max_force: Some(max_force),
            max_torque: None,
        }
    }

    /// Creates parameters with force and torque thresholds.
    #[must_use]
    pub const fn with_thresholds(max_force: f32, max_torque: f32) -> Self {
        Self {
            max_force: Some(max_force),
            max_torque: Some(max_torque),
        }
    }

    /// Builder: sets max force.
    #[must_use]
    pub const fn max_force(mut self, force: f32) -> Self {
        self.max_force = Some(force);
        self
    }

    /// Builder: sets max torque.
    #[must_use]
    pub const fn max_torque(mut self, torque: f32) -> Self {
        self.max_torque = Some(torque);
        self
    }

    /// Returns whether this constraint can break.
    #[must_use]
    pub const fn is_breakable(&self) -> bool {
        self.max_force.is_some() || self.max_torque.is_some()
    }

    /// Checks if the given force/torque magnitudes exceed thresholds.
    #[must_use]
    pub fn should_break(&self, force_magnitude: f32, torque_magnitude: f32) -> bool {
        if let Some(max) = self.max_force
            && force_magnitude > max
        {
            return true;
        }
        if let Some(max) = self.max_torque
            && torque_magnitude > max
        {
            return true;
        }
        false
    }
}

/// Motor parameters for driven constraints (sliders, hinges).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotorParams {
    /// Target velocity for velocity-mode motor.
    pub target_velocity: f32,
    /// Target position for position-mode motor.
    pub target_position: f32,
    /// Maximum force/torque the motor can apply.
    pub max_force: f32,
    /// Motor mode: position or velocity.
    pub mode: MotorMode,
}

impl Default for MotorParams {
    fn default() -> Self {
        Self {
            target_velocity: 0.0,
            target_position: 0.0,
            max_force: 0.0,
            mode: MotorMode::Disabled,
        }
    }
}

impl MotorParams {
    /// Creates disabled motor params.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            target_velocity: 0.0,
            target_position: 0.0,
            max_force: 0.0,
            mode: MotorMode::Disabled,
        }
    }

    /// Creates velocity-mode motor params.
    #[must_use]
    pub const fn velocity(target_velocity: f32, max_force: f32) -> Self {
        Self {
            target_velocity,
            target_position: 0.0,
            max_force,
            mode: MotorMode::Velocity,
        }
    }

    /// Creates position-mode motor params.
    #[must_use]
    pub const fn position(target_position: f32, max_force: f32) -> Self {
        Self {
            target_velocity: 0.0,
            target_position,
            max_force,
            mode: MotorMode::Position,
        }
    }

    /// Builder: sets target velocity and enables velocity mode.
    #[must_use]
    pub const fn with_target_velocity(mut self, velocity: f32) -> Self {
        self.target_velocity = velocity;
        self.mode = MotorMode::Velocity;
        self
    }

    /// Builder: sets target position and enables position mode.
    #[must_use]
    pub const fn with_target_position(mut self, position: f32) -> Self {
        self.target_position = position;
        self.mode = MotorMode::Position;
        self
    }

    /// Builder: sets max force.
    #[must_use]
    pub const fn with_max_force(mut self, force: f32) -> Self {
        self.max_force = force;
        self
    }

    /// Returns whether the motor is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        !matches!(self.mode, MotorMode::Disabled)
    }
}

/// Motor operating mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotorMode {
    /// Motor is disabled.
    #[default]
    Disabled,
    /// Motor targets a velocity.
    Velocity,
    /// Motor targets a position.
    Position,
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn solver_config_defaults() {
        let config = SolverConfig::default();
        assert_eq!(config.position_iterations, 4);
        assert_eq!(config.velocity_iterations, 2);
        assert_relative_eq!(config.position_damping, 0.5);
    }

    #[test]
    fn spring_params_stiff() {
        let params = SpringParams::stiff();
        assert!(params.is_hard());
        assert!(params.stiffness().is_infinite());
    }

    #[test]
    fn spring_params_from_stiffness() {
        let params = SpringParams::from_stiffness(100.0, 5.0);
        assert_relative_eq!(params.stiffness(), 100.0, epsilon = 1e-6);
        assert_relative_eq!(params.damping, 5.0);
        assert!(!params.is_hard());
    }

    #[test]
    fn break_params_unbreakable() {
        let params = BreakParams::unbreakable();
        assert!(!params.is_breakable());
        assert!(!params.should_break(1000.0, 1000.0));
    }

    #[test]
    fn break_params_force_threshold() {
        let params = BreakParams::with_max_force(100.0);
        assert!(params.is_breakable());
        assert!(!params.should_break(50.0, 0.0));
        assert!(params.should_break(150.0, 0.0));
    }

    #[test]
    fn break_params_torque_threshold() {
        let params = BreakParams::with_thresholds(100.0, 50.0);
        assert!(!params.should_break(50.0, 25.0));
        assert!(params.should_break(50.0, 75.0));
    }

    #[test]
    fn motor_params_disabled() {
        let params = MotorParams::disabled();
        assert!(!params.is_enabled());
        assert_eq!(params.mode, MotorMode::Disabled);
    }

    #[test]
    fn motor_params_velocity() {
        let params = MotorParams::velocity(5.0, 100.0);
        assert!(params.is_enabled());
        assert_eq!(params.mode, MotorMode::Velocity);
        assert_relative_eq!(params.target_velocity, 5.0);
        assert_relative_eq!(params.max_force, 100.0);
    }

    #[test]
    fn motor_params_position() {
        let params = MotorParams::position(10.0, 200.0);
        assert!(params.is_enabled());
        assert_eq!(params.mode, MotorMode::Position);
        assert_relative_eq!(params.target_position, 10.0);
    }

    #[test]
    fn config_serialization() {
        let config = SolverConfig::high_accuracy();
        let json = serde_json::to_string(&config).unwrap();
        let recovered: SolverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.position_iterations, config.position_iterations);

        let spring = SpringParams::soft(0.01, 0.5);
        let json = serde_json::to_string(&spring).unwrap();
        let recovered: SpringParams = serde_json::from_str(&json).unwrap();
        assert_relative_eq!(recovered.compliance, spring.compliance);

        let motor = MotorParams::velocity(3.0, 50.0);
        let json = serde_json::to_string(&motor).unwrap();
        let recovered: MotorParams = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.mode, MotorMode::Velocity);
    }
}

//! Constraint and joint system for physics simulation.
//!
//! Provides constraint types for connecting bodies with various joint types:
//! distance constraints, fixed joints, hinges, sliders, and springs. Includes
//! specialized support for ropes, tethers, suspended cargo, and elevators.
//!
//! # Architecture
//!
//! - [`BodyId`] / [`BodySnapshot`]: Lightweight body state for constraint solving
//! - [`ConstraintEndpoint`]: Anchor points (world or body-local)
//! - [`DistanceConstraint`]: Distance/rope constraints with slack handling
//! - [`FixedConstraint`]: Rigid weld joints
//! - [`HingeConstraint`]: Revolute joints with angle limits
//! - [`SliderConstraint`]: Prismatic joints for elevators
//! - [`SpringConstraint`]: Soft distance constraints
//! - [`RopeConstraint`]: Full rope/tether with break detection
//! - [`ConstraintSolver`]: Iterative XPBD-style solver
//!
//! # Example
//!
//! ```ignore
//! use engine_physics::constraint::*;
//! use glam::Vec3;
//!
//! // Create a rope between a world anchor and a body
//! let rope = RopeConstraint::new(
//!     ConstraintId::new(1),
//!     ConstraintEndpoint::world(Vec3::new(0.0, 10.0, 0.0)),
//!     ConstraintEndpoint::body(BodyId::new(1)),
//!     5.0, // max length
//! );
//!
//! // Create an elevator slider
//! let mut elevator = SliderConstraint::new(
//!     ConstraintId::new(2),
//!     ConstraintEndpoint::world(Vec3::ZERO),
//!     ConstraintEndpoint::body(BodyId::new(2)),
//!     Vec3::Y, // vertical axis
//! ).with_limits(0.0, 20.0);
//! elevator.set_target_position(10.0, 1000.0);
//! ```
//!
//! # Constraint Types
//!
//! | Type | Use Case | Key Features |
//! |------|----------|--------------|
//! | Distance | Rigid rods, ropes | Exact or max-length modes |
//! | Fixed | Welded attachments | Position + orientation lock |
//! | Hinge | Doors, rotating arms | Angle limits, motors |
//! | Slider | Elevators, pistons | Position limits, motors |
//! | Spring | Soft connections | Stiffness, damping |
//! | Rope | Tethers, cargo | Slack, tension, breaking |

mod anchor;
mod body;
mod config;
mod distance;
mod event;
mod fixed;
mod hinge;
mod rope;
mod slider;
mod solver;
mod spring;

pub use anchor::{BodyAnchor, ConstraintEndpoint, WorldAnchor};
pub use body::{BodyId, BodySnapshot};
pub use config::{BreakParams, MotorMode, MotorParams, SolverConfig, SpringParams};
pub use distance::{DistanceConstraint, DistanceMode};
pub use event::{
    BreakEvent, ConstraintEvent, ConstraintEvents, LimitEvent, LimitType, RopeSlackEvent,
    RopeTautEvent, TensionEvent,
};
pub use fixed::FixedConstraint;
pub use hinge::HingeConstraint;
pub use rope::{
    CargoAttachment, DetachResult, RopeBuilder, RopeConstraint, RopeState, enforce_max_length,
};
pub use slider::SliderConstraint;
pub use solver::{BodyStates, BreakableConstraint, Constraint, ConstraintSolver};
pub use spring::SpringConstraint;

use serde::{Deserialize, Serialize};

/// Unique identifier for a constraint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConstraintId(u64);

impl ConstraintId {
    /// Creates a new constraint identifier.
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

impl From<u64> for ConstraintId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<ConstraintId> for u64 {
    fn from(id: ConstraintId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_id_roundtrip() {
        let id = ConstraintId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(u64::from(id), 42);
        assert_eq!(ConstraintId::from(42u64), id);
    }

    #[test]
    fn constraint_id_serialization() {
        let id = ConstraintId::new(123);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: ConstraintId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, id);
    }
}

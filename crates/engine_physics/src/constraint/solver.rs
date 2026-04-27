//! Constraint solver for iterative position and velocity correction.
//!
//! Implements XPBD-style constraint solving with configurable iteration
//! counts and deterministic ordering.

use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::ConstraintId;
use super::body::{BodyId, BodySnapshot};
use super::config::{BreakParams, SolverConfig};
use super::distance::DistanceConstraint;
use super::event::{BreakEvent, ConstraintEvents};
use super::fixed::FixedConstraint;
use super::hinge::HingeConstraint;
use super::slider::SliderConstraint;
use super::spring::SpringConstraint;

/// A constraint that can be solved by the solver.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Constraint {
    /// Distance/rope constraint.
    Distance(DistanceConstraint),
    /// Fixed (weld) joint.
    Fixed(FixedConstraint),
    /// Hinge (revolute) joint.
    Hinge(HingeConstraint),
    /// Slider (prismatic) joint.
    Slider(SliderConstraint),
    /// Spring constraint.
    Spring(SpringConstraint),
}

impl Constraint {
    /// Returns the constraint ID.
    #[must_use]
    pub fn id(&self) -> ConstraintId {
        match self {
            Self::Distance(c) => c.id,
            Self::Fixed(c) => c.id,
            Self::Hinge(c) => c.id,
            Self::Slider(c) => c.id,
            Self::Spring(c) => c.id,
        }
    }

    /// Returns the body IDs involved in this constraint.
    #[must_use]
    pub fn body_ids(&self) -> (Option<BodyId>, Option<BodyId>) {
        match self {
            Self::Distance(c) => (c.endpoint_a.body_id(), c.endpoint_b.body_id()),
            Self::Fixed(c) => (c.endpoint_a.body_id(), c.endpoint_b.body_id()),
            Self::Hinge(c) => (c.endpoint_a.body_id(), c.endpoint_b.body_id()),
            Self::Slider(c) => (c.endpoint_a.body_id(), c.endpoint_b.body_id()),
            Self::Spring(c) => (c.endpoint_a.body_id(), c.endpoint_b.body_id()),
        }
    }
}

impl From<DistanceConstraint> for Constraint {
    fn from(c: DistanceConstraint) -> Self {
        Self::Distance(c)
    }
}

impl From<FixedConstraint> for Constraint {
    fn from(c: FixedConstraint) -> Self {
        Self::Fixed(c)
    }
}

impl From<HingeConstraint> for Constraint {
    fn from(c: HingeConstraint) -> Self {
        Self::Hinge(c)
    }
}

impl From<SliderConstraint> for Constraint {
    fn from(c: SliderConstraint) -> Self {
        Self::Slider(c)
    }
}

impl From<SpringConstraint> for Constraint {
    fn from(c: SpringConstraint) -> Self {
        Self::Spring(c)
    }
}

/// Breakable constraint wrapper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BreakableConstraint {
    /// The inner constraint.
    pub constraint: Constraint,
    /// Break parameters.
    pub break_params: BreakParams,
    /// Whether the constraint has broken.
    pub broken: bool,
    /// Accumulated force for break detection.
    accumulated_force: f32,
    /// Accumulated torque for break detection.
    accumulated_torque: f32,
}

impl BreakableConstraint {
    /// Creates a breakable constraint.
    #[must_use]
    pub fn new(constraint: Constraint, break_params: BreakParams) -> Self {
        Self {
            constraint,
            break_params,
            broken: false,
            accumulated_force: 0.0,
            accumulated_torque: 0.0,
        }
    }

    /// Returns the constraint ID.
    #[must_use]
    pub fn id(&self) -> ConstraintId {
        self.constraint.id()
    }

    /// Returns whether this constraint is broken.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.broken
    }

    /// Accumulates force for break detection.
    pub fn accumulate_force(&mut self, force: f32) {
        self.accumulated_force += force;
    }

    /// Accumulates torque for break detection.
    pub fn accumulate_torque(&mut self, torque: f32) {
        self.accumulated_torque += torque;
    }

    /// Checks if the constraint should break and updates state.
    pub fn check_break(&mut self) -> Option<BreakEvent> {
        if self.broken {
            return None;
        }

        if self
            .break_params
            .should_break(self.accumulated_force, self.accumulated_torque)
        {
            self.broken = true;
            Some(BreakEvent::from_force(
                self.id(),
                self.accumulated_force,
                Vec3::ZERO,
            ))
        } else {
            None
        }
    }

    /// Resets accumulated forces for next frame.
    pub fn reset_accumulators(&mut self) {
        self.accumulated_force = 0.0;
        self.accumulated_torque = 0.0;
    }
}

/// Body state collection for constraint solving.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BodyStates {
    bodies: HashMap<BodyId, BodySnapshot>,
}

impl BodyStates {
    /// Creates an empty body state collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a body state.
    pub fn insert(&mut self, id: BodyId, body: BodySnapshot) {
        self.bodies.insert(id, body);
    }

    /// Gets a body state reference.
    #[must_use]
    pub fn get(&self, id: BodyId) -> Option<&BodySnapshot> {
        self.bodies.get(&id)
    }

    /// Gets a mutable body state reference.
    pub fn get_mut(&mut self, id: BodyId) -> Option<&mut BodySnapshot> {
        self.bodies.get_mut(&id)
    }

    /// Returns an iterator over body IDs.
    pub fn ids(&self) -> impl Iterator<Item = BodyId> + '_ {
        self.bodies.keys().copied()
    }

    /// Returns an iterator over body states.
    pub fn iter(&self) -> impl Iterator<Item = (BodyId, &BodySnapshot)> {
        self.bodies.iter().map(|(k, v)| (*k, v))
    }

    /// Returns a mutable iterator over body states.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BodyId, &mut BodySnapshot)> {
        self.bodies.iter_mut().map(|(k, v)| (*k, v))
    }

    /// Returns the number of bodies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    /// Returns whether there are no bodies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }
}

/// Constraint solver with deterministic ordering.
#[derive(Clone, Debug, Default)]
pub struct ConstraintSolver {
    /// Solver configuration.
    pub config: SolverConfig,
    /// Constraints to solve.
    constraints: Vec<Constraint>,
    /// Constraint ordering for deterministic solving.
    solve_order: Vec<usize>,
}

impl ConstraintSolver {
    /// Creates a new solver with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a solver with the given configuration.
    #[must_use]
    pub fn with_config(config: SolverConfig) -> Self {
        Self {
            config,
            constraints: Vec::new(),
            solve_order: Vec::new(),
        }
    }

    /// Adds a constraint to the solver.
    pub fn add_constraint(&mut self, constraint: impl Into<Constraint>) {
        self.constraints.push(constraint.into());
        self.solve_order.push(self.constraints.len() - 1);
    }

    /// Removes a constraint by ID.
    pub fn remove_constraint(&mut self, id: ConstraintId) {
        if let Some(pos) = self.constraints.iter().position(|c| c.id() == id) {
            self.constraints.remove(pos);
            self.rebuild_solve_order();
        }
    }

    /// Clears all constraints.
    pub fn clear(&mut self) {
        self.constraints.clear();
        self.solve_order.clear();
    }

    /// Returns the number of constraints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    /// Returns whether there are no constraints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Gets a constraint by ID.
    #[must_use]
    pub fn get(&self, id: ConstraintId) -> Option<&Constraint> {
        self.constraints.iter().find(|c| c.id() == id)
    }

    /// Gets a mutable constraint by ID.
    pub fn get_mut(&mut self, id: ConstraintId) -> Option<&mut Constraint> {
        self.constraints.iter_mut().find(|c| c.id() == id)
    }

    /// Rebuilds solve order after constraint removal.
    fn rebuild_solve_order(&mut self) {
        self.solve_order = (0..self.constraints.len()).collect();
    }

    /// Sets a custom solve order for deterministic results.
    pub fn set_solve_order(&mut self, order: Vec<usize>) {
        self.solve_order = order;
    }

    /// Sorts constraints by ID for deterministic ordering.
    pub fn sort_by_id(&mut self) {
        let mut indexed: Vec<_> = self
            .constraints
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.id()))
            .collect();
        indexed.sort_by_key(|(_, id)| id.raw());
        self.solve_order = indexed.into_iter().map(|(i, _)| i).collect();
    }

    /// Solves all constraints for one timestep.
    pub fn solve(&mut self, bodies: &mut BodyStates, dt: f32) -> ConstraintEvents {
        let mut events = ConstraintEvents::new();

        for _ in 0..self.config.position_iterations {
            self.solve_position_iteration(bodies, dt);
        }

        for _ in 0..self.config.velocity_iterations {
            self.solve_velocity_iteration(bodies, dt, &mut events);
        }

        events
    }

    /// Performs one position correction iteration.
    fn solve_position_iteration(&mut self, bodies: &mut BodyStates, dt: f32) {
        let damping = self.config.position_damping;
        let order = self.solve_order.clone();

        for &idx in &order {
            let constraint = &mut self.constraints[idx];
            let (id_a, id_b) = constraint.body_ids();

            let snapshot_a = id_a.and_then(|id| bodies.get(id)).copied();
            let snapshot_b = id_b.and_then(|id| bodies.get(id)).copied();

            let mut body_a = snapshot_a.unwrap_or_default();
            let mut body_b = snapshot_b.unwrap_or_default();

            match constraint {
                Constraint::Distance(c) => {
                    let _ = c.solve_position(&mut body_a, &mut body_b, dt, damping);
                }
                Constraint::Fixed(c) => {
                    let _ = c.solve_position(&mut body_a, &mut body_b, dt, damping);
                    let _ = c.solve_angular(&mut body_a, &mut body_b, dt, damping);
                }
                Constraint::Hinge(c) => {
                    let _ = c.solve_position(&mut body_a, &mut body_b, dt, damping);
                    let _ = c.solve_angle_limit(&mut body_a, &mut body_b, dt, damping);
                }
                Constraint::Slider(c) => {
                    let _ = c.solve_off_axis(&mut body_a, &mut body_b, dt, damping);
                    let _ = c.solve_position_limit(&mut body_a, &mut body_b, dt, damping);
                }
                Constraint::Spring(c) => {
                    let _ = c.solve_position(&mut body_a, &mut body_b, dt, damping);
                }
            }

            if let Some(id) = id_a
                && let Some(body) = bodies.get_mut(id)
            {
                *body = body_a;
            }
            if let Some(id) = id_b
                && let Some(body) = bodies.get_mut(id)
            {
                *body = body_b;
            }
        }
    }

    /// Performs one velocity correction iteration.
    fn solve_velocity_iteration(
        &mut self,
        bodies: &mut BodyStates,
        dt: f32,
        _events: &mut ConstraintEvents,
    ) {
        let order = self.solve_order.clone();

        for &idx in &order {
            let constraint = &mut self.constraints[idx];
            let (id_a, id_b) = constraint.body_ids();

            let snapshot_a = id_a.and_then(|id| bodies.get(id)).copied();
            let snapshot_b = id_b.and_then(|id| bodies.get(id)).copied();

            let mut body_a = snapshot_a.unwrap_or_default();
            let mut body_b = snapshot_b.unwrap_or_default();

            match constraint {
                Constraint::Distance(c) => {
                    let _ = c.solve_velocity(&mut body_a, &mut body_b, dt);
                }
                Constraint::Fixed(c) => {
                    c.solve_velocity(&mut body_a, &mut body_b, dt);
                }
                Constraint::Hinge(c) => {
                    c.solve_motor(&mut body_a, &mut body_b, dt);
                }
                Constraint::Slider(c) => {
                    c.solve_motor(&mut body_a, &mut body_b, dt);
                }
                Constraint::Spring(c) => {
                    c.solve_velocity(&mut body_a, &mut body_b, dt);
                }
            }

            if let Some(id) = id_a
                && let Some(body) = bodies.get_mut(id)
            {
                *body = body_a;
            }
            if let Some(id) = id_b
                && let Some(body) = bodies.get_mut(id)
            {
                *body = body_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::anchor::ConstraintEndpoint;
    use approx::assert_relative_eq;

    #[test]
    fn solver_add_remove_constraint() {
        let mut solver = ConstraintSolver::new();

        let constraint = DistanceConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            5.0,
        );

        solver.add_constraint(constraint);
        assert_eq!(solver.len(), 1);

        solver.remove_constraint(ConstraintId::new(1));
        assert!(solver.is_empty());
    }

    #[test]
    fn solver_deterministic_order() {
        let mut solver = ConstraintSolver::new();

        for i in (0..5).rev() {
            let constraint = DistanceConstraint::new(
                ConstraintId::new(i),
                ConstraintEndpoint::body(BodyId::new(0)),
                ConstraintEndpoint::body(BodyId::new(1)),
                5.0,
            );
            solver.add_constraint(constraint);
        }

        solver.sort_by_id();

        let order: Vec<_> = solver
            .solve_order
            .iter()
            .map(|&i| solver.constraints[i].id().raw())
            .collect();

        assert_eq!(order, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn solver_solves_distance_constraint() {
        let mut solver =
            ConstraintSolver::with_config(SolverConfig::default().with_position_iterations(10));

        let constraint = DistanceConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            5.0,
        );
        solver.add_constraint(constraint);

        let mut bodies = BodyStates::new();
        bodies.insert(BodyId::new(0), BodySnapshot::new(Vec3::ZERO).with_mass(1.0));
        bodies.insert(
            BodyId::new(1),
            BodySnapshot::new(Vec3::new(10.0, 0.0, 0.0)).with_mass(1.0),
        );

        let initial_distance = (bodies.get(BodyId::new(1)).unwrap().position
            - bodies.get(BodyId::new(0)).unwrap().position)
            .length();

        solver.solve(&mut bodies, 1.0 / 60.0);

        let final_distance = (bodies.get(BodyId::new(1)).unwrap().position
            - bodies.get(BodyId::new(0)).unwrap().position)
            .length();

        assert!(final_distance < initial_distance);
        assert!((final_distance - 5.0).abs() < 0.5);
    }

    #[test]
    fn body_states_operations() {
        let mut states = BodyStates::new();
        assert!(states.is_empty());

        states.insert(BodyId::new(1), BodySnapshot::new(Vec3::X));
        states.insert(BodyId::new(2), BodySnapshot::new(Vec3::Y));

        assert_eq!(states.len(), 2);
        assert!(states.get(BodyId::new(1)).is_some());
        assert!(states.get(BodyId::new(3)).is_none());

        let body = states.get_mut(BodyId::new(1)).unwrap();
        body.position = Vec3::Z;

        let pos = states.get(BodyId::new(1)).unwrap().position;
        assert_relative_eq!(pos.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(pos.y, 0.0, epsilon = 1e-6);
        assert_relative_eq!(pos.z, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn breakable_constraint_breaks() {
        let constraint = DistanceConstraint::new(
            ConstraintId::new(1),
            ConstraintEndpoint::body(BodyId::new(0)),
            ConstraintEndpoint::body(BodyId::new(1)),
            5.0,
        );

        let mut breakable =
            BreakableConstraint::new(constraint.into(), BreakParams::with_max_force(100.0));

        assert!(!breakable.is_broken());

        breakable.accumulate_force(50.0);
        assert!(breakable.check_break().is_none());
        assert!(!breakable.is_broken());

        breakable.accumulate_force(60.0);
        let event = breakable.check_break();
        assert!(event.is_some());
        assert!(breakable.is_broken());
    }

    #[test]
    fn constraint_from_impls() {
        let distance =
            DistanceConstraint::new(ConstraintId::new(1), Vec3::ZERO.into(), Vec3::X.into(), 5.0);
        let c: Constraint = distance.into();
        assert!(matches!(c, Constraint::Distance(_)));

        let fixed = FixedConstraint::new(ConstraintId::new(2), Vec3::ZERO.into(), Vec3::X.into());
        let c: Constraint = fixed.into();
        assert!(matches!(c, Constraint::Fixed(_)));

        let hinge = HingeConstraint::new(
            ConstraintId::new(3),
            Vec3::ZERO.into(),
            Vec3::X.into(),
            Vec3::Z,
        );
        let c: Constraint = hinge.into();
        assert!(matches!(c, Constraint::Hinge(_)));

        let slider = SliderConstraint::new(
            ConstraintId::new(4),
            Vec3::ZERO.into(),
            Vec3::X.into(),
            Vec3::Y,
        );
        let c: Constraint = slider.into();
        assert!(matches!(c, Constraint::Slider(_)));

        let spring =
            SpringConstraint::new(ConstraintId::new(5), Vec3::ZERO.into(), Vec3::X.into(), 5.0);
        let c: Constraint = spring.into();
        assert!(matches!(c, Constraint::Spring(_)));
    }

    #[test]
    fn constraint_serialization() {
        let constraint = Constraint::Distance(DistanceConstraint::new(
            ConstraintId::new(42),
            Vec3::ZERO.into(),
            Vec3::X.into(),
            10.0,
        ));

        let json = serde_json::to_string(&constraint).unwrap();
        let recovered: Constraint = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.id(), ConstraintId::new(42));
    }
}

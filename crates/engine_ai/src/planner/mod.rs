//! Deterministic bounded planner for high-agency AI.
//!
//! Provides goal-oriented action planning with configurable bounds,
//! belief state tracking, and plan execution management.

pub mod action;
pub mod config;
pub mod facts;
pub mod ids;
pub mod intent;
#[expect(
    clippy::module_inception,
    reason = "planner::planner keeps the main planner type beside support modules"
)]
pub mod planner;
pub mod state;

pub use action::{ActionCost, ActionDef, ActionRegistry, ActionUtility, RiskLevel};
pub use config::{ExecutionConfig, PlanSelectionMode, PlannerConfig, PlannerLimit, PlannerStats};
pub use facts::{BeliefFingerprint, BeliefState, FactModification, FactRequirement, FactValue};
pub use ids::{
    ActionDefId, ActionInstanceId, ActorId, FactId, FactionScopeId, IntentId, LocationId, PlanId,
    ResourceTypeId,
};
pub use intent::{ActiveIntent, Intent, IntentParams, IntentPriority, IntentSet, IntentTag};
pub use planner::{
    PartialReason, Plan, PlanFailure, PlanFingerprint, PlanResult, PlannedAction, Planner,
};
pub use state::{
    ActorPlanAssignment, ExecutionFailure, PlanEvent, PlanState, PlanStatus, PlanTracker,
    PlanTrackerFingerprint, StepState, StepStatus,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannerSnapshot {
    pub config: PlannerConfig,
    pub tracker: PlanTracker,
}

impl PlannerSnapshot {
    #[must_use]
    pub fn new(config: PlannerConfig, tracker: PlanTracker) -> Self {
        Self { config, tracker }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannerSummary {
    pub active_actor_plans: usize,
    pub active_faction_plans: usize,
    pub completed_plans: usize,
    pub failed_plans: usize,
}

impl PlannerSummary {
    #[must_use]
    pub fn from_tracker(tracker: &PlanTracker) -> Self {
        Self {
            active_actor_plans: tracker.actor_count(),
            active_faction_plans: tracker.faction_count(),
            completed_plans: tracker.completed_plan_count(),
            failed_plans: tracker.failed_plan_count(),
        }
    }

    #[must_use]
    pub fn total_active(&self) -> usize {
        self.active_actor_plans + self.active_faction_plans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_summary() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan = Plan::new(PlanId::new(1), IntentId::new("test"), 0);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        let summary = PlannerSummary::from_tracker(&tracker);
        assert_eq!(summary.active_actor_plans, 1);
        assert_eq!(summary.total_active(), 1);
    }

    #[test]
    fn test_planner_snapshot_serde() {
        let config = PlannerConfig::default();
        let tracker = PlanTracker::new(ExecutionConfig::default());
        let snapshot = PlannerSnapshot::new(config, tracker);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: PlannerSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored.config.max_plan_depth,
            snapshot.config.max_plan_depth
        );
    }
}

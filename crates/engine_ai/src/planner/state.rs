//! Runtime plan state and tracker for actors and factions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::config::ExecutionConfig;
use super::ids::{ActionDefId, ActionInstanceId, ActorId, FactionScopeId, PlanId};
use super::planner::Plan;

/// Status of a plan step.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum StepStatus {
    #[default]
    Pending,
    Active,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }

    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

/// Status of an executing plan.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PlanStatus {
    #[default]
    Pending,
    Executing,
    Completed,
    Failed,
    Invalidated,
    Cancelled,
}

impl PlanStatus {
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Invalidated | Self::Cancelled
        )
    }

    #[must_use]
    pub fn is_executing(self) -> bool {
        matches!(self, Self::Executing)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Invalidated => "invalidated",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Reason for plan failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionFailure {
    StepFailed(ActionInstanceId, String),
    MaxRetriesExceeded,
    ProgressTimeout,
    PreconditionsInvalidated,
    ExternalInterruption,
}

impl ExecutionFailure {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StepFailed(_, _) => "step_failed",
            Self::MaxRetriesExceeded => "max_retries_exceeded",
            Self::ProgressTimeout => "progress_timeout",
            Self::PreconditionsInvalidated => "preconditions_invalidated",
            Self::ExternalInterruption => "external_interruption",
        }
    }
}

/// Runtime state for a single plan step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepState {
    pub instance_id: ActionInstanceId,
    pub action_def_id: ActionDefId,
    pub status: StepStatus,
    pub attempts: u32,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    pub failure_reason: Option<String>,
}

impl StepState {
    #[must_use]
    pub fn new(instance_id: ActionInstanceId, action_def_id: ActionDefId) -> Self {
        Self {
            instance_id,
            action_def_id,
            status: StepStatus::Pending,
            attempts: 0,
            started_tick: None,
            completed_tick: None,
            failure_reason: None,
        }
    }

    pub fn start(&mut self, tick: u64) {
        self.status = StepStatus::Active;
        self.started_tick = Some(tick);
        self.attempts += 1;
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = StepStatus::Completed;
        self.completed_tick = Some(tick);
    }

    pub fn fail(&mut self, tick: u64, reason: impl Into<String>) {
        self.status = StepStatus::Failed;
        self.completed_tick = Some(tick);
        self.failure_reason = Some(reason.into());
    }

    pub fn skip(&mut self, tick: u64) {
        self.status = StepStatus::Skipped;
        self.completed_tick = Some(tick);
    }

    pub fn reset(&mut self) {
        self.status = StepStatus::Pending;
        self.started_tick = None;
        self.completed_tick = None;
        self.failure_reason = None;
    }

    #[must_use]
    pub fn duration_ticks(&self) -> Option<u64> {
        match (self.started_tick, self.completed_tick) {
            (Some(start), Some(end)) => Some(end.saturating_sub(start)),
            _ => None,
        }
    }
}

/// Runtime state for executing a plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanState {
    pub plan: Plan,
    pub status: PlanStatus,
    pub current_step_index: usize,
    pub steps: Vec<StepState>,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    pub last_progress_tick: u64,
    pub failure: Option<ExecutionFailure>,
    pub replan_count: u32,
}

impl PlanState {
    #[must_use]
    pub fn new(plan: Plan, created_tick: u64) -> Self {
        let steps = plan
            .actions
            .iter()
            .map(|a| StepState::new(a.instance_id, a.action_def_id.clone()))
            .collect();

        Self {
            plan,
            status: PlanStatus::Pending,
            current_step_index: 0,
            steps,
            started_tick: None,
            completed_tick: None,
            last_progress_tick: created_tick,
            failure: None,
            replan_count: 0,
        }
    }

    pub fn start(&mut self, tick: u64) {
        self.status = PlanStatus::Executing;
        self.started_tick = Some(tick);
        self.last_progress_tick = tick;
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = PlanStatus::Completed;
        self.completed_tick = Some(tick);
    }

    pub fn fail(&mut self, tick: u64, failure: ExecutionFailure) {
        self.status = PlanStatus::Failed;
        self.completed_tick = Some(tick);
        self.failure = Some(failure);
    }

    pub fn invalidate(&mut self, tick: u64) {
        self.status = PlanStatus::Invalidated;
        self.completed_tick = Some(tick);
    }

    pub fn cancel(&mut self, tick: u64) {
        self.status = PlanStatus::Cancelled;
        self.completed_tick = Some(tick);
    }

    #[must_use]
    pub fn current_step(&self) -> Option<&StepState> {
        self.steps.get(self.current_step_index)
    }

    pub fn current_step_mut(&mut self) -> Option<&mut StepState> {
        self.steps.get_mut(self.current_step_index)
    }

    pub fn advance_step(&mut self, tick: u64) -> bool {
        self.last_progress_tick = tick;
        if self.current_step_index + 1 < self.steps.len() {
            self.current_step_index += 1;
            true
        } else {
            false
        }
    }

    pub fn record_progress(&mut self, tick: u64) {
        self.last_progress_tick = tick;
    }

    #[must_use]
    pub fn completed_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count()
    }

    #[must_use]
    pub fn failed_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.status == StepStatus::Failed)
            .count()
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "step counts bounded")]
    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 1.0;
        }
        let completed = self.completed_step_count();
        completed as f32 / self.steps.len() as f32
    }

    #[must_use]
    pub fn all_steps_complete(&self) -> bool {
        self.steps.iter().all(|s| s.status.is_terminal())
    }

    #[must_use]
    pub fn is_stalled(&self, current_tick: u64, timeout: u64) -> bool {
        current_tick.saturating_sub(self.last_progress_tick) > timeout
    }

    #[must_use]
    pub fn duration_ticks(&self) -> Option<u64> {
        match (self.started_tick, self.completed_tick) {
            (Some(start), Some(end)) => Some(end.saturating_sub(start)),
            _ => None,
        }
    }
}

/// Assignment of a plan to an actor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActorPlanAssignment {
    pub actor_id: ActorId,
    pub plan_state: PlanState,
    pub assigned_tick: u64,
}

impl ActorPlanAssignment {
    #[must_use]
    pub fn new(actor_id: ActorId, plan: Plan, assigned_tick: u64) -> Self {
        Self {
            actor_id,
            plan_state: PlanState::new(plan, assigned_tick),
            assigned_tick,
        }
    }
}

/// Tracker for plans assigned to actors and factions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanTracker {
    actor_plans: BTreeMap<ActorId, ActorPlanAssignment>,
    faction_plans: BTreeMap<FactionScopeId, Vec<PlanState>>,
    config: ExecutionConfig,
    completed_plans: Vec<PlanId>,
    failed_plans: Vec<PlanId>,
}

impl PlanTracker {
    #[must_use]
    pub fn new(config: ExecutionConfig) -> Self {
        Self {
            actor_plans: BTreeMap::new(),
            faction_plans: BTreeMap::new(),
            config,
            completed_plans: Vec::new(),
            failed_plans: Vec::new(),
        }
    }

    pub fn assign_to_actor(&mut self, actor_id: ActorId, plan: Plan, tick: u64) {
        let assignment = ActorPlanAssignment::new(actor_id, plan, tick);
        self.actor_plans.insert(actor_id, assignment);
    }

    pub fn assign_to_faction(&mut self, faction_id: FactionScopeId, plan: Plan, tick: u64) {
        let state = PlanState::new(plan, tick);
        self.faction_plans
            .entry(faction_id)
            .or_default()
            .push(state);
    }

    #[must_use]
    pub fn get_actor_plan(&self, actor_id: &ActorId) -> Option<&ActorPlanAssignment> {
        self.actor_plans.get(actor_id)
    }

    pub fn get_actor_plan_mut(&mut self, actor_id: &ActorId) -> Option<&mut ActorPlanAssignment> {
        self.actor_plans.get_mut(actor_id)
    }

    #[must_use]
    pub fn get_faction_plans(&self, faction_id: &FactionScopeId) -> Option<&[PlanState]> {
        self.faction_plans.get(faction_id).map(Vec::as_slice)
    }

    pub fn remove_actor_plan(&mut self, actor_id: &ActorId) -> Option<ActorPlanAssignment> {
        self.actor_plans.remove(actor_id)
    }

    pub fn clear_faction_plans(&mut self, faction_id: &FactionScopeId) {
        self.faction_plans.remove(faction_id);
    }

    #[must_use]
    pub fn actor_count(&self) -> usize {
        self.actor_plans.len()
    }

    #[must_use]
    pub fn faction_count(&self) -> usize {
        self.faction_plans.len()
    }

    #[must_use]
    pub fn total_plan_count(&self) -> usize {
        self.actor_plans.len() + self.faction_plans.values().map(Vec::len).sum::<usize>()
    }

    #[must_use]
    pub fn active_plan_count(&self) -> usize {
        let actor_active = self
            .actor_plans
            .values()
            .filter(|a| a.plan_state.status.is_executing())
            .count();
        let faction_active: usize = self
            .faction_plans
            .values()
            .flat_map(|plans| plans.iter())
            .filter(|p| p.status.is_executing())
            .count();
        actor_active + faction_active
    }

    pub fn tick(&mut self, current_tick: u64) -> Vec<PlanEvent> {
        let mut events = Vec::new();

        for (actor_id, assignment) in &mut self.actor_plans {
            let state = &mut assignment.plan_state;

            if !state.status.is_executing() {
                continue;
            }

            if state.is_stalled(current_tick, self.config.progress_timeout_ticks) {
                let plan_id = state.plan.id;
                state.fail(current_tick, ExecutionFailure::ProgressTimeout);
                self.failed_plans.push(plan_id);
                events.push(PlanEvent::PlanFailed {
                    plan_id,
                    actor_id: Some(*actor_id),
                    faction_id: None,
                    reason: "progress_timeout".to_string(),
                });
            }
        }

        for (faction_id, plans) in &mut self.faction_plans {
            for state in plans.iter_mut() {
                if !state.status.is_executing() {
                    continue;
                }

                if state.is_stalled(current_tick, self.config.progress_timeout_ticks) {
                    let plan_id = state.plan.id;
                    state.fail(current_tick, ExecutionFailure::ProgressTimeout);
                    self.failed_plans.push(plan_id);
                    events.push(PlanEvent::PlanFailed {
                        plan_id,
                        actor_id: None,
                        faction_id: Some(faction_id.clone()),
                        reason: "progress_timeout".to_string(),
                    });
                }
            }
        }

        events
    }

    pub fn complete_actor_plan(&mut self, actor_id: &ActorId, tick: u64) -> Option<PlanId> {
        if let Some(assignment) = self.actor_plans.get_mut(actor_id) {
            let plan_id = assignment.plan_state.plan.id;
            assignment.plan_state.complete(tick);
            self.completed_plans.push(plan_id);
            Some(plan_id)
        } else {
            None
        }
    }

    pub fn fail_actor_plan(
        &mut self,
        actor_id: &ActorId,
        tick: u64,
        failure: ExecutionFailure,
    ) -> Option<PlanId> {
        if let Some(assignment) = self.actor_plans.get_mut(actor_id) {
            let plan_id = assignment.plan_state.plan.id;
            assignment.plan_state.fail(tick, failure);
            self.failed_plans.push(plan_id);
            Some(plan_id)
        } else {
            None
        }
    }

    #[must_use]
    pub fn completed_plan_count(&self) -> usize {
        self.completed_plans.len()
    }

    #[must_use]
    pub fn failed_plan_count(&self) -> usize {
        self.failed_plans.len()
    }

    pub fn clear_history(&mut self) {
        self.completed_plans.clear();
        self.failed_plans.clear();
    }

    pub fn prune_terminal(&mut self) {
        self.actor_plans
            .retain(|_, a| !a.plan_state.status.is_terminal());
        for plans in self.faction_plans.values_mut() {
            plans.retain(|p| !p.status.is_terminal());
        }
        self.faction_plans.retain(|_, v| !v.is_empty());
    }

    pub fn iter_actor_plans(&self) -> impl Iterator<Item = (&ActorId, &ActorPlanAssignment)> {
        self.actor_plans.iter()
    }

    #[must_use]
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }

    #[must_use]
    pub fn fingerprint(&self) -> PlanTrackerFingerprint {
        let mut hasher = crc32fast::Hasher::new();
        for (actor_id, assignment) in &self.actor_plans {
            hasher.update(&actor_id.raw().to_le_bytes());
            hasher.update(&assignment.plan_state.plan.id.raw().to_le_bytes());
            hasher.update(&[assignment.plan_state.status as u8]);
        }
        for (faction_id, plans) in &self.faction_plans {
            hasher.update(faction_id.as_str().as_bytes());
            #[expect(clippy::cast_possible_truncation, reason = "plan count bounded")]
            hasher.update(&(plans.len() as u32).to_le_bytes());
        }
        PlanTrackerFingerprint {
            checksum: hasher.finalize(),
            actor_plan_count: self.actor_plans.len(),
            faction_plan_count: self.faction_plans.len(),
            completed_count: self.completed_plans.len(),
            failed_count: self.failed_plans.len(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanTrackerFingerprint {
    pub checksum: u32,
    pub actor_plan_count: usize,
    pub faction_plan_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
}

/// Events emitted during plan tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlanEvent {
    PlanAssigned {
        plan_id: PlanId,
        actor_id: Option<ActorId>,
        faction_id: Option<FactionScopeId>,
    },
    PlanStarted {
        plan_id: PlanId,
        actor_id: Option<ActorId>,
        faction_id: Option<FactionScopeId>,
    },
    StepStarted {
        plan_id: PlanId,
        step_index: usize,
        action_id: ActionDefId,
    },
    StepCompleted {
        plan_id: PlanId,
        step_index: usize,
        action_id: ActionDefId,
    },
    StepFailed {
        plan_id: PlanId,
        step_index: usize,
        action_id: ActionDefId,
        reason: String,
    },
    PlanCompleted {
        plan_id: PlanId,
        actor_id: Option<ActorId>,
        faction_id: Option<FactionScopeId>,
    },
    PlanFailed {
        plan_id: PlanId,
        actor_id: Option<ActorId>,
        faction_id: Option<FactionScopeId>,
        reason: String,
    },
    ReplanRequested {
        plan_id: PlanId,
        actor_id: Option<ActorId>,
        faction_id: Option<FactionScopeId>,
    },
}

impl PlanEvent {
    #[must_use]
    pub fn plan_id(&self) -> PlanId {
        match self {
            Self::PlanAssigned { plan_id, .. }
            | Self::PlanStarted { plan_id, .. }
            | Self::StepStarted { plan_id, .. }
            | Self::StepCompleted { plan_id, .. }
            | Self::StepFailed { plan_id, .. }
            | Self::PlanCompleted { plan_id, .. }
            | Self::PlanFailed { plan_id, .. }
            | Self::ReplanRequested { plan_id, .. } => *plan_id,
        }
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::planner::ids::IntentId;

    fn make_test_plan(id: u64) -> Plan {
        let mut plan = Plan::new(PlanId::new(id), IntentId::new("test"), 0);
        plan.add_action(
            crate::planner::planner::PlannedAction {
                instance_id: ActionInstanceId::new(1),
                action_def_id: ActionDefId::new("action1"),
                expected_cost: 5.0,
                expected_utility: 10.0,
                precondition_count: 1,
                effect_count: 1,
            },
            10,
        );
        plan.add_action(
            crate::planner::planner::PlannedAction {
                instance_id: ActionInstanceId::new(2),
                action_def_id: ActionDefId::new("action2"),
                expected_cost: 3.0,
                expected_utility: 8.0,
                precondition_count: 0,
                effect_count: 1,
            },
            5,
        );
        plan
    }

    #[test]
    fn test_step_status() {
        assert!(!StepStatus::Pending.is_terminal());
        assert!(StepStatus::Completed.is_terminal());
        assert!(StepStatus::Active.is_active());
        assert_eq!(StepStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_plan_status() {
        assert!(!PlanStatus::Executing.is_terminal());
        assert!(PlanStatus::Failed.is_terminal());
        assert!(PlanStatus::Executing.is_executing());
        assert_eq!(PlanStatus::Invalidated.as_str(), "invalidated");
    }

    #[test]
    fn test_step_state_lifecycle() {
        let mut step = StepState::new(ActionInstanceId::new(1), ActionDefId::new("test"));

        assert_eq!(step.status, StepStatus::Pending);
        assert_eq!(step.attempts, 0);

        step.start(100);
        assert_eq!(step.status, StepStatus::Active);
        assert_eq!(step.attempts, 1);
        assert_eq!(step.started_tick, Some(100));

        step.complete(150);
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.duration_ticks(), Some(50));
    }

    #[test]
    fn test_step_state_failure() {
        let mut step = StepState::new(ActionInstanceId::new(1), ActionDefId::new("test"));
        step.start(100);
        step.fail(110, "precondition violated");

        assert_eq!(step.status, StepStatus::Failed);
        assert_eq!(
            step.failure_reason,
            Some("precondition violated".to_string())
        );
    }

    #[test]
    fn test_plan_state_creation() {
        let plan = make_test_plan(1);
        let state = PlanState::new(plan, 0);

        assert_eq!(state.status, PlanStatus::Pending);
        assert_eq!(state.steps.len(), 2);
        assert_eq!(state.current_step_index, 0);
    }

    #[test]
    fn test_plan_state_execution() {
        let plan = make_test_plan(1);
        let mut state = PlanState::new(plan, 0);

        state.start(10);
        assert_eq!(state.status, PlanStatus::Executing);

        state.current_step_mut().unwrap().start(10);
        state.current_step_mut().unwrap().complete(20);

        assert!(state.advance_step(20));
        assert_eq!(state.current_step_index, 1);
        assert!((state.progress() - 0.5).abs() < f32::EPSILON);

        state.current_step_mut().unwrap().start(20);
        state.current_step_mut().unwrap().complete(30);

        assert!(!state.advance_step(30));
        assert!(state.all_steps_complete());

        state.complete(30);
        assert_eq!(state.status, PlanStatus::Completed);
    }

    #[test]
    fn test_plan_state_stall_detection() {
        let plan = make_test_plan(1);
        let mut state = PlanState::new(plan, 0);
        state.start(0);

        assert!(!state.is_stalled(50, 100));
        assert!(state.is_stalled(150, 100));

        state.record_progress(100);
        assert!(!state.is_stalled(150, 100));
    }

    #[test]
    fn test_plan_tracker() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan1 = make_test_plan(1);
        let plan2 = make_test_plan(2);

        tracker.assign_to_actor(ActorId::new(1), plan1, 0);
        tracker.assign_to_faction(FactionScopeId::new("guild"), plan2, 0);

        assert_eq!(tracker.actor_count(), 1);
        assert_eq!(tracker.faction_count(), 1);
        assert_eq!(tracker.total_plan_count(), 2);
    }

    #[test]
    fn test_plan_tracker_completion() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        let plan_id = tracker.complete_actor_plan(&ActorId::new(1), 100);
        assert!(plan_id.is_some());
        assert_eq!(tracker.completed_plan_count(), 1);
    }

    #[test]
    fn test_plan_tracker_failure() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        let plan_id =
            tracker.fail_actor_plan(&ActorId::new(1), 100, ExecutionFailure::MaxRetriesExceeded);
        assert!(plan_id.is_some());
        assert_eq!(tracker.failed_plan_count(), 1);
    }

    #[test]
    fn test_plan_tracker_tick_timeout() {
        let config = ExecutionConfig::default().with_progress_timeout(50);
        let mut tracker = PlanTracker::new(config);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        if let Some(assignment) = tracker.get_actor_plan_mut(&ActorId::new(1)) {
            assignment.plan_state.start(0);
        }

        let events = tracker.tick(100);
        assert_eq!(events.len(), 1);
        match &events[0] {
            PlanEvent::PlanFailed { reason, .. } => {
                assert_eq!(reason, "progress_timeout");
            }
            _ => panic!("expected PlanFailed event"),
        }
    }

    #[test]
    fn test_plan_tracker_prune() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        tracker.complete_actor_plan(&ActorId::new(1), 100);
        assert_eq!(tracker.actor_count(), 1);

        tracker.prune_terminal();
        assert_eq!(tracker.actor_count(), 0);
    }

    #[test]
    fn test_plan_event() {
        let event = PlanEvent::PlanCompleted {
            plan_id: PlanId::new(42),
            actor_id: Some(ActorId::new(1)),
            faction_id: None,
        };

        assert_eq!(event.plan_id(), PlanId::new(42));
    }

    #[test]
    fn test_step_state_serde() {
        let mut step = StepState::new(ActionInstanceId::new(1), ActionDefId::new("test"));
        step.start(100);
        step.complete(150);

        let json = serde_json::to_string(&step).unwrap();
        let restored: StepState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.status, StepStatus::Completed);
        assert_eq!(restored.duration_ticks(), Some(50));
    }

    #[test]
    fn test_plan_state_serde() {
        let plan = make_test_plan(1);
        let mut state = PlanState::new(plan, 0);
        state.start(10);

        let json = serde_json::to_string(&state).unwrap();
        let restored: PlanState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.status, PlanStatus::Executing);
        assert_eq!(restored.steps.len(), 2);
    }

    #[test]
    fn test_plan_tracker_serde() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: PlanTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.actor_count(), 1);
    }

    #[test]
    fn test_plan_tracker_fingerprint() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let fp1 = tracker.fingerprint();
        assert_eq!(fp1.actor_plan_count, 0);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        let fp2 = tracker.fingerprint();
        assert_eq!(fp2.actor_plan_count, 1);
        assert_ne!(fp1.checksum, fp2.checksum);
    }

    #[test]
    fn test_step_state_bincode() {
        let mut step = StepState::new(ActionInstanceId::new(1), ActionDefId::new("test"));
        step.start(100);
        step.complete(150);

        let bytes = bincode::serialize(&step).unwrap();
        let restored: StepState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.status, StepStatus::Completed);
        assert_eq!(restored.duration_ticks(), Some(50));
    }

    #[test]
    fn test_plan_state_bincode() {
        let plan = make_test_plan(1);
        let mut state = PlanState::new(plan, 0);
        state.start(10);

        let bytes = bincode::serialize(&state).unwrap();
        let restored: PlanState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.status, PlanStatus::Executing);
        assert_eq!(restored.steps.len(), 2);
    }

    #[test]
    fn test_plan_tracker_bincode() {
        let config = ExecutionConfig::default();
        let mut tracker = PlanTracker::new(config);

        let plan = make_test_plan(1);
        tracker.assign_to_actor(ActorId::new(1), plan, 0);

        let bytes = bincode::serialize(&tracker).unwrap();
        let restored: PlanTracker = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.actor_count(), 1);
    }

    #[test]
    fn test_plan_event_bincode() {
        let event = PlanEvent::PlanCompleted {
            plan_id: PlanId::new(42),
            actor_id: Some(ActorId::new(1)),
            faction_id: None,
        };

        let bytes = bincode::serialize(&event).unwrap();
        let restored: PlanEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.plan_id(), PlanId::new(42));
    }

    #[test]
    fn test_execution_failure_bincode() {
        let failure = ExecutionFailure::StepFailed(ActionInstanceId::new(5), "timeout".to_string());

        let bytes = bincode::serialize(&failure).unwrap();
        let restored: ExecutionFailure = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.as_str(), "step_failed");
    }

    #[test]
    fn test_plan_tracker_fingerprint_bincode() {
        let config = ExecutionConfig::default();
        let tracker = PlanTracker::new(config);
        let fp = tracker.fingerprint();

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: PlanTrackerFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum, fp.checksum);
    }
}

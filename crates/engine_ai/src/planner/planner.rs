//! Deterministic bounded planner for high-agency AI.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::action::{ActionDef, ActionRegistry};
use super::config::{PlanSelectionMode, PlannerConfig, PlannerLimit, PlannerStats};
use super::facts::BeliefState;
use super::ids::{ActionDefId, ActionInstanceId, IntentId, PlanId};
use super::intent::Intent;

/// Result of a planning attempt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlanResult {
    Success(Plan),
    Partial(Plan, PartialReason),
    Failure(PlanFailure),
}

impl PlanResult {
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    #[must_use]
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial(_, _))
    }

    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure(_))
    }

    #[must_use]
    pub fn plan(&self) -> Option<&Plan> {
        match self {
            Self::Success(p) | Self::Partial(p, _) => Some(p),
            Self::Failure(_) => None,
        }
    }

    #[must_use]
    pub fn into_plan(self) -> Option<Plan> {
        match self {
            Self::Success(p) | Self::Partial(p, _) => Some(p),
            Self::Failure(_) => None,
        }
    }
}

/// Reason for a partial plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartialReason {
    DepthLimitReached,
    IterationLimitReached,
    ActionLimitReached,
    TimeoutReached,
    NoProgressPossible,
}

impl PartialReason {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DepthLimitReached => "depth_limit_reached",
            Self::IterationLimitReached => "iteration_limit_reached",
            Self::ActionLimitReached => "action_limit_reached",
            Self::TimeoutReached => "timeout_reached",
            Self::NoProgressPossible => "no_progress_possible",
        }
    }
}

/// Reason for plan failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanFailure {
    NoApplicableActions,
    GoalUnreachable,
    AllPathsExhausted,
    DepthExceeded,
    IterationsExceeded,
    Timeout,
    InvalidIntent,
}

impl PlanFailure {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoApplicableActions => "no_applicable_actions",
            Self::GoalUnreachable => "goal_unreachable",
            Self::AllPathsExhausted => "all_paths_exhausted",
            Self::DepthExceeded => "depth_exceeded",
            Self::IterationsExceeded => "iterations_exceeded",
            Self::Timeout => "timeout",
            Self::InvalidIntent => "invalid_intent",
        }
    }
}

/// A planned action instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannedAction {
    pub instance_id: ActionInstanceId,
    pub action_def_id: ActionDefId,
    pub expected_cost: f32,
    pub expected_utility: f32,
    pub precondition_count: usize,
    pub effect_count: usize,
}

impl PlannedAction {
    #[must_use]
    pub fn from_def(instance_id: ActionInstanceId, def: &ActionDef) -> Self {
        Self {
            instance_id,
            action_def_id: def.id.clone(),
            expected_cost: def.cost.total_cost(),
            expected_utility: def.utility.total_utility(),
            precondition_count: def.precondition_count(),
            effect_count: def.effect_count(),
        }
    }

    #[must_use]
    pub fn score(&self) -> f32 {
        if self.expected_cost <= 0.0 {
            return self.expected_utility;
        }
        self.expected_utility / self.expected_cost
    }
}

/// A complete or partial plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub id: PlanId,
    pub intent_id: IntentId,
    pub actions: Vec<PlannedAction>,
    pub initial_state_checksum: u32,
    pub expected_final_state_checksum: u32,
    pub total_cost: f32,
    pub total_utility: f32,
    pub created_tick: u64,
    pub estimated_duration_ticks: u64,
}

impl Plan {
    #[must_use]
    pub fn new(id: PlanId, intent_id: IntentId, created_tick: u64) -> Self {
        Self {
            id,
            intent_id,
            actions: Vec::new(),
            initial_state_checksum: 0,
            expected_final_state_checksum: 0,
            total_cost: 0.0,
            total_utility: 0.0,
            created_tick,
            estimated_duration_ticks: 0,
        }
    }

    pub fn add_action(&mut self, action: PlannedAction, duration_ticks: u64) {
        self.total_cost += action.expected_cost;
        self.total_utility += action.expected_utility;
        self.estimated_duration_ticks += duration_ticks;
        self.actions.push(action);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    #[must_use]
    pub fn score(&self) -> f32 {
        if self.total_cost <= 0.0 {
            return self.total_utility;
        }
        self.total_utility / self.total_cost
    }

    #[must_use]
    pub fn action_ids(&self) -> Vec<ActionDefId> {
        self.actions
            .iter()
            .map(|a| a.action_def_id.clone())
            .collect()
    }

    #[must_use]
    pub fn first_action(&self) -> Option<&PlannedAction> {
        self.actions.first()
    }

    #[must_use]
    pub fn last_action(&self) -> Option<&PlannedAction> {
        self.actions.last()
    }

    #[must_use]
    pub fn fingerprint(&self) -> PlanFingerprint {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(self.intent_id.as_str().as_bytes());
        hasher.update(&self.initial_state_checksum.to_le_bytes());
        hasher.update(&self.expected_final_state_checksum.to_le_bytes());
        for action in &self.actions {
            hasher.update(&action.instance_id.raw().to_le_bytes());
            hasher.update(action.action_def_id.as_str().as_bytes());
        }
        PlanFingerprint {
            checksum: hasher.finalize(),
            action_count: self.actions.len(),
            plan_id: self.id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanFingerprint {
    pub checksum: u32,
    pub action_count: usize,
    pub plan_id: PlanId,
}

/// Search node for planning.
#[derive(Clone, Debug)]
struct SearchNode {
    state: BeliefState,
    actions: Vec<(ActionDefId, ActionInstanceId)>,
    g_cost: f32,
    depth: usize,
}

impl SearchNode {
    fn new(state: BeliefState) -> Self {
        Self {
            state,
            actions: Vec::new(),
            g_cost: 0.0,
            depth: 0,
        }
    }

    fn expand(
        &self,
        action_id: ActionDefId,
        instance_id: ActionInstanceId,
        new_state: BeliefState,
        action_cost: f32,
    ) -> Self {
        let mut new_actions = self.actions.clone();
        new_actions.push((action_id, instance_id));
        Self {
            state: new_state,
            actions: new_actions,
            g_cost: self.g_cost + action_cost,
            depth: self.depth + 1,
        }
    }
}

/// The deterministic bounded planner.
#[derive(Clone, Debug)]
pub struct Planner {
    config: PlannerConfig,
    selection_mode: PlanSelectionMode,
    next_plan_id: u64,
    next_instance_id: u64,
}

impl Planner {
    #[must_use]
    pub fn new(config: PlannerConfig) -> Self {
        Self {
            config,
            selection_mode: PlanSelectionMode::default(),
            next_plan_id: 1,
            next_instance_id: 1,
        }
    }

    #[must_use]
    pub fn with_selection_mode(mut self, mode: PlanSelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    #[must_use]
    pub fn config(&self) -> &PlannerConfig {
        &self.config
    }

    fn allocate_plan_id(&mut self) -> PlanId {
        let id = PlanId::new(self.next_plan_id);
        self.next_plan_id += 1;
        id
    }

    fn allocate_instance_id(&mut self) -> ActionInstanceId {
        let id = ActionInstanceId::new(self.next_instance_id);
        self.next_instance_id += 1;
        id
    }

    #[expect(
        clippy::too_many_lines,
        reason = "bounded search is easier to audit in one deterministic routine"
    )]
    pub fn plan(
        &mut self,
        intent: &Intent,
        initial_state: &BeliefState,
        actions: &ActionRegistry,
        current_tick: u64,
    ) -> (PlanResult, PlannerStats) {
        let mut stats = PlannerStats::new();

        if intent.goal_conditions.is_empty() {
            return (PlanResult::Failure(PlanFailure::InvalidIntent), stats);
        }

        if intent.is_satisfied(initial_state) {
            let plan_id = self.allocate_plan_id();
            let plan = Plan {
                id: plan_id,
                intent_id: intent.id.clone(),
                actions: Vec::new(),
                initial_state_checksum: initial_state.checksum(),
                expected_final_state_checksum: initial_state.checksum(),
                total_cost: 0.0,
                total_utility: 0.0,
                created_tick: current_tick,
                estimated_duration_ticks: 0,
            };
            return (PlanResult::Success(plan), stats);
        }

        let start_node = SearchNode::new(initial_state.clone());
        let mut open_list = vec![start_node];
        let mut found_plans: Vec<(SearchNode, BeliefState)> = Vec::new();
        let mut visited_checksums: BTreeSet<u32> = BTreeSet::new();
        visited_checksums.insert(initial_state.checksum());

        while !open_list.is_empty() && stats.iterations < self.config.max_search_iterations {
            stats.record_iteration();

            open_list.sort_by(|a, b| {
                a.g_cost
                    .partial_cmp(&b.g_cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let current = open_list.remove(0);

            if current.depth >= self.config.max_plan_depth {
                stats.record_limit(PlannerLimit::MaxDepth);
                if self.config.allow_partial_plans && !current.actions.is_empty() {
                    found_plans.push((current.clone(), current.state.clone()));
                }
                continue;
            }

            let applicable: Vec<_> = actions.applicable_actions(&current.state).collect();
            stats.record_expansion(applicable.len());

            if applicable.is_empty() {
                if self.config.allow_partial_plans && !current.actions.is_empty() {
                    found_plans.push((current.clone(), current.state.clone()));
                }
                continue;
            }

            let mut sorted_actions: Vec<_> = applicable.into_iter().collect();
            sorted_actions.sort_by(|a, b| {
                let score_a = a.score();
                let score_b = b.score();
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.as_str().cmp(b.id.as_str()))
            });

            let branch_limit = sorted_actions.len().min(self.config.max_branch_factor);
            for action_def in sorted_actions.into_iter().take(branch_limit) {
                if action_def.cost.risk.multiplier() < 1.0 - self.config.risk_tolerance {
                    continue;
                }

                let instance_id = self.allocate_instance_id();
                let new_state = action_def.apply(&current.state);
                let state_checksum = new_state.checksum();

                if visited_checksums.contains(&state_checksum) {
                    continue;
                }
                visited_checksums.insert(state_checksum);

                let new_node = current.expand(
                    action_def.id.clone(),
                    instance_id,
                    new_state.clone(),
                    action_def.cost.total_cost(),
                );

                if intent.is_satisfied(&new_state) {
                    stats.record_plan_found();
                    found_plans.push((new_node, new_state));

                    if !self.config.allow_partial_plans {
                        break;
                    }
                } else if new_node.actions.len() < self.config.max_plan_actions {
                    open_list.push(new_node);
                } else {
                    stats.record_limit(PlannerLimit::MaxActions);
                }
            }

            if !found_plans.is_empty() && !self.config.allow_partial_plans {
                break;
            }
        }

        if stats.iterations >= self.config.max_search_iterations {
            stats.record_limit(PlannerLimit::MaxIterations);
        }

        stats.record_depth(found_plans.iter().map(|(n, _)| n.depth).max().unwrap_or(0));

        if found_plans.is_empty() {
            return (PlanResult::Failure(PlanFailure::AllPathsExhausted), stats);
        }

        let (mut plan, final_state) =
            self.select_best_plan(&found_plans, intent, actions, current_tick);
        plan.initial_state_checksum = initial_state.checksum();

        let is_complete = intent.is_satisfied(&final_state);
        if is_complete {
            (PlanResult::Success(plan), stats)
        } else {
            let reason = if stats.limits_hit.contains(&PlannerLimit::MaxDepth) {
                PartialReason::DepthLimitReached
            } else if stats.limits_hit.contains(&PlannerLimit::MaxIterations) {
                PartialReason::IterationLimitReached
            } else {
                PartialReason::NoProgressPossible
            };
            (PlanResult::Partial(plan, reason), stats)
        }
    }

    fn select_best_plan(
        &mut self,
        candidates: &[(SearchNode, BeliefState)],
        intent: &Intent,
        actions: &ActionRegistry,
        current_tick: u64,
    ) -> (Plan, BeliefState) {
        let plan_id = self.allocate_plan_id();

        let scored: Vec<_> = candidates
            .iter()
            .enumerate()
            .map(|(idx, (node, final_state))| {
                let score = match self.selection_mode {
                    PlanSelectionMode::BestScore => {
                        let utility: f32 = node
                            .actions
                            .iter()
                            .filter_map(|(id, _)| actions.get(id))
                            .map(|a| a.utility.total_utility())
                            .sum();
                        if node.g_cost > 0.0 {
                            utility / node.g_cost
                        } else {
                            utility
                        }
                    }
                    #[expect(clippy::cast_precision_loss, reason = "action counts bounded")]
                    PlanSelectionMode::ShortestLength => -(node.actions.len() as f32),
                    PlanSelectionMode::LowestCost => -node.g_cost,
                    PlanSelectionMode::LowestRisk => {
                        let risk: f32 = node
                            .actions
                            .iter()
                            .filter_map(|(id, _)| actions.get(id))
                            .map(|a| 1.0 - a.cost.risk.multiplier())
                            .sum();
                        -risk
                    }
                    PlanSelectionMode::First => 0.0,
                };
                (idx, score, intent.is_satisfied(final_state))
            })
            .collect();

        let complete_plans: Vec<_> = scored.iter().filter(|(_, _, complete)| *complete).collect();

        let best_idx = if complete_plans.is_empty() {
            scored
                .iter()
                .max_by(|a, b| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.cmp(&b.0))
                })
                .map_or(0, |(idx, _, _)| *idx)
        } else {
            complete_plans
                .iter()
                .max_by(|a, b| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.cmp(&b.0))
                })
                .map_or(0, |(idx, _, _)| *idx)
        };

        let (node, final_state) = &candidates[best_idx];

        let mut plan = Plan::new(plan_id, intent.id.clone(), current_tick);
        plan.expected_final_state_checksum = final_state.checksum();

        for (action_id, instance_id) in &node.actions {
            if let Some(def) = actions.get(action_id) {
                let planned = PlannedAction::from_def(*instance_id, def);
                plan.add_action(planned, def.cost.time_ticks);
            }
        }

        (plan, final_state.clone())
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::planner::action::{ActionCost, ActionUtility};
    use crate::planner::facts::{FactModification, FactRequirement};

    fn setup_simple_scenario() -> (ActionRegistry, BeliefState, Intent) {
        let mut registry = ActionRegistry::new();

        let move_to = ActionDef::new("move_to_resource", "Move To Resource")
            .with_precondition(FactRequirement::is_false("at_resource"))
            .with_effect(FactModification::set_true("at_resource"))
            .with_cost(ActionCost::new(10))
            .with_utility(ActionUtility::new(5.0));

        let gather = ActionDef::new("gather", "Gather Resource")
            .with_precondition(FactRequirement::is_true("at_resource"))
            .with_effect(FactModification::increment("resources", 10))
            .with_cost(ActionCost::new(20))
            .with_utility(ActionUtility::new(15.0));

        registry.register(move_to);
        registry.register(gather);

        let mut state = BeliefState::new();
        state.set_bool("at_resource", false);
        state.set_int("resources", 0);

        let intent = Intent::new("gather_resources", "Gather Resources", 0)
            .with_goal_condition(FactRequirement::at_least("resources", 10));

        (registry, state, intent)
    }

    #[test]
    fn test_planner_simple_plan() {
        let (registry, world, intent) = setup_simple_scenario();
        let mut planner = Planner::new(PlannerConfig::default());

        let (result, run_stats) = planner.plan(&intent, &world, &registry, 0);

        assert!(result.is_success());
        let plan = result.plan().unwrap();
        assert_eq!(plan.len(), 2);
        assert!(run_stats.found_plans());
    }

    #[test]
    fn test_planner_already_satisfied() {
        let (registry, mut state, intent) = setup_simple_scenario();
        state.set_int("resources", 100);

        let mut planner = Planner::new(PlannerConfig::default());
        let (result, _stats) = planner.plan(&intent, &state, &registry, 0);

        assert!(result.is_success());
        let plan = result.plan().unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn test_planner_no_applicable_actions() {
        let registry = ActionRegistry::new();
        let state = BeliefState::new();
        let intent = Intent::new("impossible", "Impossible", 0)
            .with_goal_condition(FactRequirement::is_true("unreachable"));

        let mut planner = Planner::new(PlannerConfig::default());
        let (result, _stats) = planner.plan(&intent, &state, &registry, 0);

        assert!(result.is_failure());
    }

    #[test]
    fn test_planner_depth_limit() {
        let (registry, world, intent) = setup_simple_scenario();
        let config = PlannerConfig::new()
            .with_max_depth(1)
            .with_partial_plans(false);
        let mut planner = Planner::new(config);

        let (result, run_stats) = planner.plan(&intent, &world, &registry, 0);

        assert!(result.is_failure() || result.is_partial());
        assert!(run_stats.limits_hit.contains(&PlannerLimit::MaxDepth) || run_stats.hit_limits());
    }

    #[test]
    fn test_planner_partial_plan() {
        let (registry, state, intent) = setup_simple_scenario();
        let config = PlannerConfig::new()
            .with_max_depth(1)
            .with_partial_plans(true);
        let mut planner = Planner::new(config);

        let (result, _stats) = planner.plan(&intent, &state, &registry, 0);

        match result {
            PlanResult::Partial(plan, _) => {
                assert!(!plan.is_empty());
            }
            PlanResult::Success(_) | PlanResult::Failure(_) => {}
        }
    }

    #[test]
    fn test_planner_selection_shortest() {
        let (registry, state, intent) = setup_simple_scenario();
        let mut planner = Planner::new(PlannerConfig::default())
            .with_selection_mode(PlanSelectionMode::ShortestLength);

        let (result, _stats) = planner.plan(&intent, &state, &registry, 0);

        assert!(result.is_success());
    }

    #[test]
    fn test_plan_structure() {
        let mut plan = Plan::new(PlanId::new(1), IntentId::new("test"), 100);

        let action1 = PlannedAction {
            instance_id: ActionInstanceId::new(1),
            action_def_id: ActionDefId::new("action1"),
            expected_cost: 5.0,
            expected_utility: 10.0,
            precondition_count: 1,
            effect_count: 2,
        };

        let action2 = PlannedAction {
            instance_id: ActionInstanceId::new(2),
            action_def_id: ActionDefId::new("action2"),
            expected_cost: 3.0,
            expected_utility: 8.0,
            precondition_count: 0,
            effect_count: 1,
        };

        plan.add_action(action1, 10);
        plan.add_action(action2, 5);

        assert_eq!(plan.len(), 2);
        assert!((plan.total_cost - 8.0).abs() < f32::EPSILON);
        assert!((plan.total_utility - 18.0).abs() < f32::EPSILON);
        assert_eq!(plan.estimated_duration_ticks, 15);
    }

    #[test]
    fn test_planned_action_score() {
        let action = PlannedAction {
            instance_id: ActionInstanceId::new(1),
            action_def_id: ActionDefId::new("test"),
            expected_cost: 2.0,
            expected_utility: 10.0,
            precondition_count: 1,
            effect_count: 1,
        };

        assert!((action.score() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_plan_result_methods() {
        let plan = Plan::new(PlanId::new(1), IntentId::new("test"), 0);

        let success = PlanResult::Success(plan.clone());
        assert!(success.is_success());
        assert!(success.plan().is_some());

        let partial = PlanResult::Partial(plan.clone(), PartialReason::DepthLimitReached);
        assert!(partial.is_partial());
        assert!(partial.plan().is_some());

        let failure = PlanResult::Failure(PlanFailure::GoalUnreachable);
        assert!(failure.is_failure());
        assert!(failure.plan().is_none());
    }

    #[test]
    fn test_plan_serde() {
        let mut plan = Plan::new(PlanId::new(42), IntentId::acquire_resource(), 1000);
        plan.add_action(
            PlannedAction {
                instance_id: ActionInstanceId::new(1),
                action_def_id: ActionDefId::new("gather"),
                expected_cost: 10.0,
                expected_utility: 20.0,
                precondition_count: 1,
                effect_count: 2,
            },
            30,
        );

        let json = serde_json::to_string(&plan).unwrap();
        let restored: Plan = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, plan.id);
        assert_eq!(restored.len(), 1);
        assert!((restored.total_cost - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_plan_result_serde() {
        let plan = Plan::new(PlanId::new(1), IntentId::new("test"), 0);

        let result = PlanResult::Partial(plan, PartialReason::IterationLimitReached);
        let json = serde_json::to_string(&result).unwrap();
        let restored: PlanResult = serde_json::from_str(&json).unwrap();

        assert!(restored.is_partial());
    }

    #[test]
    fn test_plan_failure_serde() {
        let failure = PlanFailure::GoalUnreachable;
        let json = serde_json::to_string(&failure).unwrap();
        let restored: PlanFailure = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, PlanFailure::GoalUnreachable);
        assert_eq!(restored.as_str(), "goal_unreachable");
    }

    #[test]
    fn test_plan_fingerprint() {
        let mut plan1 = Plan::new(PlanId::new(1), IntentId::new("test"), 0);
        plan1.initial_state_checksum = 12345;
        plan1.add_action(
            PlannedAction {
                instance_id: ActionInstanceId::new(1),
                action_def_id: ActionDefId::new("action1"),
                expected_cost: 5.0,
                expected_utility: 10.0,
                precondition_count: 1,
                effect_count: 1,
            },
            10,
        );

        let fp1 = plan1.fingerprint();
        assert_eq!(fp1.action_count, 1);
        assert_eq!(fp1.plan_id, PlanId::new(1));

        let fp2 = plan1.fingerprint();
        assert_eq!(fp1.checksum, fp2.checksum);

        let mut plan2 = Plan::new(PlanId::new(2), IntentId::new("test"), 0);
        plan2.initial_state_checksum = 12345;
        let fp3 = plan2.fingerprint();
        assert_ne!(fp1.checksum, fp3.checksum);
    }

    #[test]
    fn test_plan_bincode() {
        let mut plan = Plan::new(PlanId::new(42), IntentId::acquire_resource(), 1000);
        plan.add_action(
            PlannedAction {
                instance_id: ActionInstanceId::new(1),
                action_def_id: ActionDefId::new("gather"),
                expected_cost: 10.0,
                expected_utility: 20.0,
                precondition_count: 1,
                effect_count: 2,
            },
            30,
        );

        let bytes = bincode::serialize(&plan).unwrap();
        let restored: Plan = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id, plan.id);
        assert_eq!(restored.len(), 1);
    }

    #[test]
    fn test_plan_result_bincode() {
        let plan = Plan::new(PlanId::new(1), IntentId::new("test"), 0);
        let result = PlanResult::Partial(plan, PartialReason::DepthLimitReached);

        let bytes = bincode::serialize(&result).unwrap();
        let restored: PlanResult = bincode::deserialize(&bytes).unwrap();

        assert!(restored.is_partial());
    }

    #[test]
    fn test_plan_fingerprint_bincode() {
        let plan = Plan::new(PlanId::new(1), IntentId::new("test"), 0);
        let fp = plan.fingerprint();

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: super::PlanFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum, fp.checksum);
        assert_eq!(restored.plan_id, fp.plan_id);
    }
}

//! Configuration and bounds for the planner.

use serde::{Deserialize, Serialize};

/// Configuration for the bounded planner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "planner config exposes explicit toggles for serialized game-pack tuning"
)]
pub struct PlannerConfig {
    pub max_plan_depth: usize,
    pub max_search_iterations: usize,
    pub max_plan_actions: usize,
    pub max_branch_factor: usize,
    pub timeout_ticks: u64,
    pub cost_weight: f32,
    pub utility_weight: f32,
    pub risk_tolerance: f32,
    pub prefer_shorter_plans: bool,
    pub allow_partial_plans: bool,
    pub replan_on_failure: bool,
    pub deterministic_tiebreak: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_plan_depth: 10,
            max_search_iterations: 1000,
            max_plan_actions: 20,
            max_branch_factor: 10,
            timeout_ticks: 0,
            cost_weight: 1.0,
            utility_weight: 1.0,
            risk_tolerance: 0.5,
            prefer_shorter_plans: true,
            allow_partial_plans: true,
            replan_on_failure: true,
            deterministic_tiebreak: true,
        }
    }
}

impl PlannerConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn strict() -> Self {
        Self {
            max_plan_depth: 5,
            max_search_iterations: 500,
            max_plan_actions: 10,
            max_branch_factor: 5,
            risk_tolerance: 0.2,
            allow_partial_plans: false,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn relaxed() -> Self {
        Self {
            max_plan_depth: 20,
            max_search_iterations: 5000,
            max_plan_actions: 50,
            max_branch_factor: 20,
            risk_tolerance: 0.8,
            allow_partial_plans: true,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn quick() -> Self {
        Self {
            max_plan_depth: 3,
            max_search_iterations: 100,
            max_plan_actions: 5,
            max_branch_factor: 5,
            prefer_shorter_plans: true,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_plan_depth = depth;
        self
    }

    #[must_use]
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_search_iterations = iterations;
        self
    }

    #[must_use]
    pub fn with_max_actions(mut self, actions: usize) -> Self {
        self.max_plan_actions = actions;
        self
    }

    #[must_use]
    pub fn with_risk_tolerance(mut self, tolerance: f32) -> Self {
        self.risk_tolerance = tolerance.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_partial_plans(mut self, allow: bool) -> Self {
        self.allow_partial_plans = allow;
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, ticks: u64) -> Self {
        self.timeout_ticks = ticks;
        self
    }

    #[must_use]
    pub fn is_within_bounds(&self, depth: usize, iterations: usize) -> bool {
        depth <= self.max_plan_depth && iterations <= self.max_search_iterations
    }
}

/// Limits hit during planning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlannerLimit {
    MaxDepth,
    MaxIterations,
    MaxActions,
    MaxBranchFactor,
    Timeout,
}

impl PlannerLimit {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MaxDepth => "max_depth",
            Self::MaxIterations => "max_iterations",
            Self::MaxActions => "max_actions",
            Self::MaxBranchFactor => "max_branch_factor",
            Self::Timeout => "timeout",
        }
    }
}

/// Statistics from a planning run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannerStats {
    pub iterations: usize,
    pub max_depth_reached: usize,
    pub nodes_expanded: usize,
    pub actions_considered: usize,
    pub plans_found: usize,
    pub elapsed_ticks: u64,
    pub limits_hit: Vec<PlannerLimit>,
}

impl PlannerStats {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_iteration(&mut self) {
        self.iterations += 1;
    }

    pub fn record_depth(&mut self, depth: usize) {
        self.max_depth_reached = self.max_depth_reached.max(depth);
    }

    pub fn record_expansion(&mut self, actions_count: usize) {
        self.nodes_expanded += 1;
        self.actions_considered += actions_count;
    }

    pub fn record_plan_found(&mut self) {
        self.plans_found += 1;
    }

    pub fn record_limit(&mut self, limit: PlannerLimit) {
        if !self.limits_hit.contains(&limit) {
            self.limits_hit.push(limit);
        }
    }

    pub fn set_elapsed(&mut self, ticks: u64) {
        self.elapsed_ticks = ticks;
    }

    #[must_use]
    pub fn hit_limits(&self) -> bool {
        !self.limits_hit.is_empty()
    }

    #[must_use]
    pub fn found_plans(&self) -> bool {
        self.plans_found > 0
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "iteration counts bounded")]
    pub fn efficiency(&self) -> f32 {
        if self.iterations == 0 {
            return 0.0;
        }
        self.plans_found as f32 / self.iterations as f32
    }
}

/// Mode for plan selection when multiple valid plans exist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanSelectionMode {
    #[default]
    BestScore,
    ShortestLength,
    LowestCost,
    LowestRisk,
    First,
}

impl PlanSelectionMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BestScore => "best_score",
            Self::ShortestLength => "shortest_length",
            Self::LowestCost => "lowest_cost",
            Self::LowestRisk => "lowest_risk",
            Self::First => "first",
        }
    }
}

/// Configuration for plan execution and tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub max_step_failures: u32,
    pub replan_on_invalidation: bool,
    pub replan_cooldown_ticks: u64,
    pub progress_timeout_ticks: u64,
    pub allow_interruption: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_step_failures: 3,
            replan_on_invalidation: true,
            replan_cooldown_ticks: 10,
            progress_timeout_ticks: 100,
            allow_interruption: true,
        }
    }
}

impl ExecutionConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_failures(mut self, max: u32) -> Self {
        self.max_step_failures = max;
        self
    }

    #[must_use]
    pub fn with_replan_cooldown(mut self, ticks: u64) -> Self {
        self.replan_cooldown_ticks = ticks;
        self
    }

    #[must_use]
    pub fn with_progress_timeout(mut self, ticks: u64) -> Self {
        self.progress_timeout_ticks = ticks;
        self
    }

    #[must_use]
    pub fn strict() -> Self {
        Self {
            max_step_failures: 1,
            replan_on_invalidation: false,
            replan_cooldown_ticks: 50,
            progress_timeout_ticks: 50,
            allow_interruption: false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_config_default() {
        let config = PlannerConfig::default();
        assert_eq!(config.max_plan_depth, 10);
        assert_eq!(config.max_search_iterations, 1000);
        assert!(config.deterministic_tiebreak);
    }

    #[test]
    fn test_planner_config_presets() {
        let strict = PlannerConfig::strict();
        let relaxed = PlannerConfig::relaxed();

        assert!(strict.max_plan_depth < relaxed.max_plan_depth);
        assert!(strict.risk_tolerance < relaxed.risk_tolerance);
        assert!(!strict.allow_partial_plans);
        assert!(relaxed.allow_partial_plans);
    }

    #[test]
    fn test_planner_config_builder() {
        let config = PlannerConfig::new()
            .with_max_depth(15)
            .with_max_iterations(2000)
            .with_risk_tolerance(0.7)
            .with_timeout(500);

        assert_eq!(config.max_plan_depth, 15);
        assert_eq!(config.max_search_iterations, 2000);
        assert!((config.risk_tolerance - 0.7).abs() < f32::EPSILON);
        assert_eq!(config.timeout_ticks, 500);
    }

    #[test]
    fn test_planner_config_bounds_check() {
        let config = PlannerConfig::new()
            .with_max_depth(10)
            .with_max_iterations(100);

        assert!(config.is_within_bounds(5, 50));
        assert!(config.is_within_bounds(10, 100));
        assert!(!config.is_within_bounds(11, 50));
        assert!(!config.is_within_bounds(5, 101));
    }

    #[test]
    fn test_planner_limit() {
        assert_eq!(PlannerLimit::MaxDepth.as_str(), "max_depth");
        assert_eq!(PlannerLimit::Timeout.as_str(), "timeout");
    }

    #[test]
    fn test_planner_stats() {
        let mut stats = PlannerStats::new();

        stats.record_iteration();
        stats.record_iteration();
        stats.record_depth(5);
        stats.record_depth(3);
        stats.record_expansion(4);
        stats.record_expansion(6);
        stats.record_plan_found();
        stats.record_limit(PlannerLimit::MaxDepth);

        assert_eq!(stats.iterations, 2);
        assert_eq!(stats.max_depth_reached, 5);
        assert_eq!(stats.nodes_expanded, 2);
        assert_eq!(stats.actions_considered, 10);
        assert_eq!(stats.plans_found, 1);
        assert!(stats.hit_limits());
        assert!(stats.found_plans());
    }

    #[test]
    fn test_planner_stats_efficiency() {
        let mut stats = PlannerStats::new();
        assert!((stats.efficiency()).abs() < f32::EPSILON);

        for _ in 0..10 {
            stats.record_iteration();
        }
        stats.record_plan_found();
        stats.record_plan_found();

        assert!((stats.efficiency() - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_plan_selection_mode() {
        assert_eq!(PlanSelectionMode::BestScore.as_str(), "best_score");
        assert_eq!(PlanSelectionMode::LowestRisk.as_str(), "lowest_risk");
        assert_eq!(PlanSelectionMode::default(), PlanSelectionMode::BestScore);
    }

    #[test]
    fn test_execution_config() {
        let config = ExecutionConfig::new()
            .with_max_failures(5)
            .with_replan_cooldown(20)
            .with_progress_timeout(200);

        assert_eq!(config.max_step_failures, 5);
        assert_eq!(config.replan_cooldown_ticks, 20);
        assert_eq!(config.progress_timeout_ticks, 200);
    }

    #[test]
    fn test_execution_config_strict() {
        let strict = ExecutionConfig::strict();
        assert_eq!(strict.max_step_failures, 1);
        assert!(!strict.replan_on_invalidation);
        assert!(!strict.allow_interruption);
    }

    #[test]
    fn test_config_serde() {
        let config = PlannerConfig::new()
            .with_max_depth(15)
            .with_risk_tolerance(0.6);

        let json = serde_json::to_string(&config).unwrap();
        let restored: PlannerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.max_plan_depth, 15);
        assert!((restored.risk_tolerance - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stats_serde() {
        let mut stats = PlannerStats::new();
        stats.record_iteration();
        stats.record_plan_found();
        stats.record_limit(PlannerLimit::MaxIterations);

        let json = serde_json::to_string(&stats).unwrap();
        let restored: PlannerStats = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.iterations, 1);
        assert_eq!(restored.plans_found, 1);
        assert!(restored.limits_hit.contains(&PlannerLimit::MaxIterations));
    }

    #[test]
    fn test_execution_config_serde() {
        let config = ExecutionConfig::strict();

        let json = serde_json::to_string(&config).unwrap();
        let restored: ExecutionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.max_step_failures, 1);
        assert!(!restored.allow_interruption);
    }

    #[test]
    fn test_planner_config_bincode() {
        let config = PlannerConfig::new()
            .with_max_depth(15)
            .with_risk_tolerance(0.6);

        let bytes = bincode::serialize(&config).unwrap();
        let restored: PlannerConfig = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.max_plan_depth, 15);
        assert!((restored.risk_tolerance - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_execution_config_bincode() {
        let config = ExecutionConfig::strict();

        let bytes = bincode::serialize(&config).unwrap();
        let restored: ExecutionConfig = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.max_step_failures, 1);
    }

    #[test]
    fn test_planner_stats_bincode() {
        let mut stats = PlannerStats::new();
        stats.record_iteration();
        stats.record_plan_found();
        stats.record_limit(PlannerLimit::MaxIterations);

        let bytes = bincode::serialize(&stats).unwrap();
        let restored: PlannerStats = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.iterations, 1);
        assert!(restored.limits_hit.contains(&PlannerLimit::MaxIterations));
    }

    #[test]
    fn test_plan_selection_mode_bincode() {
        let mode = PlanSelectionMode::LowestRisk;

        let bytes = bincode::serialize(&mode).unwrap();
        let restored: PlanSelectionMode = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, PlanSelectionMode::LowestRisk);
    }
}

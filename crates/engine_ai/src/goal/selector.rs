//! Goal selector with evaluation, cooldown, inertia, and hysteresis.

use super::context::GoalContext;
use super::definition::{GoalDef, GoalId};
use super::scoring::{ConsiderationScore, GoalScore, ScoringBreakdown};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuration for goal cooldown behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CooldownConfig {
    /// Whether cooldowns are enabled.
    pub enabled: bool,
    /// Global cooldown multiplier (1.0 = use goal-defined cooldowns).
    pub multiplier: f32,
}

impl Default for CooldownConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            multiplier: 1.0,
        }
    }
}

/// Configuration for goal inertia (preference to continue current goal).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InertiaConfig {
    /// Whether inertia is enabled.
    pub enabled: bool,
    /// Bonus added to current goal's score.
    pub bonus: f32,
    /// Minimum ticks before inertia kicks in.
    pub min_duration: u64,
}

impl Default for InertiaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bonus: 0.15,
            min_duration: 10,
        }
    }
}

/// Configuration for hysteresis (penalty for recently completed goals).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HysteresisConfig {
    /// Whether hysteresis is enabled.
    pub enabled: bool,
    /// Maximum penalty for just-completed goals.
    pub max_penalty: f32,
    /// Ticks over which penalty decays to zero.
    pub decay_ticks: u64,
}

impl Default for HysteresisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_penalty: 0.25,
            decay_ticks: 300,
        }
    }
}

/// Reason a goal was selected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionReason {
    /// Highest scoring viable goal.
    HighestScore,
    /// Current goal retained due to inertia.
    InertiaRetained,
    /// No viable goals; defaulting to fallback.
    FallbackOnly,
    /// Current goal cannot be interrupted.
    NonInterruptible,
    /// Current goal has minimum duration remaining.
    MinDurationActive,
}

/// Result of goal selection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalSelection {
    /// The selected goal score.
    pub selected: GoalScore,
    /// Reason for selection.
    pub reason: SelectionReason,
    /// All scored goals (sorted by score, descending).
    pub all_scores: Vec<GoalScore>,
    /// Detailed breakdowns for debugging.
    pub breakdowns: BTreeMap<GoalId, ScoringBreakdown>,
    /// Previous goal ID (if any).
    pub previous_goal: Option<GoalId>,
    /// Whether the goal changed from previous.
    pub changed: bool,
}

impl GoalSelection {
    /// Get the breakdown for the selected goal.
    #[must_use]
    pub fn selected_breakdown(&self) -> Option<&ScoringBreakdown> {
        self.breakdowns.get(&self.selected.id)
    }

    /// Get explanation for the selected goal.
    #[must_use]
    pub fn explain_selection(&self) -> String {
        let mut parts = vec![format!(
            "Selected: {} (score: {:.3})",
            self.selected.id.as_str(),
            self.selected.score
        )];

        parts.push(format!("Reason: {:?}", self.reason));

        if let Some(breakdown) = self.selected_breakdown() {
            parts.push(breakdown.explain());
        }

        if self.changed
            && let Some(ref prev) = self.previous_goal
        {
            parts.push(format!("Changed from: {}", prev.as_str()));
        }

        parts.join("\n")
    }

    /// Get the top N goals by score.
    #[must_use]
    pub fn top_goals(&self, count: usize) -> Vec<&GoalScore> {
        self.all_scores.iter().take(count).collect()
    }
}

/// Stateful goal selector with registered goals and selection history.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GoalSelector {
    /// Registered goal definitions.
    goals: BTreeMap<GoalId, GoalDef>,
    /// Current active goal.
    current_goal: Option<GoalId>,
    /// Tick when current goal was selected.
    current_goal_start_tick: u64,
    /// Goal completion history mapping goal ID to completion tick.
    completion_history: BTreeMap<GoalId, u64>,
    /// Cooldown configuration.
    pub cooldown_config: CooldownConfig,
    /// Inertia configuration.
    pub inertia_config: InertiaConfig,
    /// Hysteresis configuration.
    pub hysteresis_config: HysteresisConfig,
    /// Fallback goal when no others are viable.
    fallback_goal: Option<GoalId>,
    /// Current tick.
    current_tick: u64,
}

impl GoalSelector {
    /// Create a new goal selector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a goal definition.
    pub fn register(&mut self, goal: GoalDef) {
        self.goals.insert(goal.id.clone(), goal);
    }

    /// Unregister a goal.
    pub fn unregister(&mut self, id: &GoalId) -> Option<GoalDef> {
        self.goals.remove(id)
    }

    /// Get a goal definition.
    #[must_use]
    pub fn get_goal(&self, id: &GoalId) -> Option<&GoalDef> {
        self.goals.get(id)
    }

    /// Set the fallback goal.
    pub fn set_fallback(&mut self, goal_id: GoalId) {
        self.fallback_goal = Some(goal_id);
    }

    /// Get the current goal.
    #[must_use]
    pub fn current_goal(&self) -> Option<&GoalId> {
        self.current_goal.as_ref()
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Advance the tick counter.
    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    /// Advance to a specific tick.
    pub fn advance_to(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Record goal completion.
    pub fn complete_goal(&mut self, goal_id: &GoalId) {
        self.completion_history
            .insert(goal_id.clone(), self.current_tick);

        if self.current_goal.as_ref() == Some(goal_id) {
            self.current_goal = None;
        }
    }

    /// Abort the current goal.
    pub fn abort_current(&mut self) {
        self.current_goal = None;
    }

    /// Set the current goal directly (useful for testing or initialization).
    pub fn set_current_goal(&mut self, goal_id: GoalId) {
        self.current_goal = Some(goal_id);
        self.current_goal_start_tick = self.current_tick;
    }

    /// Check if a goal is on cooldown.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "cooldown ticks are bounded"
    )]
    pub fn is_on_cooldown(&self, goal_id: &GoalId) -> bool {
        if !self.cooldown_config.enabled {
            return false;
        }

        let Some(goal) = self.goals.get(goal_id) else {
            return false;
        };

        if goal.cooldown_ticks == 0 {
            return false;
        }

        let Some(&completion_tick) = self.completion_history.get(goal_id) else {
            return false;
        };

        let elapsed = self.current_tick.saturating_sub(completion_tick);
        let effective_cooldown =
            (goal.cooldown_ticks as f32 * self.cooldown_config.multiplier) as u64;

        elapsed < effective_cooldown
    }

    /// Calculate inertia bonus for a goal.
    #[must_use]
    fn calculate_inertia(&self, goal_id: &GoalId) -> f32 {
        if !self.inertia_config.enabled {
            return 0.0;
        }

        if self.current_goal.as_ref() != Some(goal_id) {
            return 0.0;
        }

        let duration = self
            .current_tick
            .saturating_sub(self.current_goal_start_tick);
        if duration < self.inertia_config.min_duration {
            return 0.0;
        }

        self.inertia_config.bonus
    }

    /// Calculate hysteresis penalty for a goal.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values are bounded for practical use"
    )]
    fn calculate_hysteresis(&self, goal_id: &GoalId) -> f32 {
        if !self.hysteresis_config.enabled {
            return 0.0;
        }

        if self.current_goal.as_ref() == Some(goal_id) {
            return 0.0;
        }

        let Some(&completion_tick) = self.completion_history.get(goal_id) else {
            return 0.0;
        };

        let elapsed = self.current_tick.saturating_sub(completion_tick);
        if elapsed >= self.hysteresis_config.decay_ticks {
            return 0.0;
        }

        let decay_factor = 1.0 - (elapsed as f32 / self.hysteresis_config.decay_ticks as f32);
        self.hysteresis_config.max_penalty * decay_factor
    }

    /// Check if current goal has minimum duration remaining.
    #[must_use]
    fn has_min_duration_remaining(&self) -> bool {
        let Some(ref current_id) = self.current_goal else {
            return false;
        };

        let Some(goal) = self.goals.get(current_id) else {
            return false;
        };

        if goal.min_duration == 0 {
            return false;
        }

        let duration = self
            .current_tick
            .saturating_sub(self.current_goal_start_tick);
        duration < goal.min_duration
    }

    /// Check if current goal is non-interruptible.
    #[must_use]
    fn is_current_non_interruptible(&self) -> bool {
        self.current_goal
            .as_ref()
            .and_then(|id| self.goals.get(id))
            .is_some_and(|g| !g.interruptible)
    }

    /// Score a single goal.
    #[must_use]
    fn score_goal(&self, goal: &GoalDef, context: &GoalContext) -> ScoringBreakdown {
        let mut breakdown = ScoringBreakdown::new(goal.id.clone());
        breakdown.base_priority = goal.base_priority;
        breakdown.on_cooldown = self.is_on_cooldown(&goal.id);
        breakdown.inertia_bonus = self.calculate_inertia(&goal.id);
        breakdown.hysteresis_penalty = self.calculate_hysteresis(&goal.id);

        for consideration in goal.considerations() {
            let input_value = context.resolve_input(&consideration.input);
            let curve_output = consideration.curve.evaluate(input_value);
            let weighted = curve_output * consideration.weight;
            let vetoed = consideration.is_vetoed(curve_output);

            breakdown.add_consideration(ConsiderationScore::new(
                consideration.id.clone(),
                input_value,
                curve_output,
                weighted,
                vetoed,
            ));
        }

        breakdown.compute_final_score(goal.min_threshold);
        breakdown
    }

    /// Evaluate all goals and select the best one.
    ///
    /// # Panics
    ///
    /// This function will not panic in practice. The internal `expect()` calls
    /// are guarded by checks that ensure `current_goal` is Some.
    #[must_use]
    pub fn evaluate(&mut self, context: &GoalContext) -> GoalSelection {
        let tick = context.current_tick;
        if tick > self.current_tick {
            self.current_tick = tick;
        }

        let mut breakdowns = BTreeMap::new();
        let mut scores = Vec::new();

        for goal in self.goals.values() {
            let breakdown = self.score_goal(goal, context);
            let score = GoalScore::new(goal.id.clone(), breakdown.final_score, tick);
            scores.push(score);
            breakdowns.insert(goal.id.clone(), breakdown);
        }

        scores.sort();

        if self.has_min_duration_remaining() {
            let current_id = self
                .current_goal
                .clone()
                .expect("checked by has_min_duration_remaining");
            let current_score = breakdowns.get(&current_id).map_or(0.0, |b| b.final_score);

            return GoalSelection {
                selected: GoalScore::new(current_id.clone(), current_score, tick),
                reason: SelectionReason::MinDurationActive,
                all_scores: scores,
                breakdowns,
                previous_goal: self.current_goal.clone(),
                changed: false,
            };
        }

        if self.is_current_non_interruptible() {
            let current_id = self
                .current_goal
                .clone()
                .expect("checked by is_current_non_interruptible");
            let current_breakdown = breakdowns.get(&current_id);

            if current_breakdown.is_some_and(ScoringBreakdown::is_viable) {
                let score = current_breakdown.map_or(0.0, |b| b.final_score);

                return GoalSelection {
                    selected: GoalScore::new(current_id.clone(), score, tick),
                    reason: SelectionReason::NonInterruptible,
                    all_scores: scores,
                    breakdowns,
                    previous_goal: self.current_goal.clone(),
                    changed: false,
                };
            }
        }

        let viable_scores: Vec<_> = scores
            .iter()
            .filter(|s| {
                breakdowns
                    .get(&s.id)
                    .is_some_and(ScoringBreakdown::is_viable)
            })
            .collect();

        let (selected, reason) = if let Some(best) = viable_scores.first() {
            let is_current = self.current_goal.as_ref() == Some(&best.id);
            let reason = if is_current
                && self.inertia_config.enabled
                && viable_scores.len() > 1
                && viable_scores[1].score > (best.score - self.inertia_config.bonus)
            {
                SelectionReason::InertiaRetained
            } else {
                SelectionReason::HighestScore
            };
            ((*best).clone(), reason)
        } else if let Some(ref fallback_id) = self.fallback_goal {
            let score = breakdowns.get(fallback_id).map_or(0.0, |b| b.final_score);
            (
                GoalScore::new(fallback_id.clone(), score, tick),
                SelectionReason::FallbackOnly,
            )
        } else {
            (
                GoalScore::new(GoalId::idle(), 0.0, tick),
                SelectionReason::FallbackOnly,
            )
        };

        let previous = self.current_goal.clone();
        let changed = previous.as_ref() != Some(&selected.id);

        if changed {
            self.current_goal = Some(selected.id.clone());
            self.current_goal_start_tick = tick;
        }

        GoalSelection {
            selected,
            reason,
            all_scores: scores,
            breakdowns,
            previous_goal: previous,
            changed,
        }
    }

    /// Get all registered goal IDs.
    pub fn goal_ids(&self) -> impl Iterator<Item = &GoalId> {
        self.goals.keys()
    }

    /// Get number of registered goals.
    #[must_use]
    pub fn goal_count(&self) -> usize {
        self.goals.len()
    }

    /// Clear completion history.
    pub fn clear_history(&mut self) {
        self.completion_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Consideration, GoalTag, InputBinding, UtilityCurve};

    fn make_simple_goal(id: &str, priority: f32) -> GoalDef {
        GoalDef::new(GoalId::new(id), id)
            .with_priority(priority)
            .with_consideration(
                Consideration::new("default")
                    .with_input(InputBinding::Constant(0.8))
                    .with_curve(UtilityCurve::linear()),
            )
    }

    #[test]
    fn test_selector_new() {
        let selector = GoalSelector::new();

        assert_eq!(selector.goal_count(), 0);
        assert!(selector.current_goal().is_none());
    }

    #[test]
    fn test_selector_register() {
        let mut selector = GoalSelector::new();

        selector.register(make_simple_goal("test", 1.0));

        assert_eq!(selector.goal_count(), 1);
        assert!(selector.get_goal(&GoalId::new("test")).is_some());
    }

    #[test]
    fn test_selector_unregister() {
        let mut selector = GoalSelector::new();
        selector.register(make_simple_goal("test", 1.0));

        let removed = selector.unregister(&GoalId::new("test"));

        assert!(removed.is_some());
        assert_eq!(selector.goal_count(), 0);
    }

    #[test]
    fn test_selector_evaluate_basic() {
        let mut selector = GoalSelector::new();
        selector.register(make_simple_goal("alpha", 1.0));
        selector.register(make_simple_goal("beta", 2.0));

        let ctx = GoalContext::builder().with_tick(100).build();
        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::new("beta"));
        assert_eq!(result.reason, SelectionReason::HighestScore);
        assert!(result.changed);
    }

    #[test]
    fn test_selector_evaluate_deterministic() {
        let mut selector = GoalSelector::new();
        selector.register(make_simple_goal("alpha", 1.0));
        selector.register(make_simple_goal("beta", 1.0));

        let ctx = GoalContext::builder().with_tick(100).build();
        let result1 = selector.evaluate(&ctx);

        selector.current_goal = None;
        let result2 = selector.evaluate(&ctx);

        assert_eq!(result1.selected.id, result2.selected.id);
    }

    #[test]
    fn test_selector_cooldown() {
        let mut selector = GoalSelector::new();
        let goal = GoalDef::new(GoalId::new("test"), "Test")
            .with_priority(1.0)
            .with_cooldown(100)
            .with_consideration(
                Consideration::new("c")
                    .with_input(InputBinding::Constant(1.0))
                    .with_curve(UtilityCurve::linear()),
            );

        selector.register(goal);
        selector.advance_to(50);
        selector.complete_goal(&GoalId::new("test"));

        assert!(selector.is_on_cooldown(&GoalId::new("test")));

        selector.advance_to(200);
        assert!(!selector.is_on_cooldown(&GoalId::new("test")));
    }

    #[test]
    fn test_selector_cooldown_disabled() {
        let mut selector = GoalSelector::new();
        selector.cooldown_config.enabled = false;

        let goal = GoalDef::new(GoalId::new("test"), "Test")
            .with_priority(1.0)
            .with_cooldown(100)
            .with_consideration(
                Consideration::new("c")
                    .with_input(InputBinding::Constant(1.0))
                    .with_curve(UtilityCurve::linear()),
            );

        selector.register(goal);
        selector.advance_to(50);
        selector.complete_goal(&GoalId::new("test"));

        assert!(!selector.is_on_cooldown(&GoalId::new("test")));
    }

    #[test]
    fn test_selector_inertia() {
        let mut selector = GoalSelector::new();
        selector.inertia_config.bonus = 0.5;
        selector.inertia_config.min_duration = 0;

        selector.register(make_simple_goal("current", 1.0));
        selector.register(make_simple_goal("other", 1.1));

        selector.current_goal = Some(GoalId::new("current"));
        selector.current_goal_start_tick = 0;

        let ctx = GoalContext::builder().with_tick(100).build();
        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::new("current"));
        assert_eq!(result.reason, SelectionReason::InertiaRetained);
    }

    #[test]
    fn test_selector_hysteresis() {
        let mut selector = GoalSelector::new();
        selector.hysteresis_config.max_penalty = 0.5;
        selector.hysteresis_config.decay_ticks = 100;

        selector.register(make_simple_goal("completed", 1.5));
        selector.register(make_simple_goal("other", 1.2));

        selector.advance_to(50);
        selector.complete_goal(&GoalId::new("completed"));

        let ctx = GoalContext::builder().with_tick(60).build();
        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::new("other"));
    }

    #[test]
    fn test_selector_min_duration() {
        let mut selector = GoalSelector::new();

        let goal = GoalDef::new(GoalId::new("long"), "Long Running")
            .with_priority(1.0)
            .with_min_duration(100)
            .with_consideration(
                Consideration::new("c")
                    .with_input(InputBinding::Constant(0.5))
                    .with_curve(UtilityCurve::linear()),
            );

        selector.register(goal);
        selector.register(make_simple_goal("higher", 2.0));

        selector.current_goal = Some(GoalId::new("long"));
        selector.current_goal_start_tick = 0;
        selector.advance_to(50);

        let ctx = GoalContext::builder().with_tick(50).build();
        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::new("long"));
        assert_eq!(result.reason, SelectionReason::MinDurationActive);
    }

    #[test]
    fn test_selector_non_interruptible() {
        let mut selector = GoalSelector::new();

        let goal = GoalDef::new(GoalId::new("critical"), "Critical")
            .with_priority(1.0)
            .with_interruptible(false)
            .with_consideration(
                Consideration::new("c")
                    .with_input(InputBinding::Constant(0.8))
                    .with_curve(UtilityCurve::linear()),
            );

        selector.register(goal);
        selector.register(make_simple_goal("higher", 2.0));

        selector.current_goal = Some(GoalId::new("critical"));

        let ctx = GoalContext::builder().with_tick(100).build();
        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::new("critical"));
        assert_eq!(result.reason, SelectionReason::NonInterruptible);
    }

    #[test]
    fn test_selector_fallback() {
        let mut selector = GoalSelector::new();

        let goal = GoalDef::new(GoalId::new("vetoed"), "Vetoed")
            .with_priority(1.0)
            .with_consideration(
                Consideration::new("veto")
                    .with_input(InputBinding::Constant(0.0))
                    .with_curve(UtilityCurve::linear())
                    .with_veto(0.1),
            );

        selector.register(goal);

        let fallback = GoalDef::new(GoalId::idle(), "Idle")
            .with_priority(0.1)
            .with_consideration(
                Consideration::new("always")
                    .with_input(InputBinding::Constant(1.0))
                    .with_curve(UtilityCurve::linear()),
            );

        selector.register(fallback);
        selector.set_fallback(GoalId::idle());

        let ctx = GoalContext::builder().with_tick(100).build();
        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::idle());
    }

    #[test]
    fn test_selector_complete_goal() {
        let mut selector = GoalSelector::new();
        selector.register(make_simple_goal("test", 1.0));

        selector.current_goal = Some(GoalId::new("test"));
        selector.advance_to(100);
        selector.complete_goal(&GoalId::new("test"));

        assert!(selector.current_goal().is_none());
        assert_eq!(
            selector.completion_history.get(&GoalId::new("test")),
            Some(&100)
        );
    }

    #[test]
    fn test_selector_abort() {
        let mut selector = GoalSelector::new();
        selector.current_goal = Some(GoalId::new("test"));

        selector.abort_current();

        assert!(selector.current_goal().is_none());
    }

    #[test]
    fn test_goal_selection_explain() {
        let mut selector = GoalSelector::new();
        selector.register(make_simple_goal("test", 1.0));

        let ctx = GoalContext::builder().with_tick(100).build();
        let result = selector.evaluate(&ctx);

        let explanation = result.explain_selection();
        assert!(explanation.contains("test"));
        assert!(explanation.contains("score"));
    }

    #[test]
    fn test_goal_selection_top_goals() {
        let mut selector = GoalSelector::new();
        selector.register(make_simple_goal("a", 1.0));
        selector.register(make_simple_goal("b", 2.0));
        selector.register(make_simple_goal("c", 1.5));

        let ctx = GoalContext::builder().with_tick(100).build();
        let result = selector.evaluate(&ctx);

        let top = result.top_goals(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].id, GoalId::new("b"));
    }

    #[test]
    fn test_selector_serde() {
        let mut selector = GoalSelector::new();
        selector.register(
            GoalDef::new(GoalId::satisfy_hunger(), "Satisfy Hunger")
                .with_priority(1.5)
                .with_tag(GoalTag::survival()),
        );

        selector.current_goal = Some(GoalId::satisfy_hunger());
        selector.current_tick = 500;

        let json = serde_json::to_string(&selector).unwrap();
        let restored: GoalSelector = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.goal_count(), 1);
        assert_eq!(restored.current_goal(), Some(&GoalId::satisfy_hunger()));
        assert_eq!(restored.current_tick(), 500);
    }

    #[test]
    fn test_cooldown_config_serde() {
        let config = CooldownConfig {
            enabled: false,
            multiplier: 1.5,
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: CooldownConfig = serde_json::from_str(&json).unwrap();

        assert!(!restored.enabled);
        assert!((restored.multiplier - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_selection_reason_serde() {
        let reason = SelectionReason::InertiaRetained;

        let json = serde_json::to_string(&reason).unwrap();
        let restored: SelectionReason = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, SelectionReason::InertiaRetained);
    }

    #[test]
    fn test_selector_clear_history() {
        let mut selector = GoalSelector::new();
        selector.completion_history.insert(GoalId::new("test"), 100);

        selector.clear_history();

        assert!(selector.completion_history.is_empty());
    }
}

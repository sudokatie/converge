//! Summaries and snapshots for goal state, supporting unloaded-chunk simulation.

use super::definition::{GoalId, GoalTag};
use super::selector::GoalSelector;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Summary of goal state for a single entity or group.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GoalSummary {
    /// Number of entities in this summary.
    pub entity_count: u32,
    /// Goals currently being pursued, with count.
    active_goals: BTreeMap<GoalId, u32>,
    /// Goals on cooldown, with count.
    cooldown_goals: BTreeMap<GoalId, u32>,
    /// Tag distribution of active goals.
    tag_distribution: BTreeMap<GoalTag, u32>,
    /// Average goal score across entities.
    pub average_score: f32,
    /// Tick when this summary was computed.
    pub computed_at_tick: u64,
}

impl GoalSummary {
    /// Create a new empty summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a selector's state to the summary.
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn add_selector(&mut self, selector: &GoalSelector, current_score: f32) {
        self.entity_count += 1;

        if let Some(goal_id) = selector.current_goal() {
            *self.active_goals.entry(goal_id.clone()).or_insert(0) += 1;

            if let Some(goal) = selector.get_goal(goal_id) {
                for tag in goal.tags() {
                    *self.tag_distribution.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        for goal_id in selector.goal_ids() {
            if selector.is_on_cooldown(goal_id) {
                *self.cooldown_goals.entry(goal_id.clone()).or_insert(0) += 1;
            }
        }

        let old_total = self.average_score * (self.entity_count - 1) as f32;
        self.average_score = (old_total + current_score) / self.entity_count as f32;
    }

    /// Set the computation tick.
    pub fn set_tick(&mut self, tick: u64) {
        self.computed_at_tick = tick;
    }

    /// Get the most common active goal.
    #[must_use]
    pub fn most_common_goal(&self) -> Option<&GoalId> {
        self.active_goals
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(id, _)| id)
    }

    /// Get count of entities pursuing a specific goal.
    #[must_use]
    pub fn goal_count(&self, goal_id: &GoalId) -> u32 {
        self.active_goals.get(goal_id).copied().unwrap_or(0)
    }

    /// Get count of entities with a goal on cooldown.
    #[must_use]
    pub fn cooldown_count(&self, goal_id: &GoalId) -> u32 {
        self.cooldown_goals.get(goal_id).copied().unwrap_or(0)
    }

    /// Get distribution of goals by tag.
    #[must_use]
    pub fn tag_count(&self, tag: &GoalTag) -> u32 {
        self.tag_distribution.get(tag).copied().unwrap_or(0)
    }

    /// Get the dominant tag (most common among active goals).
    #[must_use]
    pub fn dominant_tag(&self) -> Option<&GoalTag> {
        self.tag_distribution
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(tag, _)| tag)
    }

    /// Get all active goals with their counts.
    pub fn active_goals(&self) -> impl Iterator<Item = (&GoalId, u32)> {
        self.active_goals.iter().map(|(id, &count)| (id, count))
    }

    /// Check if any entity is in a critical state (survival/combat goals).
    #[must_use]
    pub fn has_critical_activity(&self) -> bool {
        self.tag_count(&GoalTag::survival()) > 0 || self.tag_count(&GoalTag::combat()) > 0
    }

    /// Get the ratio of entities in idle state.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn idle_ratio(&self) -> f32 {
        if self.entity_count == 0 {
            return 1.0;
        }

        let idle_count = self.tag_count(&GoalTag::idle());
        idle_count as f32 / self.entity_count as f32
    }

    /// Merge another summary into this one.
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn merge(&mut self, other: &Self) {
        if other.entity_count == 0 {
            return;
        }

        if self.entity_count == 0 {
            *self = other.clone();
            return;
        }

        let old_total = self.average_score * self.entity_count as f32;
        let other_total = other.average_score * other.entity_count as f32;
        let new_count = self.entity_count + other.entity_count;
        self.average_score = (old_total + other_total) / new_count as f32;

        self.entity_count = new_count;

        for (id, count) in &other.active_goals {
            *self.active_goals.entry(id.clone()).or_insert(0) += count;
        }

        for (id, count) in &other.cooldown_goals {
            *self.cooldown_goals.entry(id.clone()).or_insert(0) += count;
        }

        for (tag, count) in &other.tag_distribution {
            *self.tag_distribution.entry(tag.clone()).or_insert(0) += count;
        }
    }

    /// Create a summary from multiple selectors.
    pub fn from_selectors<'a>(
        selectors: impl Iterator<Item = (&'a GoalSelector, f32)>,
        tick: u64,
    ) -> Self {
        let mut summary = Self::new();
        for (selector, score) in selectors {
            summary.add_selector(selector, score);
        }
        summary.set_tick(tick);
        summary
    }
}

/// A snapshot of goal state suitable for persistence or unloaded chunk simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoalSnapshot {
    /// Summary of goal state.
    pub summary: GoalSummary,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
    /// Region/colony identifier.
    pub region_id: u64,
    /// Whether entities need immediate attention.
    pub needs_attention: bool,
    /// Dominant goal category.
    pub dominant_category: Option<GoalTag>,
    /// Estimated stability (0.0 = volatile, 1.0 = stable).
    pub stability: f32,
    /// Estimated ticks until state changes significantly.
    pub ticks_until_change: Option<u64>,
}

impl GoalSnapshot {
    /// Create a new snapshot.
    #[must_use]
    pub fn new(region_id: u64, summary: GoalSummary, tick: u64) -> Self {
        let needs_attention = summary.has_critical_activity();
        let dominant_category = summary.dominant_tag().cloned();
        let stability = Self::calculate_stability(&summary);
        let ticks_until_change = Self::estimate_ticks_until_change(&summary);

        Self {
            summary,
            snapshot_tick: tick,
            region_id,
            needs_attention,
            dominant_category,
            stability,
            ticks_until_change,
        }
    }

    fn calculate_stability(summary: &GoalSummary) -> f32 {
        if summary.entity_count == 0 {
            return 1.0;
        }

        let idle_factor = summary.idle_ratio();
        let critical_factor = if summary.has_critical_activity() {
            0.2
        } else {
            1.0
        };

        (idle_factor * 0.5 + critical_factor * 0.5).clamp(0.0, 1.0)
    }

    fn estimate_ticks_until_change(summary: &GoalSummary) -> Option<u64> {
        if summary.entity_count == 0 {
            return None;
        }

        let base = if summary.has_critical_activity() {
            30
        } else if summary.idle_ratio() > 0.8 {
            300
        } else {
            120
        };

        Some(base)
    }

    /// Check if the snapshot is stale.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Get the age of this snapshot in ticks.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }

    /// Check if the snapshot indicates urgent activity.
    #[must_use]
    pub fn is_urgent(&self) -> bool {
        self.needs_attention && self.stability < 0.3
    }

    /// Check if intervention might be needed soon.
    #[must_use]
    pub fn needs_intervention(&self, tick_threshold: u64) -> bool {
        self.needs_attention || self.ticks_until_change.is_some_and(|t| t < tick_threshold)
    }

    /// Estimate stability after elapsed ticks (simple projection).
    #[must_use]
    pub fn projected_stability(&self, elapsed_ticks: u64) -> f32 {
        let decay = 0.995_f32.powi(elapsed_ticks.min(1000) as i32);
        let regression = (1.0 - decay) * 0.7;
        (self.stability * decay + regression).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{GoalDef, preset};

    fn make_selector_with_goal(goal_id: &GoalId) -> GoalSelector {
        let mut selector = GoalSelector::new();

        let goal = GoalDef::new(goal_id.clone(), "Test")
            .with_priority(1.0)
            .with_tag(GoalTag::survival());

        selector.register(goal);
        selector.register(preset::preset_idle());
        selector
    }

    #[test]
    fn test_summary_new() {
        let summary = GoalSummary::new();

        assert_eq!(summary.entity_count, 0);
        assert!((summary.average_score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_summary_add_selector() {
        let mut selector = make_selector_with_goal(&GoalId::satisfy_hunger());
        selector.set_current_goal(GoalId::satisfy_hunger());

        let mut summary = GoalSummary::new();
        summary.add_selector(&selector, 0.75);

        assert_eq!(summary.entity_count, 1);
        assert_eq!(summary.goal_count(&GoalId::satisfy_hunger()), 1);
        assert!((summary.average_score - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_summary_multiple_selectors() {
        let mut selector1 = make_selector_with_goal(&GoalId::satisfy_hunger());
        selector1.set_current_goal(GoalId::satisfy_hunger());

        let mut selector2 = make_selector_with_goal(&GoalId::rest());
        selector2.set_current_goal(GoalId::rest());

        let mut summary = GoalSummary::new();
        summary.add_selector(&selector1, 0.8);
        summary.add_selector(&selector2, 0.6);

        assert_eq!(summary.entity_count, 2);
        assert_eq!(summary.goal_count(&GoalId::satisfy_hunger()), 1);
        assert_eq!(summary.goal_count(&GoalId::rest()), 1);
        assert!((summary.average_score - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_summary_most_common_goal() {
        let mut selector1 = make_selector_with_goal(&GoalId::idle());
        selector1.set_current_goal(GoalId::idle());

        let mut selector2 = make_selector_with_goal(&GoalId::idle());
        selector2.set_current_goal(GoalId::idle());

        let mut selector3 = make_selector_with_goal(&GoalId::rest());
        selector3.set_current_goal(GoalId::rest());

        let mut summary = GoalSummary::new();
        summary.add_selector(&selector1, 0.5);
        summary.add_selector(&selector2, 0.5);
        summary.add_selector(&selector3, 0.5);

        assert_eq!(summary.most_common_goal(), Some(&GoalId::idle()));
    }

    #[test]
    fn test_summary_tag_distribution() {
        let mut selector = make_selector_with_goal(&GoalId::satisfy_hunger());
        selector.set_current_goal(GoalId::satisfy_hunger());

        let mut summary = GoalSummary::new();
        summary.add_selector(&selector, 0.5);

        assert!(summary.tag_count(&GoalTag::survival()) > 0);
    }

    #[test]
    fn test_summary_idle_ratio() {
        let mut selector1 = GoalSelector::new();
        selector1.register(preset::preset_idle());
        selector1.set_current_goal(GoalId::idle());

        let mut selector2 = make_selector_with_goal(&GoalId::rest());
        selector2.set_current_goal(GoalId::rest());

        let mut summary = GoalSummary::new();
        summary.add_selector(&selector1, 0.5);
        summary.add_selector(&selector2, 0.5);

        let ratio = summary.idle_ratio();
        assert!(ratio > 0.0 && ratio < 1.0);
    }

    #[test]
    fn test_summary_merge() {
        let mut s1 = GoalSummary::new();
        s1.entity_count = 2;
        s1.average_score = 0.6;
        s1.active_goals.insert(GoalId::idle(), 2);

        let mut s2 = GoalSummary::new();
        s2.entity_count = 1;
        s2.average_score = 0.9;
        s2.active_goals.insert(GoalId::rest(), 1);

        s1.merge(&s2);

        assert_eq!(s1.entity_count, 3);
        assert!((s1.average_score - 0.7).abs() < f32::EPSILON);
        assert_eq!(s1.goal_count(&GoalId::idle()), 2);
        assert_eq!(s1.goal_count(&GoalId::rest()), 1);
    }

    #[test]
    fn test_summary_from_selectors() {
        let mut selector1 = GoalSelector::new();
        selector1.register(preset::preset_idle());
        selector1.set_current_goal(GoalId::idle());

        let selector_list = vec![(&selector1, 0.5_f32)];

        let summary = GoalSummary::from_selectors(selector_list.into_iter(), 100);

        assert_eq!(summary.entity_count, 1);
        assert_eq!(summary.computed_at_tick, 100);
    }

    #[test]
    fn test_summary_has_critical_activity() {
        let mut summary = GoalSummary::new();
        assert!(!summary.has_critical_activity());

        summary.tag_distribution.insert(GoalTag::survival(), 1);
        assert!(summary.has_critical_activity());
    }

    #[test]
    fn test_snapshot_new() {
        let mut summary = GoalSummary::new();
        summary.entity_count = 5;
        summary.tag_distribution.insert(GoalTag::idle(), 5);

        let snapshot = GoalSnapshot::new(42, summary, 100);

        assert_eq!(snapshot.region_id, 42);
        assert_eq!(snapshot.snapshot_tick, 100);
        assert!(!snapshot.needs_attention);
    }

    #[test]
    fn test_snapshot_critical_attention() {
        let mut summary = GoalSummary::new();
        summary.entity_count = 3;
        summary.tag_distribution.insert(GoalTag::survival(), 2);
        summary.tag_distribution.insert(GoalTag::combat(), 1);

        let snapshot = GoalSnapshot::new(1, summary, 100);

        assert!(snapshot.needs_attention);
        assert!(snapshot.stability < 0.5);
    }

    #[test]
    fn test_snapshot_staleness() {
        let summary = GoalSummary::new();
        let snapshot = GoalSnapshot::new(1, summary, 100);

        assert!(!snapshot.is_stale(150, 100));
        assert!(snapshot.is_stale(250, 100));
    }

    #[test]
    fn test_snapshot_projected_stability() {
        let mut summary = GoalSummary::new();
        summary.entity_count = 1;
        summary.tag_distribution.insert(GoalTag::idle(), 1);

        let snapshot = GoalSnapshot::new(1, summary, 100);
        let initial = snapshot.stability;
        let projected = snapshot.projected_stability(100);

        assert!(projected <= initial || (projected - initial).abs() < 0.1);
    }

    #[test]
    fn test_snapshot_needs_intervention() {
        let mut summary = GoalSummary::new();
        summary.entity_count = 1;
        summary.tag_distribution.insert(GoalTag::survival(), 1);

        let snapshot = GoalSnapshot::new(1, summary, 100);

        assert!(snapshot.needs_intervention(50));
    }

    #[test]
    fn test_summary_serde() {
        let mut summary = GoalSummary::new();
        summary.entity_count = 3;
        summary.average_score = 0.75;
        summary.active_goals.insert(GoalId::idle(), 2);
        summary.tag_distribution.insert(GoalTag::idle(), 2);
        summary.computed_at_tick = 500;

        let json = serde_json::to_string(&summary).unwrap();
        let restored: GoalSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.entity_count, 3);
        assert!((restored.average_score - 0.75).abs() < f32::EPSILON);
        assert_eq!(restored.goal_count(&GoalId::idle()), 2);
    }

    #[test]
    fn test_snapshot_serde() {
        let summary = GoalSummary::new();
        let snapshot = GoalSnapshot::new(42, summary, 100);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: GoalSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id, 42);
        assert_eq!(restored.snapshot_tick, 100);
    }

    #[test]
    fn test_summary_cooldown_tracking() {
        let mut selector = GoalSelector::new();
        let goal = GoalDef::new(GoalId::satisfy_hunger(), "Test")
            .with_cooldown(100)
            .with_priority(1.0);

        selector.register(goal);
        selector.advance_to(50);
        selector.complete_goal(&GoalId::satisfy_hunger());

        let mut summary = GoalSummary::new();
        summary.add_selector(&selector, 0.5);

        assert_eq!(summary.cooldown_count(&GoalId::satisfy_hunger()), 1);
    }

    #[test]
    fn test_empty_summary_defaults() {
        let summary = GoalSummary::new();

        assert!(summary.most_common_goal().is_none());
        assert!(summary.dominant_tag().is_none());
        assert!((summary.idle_ratio() - 1.0).abs() < f32::EPSILON);
    }
}

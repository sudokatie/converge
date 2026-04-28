//! Context for goal evaluation, integrating needs, sensors, and faction data.

use super::consideration::InputBinding;
use crate::faction::{FactionId, Standing};
use crate::needs::{NeedId, NeedSet};
use crate::sensor::{SensorKind, SensorSummary};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A named fact that can be queried during goal evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextFact {
    /// Name of the fact.
    pub name: String,
    /// Numeric value (usually 0.0 to 1.0).
    pub value: f32,
    /// Tick when this fact was last updated.
    pub updated_tick: u64,
}

impl ContextFact {
    /// Create a new context fact.
    #[must_use]
    pub fn new(name: impl Into<String>, value: f32, tick: u64) -> Self {
        Self {
            name: name.into(),
            value,
            updated_tick: tick,
        }
    }
}

/// Snapshot of context data for goal evaluation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GoalContext {
    /// Named facts for custom input bindings.
    facts: BTreeMap<String, ContextFact>,
    /// Normalized need values (0.0 = empty, 1.0 = full).
    need_values: BTreeMap<String, f32>,
    /// Need urgency scores.
    need_urgencies: BTreeMap<String, f32>,
    /// Current threat level (0.0 = safe, 1.0 = extreme danger).
    pub threat_level: f32,
    /// Danger proximity (0.0 = far, 1.0 = immediate).
    pub danger_proximity: f32,
    /// Stimulus intensities by sensor kind name.
    stimulus_intensities: BTreeMap<String, f32>,
    /// Count of nearby allies.
    pub ally_count: u32,
    /// Count of nearby enemies.
    pub enemy_count: u32,
    /// Territory ownership level (0.0 = unowned, 1.0 = controlled).
    pub territory_ownership: f32,
    /// Faction standings (normalized to -1.0 to 1.0).
    faction_standings: BTreeMap<String, f32>,
    /// Distance to leader (raw, not normalized).
    pub leader_distance: Option<f32>,
    /// Ticks since specific goals were completed.
    goal_completion_ticks: BTreeMap<String, u64>,
    /// Current simulation tick.
    pub current_tick: u64,
}

impl GoalContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for constructing context.
    #[must_use]
    pub fn builder() -> GoalContextBuilder {
        GoalContextBuilder::new()
    }

    /// Set a fact.
    pub fn set_fact(&mut self, name: impl Into<String>, value: f32) {
        let name = name.into();
        let fact = ContextFact::new(name.clone(), value, self.current_tick);
        self.facts.insert(name, fact);
    }

    /// Get a fact value.
    #[must_use]
    pub fn get_fact(&self, name: &str) -> Option<f32> {
        self.facts.get(name).map(|f| f.value)
    }

    /// Set a need value.
    pub fn set_need_value(&mut self, need_id: impl Into<String>, value: f32) {
        self.need_values.insert(need_id.into(), value);
    }

    /// Get a need value.
    #[must_use]
    pub fn get_need_value(&self, need_id: &str) -> Option<f32> {
        self.need_values.get(need_id).copied()
    }

    /// Set a need urgency.
    pub fn set_need_urgency(&mut self, need_id: impl Into<String>, urgency: f32) {
        self.need_urgencies.insert(need_id.into(), urgency);
    }

    /// Get a need urgency.
    #[must_use]
    pub fn get_need_urgency(&self, need_id: &str) -> Option<f32> {
        self.need_urgencies.get(need_id).copied()
    }

    /// Set stimulus intensity for a sensor kind.
    pub fn set_stimulus_intensity(&mut self, kind: impl Into<String>, intensity: f32) {
        self.stimulus_intensities.insert(kind.into(), intensity);
    }

    /// Get stimulus intensity for a sensor kind.
    #[must_use]
    pub fn get_stimulus_intensity(&self, kind: &str) -> Option<f32> {
        self.stimulus_intensities.get(kind).copied()
    }

    /// Set faction standing.
    pub fn set_faction_standing(&mut self, faction_id: impl Into<String>, standing: f32) {
        self.faction_standings.insert(faction_id.into(), standing);
    }

    /// Get faction standing.
    #[must_use]
    pub fn get_faction_standing(&self, faction_id: &str) -> Option<f32> {
        self.faction_standings.get(faction_id).copied()
    }

    /// Record a goal completion.
    pub fn record_goal_completion(&mut self, goal_id: impl Into<String>) {
        self.goal_completion_ticks
            .insert(goal_id.into(), self.current_tick);
    }

    /// Get ticks since a goal was completed.
    #[must_use]
    pub fn ticks_since_goal(&self, goal_id: &str) -> Option<u64> {
        self.goal_completion_ticks
            .get(goal_id)
            .map(|&tick| self.current_tick.saturating_sub(tick))
    }

    /// Resolve an input binding to a value.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn resolve_input(&self, binding: &InputBinding) -> f32 {
        match binding {
            InputBinding::NeedValue(need_id) => self.get_need_value(need_id).unwrap_or(1.0),

            InputBinding::NeedDeficit(need_id) => 1.0 - self.get_need_value(need_id).unwrap_or(1.0),

            InputBinding::NeedUrgency(need_id) => self
                .get_need_urgency(need_id)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),

            InputBinding::Fact(name) => self.get_fact(name).unwrap_or(0.0),

            InputBinding::ThreatLevel => self.threat_level,

            InputBinding::DangerProximity => self.danger_proximity,

            InputBinding::StimulusIntensity(kind) => {
                self.get_stimulus_intensity(kind).unwrap_or(0.0)
            }

            InputBinding::AllyCount { threshold } => {
                if *threshold == 0 {
                    0.0
                } else {
                    (self.ally_count as f32 / *threshold as f32).clamp(0.0, 1.0)
                }
            }

            InputBinding::EnemyCount { threshold } => {
                if *threshold == 0 {
                    0.0
                } else {
                    (self.enemy_count as f32 / *threshold as f32).clamp(0.0, 1.0)
                }
            }

            InputBinding::TerritoryOwnership => self.territory_ownership,

            InputBinding::FactionStanding(faction_id) => {
                let standing = self.get_faction_standing(faction_id).unwrap_or(0.0);
                f32::midpoint(standing, 1.0)
            }

            InputBinding::LeaderDistance { max_distance } => {
                if let Some(dist) = self.leader_distance {
                    if *max_distance > 0.0 {
                        (dist / max_distance).clamp(0.0, 1.0)
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            }

            InputBinding::TimeSinceGoal {
                goal_id,
                threshold_ticks,
            } => {
                if *threshold_ticks == 0 {
                    return 1.0;
                }
                let ticks = self.ticks_since_goal(goal_id).unwrap_or(u64::MAX);
                (ticks as f32 / *threshold_ticks as f32).clamp(0.0, 1.0)
            }

            InputBinding::Constant(value) => *value,
        }
    }
}

/// Builder for constructing [`GoalContext`] from various sources.
#[derive(Clone, Debug, Default)]
pub struct GoalContextBuilder {
    context: GoalContext,
}

impl GoalContextBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current tick.
    #[must_use]
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.context.current_tick = tick;
        self
    }

    /// Populate need values and urgencies from a [`NeedSet`].
    #[must_use]
    pub fn with_needs(mut self, needs: &NeedSet) -> Self {
        for need in needs.iter() {
            self.context
                .need_values
                .insert(need.id.as_str().to_string(), need.normalized());
            self.context
                .need_urgencies
                .insert(need.id.as_str().to_string(), need.urgency());
        }
        self
    }

    /// Populate sensor data from a [`SensorSummary`].
    #[must_use]
    pub fn with_sensor_summary(mut self, summary: &SensorSummary) -> Self {
        self.context.threat_level = Self::calculate_threat_level(summary);

        for kind in SensorKind::ALL {
            if let Some(kind_summary) = summary.get_kind_summary(*kind) {
                self.context
                    .stimulus_intensities
                    .insert(kind.name().to_string(), kind_summary.average_intensity());
            }
        }

        self
    }

    fn calculate_threat_level(summary: &SensorSummary) -> f32 {
        if summary.entity_count == 0 {
            return 0.0;
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "count precision loss acceptable"
        )]
        let threat_ratio = if summary.total_observations > 0 {
            summary.threat_count() as f32 / summary.total_observations as f32
        } else {
            0.0
        };

        let urgency_factor = (summary.average_urgency() / 100.0).clamp(0.0, 1.0);

        (threat_ratio * 0.6 + urgency_factor * 0.4).clamp(0.0, 1.0)
    }

    /// Set threat level directly.
    #[must_use]
    pub fn with_threat_level(mut self, level: f32) -> Self {
        self.context.threat_level = level.clamp(0.0, 1.0);
        self
    }

    /// Set danger proximity.
    #[must_use]
    pub fn with_danger_proximity(mut self, proximity: f32) -> Self {
        self.context.danger_proximity = proximity.clamp(0.0, 1.0);
        self
    }

    /// Set ally count.
    #[must_use]
    pub fn with_ally_count(mut self, count: u32) -> Self {
        self.context.ally_count = count;
        self
    }

    /// Set enemy count.
    #[must_use]
    pub fn with_enemy_count(mut self, count: u32) -> Self {
        self.context.enemy_count = count;
        self
    }

    /// Set territory ownership.
    #[must_use]
    pub fn with_territory_ownership(mut self, ownership: f32) -> Self {
        self.context.territory_ownership = ownership.clamp(0.0, 1.0);
        self
    }

    /// Set leader distance.
    #[must_use]
    pub fn with_leader_distance(mut self, distance: f32) -> Self {
        self.context.leader_distance = Some(distance);
        self
    }

    /// Set a custom fact.
    #[must_use]
    pub fn with_fact(mut self, name: impl Into<String>, value: f32) -> Self {
        self.context.set_fact(name, value);
        self
    }

    /// Add faction standings from Standing values.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "standing value range is small")]
    pub fn with_faction_standing(mut self, faction_id: &FactionId, standing: &Standing) -> Self {
        let normalized = (standing.value() as f32 / 100.0).clamp(-1.0, 1.0);
        self.context
            .faction_standings
            .insert(faction_id.as_str().to_string(), normalized);
        self
    }

    /// Add a need value directly.
    #[must_use]
    pub fn with_need_value(mut self, need_id: &NeedId, value: f32) -> Self {
        self.context
            .need_values
            .insert(need_id.as_str().to_string(), value.clamp(0.0, 1.0));
        self
    }

    /// Import goal completion history.
    #[must_use]
    pub fn with_goal_completions(mut self, completions: &BTreeMap<String, u64>) -> Self {
        self.context.goal_completion_ticks = completions.clone();
        self
    }

    /// Build the context.
    #[must_use]
    pub fn build(self) -> GoalContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::needs::Need;

    #[test]
    fn test_context_fact() {
        let fact = ContextFact::new("is_daytime", 1.0, 100);

        assert_eq!(fact.name, "is_daytime");
        assert!((fact.value - 1.0).abs() < f32::EPSILON);
        assert_eq!(fact.updated_tick, 100);
    }

    #[test]
    fn test_context_new() {
        let ctx = GoalContext::new();

        assert!((ctx.threat_level).abs() < f32::EPSILON);
        assert_eq!(ctx.ally_count, 0);
    }

    #[test]
    fn test_context_facts() {
        let mut ctx = GoalContext::new();

        ctx.set_fact("test", 0.75);
        assert!((ctx.get_fact("test").unwrap() - 0.75).abs() < f32::EPSILON);
        assert!(ctx.get_fact("missing").is_none());
    }

    #[test]
    fn test_context_needs() {
        let mut ctx = GoalContext::new();

        ctx.set_need_value("hunger", 0.6);
        ctx.set_need_urgency("hunger", 2.5);

        assert!((ctx.get_need_value("hunger").unwrap() - 0.6).abs() < f32::EPSILON);
        assert!((ctx.get_need_urgency("hunger").unwrap() - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_context_goal_completions() {
        let mut ctx = GoalContext::new();
        ctx.current_tick = 100;

        ctx.record_goal_completion("flee_danger");
        assert_eq!(ctx.ticks_since_goal("flee_danger"), Some(0));

        ctx.current_tick = 150;
        assert_eq!(ctx.ticks_since_goal("flee_danger"), Some(50));
        assert!(ctx.ticks_since_goal("unknown").is_none());
    }

    #[test]
    fn test_resolve_need_value() {
        let mut ctx = GoalContext::new();
        ctx.set_need_value("hunger", 0.7);

        let binding = InputBinding::need_value("hunger");
        assert!((ctx.resolve_input(&binding) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_need_deficit() {
        let mut ctx = GoalContext::new();
        ctx.set_need_value("hunger", 0.7);

        let binding = InputBinding::need_deficit("hunger");
        assert!((ctx.resolve_input(&binding) - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_ally_count() {
        let mut ctx = GoalContext::new();
        ctx.ally_count = 3;

        let binding = InputBinding::ally_count(5);
        assert!((ctx.resolve_input(&binding) - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_ally_count_clamped() {
        let mut ctx = GoalContext::new();
        ctx.ally_count = 10;

        let binding = InputBinding::ally_count(5);
        assert!((ctx.resolve_input(&binding) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_faction_standing() {
        let mut ctx = GoalContext::new();
        ctx.set_faction_standing("pirates", -0.5);

        let binding = InputBinding::faction_standing("pirates");
        assert!((ctx.resolve_input(&binding) - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_leader_distance() {
        let mut ctx = GoalContext::new();
        ctx.leader_distance = Some(50.0);

        let binding = InputBinding::leader_distance(100.0);
        assert!((ctx.resolve_input(&binding) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_time_since_goal() {
        let mut ctx = GoalContext::new();
        ctx.current_tick = 200;
        ctx.goal_completion_ticks.insert("rest".into(), 100);

        let binding = InputBinding::time_since_goal("rest", 200);
        assert!((ctx.resolve_input(&binding) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_constant() {
        let ctx = GoalContext::new();
        let binding = InputBinding::constant(0.42);
        assert!((ctx.resolve_input(&binding) - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_basic() {
        let ctx = GoalContext::builder()
            .with_tick(100)
            .with_threat_level(0.5)
            .with_ally_count(3)
            .with_enemy_count(2)
            .build();

        assert_eq!(ctx.current_tick, 100);
        assert!((ctx.threat_level - 0.5).abs() < f32::EPSILON);
        assert_eq!(ctx.ally_count, 3);
        assert_eq!(ctx.enemy_count, 2);
    }

    #[test]
    fn test_builder_with_needs() {
        let mut needs = NeedSet::new();
        let mut hunger = Need::new(NeedId::hunger(), 100.0, 1.0);
        hunger.set_value(60.0);
        needs.add(hunger);

        let ctx = GoalContext::builder().with_needs(&needs).build();

        assert!((ctx.get_need_value("hunger").unwrap() - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_with_fact() {
        let ctx = GoalContext::builder()
            .with_fact("is_night", 1.0)
            .with_fact("has_shelter", 0.0)
            .build();

        assert!((ctx.get_fact("is_night").unwrap() - 1.0).abs() < f32::EPSILON);
        assert!((ctx.get_fact("has_shelter").unwrap()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_with_territory() {
        let ctx = GoalContext::builder()
            .with_territory_ownership(0.75)
            .with_leader_distance(25.0)
            .build();

        assert!((ctx.territory_ownership - 0.75).abs() < f32::EPSILON);
        assert!((ctx.leader_distance.unwrap() - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_with_faction_standing() {
        let faction_id = FactionId::new("traders");
        let standing = Standing::with_value(50);

        let ctx = GoalContext::builder()
            .with_faction_standing(&faction_id, &standing)
            .build();

        assert!((ctx.get_faction_standing("traders").unwrap() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_context_serde() {
        let ctx = GoalContext::builder()
            .with_tick(500)
            .with_threat_level(0.3)
            .with_ally_count(5)
            .with_fact("custom", 0.8)
            .build();

        let json = serde_json::to_string(&ctx).unwrap();
        let restored: GoalContext = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick, 500);
        assert!((restored.threat_level - 0.3).abs() < f32::EPSILON);
        assert_eq!(restored.ally_count, 5);
        assert!((restored.get_fact("custom").unwrap() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolve_missing_values() {
        let ctx = GoalContext::new();

        assert!(
            (ctx.resolve_input(&InputBinding::need_value("missing")) - 1.0).abs() < f32::EPSILON
        );
        assert!((ctx.resolve_input(&InputBinding::need_deficit("missing"))).abs() < f32::EPSILON);
        assert!(
            (ctx.resolve_input(&InputBinding::stimulus_intensity("sight"))).abs() < f32::EPSILON
        );
    }
}

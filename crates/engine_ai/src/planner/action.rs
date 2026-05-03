//! Action definitions, registry, and scoring for the planner.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::facts::{BeliefState, FactModification, FactRequirement};
use super::ids::ActionDefId;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RiskLevel {
    #[default]
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    #[must_use]
    pub fn multiplier(self) -> f32 {
        match self {
            Self::None => 1.0,
            Self::Low => 0.9,
            Self::Medium => 0.7,
            Self::High => 0.4,
            Self::Critical => 0.1,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionCost {
    pub base_cost: f32,
    pub time_ticks: u64,
    pub resource_cost: f32,
    pub risk: RiskLevel,
}

impl ActionCost {
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "planner action costs are bounded configuration values"
    )]
    pub fn new(base_cost: u32) -> Self {
        Self {
            base_cost: base_cost as f32,
            time_ticks: u64::from(base_cost),
            resource_cost: 0.0,
            risk: RiskLevel::None,
        }
    }

    #[must_use]
    pub fn with_time(mut self, ticks: u64) -> Self {
        self.time_ticks = ticks;
        self
    }

    #[must_use]
    pub fn with_resource_cost(mut self, cost: f32) -> Self {
        self.resource_cost = cost;
        self
    }

    #[must_use]
    pub fn with_risk(mut self, risk: RiskLevel) -> Self {
        self.risk = risk;
        self
    }

    #[must_use]
    pub fn total_cost(&self) -> f32 {
        (self.base_cost + self.resource_cost) * self.risk.multiplier()
    }
}

impl Default for ActionCost {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionUtility {
    pub base_utility: f32,
    pub goal_contribution: f32,
    pub secondary_benefits: f32,
}

impl ActionUtility {
    #[must_use]
    pub fn new(base: f32) -> Self {
        Self {
            base_utility: base,
            goal_contribution: 0.0,
            secondary_benefits: 0.0,
        }
    }

    #[must_use]
    pub fn with_goal_contribution(mut self, contribution: f32) -> Self {
        self.goal_contribution = contribution;
        self
    }

    #[must_use]
    pub fn with_secondary_benefits(mut self, benefits: f32) -> Self {
        self.secondary_benefits = benefits;
        self
    }

    #[must_use]
    pub fn total_utility(&self) -> f32 {
        self.base_utility + self.goal_contribution + self.secondary_benefits
    }
}

impl Default for ActionUtility {
    fn default() -> Self {
        Self::new(1.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub preconditions: Vec<FactRequirement>,
    pub effects: Vec<FactModification>,
    pub cost: ActionCost,
    pub utility: ActionUtility,
    pub tags: Vec<String>,
}

impl ActionDef {
    #[must_use]
    pub fn new(id: impl Into<ActionDefId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            preconditions: Vec::new(),
            effects: Vec::new(),
            cost: ActionCost::default(),
            utility: ActionUtility::default(),
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_precondition(mut self, precondition: FactRequirement) -> Self {
        self.preconditions.push(precondition);
        self
    }

    #[must_use]
    pub fn with_preconditions(mut self, preconditions: Vec<FactRequirement>) -> Self {
        self.preconditions.extend(preconditions);
        self
    }

    #[must_use]
    pub fn with_effect(mut self, effect: FactModification) -> Self {
        self.effects.push(effect);
        self
    }

    #[must_use]
    pub fn with_effects(mut self, effects: Vec<FactModification>) -> Self {
        self.effects.extend(effects);
        self
    }

    #[must_use]
    pub fn with_cost(mut self, cost: ActionCost) -> Self {
        self.cost = cost;
        self
    }

    #[must_use]
    pub fn with_utility(mut self, utility: ActionUtility) -> Self {
        self.utility = utility;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    #[must_use]
    pub fn is_applicable(&self, state: &BeliefState) -> bool {
        state.satisfies(&self.preconditions)
    }

    #[must_use]
    pub fn apply(&self, state: &BeliefState) -> BeliefState {
        state.with_modifications(&self.effects)
    }

    pub fn apply_to(&self, state: &mut BeliefState) {
        state.apply_all(&self.effects);
    }

    #[must_use]
    pub fn score(&self) -> f32 {
        let cost = self.cost.total_cost();
        if cost <= 0.0 {
            return self.utility.total_utility();
        }
        self.utility.total_utility() / cost
    }

    #[must_use]
    pub fn precondition_count(&self) -> usize {
        self.preconditions.len()
    }

    #[must_use]
    pub fn effect_count(&self) -> usize {
        self.effects.len()
    }

    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionRegistry {
    actions: BTreeMap<ActionDefId, ActionDef>,
}

impl ActionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, action: ActionDef) {
        self.actions.insert(action.id.clone(), action);
    }

    pub fn unregister(&mut self, id: &ActionDefId) -> Option<ActionDef> {
        self.actions.remove(id)
    }

    #[must_use]
    pub fn get(&self, id: &ActionDefId) -> Option<&ActionDef> {
        self.actions.get(id)
    }

    #[must_use]
    pub fn contains(&self, id: &ActionDefId) -> bool {
        self.actions.contains_key(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActionDef> {
        self.actions.values()
    }

    pub fn applicable_actions<'s, 'b>(
        &'s self,
        state: &'b BeliefState,
    ) -> impl Iterator<Item = &'s ActionDef> + 'b
    where
        's: 'b,
    {
        self.actions
            .values()
            .filter(move |a| a.is_applicable(state))
    }

    pub fn actions_with_tag<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a ActionDef> {
        self.actions.values().filter(move |a| a.has_tag(tag))
    }

    #[must_use]
    pub fn best_action(&self, state: &BeliefState) -> Option<&ActionDef> {
        self.applicable_actions(state).max_by(|a, b| {
            a.score()
                .partial_cmp(&b.score())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        })
    }

    pub fn clear(&mut self) {
        self.actions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_action(id: &str) -> ActionDef {
        ActionDef::new(id, format!("Test {id}"))
            .with_cost(ActionCost::new(10))
            .with_utility(ActionUtility::new(5.0))
    }

    #[test]
    fn test_risk_level() {
        assert!((RiskLevel::None.multiplier() - 1.0).abs() < f32::EPSILON);
        assert!(RiskLevel::High.multiplier() < RiskLevel::Low.multiplier());
        assert_eq!(RiskLevel::Medium.as_str(), "medium");
    }

    #[test]
    fn test_action_cost() {
        let cost = ActionCost::new(10)
            .with_time(20)
            .with_resource_cost(5.0)
            .with_risk(RiskLevel::Low);

        assert_eq!(cost.time_ticks, 20);
        assert!((cost.total_cost() - 13.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_action_utility() {
        let utility = ActionUtility::new(10.0)
            .with_goal_contribution(5.0)
            .with_secondary_benefits(2.0);

        assert!((utility.total_utility() - 17.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_action_def_basic() {
        let action = ActionDef::new("move", "Move To Location")
            .with_precondition(FactRequirement::is_true("can_move"))
            .with_effect(FactModification::set_true("at_destination"))
            .with_cost(ActionCost::new(10))
            .with_utility(ActionUtility::new(15.0))
            .with_tag("movement");

        assert_eq!(action.id.as_str(), "move");
        assert_eq!(action.precondition_count(), 1);
        assert_eq!(action.effect_count(), 1);
        assert!(action.has_tag("movement"));
        assert!(!action.has_tag("combat"));
    }

    #[test]
    fn test_action_def_applicability() {
        let action = ActionDef::new("attack", "Attack")
            .with_precondition(FactRequirement::is_true("has_weapon"))
            .with_precondition(FactRequirement::at_least("ammo", 1));

        let mut state = BeliefState::new();
        assert!(!action.is_applicable(&state));

        state.set_bool("has_weapon", true);
        assert!(!action.is_applicable(&state));

        state.set_int("ammo", 5);
        assert!(action.is_applicable(&state));
    }

    #[test]
    fn test_action_def_apply() {
        let action = ActionDef::new("reload", "Reload")
            .with_effect(FactModification::set_int("ammo", 10))
            .with_effect(FactModification::set_false("reloading"));

        let mut state = BeliefState::new();
        state.set_int("ammo", 0);
        state.set_bool("reloading", true);

        let new_state = action.apply(&state);

        assert_eq!(state.get_int("ammo"), Some(0));
        assert_eq!(new_state.get_int("ammo"), Some(10));
        assert_eq!(new_state.get_bool("reloading"), Some(false));
    }

    #[test]
    fn test_action_def_score() {
        let low_score = ActionDef::new("slow", "Slow Action")
            .with_cost(ActionCost::new(100))
            .with_utility(ActionUtility::new(10.0));

        let high_score = ActionDef::new("fast", "Fast Action")
            .with_cost(ActionCost::new(10))
            .with_utility(ActionUtility::new(50.0));

        assert!(high_score.score() > low_score.score());
    }

    #[test]
    fn test_action_registry_basic() {
        let mut registry = ActionRegistry::new();
        assert!(registry.is_empty());

        registry.register(make_test_action("action1"));
        registry.register(make_test_action("action2"));

        assert_eq!(registry.len(), 2);
        assert!(registry.contains(&ActionDefId::new("action1")));
        assert!(registry.get(&ActionDefId::new("action1")).is_some());
    }

    #[test]
    fn test_action_registry_applicable() {
        let mut registry = ActionRegistry::new();

        let unrestricted = ActionDef::new("open", "Open");
        let restricted = ActionDef::new("attack", "Attack")
            .with_precondition(FactRequirement::is_true("has_weapon"));

        registry.register(unrestricted);
        registry.register(restricted);

        let state = BeliefState::new();
        let applicable: Vec<_> = registry.applicable_actions(&state).collect();
        assert_eq!(applicable.len(), 1);
        assert_eq!(applicable[0].id.as_str(), "open");

        let mut armed_state = BeliefState::new();
        armed_state.set_bool("has_weapon", true);
        let applicable: Vec<_> = registry.applicable_actions(&armed_state).collect();
        assert_eq!(applicable.len(), 2);
    }

    #[test]
    fn test_action_registry_best_action() {
        let mut registry = ActionRegistry::new();

        let low = ActionDef::new("low", "Low Score")
            .with_cost(ActionCost::new(100))
            .with_utility(ActionUtility::new(10.0));

        let high = ActionDef::new("high", "High Score")
            .with_cost(ActionCost::new(10))
            .with_utility(ActionUtility::new(50.0));

        registry.register(low);
        registry.register(high);

        let state = BeliefState::new();
        let best = registry.best_action(&state);

        assert!(best.is_some());
        assert_eq!(best.unwrap().id.as_str(), "high");
    }

    #[test]
    fn test_action_registry_deterministic_order() {
        let mut registry = ActionRegistry::new();

        registry.register(make_test_action("zzz"));
        registry.register(make_test_action("aaa"));
        registry.register(make_test_action("mmm"));

        let ids: Vec<_> = registry.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn test_action_def_serde() {
        let action = ActionDef::new("test", "Test Action")
            .with_precondition(FactRequirement::is_true("ready"))
            .with_effect(FactModification::set_true("done"))
            .with_cost(ActionCost::new(10).with_risk(RiskLevel::Low))
            .with_utility(ActionUtility::new(20.0));

        let json = serde_json::to_string(&action).unwrap();
        let restored: ActionDef = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, action.id);
        assert_eq!(restored.precondition_count(), 1);
        assert_eq!(restored.effect_count(), 1);
    }

    #[test]
    fn test_action_registry_serde() {
        let mut registry = ActionRegistry::new();
        registry.register(make_test_action("action1"));
        registry.register(make_test_action("action2"));

        let json = serde_json::to_string(&registry).unwrap();
        let restored: ActionRegistry = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert!(restored.contains(&ActionDefId::new("action1")));
    }

    #[test]
    fn test_action_bincode() {
        let action = ActionDef::new("test", "Test Action")
            .with_precondition(FactRequirement::at_least("health", 50))
            .with_effect(FactModification::increment("xp", 10))
            .with_cost(ActionCost::new(5))
            .with_utility(ActionUtility::new(15.0));

        let bytes = bincode::serialize(&action).unwrap();
        let restored: ActionDef = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id, action.id);
        assert_eq!(restored.precondition_count(), action.precondition_count());
    }

    #[test]
    fn test_action_registry_bincode() {
        let mut registry = ActionRegistry::new();
        registry.register(make_test_action("a"));
        registry.register(make_test_action("b"));

        let bytes = bincode::serialize(&registry).unwrap();
        let restored: ActionRegistry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.len(), 2);
    }
}

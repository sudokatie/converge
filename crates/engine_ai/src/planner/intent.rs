//! Planning intents and goals for high-agency actors.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::facts::{BeliefState, FactRequirement};
use super::ids::{FactionScopeId, IntentId, LocationId, ResourceTypeId};

/// Priority level for an intent.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum IntentPriority {
    Background,
    #[default]
    Normal,
    High,
    Urgent,
    Critical,
}

impl IntentPriority {
    #[must_use]
    pub fn weight(self) -> f32 {
        match self {
            Self::Background => 0.5,
            Self::Normal => 1.0,
            Self::High => 1.5,
            Self::Urgent => 2.0,
            Self::Critical => 3.0,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
            Self::Critical => "critical",
        }
    }
}

/// Tag for categorizing intents.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntentTag(pub String);

impl IntentTag {
    pub const SURVIVAL: &'static str = "survival";
    pub const ECONOMIC: &'static str = "economic";
    pub const MILITARY: &'static str = "military";
    pub const SOCIAL: &'static str = "social";
    pub const TERRITORIAL: &'static str = "territorial";
    pub const EXPLORATION: &'static str = "exploration";

    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    #[must_use]
    pub fn survival() -> Self {
        Self::new(Self::SURVIVAL)
    }

    #[must_use]
    pub fn economic() -> Self {
        Self::new(Self::ECONOMIC)
    }

    #[must_use]
    pub fn military() -> Self {
        Self::new(Self::MILITARY)
    }

    #[must_use]
    pub fn social() -> Self {
        Self::new(Self::SOCIAL)
    }

    #[must_use]
    pub fn territorial() -> Self {
        Self::new(Self::TERRITORIAL)
    }

    #[must_use]
    pub fn exploration() -> Self {
        Self::new(Self::EXPLORATION)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for IntentTag {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Parameters for an intent that provide context.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IntentParams {
    pub target_location: Option<LocationId>,
    pub target_resource: Option<ResourceTypeId>,
    pub target_quantity: Option<i64>,
    pub target_faction: Option<FactionScopeId>,
    pub deadline_tick: Option<u64>,
    pub min_success_probability: Option<f32>,
    pub metadata: std::collections::BTreeMap<String, String>,
}

impl IntentParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_location(mut self, location: LocationId) -> Self {
        self.target_location = Some(location);
        self
    }

    #[must_use]
    pub fn with_resource(mut self, resource: ResourceTypeId, quantity: i64) -> Self {
        self.target_resource = Some(resource);
        self.target_quantity = Some(quantity);
        self
    }

    #[must_use]
    pub fn with_faction(mut self, faction: FactionScopeId) -> Self {
        self.target_faction = Some(faction);
        self
    }

    #[must_use]
    pub fn with_deadline(mut self, tick: u64) -> Self {
        self.deadline_tick = Some(tick);
        self
    }

    #[must_use]
    pub fn with_min_probability(mut self, probability: f32) -> Self {
        self.min_success_probability = Some(probability);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn has_deadline(&self) -> bool {
        self.deadline_tick.is_some()
    }

    #[must_use]
    pub fn is_past_deadline(&self, current_tick: u64) -> bool {
        self.deadline_tick.is_some_and(|d| current_tick > d)
    }
}

/// Definition of a planning intent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Intent {
    pub id: IntentId,
    pub name: String,
    pub priority: IntentPriority,
    pub tags: BTreeSet<IntentTag>,
    pub goal_conditions: Vec<FactRequirement>,
    pub params: IntentParams,
    pub created_tick: u64,
    pub utility_weight: f32,
}

impl Intent {
    #[must_use]
    pub fn new(id: impl Into<IntentId>, name: impl Into<String>, created_tick: u64) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            priority: IntentPriority::Normal,
            tags: BTreeSet::new(),
            goal_conditions: Vec::new(),
            params: IntentParams::new(),
            created_tick,
            utility_weight: 1.0,
        }
    }

    #[must_use]
    pub fn with_priority(mut self, priority: IntentPriority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: IntentTag) -> Self {
        self.tags.insert(tag);
        self
    }

    #[must_use]
    pub fn with_goal_condition(mut self, condition: FactRequirement) -> Self {
        self.goal_conditions.push(condition);
        self
    }

    #[must_use]
    pub fn with_params(mut self, params: IntentParams) -> Self {
        self.params = params;
        self
    }

    #[must_use]
    pub fn with_utility_weight(mut self, weight: f32) -> Self {
        self.utility_weight = weight;
        self
    }

    #[must_use]
    pub fn is_satisfied(&self, state: &BeliefState) -> bool {
        state.satisfies(&self.goal_conditions)
    }

    #[must_use]
    pub fn has_tag(&self, tag: &IntentTag) -> bool {
        self.tags.contains(tag)
    }

    #[must_use]
    pub fn effective_priority(&self) -> f32 {
        self.priority.weight() * self.utility_weight
    }

    #[must_use]
    pub fn age_ticks(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick)
    }

    #[must_use]
    pub fn is_past_deadline(&self, current_tick: u64) -> bool {
        self.params.is_past_deadline(current_tick)
    }

    #[must_use]
    pub fn goal_condition_count(&self) -> usize {
        self.goal_conditions.len()
    }
}

/// Active intent with runtime state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveIntent {
    pub intent: Intent,
    pub attempts: u32,
    pub last_attempt_tick: Option<u64>,
    pub progress_estimate: f32,
    pub abandoned: bool,
    pub abandon_reason: Option<String>,
}

impl ActiveIntent {
    #[must_use]
    pub fn new(intent: Intent) -> Self {
        Self {
            intent,
            attempts: 0,
            last_attempt_tick: None,
            progress_estimate: 0.0,
            abandoned: false,
            abandon_reason: None,
        }
    }

    pub fn record_attempt(&mut self, tick: u64) {
        self.attempts += 1;
        self.last_attempt_tick = Some(tick);
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress_estimate = progress.clamp(0.0, 1.0);
    }

    pub fn abandon(&mut self, reason: impl Into<String>) {
        self.abandoned = true;
        self.abandon_reason = Some(reason.into());
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.abandoned
    }

    #[must_use]
    pub fn is_satisfied(&self, state: &BeliefState) -> bool {
        self.intent.is_satisfied(state)
    }

    #[must_use]
    pub fn ticks_since_attempt(&self, current_tick: u64) -> Option<u64> {
        self.last_attempt_tick
            .map(|t| current_tick.saturating_sub(t))
    }
}

/// Collection of intents for an actor.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IntentSet {
    intents: Vec<ActiveIntent>,
    max_intents: usize,
}

impl IntentSet {
    #[must_use]
    pub fn new(max_intents: usize) -> Self {
        Self {
            intents: Vec::new(),
            max_intents,
        }
    }

    pub fn add(&mut self, intent: Intent) -> bool {
        if self.intents.len() >= self.max_intents {
            return false;
        }
        self.intents.push(ActiveIntent::new(intent));
        self.sort_by_priority();
        true
    }

    pub fn remove(&mut self, id: &IntentId) -> Option<ActiveIntent> {
        if let Some(pos) = self.intents.iter().position(|i| &i.intent.id == id) {
            Some(self.intents.remove(pos))
        } else {
            None
        }
    }

    #[must_use]
    pub fn get(&self, id: &IntentId) -> Option<&ActiveIntent> {
        self.intents.iter().find(|i| &i.intent.id == id)
    }

    pub fn get_mut(&mut self, id: &IntentId) -> Option<&mut ActiveIntent> {
        self.intents.iter_mut().find(|i| &i.intent.id == id)
    }

    #[must_use]
    pub fn highest_priority(&self) -> Option<&ActiveIntent> {
        self.intents.iter().find(|i| i.is_active())
    }

    pub fn highest_priority_mut(&mut self) -> Option<&mut ActiveIntent> {
        self.intents.iter_mut().find(|i| i.is_active())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.intents.iter().filter(|i| i.is_active()).count()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActiveIntent> {
        self.intents.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ActiveIntent> {
        self.intents.iter_mut()
    }

    pub fn active_intents(&self) -> impl Iterator<Item = &ActiveIntent> {
        self.intents.iter().filter(|i| i.is_active())
    }

    pub fn prune_satisfied(&mut self, state: &BeliefState) -> Vec<ActiveIntent> {
        let mut satisfied = Vec::new();
        self.intents.retain(|i| {
            if i.is_satisfied(state) {
                satisfied.push(i.clone());
                false
            } else {
                true
            }
        });
        satisfied
    }

    pub fn prune_abandoned(&mut self) -> Vec<ActiveIntent> {
        let mut abandoned = Vec::new();
        self.intents.retain(|i| {
            if i.abandoned {
                abandoned.push(i.clone());
                false
            } else {
                true
            }
        });
        abandoned
    }

    fn sort_by_priority(&mut self) {
        self.intents.sort_by(|a, b| {
            b.intent
                .effective_priority()
                .partial_cmp(&a.intent.effective_priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn clear(&mut self) {
        self.intents.clear();
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_priority() {
        assert!(IntentPriority::Critical.weight() > IntentPriority::Normal.weight());
        assert!(IntentPriority::Normal.weight() > IntentPriority::Background.weight());
        assert_eq!(IntentPriority::High.as_str(), "high");
    }

    #[test]
    fn test_intent_tag() {
        let tag = IntentTag::military();
        assert_eq!(tag.as_str(), "military");

        let custom: IntentTag = "custom".into();
        assert_eq!(custom.as_str(), "custom");
    }

    #[test]
    fn test_intent_params() {
        let params = IntentParams::new()
            .with_location(LocationId::new(5))
            .with_resource(ResourceTypeId::new("gold"), 100)
            .with_deadline(1000)
            .with_metadata("key", "value");

        assert_eq!(params.target_location, Some(LocationId::new(5)));
        assert_eq!(params.target_quantity, Some(100));
        assert!(params.has_deadline());
        assert!(!params.is_past_deadline(500));
        assert!(params.is_past_deadline(1500));
    }

    #[test]
    fn test_intent_builder() {
        let intent = Intent::new(IntentId::acquire_resource(), "Acquire Gold", 0)
            .with_priority(IntentPriority::High)
            .with_tag(IntentTag::economic())
            .with_goal_condition(FactRequirement::at_least("gold", 100))
            .with_utility_weight(1.5);

        assert_eq!(intent.id.as_str(), "acquire_resource");
        assert_eq!(intent.priority, IntentPriority::High);
        assert!(intent.has_tag(&IntentTag::economic()));
        assert_eq!(intent.goal_condition_count(), 1);
    }

    #[test]
    fn test_intent_satisfaction() {
        let intent = Intent::new("get_weapon", "Get Weapon", 0)
            .with_goal_condition(FactRequirement::is_true("has_weapon"));

        let mut state = BeliefState::new();
        assert!(!intent.is_satisfied(&state));

        state.set_bool("has_weapon", true);
        assert!(intent.is_satisfied(&state));
    }

    #[test]
    fn test_intent_effective_priority() {
        let normal = Intent::new("a", "A", 0).with_priority(IntentPriority::Normal);
        let weighted = Intent::new("b", "B", 0)
            .with_priority(IntentPriority::Normal)
            .with_utility_weight(2.0);

        assert!(weighted.effective_priority() > normal.effective_priority());
    }

    #[test]
    fn test_active_intent() {
        let intent = Intent::new("test", "Test", 0);
        let mut active = ActiveIntent::new(intent);

        assert!(active.is_active());
        assert_eq!(active.attempts, 0);

        active.record_attempt(100);
        assert_eq!(active.attempts, 1);
        assert_eq!(active.last_attempt_tick, Some(100));

        active.update_progress(0.5);
        assert!((active.progress_estimate - 0.5).abs() < f32::EPSILON);

        active.abandon("too difficult");
        assert!(!active.is_active());
        assert_eq!(active.abandon_reason, Some("too difficult".to_string()));
    }

    #[test]
    fn test_intent_set() {
        let mut set = IntentSet::new(3);

        let low = Intent::new("low", "Low", 0).with_priority(IntentPriority::Background);
        let high = Intent::new("high", "High", 0).with_priority(IntentPriority::High);
        let normal = Intent::new("normal", "Normal", 0).with_priority(IntentPriority::Normal);

        assert!(set.add(low));
        assert!(set.add(high));
        assert!(set.add(normal));
        assert!(!set.add(Intent::new("extra", "Extra", 0)));

        assert_eq!(set.len(), 3);
        assert_eq!(set.highest_priority().unwrap().intent.id.as_str(), "high");
    }

    #[test]
    fn test_intent_set_prune() {
        let mut set = IntentSet::new(5);

        let satisfied_intent = Intent::new("sat", "Satisfied", 0)
            .with_goal_condition(FactRequirement::is_true("done"));
        let unsatisfied_intent = Intent::new("unsat", "Unsatisfied", 0)
            .with_goal_condition(FactRequirement::is_true("not_done"));

        set.add(satisfied_intent);
        set.add(unsatisfied_intent);

        let mut state = BeliefState::new();
        state.set_bool("done", true);

        let pruned = set.prune_satisfied(&state);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].intent.id.as_str(), "sat");
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_intent_set_abandon() {
        let mut set = IntentSet::new(5);
        set.add(Intent::new("a", "A", 0));
        set.add(Intent::new("b", "B", 0));

        if let Some(active) = set.get_mut(&IntentId::new("a")) {
            active.abandon("failed");
        }

        let abandoned = set.prune_abandoned();
        assert_eq!(abandoned.len(), 1);
        assert_eq!(set.active_count(), 1);
    }

    #[test]
    fn test_intent_serde() {
        let intent = Intent::new(IntentId::flee_threat(), "Flee From Danger", 100)
            .with_priority(IntentPriority::Urgent)
            .with_tag(IntentTag::survival())
            .with_params(IntentParams::new().with_location(LocationId::new(42)));

        let json = serde_json::to_string(&intent).unwrap();
        let restored: Intent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, intent.id);
        assert_eq!(restored.priority, IntentPriority::Urgent);
        assert!(restored.has_tag(&IntentTag::survival()));
        assert_eq!(restored.params.target_location, Some(LocationId::new(42)));
    }

    #[test]
    fn test_active_intent_serde() {
        let intent = Intent::new("test", "Test", 0);
        let mut active = ActiveIntent::new(intent);
        active.record_attempt(50);
        active.update_progress(0.3);

        let json = serde_json::to_string(&active).unwrap();
        let restored: ActiveIntent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.attempts, 1);
        assert!((restored.progress_estimate - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intent_set_serde() {
        let mut set = IntentSet::new(5);
        set.add(Intent::new("a", "A", 0).with_priority(IntentPriority::High));
        set.add(Intent::new("b", "B", 0));

        let json = serde_json::to_string(&set).unwrap();
        let restored: IntentSet = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_intent_bincode() {
        let intent = Intent::new(IntentId::defend(), "Defend Position", 100)
            .with_priority(IntentPriority::Critical)
            .with_tag(IntentTag::military())
            .with_goal_condition(FactRequirement::is_true("position_secure"));

        let bytes = bincode::serialize(&intent).unwrap();
        let restored: Intent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id, intent.id);
        assert_eq!(restored.priority, IntentPriority::Critical);
        assert!(restored.has_tag(&IntentTag::military()));
    }

    #[test]
    fn test_active_intent_bincode() {
        let intent = Intent::new("test", "Test", 0);
        let mut active = ActiveIntent::new(intent);
        active.record_attempt(50);

        let bytes = bincode::serialize(&active).unwrap();
        let restored: ActiveIntent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.attempts, 1);
    }

    #[test]
    fn test_intent_set_bincode() {
        let mut set = IntentSet::new(5);
        set.add(Intent::new("a", "A", 0).with_priority(IntentPriority::High));
        set.add(Intent::new("b", "B", 0));

        let bytes = bincode::serialize(&set).unwrap();
        let restored: IntentSet = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_intent_params_bincode() {
        let params = IntentParams::new()
            .with_location(LocationId::new(5))
            .with_resource(ResourceTypeId::new("gold"), 100)
            .with_deadline(1000);

        let bytes = bincode::serialize(&params).unwrap();
        let restored: IntentParams = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.target_location, Some(LocationId::new(5)));
        assert_eq!(restored.deadline_tick, Some(1000));
    }
}

//! Goal definitions and identifiers.

use super::consideration::Consideration;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Stable identifier for a goal type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GoalId(pub String);

impl GoalId {
    pub const SATISFY_HUNGER: &'static str = "satisfy_hunger";
    pub const SEEK_WATER: &'static str = "seek_water";
    pub const SEEK_OXYGEN: &'static str = "seek_oxygen";
    pub const WARM_UP: &'static str = "warm_up";
    pub const COOL_DOWN: &'static str = "cool_down";
    pub const REST: &'static str = "rest";
    pub const FLEE_DANGER: &'static str = "flee_danger";
    pub const SEEK_ALLIES: &'static str = "seek_allies";
    pub const DEFEND_TERRITORY: &'static str = "defend_territory";
    pub const INVESTIGATE_STIMULUS: &'static str = "investigate_stimulus";
    pub const FOLLOW_LEADER: &'static str = "follow_leader";
    pub const PATROL: &'static str = "patrol";
    pub const IDLE: &'static str = "idle";

    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn satisfy_hunger() -> Self {
        Self::new(Self::SATISFY_HUNGER)
    }

    #[must_use]
    pub fn seek_water() -> Self {
        Self::new(Self::SEEK_WATER)
    }

    #[must_use]
    pub fn seek_oxygen() -> Self {
        Self::new(Self::SEEK_OXYGEN)
    }

    #[must_use]
    pub fn warm_up() -> Self {
        Self::new(Self::WARM_UP)
    }

    #[must_use]
    pub fn cool_down() -> Self {
        Self::new(Self::COOL_DOWN)
    }

    #[must_use]
    pub fn rest() -> Self {
        Self::new(Self::REST)
    }

    #[must_use]
    pub fn flee_danger() -> Self {
        Self::new(Self::FLEE_DANGER)
    }

    #[must_use]
    pub fn seek_allies() -> Self {
        Self::new(Self::SEEK_ALLIES)
    }

    #[must_use]
    pub fn defend_territory() -> Self {
        Self::new(Self::DEFEND_TERRITORY)
    }

    #[must_use]
    pub fn investigate_stimulus() -> Self {
        Self::new(Self::INVESTIGATE_STIMULUS)
    }

    #[must_use]
    pub fn follow_leader() -> Self {
        Self::new(Self::FOLLOW_LEADER)
    }

    #[must_use]
    pub fn patrol() -> Self {
        Self::new(Self::PATROL)
    }

    #[must_use]
    pub fn idle() -> Self {
        Self::new(Self::IDLE)
    }
}

impl<T: Into<String>> From<T> for GoalId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Tag for categorizing goals.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GoalTag(pub String);

impl GoalTag {
    pub const SURVIVAL: &'static str = "survival";
    pub const SOCIAL: &'static str = "social";
    pub const COMBAT: &'static str = "combat";
    pub const EXPLORATION: &'static str = "exploration";
    pub const WORK: &'static str = "work";
    pub const IDLE: &'static str = "idle";

    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    #[must_use]
    pub fn survival() -> Self {
        Self::new(Self::SURVIVAL)
    }

    #[must_use]
    pub fn social() -> Self {
        Self::new(Self::SOCIAL)
    }

    #[must_use]
    pub fn combat() -> Self {
        Self::new(Self::COMBAT)
    }

    #[must_use]
    pub fn exploration() -> Self {
        Self::new(Self::EXPLORATION)
    }

    #[must_use]
    pub fn work() -> Self {
        Self::new(Self::WORK)
    }

    #[must_use]
    pub fn idle() -> Self {
        Self::new(Self::IDLE)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for GoalTag {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Definition of a goal with considerations for scoring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalDef {
    /// Unique identifier for this goal.
    pub id: GoalId,
    /// Human-readable name.
    pub name: String,
    /// Base priority weight (multiplied with utility score).
    pub base_priority: f32,
    /// Minimum score required to consider this goal viable.
    pub min_threshold: f32,
    /// Tags for categorization.
    tags: BTreeSet<GoalTag>,
    /// Considerations that contribute to the final score.
    considerations: Vec<Consideration>,
    /// Whether this goal can be interrupted by higher-priority goals.
    pub interruptible: bool,
    /// Cooldown ticks after completion before re-selection.
    pub cooldown_ticks: u64,
    /// Minimum ticks this goal should run once selected.
    pub min_duration: u64,
    /// Whether this goal requires completion (vs. can be abandoned).
    pub requires_completion: bool,
}

impl GoalDef {
    /// Create a new goal definition.
    #[must_use]
    pub fn new(id: GoalId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            base_priority: 1.0,
            min_threshold: 0.0,
            tags: BTreeSet::new(),
            considerations: Vec::new(),
            interruptible: true,
            cooldown_ticks: 0,
            min_duration: 0,
            requires_completion: false,
        }
    }

    /// Set base priority.
    #[must_use]
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.base_priority = priority;
        self
    }

    /// Set minimum threshold.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.min_threshold = threshold;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: GoalTag) -> Self {
        self.tags.insert(tag);
        self
    }

    /// Add a consideration.
    #[must_use]
    pub fn with_consideration(mut self, consideration: Consideration) -> Self {
        self.considerations.push(consideration);
        self
    }

    /// Set interruptibility.
    #[must_use]
    pub fn with_interruptible(mut self, interruptible: bool) -> Self {
        self.interruptible = interruptible;
        self
    }

    /// Set cooldown ticks.
    #[must_use]
    pub fn with_cooldown(mut self, ticks: u64) -> Self {
        self.cooldown_ticks = ticks;
        self
    }

    /// Set minimum duration.
    #[must_use]
    pub fn with_min_duration(mut self, ticks: u64) -> Self {
        self.min_duration = ticks;
        self
    }

    /// Set requires completion.
    #[must_use]
    pub fn with_requires_completion(mut self, required: bool) -> Self {
        self.requires_completion = required;
        self
    }

    /// Check if has tag.
    #[must_use]
    pub fn has_tag(&self, tag: &GoalTag) -> bool {
        self.tags.contains(tag)
    }

    /// Get all tags.
    pub fn tags(&self) -> impl Iterator<Item = &GoalTag> {
        self.tags.iter()
    }

    /// Get all considerations.
    pub fn considerations(&self) -> &[Consideration] {
        &self.considerations
    }

    /// Add a consideration mutably.
    pub fn add_consideration(&mut self, consideration: Consideration) {
        self.considerations.push(consideration);
    }

    /// Add a tag mutably.
    pub fn add_tag(&mut self, tag: GoalTag) {
        self.tags.insert(tag);
    }

    /// Remove a tag.
    pub fn remove_tag(&mut self, tag: &GoalTag) -> bool {
        self.tags.remove(tag)
    }

    /// Get number of considerations.
    #[must_use]
    pub fn consideration_count(&self) -> usize {
        self.considerations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{InputBinding, UtilityCurve};

    #[test]
    fn test_goal_id_constants() {
        assert_eq!(GoalId::satisfy_hunger().as_str(), "satisfy_hunger");
        assert_eq!(GoalId::flee_danger().as_str(), "flee_danger");
        assert_eq!(GoalId::idle().as_str(), "idle");
    }

    #[test]
    fn test_goal_id_from() {
        let id: GoalId = "custom_goal".into();
        assert_eq!(id.as_str(), "custom_goal");
    }

    #[test]
    fn test_goal_id_ordering() {
        let a = GoalId::new("alpha");
        let b = GoalId::new("beta");
        assert!(a < b);
    }

    #[test]
    fn test_goal_tag_constants() {
        assert_eq!(GoalTag::survival().as_str(), "survival");
        assert_eq!(GoalTag::combat().as_str(), "combat");
    }

    #[test]
    fn test_goal_def_new() {
        let goal = GoalDef::new(GoalId::satisfy_hunger(), "Satisfy Hunger");

        assert_eq!(goal.id, GoalId::satisfy_hunger());
        assert_eq!(goal.name, "Satisfy Hunger");
        assert!((goal.base_priority - 1.0).abs() < f32::EPSILON);
        assert!(goal.interruptible);
    }

    #[test]
    fn test_goal_def_builder() {
        let goal = GoalDef::new(GoalId::flee_danger(), "Flee Danger")
            .with_priority(2.0)
            .with_threshold(0.3)
            .with_tag(GoalTag::survival())
            .with_tag(GoalTag::combat())
            .with_interruptible(false)
            .with_cooldown(100)
            .with_min_duration(50)
            .with_requires_completion(true);

        assert!((goal.base_priority - 2.0).abs() < f32::EPSILON);
        assert!((goal.min_threshold - 0.3).abs() < f32::EPSILON);
        assert!(goal.has_tag(&GoalTag::survival()));
        assert!(goal.has_tag(&GoalTag::combat()));
        assert!(!goal.interruptible);
        assert_eq!(goal.cooldown_ticks, 100);
        assert_eq!(goal.min_duration, 50);
        assert!(goal.requires_completion);
    }

    #[test]
    fn test_goal_def_considerations() {
        let c1 = Consideration::new("hunger")
            .with_input(InputBinding::NeedDeficit("hunger".into()))
            .with_curve(UtilityCurve::linear());

        let c2 = Consideration::new("safety")
            .with_input(InputBinding::ThreatLevel)
            .with_curve(UtilityCurve::inverse_linear());

        let goal = GoalDef::new(GoalId::satisfy_hunger(), "Satisfy Hunger")
            .with_consideration(c1)
            .with_consideration(c2);

        assert_eq!(goal.consideration_count(), 2);
    }

    #[test]
    fn test_goal_def_tag_operations() {
        let mut goal = GoalDef::new(GoalId::idle(), "Idle");

        goal.add_tag(GoalTag::idle());
        assert!(goal.has_tag(&GoalTag::idle()));

        assert!(goal.remove_tag(&GoalTag::idle()));
        assert!(!goal.has_tag(&GoalTag::idle()));
    }

    #[test]
    fn test_goal_def_serde() {
        let goal = GoalDef::new(GoalId::rest(), "Rest")
            .with_priority(1.5)
            .with_tag(GoalTag::survival())
            .with_cooldown(60);

        let json = serde_json::to_string(&goal).unwrap();
        let restored: GoalDef = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, goal.id);
        assert_eq!(restored.name, goal.name);
        assert!((restored.base_priority - goal.base_priority).abs() < f32::EPSILON);
        assert!(restored.has_tag(&GoalTag::survival()));
        assert_eq!(restored.cooldown_ticks, 60);
    }

    #[test]
    fn test_goal_tags_iteration() {
        let goal = GoalDef::new(GoalId::patrol(), "Patrol")
            .with_tag(GoalTag::exploration())
            .with_tag(GoalTag::work());

        let tags: Vec<_> = goal.tags().collect();
        assert_eq!(tags.len(), 2);
    }
}

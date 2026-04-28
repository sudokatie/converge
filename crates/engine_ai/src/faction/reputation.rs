//! Reputation model between actors and factions.

use super::FactionId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Tier classifications for reputation standing.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ReputationTier {
    /// Extreme hostility, will attack on sight.
    Hostile,
    /// Active dislike, restricted access.
    Wary,
    /// Default starting point.
    #[default]
    Neutral,
    /// Positive disposition, some benefits.
    Friendly,
    /// Strong positive relationship, significant benefits.
    Ally,
    /// Maximum trust, full access.
    Revered,
}

impl ReputationTier {
    /// Get the minimum standing value for this tier.
    #[must_use]
    pub fn min_standing(self) -> i32 {
        match self {
            Self::Hostile => i32::MIN,
            Self::Wary => -499,
            Self::Neutral => -99,
            Self::Friendly => 100,
            Self::Ally => 500,
            Self::Revered => 1000,
        }
    }

    /// Get the next tier up (if any).
    #[must_use]
    pub fn next_up(self) -> Option<Self> {
        match self {
            Self::Hostile => Some(Self::Wary),
            Self::Wary => Some(Self::Neutral),
            Self::Neutral => Some(Self::Friendly),
            Self::Friendly => Some(Self::Ally),
            Self::Ally => Some(Self::Revered),
            Self::Revered => None,
        }
    }

    /// Get the next tier down (if any).
    #[must_use]
    pub fn next_down(self) -> Option<Self> {
        match self {
            Self::Hostile => None,
            Self::Wary => Some(Self::Hostile),
            Self::Neutral => Some(Self::Wary),
            Self::Friendly => Some(Self::Neutral),
            Self::Ally => Some(Self::Friendly),
            Self::Revered => Some(Self::Ally),
        }
    }

    /// Classify a standing value into a tier.
    #[must_use]
    pub fn classify(standing: i32) -> Self {
        if standing >= Self::Revered.min_standing() {
            Self::Revered
        } else if standing >= Self::Ally.min_standing() {
            Self::Ally
        } else if standing >= Self::Friendly.min_standing() {
            Self::Friendly
        } else if standing >= Self::Neutral.min_standing() {
            Self::Neutral
        } else if standing >= Self::Wary.min_standing() {
            Self::Wary
        } else {
            Self::Hostile
        }
    }

    /// Check if this tier allows access/entry.
    #[must_use]
    pub fn allows_access(self) -> bool {
        matches!(
            self,
            Self::Neutral | Self::Friendly | Self::Ally | Self::Revered
        )
    }

    /// Check if this tier allows building.
    #[must_use]
    pub fn allows_building(self) -> bool {
        matches!(self, Self::Ally | Self::Revered)
    }

    /// Check if this tier allows harvesting.
    #[must_use]
    pub fn allows_harvesting(self) -> bool {
        matches!(self, Self::Friendly | Self::Ally | Self::Revered)
    }

    /// Check if hostile (will attack on sight).
    #[must_use]
    pub fn is_hostile(self) -> bool {
        self == Self::Hostile
    }

    /// Check if considered a threat.
    #[must_use]
    pub fn is_threat(self) -> bool {
        matches!(self, Self::Hostile | Self::Wary)
    }
}

/// Configuration for reputation behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Minimum standing value (floor).
    pub min_standing: i32,
    /// Maximum standing value (ceiling).
    pub max_standing: i32,
    /// Maximum change per single event.
    pub max_delta: i32,
    /// Decay rate per tick (toward neutral).
    pub decay_rate: f32,
    /// Standing value to decay toward.
    pub decay_target: i32,
    /// Maximum events to keep in history.
    pub max_history: usize,
}

impl ReputationConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a stricter config with slower reputation gain.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            min_standing: -2000,
            max_standing: 2000,
            max_delta: 50,
            decay_rate: 0.002,
            decay_target: 0,
            max_history: 50,
        }
    }

    /// Create a lenient config with faster reputation changes.
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            min_standing: -1000,
            max_standing: 1500,
            max_delta: 200,
            decay_rate: 0.0,
            decay_target: 0,
            max_history: 20,
        }
    }
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            min_standing: -1500,
            max_standing: 1500,
            max_delta: 100,
            decay_rate: 0.001,
            decay_target: 0,
            max_history: 100,
        }
    }
}

/// A reputation standing with a single faction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Standing {
    /// Current numeric standing value.
    value: i32,
    /// Current tier classification.
    tier: ReputationTier,
    /// Tick when last modified.
    last_change_tick: u64,
    /// Whether decay is enabled.
    decay_enabled: bool,
    /// Local multiplier for reputation changes.
    multiplier: f32,
}

impl Standing {
    /// Create a new standing at neutral.
    #[must_use]
    pub fn new() -> Self {
        Self::with_value(0)
    }

    /// Create with a specific value.
    #[must_use]
    pub fn with_value(value: i32) -> Self {
        Self {
            value,
            tier: ReputationTier::classify(value),
            last_change_tick: 0,
            decay_enabled: true,
            multiplier: 1.0,
        }
    }

    /// Get current value.
    #[must_use]
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Get current tier.
    #[must_use]
    pub fn tier(&self) -> ReputationTier {
        self.tier
    }

    /// Get tick of last change.
    #[must_use]
    pub fn last_change_tick(&self) -> u64 {
        self.last_change_tick
    }

    /// Check if decay is enabled.
    #[must_use]
    pub fn decay_enabled(&self) -> bool {
        self.decay_enabled
    }

    /// Set decay enabled.
    pub fn set_decay_enabled(&mut self, enabled: bool) {
        self.decay_enabled = enabled;
    }

    /// Get the local multiplier.
    #[must_use]
    pub fn multiplier(&self) -> f32 {
        self.multiplier
    }

    /// Set the local multiplier.
    pub fn set_multiplier(&mut self, multiplier: f32) {
        self.multiplier = multiplier.max(0.0);
    }

    /// Apply a delta, clamping to config bounds. Returns the tier transition if any.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "delta values bounded by config, precision loss acceptable"
    )]
    pub fn apply_delta(
        &mut self,
        delta: i32,
        tick: u64,
        config: &ReputationConfig,
    ) -> Option<(ReputationTier, ReputationTier)> {
        let effective_delta = ((delta as f32) * self.multiplier) as i32;
        let clamped_delta = effective_delta.clamp(-config.max_delta, config.max_delta);
        let new_value =
            (self.value + clamped_delta).clamp(config.min_standing, config.max_standing);

        let old_tier = self.tier;
        self.value = new_value;
        self.tier = ReputationTier::classify(new_value);
        self.last_change_tick = tick;

        if self.tier == old_tier {
            None
        } else {
            Some((old_tier, self.tier))
        }
    }

    /// Set value directly (bypasses clamping, use carefully).
    pub fn set_value(&mut self, value: i32, tick: u64, config: &ReputationConfig) {
        self.value = value.clamp(config.min_standing, config.max_standing);
        self.tier = ReputationTier::classify(self.value);
        self.last_change_tick = tick;
    }

    /// Apply decay toward target.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "decay values bounded, precision loss acceptable"
    )]
    pub fn apply_decay(&mut self, config: &ReputationConfig) {
        if !self.decay_enabled || config.decay_rate <= 0.0 {
            return;
        }

        let diff = config.decay_target - self.value;
        if diff == 0 {
            return;
        }

        let decay_amount = ((diff.abs() as f32) * config.decay_rate).max(1.0) as i32;
        let clamped = decay_amount.min(diff.abs());

        if diff > 0 {
            self.value += clamped;
        } else {
            self.value -= clamped;
        }

        self.tier = ReputationTier::classify(self.value);
    }

    /// Get normalized value (-1.0 to 1.0 based on config bounds).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "standing values bounded")]
    pub fn normalized(&self, config: &ReputationConfig) -> f32 {
        if self.value >= 0 {
            (self.value as f32) / (config.max_standing as f32)
        } else {
            (self.value as f32) / (config.min_standing.abs() as f32)
        }
    }

    /// Get progress toward next tier (0.0 to 1.0).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "standing values bounded")]
    pub fn progress_to_next(&self) -> f32 {
        let current_min = self.tier.min_standing();
        let next_min = self
            .tier
            .next_up()
            .map_or(self.value, ReputationTier::min_standing);

        if next_min == current_min {
            return 1.0;
        }

        let range = next_min - current_min;
        let progress = self.value - current_min;

        (progress as f32 / range as f32).clamp(0.0, 1.0)
    }
}

impl Default for Standing {
    fn default() -> Self {
        Self::new()
    }
}

/// A reputation change event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReputationEvent {
    /// Delta applied.
    pub delta: i32,
    /// Reason/source of the change.
    pub reason: String,
    /// Tick when occurred.
    pub tick: u64,
    /// Old tier (if transition happened).
    pub old_tier: Option<ReputationTier>,
    /// New tier (if transition happened).
    pub new_tier: Option<ReputationTier>,
}

impl ReputationEvent {
    #[must_use]
    pub fn new(delta: i32, reason: impl Into<String>, tick: u64) -> Self {
        Self {
            delta,
            reason: reason.into(),
            tick,
            old_tier: None,
            new_tier: None,
        }
    }

    #[must_use]
    pub fn with_transition(mut self, old: ReputationTier, new: ReputationTier) -> Self {
        self.old_tier = Some(old);
        self.new_tier = Some(new);
        self
    }

    #[must_use]
    pub fn has_transition(&self) -> bool {
        self.old_tier.is_some() && self.new_tier.is_some()
    }
}

/// History of reputation changes with summarization.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReputationHistory {
    events: Vec<ReputationEvent>,
    total_positive: i32,
    total_negative: i32,
    transition_count: u32,
}

impl ReputationHistory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to history.
    pub fn add(&mut self, event: ReputationEvent, max_events: usize) {
        if event.delta > 0 {
            self.total_positive += event.delta;
        } else {
            self.total_negative += event.delta.abs();
        }

        if event.has_transition() {
            self.transition_count += 1;
        }

        self.events.push(event);

        while self.events.len() > max_events {
            self.events.remove(0);
        }
    }

    /// Get total positive reputation gained.
    #[must_use]
    pub fn total_positive(&self) -> i32 {
        self.total_positive
    }

    /// Get total negative reputation lost.
    #[must_use]
    pub fn total_negative(&self) -> i32 {
        self.total_negative
    }

    /// Get number of tier transitions.
    #[must_use]
    pub fn transition_count(&self) -> u32 {
        self.transition_count
    }

    /// Get recent events.
    pub fn events(&self) -> &[ReputationEvent] {
        &self.events
    }

    /// Get event count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear history (keeps summary).
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

/// A structured reputation delta for application.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReputationDelta {
    /// Amount to change.
    pub amount: i32,
    /// Reason for the change.
    pub reason: String,
    /// Optional tag for filtering/analysis.
    pub tag: Option<String>,
}

impl ReputationDelta {
    #[must_use]
    pub fn new(amount: i32, reason: impl Into<String>) -> Self {
        Self {
            amount,
            reason: reason.into(),
            tag: None,
        }
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    #[must_use]
    pub fn positive(amount: i32, reason: impl Into<String>) -> Self {
        Self::new(amount.abs(), reason)
    }

    #[must_use]
    pub fn negative(amount: i32, reason: impl Into<String>) -> Self {
        Self::new(-amount.abs(), reason)
    }
}

/// Priority entry for sorting reputations.
#[derive(Clone, Debug)]
pub struct StandingPriority {
    pub faction_id: FactionId,
    pub value: i32,
    pub tier: ReputationTier,
}

impl PartialEq for StandingPriority {
    fn eq(&self, other: &Self) -> bool {
        self.faction_id == other.faction_id
    }
}

impl Eq for StandingPriority {}

impl PartialOrd for StandingPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StandingPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .value
            .cmp(&self.value)
            .then_with(|| self.faction_id.cmp(&other.faction_id))
    }
}

/// Collection of reputations with multiple factions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReputationSet {
    standings: BTreeMap<FactionId, Standing>,
    histories: BTreeMap<FactionId, ReputationHistory>,
    config: ReputationConfig,
    current_tick: u64,
}

impl ReputationSet {
    /// Create with default config.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ReputationConfig::default())
    }

    /// Create with specific config.
    #[must_use]
    pub fn with_config(config: ReputationConfig) -> Self {
        Self {
            standings: BTreeMap::new(),
            histories: BTreeMap::new(),
            config,
            current_tick: 0,
        }
    }

    /// Get the config.
    #[must_use]
    pub fn config(&self) -> &ReputationConfig {
        &self.config
    }

    /// Get standing with a faction, creating neutral if not present.
    pub fn get_or_create(&mut self, faction: &FactionId) -> &Standing {
        self.standings.entry(faction.clone()).or_default()
    }

    /// Get standing with a faction (if exists).
    #[must_use]
    pub fn get(&self, faction: &FactionId) -> Option<&Standing> {
        self.standings.get(faction)
    }

    /// Get mutable standing.
    pub fn get_mut(&mut self, faction: &FactionId) -> Option<&mut Standing> {
        self.standings.get_mut(faction)
    }

    /// Get history for a faction.
    #[must_use]
    pub fn history(&self, faction: &FactionId) -> Option<&ReputationHistory> {
        self.histories.get(faction)
    }

    /// Apply a reputation delta, recording history.
    pub fn apply_delta(
        &mut self,
        faction: &FactionId,
        delta: ReputationDelta,
        tick: u64,
    ) -> Option<ReputationEvent> {
        let standing = self.standings.entry(faction.clone()).or_default();

        let transition = standing.apply_delta(delta.amount, tick, &self.config);

        let mut event = ReputationEvent::new(delta.amount, delta.reason, tick);
        if let Some((old, new)) = transition {
            event = event.with_transition(old, new);
        }

        let history = self.histories.entry(faction.clone()).or_default();
        history.add(event.clone(), self.config.max_history);

        Some(event)
    }

    /// Set standing directly for a faction.
    pub fn set_standing(&mut self, faction: &FactionId, value: i32, tick: u64) {
        let standing = self.standings.entry(faction.clone()).or_default();
        standing.set_value(value, tick, &self.config);
    }

    /// Get tier with a faction.
    #[must_use]
    pub fn tier(&self, faction: &FactionId) -> ReputationTier {
        self.standings
            .get(faction)
            .map_or(ReputationTier::Neutral, Standing::tier)
    }

    /// Get value with a faction.
    #[must_use]
    pub fn value(&self, faction: &FactionId) -> i32 {
        self.standings.get(faction).map_or(0, Standing::value)
    }

    /// Check if hostile with faction.
    #[must_use]
    pub fn is_hostile(&self, faction: &FactionId) -> bool {
        self.tier(faction).is_hostile()
    }

    /// Check if considered a threat by faction.
    #[must_use]
    pub fn is_threat(&self, faction: &FactionId) -> bool {
        self.tier(faction).is_threat()
    }

    /// Check if access is allowed by faction.
    #[must_use]
    pub fn allows_access(&self, faction: &FactionId) -> bool {
        self.tier(faction).allows_access()
    }

    /// Check if building is allowed.
    #[must_use]
    pub fn allows_building(&self, faction: &FactionId) -> bool {
        self.tier(faction).allows_building()
    }

    /// Check if harvesting is allowed.
    #[must_use]
    pub fn allows_harvesting(&self, faction: &FactionId) -> bool {
        self.tier(faction).allows_harvesting()
    }

    /// Get number of tracked factions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.standings.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.standings.is_empty()
    }

    /// Iterate over all standings.
    pub fn iter(&self) -> impl Iterator<Item = (&FactionId, &Standing)> {
        self.standings.iter()
    }

    /// Get factions in a specific tier.
    pub fn in_tier(&self, tier: ReputationTier) -> impl Iterator<Item = &FactionId> {
        self.standings
            .iter()
            .filter(move |(_, s)| s.tier() == tier)
            .map(|(id, _)| id)
    }

    /// Get standings sorted by value (highest first, deterministic).
    #[must_use]
    pub fn sorted(&self) -> Vec<StandingPriority> {
        let mut priorities: Vec<_> = self
            .standings
            .iter()
            .map(|(id, s)| StandingPriority {
                faction_id: id.clone(),
                value: s.value(),
                tier: s.tier(),
            })
            .collect();
        priorities.sort();
        priorities
    }

    /// Get the faction with highest standing.
    #[must_use]
    pub fn highest(&self) -> Option<FactionId> {
        self.sorted().into_iter().next().map(|p| p.faction_id)
    }

    /// Get the faction with lowest standing.
    #[must_use]
    pub fn lowest(&self) -> Option<FactionId> {
        self.sorted().into_iter().last().map(|p| p.faction_id)
    }

    /// Tick decay for all standings.
    pub fn tick(&mut self) {
        self.current_tick += 1;
        for standing in self.standings.values_mut() {
            standing.apply_decay(&self.config);
        }
    }

    /// Advance to a specific tick.
    pub fn advance_to(&mut self, tick: u64) {
        while self.current_tick < tick {
            self.tick();
        }
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Remove a faction from tracking.
    pub fn remove(&mut self, faction: &FactionId) -> Option<Standing> {
        self.histories.remove(faction);
        self.standings.remove(faction)
    }

    /// Clear all standings.
    pub fn clear(&mut self) {
        self.standings.clear();
        self.histories.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_tier_classify() {
        assert_eq!(ReputationTier::classify(1500), ReputationTier::Revered);
        assert_eq!(ReputationTier::classify(1000), ReputationTier::Revered);
        assert_eq!(ReputationTier::classify(500), ReputationTier::Ally);
        assert_eq!(ReputationTier::classify(100), ReputationTier::Friendly);
        assert_eq!(ReputationTier::classify(0), ReputationTier::Neutral);
        assert_eq!(ReputationTier::classify(-100), ReputationTier::Wary);
        assert_eq!(ReputationTier::classify(-500), ReputationTier::Hostile);
        assert_eq!(ReputationTier::classify(-1000), ReputationTier::Hostile);
    }

    #[test]
    fn test_reputation_tier_permissions() {
        assert!(!ReputationTier::Hostile.allows_access());
        assert!(!ReputationTier::Wary.allows_access());
        assert!(ReputationTier::Neutral.allows_access());
        assert!(ReputationTier::Friendly.allows_access());
        assert!(ReputationTier::Ally.allows_access());
        assert!(ReputationTier::Revered.allows_access());

        assert!(!ReputationTier::Friendly.allows_building());
        assert!(ReputationTier::Ally.allows_building());

        assert!(ReputationTier::Friendly.allows_harvesting());
        assert!(!ReputationTier::Neutral.allows_harvesting());
    }

    #[test]
    fn test_reputation_tier_navigation() {
        assert_eq!(
            ReputationTier::Neutral.next_up(),
            Some(ReputationTier::Friendly)
        );
        assert_eq!(
            ReputationTier::Neutral.next_down(),
            Some(ReputationTier::Wary)
        );
        assert_eq!(ReputationTier::Revered.next_up(), None);
        assert_eq!(ReputationTier::Hostile.next_down(), None);
    }

    #[test]
    fn test_standing_new() {
        let s = Standing::new();
        assert_eq!(s.value(), 0);
        assert_eq!(s.tier(), ReputationTier::Neutral);
    }

    #[test]
    fn test_standing_with_value() {
        let s = Standing::with_value(600);
        assert_eq!(s.value(), 600);
        assert_eq!(s.tier(), ReputationTier::Ally);
    }

    #[test]
    fn test_standing_apply_delta() {
        let mut s = Standing::new();
        let config = ReputationConfig::default();

        let transition = s.apply_delta(150, 1, &config);
        assert_eq!(s.value(), 100);
        assert_eq!(s.tier(), ReputationTier::Friendly);
        assert!(transition.is_some());
        let (old, new) = transition.unwrap();
        assert_eq!(old, ReputationTier::Neutral);
        assert_eq!(new, ReputationTier::Friendly);
    }

    #[test]
    fn test_standing_clamping() {
        let mut s = Standing::with_value(1400);
        let config = ReputationConfig::default();

        s.apply_delta(500, 1, &config);
        assert_eq!(s.value(), 1500);

        s.apply_delta(100, 2, &config);
        assert_eq!(s.value(), 1500);
    }

    #[test]
    fn test_standing_multiplier() {
        let mut s = Standing::new();
        let config = ReputationConfig::default();

        s.set_multiplier(2.0);
        s.apply_delta(50, 1, &config);
        assert_eq!(s.value(), 100);
    }

    #[test]
    fn test_standing_decay() {
        let mut s = Standing::with_value(500);
        let config = ReputationConfig {
            decay_rate: 0.1,
            decay_target: 0,
            ..Default::default()
        };

        s.apply_decay(&config);
        assert!(s.value() < 500);
    }

    #[test]
    fn test_standing_decay_disabled() {
        let mut s = Standing::with_value(500);
        let config = ReputationConfig {
            decay_rate: 0.1,
            decay_target: 0,
            ..Default::default()
        };

        s.set_decay_enabled(false);
        s.apply_decay(&config);
        assert_eq!(s.value(), 500);
    }

    #[test]
    fn test_standing_progress() {
        let s = Standing::with_value(300);
        let progress = s.progress_to_next();
        assert!(progress > 0.0 && progress < 1.0);
    }

    #[test]
    fn test_reputation_event() {
        let event = ReputationEvent::new(50, "helped villager", 100);
        assert_eq!(event.delta, 50);
        assert!(!event.has_transition());

        let event = event.with_transition(ReputationTier::Neutral, ReputationTier::Friendly);
        assert!(event.has_transition());
    }

    #[test]
    fn test_reputation_history() {
        let mut history = ReputationHistory::new();

        history.add(ReputationEvent::new(50, "good deed", 1), 10);
        history.add(ReputationEvent::new(-30, "bad deed", 2), 10);

        assert_eq!(history.total_positive(), 50);
        assert_eq!(history.total_negative(), 30);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_reputation_history_pruning() {
        let mut history = ReputationHistory::new();

        for i in 0..20 {
            history.add(ReputationEvent::new(10, format!("event {i}"), i), 5);
        }

        assert_eq!(history.len(), 5);
        assert_eq!(history.total_positive(), 200);
    }

    #[test]
    fn test_reputation_delta() {
        let d = ReputationDelta::positive(50, "helped");
        assert_eq!(d.amount, 50);

        let d = ReputationDelta::negative(30, "attacked");
        assert_eq!(d.amount, -30);

        let d = ReputationDelta::new(25, "quest").with_tag("quest_reward");
        assert_eq!(d.tag.as_deref(), Some("quest_reward"));
    }

    #[test]
    fn test_reputation_set_basic() {
        let mut set = ReputationSet::new();
        let faction = FactionId::new("miners");

        set.apply_delta(&faction, ReputationDelta::positive(150, "helped"), 1);

        assert_eq!(set.tier(&faction), ReputationTier::Friendly);
        assert_eq!(set.value(&faction), 100);
        assert!(set.allows_access(&faction));
    }

    #[test]
    fn test_reputation_set_multiple_factions() {
        let mut set = ReputationSet::new();

        set.apply_delta(
            &FactionId::new("a"),
            ReputationDelta::positive(50, "quest"),
            1,
        );
        set.apply_delta(
            &FactionId::new("b"),
            ReputationDelta::negative(100, "attacked"),
            1,
        );
        set.apply_delta(
            &FactionId::new("c"),
            ReputationDelta::positive(100, "alliance"),
            1,
        );

        assert_eq!(set.len(), 3);

        let sorted = set.sorted();
        assert_eq!(sorted[0].faction_id.as_str(), "c");
        assert_eq!(sorted[1].faction_id.as_str(), "a");
        assert_eq!(sorted[2].faction_id.as_str(), "b");
    }

    #[test]
    fn test_reputation_set_tier_queries() {
        let mut set = ReputationSet::new();

        set.set_standing(&FactionId::new("friend"), 200, 0);
        set.set_standing(&FactionId::new("enemy"), -600, 0);
        set.set_standing(&FactionId::new("neutral"), 0, 0);

        let friendly: Vec<_> = set.in_tier(ReputationTier::Friendly).collect();
        assert_eq!(friendly.len(), 1);

        assert!(set.is_hostile(&FactionId::new("enemy")));
        assert!(!set.is_hostile(&FactionId::new("friend")));
    }

    #[test]
    fn test_reputation_set_tick_decay() {
        let config = ReputationConfig {
            decay_rate: 0.1,
            decay_target: 0,
            ..Default::default()
        };
        let mut set = ReputationSet::with_config(config);

        set.set_standing(&FactionId::new("test"), 500, 0);
        let initial = set.value(&FactionId::new("test"));

        set.tick();
        let after = set.value(&FactionId::new("test"));

        assert!(after < initial);
    }

    #[test]
    fn test_reputation_set_history() {
        let mut set = ReputationSet::new();
        let faction = FactionId::new("test");

        set.apply_delta(&faction, ReputationDelta::positive(50, "deed 1"), 1);
        set.apply_delta(&faction, ReputationDelta::positive(30, "deed 2"), 2);

        let history = set.history(&faction).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history.total_positive(), 80);
    }

    #[test]
    fn test_reputation_set_deterministic_ordering() {
        let mut set = ReputationSet::new();

        set.set_standing(&FactionId::new("z"), 100, 0);
        set.set_standing(&FactionId::new("a"), 100, 0);
        set.set_standing(&FactionId::new("m"), 100, 0);

        let sorted = set.sorted();
        assert_eq!(sorted[0].faction_id.as_str(), "a");
        assert_eq!(sorted[1].faction_id.as_str(), "m");
        assert_eq!(sorted[2].faction_id.as_str(), "z");
    }

    #[test]
    fn test_standing_serde() {
        let mut s = Standing::with_value(500);
        s.set_multiplier(1.5);
        s.set_decay_enabled(false);

        let json = serde_json::to_string(&s).unwrap();
        let restored: Standing = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.value(), 500);
        assert!((restored.multiplier() - 1.5).abs() < f32::EPSILON);
        assert!(!restored.decay_enabled());
    }

    #[test]
    fn test_reputation_set_serde() {
        let mut set = ReputationSet::new();
        set.apply_delta(
            &FactionId::new("faction_a"),
            ReputationDelta::positive(200, "helped"),
            100,
        );
        set.apply_delta(
            &FactionId::new("faction_b"),
            ReputationDelta::negative(100, "attacked"),
            101,
        );

        let json = serde_json::to_string(&set).unwrap();
        let restored: ReputationSet = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert_eq!(
            restored.tier(&FactionId::new("faction_a")),
            ReputationTier::Friendly
        );
    }

    #[test]
    fn test_reputation_config_presets() {
        let strict = ReputationConfig::strict();
        let lenient = ReputationConfig::lenient();

        assert!(strict.max_delta < lenient.max_delta);
        assert!(strict.decay_rate > lenient.decay_rate);
    }
}

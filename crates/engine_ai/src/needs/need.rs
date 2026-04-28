//! Core need representation and need sets.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Identifier for a need type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NeedId(pub String);

impl NeedId {
    pub const HUNGER: &'static str = "hunger";
    pub const THIRST: &'static str = "thirst";
    pub const OXYGEN: &'static str = "oxygen";
    pub const WARMTH: &'static str = "warmth";
    pub const REST: &'static str = "rest";
    pub const MORALE: &'static str = "morale";
    pub const SAFETY: &'static str = "safety";
    pub const SOCIAL: &'static str = "social";

    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn hunger() -> Self {
        Self::new(Self::HUNGER)
    }

    #[must_use]
    pub fn thirst() -> Self {
        Self::new(Self::THIRST)
    }

    #[must_use]
    pub fn oxygen() -> Self {
        Self::new(Self::OXYGEN)
    }

    #[must_use]
    pub fn warmth() -> Self {
        Self::new(Self::WARMTH)
    }

    #[must_use]
    pub fn rest() -> Self {
        Self::new(Self::REST)
    }

    #[must_use]
    pub fn morale() -> Self {
        Self::new(Self::MORALE)
    }

    #[must_use]
    pub fn safety() -> Self {
        Self::new(Self::SAFETY)
    }

    #[must_use]
    pub fn social() -> Self {
        Self::new(Self::SOCIAL)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for NeedId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// State classification of a need based on current value and thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NeedState {
    /// Need is satisfied (above high threshold).
    Satisfied,
    /// Need is normal (between low and high thresholds).
    Normal,
    /// Need is low (below low threshold but above critical).
    Low,
    /// Need is critical (at or below critical threshold).
    Critical,
}

impl NeedState {
    /// Returns the urgency weight for priority scoring (higher = more urgent).
    #[must_use]
    pub fn urgency_weight(self) -> u32 {
        match self {
            Self::Satisfied => 0,
            Self::Normal => 1,
            Self::Low => 3,
            Self::Critical => 10,
        }
    }
}

/// Kind of threshold for event generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThresholdKind {
    /// Crossed above high threshold (became satisfied).
    High,
    /// Crossed below low threshold (became low).
    Low,
    /// Crossed below critical threshold (became critical).
    Critical,
}

/// A threshold configuration for a need.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Threshold {
    /// High threshold (above = satisfied).
    pub high: f32,
    /// Low threshold (below = low state).
    pub low: f32,
    /// Critical threshold (below = critical state).
    pub critical: f32,
}

impl Threshold {
    #[must_use]
    pub fn new(critical: f32, low: f32, high: f32) -> Self {
        Self {
            high,
            low,
            critical,
        }
    }

    /// Classify a value into a [`NeedState`].
    #[must_use]
    pub fn classify(&self, value: f32) -> NeedState {
        if value <= self.critical {
            NeedState::Critical
        } else if value <= self.low {
            NeedState::Low
        } else if value >= self.high {
            NeedState::Satisfied
        } else {
            NeedState::Normal
        }
    }
}

impl Default for Threshold {
    fn default() -> Self {
        Self {
            high: 80.0,
            low: 30.0,
            critical: 10.0,
        }
    }
}

/// Event generated when a need crosses a threshold.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeedEvent {
    /// Which need generated this event.
    pub need_id: NeedId,
    /// Previous state before crossing.
    pub previous_state: NeedState,
    /// New state after crossing.
    pub new_state: NeedState,
    /// Current value when event fired.
    pub value: f32,
    /// Tick when event occurred.
    pub tick: u64,
}

/// A single need with value, bounds, and decay/recovery rates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Need {
    /// Unique identifier for this need type.
    pub id: NeedId,
    /// Current value (0.0 to max).
    value: f32,
    /// Maximum value.
    max: f32,
    /// Base decay rate per tick (subtracted each tick).
    decay_rate: f32,
    /// Base recovery rate per tick when recovering (added each tick).
    recovery_rate: f32,
    /// Whether this need is currently in recovery mode.
    recovering: bool,
    /// Thresholds for state classification.
    thresholds: Threshold,
    /// Cached current state.
    state: NeedState,
    /// Priority weight for urgency scoring (base multiplier).
    priority_weight: f32,
}

impl Need {
    /// Create a new need with the given parameters.
    #[must_use]
    pub fn new(id: NeedId, max: f32, decay_rate: f32) -> Self {
        let thresholds = Threshold::default();
        let state = thresholds.classify(max);
        Self {
            id,
            value: max,
            max,
            decay_rate,
            recovery_rate: decay_rate * 2.0,
            recovering: false,
            thresholds,
            state,
            priority_weight: 1.0,
        }
    }

    /// Create a need with full configuration.
    #[must_use]
    pub fn with_config(
        id: NeedId,
        value: f32,
        max: f32,
        decay_rate: f32,
        recovery_rate: f32,
        thresholds: Threshold,
        priority_weight: f32,
    ) -> Self {
        let clamped_value = value.clamp(0.0, max);
        let state = thresholds.classify(clamped_value);
        Self {
            id,
            value: clamped_value,
            max,
            decay_rate,
            recovery_rate,
            recovering: false,
            thresholds,
            state,
            priority_weight,
        }
    }

    /// Get the current value.
    #[must_use]
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Get the maximum value.
    #[must_use]
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Get the current state.
    #[must_use]
    pub fn state(&self) -> NeedState {
        self.state
    }

    /// Get the normalized value (0.0 to 1.0).
    #[must_use]
    pub fn normalized(&self) -> f32 {
        if self.max > 0.0 {
            self.value / self.max
        } else {
            0.0
        }
    }

    /// Get the current decay rate.
    #[must_use]
    pub fn decay_rate(&self) -> f32 {
        self.decay_rate
    }

    /// Get the current recovery rate.
    #[must_use]
    pub fn recovery_rate(&self) -> f32 {
        self.recovery_rate
    }

    /// Check if the need is in recovery mode.
    #[must_use]
    pub fn is_recovering(&self) -> bool {
        self.recovering
    }

    /// Calculate urgency score (higher = more urgent).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "urgency_weight max is 10, no precision loss"
    )]
    pub fn urgency(&self) -> f32 {
        let deficit = 1.0 - self.normalized();
        let state_weight = self.state.urgency_weight() as f32;
        deficit * state_weight * self.priority_weight
    }

    /// Set the value directly (clamped to bounds).
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(0.0, self.max);
        self.state = self.thresholds.classify(self.value);
    }

    /// Apply a delta to the value (clamped to bounds).
    pub fn apply_delta(&mut self, delta: f32) {
        self.set_value(self.value + delta);
    }

    /// Set recovery mode.
    pub fn set_recovering(&mut self, recovering: bool) {
        self.recovering = recovering;
    }

    /// Update decay rate (e.g., from status effects).
    pub fn set_decay_rate(&mut self, rate: f32) {
        self.decay_rate = rate.max(0.0);
    }

    /// Update recovery rate.
    pub fn set_recovery_rate(&mut self, rate: f32) {
        self.recovery_rate = rate.max(0.0);
    }

    /// Tick the need forward, returning an event if a threshold was crossed.
    pub fn tick(&mut self, tick_number: u64, decay_multiplier: f32) -> Option<NeedEvent> {
        let previous_state = self.state;

        let delta = if self.recovering {
            self.recovery_rate * decay_multiplier
        } else {
            -self.decay_rate * decay_multiplier
        };

        self.value = (self.value + delta).clamp(0.0, self.max);
        self.state = self.thresholds.classify(self.value);

        if self.state == previous_state {
            None
        } else {
            Some(NeedEvent {
                need_id: self.id.clone(),
                previous_state,
                new_state: self.state,
                value: self.value,
                tick: tick_number,
            })
        }
    }
}

/// Entry for priority queue ordering.
#[derive(Clone, Debug)]
pub struct NeedPriority {
    pub id: NeedId,
    pub urgency: f32,
}

impl PartialEq for NeedPriority {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for NeedPriority {}

impl PartialOrd for NeedPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NeedPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.urgency.partial_cmp(&self.urgency) {
            Some(Ordering::Equal) | None => self.id.cmp(&other.id),
            Some(ord) => ord,
        }
    }
}

/// A collection of needs for a single creature.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NeedSet {
    needs: BTreeMap<NeedId, Need>,
    current_tick: u64,
}

impl NeedSet {
    /// Create a new empty need set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from an iterator of needs.
    pub fn from_needs(needs: impl IntoIterator<Item = Need>) -> Self {
        let mut set = Self::new();
        for need in needs {
            set.add(need);
        }
        set
    }

    /// Add a need to the set.
    pub fn add(&mut self, need: Need) {
        self.needs.insert(need.id.clone(), need);
    }

    /// Remove a need from the set.
    pub fn remove(&mut self, id: &NeedId) -> Option<Need> {
        self.needs.remove(id)
    }

    /// Get a need by ID.
    #[must_use]
    pub fn get(&self, id: &NeedId) -> Option<&Need> {
        self.needs.get(id)
    }

    /// Get a mutable need by ID.
    pub fn get_mut(&mut self, id: &NeedId) -> Option<&mut Need> {
        self.needs.get_mut(id)
    }

    /// Check if a need exists.
    #[must_use]
    pub fn contains(&self, id: &NeedId) -> bool {
        self.needs.contains_key(id)
    }

    /// Get the number of needs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.needs.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.needs.is_empty()
    }

    /// Iterate over all needs.
    pub fn iter(&self) -> impl Iterator<Item = &Need> {
        self.needs.values()
    }

    /// Iterate over all needs mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Need> {
        self.needs.values_mut()
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Tick all needs forward, collecting generated events.
    pub fn tick(&mut self, decay_multiplier: f32) -> Vec<NeedEvent> {
        self.current_tick += 1;
        let tick = self.current_tick;
        self.needs
            .values_mut()
            .filter_map(|need| need.tick(tick, decay_multiplier))
            .collect()
    }

    /// Get needs sorted by urgency (most urgent first) with deterministic ordering.
    #[must_use]
    pub fn priorities(&self) -> Vec<NeedPriority> {
        let mut priorities: Vec<_> = self
            .needs
            .values()
            .map(|n| NeedPriority {
                id: n.id.clone(),
                urgency: n.urgency(),
            })
            .collect();
        priorities.sort();
        priorities
    }

    /// Get the most urgent need.
    #[must_use]
    pub fn most_urgent(&self) -> Option<&Need> {
        self.priorities()
            .first()
            .and_then(|p| self.needs.get(&p.id))
    }

    /// Get all needs in a specific state.
    pub fn in_state(&self, state: NeedState) -> impl Iterator<Item = &Need> {
        self.needs.values().filter(move |n| n.state() == state)
    }

    /// Count needs in each state.
    #[must_use]
    pub fn state_counts(&self) -> [usize; 4] {
        let mut counts = [0usize; 4];
        for need in self.needs.values() {
            let idx = match need.state() {
                NeedState::Satisfied => 0,
                NeedState::Normal => 1,
                NeedState::Low => 2,
                NeedState::Critical => 3,
            };
            counts[idx] += 1;
        }
        counts
    }

    /// Check if any need is critical.
    #[must_use]
    pub fn has_critical(&self) -> bool {
        self.needs
            .values()
            .any(|n| n.state() == NeedState::Critical)
    }

    /// Check if any need is low or critical.
    #[must_use]
    pub fn has_low_or_critical(&self) -> bool {
        self.needs.values().any(|n| {
            let s = n.state();
            s == NeedState::Low || s == NeedState::Critical
        })
    }

    /// Calculate total urgency score.
    #[must_use]
    pub fn total_urgency(&self) -> f32 {
        self.needs.values().map(Need::urgency).sum()
    }

    /// Apply a delta to a specific need.
    pub fn apply_delta(&mut self, id: &NeedId, delta: f32) -> bool {
        if let Some(need) = self.needs.get_mut(id) {
            need.apply_delta(delta);
            true
        } else {
            false
        }
    }

    /// Set recovery mode for a specific need.
    pub fn set_recovering(&mut self, id: &NeedId, recovering: bool) -> bool {
        if let Some(need) = self.needs.get_mut(id) {
            need.set_recovering(recovering);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_need_id_constants() {
        assert_eq!(NeedId::hunger().as_str(), "hunger");
        assert_eq!(NeedId::thirst().as_str(), "thirst");
        assert_eq!(NeedId::oxygen().as_str(), "oxygen");
    }

    #[test]
    fn test_threshold_classify() {
        let t = Threshold::new(10.0, 30.0, 80.0);

        assert_eq!(t.classify(100.0), NeedState::Satisfied);
        assert_eq!(t.classify(80.0), NeedState::Satisfied);
        assert_eq!(t.classify(50.0), NeedState::Normal);
        assert_eq!(t.classify(30.0), NeedState::Low);
        assert_eq!(t.classify(20.0), NeedState::Low);
        assert_eq!(t.classify(10.0), NeedState::Critical);
        assert_eq!(t.classify(0.0), NeedState::Critical);
    }

    #[test]
    fn test_need_new() {
        let need = Need::new(NeedId::hunger(), 100.0, 1.0);

        assert!((need.value() - 100.0).abs() < f32::EPSILON);
        assert!((need.max() - 100.0).abs() < f32::EPSILON);
        assert_eq!(need.state(), NeedState::Satisfied);
        assert!((need.normalized() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_tick_decay() {
        let mut need = Need::new(NeedId::hunger(), 100.0, 5.0);
        need.set_value(50.0);

        need.tick(1, 1.0);

        assert!((need.value() - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_tick_recovery() {
        let mut need = Need::new(NeedId::hunger(), 100.0, 5.0);
        need.set_value(50.0);
        need.set_recovering(true);

        need.tick(1, 1.0);

        assert!((need.value() - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_tick_generates_event() {
        let mut need = Need::with_config(
            NeedId::hunger(),
            31.0,
            100.0,
            5.0,
            10.0,
            Threshold::new(10.0, 30.0, 80.0),
            1.0,
        );

        let event = need.tick(1, 1.0);

        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.previous_state, NeedState::Normal);
        assert_eq!(event.new_state, NeedState::Low);
        assert_eq!(event.tick, 1);
    }

    #[test]
    fn test_need_clamping() {
        let mut need = Need::new(NeedId::hunger(), 100.0, 1.0);

        need.apply_delta(50.0);
        assert!((need.value() - 100.0).abs() < f32::EPSILON);

        need.apply_delta(-200.0);
        assert!((need.value()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_urgency() {
        let mut need = Need::new(NeedId::hunger(), 100.0, 1.0);

        need.set_value(100.0);
        assert!((need.urgency()).abs() < f32::EPSILON);

        need.set_value(5.0);
        assert!(need.urgency() > 5.0);
    }

    #[test]
    fn test_need_set_basic() {
        let mut set = NeedSet::new();
        set.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        set.add(Need::new(NeedId::thirst(), 100.0, 2.0));

        assert_eq!(set.len(), 2);
        assert!(set.contains(&NeedId::hunger()));
        assert!(set.contains(&NeedId::thirst()));
        assert!(!set.contains(&NeedId::oxygen()));
    }

    #[test]
    fn test_need_set_tick() {
        let mut set = NeedSet::new();
        set.add(Need::new(NeedId::hunger(), 100.0, 5.0));
        set.add(Need::new(NeedId::thirst(), 100.0, 10.0));

        set.tick(1.0);

        let hunger = set.get(&NeedId::hunger()).unwrap();
        let thirst = set.get(&NeedId::thirst()).unwrap();

        assert!((hunger.value() - 95.0).abs() < f32::EPSILON);
        assert!((thirst.value() - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_set_priorities() {
        let mut set = NeedSet::new();

        let mut hunger = Need::new(NeedId::hunger(), 100.0, 1.0);
        hunger.set_value(50.0);
        set.add(hunger);

        let mut thirst = Need::new(NeedId::thirst(), 100.0, 1.0);
        thirst.set_value(5.0);
        set.add(thirst);

        let priorities = set.priorities();

        assert_eq!(priorities[0].id, NeedId::thirst());
        assert!(priorities[0].urgency > priorities[1].urgency);
    }

    #[test]
    fn test_need_set_deterministic_ordering() {
        let mut set = NeedSet::new();

        let mut hunger = Need::new(NeedId::hunger(), 100.0, 1.0);
        hunger.set_value(50.0);
        set.add(hunger);

        let mut thirst = Need::new(NeedId::thirst(), 100.0, 1.0);
        thirst.set_value(50.0);
        set.add(thirst);

        let p1 = set.priorities();
        let p2 = set.priorities();

        assert_eq!(p1[0].id, p2[0].id);
        assert_eq!(p1[1].id, p2[1].id);
    }

    #[test]
    fn test_need_set_state_counts() {
        let mut set = NeedSet::new();

        let mut n1 = Need::new(NeedId::hunger(), 100.0, 1.0);
        n1.set_value(90.0);
        set.add(n1);

        let mut n2 = Need::new(NeedId::thirst(), 100.0, 1.0);
        n2.set_value(50.0);
        set.add(n2);

        let mut n3 = Need::new(NeedId::oxygen(), 100.0, 1.0);
        n3.set_value(5.0);
        set.add(n3);

        let counts = set.state_counts();

        assert_eq!(counts[0], 1); // Satisfied
        assert_eq!(counts[1], 1); // Normal
        assert_eq!(counts[3], 1); // Critical
    }

    #[test]
    fn test_need_set_from_needs() {
        let needs = vec![
            Need::new(NeedId::hunger(), 100.0, 1.0),
            Need::new(NeedId::thirst(), 100.0, 2.0),
        ];

        let set = NeedSet::from_needs(needs);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_need_priority_ordering() {
        let p1 = NeedPriority {
            id: NeedId::hunger(),
            urgency: 5.0,
        };
        let p2 = NeedPriority {
            id: NeedId::thirst(),
            urgency: 10.0,
        };

        assert!(p2 < p1);
    }

    #[test]
    fn test_serde_round_trip() {
        let mut need = Need::new(NeedId::hunger(), 100.0, 5.0);
        need.set_value(42.0);

        let json = serde_json::to_string(&need).unwrap();
        let restored: Need = serde_json::from_str(&json).unwrap();

        assert!((restored.value() - 42.0).abs() < f32::EPSILON);
        assert_eq!(restored.id, NeedId::hunger());
    }

    #[test]
    fn test_need_set_serde() {
        let mut set = NeedSet::new();
        set.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        set.add(Need::new(NeedId::thirst(), 100.0, 2.0));

        let json = serde_json::to_string(&set).unwrap();
        let restored: NeedSet = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert!(restored.contains(&NeedId::hunger()));
    }
}

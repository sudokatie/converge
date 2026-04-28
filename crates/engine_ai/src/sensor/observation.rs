//! Observations from sensed stimuli with memory and decay.

use super::{DetectionStrength, SensorKind, StimulusId, StimulusSource};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Unique identifier for an observation instance.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservationId(pub u64);

impl ObservationId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// A sensed observation from a stimulus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Unique identifier for this observation.
    pub id: ObservationId,
    /// The stimulus that caused this observation.
    pub stimulus_id: StimulusId,
    /// Source entity of the stimulus.
    pub source: StimulusSource,
    /// Sensor channel that detected this.
    pub kind: SensorKind,
    /// Estimated position of the source (may be imprecise).
    pub position: [f32; 3],
    /// Effective intensity after attenuation/occlusion.
    pub intensity: f32,
    /// Detection strength classification.
    pub strength: DetectionStrength,
    /// Distance from observer to source.
    pub distance: f32,
    /// Tick when first observed.
    pub observed_tick: u64,
    /// Tick when last refreshed (for persistent stimuli).
    pub last_refresh_tick: u64,
    /// Optional tag from the stimulus.
    pub tag: Option<String>,
    /// Priority score (computed from intensity, strength, kind weight).
    pub priority: f32,
    /// Confidence in position accuracy (0.0 to 1.0).
    pub position_confidence: f32,
}

impl Observation {
    /// Create a new observation.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "observation needs all these fields"
    )]
    pub fn new(
        id: ObservationId,
        stimulus_id: StimulusId,
        source: StimulusSource,
        kind: SensorKind,
        position: [f32; 3],
        intensity: f32,
        strength: DetectionStrength,
        distance: f32,
        tick: u64,
    ) -> Self {
        let priority = intensity * strength.weight();
        let position_confidence = Self::calculate_position_confidence(kind, strength, distance);

        Self {
            id,
            stimulus_id,
            source,
            kind,
            position,
            intensity,
            strength,
            distance,
            observed_tick: tick,
            last_refresh_tick: tick,
            tag: None,
            priority,
            position_confidence,
        }
    }

    fn calculate_position_confidence(
        kind: SensorKind,
        strength: DetectionStrength,
        distance: f32,
    ) -> f32 {
        let base = match kind {
            SensorKind::Sight => 0.95,
            SensorKind::Sound => 0.7,
            SensorKind::Vibration | SensorKind::ElectricalField => 0.5,
            SensorKind::Smell => 0.3,
            SensorKind::Heat => 0.6,
            SensorKind::Pressure => 0.4,
        };

        let strength_factor = match strength {
            DetectionStrength::None => 0.0,
            DetectionStrength::Faint => 0.5,
            DetectionStrength::Weak => 0.75,
            DetectionStrength::Strong => 1.0,
        };

        let distance_factor = 1.0 / (1.0 + distance * 0.01);

        (base * strength_factor * distance_factor).clamp(0.0, 1.0)
    }

    /// Set tag from stimulus.
    #[must_use]
    pub fn with_tag(mut self, tag: Option<String>) -> Self {
        self.tag = tag;
        self
    }

    /// Set a priority boost.
    #[must_use]
    pub fn with_priority_boost(mut self, boost: f32) -> Self {
        self.priority += boost;
        self
    }

    /// Refresh this observation (update `last_refresh_tick`).
    pub fn refresh(&mut self, tick: u64, new_intensity: f32, new_strength: DetectionStrength) {
        self.last_refresh_tick = tick;
        self.intensity = new_intensity;
        self.strength = new_strength;
        self.priority = new_intensity * new_strength.weight();
    }

    /// Get the age of this observation in ticks.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.observed_tick)
    }

    /// Get ticks since last refresh.
    #[must_use]
    pub fn staleness(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_refresh_tick)
    }

    /// Check if this observation is stale.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        self.staleness(current_tick) > max_staleness
    }
}

impl Eq for Observation {}

impl PartialOrd for Observation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Observation {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.kind.cmp(&other.kind))
            .then_with(|| self.observed_tick.cmp(&other.observed_tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Entry for priority queue with deterministic ordering.
#[derive(Clone, Debug)]
pub struct ObservationPriority {
    pub id: ObservationId,
    pub kind: SensorKind,
    pub priority: f32,
    pub observed_tick: u64,
}

impl PartialEq for ObservationPriority {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ObservationPriority {}

impl PartialOrd for ObservationPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObservationPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.kind.cmp(&other.kind))
            .then_with(|| self.observed_tick.cmp(&other.observed_tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Memory configuration for observation decay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of observations to retain.
    pub max_observations: usize,
    /// Ticks before an observation is considered stale.
    pub staleness_threshold: u64,
    /// Ticks before an observation is forgotten entirely.
    pub forget_threshold: u64,
    /// Decay factor applied to priority each tick (0.0 to 1.0).
    pub priority_decay_rate: f32,
}

impl MemoryConfig {
    #[must_use]
    pub fn new(
        max_observations: usize,
        staleness_threshold: u64,
        forget_threshold: u64,
        priority_decay_rate: f32,
    ) -> Self {
        Self {
            max_observations,
            staleness_threshold,
            forget_threshold,
            priority_decay_rate,
        }
    }

    /// Short-term memory (quick forget).
    #[must_use]
    pub fn short_term() -> Self {
        Self {
            max_observations: 20,
            staleness_threshold: 60,
            forget_threshold: 120,
            priority_decay_rate: 0.95,
        }
    }

    /// Standard memory.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            max_observations: 50,
            staleness_threshold: 300,
            forget_threshold: 600,
            priority_decay_rate: 0.98,
        }
    }

    /// Long-term memory (slow forget).
    #[must_use]
    pub fn long_term() -> Self {
        Self {
            max_observations: 100,
            staleness_threshold: 1200,
            forget_threshold: 3600,
            priority_decay_rate: 0.995,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Memory storage for observations with decay.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObservationMemory {
    /// Configuration for this memory.
    config: MemoryConfig,
    /// Stored observations by ID.
    observations: BTreeMap<ObservationId, Observation>,
    /// Current tick for aging.
    current_tick: u64,
}

impl ObservationMemory {
    /// Create with default config.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MemoryConfig::default())
    }

    /// Create with specific config.
    #[must_use]
    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            config,
            observations: BTreeMap::new(),
            current_tick: 0,
        }
    }

    /// Store a new observation.
    pub fn remember(&mut self, observation: Observation) {
        self.observations
            .insert(observation.id.clone(), observation);
        self.prune_if_needed();
    }

    /// Refresh an existing observation or add as new.
    pub fn refresh_or_remember(
        &mut self,
        stimulus_id: &StimulusId,
        observation: Observation,
        tick: u64,
    ) {
        if let Some(existing) = self
            .observations
            .values_mut()
            .find(|o| o.stimulus_id == *stimulus_id)
        {
            existing.refresh(tick, observation.intensity, observation.strength);
        } else {
            self.remember(observation);
        }
    }

    /// Get an observation by ID.
    #[must_use]
    pub fn get(&self, id: &ObservationId) -> Option<&Observation> {
        self.observations.get(id)
    }

    /// Remove an observation.
    pub fn forget(&mut self, id: &ObservationId) -> Option<Observation> {
        self.observations.remove(id)
    }

    /// Forget all observations from a source.
    pub fn forget_source(&mut self, source: &StimulusSource) {
        self.observations.retain(|_, o| o.source != *source);
    }

    /// Forget all observations of a kind.
    pub fn forget_kind(&mut self, kind: SensorKind) {
        self.observations.retain(|_, o| o.kind != kind);
    }

    /// Clear all observations.
    pub fn clear(&mut self) {
        self.observations.clear();
    }

    /// Get number of stored observations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Iterate over observations.
    pub fn iter(&self) -> impl Iterator<Item = &Observation> {
        self.observations.values()
    }

    /// Get observations of a specific kind.
    pub fn by_kind(&self, kind: SensorKind) -> impl Iterator<Item = &Observation> {
        self.observations.values().filter(move |o| o.kind == kind)
    }

    /// Get observations from a specific source.
    pub fn by_source(&self, source: &StimulusSource) -> impl Iterator<Item = &Observation> {
        self.observations
            .values()
            .filter(move |o| &o.source == source)
    }

    /// Get fresh (non-stale) observations.
    pub fn fresh(&self) -> impl Iterator<Item = &Observation> {
        let threshold = self.config.staleness_threshold;
        let tick = self.current_tick;
        self.observations
            .values()
            .filter(move |o| !o.is_stale(tick, threshold))
    }

    /// Get stale observations.
    pub fn stale(&self) -> impl Iterator<Item = &Observation> {
        let threshold = self.config.staleness_threshold;
        let tick = self.current_tick;
        self.observations
            .values()
            .filter(move |o| o.is_stale(tick, threshold))
    }

    /// Tick the memory forward, applying decay and pruning.
    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.apply_decay();
        self.prune_forgotten();
    }

    /// Advance to a specific tick.
    pub fn advance_to(&mut self, tick: u64) {
        while self.current_tick < tick {
            self.tick();
        }
    }

    fn apply_decay(&mut self) {
        let decay = self.config.priority_decay_rate;
        for observation in self.observations.values_mut() {
            observation.priority *= decay;
        }
    }

    fn prune_forgotten(&mut self) {
        let threshold = self.config.forget_threshold;
        let tick = self.current_tick;
        self.observations
            .retain(|_, o| o.staleness(tick) <= threshold);
    }

    fn prune_if_needed(&mut self) {
        if self.observations.len() <= self.config.max_observations {
            return;
        }

        let mut priorities: Vec<_> = self
            .observations
            .iter()
            .map(|(id, o)| ObservationPriority {
                id: id.clone(),
                kind: o.kind,
                priority: o.priority,
                observed_tick: o.observed_tick,
            })
            .collect();

        priorities.sort();

        let to_remove: Vec<_> = priorities
            .iter()
            .skip(self.config.max_observations)
            .map(|p| p.id.clone())
            .collect();

        for id in to_remove {
            self.observations.remove(&id);
        }
    }

    /// Get observations sorted by priority (deterministic).
    #[must_use]
    pub fn priorities(&self) -> Vec<ObservationPriority> {
        let mut priorities: Vec<_> = self
            .observations
            .values()
            .map(|o| ObservationPriority {
                id: o.id.clone(),
                kind: o.kind,
                priority: o.priority,
                observed_tick: o.observed_tick,
            })
            .collect();
        priorities.sort();
        priorities
    }

    /// Get the highest priority observation.
    #[must_use]
    pub fn most_urgent(&self) -> Option<&Observation> {
        self.priorities()
            .first()
            .and_then(|p| self.observations.get(&p.id))
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the config.
    #[must_use]
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
}

/// A collection of observations from a single sensing pass.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObservationSet {
    observations: Vec<Observation>,
    tick: u64,
}

impl ObservationSet {
    /// Create a new empty set.
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            observations: Vec::new(),
            tick,
        }
    }

    /// Add an observation.
    pub fn add(&mut self, observation: Observation) {
        self.observations.push(observation);
    }

    /// Get the tick when this set was created.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Get number of observations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Iterate over observations.
    pub fn iter(&self) -> impl Iterator<Item = &Observation> {
        self.observations.iter()
    }

    /// Consume into vector of observations.
    #[must_use]
    pub fn into_vec(self) -> Vec<Observation> {
        self.observations
    }

    /// Get observations sorted by priority (deterministic).
    #[must_use]
    pub fn sorted(&self) -> Vec<&Observation> {
        let mut obs: Vec<_> = self.observations.iter().collect();
        obs.sort();
        obs
    }

    /// Get observations by kind.
    pub fn by_kind(&self, kind: SensorKind) -> impl Iterator<Item = &Observation> {
        self.observations.iter().filter(move |o| o.kind == kind)
    }

    /// Get total priority score.
    #[must_use]
    pub fn total_priority(&self) -> f32 {
        self.observations.iter().map(|o| o.priority).sum()
    }

    /// Get count by kind.
    #[must_use]
    pub fn count_by_kind(&self) -> BTreeMap<SensorKind, usize> {
        let mut counts = BTreeMap::new();
        for obs in &self.observations {
            *counts.entry(obs.kind).or_insert(0) += 1;
        }
        counts
    }

    /// Merge into memory, refreshing existing observations.
    pub fn merge_into_memory(self, memory: &mut ObservationMemory) {
        let tick = self.tick;
        for obs in self.observations {
            let stimulus_id = obs.stimulus_id.clone();
            memory.refresh_or_remember(&stimulus_id, obs, tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_observation(id: u64, kind: SensorKind, intensity: f32, tick: u64) -> Observation {
        Observation::new(
            ObservationId::new(id),
            StimulusId::new(id),
            StimulusSource::new(1),
            kind,
            [0.0, 0.0, 0.0],
            intensity,
            DetectionStrength::Strong,
            10.0,
            tick,
        )
    }

    #[test]
    fn test_observation_id() {
        let id = ObservationId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_observation_new() {
        let obs = make_observation(1, SensorKind::Sound, 50.0, 100);

        assert_eq!(obs.kind, SensorKind::Sound);
        assert!((obs.intensity - 50.0).abs() < f32::EPSILON);
        assert_eq!(obs.observed_tick, 100);
        assert_eq!(obs.last_refresh_tick, 100);
    }

    #[test]
    fn test_observation_age_staleness() {
        let obs = make_observation(1, SensorKind::Sound, 50.0, 100);

        assert_eq!(obs.age(100), 0);
        assert_eq!(obs.age(150), 50);
        assert_eq!(obs.staleness(150), 50);
    }

    #[test]
    fn test_observation_refresh() {
        let mut obs = make_observation(1, SensorKind::Sound, 50.0, 100);
        obs.refresh(150, 75.0, DetectionStrength::Weak);

        assert_eq!(obs.last_refresh_tick, 150);
        assert!((obs.intensity - 75.0).abs() < f32::EPSILON);
        assert_eq!(obs.strength, DetectionStrength::Weak);
    }

    #[test]
    fn test_observation_ordering_deterministic() {
        let o1 = make_observation(1, SensorKind::Sound, 50.0, 100);
        let o2 = make_observation(2, SensorKind::Sound, 50.0, 100);

        let cmp1 = o1.cmp(&o2);
        let cmp2 = o1.cmp(&o2);
        assert_eq!(cmp1, cmp2);
    }

    #[test]
    fn test_observation_ordering_by_priority() {
        let high = make_observation(1, SensorKind::Sound, 100.0, 100);
        let low = make_observation(2, SensorKind::Sound, 10.0, 100);

        assert!(high < low);
    }

    #[test]
    fn test_observation_ordering_tiebreak_kind() {
        let sight = make_observation(1, SensorKind::Sight, 50.0, 100);
        let sound = make_observation(2, SensorKind::Sound, 50.0, 100);

        assert!(sight < sound);
    }

    #[test]
    fn test_observation_ordering_tiebreak_tick() {
        let early = make_observation(1, SensorKind::Sound, 50.0, 100);
        let late = make_observation(2, SensorKind::Sound, 50.0, 200);

        assert!(early < late);
    }

    #[test]
    fn test_observation_ordering_tiebreak_id() {
        let obs1 = make_observation(1, SensorKind::Sound, 50.0, 100);
        let obs2 = make_observation(2, SensorKind::Sound, 50.0, 100);

        assert!(obs1 < obs2);
    }

    #[test]
    fn test_memory_config_presets() {
        let short = MemoryConfig::short_term();
        let standard = MemoryConfig::standard();
        let long = MemoryConfig::long_term();

        assert!(short.forget_threshold < standard.forget_threshold);
        assert!(standard.forget_threshold < long.forget_threshold);
    }

    #[test]
    fn test_observation_memory_remember() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 100));

        assert_eq!(memory.len(), 1);
        assert!(memory.get(&ObservationId::new(1)).is_some());
    }

    #[test]
    fn test_observation_memory_forget() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 100));
        memory.forget(&ObservationId::new(1));

        assert!(memory.is_empty());
    }

    #[test]
    fn test_observation_memory_by_kind() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 100));
        memory.remember(make_observation(2, SensorKind::Sight, 50.0, 100));
        memory.remember(make_observation(3, SensorKind::Sound, 30.0, 100));

        let sounds: Vec<_> = memory.by_kind(SensorKind::Sound).collect();
        assert_eq!(sounds.len(), 2);
    }

    #[test]
    fn test_observation_memory_tick_decay() {
        let mut memory = ObservationMemory::with_config(MemoryConfig {
            priority_decay_rate: 0.5,
            ..MemoryConfig::default()
        });

        memory.remember(make_observation(1, SensorKind::Sound, 100.0, 0));
        let initial_priority = memory.get(&ObservationId::new(1)).unwrap().priority;

        memory.tick();
        let decayed_priority = memory.get(&ObservationId::new(1)).unwrap().priority;

        assert!(decayed_priority < initial_priority);
        assert!((decayed_priority - initial_priority * 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_observation_memory_prune_forgotten() {
        let mut memory = ObservationMemory::with_config(MemoryConfig {
            forget_threshold: 10,
            ..MemoryConfig::default()
        });

        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 0));

        for _ in 0..15 {
            memory.tick();
        }

        assert!(memory.is_empty());
    }

    #[test]
    fn test_observation_memory_prune_max() {
        let mut memory = ObservationMemory::with_config(MemoryConfig {
            max_observations: 3,
            ..MemoryConfig::default()
        });

        memory.remember(make_observation(1, SensorKind::Sound, 10.0, 0));
        memory.remember(make_observation(2, SensorKind::Sound, 50.0, 0));
        memory.remember(make_observation(3, SensorKind::Sound, 30.0, 0));
        memory.remember(make_observation(4, SensorKind::Sound, 100.0, 0));

        assert_eq!(memory.len(), 3);
        assert!(memory.get(&ObservationId::new(1)).is_none());
    }

    #[test]
    fn test_observation_memory_priorities_deterministic() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 100));
        memory.remember(make_observation(2, SensorKind::Sound, 50.0, 100));

        let p1 = memory.priorities();
        let p2 = memory.priorities();

        assert_eq!(p1.len(), p2.len());
        for (a, b) in p1.iter().zip(p2.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn test_observation_memory_most_urgent() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(1, SensorKind::Sound, 10.0, 0));
        memory.remember(make_observation(2, SensorKind::Sound, 100.0, 0));
        memory.remember(make_observation(3, SensorKind::Sound, 50.0, 0));

        let urgent = memory.most_urgent().unwrap();
        assert_eq!(urgent.id, ObservationId::new(2));
    }

    #[test]
    fn test_observation_memory_fresh_stale() {
        let mut memory = ObservationMemory::with_config(MemoryConfig {
            staleness_threshold: 5,
            forget_threshold: 100,
            ..MemoryConfig::default()
        });

        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 0));
        assert_eq!(memory.fresh().count(), 1);
        assert_eq!(memory.stale().count(), 0);

        for _ in 0..10 {
            memory.tick();
        }

        assert_eq!(memory.fresh().count(), 0);
        assert_eq!(memory.stale().count(), 1);
    }

    #[test]
    fn test_observation_set_basic() {
        let mut set = ObservationSet::new(100);
        set.add(make_observation(1, SensorKind::Sound, 50.0, 100));
        set.add(make_observation(2, SensorKind::Sight, 30.0, 100));

        assert_eq!(set.len(), 2);
        assert_eq!(set.tick(), 100);
    }

    #[test]
    fn test_observation_set_sorted() {
        let mut set = ObservationSet::new(100);
        set.add(make_observation(1, SensorKind::Sound, 10.0, 100));
        set.add(make_observation(2, SensorKind::Sound, 100.0, 100));
        set.add(make_observation(3, SensorKind::Sound, 50.0, 100));

        let sorted = set.sorted();
        assert_eq!(sorted[0].id, ObservationId::new(2));
        assert_eq!(sorted[1].id, ObservationId::new(3));
        assert_eq!(sorted[2].id, ObservationId::new(1));
    }

    #[test]
    fn test_observation_set_by_kind() {
        let mut set = ObservationSet::new(100);
        set.add(make_observation(1, SensorKind::Sound, 50.0, 100));
        set.add(make_observation(2, SensorKind::Sight, 30.0, 100));

        assert_eq!(set.by_kind(SensorKind::Sound).count(), 1);
        assert_eq!(set.by_kind(SensorKind::Smell).count(), 0);
    }

    #[test]
    fn test_observation_set_count_by_kind() {
        let mut set = ObservationSet::new(100);
        set.add(make_observation(1, SensorKind::Sound, 50.0, 100));
        set.add(make_observation(2, SensorKind::Sound, 30.0, 100));
        set.add(make_observation(3, SensorKind::Sight, 40.0, 100));

        let counts = set.count_by_kind();
        assert_eq!(counts.get(&SensorKind::Sound), Some(&2));
        assert_eq!(counts.get(&SensorKind::Sight), Some(&1));
    }

    #[test]
    fn test_observation_set_merge_into_memory() {
        let mut memory = ObservationMemory::new();
        let mut set = ObservationSet::new(100);
        set.add(make_observation(1, SensorKind::Sound, 50.0, 100));

        set.merge_into_memory(&mut memory);

        assert_eq!(memory.len(), 1);
    }

    #[test]
    fn test_observation_serde() {
        let obs = make_observation(42, SensorKind::Vibration, 75.0, 500)
            .with_tag(Some("footstep".to_string()));

        let json = serde_json::to_string(&obs).unwrap();
        let restored: Observation = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, ObservationId::new(42));
        assert_eq!(restored.kind, SensorKind::Vibration);
        assert_eq!(restored.tag.as_deref(), Some("footstep"));
    }

    #[test]
    fn test_observation_memory_serde() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(1, SensorKind::Sound, 50.0, 100));
        memory.remember(make_observation(2, SensorKind::Sight, 30.0, 100));

        let json = serde_json::to_string(&memory).unwrap();
        let restored: ObservationMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_observation_set_serde() {
        let mut set = ObservationSet::new(100);
        set.add(make_observation(1, SensorKind::Sound, 50.0, 100));

        let json = serde_json::to_string(&set).unwrap();
        let restored: ObservationSet = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 1);
        assert_eq!(restored.tick(), 100);
    }

    #[test]
    fn test_position_confidence_varies_by_sensor() {
        let sight = make_observation(1, SensorKind::Sight, 50.0, 0);
        let smell = make_observation(2, SensorKind::Smell, 50.0, 0);

        assert!(sight.position_confidence > smell.position_confidence);
    }
}

//! Per-agent sensor suites and sensor profiles.

use super::{
    DetectionStrength, MemoryConfig, Observation, ObservationId, ObservationMemory, ObservationSet,
    SensorConfig, SensorKind, SensorSpec, Stimulus,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identifier for a sensor profile (creature type).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SensorProfileId(pub String);

impl SensorProfileId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for SensorProfileId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// A profile defining the sensor configuration for a creature type.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SensorProfile {
    /// Unique identifier for this profile.
    pub id: SensorProfileId,
    /// Sensor specifications by kind.
    sensors: BTreeMap<SensorKind, SensorSpec>,
    /// Memory configuration for this profile.
    pub memory_config: MemoryConfig,
}

impl SensorProfile {
    /// Create a new empty profile.
    #[must_use]
    pub fn new(id: SensorProfileId) -> Self {
        Self {
            id,
            sensors: BTreeMap::new(),
            memory_config: MemoryConfig::default(),
        }
    }

    /// Add a sensor specification.
    pub fn add_sensor(&mut self, spec: SensorSpec) {
        self.sensors.insert(spec.kind, spec);
    }

    /// Builder method to add a sensor.
    #[must_use]
    pub fn with_sensor(mut self, spec: SensorSpec) -> Self {
        self.add_sensor(spec);
        self
    }

    /// Builder method to set memory config.
    #[must_use]
    pub fn with_memory(mut self, config: MemoryConfig) -> Self {
        self.memory_config = config;
        self
    }

    /// Get a sensor spec by kind.
    #[must_use]
    pub fn get_sensor(&self, kind: SensorKind) -> Option<&SensorSpec> {
        self.sensors.get(&kind)
    }

    /// Get a mutable sensor spec.
    pub fn get_sensor_mut(&mut self, kind: SensorKind) -> Option<&mut SensorSpec> {
        self.sensors.get_mut(&kind)
    }

    /// Check if profile has a sensor.
    #[must_use]
    pub fn has_sensor(&self, kind: SensorKind) -> bool {
        self.sensors.contains_key(&kind)
    }

    /// Get number of sensors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sensors.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }

    /// Iterate over sensor kinds.
    pub fn sensor_kinds(&self) -> impl Iterator<Item = SensorKind> + '_ {
        self.sensors.keys().copied()
    }

    /// Iterate over sensor specs.
    pub fn sensors(&self) -> impl Iterator<Item = &SensorSpec> {
        self.sensors.values()
    }

    /// Create a sensor suite from this profile.
    #[must_use]
    pub fn create_suite(&self) -> SensorSuite {
        let mut suite = SensorSuite::new();
        for spec in self.sensors.values() {
            suite.add_sensor(spec.kind, spec.config.clone());
        }
        suite.memory = ObservationMemory::with_config(self.memory_config.clone());
        suite
    }

    /// Create a standard humanoid profile.
    #[must_use]
    pub fn humanoid() -> Self {
        Self::new(SensorProfileId::new("humanoid"))
            .with_sensor(SensorSpec::humanoid_vision())
            .with_sensor(SensorSpec::humanoid_hearing())
            .with_sensor(SensorSpec::new(SensorKind::Smell))
            .with_memory(MemoryConfig::standard())
    }

    /// Create a predator profile with enhanced senses.
    #[must_use]
    pub fn predator() -> Self {
        Self::new(SensorProfileId::new("predator"))
            .with_sensor(
                SensorSpec::humanoid_vision()
                    .with_config(
                        SensorConfig::basic(SensorKind::Sight)
                            .with_range(80.0)
                            .with_sensitivity(1.5),
                    )
                    .with_priority_weight(2.5),
            )
            .with_sensor(
                SensorSpec::humanoid_hearing()
                    .with_config(SensorConfig::basic(SensorKind::Sound).with_sensitivity(2.0)),
            )
            .with_sensor(SensorSpec::keen_smell())
            .with_sensor(SensorSpec::seismic())
            .with_memory(MemoryConfig::long_term())
    }

    /// Create a prey profile with wide awareness.
    #[must_use]
    pub fn prey() -> Self {
        Self::new(SensorProfileId::new("prey"))
            .with_sensor(
                SensorSpec::new(SensorKind::Sight).with_config(
                    SensorConfig::basic(SensorKind::Sight)
                        .with_range(50.0)
                        .with_fov(Some(std::f32::consts::PI * 1.5)),
                ),
            )
            .with_sensor(SensorSpec::humanoid_hearing())
            .with_sensor(SensorSpec::seismic())
            .with_memory(MemoryConfig::short_term())
    }

    /// Create an aquatic creature profile.
    #[must_use]
    pub fn aquatic() -> Self {
        Self::new(SensorProfileId::new("aquatic"))
            .with_sensor(
                SensorSpec::new(SensorKind::Sight)
                    .with_config(SensorConfig::basic(SensorKind::Sight).with_range(30.0)),
            )
            .with_sensor(SensorSpec::new(SensorKind::Pressure))
            .with_sensor(SensorSpec::seismic())
            .with_sensor(SensorSpec::electroreception())
            .with_memory(MemoryConfig::standard())
    }

    /// Create an underground creature profile.
    #[must_use]
    pub fn subterranean() -> Self {
        Self::new(SensorProfileId::new("subterranean"))
            .with_sensor(
                SensorSpec::seismic().with_config(
                    SensorConfig::basic(SensorKind::Vibration)
                        .with_range(50.0)
                        .with_sensitivity(3.0),
                ),
            )
            .with_sensor(SensorSpec::thermal())
            .with_sensor(SensorSpec::keen_smell())
            .with_memory(MemoryConfig::standard())
    }
}

/// Result of checking whether a stimulus can be sensed.
#[derive(Clone, Debug)]
pub struct SenseResult {
    pub can_sense: bool,
    pub effective_intensity: f32,
    pub strength: DetectionStrength,
    pub distance: f32,
}

/// A collection of sensors for a single entity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SensorSuite {
    /// Sensors by kind.
    sensors: BTreeMap<SensorKind, SensorConfig>,
    /// Position of the sensor suite owner.
    pub position: [f32; 3],
    /// Facing direction (unit vector, for directional sensors).
    pub facing: [f32; 3],
    /// Observation memory.
    pub memory: ObservationMemory,
    /// Next observation ID to assign.
    next_observation_id: u64,
    /// Global enabled state (can disable all sensors at once).
    enabled: bool,
}

impl SensorSuite {
    /// Create a new empty suite.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sensors: BTreeMap::new(),
            position: [0.0, 0.0, 0.0],
            facing: [1.0, 0.0, 0.0],
            memory: ObservationMemory::new(),
            next_observation_id: 1,
            enabled: true,
        }
    }

    /// Add a sensor to the suite.
    pub fn add_sensor(&mut self, kind: SensorKind, config: SensorConfig) {
        self.sensors.insert(kind, config);
    }

    /// Remove a sensor from the suite.
    pub fn remove_sensor(&mut self, kind: SensorKind) -> Option<SensorConfig> {
        self.sensors.remove(&kind)
    }

    /// Get a sensor config.
    #[must_use]
    pub fn get_sensor(&self, kind: SensorKind) -> Option<&SensorConfig> {
        self.sensors.get(&kind)
    }

    /// Get a mutable sensor config.
    pub fn get_sensor_mut(&mut self, kind: SensorKind) -> Option<&mut SensorConfig> {
        self.sensors.get_mut(&kind)
    }

    /// Check if suite has a sensor kind.
    #[must_use]
    pub fn has_sensor(&self, kind: SensorKind) -> bool {
        self.sensors.contains_key(&kind)
    }

    /// Get number of sensors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sensors.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }

    /// Check if enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set global enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Enable a specific sensor.
    pub fn enable_sensor(&mut self, kind: SensorKind) {
        if let Some(config) = self.sensors.get_mut(&kind) {
            config.enabled = true;
        }
    }

    /// Disable a specific sensor.
    pub fn disable_sensor(&mut self, kind: SensorKind) {
        if let Some(config) = self.sensors.get_mut(&kind) {
            config.enabled = false;
        }
    }

    /// Update position and facing.
    pub fn set_transform(&mut self, position: [f32; 3], facing: [f32; 3]) {
        self.position = position;
        self.facing = facing;
    }

    /// Iterate over sensor kinds.
    pub fn sensor_kinds(&self) -> impl Iterator<Item = SensorKind> + '_ {
        self.sensors.keys().copied()
    }

    /// Get all enabled sensor kinds.
    pub fn enabled_sensors(&self) -> impl Iterator<Item = SensorKind> + '_ {
        self.sensors
            .iter()
            .filter(|(_, c)| c.enabled)
            .map(|(k, _)| *k)
    }

    fn generate_observation_id(&mut self) -> ObservationId {
        let id = ObservationId::new(self.next_observation_id);
        self.next_observation_id += 1;
        id
    }

    /// Check if a stimulus can be sensed by this suite.
    #[must_use]
    pub fn can_sense(
        &self,
        stimulus: &Stimulus,
        is_blocked: bool,
        blocker_count: u32,
    ) -> SenseResult {
        if !self.enabled {
            return SenseResult {
                can_sense: false,
                effective_intensity: 0.0,
                strength: DetectionStrength::None,
                distance: 0.0,
            };
        }

        let Some(config) = self.sensors.get(&stimulus.kind) else {
            return SenseResult {
                can_sense: false,
                effective_intensity: 0.0,
                strength: DetectionStrength::None,
                distance: 0.0,
            };
        };

        if !config.enabled {
            return SenseResult {
                can_sense: false,
                effective_intensity: 0.0,
                strength: DetectionStrength::None,
                distance: 0.0,
            };
        }

        let distance = self.distance_to(stimulus.position);

        if !self.in_field_of_view(stimulus.position, config) {
            return SenseResult {
                can_sense: false,
                effective_intensity: 0.0,
                strength: DetectionStrength::None,
                distance,
            };
        }

        let effective =
            config.effective_intensity(stimulus.intensity, distance, is_blocked, blocker_count);
        let strength = config.detection_strength(effective);
        let can_sense = strength != DetectionStrength::None;

        SenseResult {
            can_sense,
            effective_intensity: effective,
            strength,
            distance,
        }
    }

    fn distance_to(&self, point: [f32; 3]) -> f32 {
        let dx = self.position[0] - point[0];
        let dy = self.position[1] - point[1];
        let dz = self.position[2] - point[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn in_field_of_view(&self, point: [f32; 3], config: &SensorConfig) -> bool {
        let Some(fov) = config.field_of_view else {
            return true;
        };

        let dx = point[0] - self.position[0];
        let dy = point[1] - self.position[1];
        let dz = point[2] - self.position[2];

        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if len < f32::EPSILON {
            return true;
        }

        let dir = [dx / len, dy / len, dz / len];
        let dot = self.facing[0] * dir[0] + self.facing[1] * dir[1] + self.facing[2] * dir[2];

        dot.clamp(-1.0, 1.0).acos() <= fov / 2.0
    }

    /// Sense a single stimulus, returning an observation if detected.
    pub fn sense(
        &mut self,
        stimulus: &Stimulus,
        is_blocked: bool,
        blocker_count: u32,
        tick: u64,
    ) -> Option<Observation> {
        let result = self.can_sense(stimulus, is_blocked, blocker_count);

        if !result.can_sense {
            return None;
        }

        let observation = Observation::new(
            self.generate_observation_id(),
            stimulus.id.clone(),
            stimulus.source.clone(),
            stimulus.kind,
            stimulus.position,
            result.effective_intensity,
            result.strength,
            result.distance,
            tick,
        )
        .with_tag(stimulus.tag.clone())
        .with_priority_boost(stimulus.priority_boost);

        Some(observation)
    }

    /// Sense multiple stimuli, returning all detected observations.
    pub fn sense_all<'a>(
        &mut self,
        stimuli: impl Iterator<Item = StimuliWithOcclusion<'a>>,
        tick: u64,
    ) -> ObservationSet {
        let mut set = ObservationSet::new(tick);

        for swoc in stimuli {
            if let Some(obs) = self.sense(swoc.stimulus, swoc.is_blocked, swoc.blocker_count, tick)
            {
                set.add(obs);
            }
        }

        set
    }

    /// Sense and immediately store in memory.
    pub fn sense_and_remember(
        &mut self,
        stimulus: &Stimulus,
        is_blocked: bool,
        blocker_count: u32,
        tick: u64,
    ) -> bool {
        if let Some(obs) = self.sense(stimulus, is_blocked, blocker_count, tick) {
            self.memory.refresh_or_remember(&stimulus.id, obs, tick);
            true
        } else {
            false
        }
    }

    /// Tick memory forward.
    pub fn tick_memory(&mut self) {
        self.memory.tick();
    }

    /// Get memory reference.
    #[must_use]
    pub fn memory(&self) -> &ObservationMemory {
        &self.memory
    }

    /// Get mutable memory reference.
    pub fn memory_mut(&mut self) -> &mut ObservationMemory {
        &mut self.memory
    }

    /// Get the most urgent current observation from memory.
    #[must_use]
    pub fn most_urgent_observation(&self) -> Option<&Observation> {
        self.memory.most_urgent()
    }

    /// Get count of fresh observations by kind.
    #[must_use]
    pub fn fresh_observation_counts(&self) -> BTreeMap<SensorKind, usize> {
        let mut counts = BTreeMap::new();
        for obs in self.memory.fresh() {
            *counts.entry(obs.kind).or_insert(0) += 1;
        }
        counts
    }
}

/// Helper struct for providing stimuli with occlusion info.
pub struct StimuliWithOcclusion<'a> {
    pub stimulus: &'a Stimulus,
    pub is_blocked: bool,
    pub blocker_count: u32,
}

impl<'a> StimuliWithOcclusion<'a> {
    #[must_use]
    pub fn new(stimulus: &'a Stimulus, is_blocked: bool, blocker_count: u32) -> Self {
        Self {
            stimulus,
            is_blocked,
            blocker_count,
        }
    }

    #[must_use]
    pub fn unblocked(stimulus: &'a Stimulus) -> Self {
        Self::new(stimulus, false, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::{StimulusId, StimulusSource};

    fn make_stimulus(kind: SensorKind, position: [f32; 3], intensity: f32) -> Stimulus {
        Stimulus::new(
            StimulusId::new(1),
            kind,
            StimulusSource::new(1),
            position,
            intensity,
        )
    }

    #[test]
    fn test_sensor_profile_id() {
        let id = SensorProfileId::new("test");
        assert_eq!(id.as_str(), "test");
    }

    #[test]
    fn test_sensor_profile_new() {
        let profile = SensorProfile::new(SensorProfileId::new("test"));
        assert!(profile.is_empty());
    }

    #[test]
    fn test_sensor_profile_with_sensor() {
        let profile = SensorProfile::new(SensorProfileId::new("test"))
            .with_sensor(SensorSpec::new(SensorKind::Sight))
            .with_sensor(SensorSpec::new(SensorKind::Sound));

        assert_eq!(profile.len(), 2);
        assert!(profile.has_sensor(SensorKind::Sight));
        assert!(profile.has_sensor(SensorKind::Sound));
        assert!(!profile.has_sensor(SensorKind::Smell));
    }

    #[test]
    fn test_sensor_profile_humanoid() {
        let profile = SensorProfile::humanoid();
        assert!(profile.has_sensor(SensorKind::Sight));
        assert!(profile.has_sensor(SensorKind::Sound));
        assert!(profile.has_sensor(SensorKind::Smell));
    }

    #[test]
    fn test_sensor_profile_predator() {
        let profile = SensorProfile::predator();
        assert!(profile.has_sensor(SensorKind::Sight));
        assert!(profile.has_sensor(SensorKind::Vibration));
    }

    #[test]
    fn test_sensor_profile_aquatic() {
        let profile = SensorProfile::aquatic();
        assert!(profile.has_sensor(SensorKind::ElectricalField));
        assert!(profile.has_sensor(SensorKind::Pressure));
    }

    #[test]
    fn test_sensor_profile_create_suite() {
        let profile = SensorProfile::humanoid();
        let suite = profile.create_suite();

        assert!(suite.has_sensor(SensorKind::Sight));
        assert!(suite.has_sensor(SensorKind::Sound));
    }

    #[test]
    fn test_sensor_suite_new() {
        let suite = SensorSuite::new();
        assert!(suite.is_empty());
        assert!(suite.is_enabled());
    }

    #[test]
    fn test_sensor_suite_add_sensor() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sight, SensorConfig::basic(SensorKind::Sight));

        assert_eq!(suite.len(), 1);
        assert!(suite.has_sensor(SensorKind::Sight));
    }

    #[test]
    fn test_sensor_suite_enable_disable() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sight, SensorConfig::basic(SensorKind::Sight));

        suite.disable_sensor(SensorKind::Sight);
        assert!(!suite.get_sensor(SensorKind::Sight).unwrap().enabled);

        suite.enable_sensor(SensorKind::Sight);
        assert!(suite.get_sensor(SensorKind::Sight).unwrap().enabled);
    }

    #[test]
    fn test_sensor_suite_global_disable() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sight, SensorConfig::basic(SensorKind::Sight));

        let stimulus = make_stimulus(SensorKind::Sight, [5.0, 0.0, 0.0], 100.0);

        suite.set_enabled(false);
        let result = suite.can_sense(&stimulus, false, 0);
        assert!(!result.can_sense);

        suite.set_enabled(true);
        let result = suite.can_sense(&stimulus, false, 0);
        assert!(result.can_sense);
    }

    #[test]
    fn test_sensor_suite_can_sense_no_sensor() {
        let suite = SensorSuite::new();
        let stimulus = make_stimulus(SensorKind::Sound, [5.0, 0.0, 0.0], 100.0);

        let result = suite.can_sense(&stimulus, false, 0);
        assert!(!result.can_sense);
    }

    #[test]
    fn test_sensor_suite_can_sense_basic() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(
            SensorKind::Sound,
            SensorConfig::basic(SensorKind::Sound).with_range(50.0),
        );

        let stimulus = make_stimulus(SensorKind::Sound, [10.0, 0.0, 0.0], 100.0);
        let result = suite.can_sense(&stimulus, false, 0);

        assert!(result.can_sense);
        assert!(result.effective_intensity > 0.0);
        assert!((result.distance - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_suite_can_sense_out_of_range() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(
            SensorKind::Sound,
            SensorConfig::basic(SensorKind::Sound).with_range(10.0),
        );

        let stimulus = make_stimulus(SensorKind::Sound, [50.0, 0.0, 0.0], 100.0);
        let result = suite.can_sense(&stimulus, false, 0);

        assert!(!result.can_sense);
    }

    #[test]
    fn test_sensor_suite_can_sense_blocked() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sight, SensorConfig::basic(SensorKind::Sight));

        let stimulus = make_stimulus(SensorKind::Sight, [10.0, 0.0, 0.0], 100.0);

        let clear = suite.can_sense(&stimulus, false, 0);
        assert!(clear.can_sense);

        let blocked = suite.can_sense(&stimulus, true, 1);
        assert!(!blocked.can_sense);
    }

    #[test]
    fn test_sensor_suite_field_of_view() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(
            SensorKind::Sight,
            SensorConfig::basic(SensorKind::Sight).with_fov(Some(std::f32::consts::FRAC_PI_2)),
        );
        suite.facing = [1.0, 0.0, 0.0];

        let in_front = make_stimulus(SensorKind::Sight, [10.0, 0.0, 0.0], 100.0);
        let behind = make_stimulus(SensorKind::Sight, [-10.0, 0.0, 0.0], 100.0);

        assert!(suite.can_sense(&in_front, false, 0).can_sense);
        assert!(!suite.can_sense(&behind, false, 0).can_sense);
    }

    #[test]
    fn test_sensor_suite_sense() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(
            SensorKind::Sound,
            SensorConfig::basic(SensorKind::Sound).with_range(50.0),
        );

        let stimulus = make_stimulus(SensorKind::Sound, [10.0, 0.0, 0.0], 100.0);
        let obs = suite.sense(&stimulus, false, 0, 100);

        assert!(obs.is_some());
        let obs = obs.unwrap();
        assert_eq!(obs.kind, SensorKind::Sound);
        assert_eq!(obs.observed_tick, 100);
    }

    #[test]
    fn test_sensor_suite_sense_all() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));
        suite.add_sensor(SensorKind::Sight, SensorConfig::basic(SensorKind::Sight));

        let s1 = make_stimulus(SensorKind::Sound, [5.0, 0.0, 0.0], 100.0);
        let s2 = make_stimulus(SensorKind::Sight, [10.0, 0.0, 0.0], 50.0);

        let stimuli = [
            StimuliWithOcclusion::unblocked(&s1),
            StimuliWithOcclusion::unblocked(&s2),
        ];

        let set = suite.sense_all(stimuli.into_iter(), 100);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_sensor_suite_sense_and_remember() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));

        let stimulus = make_stimulus(SensorKind::Sound, [10.0, 0.0, 0.0], 100.0);

        assert!(suite.sense_and_remember(&stimulus, false, 0, 100));
        assert_eq!(suite.memory.len(), 1);
    }

    #[test]
    fn test_sensor_suite_tick_memory() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));

        let stimulus = make_stimulus(SensorKind::Sound, [10.0, 0.0, 0.0], 100.0);
        suite.sense_and_remember(&stimulus, false, 0, 0);

        let initial = suite.memory.get(&ObservationId::new(1)).unwrap().priority;
        suite.tick_memory();
        let decayed = suite.memory.get(&ObservationId::new(1)).unwrap().priority;

        assert!(decayed < initial);
    }

    #[test]
    fn test_sensor_suite_disabled_sensor_not_sensed() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(
            SensorKind::Sound,
            SensorConfig::basic(SensorKind::Sound).with_enabled(false),
        );

        let stimulus = make_stimulus(SensorKind::Sound, [5.0, 0.0, 0.0], 100.0);
        let result = suite.can_sense(&stimulus, false, 0);

        assert!(!result.can_sense);
    }

    #[test]
    fn test_sensor_suite_serde() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));
        suite.position = [1.0, 2.0, 3.0];

        let json = serde_json::to_string(&suite).unwrap();
        let restored: SensorSuite = serde_json::from_str(&json).unwrap();

        assert!(restored.has_sensor(SensorKind::Sound));
        assert!((restored.position[0] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_profile_serde() {
        let profile = SensorProfile::humanoid();

        let json = serde_json::to_string(&profile).unwrap();
        let restored: SensorProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, profile.id);
        assert!(restored.has_sensor(SensorKind::Sight));
    }

    #[test]
    fn test_observation_id_deterministic() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));

        let s1 = make_stimulus(SensorKind::Sound, [5.0, 0.0, 0.0], 100.0);
        let obs1 = suite.sense(&s1, false, 0, 0).unwrap();

        let s2 = Stimulus::new(
            StimulusId::new(2),
            SensorKind::Sound,
            StimulusSource::new(2),
            [10.0, 0.0, 0.0],
            50.0,
        );
        let obs2 = suite.sense(&s2, false, 0, 0).unwrap();

        assert_eq!(obs1.id, ObservationId::new(1));
        assert_eq!(obs2.id, ObservationId::new(2));
    }

    #[test]
    fn test_all_profile_kinds_create_valid_suites() {
        let profiles = [
            SensorProfile::humanoid(),
            SensorProfile::predator(),
            SensorProfile::prey(),
            SensorProfile::aquatic(),
            SensorProfile::subterranean(),
        ];

        for profile in profiles {
            let suite = profile.create_suite();
            assert!(!suite.is_empty());
        }
    }
}

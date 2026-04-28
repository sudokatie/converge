//! Summaries and snapshots for sensor data, supporting unloaded-chunk simulation.

use super::{DetectionStrength, ObservationMemory, SensorKind, SensorSuite};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Summary of stimuli detected by a sensor kind.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StimulusSummary {
    /// Sensor kind this summary is for.
    pub kind: SensorKind,
    /// Count of stimuli detected.
    pub count: u32,
    /// Count by detection strength.
    pub by_strength: [u32; 4],
    /// Total intensity of all detected stimuli.
    pub total_intensity: f32,
    /// Maximum intensity detected.
    pub max_intensity: f32,
    /// Minimum intensity detected (if any).
    pub min_intensity: f32,
    /// Average distance to detected stimuli.
    pub avg_distance: f32,
    /// Most common tag (if any).
    pub most_common_tag: Option<String>,
}

impl StimulusSummary {
    /// Create a new empty summary for a kind.
    #[must_use]
    pub fn new(kind: SensorKind) -> Self {
        Self {
            kind,
            count: 0,
            by_strength: [0; 4],
            total_intensity: 0.0,
            max_intensity: f32::MIN,
            min_intensity: f32::MAX,
            avg_distance: 0.0,
            most_common_tag: None,
        }
    }

    /// Add an observation to the summary.
    pub fn add(&mut self, intensity: f32, strength: DetectionStrength, distance: f32) {
        self.count += 1;
        self.total_intensity += intensity;
        self.max_intensity = self.max_intensity.max(intensity);
        self.min_intensity = self.min_intensity.min(intensity);

        let idx = match strength {
            DetectionStrength::None => 0,
            DetectionStrength::Faint => 1,
            DetectionStrength::Weak => 2,
            DetectionStrength::Strong => 3,
        };
        self.by_strength[idx] += 1;

        #[expect(
            clippy::cast_precision_loss,
            reason = "count precision loss acceptable"
        )]
        {
            self.avg_distance =
                (self.avg_distance * (self.count - 1) as f32 + distance) / self.count as f32;
        }
    }

    /// Get average intensity.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn average_intensity(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.total_intensity / self.count as f32
        }
    }

    /// Get count of strong detections.
    #[must_use]
    pub fn strong_count(&self) -> u32 {
        self.by_strength[3]
    }

    /// Get count of any detections (not None).
    #[must_use]
    pub fn detected_count(&self) -> u32 {
        self.by_strength[1] + self.by_strength[2] + self.by_strength[3]
    }

    /// Check if any threats (strong detections) exist.
    #[must_use]
    pub fn has_threats(&self) -> bool {
        self.by_strength[3] > 0
    }

    /// Merge another summary into this one.
    pub fn merge(&mut self, other: &Self) {
        if self.count == 0 {
            *self = other.clone();
            return;
        }

        if other.count == 0 {
            return;
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "count precision loss acceptable"
        )]
        let new_avg = (self.avg_distance * self.count as f32
            + other.avg_distance * other.count as f32)
            / (self.count + other.count) as f32;

        self.count += other.count;
        self.total_intensity += other.total_intensity;
        self.max_intensity = self.max_intensity.max(other.max_intensity);
        self.min_intensity = self.min_intensity.min(other.min_intensity);
        self.avg_distance = new_avg;

        for i in 0..4 {
            self.by_strength[i] += other.by_strength[i];
        }
    }
}

/// Aggregated summary of sensor observations across multiple entities.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SensorSummary {
    /// Number of entities included in this summary.
    pub entity_count: u32,
    /// Summaries by sensor kind.
    summaries: BTreeMap<SensorKind, StimulusSummary>,
    /// Total number of observations across all entities.
    pub total_observations: u32,
    /// Total urgency/threat level.
    pub total_urgency: f32,
    /// Tick when this summary was computed.
    pub computed_at_tick: u64,
}

impl SensorSummary {
    /// Create a new empty summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a sensor suite's observations to the summary.
    pub fn add_suite(&mut self, suite: &SensorSuite) {
        self.entity_count += 1;

        for obs in suite.memory().iter() {
            let summary = self
                .summaries
                .entry(obs.kind)
                .or_insert_with(|| StimulusSummary::new(obs.kind));

            summary.add(obs.intensity, obs.strength, obs.distance);
            self.total_observations += 1;
            self.total_urgency += obs.priority;
        }
    }

    /// Add observations from a memory directly.
    pub fn add_memory(&mut self, memory: &ObservationMemory) {
        self.entity_count += 1;

        for obs in memory.iter() {
            let summary = self
                .summaries
                .entry(obs.kind)
                .or_insert_with(|| StimulusSummary::new(obs.kind));

            summary.add(obs.intensity, obs.strength, obs.distance);
            self.total_observations += 1;
            self.total_urgency += obs.priority;
        }
    }

    /// Set the tick when computed.
    pub fn set_tick(&mut self, tick: u64) {
        self.computed_at_tick = tick;
    }

    /// Get summary for a specific sensor kind.
    #[must_use]
    pub fn get_kind_summary(&self, kind: SensorKind) -> Option<&StimulusSummary> {
        self.summaries.get(&kind)
    }

    /// Iterate over all kind summaries.
    pub fn kind_summaries(&self) -> impl Iterator<Item = &StimulusSummary> {
        self.summaries.values()
    }

    /// Get the sensor kind with the most observations.
    #[must_use]
    pub fn most_active_sensor(&self) -> Option<SensorKind> {
        self.summaries
            .iter()
            .max_by_key(|(_, s)| s.count)
            .map(|(k, _)| *k)
    }

    /// Check if any sensor has detected threats.
    #[must_use]
    pub fn has_threats(&self) -> bool {
        self.summaries.values().any(StimulusSummary::has_threats)
    }

    /// Get total threat count.
    #[must_use]
    pub fn threat_count(&self) -> u32 {
        self.summaries
            .values()
            .map(StimulusSummary::strong_count)
            .sum()
    }

    /// Merge another summary into this one.
    pub fn merge(&mut self, other: &Self) {
        self.entity_count += other.entity_count;
        self.total_observations += other.total_observations;
        self.total_urgency += other.total_urgency;

        for (kind, other_summary) in &other.summaries {
            self.summaries
                .entry(*kind)
                .or_insert_with(|| StimulusSummary::new(*kind))
                .merge(other_summary);
        }
    }

    /// Create from an iterator of sensor suites.
    pub fn from_suites<'a>(suites: impl Iterator<Item = &'a SensorSuite>, tick: u64) -> Self {
        let mut summary = Self::new();
        for suite in suites {
            summary.add_suite(suite);
        }
        summary.set_tick(tick);
        summary
    }

    /// Get average urgency per entity.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn average_urgency(&self) -> f32 {
        if self.entity_count == 0 {
            0.0
        } else {
            self.total_urgency / self.entity_count as f32
        }
    }

    /// Get observations per entity.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn observations_per_entity(&self) -> f32 {
        if self.entity_count == 0 {
            0.0
        } else {
            self.total_observations as f32 / self.entity_count as f32
        }
    }
}

/// A snapshot of sensor state suitable for persistence or unloaded chunk simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorSnapshot {
    /// Summary of sensor observations.
    pub summary: SensorSummary,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
    /// Region/colony identifier.
    pub region_id: u64,
    /// Estimated threat level (0.0 = safe, 1.0 = extreme danger).
    pub threat_level: f32,
    /// Whether attention is needed soon.
    pub needs_attention: bool,
    /// Dominant sensory input kind.
    pub dominant_kind: Option<SensorKind>,
    /// Estimated ticks until threat level changes significantly.
    pub ticks_until_change: Option<u64>,
}

impl SensorSnapshot {
    /// Create a new snapshot.
    #[must_use]
    pub fn new(region_id: u64, summary: SensorSummary, tick: u64) -> Self {
        let threat_level = Self::calculate_threat_level(&summary);
        let needs_attention = threat_level > 0.5 || summary.has_threats();
        let dominant_kind = summary.most_active_sensor();
        let ticks_until_change = Self::estimate_ticks_until_change(&summary);

        Self {
            summary,
            snapshot_tick: tick,
            region_id,
            threat_level,
            needs_attention,
            dominant_kind,
            ticks_until_change,
        }
    }

    fn calculate_threat_level(summary: &SensorSummary) -> f32 {
        if summary.entity_count == 0 {
            return 0.0;
        }

        let threat_ratio = if summary.total_observations > 0 {
            #[expect(
                clippy::cast_precision_loss,
                reason = "count precision loss acceptable"
            )]
            {
                summary.threat_count() as f32 / summary.total_observations as f32
            }
        } else {
            0.0
        };

        let urgency_factor = (summary.average_urgency() / 100.0).clamp(0.0, 1.0);

        (threat_ratio * 0.6 + urgency_factor * 0.4).clamp(0.0, 1.0)
    }

    fn estimate_ticks_until_change(summary: &SensorSummary) -> Option<u64> {
        if summary.total_observations == 0 {
            return None;
        }

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "estimation bounded"
        )]
        {
            let base_ticks = (100.0 / (1.0 + summary.average_urgency())) as u64;
            Some(base_ticks.max(10))
        }
    }

    /// Check if the snapshot is stale and needs recomputing.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Get the age of this snapshot in ticks.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }

    /// Check if the snapshot indicates a dangerous state.
    #[must_use]
    pub fn is_dangerous(&self) -> bool {
        self.threat_level > 0.7 || self.summary.has_threats()
    }

    /// Check if intervention might be needed soon.
    #[must_use]
    pub fn needs_intervention(&self, tick_threshold: u64) -> bool {
        self.needs_attention || self.ticks_until_change.is_some_and(|t| t < tick_threshold)
    }

    /// Estimate threat level after elapsed ticks (simple decay model).
    #[must_use]
    pub fn projected_threat(&self, elapsed_ticks: u64) -> f32 {
        let decay_factor = 0.99_f32.powi(elapsed_ticks.min(1000) as i32);
        self.threat_level * decay_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::{
        DetectionStrength, Observation, ObservationId, ObservationMemory, SensorConfig, StimulusId,
        StimulusSource,
    };

    fn make_observation(
        id: u64,
        kind: SensorKind,
        intensity: f32,
        strength: DetectionStrength,
    ) -> Observation {
        Observation::new(
            ObservationId::new(id),
            StimulusId::new(id),
            StimulusSource::new(1),
            kind,
            [0.0, 0.0, 0.0],
            intensity,
            strength,
            10.0,
            0,
        )
    }

    #[test]
    fn test_stimulus_summary_new() {
        let summary = StimulusSummary::new(SensorKind::Sound);
        assert_eq!(summary.kind, SensorKind::Sound);
        assert_eq!(summary.count, 0);
    }

    #[test]
    fn test_stimulus_summary_add() {
        let mut summary = StimulusSummary::new(SensorKind::Sound);

        summary.add(50.0, DetectionStrength::Strong, 10.0);
        summary.add(30.0, DetectionStrength::Weak, 20.0);

        assert_eq!(summary.count, 2);
        assert!((summary.total_intensity - 80.0).abs() < f32::EPSILON);
        assert!((summary.max_intensity - 50.0).abs() < f32::EPSILON);
        assert!((summary.min_intensity - 30.0).abs() < f32::EPSILON);
        assert_eq!(summary.strong_count(), 1);
    }

    #[test]
    fn test_stimulus_summary_average() {
        let mut summary = StimulusSummary::new(SensorKind::Sound);
        summary.add(50.0, DetectionStrength::Strong, 10.0);
        summary.add(30.0, DetectionStrength::Weak, 20.0);

        assert!((summary.average_intensity() - 40.0).abs() < f32::EPSILON);
        assert!((summary.avg_distance - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stimulus_summary_merge() {
        let mut s1 = StimulusSummary::new(SensorKind::Sound);
        s1.add(50.0, DetectionStrength::Strong, 10.0);

        let mut s2 = StimulusSummary::new(SensorKind::Sound);
        s2.add(30.0, DetectionStrength::Weak, 20.0);

        s1.merge(&s2);

        assert_eq!(s1.count, 2);
        assert_eq!(s1.strong_count(), 1);
    }

    #[test]
    fn test_sensor_summary_new() {
        let summary = SensorSummary::new();
        assert_eq!(summary.entity_count, 0);
        assert_eq!(summary.total_observations, 0);
    }

    #[test]
    fn test_sensor_summary_add_memory() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));
        memory.remember(make_observation(
            2,
            SensorKind::Sight,
            30.0,
            DetectionStrength::Weak,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);

        assert_eq!(summary.entity_count, 1);
        assert_eq!(summary.total_observations, 2);
        assert!(summary.get_kind_summary(SensorKind::Sound).is_some());
        assert!(summary.get_kind_summary(SensorKind::Sight).is_some());
    }

    #[test]
    fn test_sensor_summary_add_suite() {
        let mut suite = SensorSuite::new();
        suite.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));
        suite.memory_mut().remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));

        let mut summary = SensorSummary::new();
        summary.add_suite(&suite);

        assert_eq!(summary.entity_count, 1);
        assert_eq!(summary.total_observations, 1);
    }

    #[test]
    fn test_sensor_summary_has_threats() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);

        assert!(summary.has_threats());
        assert_eq!(summary.threat_count(), 1);
    }

    #[test]
    fn test_sensor_summary_merge() {
        let mut memory1 = ObservationMemory::new();
        memory1.remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));

        let mut memory2 = ObservationMemory::new();
        memory2.remember(make_observation(
            2,
            SensorKind::Sight,
            30.0,
            DetectionStrength::Weak,
        ));

        let mut s1 = SensorSummary::new();
        s1.add_memory(&memory1);

        let mut s2 = SensorSummary::new();
        s2.add_memory(&memory2);

        s1.merge(&s2);

        assert_eq!(s1.entity_count, 2);
        assert_eq!(s1.total_observations, 2);
    }

    #[test]
    fn test_sensor_summary_most_active() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));
        memory.remember(make_observation(
            2,
            SensorKind::Sound,
            30.0,
            DetectionStrength::Weak,
        ));
        memory.remember(make_observation(
            3,
            SensorKind::Sight,
            40.0,
            DetectionStrength::Weak,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);

        assert_eq!(summary.most_active_sensor(), Some(SensorKind::Sound));
    }

    #[test]
    fn test_sensor_snapshot_new() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);

        let snapshot = SensorSnapshot::new(42, summary, 100);

        assert_eq!(snapshot.region_id, 42);
        assert_eq!(snapshot.snapshot_tick, 100);
        assert!(snapshot.threat_level > 0.0);
    }

    #[test]
    fn test_sensor_snapshot_staleness() {
        let summary = SensorSummary::new();
        let snapshot = SensorSnapshot::new(1, summary, 100);

        assert!(!snapshot.is_stale(150, 100));
        assert!(snapshot.is_stale(250, 100));
    }

    #[test]
    fn test_sensor_snapshot_dangerous() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            100.0,
            DetectionStrength::Strong,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);

        let snapshot = SensorSnapshot::new(1, summary, 100);

        assert!(snapshot.is_dangerous());
    }

    #[test]
    fn test_sensor_snapshot_projected_threat() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            100.0,
            DetectionStrength::Strong,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);

        let snapshot = SensorSnapshot::new(1, summary, 100);
        let initial = snapshot.threat_level;
        let projected = snapshot.projected_threat(100);

        assert!(projected < initial);
    }

    #[test]
    fn test_stimulus_summary_serde() {
        let mut summary = StimulusSummary::new(SensorKind::Vibration);
        summary.add(50.0, DetectionStrength::Strong, 10.0);

        let json = serde_json::to_string(&summary).unwrap();
        let restored: StimulusSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.kind, SensorKind::Vibration);
        assert_eq!(restored.count, 1);
    }

    #[test]
    fn test_sensor_summary_serde() {
        let mut memory = ObservationMemory::new();
        memory.remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));

        let mut summary = SensorSummary::new();
        summary.add_memory(&memory);
        summary.set_tick(100);

        let json = serde_json::to_string(&summary).unwrap();
        let restored: SensorSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.entity_count, 1);
        assert_eq!(restored.computed_at_tick, 100);
    }

    #[test]
    fn test_sensor_snapshot_serde() {
        let summary = SensorSummary::new();
        let snapshot = SensorSnapshot::new(42, summary, 100);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: SensorSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id, 42);
        assert_eq!(restored.snapshot_tick, 100);
    }

    #[test]
    fn test_empty_summary_metrics() {
        let summary = SensorSummary::new();

        assert!((summary.average_urgency()).abs() < f32::EPSILON);
        assert!((summary.observations_per_entity()).abs() < f32::EPSILON);
        assert!(!summary.has_threats());
    }

    #[test]
    fn test_from_suites() {
        let mut suite1 = SensorSuite::new();
        suite1.add_sensor(SensorKind::Sound, SensorConfig::basic(SensorKind::Sound));
        suite1.memory_mut().remember(make_observation(
            1,
            SensorKind::Sound,
            50.0,
            DetectionStrength::Strong,
        ));

        let mut suite2 = SensorSuite::new();
        suite2.add_sensor(SensorKind::Sight, SensorConfig::basic(SensorKind::Sight));
        suite2.memory_mut().remember(make_observation(
            2,
            SensorKind::Sight,
            30.0,
            DetectionStrength::Weak,
        ));

        let all_suites = [suite1, suite2];
        let summary = SensorSummary::from_suites(all_suites.iter(), 200);

        assert_eq!(summary.entity_count, 2);
        assert_eq!(summary.computed_at_tick, 200);
    }
}

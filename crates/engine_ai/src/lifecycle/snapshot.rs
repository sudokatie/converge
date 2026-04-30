//! Lifecycle snapshot and summary types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::state::{GrowthPhase, LifecycleStage};
use super::tracker::LifecycleTracker;

/// Full snapshot of lifecycle state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LifecycleSnapshot {
    pub tick: u64,
    pub total_creatures: u32,
    pub egg_count: u32,
    pub living_count: u32,
    pub metamorphosis_count: u32,
    pub corpse_count: u32,
    pub living_by_phase: BTreeMap<String, u32>,
    pub total_biomass: f32,
    pub average_health: f32,
}

impl LifecycleSnapshot {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_tracker(tracker: &LifecycleTracker, tick: u64) -> Self {
        let mut snapshot = Self::new(tick);

        let mut total_health = 0.0f32;
        let mut health_count = 0u32;

        for id in tracker.all_ids() {
            if let Some(stage) = tracker.get_stage(id) {
                snapshot.total_creatures += 1;

                match stage {
                    LifecycleStage::Egg(_) => {
                        snapshot.egg_count += 1;
                    }
                    LifecycleStage::Living(state) => {
                        snapshot.living_count += 1;
                        let phase_name = state.phase.to_string();
                        *snapshot.living_by_phase.entry(phase_name).or_insert(0) += 1;
                        total_health += state.health;
                        health_count += 1;
                    }
                    LifecycleStage::Metamorphosis(state) => {
                        snapshot.metamorphosis_count += 1;
                        total_health += state.health;
                        health_count += 1;
                    }
                    LifecycleStage::Corpse(state) => {
                        snapshot.corpse_count += 1;
                        snapshot.total_biomass += state.remaining_biomass;
                    }
                }
            }
        }

        if health_count > 0 {
            #[expect(clippy::cast_precision_loss, reason = "health_count bounded")]
            let count_f32 = health_count as f32;
            snapshot.average_health = total_health / count_f32;
        }

        snapshot
    }

    #[must_use]
    pub fn alive_count(&self) -> u32 {
        self.living_count + self.metamorphosis_count
    }

    #[must_use]
    pub fn phase_count(&self, phase: GrowthPhase) -> u32 {
        self.living_by_phase
            .get(&phase.to_string())
            .copied()
            .unwrap_or(0)
    }
}

/// Lightweight summary for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LifecycleSummary {
    pub tick: u64,
    pub total_count: u32,
    pub alive_count: u32,
    pub egg_count: u32,
    pub corpse_count: u32,
    pub average_health: f32,
}

impl LifecycleSummary {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_tracker(tracker: &LifecycleTracker, tick: u64) -> Self {
        let snapshot = LifecycleSnapshot::from_tracker(tracker, tick);
        Self::from(&snapshot)
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded in practice")]
    pub fn population_ratio(&self) -> f32 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.alive_count as f32 / self.total_count as f32
    }
}

impl From<&LifecycleSnapshot> for LifecycleSummary {
    fn from(snapshot: &LifecycleSnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            total_count: snapshot.total_creatures,
            alive_count: snapshot.alive_count(),
            egg_count: snapshot.egg_count,
            corpse_count: snapshot.corpse_count,
            average_health: snapshot.average_health,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{LifecycleConfig, LifecycleId};

    #[test]
    fn test_snapshot_new() {
        let snapshot = LifecycleSnapshot::new(100);
        assert_eq!(snapshot.tick, 100);
        assert_eq!(snapshot.total_creatures, 0);
    }

    #[test]
    fn test_snapshot_from_empty_tracker() {
        let tracker = LifecycleTracker::new(LifecycleConfig::standard());
        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 0);

        assert_eq!(snapshot.total_creatures, 0);
        assert_eq!(snapshot.egg_count, 0);
        assert_eq!(snapshot.living_count, 0);
        assert_eq!(snapshot.corpse_count, 0);
    }

    #[test]
    fn test_snapshot_from_tracker_with_creatures() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_egg(LifecycleId::new(1), 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);
        tracker.spawn_living(LifecycleId::new(3), GrowthPhase::Juvenile, 0);
        tracker.spawn_corpse(LifecycleId::new(4), 50.0, 0);

        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 0);

        assert_eq!(snapshot.total_creatures, 4);
        assert_eq!(snapshot.egg_count, 1);
        assert_eq!(snapshot.living_count, 2);
        assert_eq!(snapshot.corpse_count, 1);
        assert_eq!(snapshot.alive_count(), 2);
    }

    #[test]
    fn test_snapshot_phase_count() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_living(LifecycleId::new(1), GrowthPhase::Adult, 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);
        tracker.spawn_living(LifecycleId::new(3), GrowthPhase::Juvenile, 0);

        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 0);

        assert_eq!(snapshot.phase_count(GrowthPhase::Adult), 2);
        assert_eq!(snapshot.phase_count(GrowthPhase::Juvenile), 1);
        assert_eq!(snapshot.phase_count(GrowthPhase::Elder), 0);
    }

    #[test]
    fn test_snapshot_average_health() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_living(LifecycleId::new(1), GrowthPhase::Adult, 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);

        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 0);
        assert!((snapshot.average_health - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_summary_new() {
        let summary = LifecycleSummary::new(50);
        assert_eq!(summary.tick, 50);
        assert_eq!(summary.total_count, 0);
    }

    #[test]
    fn test_summary_from_tracker() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_egg(LifecycleId::new(1), 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);
        tracker.spawn_corpse(LifecycleId::new(3), 100.0, 0);

        let summary = LifecycleSummary::from_tracker(&tracker, 0);

        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.alive_count, 1);
        assert_eq!(summary.egg_count, 1);
        assert_eq!(summary.corpse_count, 1);
    }

    #[test]
    fn test_summary_from_snapshot() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_living(LifecycleId::new(1), GrowthPhase::Adult, 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Juvenile, 0);

        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 100);
        let summary = LifecycleSummary::from(&snapshot);

        assert_eq!(summary.tick, 100);
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.alive_count, 2);
    }

    #[test]
    fn test_summary_population_ratio() {
        let mut summary = LifecycleSummary::new(0);
        summary.total_count = 10;
        summary.alive_count = 5;

        assert!((summary.population_ratio() - 0.5).abs() < f32::EPSILON);

        let empty_summary = LifecycleSummary::new(0);
        assert!((empty_summary.population_ratio()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_snapshot_serde() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);
        tracker.spawn_living(LifecycleId::new(1), GrowthPhase::Adult, 0);

        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 100);
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: LifecycleSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 100);
        assert_eq!(restored.living_count, 1);
    }

    #[test]
    fn test_summary_serde() {
        let summary = LifecycleSummary {
            tick: 200,
            total_count: 10,
            alive_count: 5,
            egg_count: 2,
            corpse_count: 3,
            average_health: 0.75,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: LifecycleSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 200);
        assert_eq!(restored.total_count, 10);
    }
}

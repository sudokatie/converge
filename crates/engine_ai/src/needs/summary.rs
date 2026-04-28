//! Colony-level aggregation and snapshots for unloaded chunks.

use super::{NeedId, NeedSet, NeedState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Histogram bucket for need value distribution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedHistogram {
    /// Count of needs in critical state (0-10%).
    pub critical: u32,
    /// Count of needs in low state (10-30%).
    pub low: u32,
    /// Count of needs in normal state (30-80%).
    pub normal: u32,
    /// Count of needs in satisfied state (80-100%).
    pub satisfied: u32,
}

impl NeedHistogram {
    /// Create a new empty histogram.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a need state to the histogram.
    pub fn add(&mut self, state: NeedState) {
        match state {
            NeedState::Critical => self.critical += 1,
            NeedState::Low => self.low += 1,
            NeedState::Normal => self.normal += 1,
            NeedState::Satisfied => self.satisfied += 1,
        }
    }

    /// Get total count.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.critical + self.low + self.normal + self.satisfied
    }

    /// Check if any are in distress (critical or low).
    #[must_use]
    pub fn has_distress(&self) -> bool {
        self.critical > 0 || self.low > 0
    }

    /// Get percentage in distress.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn distress_percentage(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        (self.critical + self.low) as f32 / total as f32 * 100.0
    }

    /// Merge another histogram into this one.
    pub fn merge(&mut self, other: &Self) {
        self.critical += other.critical;
        self.low += other.low;
        self.normal += other.normal;
        self.satisfied += other.satisfied;
    }
}

/// Summary statistics for a single need type across a colony.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeedSummary {
    /// Need type identifier.
    pub need_id: NeedId,
    /// Number of creatures with this need.
    pub count: u32,
    /// Minimum value across all creatures.
    pub min: f32,
    /// Maximum value across all creatures.
    pub max: f32,
    /// Sum of all values (for averaging).
    pub sum: f32,
    /// Sum of all urgency scores.
    pub urgency_sum: f32,
    /// State distribution histogram.
    pub histogram: NeedHistogram,
}

impl NeedSummary {
    /// Create a new summary for a need type.
    #[must_use]
    pub fn new(need_id: NeedId) -> Self {
        Self {
            need_id,
            count: 0,
            min: f32::MAX,
            max: f32::MIN,
            sum: 0.0,
            urgency_sum: 0.0,
            histogram: NeedHistogram::new(),
        }
    }

    /// Add a value to the summary.
    pub fn add(&mut self, value: f32, state: NeedState, urgency: f32) {
        self.count += 1;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.urgency_sum += urgency;
        self.histogram.add(state);
    }

    /// Get the average value.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn average(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f32
        }
    }

    /// Get the average urgency.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn average_urgency(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.urgency_sum / self.count as f32
        }
    }

    /// Check if this need type is in distress for any creature.
    #[must_use]
    pub fn has_distress(&self) -> bool {
        self.histogram.has_distress()
    }

    /// Merge another summary into this one.
    pub fn merge(&mut self, other: &Self) {
        self.count += other.count;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.sum += other.sum;
        self.urgency_sum += other.urgency_sum;
        self.histogram.merge(&other.histogram);
    }
}

/// Aggregated summary of all needs across a colony.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColonySummary {
    /// Number of creatures in the colony.
    pub creature_count: u32,
    /// Per-need summaries.
    summaries: BTreeMap<NeedId, NeedSummary>,
    /// Total urgency across all creatures and needs.
    pub total_urgency: f32,
    /// Tick when this summary was computed.
    pub computed_at_tick: u64,
}

impl ColonySummary {
    /// Create a new empty colony summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a creature's need set to the summary.
    pub fn add_creature(&mut self, needs: &NeedSet) {
        self.creature_count += 1;

        for need in needs.iter() {
            let summary = self
                .summaries
                .entry(need.id.clone())
                .or_insert_with(|| NeedSummary::new(need.id.clone()));

            summary.add(need.value(), need.state(), need.urgency());
            self.total_urgency += need.urgency();
        }
    }

    /// Set the tick when computed.
    pub fn set_tick(&mut self, tick: u64) {
        self.computed_at_tick = tick;
    }

    /// Get summary for a specific need type.
    #[must_use]
    pub fn get_need_summary(&self, need_id: &NeedId) -> Option<&NeedSummary> {
        self.summaries.get(need_id)
    }

    /// Iterate over all need summaries.
    pub fn need_summaries(&self) -> impl Iterator<Item = &NeedSummary> {
        self.summaries.values()
    }

    /// Get the need type with the highest average urgency.
    #[must_use]
    pub fn most_urgent_need(&self) -> Option<&NeedId> {
        self.summaries
            .values()
            .max_by(|a, b| {
                a.average_urgency()
                    .partial_cmp(&b.average_urgency())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| &s.need_id)
    }

    /// Check if any creature has any critical need.
    #[must_use]
    pub fn has_critical(&self) -> bool {
        self.summaries.values().any(|s| s.histogram.critical > 0)
    }

    /// Check if any creature has any low or critical need.
    #[must_use]
    pub fn has_distress(&self) -> bool {
        self.summaries.values().any(NeedSummary::has_distress)
    }

    /// Get overall colony wellness score (0.0 = terrible, 1.0 = perfect).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "count precision loss acceptable"
    )]
    pub fn wellness_score(&self) -> f32 {
        if self.creature_count == 0 {
            return 1.0;
        }

        let mut total_satisfied = 0u32;
        let mut total_needs = 0u32;

        for summary in self.summaries.values() {
            total_satisfied += summary.histogram.satisfied + summary.histogram.normal;
            total_needs += summary.histogram.total();
        }

        if total_needs == 0 {
            return 1.0;
        }

        total_satisfied as f32 / total_needs as f32
    }

    /// Get number of creatures in distress (any low or critical need).
    #[must_use]
    pub fn creatures_in_distress(&self) -> u32 {
        let mut max_distress = 0u32;
        for summary in self.summaries.values() {
            max_distress = max_distress.max(summary.histogram.critical + summary.histogram.low);
        }
        max_distress
    }

    /// Merge another colony summary into this one.
    pub fn merge(&mut self, other: &Self) {
        self.creature_count += other.creature_count;
        self.total_urgency += other.total_urgency;

        for (id, other_summary) in &other.summaries {
            self.summaries
                .entry(id.clone())
                .or_insert_with(|| NeedSummary::new(id.clone()))
                .merge(other_summary);
        }
    }

    /// Create from an iterator of need sets.
    pub fn from_need_sets<'a>(need_sets: impl Iterator<Item = &'a NeedSet>, tick: u64) -> Self {
        let mut summary = Self::new();
        for needs in need_sets {
            summary.add_creature(needs);
        }
        summary.set_tick(tick);
        summary
    }
}

/// A snapshot of colony state suitable for persistence or unloaded chunk simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColonySnapshot {
    /// Summary of needs across the colony.
    pub summary: ColonySummary,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
    /// Colony identifier.
    pub colony_id: u64,
    /// Number of ticks to fast-forward per real tick when unloaded.
    pub time_acceleration: f32,
    /// Estimated ticks until next creature enters critical state.
    pub ticks_until_critical: Option<u64>,
}

impl ColonySnapshot {
    /// Create a new colony snapshot.
    #[must_use]
    pub fn new(colony_id: u64, summary: ColonySummary, tick: u64) -> Self {
        let ticks_until_critical = Self::estimate_ticks_until_critical(&summary);

        Self {
            summary,
            snapshot_tick: tick,
            colony_id,
            time_acceleration: 1.0,
            ticks_until_critical,
        }
    }

    /// Estimate ticks until any creature might become critical based on averages.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "estimation bounded by avg check"
    )]
    fn estimate_ticks_until_critical(summary: &ColonySummary) -> Option<u64> {
        if summary.has_critical() {
            return Some(0);
        }

        let mut min_ticks = u64::MAX;

        for need_summary in summary.summaries.values() {
            if need_summary.count == 0 {
                continue;
            }

            let avg = need_summary.average();
            if avg <= 10.0 {
                return Some(0);
            }

            let ticks = (avg - 10.0) as u64;
            min_ticks = min_ticks.min(ticks);
        }

        if min_ticks == u64::MAX {
            None
        } else {
            Some(min_ticks)
        }
    }

    /// Check if the snapshot is stale and needs recomputing.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Check if intervention might be needed soon.
    #[must_use]
    pub fn needs_attention(&self, tick_threshold: u64) -> bool {
        self.summary.has_critical()
            || self
                .ticks_until_critical
                .is_some_and(|t| t < tick_threshold)
    }

    /// Get the age of this snapshot in ticks.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::needs::Need;

    #[test]
    fn test_histogram_add() {
        let mut hist = NeedHistogram::new();

        hist.add(NeedState::Critical);
        hist.add(NeedState::Low);
        hist.add(NeedState::Normal);
        hist.add(NeedState::Satisfied);
        hist.add(NeedState::Satisfied);

        assert_eq!(hist.critical, 1);
        assert_eq!(hist.low, 1);
        assert_eq!(hist.normal, 1);
        assert_eq!(hist.satisfied, 2);
        assert_eq!(hist.total(), 5);
    }

    #[test]
    fn test_histogram_distress() {
        let mut hist = NeedHistogram::new();
        hist.add(NeedState::Satisfied);
        hist.add(NeedState::Normal);

        assert!(!hist.has_distress());

        hist.add(NeedState::Low);
        assert!(hist.has_distress());
    }

    #[test]
    fn test_histogram_distress_percentage() {
        let mut hist = NeedHistogram::new();
        hist.add(NeedState::Critical);
        hist.add(NeedState::Low);
        hist.add(NeedState::Normal);
        hist.add(NeedState::Satisfied);

        assert!((hist.distress_percentage() - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_summary_add() {
        let mut summary = NeedSummary::new(NeedId::hunger());

        summary.add(80.0, NeedState::Satisfied, 0.2);
        summary.add(40.0, NeedState::Normal, 0.6);
        summary.add(5.0, NeedState::Critical, 9.5);

        assert_eq!(summary.count, 3);
        assert!((summary.min - 5.0).abs() < f32::EPSILON);
        assert!((summary.max - 80.0).abs() < f32::EPSILON);
        assert!((summary.average() - 41.666_668).abs() < 0.001);
    }

    #[test]
    fn test_colony_summary_add_creature() {
        let mut summary = ColonySummary::new();

        let mut needs1 = NeedSet::new();
        needs1.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        needs1.add(Need::new(NeedId::thirst(), 100.0, 1.0));

        let mut needs2 = NeedSet::new();
        let mut hunger = Need::new(NeedId::hunger(), 100.0, 1.0);
        hunger.set_value(20.0);
        needs2.add(hunger);
        needs2.add(Need::new(NeedId::thirst(), 100.0, 1.0));

        summary.add_creature(&needs1);
        summary.add_creature(&needs2);

        assert_eq!(summary.creature_count, 2);

        let hunger_summary = summary.get_need_summary(&NeedId::hunger()).unwrap();
        assert_eq!(hunger_summary.count, 2);
        assert!((hunger_summary.min - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_colony_summary_wellness() {
        let mut summary = ColonySummary::new();

        let mut needs = NeedSet::new();
        needs.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        needs.add(Need::new(NeedId::thirst(), 100.0, 1.0));

        summary.add_creature(&needs);

        assert!((summary.wellness_score() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_colony_summary_merge() {
        let mut summary1 = ColonySummary::new();
        let mut needs1 = NeedSet::new();
        needs1.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        summary1.add_creature(&needs1);

        let mut summary2 = ColonySummary::new();
        let mut needs2 = NeedSet::new();
        needs2.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        summary2.add_creature(&needs2);

        summary1.merge(&summary2);

        assert_eq!(summary1.creature_count, 2);
        let hunger = summary1.get_need_summary(&NeedId::hunger()).unwrap();
        assert_eq!(hunger.count, 2);
    }

    #[test]
    fn test_colony_snapshot_new() {
        let mut summary = ColonySummary::new();
        let mut needs = NeedSet::new();
        needs.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        summary.add_creature(&needs);

        let snapshot = ColonySnapshot::new(1, summary, 100);

        assert_eq!(snapshot.colony_id, 1);
        assert_eq!(snapshot.snapshot_tick, 100);
    }

    #[test]
    fn test_colony_snapshot_staleness() {
        let summary = ColonySummary::new();
        let snapshot = ColonySnapshot::new(1, summary, 100);

        assert!(!snapshot.is_stale(150, 100));
        assert!(snapshot.is_stale(250, 100));
    }

    #[test]
    fn test_colony_snapshot_needs_attention() {
        let mut summary = ColonySummary::new();
        let mut needs = NeedSet::new();
        let mut hunger = Need::new(NeedId::hunger(), 100.0, 1.0);
        hunger.set_value(5.0);
        needs.add(hunger);
        summary.add_creature(&needs);

        let snapshot = ColonySnapshot::new(1, summary, 100);

        assert!(snapshot.needs_attention(1000));
    }

    #[test]
    fn test_colony_summary_from_need_sets() {
        let mut needs1 = NeedSet::new();
        needs1.add(Need::new(NeedId::hunger(), 100.0, 1.0));

        let mut needs2 = NeedSet::new();
        needs2.add(Need::new(NeedId::hunger(), 100.0, 1.0));

        let need_sets = [needs1, needs2];
        let summary = ColonySummary::from_need_sets(need_sets.iter(), 100);

        assert_eq!(summary.creature_count, 2);
        assert_eq!(summary.computed_at_tick, 100);
    }

    #[test]
    fn test_serde_colony_summary() {
        let mut summary = ColonySummary::new();
        let mut needs = NeedSet::new();
        needs.add(Need::new(NeedId::hunger(), 100.0, 1.0));
        summary.add_creature(&needs);
        summary.set_tick(50);

        let json = serde_json::to_string(&summary).unwrap();
        let restored: ColonySummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.creature_count, 1);
        assert_eq!(restored.computed_at_tick, 50);
    }

    #[test]
    fn test_serde_colony_snapshot() {
        let summary = ColonySummary::new();
        let snapshot = ColonySnapshot::new(42, summary, 100);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: ColonySnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.colony_id, 42);
        assert_eq!(restored.snapshot_tick, 100);
    }
}

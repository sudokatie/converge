//! Player competence signal tracking.

use super::ids::CompetenceSignalId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A recorded competence signal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompetenceSignal {
    /// Signal type identifier.
    pub signal_id: CompetenceSignalId,
    /// Signal value (0.0 = low competence, 1.0 = high competence).
    pub value: f32,
    /// Tick when signal was recorded.
    pub tick: u64,
    /// Weight for this signal type.
    pub weight: f32,
}

impl CompetenceSignal {
    #[must_use]
    pub fn new(signal_id: impl Into<CompetenceSignalId>, value: f32, tick: u64) -> Self {
        Self {
            signal_id: signal_id.into(),
            value: value.clamp(0.0, 1.0),
            tick,
            weight: 1.0,
        }
    }

    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.max(0.0);
        self
    }

    #[must_use]
    pub fn weighted_value(&self) -> f32 {
        self.value * self.weight
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.tick)
    }

    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_age: u64) -> bool {
        self.age(current_tick) > max_age
    }
}

/// Categories of competence signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompetenceCategory {
    /// Task completion rate and efficiency.
    TaskEfficiency,
    /// Resource management effectiveness.
    ResourceManagement,
    /// Disaster response and recovery.
    DisasterResponse,
    /// Colony growth and expansion.
    ColonyGrowth,
    /// Combat effectiveness.
    Combat,
    /// Building and construction.
    Construction,
    /// Research and technology.
    Research,
}

impl CompetenceCategory {
    #[must_use]
    pub fn default_weight(self) -> f32 {
        match self {
            Self::TaskEfficiency | Self::Combat => 1.0,
            Self::ResourceManagement => 1.2,
            Self::DisasterResponse => 1.5,
            Self::ColonyGrowth => 0.8,
            Self::Construction => 0.9,
            Self::Research => 0.7,
        }
    }
}

/// Configuration for competence tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompetenceConfig {
    /// Maximum signals to retain per category.
    pub max_signals_per_category: usize,
    /// Signal age after which it's considered stale.
    pub signal_staleness_ticks: u64,
    /// Decay factor for older signals (0.0 to 1.0).
    pub age_decay_factor: f32,
    /// Category weights.
    pub category_weights: BTreeMap<CompetenceCategory, f32>,
}

impl CompetenceConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_signals(mut self, max: usize) -> Self {
        self.max_signals_per_category = max.max(1);
        self
    }

    #[must_use]
    pub fn with_staleness_ticks(mut self, ticks: u64) -> Self {
        self.signal_staleness_ticks = ticks.max(1);
        self
    }

    #[must_use]
    pub fn with_age_decay(mut self, factor: f32) -> Self {
        self.age_decay_factor = factor.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_category_weight(mut self, category: CompetenceCategory, weight: f32) -> Self {
        self.category_weights.insert(category, weight.max(0.0));
        self
    }

    #[must_use]
    pub fn category_weight(&self, category: CompetenceCategory) -> f32 {
        self.category_weights
            .get(&category)
            .copied()
            .unwrap_or_else(|| category.default_weight())
    }
}

impl Default for CompetenceConfig {
    fn default() -> Self {
        Self {
            max_signals_per_category: 20,
            signal_staleness_ticks: 5000,
            age_decay_factor: 0.1,
            category_weights: BTreeMap::new(),
        }
    }
}

/// Tracker for player competence signals.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompetenceTracker {
    /// Configuration.
    config: CompetenceConfig,
    /// Signals grouped by category.
    signals: BTreeMap<CompetenceCategory, Vec<CompetenceSignal>>,
    /// Running weighted average per category.
    averages: BTreeMap<CompetenceCategory, f32>,
    /// Overall competence score.
    overall_score: f32,
    /// Last update tick.
    last_update_tick: u64,
}

impl CompetenceTracker {
    #[must_use]
    pub fn new(config: CompetenceConfig) -> Self {
        Self {
            config,
            signals: BTreeMap::new(),
            averages: BTreeMap::new(),
            overall_score: 0.5,
            last_update_tick: 0,
        }
    }

    #[must_use]
    pub fn config(&self) -> &CompetenceConfig {
        &self.config
    }

    #[must_use]
    pub fn overall_score(&self) -> f32 {
        self.overall_score
    }

    #[must_use]
    pub fn category_score(&self, category: CompetenceCategory) -> f32 {
        self.averages.get(&category).copied().unwrap_or(0.5)
    }

    #[must_use]
    pub fn last_update_tick(&self) -> u64 {
        self.last_update_tick
    }

    #[must_use]
    pub fn signal_count(&self) -> usize {
        self.signals.values().map(Vec::len).sum()
    }

    #[must_use]
    pub fn category_signal_count(&self, category: CompetenceCategory) -> usize {
        self.signals.get(&category).map_or(0, Vec::len)
    }

    pub fn record_signal(&mut self, category: CompetenceCategory, signal: CompetenceSignal) {
        let signals = self.signals.entry(category).or_default();
        signals.push(signal);

        if signals.len() > self.config.max_signals_per_category {
            signals.remove(0);
        }
    }

    pub fn record_value(
        &mut self,
        category: CompetenceCategory,
        signal_id: impl Into<CompetenceSignalId>,
        value: f32,
        tick: u64,
    ) {
        let signal = CompetenceSignal::new(signal_id, value, tick);
        self.record_signal(category, signal);
    }

    pub fn cleanup_stale(&mut self, current_tick: u64) {
        let staleness = self.config.signal_staleness_ticks;
        for signals in self.signals.values_mut() {
            signals.retain(|s| !s.is_stale(current_tick, staleness));
        }
    }

    pub fn update(&mut self, current_tick: u64) {
        self.cleanup_stale(current_tick);
        self.update_averages(current_tick);
        self.update_overall();
        self.last_update_tick = current_tick;
    }

    fn update_averages(&mut self, current_tick: u64) {
        let decay_factor = self.config.age_decay_factor;
        let staleness = self.config.signal_staleness_ticks;

        for (category, signals) in &self.signals {
            if signals.is_empty() {
                continue;
            }

            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;

            for signal in signals {
                let age = signal.age(current_tick);
                #[expect(clippy::cast_precision_loss, reason = "tick values bounded")]
                let age_weight = 1.0 - (age as f32 / staleness as f32) * decay_factor;
                let weight = signal.weight * age_weight.max(0.0);

                weighted_sum += signal.value * weight;
                total_weight += weight;
            }

            let average = if total_weight > 0.0 {
                weighted_sum / total_weight
            } else {
                0.5
            };

            self.averages.insert(*category, average);
        }
    }

    fn update_overall(&mut self) {
        if self.averages.is_empty() {
            self.overall_score = 0.5;
            return;
        }

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (category, score) in &self.averages {
            let weight = self.config.category_weight(*category);
            weighted_sum += score * weight;
            total_weight += weight;
        }

        self.overall_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.5
        };
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.overall_score.to_le_bytes());
        hasher.update(&self.last_update_tick.to_le_bytes());
        #[expect(clippy::cast_possible_truncation, reason = "count bounded")]
        {
            hasher.update(&(self.signal_count() as u32).to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Summary of competence state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompetenceSummary {
    pub tick: u64,
    pub overall_score: f32,
    pub category_scores: BTreeMap<CompetenceCategory, f32>,
    pub total_signals: usize,
    pub trend: CompetenceTrend,
}

impl CompetenceSummary {
    #[must_use]
    pub fn from_tracker(tracker: &CompetenceTracker) -> Self {
        Self {
            tick: tracker.last_update_tick,
            overall_score: tracker.overall_score,
            category_scores: tracker.averages.clone(),
            total_signals: tracker.signal_count(),
            trend: CompetenceTrend::Stable,
        }
    }

    #[must_use]
    pub fn is_struggling(&self) -> bool {
        self.overall_score < 0.3
    }

    #[must_use]
    pub fn is_proficient(&self) -> bool {
        self.overall_score > 0.7
    }
}

/// Trend in competence over time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompetenceTrend {
    Improving,
    #[default]
    Stable,
    Declining,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_competence_signal_new() {
        let signal = CompetenceSignal::new("task_complete", 0.8, 100);

        assert_eq!(signal.signal_id.as_str(), "task_complete");
        assert!((signal.value - 0.8).abs() < f32::EPSILON);
        assert_eq!(signal.tick, 100);
        assert!((signal.weight - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_competence_signal_clamping() {
        let signal = CompetenceSignal::new("test", 1.5, 0);
        assert!((signal.value - 1.0).abs() < f32::EPSILON);

        let signal = CompetenceSignal::new("test", -0.5, 0);
        assert!(signal.value.abs() < f32::EPSILON);
    }

    #[test]
    fn test_competence_signal_age() {
        let signal = CompetenceSignal::new("test", 0.5, 100);

        assert_eq!(signal.age(150), 50);
        assert_eq!(signal.age(100), 0);
        assert_eq!(signal.age(50), 0);
    }

    #[test]
    fn test_competence_signal_staleness() {
        let signal = CompetenceSignal::new("test", 0.5, 100);

        assert!(!signal.is_stale(150, 100));
        assert!(signal.is_stale(250, 100));
    }

    #[test]
    fn test_competence_tracker_new() {
        let tracker = CompetenceTracker::new(CompetenceConfig::new());

        assert!((tracker.overall_score() - 0.5).abs() < f32::EPSILON);
        assert_eq!(tracker.signal_count(), 0);
    }

    #[test]
    fn test_competence_tracker_record() {
        let mut tracker = CompetenceTracker::new(CompetenceConfig::new());

        tracker.record_value(CompetenceCategory::TaskEfficiency, "complete", 0.9, 100);
        tracker.record_value(CompetenceCategory::TaskEfficiency, "speed", 0.7, 100);

        assert_eq!(tracker.signal_count(), 2);
        assert_eq!(
            tracker.category_signal_count(CompetenceCategory::TaskEfficiency),
            2
        );
    }

    #[test]
    fn test_competence_tracker_update() {
        let mut tracker = CompetenceTracker::new(CompetenceConfig::new());

        tracker.record_value(CompetenceCategory::TaskEfficiency, "a", 0.8, 100);
        tracker.record_value(CompetenceCategory::ResourceManagement, "b", 0.6, 100);

        tracker.update(100);

        assert!(tracker.overall_score() > 0.0);
        assert!(tracker.overall_score() < 1.0);
        assert!(tracker.category_score(CompetenceCategory::TaskEfficiency) > 0.5);
    }

    #[test]
    fn test_competence_tracker_cleanup() {
        let config = CompetenceConfig::new().with_staleness_ticks(100);
        let mut tracker = CompetenceTracker::new(config);

        tracker.record_value(CompetenceCategory::TaskEfficiency, "old", 0.5, 0);
        tracker.record_value(CompetenceCategory::TaskEfficiency, "new", 0.5, 200);

        assert_eq!(tracker.signal_count(), 2);

        tracker.cleanup_stale(250);

        assert_eq!(tracker.signal_count(), 1);
    }

    #[test]
    fn test_competence_summary() {
        let mut tracker = CompetenceTracker::new(CompetenceConfig::new());
        tracker.record_value(CompetenceCategory::TaskEfficiency, "test", 0.8, 100);
        tracker.update(100);

        let summary = CompetenceSummary::from_tracker(&tracker);

        assert!(!summary.is_struggling());
        assert_eq!(summary.total_signals, 1);
    }

    #[test]
    fn test_competence_category_default_weight() {
        assert!((CompetenceCategory::DisasterResponse.default_weight() - 1.5).abs() < f32::EPSILON);
        assert!((CompetenceCategory::TaskEfficiency.default_weight() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_competence_config_builder() {
        let config = CompetenceConfig::new()
            .with_max_signals(50)
            .with_staleness_ticks(1000)
            .with_category_weight(CompetenceCategory::Combat, 2.0);

        assert_eq!(config.max_signals_per_category, 50);
        assert_eq!(config.signal_staleness_ticks, 1000);
        assert!((config.category_weight(CompetenceCategory::Combat) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_competence_signal() {
        let signal = CompetenceSignal::new("test", 0.75, 500).with_weight(1.5);

        let json = serde_json::to_string(&signal).unwrap();
        let restored: CompetenceSignal = serde_json::from_str(&json).unwrap();

        assert!((restored.value - 0.75).abs() < f32::EPSILON);
        assert!((restored.weight - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_competence_tracker() {
        let mut tracker = CompetenceTracker::new(CompetenceConfig::new());
        tracker.record_value(CompetenceCategory::TaskEfficiency, "test", 0.8, 100);
        tracker.update(100);

        let bytes = bincode::serialize(&tracker).unwrap();
        let restored: CompetenceTracker = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum(), tracker.checksum());
        assert_eq!(restored.signal_count(), 1);
    }

    #[test]
    fn test_bincode_competence_summary() {
        let summary = CompetenceSummary {
            overall_score: 0.65,
            total_signals: 10,
            ..Default::default()
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: CompetenceSummary = bincode::deserialize(&bytes).unwrap();

        assert!((restored.overall_score - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn test_checksum_consistency() {
        let mut tracker1 = CompetenceTracker::new(CompetenceConfig::new());
        let mut tracker2 = CompetenceTracker::new(CompetenceConfig::new());

        tracker1.record_value(CompetenceCategory::TaskEfficiency, "test", 0.8, 100);
        tracker2.record_value(CompetenceCategory::TaskEfficiency, "test", 0.8, 100);

        tracker1.update(100);
        tracker2.update(100);

        assert_eq!(tracker1.checksum(), tracker2.checksum());
    }
}

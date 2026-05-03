//! Disaster history and recent-event memory for director pacing.

use super::ids::DisasterId;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Category of disaster event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DisasterCategory {
    /// Environmental disasters (storms, earthquakes, etc.).
    Environmental,
    /// Resource depletion or shortage.
    ResourceCrisis,
    /// Disease outbreak.
    Disease,
    /// Hostile attack.
    Attack,
    /// Equipment or system failure.
    SystemFailure,
    /// Social unrest or panic.
    SocialCrisis,
    /// Fire or explosion.
    Fire,
    /// Structural collapse.
    Structural,
}

impl DisasterCategory {
    #[must_use]
    pub fn severity_multiplier(self) -> f32 {
        match self {
            Self::Environmental => 1.2,
            Self::ResourceCrisis => 1.0,
            Self::Disease => 1.5,
            Self::Attack => 1.4,
            Self::SystemFailure => 0.8,
            Self::SocialCrisis => 1.1,
            Self::Fire => 1.3,
            Self::Structural => 1.6,
        }
    }

    #[must_use]
    pub fn recovery_time_multiplier(self) -> f32 {
        match self {
            Self::Environmental => 1.0,
            Self::ResourceCrisis => 1.5,
            Self::Disease => 2.0,
            Self::Attack => 1.2,
            Self::SystemFailure => 0.5,
            Self::SocialCrisis => 1.8,
            Self::Fire => 0.8,
            Self::Structural => 1.4,
        }
    }
}

/// Severity level of a disaster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DisasterSeverity {
    Minor,
    Moderate,
    Major,
    Catastrophic,
}

impl DisasterSeverity {
    #[must_use]
    pub fn weight(self) -> f32 {
        match self {
            Self::Minor => 0.25,
            Self::Moderate => 0.5,
            Self::Major => 0.75,
            Self::Catastrophic => 1.0,
        }
    }

    #[must_use]
    pub fn grace_period_multiplier(self) -> f32 {
        match self {
            Self::Minor => 0.5,
            Self::Moderate => 1.0,
            Self::Major => 1.5,
            Self::Catastrophic => 2.5,
        }
    }
}

/// Record of a disaster event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisasterRecord {
    pub id: DisasterId,
    pub category: DisasterCategory,
    pub severity: DisasterSeverity,
    /// Tick when disaster started.
    pub start_tick: u64,
    /// Tick when disaster ended (None if ongoing).
    pub end_tick: Option<u64>,
    /// Casualties caused.
    pub casualties: u32,
    /// Resources lost.
    pub resources_lost: u32,
    /// Shelters damaged.
    pub shelters_damaged: u32,
    /// Description of the disaster.
    pub description: String,
}

impl DisasterRecord {
    #[must_use]
    pub fn new(
        id: DisasterId,
        category: DisasterCategory,
        severity: DisasterSeverity,
        start_tick: u64,
    ) -> Self {
        Self {
            id,
            category,
            severity,
            start_tick,
            end_tick: None,
            casualties: 0,
            resources_lost: 0,
            shelters_damaged: 0,
            description: String::new(),
        }
    }

    #[must_use]
    pub fn with_casualties(mut self, casualties: u32) -> Self {
        self.casualties = casualties;
        self
    }

    #[must_use]
    pub fn with_resources_lost(mut self, resources: u32) -> Self {
        self.resources_lost = resources;
        self
    }

    #[must_use]
    pub fn with_shelters_damaged(mut self, shelters: u32) -> Self {
        self.shelters_damaged = shelters;
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn end(&mut self, tick: u64) {
        self.end_tick = Some(tick);
    }

    #[must_use]
    pub fn is_ongoing(&self) -> bool {
        self.end_tick.is_none()
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> u64 {
        self.end_tick
            .unwrap_or(current_tick)
            .saturating_sub(self.start_tick)
    }

    #[must_use]
    pub fn ticks_since_end(&self, current_tick: u64) -> Option<u64> {
        self.end_tick.map(|end| current_tick.saturating_sub(end))
    }

    #[must_use]
    pub fn impact_score(&self) -> f32 {
        let base = self.severity.weight() * self.category.severity_multiplier();
        #[expect(clippy::cast_precision_loss, reason = "bounded values")]
        {
            let casualty_factor = 1.0 + (self.casualties as f32 * 0.1).min(0.5);
            let resource_factor = 1.0 + (self.resources_lost as f32 * 0.001).min(0.3);
            let shelter_factor = 1.0 + (self.shelters_damaged as f32 * 0.05).min(0.2);
            base * casualty_factor * resource_factor * shelter_factor
        }
    }

    #[must_use]
    pub fn recency_weight(&self, current_tick: u64, decay_ticks: u64) -> f32 {
        let elapsed = match self.end_tick {
            Some(end) => current_tick.saturating_sub(end),
            None => 0,
        };

        if elapsed == 0 {
            return 1.0;
        }

        #[expect(clippy::cast_precision_loss, reason = "tick values bounded")]
        {
            (1.0 - (elapsed as f32 / decay_ticks as f32)).max(0.0)
        }
    }
}

/// Configuration for disaster history tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisasterHistoryConfig {
    /// Maximum disasters to retain.
    pub max_records: usize,
    /// Ticks for disaster memory decay.
    pub memory_decay_ticks: u64,
    /// Ticks to consider a disaster "recent".
    pub recent_threshold_ticks: u64,
    /// Base grace period after disasters.
    pub base_grace_period: u64,
}

impl DisasterHistoryConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_records(mut self, max: usize) -> Self {
        self.max_records = max.max(1);
        self
    }

    #[must_use]
    pub fn with_memory_decay(mut self, ticks: u64) -> Self {
        self.memory_decay_ticks = ticks.max(1);
        self
    }

    #[must_use]
    pub fn with_recent_threshold(mut self, ticks: u64) -> Self {
        self.recent_threshold_ticks = ticks;
        self
    }

    #[must_use]
    pub fn with_base_grace_period(mut self, ticks: u64) -> Self {
        self.base_grace_period = ticks;
        self
    }
}

impl Default for DisasterHistoryConfig {
    fn default() -> Self {
        Self {
            max_records: 50,
            memory_decay_ticks: 3000,
            recent_threshold_ticks: 1000,
            base_grace_period: 500,
        }
    }
}

/// Tracker for disaster history.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DisasterHistory {
    config: DisasterHistoryConfig,
    records: VecDeque<DisasterRecord>,
    next_id: u64,
    total_disasters: u64,
    total_casualties: u64,
}

impl DisasterHistory {
    #[must_use]
    pub fn new(config: DisasterHistoryConfig) -> Self {
        Self {
            config,
            records: VecDeque::new(),
            next_id: 1,
            total_disasters: 0,
            total_casualties: 0,
        }
    }

    #[must_use]
    pub fn config(&self) -> &DisasterHistoryConfig {
        &self.config
    }

    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn total_disasters(&self) -> u64 {
        self.total_disasters
    }

    #[must_use]
    pub fn total_casualties(&self) -> u64 {
        self.total_casualties
    }

    pub fn record_disaster(
        &mut self,
        category: DisasterCategory,
        severity: DisasterSeverity,
        tick: u64,
    ) -> DisasterId {
        let id = DisasterId::new(self.next_id);
        self.next_id += 1;

        let record = DisasterRecord::new(id, category, severity, tick);
        self.records.push_back(record);
        self.total_disasters += 1;

        while self.records.len() > self.config.max_records {
            self.records.pop_front();
        }

        id
    }

    pub fn end_disaster(
        &mut self,
        id: DisasterId,
        tick: u64,
        casualties: u32,
        resources_lost: u32,
        shelters_damaged: u32,
    ) {
        if let Some(record) = self.records.iter_mut().find(|r| r.id == id) {
            record.end(tick);
            record.casualties = casualties;
            record.resources_lost = resources_lost;
            record.shelters_damaged = shelters_damaged;
            self.total_casualties += u64::from(casualties);
        }
    }

    #[must_use]
    pub fn get_disaster(&self, id: DisasterId) -> Option<&DisasterRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DisasterRecord> {
        self.records.iter()
    }

    #[must_use]
    pub fn ongoing_disasters(&self) -> Vec<&DisasterRecord> {
        self.records.iter().filter(|r| r.is_ongoing()).collect()
    }

    #[must_use]
    pub fn recent_disasters(&self, current_tick: u64) -> Vec<&DisasterRecord> {
        let threshold = self.config.recent_threshold_ticks;
        self.records
            .iter()
            .filter(|r| {
                r.is_ongoing()
                    || r.ticks_since_end(current_tick)
                        .is_some_and(|t| t < threshold)
            })
            .collect()
    }

    #[must_use]
    pub fn most_recent_disaster(&self) -> Option<&DisasterRecord> {
        self.records.back()
    }

    #[must_use]
    pub fn disasters_by_category(&self, category: DisasterCategory) -> Vec<&DisasterRecord> {
        self.records
            .iter()
            .filter(|r| r.category == category)
            .collect()
    }

    #[must_use]
    pub fn weighted_recent_impact(&self, current_tick: u64) -> f32 {
        let decay = self.config.memory_decay_ticks;
        self.records
            .iter()
            .map(|r| r.impact_score() * r.recency_weight(current_tick, decay))
            .sum()
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "base_grace_period is small config value; positive bounded result"
    )]
    pub fn recommended_grace_period(&self, current_tick: u64) -> u64 {
        let recent = self.recent_disasters(current_tick);
        if recent.is_empty() {
            return 0;
        }

        let max_severity = recent
            .iter()
            .map(|r| r.severity)
            .max()
            .unwrap_or(DisasterSeverity::Minor);
        let base = self.config.base_grace_period;
        (base as f32 * max_severity.grace_period_multiplier()) as u64
    }

    #[must_use]
    pub fn ticks_since_last_disaster(&self, current_tick: u64) -> Option<u64> {
        self.records
            .back()
            .and_then(|r| r.end_tick.map(|end| current_tick.saturating_sub(end)))
    }

    #[must_use]
    pub fn is_in_grace_period(&self, current_tick: u64) -> bool {
        let grace = self.recommended_grace_period(current_tick);
        if grace == 0 {
            return false;
        }

        self.ticks_since_last_disaster(current_tick)
            .is_some_and(|elapsed| elapsed < grace)
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.next_id.to_le_bytes());
        hasher.update(&self.total_disasters.to_le_bytes());
        hasher.update(&self.total_casualties.to_le_bytes());
        #[expect(clippy::cast_possible_truncation, reason = "count bounded")]
        {
            hasher.update(&(self.records.len() as u32).to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Summary of disaster history state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DisasterHistorySummary {
    pub tick: u64,
    pub total_disasters: u64,
    pub total_casualties: u64,
    pub recent_disaster_count: usize,
    pub ongoing_disaster_count: usize,
    pub weighted_impact: f32,
    pub in_grace_period: bool,
    pub recommended_grace_ticks: u64,
}

impl DisasterHistorySummary {
    #[must_use]
    pub fn from_history(history: &DisasterHistory, current_tick: u64) -> Self {
        Self {
            tick: current_tick,
            total_disasters: history.total_disasters,
            total_casualties: history.total_casualties,
            recent_disaster_count: history.recent_disasters(current_tick).len(),
            ongoing_disaster_count: history.ongoing_disasters().len(),
            weighted_impact: history.weighted_recent_impact(current_tick),
            in_grace_period: history.is_in_grace_period(current_tick),
            recommended_grace_ticks: history.recommended_grace_period(current_tick),
        }
    }

    #[must_use]
    pub fn has_recent_trauma(&self) -> bool {
        self.weighted_impact > 0.5 || self.recent_disaster_count > 0
    }

    #[must_use]
    pub fn is_calm(&self) -> bool {
        self.ongoing_disaster_count == 0 && self.weighted_impact < 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disaster_category_multipliers() {
        assert!(DisasterCategory::Structural.severity_multiplier() > 1.0);
        assert!(DisasterCategory::Disease.recovery_time_multiplier() > 1.0);
    }

    #[test]
    fn test_disaster_severity_weight() {
        assert!((DisasterSeverity::Minor.weight() - 0.25).abs() < f32::EPSILON);
        assert!((DisasterSeverity::Catastrophic.weight() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_disaster_record_new() {
        let record = DisasterRecord::new(
            DisasterId::new(1),
            DisasterCategory::Fire,
            DisasterSeverity::Major,
            100,
        );

        assert!(record.is_ongoing());
        assert_eq!(record.duration(150), 50);
    }

    #[test]
    fn test_disaster_record_end() {
        let mut record = DisasterRecord::new(
            DisasterId::new(1),
            DisasterCategory::Fire,
            DisasterSeverity::Major,
            100,
        )
        .with_casualties(5)
        .with_resources_lost(100);

        record.end(200);

        assert!(!record.is_ongoing());
        assert_eq!(record.duration(300), 100);
        assert_eq!(record.ticks_since_end(250), Some(50));
    }

    #[test]
    fn test_disaster_record_impact_score() {
        let record = DisasterRecord::new(
            DisasterId::new(1),
            DisasterCategory::Attack,
            DisasterSeverity::Major,
            100,
        )
        .with_casualties(10)
        .with_shelters_damaged(2);

        let impact = record.impact_score();
        assert!(impact > 0.0);
    }

    #[test]
    fn test_disaster_record_recency_weight() {
        let mut record = DisasterRecord::new(
            DisasterId::new(1),
            DisasterCategory::Fire,
            DisasterSeverity::Minor,
            100,
        );
        record.end(200);

        assert!((record.recency_weight(200, 1000) - 1.0).abs() < f32::EPSILON);
        assert!(record.recency_weight(700, 1000) < 1.0);
        assert!(record.recency_weight(1300, 1000).abs() < f32::EPSILON);
    }

    #[test]
    fn test_disaster_history_new() {
        let history = DisasterHistory::new(DisasterHistoryConfig::new());

        assert_eq!(history.record_count(), 0);
        assert_eq!(history.total_disasters(), 0);
    }

    #[test]
    fn test_disaster_history_record() {
        let mut history = DisasterHistory::new(DisasterHistoryConfig::new());

        let id = history.record_disaster(DisasterCategory::Fire, DisasterSeverity::Moderate, 100);

        assert_eq!(history.record_count(), 1);
        assert_eq!(history.total_disasters(), 1);
        assert!(history.get_disaster(id).is_some());
    }

    #[test]
    fn test_disaster_history_end() {
        let mut history = DisasterHistory::new(DisasterHistoryConfig::new());

        let id = history.record_disaster(DisasterCategory::Fire, DisasterSeverity::Moderate, 100);
        history.end_disaster(id, 200, 3, 50, 1);

        let record = history.get_disaster(id).unwrap();
        assert!(!record.is_ongoing());
        assert_eq!(record.casualties, 3);
        assert_eq!(history.total_casualties(), 3);
    }

    #[test]
    fn test_disaster_history_recent() {
        let config = DisasterHistoryConfig::new().with_recent_threshold(500);
        let mut history = DisasterHistory::new(config);

        let id1 = history.record_disaster(DisasterCategory::Fire, DisasterSeverity::Minor, 100);
        history.end_disaster(id1, 150, 0, 0, 0);

        let id2 = history.record_disaster(DisasterCategory::Attack, DisasterSeverity::Major, 800);
        history.end_disaster(id2, 850, 5, 100, 2);

        let recent = history.recent_disasters(900);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].category, DisasterCategory::Attack);
    }

    #[test]
    fn test_disaster_history_grace_period() {
        let config = DisasterHistoryConfig::new()
            .with_base_grace_period(100)
            .with_recent_threshold(500);
        let mut history = DisasterHistory::new(config);

        let id = history.record_disaster(
            DisasterCategory::Structural,
            DisasterSeverity::Catastrophic,
            100,
        );
        history.end_disaster(id, 200, 10, 500, 5);

        let grace = history.recommended_grace_period(200);
        assert!(grace > 100);

        assert!(history.is_in_grace_period(220));
        assert!(!history.is_in_grace_period(200 + grace + 100));
    }

    #[test]
    fn test_disaster_history_weighted_impact() {
        let mut history = DisasterHistory::new(DisasterHistoryConfig::new());

        let id = history.record_disaster(DisasterCategory::Fire, DisasterSeverity::Major, 100);
        history.end_disaster(id, 150, 5, 200, 1);

        let impact = history.weighted_recent_impact(200);
        assert!(impact > 0.0);
    }

    #[test]
    fn test_disaster_history_summary() {
        let mut history = DisasterHistory::new(DisasterHistoryConfig::new());

        let id =
            history.record_disaster(DisasterCategory::Disease, DisasterSeverity::Moderate, 100);
        history.end_disaster(id, 500, 2, 0, 0);

        let summary = DisasterHistorySummary::from_history(&history, 600);

        assert_eq!(summary.total_disasters, 1);
        assert_eq!(summary.total_casualties, 2);
        assert!(summary.has_recent_trauma());
    }

    #[test]
    fn test_disaster_history_max_records() {
        let config = DisasterHistoryConfig::new().with_max_records(3);
        let mut history = DisasterHistory::new(config);

        for i in 0..5 {
            history.record_disaster(DisasterCategory::Fire, DisasterSeverity::Minor, i * 100);
        }

        assert_eq!(history.record_count(), 3);
        assert_eq!(history.total_disasters(), 5);
    }

    #[test]
    fn test_serde_disaster_record() {
        let record = DisasterRecord::new(
            DisasterId::new(42),
            DisasterCategory::Attack,
            DisasterSeverity::Major,
            500,
        )
        .with_casualties(10)
        .with_description("Hostile raid");

        let json = serde_json::to_string(&record).unwrap();
        let restored: DisasterRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.raw(), 42);
        assert_eq!(restored.casualties, 10);
        assert_eq!(restored.description, "Hostile raid");
    }

    #[test]
    fn test_bincode_disaster_history() {
        let mut history = DisasterHistory::new(DisasterHistoryConfig::new());

        let id = history.record_disaster(
            DisasterCategory::Environmental,
            DisasterSeverity::Moderate,
            100,
        );
        history.end_disaster(id, 200, 1, 50, 0);

        let bytes = bincode::serialize(&history).unwrap();
        let restored: DisasterHistory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum(), history.checksum());
        assert_eq!(restored.total_disasters(), 1);
    }

    #[test]
    fn test_bincode_disaster_history_summary() {
        let summary = DisasterHistorySummary {
            tick: 1000,
            total_disasters: 5,
            total_casualties: 15,
            recent_disaster_count: 1,
            ongoing_disaster_count: 0,
            weighted_impact: 0.35,
            in_grace_period: true,
            recommended_grace_ticks: 200,
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: DisasterHistorySummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 1000);
        assert!(restored.in_grace_period);
    }

    #[test]
    fn test_checksum_consistency() {
        let mut history1 = DisasterHistory::new(DisasterHistoryConfig::new());
        let mut history2 = DisasterHistory::new(DisasterHistoryConfig::new());

        let id1 = history1.record_disaster(DisasterCategory::Fire, DisasterSeverity::Minor, 100);
        let id2 = history2.record_disaster(DisasterCategory::Fire, DisasterSeverity::Minor, 100);

        history1.end_disaster(id1, 150, 1, 0, 0);
        history2.end_disaster(id2, 150, 1, 0, 0);

        assert_eq!(history1.checksum(), history2.checksum());
    }
}

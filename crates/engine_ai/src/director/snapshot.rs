//! Snapshots of director state.

use super::competence::CompetenceSummary;
use super::disaster::DisasterHistorySummary;
use super::pacing::{PacingLevel, PacingSummary};
use super::pressure::StockpileSummary;
use super::shelter::ShelterQualitySummary;
use serde::{Deserialize, Serialize};

/// Complete snapshot of director state at a point in time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectorSnapshot {
    pub tick: u64,
    /// Pacing state summary.
    pub pacing: PacingSummary,
    /// Competence summary.
    pub competence: CompetenceSummary,
    /// Stockpile pressure summary.
    pub stockpile: StockpileSummary,
    /// Shelter quality summary.
    pub shelter: ShelterQualitySummary,
    /// Disaster history summary.
    pub disaster: DisasterHistorySummary,
    /// Number of pending recommendations.
    pub pending_recommendations: usize,
    /// Number of events in log.
    pub event_log_size: usize,
    /// Composite health score (0.0 = critical, 1.0 = excellent).
    pub health_score: f32,
    /// Snapshot checksum.
    pub checksum: u32,
}

impl DirectorSnapshot {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            pacing: PacingSummary::default(),
            competence: CompetenceSummary::default(),
            stockpile: StockpileSummary::default(),
            shelter: ShelterQualitySummary::default(),
            disaster: DisasterHistorySummary::default(),
            pending_recommendations: 0,
            event_log_size: 0,
            health_score: 0.5,
            checksum: 0,
        }
    }

    #[must_use]
    pub fn with_pacing(mut self, summary: PacingSummary) -> Self {
        self.pacing = summary;
        self
    }

    #[must_use]
    pub fn with_competence(mut self, summary: CompetenceSummary) -> Self {
        self.competence = summary;
        self
    }

    #[must_use]
    pub fn with_stockpile(mut self, summary: StockpileSummary) -> Self {
        self.stockpile = summary;
        self
    }

    #[must_use]
    pub fn with_shelter(mut self, summary: ShelterQualitySummary) -> Self {
        self.shelter = summary;
        self
    }

    #[must_use]
    pub fn with_disaster(mut self, summary: DisasterHistorySummary) -> Self {
        self.disaster = summary;
        self
    }

    pub fn compute_health_score(&mut self) {
        let competence_factor = self.competence.overall_score;
        let stockpile_factor = 1.0 - self.stockpile.overall_pressure;
        let shelter_factor = self.shelter.overall_quality;
        let disaster_factor = if self.disaster.ongoing_disaster_count > 0 {
            0.3
        } else if self.disaster.in_grace_period {
            0.6
        } else {
            1.0
        };

        self.health_score = (competence_factor * 0.25
            + stockpile_factor * 0.3
            + shelter_factor * 0.25
            + disaster_factor * 0.2)
            .clamp(0.0, 1.0);
    }

    pub fn compute_checksum(&mut self) {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.pacing.current_intensity.to_le_bytes());
        hasher.update(&self.competence.overall_score.to_le_bytes());
        hasher.update(&self.stockpile.overall_pressure.to_le_bytes());
        hasher.update(&self.shelter.overall_quality.to_le_bytes());
        hasher.update(&self.disaster.total_disasters.to_le_bytes());
        hasher.update(&self.health_score.to_le_bytes());
        self.checksum = hasher.finalize();
    }

    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.health_score < 0.3
            || self.disaster.ongoing_disaster_count > 0
            || self.stockpile.critical_count > 0
    }

    #[must_use]
    pub fn is_stable(&self) -> bool {
        self.health_score > 0.6
            && self.disaster.ongoing_disaster_count == 0
            && !self.disaster.in_grace_period
    }

    #[must_use]
    pub fn suggested_pacing_adjustment(&self) -> f32 {
        if self.is_critical() {
            return -0.2;
        }
        if self.health_score < 0.4 {
            return -0.1;
        }
        if self.is_stable() && self.competence.overall_score > 0.7 {
            return 0.05;
        }
        0.0
    }
}

impl Default for DirectorSnapshot {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Lightweight summary of director state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DirectorSummary {
    pub tick: u64,
    pub pacing_level: PacingLevel,
    pub pacing_intensity: f32,
    pub competence_score: f32,
    pub stockpile_pressure: f32,
    pub shelter_quality: f32,
    pub health_score: f32,
    pub in_grace_period: bool,
    pub has_ongoing_disaster: bool,
    pub pending_recommendations: usize,
}

impl DirectorSummary {
    #[must_use]
    pub fn from_snapshot(snapshot: &DirectorSnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            pacing_level: snapshot.pacing.level,
            pacing_intensity: snapshot.pacing.current_intensity,
            competence_score: snapshot.competence.overall_score,
            stockpile_pressure: snapshot.stockpile.overall_pressure,
            shelter_quality: snapshot.shelter.overall_quality,
            health_score: snapshot.health_score,
            in_grace_period: snapshot.pacing.in_grace_period || snapshot.disaster.in_grace_period,
            has_ongoing_disaster: snapshot.disaster.ongoing_disaster_count > 0,
            pending_recommendations: snapshot.pending_recommendations,
        }
    }

    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.health_score > 0.6 && !self.has_ongoing_disaster
    }

    #[must_use]
    pub fn is_struggling(&self) -> bool {
        self.health_score < 0.3 || self.stockpile_pressure > 0.7
    }

    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.pending_recommendations > 5 || self.has_ongoing_disaster || self.is_struggling()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::ids::PacingProfileId;

    #[test]
    fn test_director_snapshot_new() {
        let snapshot = DirectorSnapshot::new(100);

        assert_eq!(snapshot.tick, 100);
        assert!((snapshot.health_score - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_director_snapshot_health_score() {
        let mut snapshot = DirectorSnapshot::new(100);

        snapshot.competence.overall_score = 0.8;
        snapshot.stockpile.overall_pressure = 0.2;
        snapshot.shelter.overall_quality = 0.7;

        snapshot.compute_health_score();

        assert!(snapshot.health_score > 0.5);
    }

    #[test]
    fn test_director_snapshot_health_score_disaster() {
        let mut snapshot = DirectorSnapshot::new(100);

        snapshot.competence.overall_score = 0.8;
        snapshot.stockpile.overall_pressure = 0.2;
        snapshot.shelter.overall_quality = 0.7;
        snapshot.disaster.ongoing_disaster_count = 1;

        snapshot.compute_health_score();

        let healthy_score = snapshot.health_score;

        snapshot.disaster.ongoing_disaster_count = 0;
        snapshot.compute_health_score();

        assert!(snapshot.health_score > healthy_score);
    }

    #[test]
    fn test_director_snapshot_is_critical() {
        let mut snapshot = DirectorSnapshot::new(100);
        snapshot.health_score = 0.2;

        assert!(snapshot.is_critical());

        snapshot.health_score = 0.5;
        snapshot.disaster.ongoing_disaster_count = 1;

        assert!(snapshot.is_critical());
    }

    #[test]
    fn test_director_snapshot_is_stable() {
        let mut snapshot = DirectorSnapshot::new(100);
        snapshot.health_score = 0.8;

        assert!(snapshot.is_stable());

        snapshot.disaster.in_grace_period = true;

        assert!(!snapshot.is_stable());
    }

    #[test]
    fn test_director_snapshot_suggested_adjustment() {
        let mut snapshot = DirectorSnapshot::new(100);
        snapshot.health_score = 0.2;

        assert!(snapshot.suggested_pacing_adjustment() < 0.0);

        snapshot.health_score = 0.8;
        snapshot.competence.overall_score = 0.8;

        assert!(snapshot.suggested_pacing_adjustment() > 0.0);
    }

    #[test]
    fn test_director_snapshot_checksum() {
        let mut snapshot1 = DirectorSnapshot::new(100);
        snapshot1.competence.overall_score = 0.7;
        snapshot1.compute_checksum();

        let mut snapshot2 = DirectorSnapshot::new(100);
        snapshot2.competence.overall_score = 0.7;
        snapshot2.compute_checksum();

        assert_eq!(snapshot1.checksum, snapshot2.checksum);
    }

    #[test]
    fn test_director_summary_from_snapshot() {
        let mut snapshot = DirectorSnapshot::new(500);
        snapshot.pacing = PacingSummary {
            tick: 500,
            profile_id: PacingProfileId::new("normal"),
            current_intensity: 0.6,
            effective_intensity: 0.58,
            level: PacingLevel::Normal,
            in_grace_period: false,
            locked: false,
        };
        snapshot.competence.overall_score = 0.75;
        snapshot.health_score = 0.7;

        let summary = DirectorSummary::from_snapshot(&snapshot);

        assert_eq!(summary.tick, 500);
        assert_eq!(summary.pacing_level, PacingLevel::Normal);
        assert!((summary.competence_score - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_director_summary_is_healthy() {
        let summary = DirectorSummary {
            health_score: 0.8,
            has_ongoing_disaster: false,
            ..Default::default()
        };

        assert!(summary.is_healthy());

        let struggling = DirectorSummary {
            health_score: 0.2,
            ..Default::default()
        };

        assert!(struggling.is_struggling());
    }

    #[test]
    fn test_director_summary_needs_attention() {
        let summary = DirectorSummary {
            pending_recommendations: 10,
            ..Default::default()
        };

        assert!(summary.needs_attention());

        let urgent = DirectorSummary {
            has_ongoing_disaster: true,
            ..Default::default()
        };

        assert!(urgent.needs_attention());
    }

    #[test]
    fn test_serde_director_snapshot() {
        let mut snapshot = DirectorSnapshot::new(200);
        snapshot.competence.overall_score = 0.65;
        snapshot.compute_health_score();
        snapshot.compute_checksum();

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: DirectorSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 200);
        assert_eq!(restored.checksum, snapshot.checksum);
    }

    #[test]
    fn test_bincode_director_snapshot() {
        let mut snapshot = DirectorSnapshot::new(300);
        snapshot.stockpile.overall_pressure = 0.4;
        snapshot.shelter.overall_quality = 0.6;
        snapshot.compute_health_score();
        snapshot.compute_checksum();

        let bytes = bincode::serialize(&snapshot).unwrap();
        let restored: DirectorSnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 300);
        assert_eq!(restored.checksum, snapshot.checksum);
    }

    #[test]
    fn test_bincode_director_summary() {
        let summary = DirectorSummary {
            tick: 400,
            pacing_level: PacingLevel::Tense,
            pacing_intensity: 0.7,
            competence_score: 0.6,
            stockpile_pressure: 0.35,
            shelter_quality: 0.75,
            health_score: 0.65,
            in_grace_period: true,
            has_ongoing_disaster: false,
            pending_recommendations: 3,
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: DirectorSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 400);
        assert!(restored.in_grace_period);
    }
}

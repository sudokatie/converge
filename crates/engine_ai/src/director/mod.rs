//! Director AI for adaptive pacing based on colony state.
//!
//! The director system monitors player competence, stockpile pressure,
//! shelter quality, and disaster history to adaptively adjust game pacing.
//! It provides data-driven, deterministic recommendations without any RNG.

mod competence;
mod config;
mod disaster;
mod ids;
mod pacing;
mod pressure;
mod projection;
mod recommendation;
mod shelter;
mod snapshot;

pub use competence::{
    CompetenceCategory, CompetenceConfig, CompetenceSignal, CompetenceSummary, CompetenceTracker,
    CompetenceTrend,
};
pub use config::{DirectorConfig, PacingThresholds};
pub use disaster::{
    DisasterCategory, DisasterHistory, DisasterHistoryConfig, DisasterHistorySummary,
    DisasterRecord, DisasterSeverity,
};
pub use ids::{CompetenceSignalId, DirectorId, DisasterId, PacingProfileId, RecommendationId};
pub use pacing::{
    PacingLevel, PacingProfileDef, PacingProfileRegistry, PacingState, PacingSummary,
    presets as pacing_presets,
};
pub use pressure::{StockpileCategory, StockpilePressureInput, StockpileStatus, StockpileSummary};
pub use projection::{DirectorProjection, DirectorTrend, ProjectionFactors};
pub use recommendation::{
    DirectorEvent, DirectorEventKind, DirectorEventLog, Recommendation, RecommendationCategory,
    RecommendationPriority, RecommendationQueue,
};
pub use shelter::{
    ShelterQualityAssessment, ShelterQualityFactor, ShelterQualityInput, ShelterQualitySummary,
};
pub use snapshot::{DirectorSnapshot, DirectorSummary};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_integration_basic() {
        let mut competence = CompetenceTracker::new(CompetenceConfig::new());
        competence.record_value(CompetenceCategory::TaskEfficiency, "test", 0.7, 100);
        competence.update(100);

        let mut stockpile = StockpilePressureInput::new();
        stockpile.add_stockpile(StockpileStatus::new(StockpileCategory::Food, 80, 100));
        stockpile.update(100);

        let mut shelter = ShelterQualityInput::new();
        let mut assessment = ShelterQualityAssessment::new("hab_1", 50)
            .with_occupancy(30)
            .with_factor(ShelterQualityFactor::Structural, 0.8);
        assessment.compute_overall();
        shelter.add_shelter(assessment);
        shelter.update(100);

        let disaster = DisasterHistory::new(DisasterHistoryConfig::new());

        let mut snapshot = DirectorSnapshot::new(100)
            .with_competence(CompetenceSummary::from_tracker(&competence))
            .with_stockpile(StockpileSummary::from_input(&stockpile))
            .with_shelter(ShelterQualitySummary::from_input(&shelter))
            .with_disaster(DisasterHistorySummary::from_history(&disaster, 100));

        snapshot.compute_health_score();
        snapshot.compute_checksum();

        assert!(snapshot.health_score > 0.0);
        assert!(!snapshot.is_critical());
    }

    #[test]
    fn test_director_pacing_flow() {
        let registry = PacingProfileRegistry::with_presets();
        let profile = registry.get(&PacingProfileId::new("normal")).unwrap();

        let mut state = PacingState::new(profile.id.clone(), profile.target_intensity);

        state.set_target(0.7);
        state.adjust_toward_target(
            profile.change_rate,
            profile.min_intensity,
            profile.max_intensity,
            0.2,
            100,
        );

        let summary = PacingSummary::from_state(&state, 100);
        assert!(!summary.in_grace_period);
    }

    #[test]
    fn test_director_recommendation_generation() {
        let mut queue = RecommendationQueue::new(50);
        let mut log = DirectorEventLog::new(100);

        let id = queue.generate_id();
        let rec = Recommendation::new(
            id,
            RecommendationCategory::Pacing,
            RecommendationPriority::High,
            "Reduce pacing intensity",
            100,
        )
        .with_target_value(0.4)
        .with_confidence(0.8);

        queue.push(rec);
        log.push(DirectorEvent::recommendation_generated(100, id));

        assert_eq!(queue.pending().len(), 1);
        assert_eq!(log.len(), 1);

        let highest = queue.highest_priority().unwrap();
        assert_eq!(highest.id, id);
    }

    #[test]
    fn test_director_disaster_grace_period() {
        let config = DisasterHistoryConfig::new()
            .with_base_grace_period(200)
            .with_recent_threshold(500);
        let mut history = DisasterHistory::new(config);

        let id = history.record_disaster(DisasterCategory::Fire, DisasterSeverity::Major, 100);
        history.end_disaster(id, 200, 5, 100, 1);

        let summary = DisasterHistorySummary::from_history(&history, 250);
        assert!(summary.in_grace_period);
        assert!(summary.has_recent_trauma());
    }

    #[test]
    fn test_director_projection() {
        let factors = ProjectionFactors::new()
            .with_competence_trend(0.1)
            .with_stockpile_trend(-0.05)
            .with_shelter_stability(0.8)
            .with_disaster_recovery(0.7);

        let projection = DirectorProjection::new(100, 1000)
            .with_intensity(0.55)
            .with_health(0.8)
            .with_critical_risk(0.05)
            .with_factors(factors)
            .with_confidence(0.85);

        assert!(!projection.is_concerning());
        assert!(projection.is_optimistic());
    }

    #[test]
    fn test_director_summary_from_snapshot() {
        let mut snapshot = DirectorSnapshot::new(500);
        snapshot.competence.overall_score = 0.7;
        snapshot.stockpile.overall_pressure = 0.3;
        snapshot.shelter.overall_quality = 0.6;
        snapshot.compute_health_score();

        let summary = DirectorSummary::from_snapshot(&snapshot);

        assert!(summary.is_healthy());
        assert!(!summary.is_struggling());
        assert!(!summary.needs_attention());
    }

    #[test]
    fn test_director_config_weights() {
        let config = DirectorConfig::new()
            .with_weights(1.5, 1.0, 1.0, 2.0)
            .with_pacing_bounds(0.2, 0.9);

        assert!((config.total_weight() - 5.5).abs() < f32::EPSILON);
        assert!((config.normalized_disaster_weight() - (2.0 / 5.5)).abs() < 0.001);
    }

    #[test]
    fn test_director_pacing_presets() {
        let presets = pacing_presets::all_presets();
        assert!(presets.len() >= 4);

        let peaceful = pacing_presets::peaceful();
        let survival = pacing_presets::survival();

        assert!(peaceful.target_intensity < survival.target_intensity);
        assert!(peaceful.disaster_multiplier < survival.disaster_multiplier);
    }

    #[test]
    fn test_deterministic_behavior() {
        let mut tracker1 = CompetenceTracker::new(CompetenceConfig::new());
        let mut tracker2 = CompetenceTracker::new(CompetenceConfig::new());

        for i in 0..10 {
            tracker1.record_value(CompetenceCategory::TaskEfficiency, "sig", 0.7, i * 10);
            tracker2.record_value(CompetenceCategory::TaskEfficiency, "sig", 0.7, i * 10);
        }

        tracker1.update(100);
        tracker2.update(100);

        assert_eq!(tracker1.checksum(), tracker2.checksum());
        assert!((tracker1.overall_score() - tracker2.overall_score()).abs() < f32::EPSILON);
    }
}

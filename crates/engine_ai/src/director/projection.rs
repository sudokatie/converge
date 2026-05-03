//! Projections of future director state.

use super::pacing::PacingLevel;
use serde::{Deserialize, Serialize};

/// Projection of future director state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectorProjection {
    pub base_tick: u64,
    pub projected_tick: u64,
    /// Projected pacing intensity.
    pub projected_intensity: f32,
    /// Projected pacing level.
    pub projected_level: PacingLevel,
    /// Projected health score.
    pub projected_health: f32,
    /// Estimated disasters in projection window.
    pub estimated_disasters: u32,
    /// Estimated challenges in projection window.
    pub estimated_challenges: u32,
    /// Estimated respite periods.
    pub estimated_respite_ticks: u64,
    /// Risk of critical state.
    pub critical_risk: f32,
    /// Confidence in projection (0.0 to 1.0).
    pub confidence: f32,
    /// Factors contributing to projection.
    pub factors: ProjectionFactors,
}

impl DirectorProjection {
    #[must_use]
    pub fn new(base_tick: u64, projected_tick: u64) -> Self {
        Self {
            base_tick,
            projected_tick,
            projected_intensity: 0.5,
            projected_level: PacingLevel::Normal,
            projected_health: 0.5,
            estimated_disasters: 0,
            estimated_challenges: 0,
            estimated_respite_ticks: 0,
            critical_risk: 0.0,
            confidence: 1.0,
            factors: ProjectionFactors::default(),
        }
    }

    #[must_use]
    pub fn projection_window(&self) -> u64 {
        self.projected_tick.saturating_sub(self.base_tick)
    }

    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.projected_intensity = intensity.clamp(0.0, 1.0);
        self.projected_level = PacingLevel::from_intensity(intensity);
        self
    }

    #[must_use]
    pub fn with_health(mut self, health: f32) -> Self {
        self.projected_health = health.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_estimates(mut self, disasters: u32, challenges: u32, respite: u64) -> Self {
        self.estimated_disasters = disasters;
        self.estimated_challenges = challenges;
        self.estimated_respite_ticks = respite;
        self
    }

    #[must_use]
    pub fn with_critical_risk(mut self, risk: f32) -> Self {
        self.critical_risk = risk.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_factors(mut self, factors: ProjectionFactors) -> Self {
        self.factors = factors;
        self
    }

    #[must_use]
    pub fn is_concerning(&self) -> bool {
        self.critical_risk > 0.3 || self.projected_health < 0.4
    }

    #[must_use]
    pub fn is_optimistic(&self) -> bool {
        self.critical_risk < 0.1 && self.projected_health > 0.7 && self.confidence > 0.5
    }

    pub fn apply_confidence_decay(&mut self) {
        let window = self.projection_window();
        #[expect(clippy::cast_precision_loss, reason = "tick values bounded")]
        {
            let decay = 1.0 - (window as f32 / 10000.0).min(0.8);
            self.confidence *= decay;
        }
    }
}

impl Default for DirectorProjection {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Factors contributing to a projection.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectionFactors {
    /// Competence trend contribution.
    pub competence_trend: f32,
    /// Stockpile pressure trend.
    pub stockpile_trend: f32,
    /// Shelter stability factor.
    pub shelter_stability: f32,
    /// Disaster recovery factor.
    pub disaster_recovery: f32,
    /// Pacing momentum factor.
    pub pacing_momentum: f32,
}

impl ProjectionFactors {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_competence_trend(mut self, trend: f32) -> Self {
        self.competence_trend = trend.clamp(-1.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_stockpile_trend(mut self, trend: f32) -> Self {
        self.stockpile_trend = trend.clamp(-1.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_shelter_stability(mut self, stability: f32) -> Self {
        self.shelter_stability = stability.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_disaster_recovery(mut self, recovery: f32) -> Self {
        self.disaster_recovery = recovery.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_pacing_momentum(mut self, momentum: f32) -> Self {
        self.pacing_momentum = momentum.clamp(-1.0, 1.0);
        self
    }

    #[must_use]
    pub fn overall_trend(&self) -> f32 {
        (self.competence_trend + self.stockpile_trend + self.pacing_momentum) / 3.0
    }

    #[must_use]
    pub fn stability_factor(&self) -> f32 {
        f32::midpoint(self.shelter_stability, self.disaster_recovery)
    }
}

/// Trend direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectorTrend {
    Improving,
    #[default]
    Stable,
    Declining,
    Volatile,
}

impl DirectorTrend {
    #[must_use]
    pub fn from_delta(delta: f32, threshold: f32) -> Self {
        if delta.abs() < threshold {
            Self::Stable
        } else if delta > 0.0 {
            Self::Improving
        } else {
            Self::Declining
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_projection_new() {
        let projection = DirectorProjection::new(100, 500);

        assert_eq!(projection.base_tick, 100);
        assert_eq!(projection.projected_tick, 500);
        assert_eq!(projection.projection_window(), 400);
    }

    #[test]
    fn test_director_projection_builder() {
        let projection = DirectorProjection::new(0, 1000)
            .with_intensity(0.7)
            .with_health(0.6)
            .with_critical_risk(0.2)
            .with_confidence(0.8);

        assert!((projection.projected_intensity - 0.7).abs() < f32::EPSILON);
        assert!((projection.projected_health - 0.6).abs() < f32::EPSILON);
        assert!((projection.critical_risk - 0.2).abs() < f32::EPSILON);
        assert!((projection.confidence - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_director_projection_intensity_level() {
        let projection = DirectorProjection::new(0, 100).with_intensity(0.85);

        assert_eq!(projection.projected_level, PacingLevel::Intense);
    }

    #[test]
    fn test_director_projection_estimates() {
        let projection = DirectorProjection::new(0, 1000).with_estimates(2, 10, 200);

        assert_eq!(projection.estimated_disasters, 2);
        assert_eq!(projection.estimated_challenges, 10);
        assert_eq!(projection.estimated_respite_ticks, 200);
    }

    #[test]
    fn test_director_projection_concerning() {
        let concerning = DirectorProjection::new(0, 100).with_critical_risk(0.5);
        assert!(concerning.is_concerning());

        let low_health = DirectorProjection::new(0, 100).with_health(0.3);
        assert!(low_health.is_concerning());

        let ok = DirectorProjection::new(0, 100)
            .with_critical_risk(0.1)
            .with_health(0.6);
        assert!(!ok.is_concerning());
    }

    #[test]
    fn test_director_projection_optimistic() {
        let optimistic = DirectorProjection::new(0, 100)
            .with_critical_risk(0.05)
            .with_health(0.8)
            .with_confidence(0.7);

        assert!(optimistic.is_optimistic());

        let not_confident = DirectorProjection::new(0, 100)
            .with_critical_risk(0.05)
            .with_health(0.8)
            .with_confidence(0.3);

        assert!(!not_confident.is_optimistic());
    }

    #[test]
    fn test_director_projection_confidence_decay() {
        let mut projection = DirectorProjection::new(0, 5000).with_confidence(1.0);

        projection.apply_confidence_decay();

        assert!(projection.confidence < 1.0);
    }

    #[test]
    fn test_projection_factors() {
        let factors = ProjectionFactors::new()
            .with_competence_trend(0.1)
            .with_stockpile_trend(-0.2)
            .with_pacing_momentum(0.3);

        let overall = factors.overall_trend();
        assert!(overall.abs() < 0.5);
    }

    #[test]
    fn test_projection_factors_stability() {
        let factors = ProjectionFactors::new()
            .with_shelter_stability(0.8)
            .with_disaster_recovery(0.6);

        let stability = factors.stability_factor();
        assert!((stability - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_director_trend_from_delta() {
        assert_eq!(DirectorTrend::from_delta(0.01, 0.05), DirectorTrend::Stable);
        assert_eq!(
            DirectorTrend::from_delta(0.1, 0.05),
            DirectorTrend::Improving
        );
        assert_eq!(
            DirectorTrend::from_delta(-0.1, 0.05),
            DirectorTrend::Declining
        );
    }

    #[test]
    fn test_serde_director_projection() {
        let projection = DirectorProjection::new(100, 1000)
            .with_intensity(0.65)
            .with_health(0.7)
            .with_estimates(1, 5, 100);

        let json = serde_json::to_string(&projection).unwrap();
        let restored: DirectorProjection = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.base_tick, 100);
        assert_eq!(restored.projected_tick, 1000);
        assert_eq!(restored.estimated_disasters, 1);
    }

    #[test]
    fn test_bincode_director_projection() {
        let projection = DirectorProjection::new(0, 500)
            .with_intensity(0.5)
            .with_confidence(0.9)
            .with_factors(
                ProjectionFactors::new()
                    .with_competence_trend(0.1)
                    .with_shelter_stability(0.8),
            );

        let bytes = bincode::serialize(&projection).unwrap();
        let restored: DirectorProjection = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.projected_tick, 500);
        assert!((restored.factors.competence_trend - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_projection_factors() {
        let factors = ProjectionFactors::new()
            .with_competence_trend(0.2)
            .with_stockpile_trend(-0.1)
            .with_shelter_stability(0.75)
            .with_disaster_recovery(0.5)
            .with_pacing_momentum(0.05);

        let bytes = bincode::serialize(&factors).unwrap();
        let restored: ProjectionFactors = bincode::deserialize(&bytes).unwrap();

        assert!((restored.competence_trend - 0.2).abs() < f32::EPSILON);
        assert!((restored.shelter_stability - 0.75).abs() < f32::EPSILON);
    }
}

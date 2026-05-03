//! Configuration for the director system.

use serde::{Deserialize, Serialize};

/// Configuration for the director AI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectorConfig {
    /// Ticks between pacing evaluations.
    pub evaluation_interval: u64,
    /// Maximum pacing adjustment per tick (0.0 to 1.0).
    pub max_adjustment_rate: f32,
    /// Number of recent disasters to track.
    pub disaster_history_capacity: usize,
    /// Ticks for disaster memory decay.
    pub disaster_memory_decay: u64,
    /// Weight for competence signals in pacing.
    pub competence_weight: f32,
    /// Weight for stockpile pressure in pacing.
    pub stockpile_weight: f32,
    /// Weight for shelter quality in pacing.
    pub shelter_weight: f32,
    /// Weight for recent disasters in pacing.
    pub disaster_weight: f32,
    /// Minimum pacing intensity (0.0 to 1.0).
    pub min_pacing: f32,
    /// Maximum pacing intensity (0.0 to 1.0).
    pub max_pacing: f32,
    /// Whether to auto-adjust pacing.
    pub auto_adjust: bool,
    /// Smoothing factor for exponential moving average (0.0 to 1.0).
    pub smoothing_factor: f32,
    /// Ticks of calm after a disaster before ramping back up.
    pub post_disaster_grace_period: u64,
    /// Maximum recommendations to keep.
    pub max_recommendations: usize,
}

impl DirectorConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_evaluation_interval(mut self, ticks: u64) -> Self {
        self.evaluation_interval = ticks.max(1);
        self
    }

    #[must_use]
    pub fn with_max_adjustment_rate(mut self, rate: f32) -> Self {
        self.max_adjustment_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_disaster_history_capacity(mut self, capacity: usize) -> Self {
        self.disaster_history_capacity = capacity.max(1);
        self
    }

    #[must_use]
    pub fn with_weights(
        mut self,
        competence: f32,
        stockpile: f32,
        shelter: f32,
        disaster: f32,
    ) -> Self {
        self.competence_weight = competence.max(0.0);
        self.stockpile_weight = stockpile.max(0.0);
        self.shelter_weight = shelter.max(0.0);
        self.disaster_weight = disaster.max(0.0);
        self
    }

    #[must_use]
    pub fn with_pacing_bounds(mut self, min: f32, max: f32) -> Self {
        self.min_pacing = min.clamp(0.0, 1.0);
        self.max_pacing = max.clamp(0.0, 1.0).max(self.min_pacing);
        self
    }

    #[must_use]
    pub fn with_auto_adjust(mut self, enabled: bool) -> Self {
        self.auto_adjust = enabled;
        self
    }

    #[must_use]
    pub fn with_smoothing_factor(mut self, factor: f32) -> Self {
        self.smoothing_factor = factor.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_post_disaster_grace_period(mut self, ticks: u64) -> Self {
        self.post_disaster_grace_period = ticks;
        self
    }

    #[must_use]
    pub fn with_max_recommendations(mut self, max: usize) -> Self {
        self.max_recommendations = max.max(1);
        self
    }

    #[must_use]
    pub fn total_weight(&self) -> f32 {
        self.competence_weight + self.stockpile_weight + self.shelter_weight + self.disaster_weight
    }

    #[must_use]
    pub fn normalized_competence_weight(&self) -> f32 {
        let total = self.total_weight();
        if total == 0.0 {
            0.25
        } else {
            self.competence_weight / total
        }
    }

    #[must_use]
    pub fn normalized_stockpile_weight(&self) -> f32 {
        let total = self.total_weight();
        if total == 0.0 {
            0.25
        } else {
            self.stockpile_weight / total
        }
    }

    #[must_use]
    pub fn normalized_shelter_weight(&self) -> f32 {
        let total = self.total_weight();
        if total == 0.0 {
            0.25
        } else {
            self.shelter_weight / total
        }
    }

    #[must_use]
    pub fn normalized_disaster_weight(&self) -> f32 {
        let total = self.total_weight();
        if total == 0.0 {
            0.25
        } else {
            self.disaster_weight / total
        }
    }
}

impl Default for DirectorConfig {
    fn default() -> Self {
        Self {
            evaluation_interval: 100,
            max_adjustment_rate: 0.1,
            disaster_history_capacity: 50,
            disaster_memory_decay: 3000,
            competence_weight: 1.0,
            stockpile_weight: 1.0,
            shelter_weight: 1.0,
            disaster_weight: 1.5,
            min_pacing: 0.1,
            max_pacing: 1.0,
            auto_adjust: true,
            smoothing_factor: 0.2,
            post_disaster_grace_period: 500,
            max_recommendations: 100,
        }
    }
}

/// Thresholds for pacing transitions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacingThresholds {
    /// Competence below this triggers easier pacing.
    pub low_competence: f32,
    /// Competence above this allows harder pacing.
    pub high_competence: f32,
    /// Stockpile pressure below this suggests abundance.
    pub low_stockpile_pressure: f32,
    /// Stockpile pressure above this indicates scarcity.
    pub high_stockpile_pressure: f32,
    /// Shelter quality below this is concerning.
    pub low_shelter_quality: f32,
    /// Shelter quality above this is comfortable.
    pub high_shelter_quality: f32,
    /// Disaster recency below this (ticks ago) is recent.
    pub recent_disaster_threshold: u64,
}

impl PacingThresholds {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_competence_bounds(mut self, low: f32, high: f32) -> Self {
        self.low_competence = low.clamp(0.0, 1.0);
        self.high_competence = high.clamp(0.0, 1.0).max(self.low_competence);
        self
    }

    #[must_use]
    pub fn with_stockpile_bounds(mut self, low: f32, high: f32) -> Self {
        self.low_stockpile_pressure = low.clamp(0.0, 1.0);
        self.high_stockpile_pressure = high.clamp(0.0, 1.0).max(self.low_stockpile_pressure);
        self
    }

    #[must_use]
    pub fn with_shelter_bounds(mut self, low: f32, high: f32) -> Self {
        self.low_shelter_quality = low.clamp(0.0, 1.0);
        self.high_shelter_quality = high.clamp(0.0, 1.0).max(self.low_shelter_quality);
        self
    }

    #[must_use]
    pub fn with_recent_disaster_threshold(mut self, ticks: u64) -> Self {
        self.recent_disaster_threshold = ticks;
        self
    }
}

impl Default for PacingThresholds {
    fn default() -> Self {
        Self {
            low_competence: 0.3,
            high_competence: 0.7,
            low_stockpile_pressure: 0.3,
            high_stockpile_pressure: 0.7,
            low_shelter_quality: 0.4,
            high_shelter_quality: 0.8,
            recent_disaster_threshold: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_config_defaults() {
        let config = DirectorConfig::new();

        assert!(config.evaluation_interval > 0);
        assert!(config.max_adjustment_rate > 0.0);
        assert!(config.max_adjustment_rate <= 1.0);
        assert!(config.min_pacing < config.max_pacing);
        assert!(config.auto_adjust);
    }

    #[test]
    fn test_director_config_builder() {
        let config = DirectorConfig::new()
            .with_evaluation_interval(50)
            .with_max_adjustment_rate(0.05)
            .with_pacing_bounds(0.2, 0.9)
            .with_auto_adjust(false);

        assert_eq!(config.evaluation_interval, 50);
        assert!((config.max_adjustment_rate - 0.05).abs() < f32::EPSILON);
        assert!((config.min_pacing - 0.2).abs() < f32::EPSILON);
        assert!((config.max_pacing - 0.9).abs() < f32::EPSILON);
        assert!(!config.auto_adjust);
    }

    #[test]
    fn test_director_config_weights() {
        let config = DirectorConfig::new().with_weights(2.0, 1.0, 1.0, 2.0);

        assert!((config.competence_weight - 2.0).abs() < f32::EPSILON);
        assert!((config.total_weight() - 6.0).abs() < f32::EPSILON);
        assert!((config.normalized_competence_weight() - (2.0 / 6.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_director_config_normalized_weights_zero() {
        let config = DirectorConfig::new().with_weights(0.0, 0.0, 0.0, 0.0);

        assert!((config.normalized_competence_weight() - 0.25).abs() < f32::EPSILON);
        assert!((config.normalized_stockpile_weight() - 0.25).abs() < f32::EPSILON);
        assert!((config.normalized_shelter_weight() - 0.25).abs() < f32::EPSILON);
        assert!((config.normalized_disaster_weight() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pacing_thresholds_defaults() {
        let thresholds = PacingThresholds::new();

        assert!(thresholds.low_competence < thresholds.high_competence);
        assert!(thresholds.low_stockpile_pressure < thresholds.high_stockpile_pressure);
        assert!(thresholds.low_shelter_quality < thresholds.high_shelter_quality);
    }

    #[test]
    fn test_pacing_thresholds_builder() {
        let thresholds = PacingThresholds::new()
            .with_competence_bounds(0.2, 0.8)
            .with_stockpile_bounds(0.25, 0.75)
            .with_recent_disaster_threshold(500);

        assert!((thresholds.low_competence - 0.2).abs() < f32::EPSILON);
        assert!((thresholds.high_competence - 0.8).abs() < f32::EPSILON);
        assert_eq!(thresholds.recent_disaster_threshold, 500);
    }

    #[test]
    fn test_serde_director_config() {
        let config = DirectorConfig::new()
            .with_evaluation_interval(200)
            .with_max_recommendations(50);

        let json = serde_json::to_string(&config).unwrap();
        let restored: DirectorConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.evaluation_interval, 200);
        assert_eq!(restored.max_recommendations, 50);
    }

    #[test]
    fn test_bincode_director_config() {
        let config = DirectorConfig::new()
            .with_disaster_history_capacity(100)
            .with_smoothing_factor(0.3);

        let bytes = bincode::serialize(&config).unwrap();
        let restored: DirectorConfig = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.disaster_history_capacity, 100);
        assert!((restored.smoothing_factor - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_pacing_thresholds() {
        let thresholds = PacingThresholds::new().with_competence_bounds(0.15, 0.85);

        let json = serde_json::to_string(&thresholds).unwrap();
        let restored: PacingThresholds = serde_json::from_str(&json).unwrap();

        assert!((restored.low_competence - 0.15).abs() < f32::EPSILON);
        assert!((restored.high_competence - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_pacing_thresholds() {
        let thresholds = PacingThresholds::new().with_shelter_bounds(0.3, 0.9);

        let bytes = bincode::serialize(&thresholds).unwrap();
        let restored: PacingThresholds = bincode::deserialize(&bytes).unwrap();

        assert!((restored.low_shelter_quality - 0.3).abs() < f32::EPSILON);
        assert!((restored.high_shelter_quality - 0.9).abs() < f32::EPSILON);
    }
}

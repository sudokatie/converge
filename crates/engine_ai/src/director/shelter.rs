//! Shelter quality inputs for director pacing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Quality factors for a shelter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ShelterQualityFactor {
    /// Structural integrity.
    Structural,
    /// Protection from elements.
    WeatherProtection,
    /// Thermal comfort.
    ThermalComfort,
    /// Space per occupant.
    SpacePerOccupant,
    /// Access to resources.
    ResourceAccess,
    /// Safety from threats.
    Safety,
    /// Sanitation quality.
    Sanitation,
    /// Lighting and visibility.
    Lighting,
}

impl ShelterQualityFactor {
    #[must_use]
    pub fn importance_weight(self) -> f32 {
        match self {
            Self::Structural => 1.5,
            Self::WeatherProtection => 1.3,
            Self::ThermalComfort => 1.0,
            Self::SpacePerOccupant => 0.8,
            Self::ResourceAccess => 0.9,
            Self::Safety => 1.4,
            Self::Sanitation => 1.1,
            Self::Lighting => 0.5,
        }
    }

    #[must_use]
    pub fn is_critical(self) -> bool {
        matches!(
            self,
            Self::Structural | Self::Safety | Self::WeatherProtection
        )
    }
}

/// Quality assessment for a shelter unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShelterQualityAssessment {
    /// Shelter identifier.
    pub shelter_id: String,
    /// Quality scores by factor (0.0 = terrible, 1.0 = excellent).
    pub factor_scores: BTreeMap<ShelterQualityFactor, f32>,
    /// Current occupancy.
    pub occupancy: u32,
    /// Maximum capacity.
    pub capacity: u32,
    /// Overall quality score.
    pub overall_quality: f32,
    /// Tick when assessed.
    pub assessed_tick: u64,
}

impl ShelterQualityAssessment {
    #[must_use]
    pub fn new(shelter_id: impl Into<String>, capacity: u32) -> Self {
        Self {
            shelter_id: shelter_id.into(),
            factor_scores: BTreeMap::new(),
            occupancy: 0,
            capacity,
            overall_quality: 0.5,
            assessed_tick: 0,
        }
    }

    #[must_use]
    pub fn with_occupancy(mut self, occupancy: u32) -> Self {
        self.occupancy = occupancy.min(self.capacity);
        self
    }

    pub fn set_factor(&mut self, factor: ShelterQualityFactor, score: f32) {
        self.factor_scores.insert(factor, score.clamp(0.0, 1.0));
    }

    #[must_use]
    pub fn with_factor(mut self, factor: ShelterQualityFactor, score: f32) -> Self {
        self.set_factor(factor, score);
        self
    }

    pub fn compute_overall(&mut self) {
        if self.factor_scores.is_empty() {
            self.overall_quality = 0.5;
            return;
        }

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (factor, score) in &self.factor_scores {
            let weight = factor.importance_weight();
            weighted_sum += score * weight;
            total_weight += weight;
        }

        self.overall_quality = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.5
        };
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn occupancy_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.occupancy as f32 / self.capacity as f32
    }

    #[must_use]
    pub fn is_overcrowded(&self) -> bool {
        self.occupancy_ratio() > 0.9
    }

    #[must_use]
    pub fn has_vacancy(&self) -> bool {
        self.occupancy < self.capacity
    }

    #[must_use]
    pub fn vacancy_count(&self) -> u32 {
        self.capacity.saturating_sub(self.occupancy)
    }

    #[must_use]
    pub fn critical_factor_score(&self) -> f32 {
        let critical_factors: Vec<_> = self
            .factor_scores
            .iter()
            .filter(|(f, _)| f.is_critical())
            .map(|(_, s)| *s)
            .collect();

        if critical_factors.is_empty() {
            return 0.5;
        }

        #[expect(clippy::cast_precision_loss, reason = "count bounded")]
        {
            critical_factors.iter().sum::<f32>() / critical_factors.len() as f32
        }
    }

    #[must_use]
    pub fn has_critical_deficiency(&self) -> bool {
        self.factor_scores
            .iter()
            .any(|(f, s)| f.is_critical() && *s < 0.3)
    }
}

/// Aggregated shelter quality input for the director.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShelterQualityInput {
    /// Individual shelter assessments.
    pub shelters: Vec<ShelterQualityAssessment>,
    /// Overall quality across all shelters.
    pub overall_quality: f32,
    /// Total capacity.
    pub total_capacity: u32,
    /// Total occupancy.
    pub total_occupancy: u32,
    /// Number of shelters with critical deficiencies.
    pub critical_deficiency_count: u32,
    /// Tick when computed.
    pub tick: u64,
}

impl ShelterQualityInput {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_shelter(&mut self, assessment: ShelterQualityAssessment) {
        self.shelters.push(assessment);
    }

    #[expect(clippy::cast_precision_loss, reason = "counts bounded")]
    pub fn update(&mut self, tick: u64) {
        self.tick = tick;

        self.total_capacity = self.shelters.iter().map(|s| s.capacity).sum();
        self.total_occupancy = self.shelters.iter().map(|s| s.occupancy).sum();

        self.critical_deficiency_count = u32::try_from(
            self.shelters
                .iter()
                .filter(|s| s.has_critical_deficiency())
                .count(),
        )
        .unwrap_or(u32::MAX);

        if self.shelters.is_empty() {
            self.overall_quality = 0.0;
        } else {
            let weighted_sum: f32 = self
                .shelters
                .iter()
                .map(|s| s.overall_quality * s.capacity as f32)
                .sum();
            self.overall_quality = if self.total_capacity > 0 {
                weighted_sum / self.total_capacity as f32
            } else {
                0.0
            };
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn coverage_ratio(&self, population: u32) -> f32 {
        if population == 0 {
            return 1.0;
        }
        (self.total_capacity as f32 / population as f32).min(1.0)
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn occupancy_ratio(&self) -> f32 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        self.total_occupancy as f32 / self.total_capacity as f32
    }

    #[must_use]
    pub fn total_vacancy(&self) -> u32 {
        self.total_capacity.saturating_sub(self.total_occupancy)
    }

    #[must_use]
    pub fn has_critical_issues(&self) -> bool {
        self.critical_deficiency_count > 0 || self.overall_quality < 0.3
    }

    #[must_use]
    pub fn average_quality(&self) -> f32 {
        if self.shelters.is_empty() {
            return 0.0;
        }

        #[expect(clippy::cast_precision_loss, reason = "count bounded")]
        {
            self.shelters.iter().map(|s| s.overall_quality).sum::<f32>()
                / self.shelters.len() as f32
        }
    }

    #[must_use]
    pub fn worst_shelter(&self) -> Option<&ShelterQualityAssessment> {
        self.shelters.iter().min_by(|a, b| {
            a.overall_quality
                .partial_cmp(&b.overall_quality)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.overall_quality.to_le_bytes());
        hasher.update(&self.total_capacity.to_le_bytes());
        hasher.update(&self.total_occupancy.to_le_bytes());
        hasher.update(&self.tick.to_le_bytes());
        hasher.finalize()
    }
}

/// Summary of shelter quality state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShelterQualitySummary {
    pub tick: u64,
    pub overall_quality: f32,
    pub shelter_count: usize,
    pub total_capacity: u32,
    pub total_occupancy: u32,
    pub critical_deficiency_count: u32,
    pub average_factor_quality: f32,
}

impl ShelterQualitySummary {
    #[must_use]
    pub fn from_input(input: &ShelterQualityInput) -> Self {
        Self {
            tick: input.tick,
            overall_quality: input.overall_quality,
            shelter_count: input.shelters.len(),
            total_capacity: input.total_capacity,
            total_occupancy: input.total_occupancy,
            critical_deficiency_count: input.critical_deficiency_count,
            average_factor_quality: input.average_quality(),
        }
    }

    #[must_use]
    pub fn is_adequate(&self) -> bool {
        self.overall_quality >= 0.5 && self.critical_deficiency_count == 0
    }

    #[must_use]
    pub fn is_excellent(&self) -> bool {
        self.overall_quality >= 0.8 && self.critical_deficiency_count == 0
    }

    #[must_use]
    pub fn is_poor(&self) -> bool {
        self.overall_quality < 0.3 || self.critical_deficiency_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shelter_quality_factor_weight() {
        assert!(ShelterQualityFactor::Structural.importance_weight() > 1.0);
        assert!(ShelterQualityFactor::Lighting.importance_weight() < 1.0);
    }

    #[test]
    fn test_shelter_quality_factor_critical() {
        assert!(ShelterQualityFactor::Structural.is_critical());
        assert!(ShelterQualityFactor::Safety.is_critical());
        assert!(!ShelterQualityFactor::Lighting.is_critical());
    }

    #[test]
    fn test_shelter_quality_assessment_new() {
        let assessment = ShelterQualityAssessment::new("hab_1", 50).with_occupancy(30);

        assert_eq!(assessment.shelter_id, "hab_1");
        assert_eq!(assessment.capacity, 50);
        assert_eq!(assessment.occupancy, 30);
        assert!((assessment.occupancy_ratio() - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_shelter_quality_assessment_factors() {
        let mut assessment = ShelterQualityAssessment::new("hab_1", 100)
            .with_factor(ShelterQualityFactor::Structural, 0.9)
            .with_factor(ShelterQualityFactor::Safety, 0.8)
            .with_factor(ShelterQualityFactor::Lighting, 0.5);

        assessment.compute_overall();

        assert!(assessment.overall_quality > 0.5);
        assert!(assessment.critical_factor_score() > 0.8);
        assert!(!assessment.has_critical_deficiency());
    }

    #[test]
    fn test_shelter_quality_assessment_critical_deficiency() {
        let mut assessment = ShelterQualityAssessment::new("hab_1", 100)
            .with_factor(ShelterQualityFactor::Structural, 0.2)
            .with_factor(ShelterQualityFactor::Safety, 0.8);

        assessment.compute_overall();

        assert!(assessment.has_critical_deficiency());
    }

    #[test]
    fn test_shelter_quality_input() {
        let mut input = ShelterQualityInput::new();

        let mut shelter1 = ShelterQualityAssessment::new("hab_1", 50)
            .with_occupancy(40)
            .with_factor(ShelterQualityFactor::Structural, 0.9);
        shelter1.compute_overall();

        let mut shelter2 = ShelterQualityAssessment::new("hab_2", 50)
            .with_occupancy(30)
            .with_factor(ShelterQualityFactor::Structural, 0.7);
        shelter2.compute_overall();

        input.add_shelter(shelter1);
        input.add_shelter(shelter2);
        input.update(100);

        assert_eq!(input.total_capacity, 100);
        assert_eq!(input.total_occupancy, 70);
        assert!(input.overall_quality > 0.0);
    }

    #[test]
    fn test_shelter_quality_input_coverage() {
        let mut input = ShelterQualityInput::new();

        let mut shelter = ShelterQualityAssessment::new("hab_1", 80).with_occupancy(60);
        shelter.compute_overall();
        input.add_shelter(shelter);
        input.update(100);

        assert!((input.coverage_ratio(100) - 0.8).abs() < f32::EPSILON);
        assert_eq!(input.total_vacancy(), 20);
    }

    #[test]
    fn test_shelter_quality_summary() {
        let mut input = ShelterQualityInput::new();

        let mut shelter = ShelterQualityAssessment::new("hab_1", 100)
            .with_occupancy(50)
            .with_factor(ShelterQualityFactor::Structural, 0.9)
            .with_factor(ShelterQualityFactor::Safety, 0.85);
        shelter.compute_overall();

        input.add_shelter(shelter);
        input.update(100);

        let summary = ShelterQualitySummary::from_input(&input);

        assert!(summary.is_adequate());
        assert!(!summary.is_poor());
        assert_eq!(summary.shelter_count, 1);
    }

    #[test]
    fn test_serde_shelter_quality_assessment() {
        let mut assessment = ShelterQualityAssessment::new("test_hab", 75)
            .with_occupancy(50)
            .with_factor(ShelterQualityFactor::Structural, 0.85);
        assessment.compute_overall();

        let json = serde_json::to_string(&assessment).unwrap();
        let restored: ShelterQualityAssessment = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.shelter_id, "test_hab");
        assert_eq!(restored.capacity, 75);
        assert_eq!(restored.occupancy, 50);
    }

    #[test]
    fn test_bincode_shelter_quality_input() {
        let mut input = ShelterQualityInput::new();

        let mut shelter = ShelterQualityAssessment::new("hab", 100)
            .with_occupancy(80)
            .with_factor(ShelterQualityFactor::Safety, 0.9);
        shelter.compute_overall();

        input.add_shelter(shelter);
        input.update(500);

        let bytes = bincode::serialize(&input).unwrap();
        let restored: ShelterQualityInput = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.checksum(), input.checksum());
        assert_eq!(restored.tick, 500);
    }

    #[test]
    fn test_bincode_shelter_quality_summary() {
        let summary = ShelterQualitySummary {
            tick: 200,
            overall_quality: 0.75,
            shelter_count: 5,
            total_capacity: 500,
            total_occupancy: 350,
            critical_deficiency_count: 0,
            average_factor_quality: 0.7,
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: ShelterQualitySummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 200);
        assert!((restored.overall_quality - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_checksum_consistency() {
        let mut input1 = ShelterQualityInput::new();
        let mut input2 = ShelterQualityInput::new();

        let mut shelter1 = ShelterQualityAssessment::new("hab", 100).with_occupancy(50);
        let mut shelter2 = ShelterQualityAssessment::new("hab", 100).with_occupancy(50);
        shelter1.compute_overall();
        shelter2.compute_overall();

        input1.add_shelter(shelter1);
        input2.add_shelter(shelter2);
        input1.update(100);
        input2.update(100);

        assert_eq!(input1.checksum(), input2.checksum());
    }
}

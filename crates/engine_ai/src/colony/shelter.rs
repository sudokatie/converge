//! Colony shelter system for habitability and protection ratings.

use super::ids::ShelterId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Rating value from 0.0 to 1.0.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Rating(f32);

impl Rating {
    pub const MIN: Self = Self(0.0);
    pub const MAX: Self = Self(1.0);

    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    #[must_use]
    pub fn raw(self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn is_critical(self) -> bool {
        self.0 < 0.2
    }

    #[must_use]
    pub fn is_poor(self) -> bool {
        self.0 < 0.4
    }

    #[must_use]
    pub fn is_adequate(self) -> bool {
        self.0 >= 0.4 && self.0 < 0.7
    }

    #[must_use]
    pub fn is_good(self) -> bool {
        self.0 >= 0.7 && self.0 < 0.9
    }

    #[must_use]
    pub fn is_excellent(self) -> bool {
        self.0 >= 0.9
    }
}

impl Default for Rating {
    fn default() -> Self {
        Self(0.5)
    }
}

/// Category of shelter rating.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RatingCategory {
    Safety,
    Comfort,
    Warmth,
    Cooling,
    AirQuality,
    Pressure,
    Supplies,
    Crowding,
    Access,
    HazardExposure,
}

impl RatingCategory {
    #[must_use]
    pub fn default_weight(self) -> f32 {
        match self {
            Self::Safety => 2.0,
            Self::Pressure => 1.8,
            Self::AirQuality | Self::HazardExposure => 1.5,
            Self::Warmth | Self::Cooling => 1.2,
            Self::Supplies => 1.0,
            Self::Crowding => 0.8,
            Self::Comfort => 0.6,
            Self::Access => 0.5,
        }
    }
}

/// Detailed ratings for a shelter.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShelterRatings {
    pub safety: Rating,
    pub comfort: Rating,
    pub warmth: Rating,
    pub cooling: Rating,
    pub air_quality: Rating,
    pub pressure: Rating,
    pub supplies: Rating,
    pub crowding: Rating,
    pub access: Rating,
    pub hazard_exposure: Rating,
}

impl ShelterRatings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_safety(mut self, value: f32) -> Self {
        self.safety = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_comfort(mut self, value: f32) -> Self {
        self.comfort = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_warmth(mut self, value: f32) -> Self {
        self.warmth = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_cooling(mut self, value: f32) -> Self {
        self.cooling = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_air_quality(mut self, value: f32) -> Self {
        self.air_quality = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_pressure(mut self, value: f32) -> Self {
        self.pressure = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_supplies(mut self, value: f32) -> Self {
        self.supplies = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_crowding(mut self, value: f32) -> Self {
        self.crowding = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_access(mut self, value: f32) -> Self {
        self.access = Rating::new(value);
        self
    }

    #[must_use]
    pub fn with_hazard_exposure(mut self, value: f32) -> Self {
        self.hazard_exposure = Rating::new(value);
        self
    }

    #[must_use]
    pub fn get(&self, category: RatingCategory) -> Rating {
        match category {
            RatingCategory::Safety => self.safety,
            RatingCategory::Comfort => self.comfort,
            RatingCategory::Warmth => self.warmth,
            RatingCategory::Cooling => self.cooling,
            RatingCategory::AirQuality => self.air_quality,
            RatingCategory::Pressure => self.pressure,
            RatingCategory::Supplies => self.supplies,
            RatingCategory::Crowding => self.crowding,
            RatingCategory::Access => self.access,
            RatingCategory::HazardExposure => self.hazard_exposure,
        }
    }

    pub fn set(&mut self, category: RatingCategory, value: f32) {
        let rating = Rating::new(value);
        match category {
            RatingCategory::Safety => self.safety = rating,
            RatingCategory::Comfort => self.comfort = rating,
            RatingCategory::Warmth => self.warmth = rating,
            RatingCategory::Cooling => self.cooling = rating,
            RatingCategory::AirQuality => self.air_quality = rating,
            RatingCategory::Pressure => self.pressure = rating,
            RatingCategory::Supplies => self.supplies = rating,
            RatingCategory::Crowding => self.crowding = rating,
            RatingCategory::Access => self.access = rating,
            RatingCategory::HazardExposure => self.hazard_exposure = rating,
        }
    }

    #[must_use]
    pub fn weighted_average(&self, weights: &ShelterWeights) -> f32 {
        let total_weight = weights.total();
        if total_weight == 0.0 {
            return 0.0;
        }

        let sum = self.safety.raw() * weights.safety
            + self.comfort.raw() * weights.comfort
            + self.warmth.raw() * weights.warmth
            + self.cooling.raw() * weights.cooling
            + self.air_quality.raw() * weights.air_quality
            + self.pressure.raw() * weights.pressure
            + self.supplies.raw() * weights.supplies
            + self.crowding.raw() * weights.crowding
            + self.access.raw() * weights.access
            + self.hazard_exposure.raw() * weights.hazard_exposure;

        sum / total_weight
    }

    #[must_use]
    pub fn minimum(&self) -> Rating {
        let min = self
            .safety
            .raw()
            .min(self.comfort.raw())
            .min(self.warmth.raw())
            .min(self.cooling.raw())
            .min(self.air_quality.raw())
            .min(self.pressure.raw())
            .min(self.supplies.raw())
            .min(self.crowding.raw())
            .min(self.access.raw())
            .min(self.hazard_exposure.raw());
        Rating::new(min)
    }

    #[must_use]
    pub fn critical_issues(&self) -> Vec<RatingCategory> {
        let mut issues = Vec::new();
        if self.safety.is_critical() {
            issues.push(RatingCategory::Safety);
        }
        if self.pressure.is_critical() {
            issues.push(RatingCategory::Pressure);
        }
        if self.air_quality.is_critical() {
            issues.push(RatingCategory::AirQuality);
        }
        if self.warmth.is_critical() {
            issues.push(RatingCategory::Warmth);
        }
        if self.cooling.is_critical() {
            issues.push(RatingCategory::Cooling);
        }
        if self.supplies.is_critical() {
            issues.push(RatingCategory::Supplies);
        }
        if self.crowding.is_critical() {
            issues.push(RatingCategory::Crowding);
        }
        if self.hazard_exposure.is_critical() {
            issues.push(RatingCategory::HazardExposure);
        }
        issues
    }
}

/// Weights for shelter rating calculations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShelterWeights {
    pub safety: f32,
    pub comfort: f32,
    pub warmth: f32,
    pub cooling: f32,
    pub air_quality: f32,
    pub pressure: f32,
    pub supplies: f32,
    pub crowding: f32,
    pub access: f32,
    pub hazard_exposure: f32,
}

impl Default for ShelterWeights {
    fn default() -> Self {
        Self {
            safety: RatingCategory::Safety.default_weight(),
            comfort: RatingCategory::Comfort.default_weight(),
            warmth: RatingCategory::Warmth.default_weight(),
            cooling: RatingCategory::Cooling.default_weight(),
            air_quality: RatingCategory::AirQuality.default_weight(),
            pressure: RatingCategory::Pressure.default_weight(),
            supplies: RatingCategory::Supplies.default_weight(),
            crowding: RatingCategory::Crowding.default_weight(),
            access: RatingCategory::Access.default_weight(),
            hazard_exposure: RatingCategory::HazardExposure.default_weight(),
        }
    }
}

impl ShelterWeights {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn total(&self) -> f32 {
        self.safety
            + self.comfort
            + self.warmth
            + self.cooling
            + self.air_quality
            + self.pressure
            + self.supplies
            + self.crowding
            + self.access
            + self.hazard_exposure
    }
}

/// Shelter definition with capacity and ratings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shelter {
    pub id: ShelterId,
    pub name: String,
    pub capacity: u32,
    pub current_occupancy: u32,
    pub ratings: ShelterRatings,
    pub is_operational: bool,
    pub distance_from_center: f32,
    pub created_tick: u64,
    pub last_updated_tick: u64,
}

impl Shelter {
    #[must_use]
    pub fn new(id: ShelterId, name: impl Into<String>, capacity: u32, created_tick: u64) -> Self {
        Self {
            id,
            name: name.into(),
            capacity,
            current_occupancy: 0,
            ratings: ShelterRatings::new()
                .with_safety(0.7)
                .with_comfort(0.5)
                .with_warmth(0.6)
                .with_cooling(0.6)
                .with_air_quality(0.7)
                .with_pressure(0.8)
                .with_supplies(0.5)
                .with_crowding(1.0)
                .with_access(0.7)
                .with_hazard_exposure(0.8),
            is_operational: true,
            distance_from_center: 0.0,
            created_tick,
            last_updated_tick: created_tick,
        }
    }

    #[must_use]
    pub fn with_ratings(mut self, ratings: ShelterRatings) -> Self {
        self.ratings = ratings;
        self
    }

    #[must_use]
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance_from_center = distance;
        self
    }

    #[must_use]
    pub fn available_capacity(&self) -> u32 {
        self.capacity.saturating_sub(self.current_occupancy)
    }

    #[must_use]
    pub fn can_accept(&self, count: u32) -> bool {
        self.is_operational && self.available_capacity() >= count
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current_occupancy >= self.capacity
    }

    #[must_use]
    pub fn is_overcrowded(&self) -> bool {
        self.current_occupancy > self.capacity
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn crowding_factor(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.current_occupancy as f32 / self.capacity as f32
    }

    pub fn update_crowding_rating(&mut self) {
        let factor = self.crowding_factor();
        let rating = if factor <= 0.7 {
            1.0
        } else if factor <= 0.9 {
            0.8
        } else if factor <= 1.0 {
            0.5
        } else if factor <= 1.2 {
            0.3
        } else {
            0.1
        };
        self.ratings.crowding = Rating::new(rating);
    }

    pub fn admit(&mut self, count: u32, tick: u64) {
        self.current_occupancy = self.current_occupancy.saturating_add(count);
        self.update_crowding_rating();
        self.last_updated_tick = tick;
    }

    pub fn release(&mut self, count: u32, tick: u64) {
        self.current_occupancy = self.current_occupancy.saturating_sub(count);
        self.update_crowding_rating();
        self.last_updated_tick = tick;
    }

    #[must_use]
    pub fn overall_value(&self, weights: &ShelterWeights) -> f32 {
        self.ratings.weighted_average(weights)
    }

    #[must_use]
    pub fn has_critical_issues(&self) -> bool {
        !self.ratings.critical_issues().is_empty()
    }

    pub fn disable(&mut self, tick: u64) {
        self.is_operational = false;
        self.last_updated_tick = tick;
    }

    pub fn enable(&mut self, tick: u64) {
        self.is_operational = true;
        self.last_updated_tick = tick;
    }
}

/// Registry for shelters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShelterRegistry {
    shelters: BTreeMap<ShelterId, Shelter>,
    next_id: u64,
}

impl ShelterRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(
        &mut self,
        name: impl Into<String>,
        capacity: u32,
        created_tick: u64,
    ) -> ShelterId {
        let id = ShelterId::new(self.next_id);
        self.next_id += 1;
        let shelter = Shelter::new(id, name, capacity, created_tick);
        self.shelters.insert(id, shelter);
        id
    }

    pub fn register(&mut self, shelter: Shelter) {
        let id = shelter.id;
        self.shelters.insert(id, shelter);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: ShelterId) -> Option<Shelter> {
        self.shelters.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: ShelterId) -> Option<&Shelter> {
        self.shelters.get(&id)
    }

    pub fn get_mut(&mut self, id: ShelterId) -> Option<&mut Shelter> {
        self.shelters.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Shelter> {
        self.shelters.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Shelter> {
        self.shelters.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.shelters.len()
    }

    pub fn operational(&self) -> impl Iterator<Item = &Shelter> {
        self.shelters.values().filter(|s| s.is_operational)
    }

    pub fn with_capacity(&self, needed: u32) -> impl Iterator<Item = &Shelter> {
        self.shelters.values().filter(move |s| s.can_accept(needed))
    }

    #[must_use]
    pub fn total_capacity(&self) -> u32 {
        self.shelters.values().map(|s| s.capacity).sum()
    }

    #[must_use]
    pub fn total_occupancy(&self) -> u32 {
        self.shelters.values().map(|s| s.current_occupancy).sum()
    }

    #[must_use]
    pub fn total_available(&self) -> u32 {
        self.shelters
            .values()
            .filter(|s| s.is_operational)
            .map(Shelter::available_capacity)
            .sum()
    }

    #[must_use]
    pub fn best_shelter(&self, weights: &ShelterWeights, min_capacity: u32) -> Option<ShelterId> {
        self.shelters
            .values()
            .filter(|s| s.is_operational && s.available_capacity() >= min_capacity)
            .max_by(|a, b| {
                let val_a = a.overall_value(weights);
                let val_b = b.overall_value(weights);
                val_a
                    .partial_cmp(&val_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.id)
    }
}

/// Colony-level shelter coverage summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShelterCoverage {
    pub tick: u64,
    pub total_shelters: u32,
    pub operational_shelters: u32,
    pub total_capacity: u32,
    pub total_occupancy: u32,
    pub average_rating: f32,
    pub minimum_rating: f32,
    pub shelters_with_critical_issues: u32,
    pub overcrowded_shelters: u32,
    pub coverage_ratio: f32,
}

impl ShelterCoverage {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "bounded values"
    )]
    pub fn from_registry(registry: &ShelterRegistry, population: u32, tick: u64) -> Self {
        let weights = ShelterWeights::default();
        let total_shelters = registry.count() as u32;
        let total_capacity = registry.total_capacity();

        let operational: Vec<_> = registry.operational().collect();
        let operational_count = operational.len() as u32;

        let (avg_rating, min_rating) = if operational.is_empty() {
            (0.0, 0.0)
        } else {
            let ratings: Vec<f32> = operational
                .iter()
                .map(|s| s.overall_value(&weights))
                .collect();
            let sum: f32 = ratings.iter().sum();
            let avg = sum / ratings.len() as f32;
            let min = ratings
                .iter()
                .copied()
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);
            (avg, min)
        };

        let critical_count = registry.iter().filter(|s| s.has_critical_issues()).count() as u32;

        let overcrowded_count = registry.iter().filter(|s| s.is_overcrowded()).count() as u32;

        let coverage = if population == 0 {
            1.0
        } else {
            (total_capacity as f32 / population as f32).min(1.0)
        };

        Self {
            tick,
            total_shelters,
            operational_shelters: operational_count,
            total_capacity,
            total_occupancy: registry.total_occupancy(),
            average_rating: avg_rating,
            minimum_rating: min_rating,
            shelters_with_critical_issues: critical_count,
            overcrowded_shelters: overcrowded_count,
            coverage_ratio: coverage,
        }
    }

    #[must_use]
    pub fn is_adequate(&self, min_coverage: f32, min_rating: f32) -> bool {
        self.coverage_ratio >= min_coverage
            && self.minimum_rating >= min_rating
            && self.overcrowded_shelters == 0
    }

    #[must_use]
    pub fn shortage(&self, population: u32) -> u32 {
        population.saturating_sub(self.total_capacity)
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.total_shelters.to_le_bytes());
        hasher.update(&self.total_capacity.to_le_bytes());
        hasher.update(&self.total_occupancy.to_le_bytes());
        hasher.update(&self.average_rating.to_le_bytes());
        hasher.finalize()
    }
}

/// Recommendation for shelter improvement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShelterRecommendation {
    pub shelter: Option<ShelterId>,
    pub category: RatingCategory,
    pub current_rating: f32,
    pub recommended_action: String,
    pub priority: i32,
}

impl ShelterRecommendation {
    #[must_use]
    pub fn new(
        shelter: Option<ShelterId>,
        category: RatingCategory,
        current_rating: f32,
        action: impl Into<String>,
        priority: i32,
    ) -> Self {
        Self {
            shelter,
            category,
            current_rating,
            recommended_action: action.into(),
            priority,
        }
    }
}

/// Generate recommendations for shelter improvements.
#[must_use]
pub fn generate_recommendations(
    registry: &ShelterRegistry,
    population: u32,
    max_recommendations: usize,
) -> Vec<ShelterRecommendation> {
    let mut recommendations = Vec::new();

    let coverage = ShelterCoverage::from_registry(registry, population, 0);
    if coverage.coverage_ratio < 1.0 {
        recommendations.push(ShelterRecommendation::new(
            None,
            RatingCategory::Safety,
            coverage.coverage_ratio,
            format!(
                "Build additional shelter capacity ({} units needed)",
                coverage.shortage(population)
            ),
            100,
        ));
    }

    for shelter in registry.iter() {
        let issues = shelter.ratings.critical_issues();
        for category in issues {
            let action = match category {
                RatingCategory::Safety => "Reinforce structural integrity",
                RatingCategory::Pressure => "Repair pressure seals",
                RatingCategory::AirQuality => "Install air filtration",
                RatingCategory::Warmth => "Add heating system",
                RatingCategory::Cooling => "Add cooling system",
                RatingCategory::Supplies => "Stockpile emergency supplies",
                RatingCategory::Crowding => "Reduce occupancy or expand",
                RatingCategory::HazardExposure => "Add protective barriers",
                _ => "Address issue",
            };

            recommendations.push(ShelterRecommendation::new(
                Some(shelter.id),
                category,
                shelter.ratings.get(category).raw(),
                action,
                90,
            ));
        }

        if shelter.is_overcrowded() {
            recommendations.push(ShelterRecommendation::new(
                Some(shelter.id),
                RatingCategory::Crowding,
                shelter.ratings.crowding.raw(),
                "Relocate occupants to reduce crowding",
                80,
            ));
        }
    }

    recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));
    recommendations.truncate(max_recommendations);
    recommendations
}

/// Fingerprint for shelter state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShelterFingerprint(pub u32);

impl ShelterFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "count bounded by game limits"
    )]
    pub fn from_registry(registry: &ShelterRegistry, tick: u64) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&tick.to_le_bytes());
        hasher.update(&u64::from(registry.count() as u32).to_le_bytes());
        hasher.update(&u64::from(registry.total_capacity()).to_le_bytes());
        hasher.update(&u64::from(registry.total_occupancy()).to_le_bytes());

        for shelter in registry.iter() {
            hasher.update(&shelter.id.raw().to_le_bytes());
            hasher.update(&shelter.current_occupancy.to_le_bytes());
            hasher.update(&u8::from(shelter.is_operational).to_le_bytes());
        }

        Self(hasher.finalize())
    }
}

impl std::fmt::Display for ShelterFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shelter:{:08x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_basic() {
        let rating = Rating::new(0.75);
        assert!((rating.raw() - 0.75).abs() < 0.001);
        assert!(rating.is_good());
        assert!(!rating.is_critical());
    }

    #[test]
    fn test_rating_clamping() {
        let low = Rating::new(-0.5);
        let high = Rating::new(1.5);

        assert!((low.raw() - 0.0).abs() < 0.001);
        assert!((high.raw() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rating_categories() {
        let critical = Rating::new(0.1);
        let poor = Rating::new(0.3);
        let adequate = Rating::new(0.5);
        let good = Rating::new(0.8);
        let excellent = Rating::new(0.95);

        assert!(critical.is_critical());
        assert!(poor.is_poor());
        assert!(adequate.is_adequate());
        assert!(good.is_good());
        assert!(excellent.is_excellent());
    }

    #[test]
    fn test_shelter_ratings_weighted_average() {
        let ratings = ShelterRatings::new()
            .with_safety(1.0)
            .with_comfort(0.5)
            .with_warmth(0.8)
            .with_cooling(0.8)
            .with_air_quality(0.9)
            .with_pressure(1.0)
            .with_supplies(0.6)
            .with_crowding(0.7)
            .with_access(0.5)
            .with_hazard_exposure(0.8);

        let weights = ShelterWeights::default();
        let avg = ratings.weighted_average(&weights);

        assert!(avg > 0.0);
        assert!(avg <= 1.0);
    }

    #[test]
    fn test_shelter_ratings_critical_issues() {
        let ratings = ShelterRatings::new()
            .with_safety(0.1)
            .with_pressure(0.15)
            .with_comfort(0.8);

        let issues = ratings.critical_issues();
        assert!(issues.contains(&RatingCategory::Safety));
        assert!(issues.contains(&RatingCategory::Pressure));
        assert!(!issues.contains(&RatingCategory::Comfort));
    }

    #[test]
    fn test_shelter_basic() {
        let shelter = Shelter::new(ShelterId::new(1), "Main Hab", 100, 0);

        assert_eq!(shelter.capacity, 100);
        assert_eq!(shelter.current_occupancy, 0);
        assert!(shelter.is_operational);
        assert!(!shelter.is_full());
    }

    #[test]
    fn test_shelter_occupancy() {
        let mut shelter = Shelter::new(ShelterId::new(1), "Test Hab", 50, 0);

        shelter.admit(30, 10);
        assert_eq!(shelter.current_occupancy, 30);
        assert_eq!(shelter.available_capacity(), 20);
        assert!(!shelter.is_full());

        shelter.admit(20, 20);
        assert!(shelter.is_full());

        shelter.admit(10, 30);
        assert!(shelter.is_overcrowded());
    }

    #[test]
    fn test_shelter_crowding_rating() {
        let mut shelter = Shelter::new(ShelterId::new(1), "Test", 100, 0);

        shelter.current_occupancy = 50;
        shelter.update_crowding_rating();
        assert!(shelter.ratings.crowding.raw() >= 0.9);

        shelter.current_occupancy = 95;
        shelter.update_crowding_rating();
        assert!(shelter.ratings.crowding.raw() < 0.9);

        shelter.current_occupancy = 120;
        shelter.update_crowding_rating();
        assert!(shelter.ratings.crowding.raw() < 0.4);
    }

    #[test]
    fn test_shelter_registry() {
        let mut registry = ShelterRegistry::new();

        let id1 = registry.create("Hab 1", 100, 0);
        let _id2 = registry.create("Hab 2", 50, 0);

        assert_eq!(registry.count(), 2);
        assert_eq!(registry.total_capacity(), 150);

        let shelter = registry.get_mut(id1).unwrap();
        shelter.admit(30, 10);

        assert_eq!(registry.total_occupancy(), 30);
        assert_eq!(registry.total_available(), 120);
    }

    #[test]
    fn test_shelter_registry_best_shelter() {
        let mut registry = ShelterRegistry::new();

        let id1 = registry.create("Poor Hab", 100, 0);
        let id2 = registry.create("Good Hab", 100, 0);

        {
            let shelter = registry.get_mut(id1).unwrap();
            shelter.ratings = ShelterRatings::new().with_safety(0.3);
        }
        {
            let shelter = registry.get_mut(id2).unwrap();
            shelter.ratings = ShelterRatings::new().with_safety(0.9);
        }

        let weights = ShelterWeights::default();
        let best = registry.best_shelter(&weights, 1);

        assert_eq!(best, Some(id2));
    }

    #[test]
    fn test_shelter_coverage() {
        let mut registry = ShelterRegistry::new();
        registry.create("Hab 1", 100, 0);
        registry.create("Hab 2", 100, 0);

        let coverage = ShelterCoverage::from_registry(&registry, 150, 100);

        assert_eq!(coverage.total_shelters, 2);
        assert_eq!(coverage.total_capacity, 200);
        assert!(coverage.coverage_ratio >= 1.0);
        assert!(coverage.is_adequate(1.0, 0.3));
    }

    #[test]
    fn test_shelter_coverage_shortage() {
        let mut registry = ShelterRegistry::new();
        registry.create("Small Hab", 50, 0);

        let coverage = ShelterCoverage::from_registry(&registry, 100, 0);

        assert!(coverage.coverage_ratio < 1.0);
        assert_eq!(coverage.shortage(100), 50);
    }

    #[test]
    fn test_generate_recommendations() {
        let mut registry = ShelterRegistry::new();
        let id = registry.create("Damaged Hab", 100, 0);

        {
            let shelter = registry.get_mut(id).unwrap();
            shelter.ratings = ShelterRatings::new()
                .with_safety(0.1)
                .with_pressure(0.1)
                .with_air_quality(0.8);
        }

        let recommendations = generate_recommendations(&registry, 100, 10);

        assert!(!recommendations.is_empty());
        assert!(
            recommendations
                .iter()
                .any(|r| r.category == RatingCategory::Safety)
        );
        assert!(
            recommendations
                .iter()
                .any(|r| r.category == RatingCategory::Pressure)
        );
    }

    #[test]
    fn test_shelter_fingerprint() {
        let mut registry = ShelterRegistry::new();
        registry.create("Hab", 100, 0);

        let fp1 = ShelterFingerprint::from_registry(&registry, 0);
        let fp2 = ShelterFingerprint::from_registry(&registry, 0);

        assert_eq!(fp1, fp2);
        assert_eq!(format!("{fp1}"), format!("shelter:{:08x}", fp1.raw()));
    }

    #[test]
    fn test_shelter_fingerprint_changes() {
        let mut registry = ShelterRegistry::new();
        let id = registry.create("Hab", 100, 0);

        let fp_before = ShelterFingerprint::from_registry(&registry, 0);

        registry.get_mut(id).unwrap().admit(50, 10);

        let fp_after = ShelterFingerprint::from_registry(&registry, 0);

        assert_ne!(fp_before, fp_after);
    }

    #[test]
    fn test_serde_shelter() {
        let shelter = Shelter::new(ShelterId::new(42), "Test Hab", 100, 50)
            .with_distance(10.0)
            .with_ratings(ShelterRatings::new().with_safety(0.9));

        let json = serde_json::to_string(&shelter).unwrap();
        let restored: Shelter = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, shelter.id);
        assert_eq!(restored.capacity, shelter.capacity);
        assert!((restored.distance_from_center - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_serde_coverage() {
        let coverage = ShelterCoverage {
            tick: 100,
            total_shelters: 5,
            operational_shelters: 4,
            total_capacity: 500,
            total_occupancy: 300,
            average_rating: 0.75,
            minimum_rating: 0.5,
            shelters_with_critical_issues: 1,
            overcrowded_shelters: 0,
            coverage_ratio: 1.0,
        };

        let json = serde_json::to_string(&coverage).unwrap();
        let restored: ShelterCoverage = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, coverage);
    }

    #[test]
    fn test_bincode_shelter() {
        let shelter = Shelter::new(ShelterId::new(99), "Bincode Hab", 200, 100);

        let bytes = bincode::serialize(&shelter).unwrap();
        let restored: Shelter = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id.raw(), 99);
        assert_eq!(restored.capacity, 200);
    }

    #[test]
    fn test_bincode_ratings() {
        let ratings = ShelterRatings::new()
            .with_safety(0.9)
            .with_comfort(0.7)
            .with_pressure(0.8);

        let bytes = bincode::serialize(&ratings).unwrap();
        let restored: ShelterRatings = bincode::deserialize(&bytes).unwrap();

        assert!((restored.safety.raw() - 0.9).abs() < 0.001);
        assert!((restored.comfort.raw() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_bincode_coverage() {
        let mut registry = ShelterRegistry::new();
        registry.create("Hab", 100, 0);
        let coverage = ShelterCoverage::from_registry(&registry, 50, 100);

        let bytes = bincode::serialize(&coverage).unwrap();
        let restored: ShelterCoverage = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 100);
        assert_eq!(restored.total_shelters, 1);
    }

    #[test]
    fn test_bincode_fingerprint() {
        let fp = ShelterFingerprint(0xCAFE_BABE);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: ShelterFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.raw(), 0xCAFE_BABE);
    }

    #[test]
    fn test_bincode_recommendation() {
        let rec = ShelterRecommendation::new(
            Some(ShelterId::new(5)),
            RatingCategory::Safety,
            0.3,
            "Fix it",
            90,
        );

        let bytes = bincode::serialize(&rec).unwrap();
        let restored: ShelterRecommendation = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.shelter, Some(ShelterId::new(5)));
        assert_eq!(restored.priority, 90);
    }
}

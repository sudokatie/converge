//! Interest management for simulation scheduling.
//!
//! Provides field/hazard relevance tracking to augment distance-based fidelity.
//! Regions with active hazards or important fields can receive higher simulation
//! attention without requiring observer proximity.
//!
//! # Design
//!
//! Interest sources are categorized by type (scalar field, vector field, hazard, etc.)
//! and assigned relevance weights. The system tracks staleness via tick counts and
//! supports automatic decay. Interest scores influence priority calculation and
//! environment hint generation without replacing the core distance-based fidelity.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::environment::{FieldChannel, HazardKind, VectorFieldChannel};

/// Category of interest source for a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InterestCategory {
    /// Interest from a scalar environmental field (temperature, radiation, etc.).
    ScalarField(FieldChannel),

    /// Interest from a vector environmental field (wind, current, etc.).
    VectorField(VectorFieldChannel),

    /// Interest from an active hazard (fire, infection, etc.).
    Hazard(HazardKind),

    /// Interest from structural activity (load changes, stress, collapse risk).
    Structural,

    /// Interest from fluid dynamics activity.
    Fluid,

    /// Interest from conduit network activity.
    Conduit,

    /// Manual interest boost with a user-defined tag (0-255).
    Manual(u8),
}

impl InterestCategory {
    /// Whether this category affects scalar field simulation hints.
    #[must_use]
    pub const fn affects_scalar_fields(&self) -> bool {
        matches!(self, InterestCategory::ScalarField(_))
    }

    /// Whether this category affects vector field simulation hints.
    #[must_use]
    pub const fn affects_vector_fields(&self) -> bool {
        matches!(self, InterestCategory::VectorField(_))
    }

    /// Whether this category affects hazard spread simulation hints.
    #[must_use]
    pub const fn affects_hazard_spread(&self) -> bool {
        matches!(self, InterestCategory::Hazard(_))
    }
}

/// A single interest entry tracking relevance and staleness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterestEntry {
    /// Relevance weight (higher = more important). Typically 0.0-1.0.
    weight: f32,

    /// Tick when this interest was last refreshed.
    last_refresh_tick: u64,
}

impl InterestEntry {
    /// Create a new interest entry.
    #[must_use]
    pub fn new(weight: f32, current_tick: u64) -> Self {
        Self {
            weight: weight.max(0.0),
            last_refresh_tick: current_tick,
        }
    }

    /// Get the relevance weight.
    #[must_use]
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Get the tick when this interest was last refreshed.
    #[must_use]
    pub fn last_refresh_tick(&self) -> u64 {
        self.last_refresh_tick
    }

    /// Calculate staleness in ticks since last refresh.
    #[must_use]
    pub fn staleness(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_refresh_tick)
    }

    /// Update weight and refresh timestamp.
    pub fn refresh(&mut self, weight: f32, current_tick: u64) {
        self.weight = weight.max(0.0);
        self.last_refresh_tick = current_tick;
    }

    /// Apply decay to the weight. Returns true if weight is still positive.
    pub fn decay(&mut self, factor: f32) -> bool {
        self.weight *= factor.clamp(0.0, 1.0);
        self.weight > 0.0
    }
}

/// Interest tracking for a single region.
///
/// Collects interest sources with weights and staleness, computes aggregate
/// scores, and tracks which environment systems should be active.
#[derive(Clone, Debug, Default)]
pub struct RegionInterest {
    /// Interest entries keyed by category. `BTreeMap` ensures deterministic iteration.
    entries: BTreeMap<InterestCategory, InterestEntry>,
}

impl RegionInterest {
    /// Create empty interest tracking.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any interest is tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of tracked interest sources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Set or update interest for a category.
    pub fn set(&mut self, category: InterestCategory, weight: f32, current_tick: u64) {
        if weight <= 0.0 {
            self.entries.remove(&category);
        } else {
            self.entries
                .entry(category)
                .and_modify(|e| e.refresh(weight, current_tick))
                .or_insert_with(|| InterestEntry::new(weight, current_tick));
        }
    }

    /// Get interest entry for a category.
    #[must_use]
    pub fn get(&self, category: InterestCategory) -> Option<&InterestEntry> {
        self.entries.get(&category)
    }

    /// Remove interest for a category.
    pub fn remove(&mut self, category: InterestCategory) -> bool {
        self.entries.remove(&category).is_some()
    }

    /// Clear all interest entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Apply decay to all entries, removing those that decay to zero.
    ///
    /// Returns the number of entries removed.
    pub fn decay_all(&mut self, factor: f32) -> usize {
        let initial_len = self.entries.len();
        self.entries.retain(|_, entry| entry.decay(factor));
        initial_len - self.entries.len()
    }

    /// Remove entries that exceed the staleness threshold.
    ///
    /// Returns the number of entries removed.
    pub fn prune_stale(&mut self, current_tick: u64, max_staleness: u64) -> usize {
        let initial_len = self.entries.len();
        self.entries
            .retain(|_, entry| entry.staleness(current_tick) <= max_staleness);
        initial_len - self.entries.len()
    }

    /// Compute the total interest score (sum of all weights).
    #[must_use]
    pub fn total_score(&self) -> f32 {
        self.entries.values().map(InterestEntry::weight).sum()
    }

    /// Compute the maximum interest weight across all entries.
    #[must_use]
    pub fn max_weight(&self) -> f32 {
        self.entries
            .values()
            .map(InterestEntry::weight)
            .fold(0.0, f32::max)
    }

    /// Check if scalar field interest is present.
    #[must_use]
    pub fn has_scalar_field_interest(&self) -> bool {
        self.entries
            .keys()
            .any(InterestCategory::affects_scalar_fields)
    }

    /// Check if vector field interest is present.
    #[must_use]
    pub fn has_vector_field_interest(&self) -> bool {
        self.entries
            .keys()
            .any(InterestCategory::affects_vector_fields)
    }

    /// Check if hazard spread interest is present.
    #[must_use]
    pub fn has_hazard_interest(&self) -> bool {
        self.entries
            .keys()
            .any(InterestCategory::affects_hazard_spread)
    }

    /// Iterate over all entries in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&InterestCategory, &InterestEntry)> {
        self.entries.iter()
    }

    /// Get categories with weight above a threshold.
    pub fn active_categories(&self, min_weight: f32) -> Vec<InterestCategory> {
        self.entries
            .iter()
            .filter(|(_, e)| e.weight() >= min_weight)
            .map(|(c, _)| *c)
            .collect()
    }
}

/// Configuration for interest-based priority and hint generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterestConfig {
    /// Multiplier for converting interest score to priority boost.
    pub score_to_priority_multiplier: f32,

    /// Minimum interest score to enable scalar field hints.
    pub scalar_field_threshold: f32,

    /// Minimum interest score to enable vector field hints.
    pub vector_field_threshold: f32,

    /// Minimum interest score to enable hazard spread hints.
    pub hazard_threshold: f32,

    /// Decay factor applied per tick (0.0-1.0, 1.0 = no decay).
    pub decay_factor: f32,

    /// Maximum staleness in ticks before interest is pruned.
    pub max_staleness_ticks: u64,

    /// Whether interest can override dormant fidelity to distant.
    pub can_promote_dormant: bool,
}

impl Default for InterestConfig {
    fn default() -> Self {
        Self {
            score_to_priority_multiplier: 50_000.0,
            scalar_field_threshold: 0.1,
            vector_field_threshold: 0.2,
            hazard_threshold: 0.15,
            decay_factor: 1.0,
            max_staleness_ticks: 600,
            can_promote_dormant: true,
        }
    }
}

impl InterestConfig {
    /// Calculate priority boost from interest score.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "i32::MAX as f32 loses precision but is acceptable for clamping"
    )]
    pub fn priority_boost(&self, score: f32) -> i32 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped to i32 range, truncation is intended"
        )]
        let boost = (score * self.score_to_priority_multiplier).clamp(0.0, i32::MAX as f32) as i32;
        boost
    }
}

/// Summary of interest state for a region.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterestSummary {
    /// Total interest score.
    pub total_score: f32,

    /// Maximum single-entry weight.
    pub max_weight: f32,

    /// Number of active interest entries.
    pub entry_count: usize,

    /// Whether scalar field interest is present.
    pub has_scalar_fields: bool,

    /// Whether vector field interest is present.
    pub has_vector_fields: bool,

    /// Whether hazard interest is present.
    pub has_hazards: bool,
}

impl InterestSummary {
    /// Create a summary from region interest.
    #[must_use]
    pub fn from_interest(interest: &RegionInterest) -> Self {
        Self {
            total_score: interest.total_score(),
            max_weight: interest.max_weight(),
            entry_count: interest.len(),
            has_scalar_fields: interest.has_scalar_field_interest(),
            has_vector_fields: interest.has_vector_field_interest(),
            has_hazards: interest.has_hazard_interest(),
        }
    }

    /// Check if this summary indicates meaningful interest.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.entry_count > 0 && self.total_score > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interest_entry_new() {
        let entry = InterestEntry::new(0.5, 100);
        assert!((entry.weight() - 0.5).abs() < f32::EPSILON);
        assert_eq!(entry.last_refresh_tick(), 100);
    }

    #[test]
    fn interest_entry_negative_weight_clamped() {
        let entry = InterestEntry::new(-0.5, 0);
        assert!(entry.weight() >= 0.0);
    }

    #[test]
    fn interest_entry_staleness() {
        let entry = InterestEntry::new(0.5, 100);
        assert_eq!(entry.staleness(150), 50);
        assert_eq!(entry.staleness(100), 0);
        assert_eq!(entry.staleness(50), 0); // saturating
    }

    #[test]
    fn interest_entry_refresh() {
        let mut entry = InterestEntry::new(0.5, 100);
        entry.refresh(0.8, 200);
        assert!((entry.weight() - 0.8).abs() < f32::EPSILON);
        assert_eq!(entry.last_refresh_tick(), 200);
    }

    #[test]
    fn interest_entry_decay() {
        let mut entry = InterestEntry::new(1.0, 0);
        assert!(entry.decay(0.5));
        assert!((entry.weight() - 0.5).abs() < f32::EPSILON);
        assert!(entry.decay(0.5));
        assert!((entry.weight() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn interest_entry_decay_to_zero() {
        let mut entry = InterestEntry::new(1.0, 0);
        // Decay by 0.01 repeatedly until weight becomes effectively zero
        for _ in 0..1000 {
            if !entry.decay(0.01) {
                return; // Successfully decayed to zero
            }
        }
        // After aggressive decay, weight should be practically zero
        assert!(entry.weight() < 1e-30);
    }

    #[test]
    fn region_interest_empty() {
        let interest = RegionInterest::new();
        assert!(interest.is_empty());
        assert_eq!(interest.len(), 0);
        assert!(interest.total_score().abs() < f32::EPSILON);
    }

    #[test]
    fn region_interest_set_and_get() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 100);

        assert!(!interest.is_empty());
        let entry = interest
            .get(InterestCategory::Hazard(HazardKind::Fire))
            .unwrap();
        assert!((entry.weight() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn region_interest_set_zero_removes() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Structural, 0.5, 0);
        assert_eq!(interest.len(), 1);

        interest.set(InterestCategory::Structural, 0.0, 10);
        assert!(interest.is_empty());
    }

    #[test]
    fn region_interest_remove() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Fluid, 0.5, 0);
        assert!(interest.remove(InterestCategory::Fluid));
        assert!(!interest.remove(InterestCategory::Fluid));
    }

    #[test]
    fn region_interest_clear() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Fluid, 0.5, 0);
        interest.set(InterestCategory::Conduit, 0.3, 0);
        interest.clear();
        assert!(interest.is_empty());
    }

    #[test]
    fn region_interest_decay_all() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 1.0, 0);
        interest.set(InterestCategory::Structural, 1.0, 0);

        // Single decay reduces weights
        let removed = interest.decay_all(0.5);
        assert_eq!(removed, 0);
        assert!((interest.total_score() - 1.0).abs() < f32::EPSILON);

        // Keep decaying with aggressive factor until entries are removed
        for _ in 0..1000 {
            interest.decay_all(0.01);
        }
        // After extreme decay, entries should be removed
        assert!(interest.is_empty() || interest.total_score() < 1e-30);
    }

    #[test]
    fn region_interest_prune_stale() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);
        interest.set(InterestCategory::Structural, 0.5, 80);

        // At tick 100, fire is 100 ticks stale, structural is 20 ticks stale
        let removed = interest.prune_stale(100, 50);
        assert_eq!(removed, 1); // fire is too stale, structural is within threshold
        assert_eq!(interest.len(), 1);
        assert!(interest.get(InterestCategory::Structural).is_some());
    }

    #[test]
    fn region_interest_total_score() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.5, 0);
        interest.set(InterestCategory::Structural, 0.3, 0);
        interest.set(InterestCategory::Fluid, 0.2, 0);

        assert!((interest.total_score() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn region_interest_max_weight() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.5, 0);
        interest.set(InterestCategory::Structural, 0.9, 0);
        interest.set(InterestCategory::Fluid, 0.2, 0);

        assert!((interest.max_weight() - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn region_interest_has_category_interests() {
        let mut interest = RegionInterest::new();
        interest.set(
            InterestCategory::ScalarField(FieldChannel::Temperature),
            0.5,
            0,
        );
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.5, 0);

        assert!(interest.has_scalar_field_interest());
        assert!(!interest.has_vector_field_interest());
        assert!(interest.has_hazard_interest());
    }

    #[test]
    fn region_interest_active_categories() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);
        interest.set(InterestCategory::Structural, 0.05, 0);
        interest.set(InterestCategory::Fluid, 0.3, 0);

        let active = interest.active_categories(0.1);
        assert_eq!(active.len(), 2);
        assert!(active.contains(&InterestCategory::Hazard(HazardKind::Fire)));
        assert!(active.contains(&InterestCategory::Fluid));
    }

    #[test]
    fn region_interest_deterministic_iteration() {
        let mut interest1 = RegionInterest::new();
        interest1.set(InterestCategory::Fluid, 0.3, 0);
        interest1.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);
        interest1.set(InterestCategory::Structural, 0.5, 0);

        let mut interest2 = RegionInterest::new();
        interest2.set(InterestCategory::Structural, 0.5, 0);
        interest2.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);
        interest2.set(InterestCategory::Fluid, 0.3, 0);

        let cats1: Vec<_> = interest1.iter().map(|(c, _)| c).collect();
        let cats2: Vec<_> = interest2.iter().map(|(c, _)| c).collect();
        assert_eq!(cats1, cats2);
    }

    #[test]
    fn interest_config_default() {
        let config = InterestConfig::default();
        assert!(config.score_to_priority_multiplier > 0.0);
        assert!(config.decay_factor > 0.0);
        assert!(config.max_staleness_ticks > 0);
    }

    #[test]
    fn interest_config_priority_boost() {
        let config = InterestConfig::default();
        let boost = config.priority_boost(1.0);
        assert_eq!(boost, 50_000);

        let boost = config.priority_boost(0.5);
        assert_eq!(boost, 25_000);
    }

    #[test]
    fn interest_config_priority_boost_clamped() {
        let config = InterestConfig {
            score_to_priority_multiplier: 1e12,
            ..Default::default()
        };
        let boost = config.priority_boost(1e6);
        // Verify the result is positive and at a reasonable max (clamping worked)
        assert!(boost > 0);
        assert_eq!(boost, i32::MAX);
    }

    #[test]
    fn interest_summary_from_interest() {
        let mut interest = RegionInterest::new();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);
        interest.set(
            InterestCategory::ScalarField(FieldChannel::Radiation),
            0.5,
            0,
        );

        let summary = InterestSummary::from_interest(&interest);
        assert!((summary.total_score - 1.3).abs() < f32::EPSILON);
        assert!((summary.max_weight - 0.8).abs() < f32::EPSILON);
        assert_eq!(summary.entry_count, 2);
        assert!(summary.has_scalar_fields);
        assert!(!summary.has_vector_fields);
        assert!(summary.has_hazards);
        assert!(summary.is_active());
    }

    #[test]
    fn interest_summary_empty() {
        let interest = RegionInterest::new();
        let summary = InterestSummary::from_interest(&interest);
        assert!(!summary.is_active());
        assert_eq!(summary.entry_count, 0);
    }

    #[test]
    fn interest_category_serde_roundtrip() {
        let categories = [
            InterestCategory::ScalarField(FieldChannel::Temperature),
            InterestCategory::VectorField(VectorFieldChannel::Wind),
            InterestCategory::Hazard(HazardKind::Fire),
            InterestCategory::Structural,
            InterestCategory::Fluid,
            InterestCategory::Conduit,
            InterestCategory::Manual(42),
        ];

        for cat in categories {
            let json = serde_json::to_string(&cat).unwrap();
            let recovered: InterestCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, cat);
        }
    }
}

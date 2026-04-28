//! Regional population state and zone bias.

use super::budget::{DespawnBudget, SpawnBudget};
use super::species::{SpeciesCapId, SpeciesRegistry};
use super::threat::RegionalThreat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bias toward safe or hostile creature spawns.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ZoneBias {
    /// Only safe creatures spawn.
    SafeOnly,
    /// Prefer safe creatures.
    SafePreferred,
    /// Balanced spawning.
    #[default]
    Balanced,
    /// Prefer hostile creatures.
    HostilePreferred,
    /// Only hostile creatures spawn.
    HostileOnly,
}

impl ZoneBias {
    /// Get multiplier for hostile spawns.
    #[must_use]
    pub fn hostile_multiplier(self) -> f32 {
        match self {
            Self::SafeOnly => 0.0,
            Self::SafePreferred => 0.3,
            Self::Balanced => 1.0,
            Self::HostilePreferred => 1.5,
            Self::HostileOnly => 2.0,
        }
    }

    /// Get multiplier for passive spawns.
    #[must_use]
    pub fn passive_multiplier(self) -> f32 {
        match self {
            Self::SafeOnly => 2.0,
            Self::SafePreferred => 1.5,
            Self::Balanced => 1.0,
            Self::HostilePreferred => 0.5,
            Self::HostileOnly => 0.0,
        }
    }

    /// Create from threat level multiplier (0.0 = safe, 2.0 = hostile only).
    #[must_use]
    pub fn from_hostile_multiplier(multiplier: f32) -> Self {
        if multiplier <= 0.0 {
            Self::SafeOnly
        } else if multiplier < 0.7 {
            Self::SafePreferred
        } else if multiplier < 1.3 {
            Self::Balanced
        } else if multiplier < 2.0 {
            Self::HostilePreferred
        } else {
            Self::HostileOnly
        }
    }
}

/// Population counts for a region.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PopulationCounts {
    /// Per-species population counts.
    species: BTreeMap<SpeciesCapId, u32>,
    /// Total population.
    total: u32,
    /// Hostile population.
    hostile: u32,
    /// Passive population.
    passive: u32,
    /// Last update tick.
    last_update_tick: u64,
}

impl PopulationCounts {
    /// Create new population counts.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get count for a species.
    #[must_use]
    pub fn get(&self, species: &SpeciesCapId) -> u32 {
        self.species.get(species).copied().unwrap_or(0)
    }

    /// Set count for a species.
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "totals are maintained non-negative and bounded"
    )]
    pub fn set(&mut self, species: SpeciesCapId, count: u32, is_hostile: bool) {
        let old_count = self.species.insert(species, count).unwrap_or(0);
        let delta = i64::from(count) - i64::from(old_count);

        self.total = (i64::from(self.total) + delta).max(0) as u32;
        if is_hostile {
            self.hostile = (i64::from(self.hostile) + delta).max(0) as u32;
        } else {
            self.passive = (i64::from(self.passive) + delta).max(0) as u32;
        }
    }

    /// Increment count for a species.
    pub fn increment(&mut self, species: &SpeciesCapId, is_hostile: bool) {
        *self.species.entry(species.clone()).or_insert(0) += 1;
        self.total += 1;
        if is_hostile {
            self.hostile += 1;
        } else {
            self.passive += 1;
        }
    }

    /// Decrement count for a species.
    pub fn decrement(&mut self, species: &SpeciesCapId, is_hostile: bool) {
        if let Some(count) = self.species.get_mut(species) {
            *count = count.saturating_sub(1);
        }
        self.total = self.total.saturating_sub(1);
        if is_hostile {
            self.hostile = self.hostile.saturating_sub(1);
        } else {
            self.passive = self.passive.saturating_sub(1);
        }
    }

    /// Get total population.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Get hostile population.
    #[must_use]
    pub fn hostile(&self) -> u32 {
        self.hostile
    }

    /// Get passive population.
    #[must_use]
    pub fn passive(&self) -> u32 {
        self.passive
    }

    /// Get hostile ratio (0.0-1.0).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "population counts are bounded")]
    pub fn hostile_ratio(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.hostile as f32 / self.total as f32
        }
    }

    /// Iterate over species counts (deterministic order).
    pub fn iter(&self) -> impl Iterator<Item = (&SpeciesCapId, u32)> {
        self.species.iter().map(|(k, &v)| (k, v))
    }

    /// Update from registry.
    pub fn sync_from_registry(&mut self, registry: &SpeciesRegistry, tick: u64) {
        self.species.clear();
        self.total = 0;
        self.hostile = 0;
        self.passive = 0;

        for species in registry.iter_species() {
            let count = species.current();
            if count > 0 {
                self.species.insert(species.id.clone(), count);
                self.total += count;
                if species.hostile {
                    self.hostile += count;
                } else {
                    self.passive += count;
                }
            }
        }

        self.last_update_tick = tick;
    }
}

/// Regional population management state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionalPopulation {
    /// Region identifier.
    pub region_id: String,
    /// Population counts.
    counts: PopulationCounts,
    /// Spawn budget.
    spawn_budget: SpawnBudget,
    /// Despawn budget.
    despawn_budget: DespawnBudget,
    /// Threat state.
    threat: RegionalThreat,
    /// Zone bias.
    bias: ZoneBias,
    /// Soft population cap for region.
    pub soft_cap: u32,
    /// Hard population cap for region.
    pub hard_cap: u32,
    /// Whether region is loaded.
    pub loaded: bool,
    /// Current tick.
    current_tick: u64,
}

impl RegionalPopulation {
    /// Create a new regional population tracker.
    #[must_use]
    pub fn new(region_id: impl Into<String>) -> Self {
        Self {
            region_id: region_id.into(),
            counts: PopulationCounts::new(),
            spawn_budget: SpawnBudget::default(),
            despawn_budget: DespawnBudget::new(5),
            threat: RegionalThreat::default(),
            bias: ZoneBias::Balanced,
            soft_cap: 100,
            hard_cap: 200,
            loaded: true,
            current_tick: 0,
        }
    }

    #[must_use]
    pub fn with_caps(mut self, soft_cap: u32, hard_cap: u32) -> Self {
        self.soft_cap = soft_cap;
        self.hard_cap = hard_cap;
        self
    }

    #[must_use]
    pub fn with_spawn_budget(mut self, budget: SpawnBudget) -> Self {
        self.spawn_budget = budget;
        self
    }

    #[must_use]
    pub fn with_despawn_budget(mut self, budget: DespawnBudget) -> Self {
        self.despawn_budget = budget;
        self
    }

    #[must_use]
    pub fn with_threat(mut self, threat: RegionalThreat) -> Self {
        self.threat = threat;
        self
    }

    #[must_use]
    pub fn with_bias(mut self, bias: ZoneBias) -> Self {
        self.bias = bias;
        self
    }

    /// Get population counts.
    #[must_use]
    pub fn counts(&self) -> &PopulationCounts {
        &self.counts
    }

    /// Get mutable population counts.
    pub fn counts_mut(&mut self) -> &mut PopulationCounts {
        &mut self.counts
    }

    /// Get spawn budget.
    #[must_use]
    pub fn spawn_budget(&self) -> &SpawnBudget {
        &self.spawn_budget
    }

    /// Get mutable spawn budget.
    pub fn spawn_budget_mut(&mut self) -> &mut SpawnBudget {
        &mut self.spawn_budget
    }

    /// Get despawn budget.
    #[must_use]
    pub fn despawn_budget(&self) -> &DespawnBudget {
        &self.despawn_budget
    }

    /// Get mutable despawn budget.
    pub fn despawn_budget_mut(&mut self) -> &mut DespawnBudget {
        &mut self.despawn_budget
    }

    /// Get threat state.
    #[must_use]
    pub fn threat(&self) -> &RegionalThreat {
        &self.threat
    }

    /// Get mutable threat state.
    pub fn threat_mut(&mut self) -> &mut RegionalThreat {
        &mut self.threat
    }

    /// Get zone bias.
    #[must_use]
    pub fn bias(&self) -> ZoneBias {
        self.bias
    }

    /// Set zone bias.
    pub fn set_bias(&mut self, bias: ZoneBias) {
        self.bias = bias;
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Calculate population pressure (0.0 = empty, 1.0 = at hard cap).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "population counts are bounded")]
    pub fn pressure(&self) -> f32 {
        if self.hard_cap == 0 {
            return 1.0;
        }
        (self.counts.total() as f32 / self.hard_cap as f32).clamp(0.0, 1.0)
    }

    /// Check if at hard cap.
    #[must_use]
    pub fn at_hard_cap(&self) -> bool {
        self.counts.total() >= self.hard_cap
    }

    /// Check if above soft cap.
    #[must_use]
    pub fn above_soft_cap(&self) -> bool {
        self.counts.total() > self.soft_cap
    }

    /// Get room for more spawns.
    #[must_use]
    pub fn spawn_room(&self) -> u32 {
        self.hard_cap.saturating_sub(self.counts.total())
    }

    /// Get excess over soft cap.
    #[must_use]
    pub fn excess(&self) -> u32 {
        self.counts.total().saturating_sub(self.soft_cap)
    }

    /// Calculate effective spawn multiplier considering all factors.
    #[must_use]
    pub fn effective_spawn_multiplier(&self, is_hostile: bool) -> f32 {
        let base = self.spawn_budget.pacing().spawn_multiplier();
        let pressure_factor = 1.0 - self.pressure();

        let type_factor = if is_hostile {
            self.bias.hostile_multiplier() * self.threat.hostile_spawn_multiplier()
        } else {
            self.bias.passive_multiplier() * self.threat.passive_spawn_multiplier()
        };

        base * pressure_factor * type_factor
    }

    /// Tick the region.
    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.spawn_budget.tick();
        self.despawn_budget.reset();
        self.threat.tick(self.current_tick);
    }

    /// Update bias from threat level.
    pub fn update_bias_from_threat(&mut self) {
        let multiplier = self.threat.hostile_spawn_multiplier();
        self.bias = ZoneBias::from_hostile_multiplier(multiplier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::population::threat::{ThreatConfig, ThreatLevel};

    #[test]
    fn test_zone_bias_multipliers() {
        assert!((ZoneBias::SafeOnly.hostile_multiplier()).abs() < f32::EPSILON);
        assert!((ZoneBias::SafeOnly.passive_multiplier() - 2.0).abs() < f32::EPSILON);

        assert!((ZoneBias::Balanced.hostile_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((ZoneBias::Balanced.passive_multiplier() - 1.0).abs() < f32::EPSILON);

        assert!(ZoneBias::HostileOnly.hostile_multiplier() > 1.0);
        assert!((ZoneBias::HostileOnly.passive_multiplier()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zone_bias_from_multiplier() {
        assert_eq!(ZoneBias::from_hostile_multiplier(0.0), ZoneBias::SafeOnly);
        assert_eq!(ZoneBias::from_hostile_multiplier(1.0), ZoneBias::Balanced);
        assert_eq!(
            ZoneBias::from_hostile_multiplier(2.0),
            ZoneBias::HostileOnly
        );
    }

    #[test]
    fn test_population_counts_new() {
        let counts = PopulationCounts::new();

        assert_eq!(counts.total(), 0);
        assert_eq!(counts.hostile(), 0);
        assert_eq!(counts.passive(), 0);
    }

    #[test]
    fn test_population_counts_set() {
        let mut counts = PopulationCounts::new();

        counts.set(SpeciesCapId::new("wolf"), 10, true);
        counts.set(SpeciesCapId::new("deer"), 20, false);

        assert_eq!(counts.get(&SpeciesCapId::new("wolf")), 10);
        assert_eq!(counts.get(&SpeciesCapId::new("deer")), 20);
        assert_eq!(counts.total(), 30);
        assert_eq!(counts.hostile(), 10);
        assert_eq!(counts.passive(), 20);
    }

    #[test]
    fn test_population_counts_increment_decrement() {
        let mut counts = PopulationCounts::new();

        counts.increment(&SpeciesCapId::new("wolf"), true);
        counts.increment(&SpeciesCapId::new("wolf"), true);
        counts.increment(&SpeciesCapId::new("deer"), false);

        assert_eq!(counts.get(&SpeciesCapId::new("wolf")), 2);
        assert_eq!(counts.total(), 3);

        counts.decrement(&SpeciesCapId::new("wolf"), true);
        assert_eq!(counts.get(&SpeciesCapId::new("wolf")), 1);
        assert_eq!(counts.total(), 2);
    }

    #[test]
    fn test_population_counts_hostile_ratio() {
        let mut counts = PopulationCounts::new();

        assert!((counts.hostile_ratio()).abs() < f32::EPSILON);

        counts.set(SpeciesCapId::new("wolf"), 25, true);
        counts.set(SpeciesCapId::new("deer"), 75, false);

        assert!((counts.hostile_ratio() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_population_counts_deterministic_iter() {
        let mut counts = PopulationCounts::new();

        counts.set(SpeciesCapId::new("zebra"), 1, false);
        counts.set(SpeciesCapId::new("ant"), 2, false);
        counts.set(SpeciesCapId::new("moose"), 3, false);

        let species: Vec<_> = counts.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(species, vec!["ant", "moose", "zebra"]);
    }

    #[test]
    fn test_regional_population_new() {
        let region = RegionalPopulation::new("test_region");

        assert_eq!(region.region_id, "test_region");
        assert_eq!(region.counts().total(), 0);
        assert!(region.loaded);
    }

    #[test]
    fn test_regional_population_caps() {
        let region = RegionalPopulation::new("test").with_caps(50, 100);

        assert_eq!(region.soft_cap, 50);
        assert_eq!(region.hard_cap, 100);
    }

    #[test]
    fn test_regional_population_pressure() {
        let mut region = RegionalPopulation::new("test").with_caps(50, 100);

        assert!((region.pressure()).abs() < f32::EPSILON);

        region.counts_mut().set(SpeciesCapId::new("wolf"), 50, true);
        assert!((region.pressure() - 0.5).abs() < f32::EPSILON);

        region
            .counts_mut()
            .set(SpeciesCapId::new("wolf"), 100, true);
        assert!((region.pressure() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_regional_population_at_cap() {
        let mut region = RegionalPopulation::new("test").with_caps(50, 100);

        assert!(!region.at_hard_cap());
        assert!(!region.above_soft_cap());

        region.counts_mut().set(SpeciesCapId::new("wolf"), 60, true);
        assert!(!region.at_hard_cap());
        assert!(region.above_soft_cap());

        region
            .counts_mut()
            .set(SpeciesCapId::new("wolf"), 100, true);
        assert!(region.at_hard_cap());
    }

    #[test]
    fn test_regional_population_spawn_room() {
        let mut region = RegionalPopulation::new("test").with_caps(50, 100);

        assert_eq!(region.spawn_room(), 100);

        region.counts_mut().set(SpeciesCapId::new("wolf"), 30, true);
        assert_eq!(region.spawn_room(), 70);
    }

    #[test]
    fn test_regional_population_excess() {
        let mut region = RegionalPopulation::new("test").with_caps(50, 100);

        assert_eq!(region.excess(), 0);

        region.counts_mut().set(SpeciesCapId::new("wolf"), 70, true);
        assert_eq!(region.excess(), 20);
    }

    #[test]
    fn test_regional_population_effective_spawn_multiplier() {
        let region = RegionalPopulation::new("test")
            .with_caps(50, 100)
            .with_bias(ZoneBias::Balanced);

        let hostile_mult = region.effective_spawn_multiplier(true);
        let passive_mult = region.effective_spawn_multiplier(false);

        assert!(hostile_mult > 0.0);
        assert!(passive_mult > 0.0);
    }

    #[test]
    fn test_regional_population_tick() {
        let mut region = RegionalPopulation::new("test");
        let initial_tick = region.current_tick();

        region.tick();

        assert_eq!(region.current_tick(), initial_tick + 1);
    }

    #[test]
    fn test_regional_population_update_bias_from_threat() {
        let threat = RegionalThreat::new(ThreatConfig::new(ThreatLevel::Safe));
        let mut region = RegionalPopulation::new("test").with_threat(threat);

        region.update_bias_from_threat();

        assert_eq!(region.bias(), ZoneBias::SafeOnly);
    }

    #[test]
    fn test_serde_population_counts() {
        let mut counts = PopulationCounts::new();
        counts.set(SpeciesCapId::new("wolf"), 10, true);
        counts.set(SpeciesCapId::new("deer"), 20, false);

        let json = serde_json::to_string(&counts).unwrap();
        let restored: PopulationCounts = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.total(), 30);
        assert_eq!(restored.get(&SpeciesCapId::new("wolf")), 10);
    }

    #[test]
    fn test_serde_regional_population() {
        let region = RegionalPopulation::new("test_region")
            .with_caps(50, 100)
            .with_bias(ZoneBias::HostilePreferred);

        let json = serde_json::to_string(&region).unwrap();
        let restored: RegionalPopulation = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id, "test_region");
        assert_eq!(restored.soft_cap, 50);
        assert_eq!(restored.bias(), ZoneBias::HostilePreferred);
    }
}

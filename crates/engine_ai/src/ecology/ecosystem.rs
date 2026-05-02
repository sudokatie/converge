//! Multi-species ecosystem simulation with population dynamics.
//!
//! Provides deterministic population balancing over time with predator-prey
//! relationships, resource carrying capacity, competition, and migration.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique identifier for a species in the ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesId(String);

impl SpeciesId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for a region in the ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EcosystemRegionId(String);

impl EcosystemRegionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Composite key identifying a population (species in a region).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PopulationKey {
    pub species: SpeciesId,
    pub region: EcosystemRegionId,
}

impl PopulationKey {
    pub fn new(species: SpeciesId, region: EcosystemRegionId) -> Self {
        Self { species, region }
    }
}

/// Trophic role of a species in the food web.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrophicRole {
    Producer,
    PrimaryConsumer,
    SecondaryConsumer,
    ApexPredator,
    Decomposer,
}

/// Base parameters for a species.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species {
    pub id: SpeciesId,
    pub name: String,
    pub trophic_role: TrophicRole,
    /// Base reproduction rate per tick (0.0 to 1.0).
    pub base_birth_rate: f32,
    /// Base mortality rate per tick (0.0 to 1.0).
    pub base_death_rate: f32,
    /// Resource units consumed per individual per tick.
    pub resource_consumption: f32,
    /// Minimum viable population before local extinction risk.
    pub minimum_viable_population: u32,
    /// Maximum density per region before overcrowding effects.
    pub max_density: u32,
}

impl Species {
    pub fn new(id: impl Into<String>, name: impl Into<String>, role: TrophicRole) -> Self {
        Self {
            id: SpeciesId::new(id),
            name: name.into(),
            trophic_role: role,
            base_birth_rate: 0.1,
            base_death_rate: 0.05,
            resource_consumption: 1.0,
            minimum_viable_population: 10,
            max_density: 1000,
        }
    }

    #[must_use]
    pub fn with_birth_rate(mut self, rate: f32) -> Self {
        self.base_birth_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_death_rate(mut self, rate: f32) -> Self {
        self.base_death_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_resource_consumption(mut self, amount: f32) -> Self {
        self.resource_consumption = amount.max(0.0);
        self
    }

    #[must_use]
    pub fn with_minimum_viable_population(mut self, mvp: u32) -> Self {
        self.minimum_viable_population = mvp;
        self
    }

    #[must_use]
    pub fn with_max_density(mut self, density: u32) -> Self {
        self.max_density = density.max(1);
        self
    }
}

/// A population of a species in a specific region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    pub key: PopulationKey,
    pub count: u32,
    /// Accumulated stress from resource scarcity (0.0 to 1.0).
    pub stress: f32,
    /// Ticks since last migration event.
    pub ticks_since_migration: u32,
}

impl Population {
    pub fn new(species: SpeciesId, region: EcosystemRegionId, count: u32) -> Self {
        Self {
            key: PopulationKey::new(species, region),
            count,
            stress: 0.0,
            ticks_since_migration: 0,
        }
    }

    pub fn is_extinct(&self) -> bool {
        self.count == 0
    }

    pub fn is_stressed(&self) -> bool {
        self.stress > 0.5
    }
}

/// Predator-prey relationship between two species.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredatorPreyRelation {
    pub predator: SpeciesId,
    pub prey: SpeciesId,
    /// Predation efficiency (prey killed per predator per tick, 0.0 to 1.0).
    pub predation_rate: f32,
    /// Conversion efficiency of prey to predator growth (0.0 to 1.0).
    pub conversion_efficiency: f32,
}

impl PredatorPreyRelation {
    pub fn new(predator: SpeciesId, prey: SpeciesId) -> Self {
        Self {
            predator,
            prey,
            predation_rate: 0.01,
            conversion_efficiency: 0.1,
        }
    }

    #[must_use]
    pub fn with_predation_rate(mut self, rate: f32) -> Self {
        self.predation_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_conversion_efficiency(mut self, efficiency: f32) -> Self {
        self.conversion_efficiency = efficiency.clamp(0.0, 1.0);
        self
    }
}

/// Competition relationship between species sharing resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorRelation {
    pub species_a: SpeciesId,
    pub species_b: SpeciesId,
    /// Competition coefficient (impact on each other's growth, 0.0 to 1.0).
    pub competition_coefficient: f32,
}

impl CompetitorRelation {
    pub fn new(species_a: SpeciesId, species_b: SpeciesId) -> Self {
        Self {
            species_a,
            species_b,
            competition_coefficient: 0.5,
        }
    }

    #[must_use]
    pub fn with_coefficient(mut self, coeff: f32) -> Self {
        self.competition_coefficient = coeff.clamp(0.0, 1.0);
        self
    }

    pub fn involves(&self, species: &SpeciesId) -> bool {
        &self.species_a == species || &self.species_b == species
    }

    pub fn other(&self, species: &SpeciesId) -> Option<&SpeciesId> {
        if &self.species_a == species {
            Some(&self.species_b)
        } else if &self.species_b == species {
            Some(&self.species_a)
        } else {
            None
        }
    }
}

/// Migration corridor between two regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCorridor {
    pub from: EcosystemRegionId,
    pub to: EcosystemRegionId,
    /// Fraction of stressed population that will migrate (0.0 to 1.0).
    pub migration_rate: f32,
    /// Species that can use this corridor (empty = all species).
    pub allowed_species: Vec<SpeciesId>,
}

impl MigrationCorridor {
    pub fn new(from: EcosystemRegionId, to: EcosystemRegionId) -> Self {
        Self {
            from,
            to,
            migration_rate: 0.1,
            allowed_species: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_migration_rate(mut self, rate: f32) -> Self {
        self.migration_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_allowed_species(mut self, species: Vec<SpeciesId>) -> Self {
        self.allowed_species = species;
        self.allowed_species.sort();
        self
    }

    pub fn allows(&self, species: &SpeciesId) -> bool {
        self.allowed_species.is_empty() || self.allowed_species.binary_search(species).is_ok()
    }
}

/// Region with carrying capacity for resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemRegion {
    pub id: EcosystemRegionId,
    pub name: String,
    /// Total resource units available.
    pub resource_capacity: f32,
    /// Current available resources.
    pub current_resources: f32,
    /// Resource regeneration rate per tick.
    pub regeneration_rate: f32,
}

impl EcosystemRegion {
    pub fn new(id: impl Into<String>, name: impl Into<String>, capacity: f32) -> Self {
        Self {
            id: EcosystemRegionId::new(id),
            name: name.into(),
            resource_capacity: capacity,
            current_resources: capacity,
            regeneration_rate: 0.05,
        }
    }

    #[must_use]
    pub fn with_regeneration_rate(mut self, rate: f32) -> Self {
        self.regeneration_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn resource_fraction(&self) -> f32 {
        if self.resource_capacity > 0.0 {
            self.current_resources / self.resource_capacity
        } else {
            0.0
        }
    }

    pub fn regenerate(&mut self) {
        let deficit = self.resource_capacity - self.current_resources;
        self.current_resources += deficit * self.regeneration_rate;
        self.current_resources = self.current_resources.min(self.resource_capacity);
    }

    pub fn consume(&mut self, amount: f32) -> f32 {
        let consumed = amount.min(self.current_resources);
        self.current_resources -= consumed;
        consumed
    }
}

/// Kind of ecosystem event.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EcosystemEventKind {
    /// Population fell below minimum viable threshold.
    LocalExtinction,
    /// Species completely gone from all regions.
    GlobalExtinction,
    /// Population starving due to resource scarcity.
    Starvation,
    /// Population exceeds comfortable density.
    Overpopulation,
    /// Population migrated to another region.
    Migration,
    /// Population recovered from stress.
    Recovery,
    /// Predation event significantly impacted prey.
    PredationPressure,
    /// Competition significantly impacted growth.
    CompetitionPressure,
    /// New population established in a region.
    Colonization,
}

/// An event that occurred during ecosystem simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemEvent {
    pub tick: u64,
    pub kind: EcosystemEventKind,
    pub species: SpeciesId,
    pub region: Option<EcosystemRegionId>,
    pub details: String,
}

impl EcosystemEvent {
    pub fn new(
        tick: u64,
        kind: EcosystemEventKind,
        species: SpeciesId,
        region: Option<EcosystemRegionId>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            tick,
            kind,
            species,
            region,
            details: details.into(),
        }
    }
}

/// Summary statistics for the ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemSummary {
    pub tick: u64,
    pub total_population: u64,
    pub species_count: usize,
    pub region_count: usize,
    pub extinct_species: Vec<SpeciesId>,
    pub stressed_populations: usize,
    pub average_resource_fraction: f32,
}

/// Projection of ecosystem state at a future tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemProjection {
    pub target_tick: u64,
    pub projected_populations: BTreeMap<PopulationKey, u32>,
    pub extinction_risk: BTreeMap<SpeciesId, f32>,
    pub resource_depletion_risk: BTreeMap<EcosystemRegionId, f32>,
}

/// CRC32-based fingerprint for ecosystem state verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcosystemFingerprint(pub u32);

/// Result of a single ecosystem tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemTickResult {
    pub tick: u64,
    pub events: Vec<EcosystemEvent>,
    pub births: u64,
    pub deaths: u64,
    pub migrations: u64,
}

/// Configuration for ecosystem simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Stress threshold that triggers migration behavior.
    pub migration_stress_threshold: f32,
    /// Minimum ticks between migration events for a population.
    pub migration_cooldown: u32,
    /// Starvation mortality multiplier when resources are scarce.
    pub starvation_mortality_multiplier: f32,
    /// Overcrowding mortality multiplier when over max density.
    pub overcrowding_mortality_multiplier: f32,
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            migration_stress_threshold: 0.6,
            migration_cooldown: 5,
            starvation_mortality_multiplier: 2.0,
            overcrowding_mortality_multiplier: 1.5,
        }
    }
}

impl EcosystemConfig {
    #[must_use]
    pub fn with_migration_stress_threshold(mut self, threshold: f32) -> Self {
        self.migration_stress_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_migration_cooldown(mut self, cooldown: u32) -> Self {
        self.migration_cooldown = cooldown;
        self
    }

    #[must_use]
    pub fn with_starvation_mortality_multiplier(mut self, mult: f32) -> Self {
        self.starvation_mortality_multiplier = mult.max(1.0);
        self
    }

    #[must_use]
    pub fn with_overcrowding_mortality_multiplier(mut self, mult: f32) -> Self {
        self.overcrowding_mortality_multiplier = mult.max(1.0);
        self
    }
}

/// Main ecosystem simulator managing multi-species population dynamics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemSimulator {
    current_tick: u64,
    config: EcosystemConfig,
    species: BTreeMap<SpeciesId, Species>,
    regions: BTreeMap<EcosystemRegionId, EcosystemRegion>,
    populations: BTreeMap<PopulationKey, Population>,
    predator_prey: Vec<PredatorPreyRelation>,
    competitors: Vec<CompetitorRelation>,
    corridors: Vec<MigrationCorridor>,
}

impl EcosystemSimulator {
    pub fn new(config: EcosystemConfig) -> Self {
        Self {
            current_tick: 0,
            config,
            species: BTreeMap::new(),
            regions: BTreeMap::new(),
            populations: BTreeMap::new(),
            predator_prey: Vec::new(),
            competitors: Vec::new(),
            corridors: Vec::new(),
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn add_species(&mut self, species: Species) {
        self.species.insert(species.id.clone(), species);
    }

    pub fn add_region(&mut self, region: EcosystemRegion) {
        self.regions.insert(region.id.clone(), region);
    }

    pub fn add_population(&mut self, population: Population) {
        self.populations.insert(population.key.clone(), population);
    }

    pub fn add_predator_prey(&mut self, relation: PredatorPreyRelation) {
        self.predator_prey.push(relation);
        self.predator_prey
            .sort_by(|a, b| (&a.predator, &a.prey).cmp(&(&b.predator, &b.prey)));
    }

    pub fn add_competitor(&mut self, relation: CompetitorRelation) {
        self.competitors.push(relation);
        self.competitors
            .sort_by(|a, b| (&a.species_a, &a.species_b).cmp(&(&b.species_a, &b.species_b)));
    }

    pub fn add_corridor(&mut self, corridor: MigrationCorridor) {
        self.corridors.push(corridor);
        self.corridors
            .sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    }

    pub fn get_species(&self, id: &SpeciesId) -> Option<&Species> {
        self.species.get(id)
    }

    pub fn get_region(&self, id: &EcosystemRegionId) -> Option<&EcosystemRegion> {
        self.regions.get(id)
    }

    pub fn get_population(&self, key: &PopulationKey) -> Option<&Population> {
        self.populations.get(key)
    }

    pub fn populations_in_region(&self, region: &EcosystemRegionId) -> Vec<&Population> {
        self.populations
            .values()
            .filter(|p| &p.key.region == region)
            .collect()
    }

    pub fn populations_of_species(&self, species: &SpeciesId) -> Vec<&Population> {
        self.populations
            .values()
            .filter(|p| &p.key.species == species)
            .collect()
    }

    pub fn total_population_of_species(&self, species: &SpeciesId) -> u64 {
        self.populations
            .values()
            .filter(|p| &p.key.species == species)
            .map(|p| u64::from(p.count))
            .sum()
    }

    pub fn is_species_extinct(&self, species: &SpeciesId) -> bool {
        self.total_population_of_species(species) == 0
    }

    /// Execute one simulation tick.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::too_many_lines
    )]
    pub fn tick(&mut self) -> EcosystemTickResult {
        self.current_tick += 1;
        let tick = self.current_tick;

        let mut events = Vec::new();
        let mut total_births: u64 = 0;
        let mut total_deaths: u64 = 0;

        // Collect keys for deterministic iteration
        let pop_keys: Vec<_> = self.populations.keys().cloned().collect();

        // Phase 1: Resource regeneration
        for region in self.regions.values_mut() {
            region.regenerate();
        }

        // Phase 2: Calculate population changes (births, deaths, stress)
        let mut deltas: BTreeMap<PopulationKey, (i64, f32)> = BTreeMap::new();

        for key in &pop_keys {
            let Some(pop) = self.populations.get(key) else {
                continue;
            };
            if pop.is_extinct() {
                continue;
            }

            let Some(species) = self.species.get(&key.species) else {
                continue;
            };
            let Some(region) = self.regions.get(&key.region) else {
                continue;
            };

            let count = pop.count as f32;

            // Resource consumption
            let needed = count * species.resource_consumption;
            let available = region.current_resources;
            let food_ratio = if needed > 0.0 {
                (available / needed).min(1.0)
            } else {
                1.0
            };

            // Carrying capacity effect (logistic growth)
            let density_ratio = count / species.max_density as f32;
            let capacity_factor = (1.0 - density_ratio).max(0.0);

            // Competition pressure
            let competition_pressure = self.calculate_competition_pressure(key);

            // Predation pressure (as prey)
            let predation_loss = self.calculate_predation_loss(key);

            // Birth rate modified by food and capacity
            let effective_birth_rate = species.base_birth_rate
                * food_ratio
                * capacity_factor
                * (1.0 - competition_pressure);

            // Death rate modified by starvation and overcrowding
            let mut effective_death_rate = species.base_death_rate;
            let mut new_stress = pop.stress;

            if food_ratio < 0.5 {
                effective_death_rate *= self.config.starvation_mortality_multiplier;
                new_stress = (new_stress + 0.2).min(1.0);
                if food_ratio < 0.3 {
                    events.push(EcosystemEvent::new(
                        tick,
                        EcosystemEventKind::Starvation,
                        key.species.clone(),
                        Some(key.region.clone()),
                        format!("Population starving, food ratio: {food_ratio:.2}"),
                    ));
                }
            } else {
                new_stress = (new_stress - 0.1).max(0.0);
            }

            if density_ratio > 1.0 {
                effective_death_rate *= self.config.overcrowding_mortality_multiplier;
                events.push(EcosystemEvent::new(
                    tick,
                    EcosystemEventKind::Overpopulation,
                    key.species.clone(),
                    Some(key.region.clone()),
                    format!("Overcrowding at {:.1}% density", density_ratio * 100.0),
                ));
            }

            // Calculate net change
            let births = (count * effective_birth_rate).round() as i64;
            let natural_deaths = (count * effective_death_rate).round() as i64;
            let predation_deaths = i64::from(predation_loss);

            total_births += births as u64;
            total_deaths += (natural_deaths + predation_deaths) as u64;

            let net_change = births - natural_deaths - predation_deaths;
            deltas.insert(key.clone(), (net_change, new_stress));
        }

        // Phase 3: Apply predator gains
        let predator_gains = self.calculate_predator_gains(&pop_keys);
        for (key, gain) in predator_gains {
            deltas
                .entry(key)
                .and_modify(|(delta, _)| *delta += i64::from(gain))
                .or_insert((i64::from(gain), 0.0));
            total_births += u64::from(gain);
        }

        // Phase 4: Apply changes
        for (key, (delta, stress)) in &deltas {
            if let Some(pop) = self.populations.get_mut(key) {
                let new_count = (i64::from(pop.count) + delta).max(0) as u32;

                // Check for extinction
                if new_count == 0 && pop.count > 0 {
                    events.push(EcosystemEvent::new(
                        tick,
                        EcosystemEventKind::LocalExtinction,
                        key.species.clone(),
                        Some(key.region.clone()),
                        "Population went extinct in region",
                    ));
                }

                // Check for recovery from stress
                if pop.stress > 0.5 && *stress <= 0.5 {
                    events.push(EcosystemEvent::new(
                        tick,
                        EcosystemEventKind::Recovery,
                        key.species.clone(),
                        Some(key.region.clone()),
                        "Population recovered from stress",
                    ));
                }

                pop.count = new_count;
                pop.stress = *stress;
                pop.ticks_since_migration += 1;
            }
        }

        // Phase 5: Resource consumption
        for key in &pop_keys {
            let Some(pop) = self.populations.get(key) else {
                continue;
            };
            let Some(species) = self.species.get(&key.species) else {
                continue;
            };
            let Some(region) = self.regions.get_mut(&key.region) else {
                continue;
            };

            let consumption = pop.count as f32 * species.resource_consumption;
            region.consume(consumption);
        }

        // Phase 6: Migration
        let migration_events = self.process_migrations();
        let total_migrations: u64 = migration_events.len() as u64;
        events.extend(migration_events);

        // Phase 7: Check for global extinctions
        for species_id in self.species.keys() {
            if self.is_species_extinct(species_id) {
                let has_prior_pop = pop_keys.iter().any(|k| &k.species == species_id);
                if has_prior_pop {
                    events.push(EcosystemEvent::new(
                        tick,
                        EcosystemEventKind::GlobalExtinction,
                        species_id.clone(),
                        None,
                        "Species extinct globally",
                    ));
                }
            }
        }

        // Sort events for determinism
        events.sort_by(|a, b| {
            (&a.kind, &a.species, &a.region).cmp(&(&b.kind, &b.species, &b.region))
        });

        EcosystemTickResult {
            tick,
            events,
            births: total_births,
            deaths: total_deaths,
            migrations: total_migrations,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn calculate_competition_pressure(&self, key: &PopulationKey) -> f32 {
        let mut pressure = 0.0f32;

        for rel in &self.competitors {
            if let Some(other_species) = rel.other(&key.species) {
                let other_key = PopulationKey::new(other_species.clone(), key.region.clone());
                if let Some(other_pop) = self.populations.get(&other_key)
                    && let Some(other_spec) = self.species.get(other_species)
                {
                    let other_density = other_pop.count as f32 / other_spec.max_density as f32;
                    pressure += rel.competition_coefficient * other_density;
                }
            }
        }

        pressure.min(0.9)
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn calculate_predation_loss(&self, key: &PopulationKey) -> u32 {
        let mut loss = 0u32;

        for rel in &self.predator_prey {
            if rel.prey == key.species {
                let predator_key = PopulationKey::new(rel.predator.clone(), key.region.clone());
                if let Some(predator_pop) = self.populations.get(&predator_key)
                    && let Some(prey_pop) = self.populations.get(key)
                {
                    let kills =
                        (predator_pop.count as f32 * rel.predation_rate * prey_pop.count as f32)
                            .round() as u32;
                    loss += kills.min(prey_pop.count);
                }
            }
        }

        loss
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn calculate_predator_gains(&self, pop_keys: &[PopulationKey]) -> BTreeMap<PopulationKey, u32> {
        let mut gains = BTreeMap::new();

        for rel in &self.predator_prey {
            for key in pop_keys {
                if key.species == rel.prey {
                    let predator_key = PopulationKey::new(rel.predator.clone(), key.region.clone());
                    if let Some(predator_pop) = self.populations.get(&predator_key)
                        && let Some(prey_pop) = self.populations.get(key)
                    {
                        let kills = (predator_pop.count as f32
                            * rel.predation_rate
                            * prey_pop.count as f32)
                            .round() as u32;
                        let actual_kills = kills.min(prey_pop.count);
                        let new_predators =
                            (actual_kills as f32 * rel.conversion_efficiency).round() as u32;
                        *gains.entry(predator_key).or_insert(0) += new_predators;
                    }
                }
            }
        }

        gains
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn process_migrations(&mut self) -> Vec<EcosystemEvent> {
        let mut events = Vec::new();
        let mut migrations: Vec<(PopulationKey, PopulationKey, u32)> = Vec::new();

        // Collect migration candidates
        for corridor in &self.corridors {
            for (key, pop) in &self.populations {
                if key.region != corridor.from {
                    continue;
                }
                if !corridor.allows(&key.species) {
                    continue;
                }
                if pop.stress < self.config.migration_stress_threshold {
                    continue;
                }
                if pop.ticks_since_migration < self.config.migration_cooldown {
                    continue;
                }
                if pop.count == 0 {
                    continue;
                }

                let migrants = (pop.count as f32 * corridor.migration_rate).round() as u32;
                if migrants > 0 {
                    let dest_key = PopulationKey::new(key.species.clone(), corridor.to.clone());
                    migrations.push((key.clone(), dest_key, migrants));
                }
            }
        }

        // Sort migrations for determinism
        migrations.sort();

        // Apply migrations
        for (from_key, to_key, count) in migrations {
            if let Some(from_pop) = self.populations.get_mut(&from_key) {
                let actual = count.min(from_pop.count);
                from_pop.count -= actual;
                from_pop.ticks_since_migration = 0;

                let dest_pop = self.populations.entry(to_key.clone()).or_insert_with(|| {
                    Population::new(to_key.species.clone(), to_key.region.clone(), 0)
                });

                let is_colonization = dest_pop.count == 0;
                dest_pop.count += actual;

                let event_kind = if is_colonization {
                    EcosystemEventKind::Colonization
                } else {
                    EcosystemEventKind::Migration
                };

                events.push(EcosystemEvent::new(
                    self.current_tick,
                    event_kind,
                    from_key.species.clone(),
                    Some(to_key.region.clone()),
                    format!(
                        "{} individuals migrated from {}",
                        actual,
                        from_key.region.as_str()
                    ),
                ));
            }
        }

        events
    }

    /// Generate a summary of current ecosystem state.
    #[allow(clippy::cast_precision_loss)]
    pub fn summary(&self) -> EcosystemSummary {
        let total_population: u64 = self.populations.values().map(|p| u64::from(p.count)).sum();
        let extinct_species: Vec<_> = self
            .species
            .keys()
            .filter(|id| self.is_species_extinct(id))
            .cloned()
            .collect();
        let stressed_populations = self
            .populations
            .values()
            .filter(|p| p.is_stressed())
            .count();

        let total_resource_fraction: f32 = self
            .regions
            .values()
            .map(EcosystemRegion::resource_fraction)
            .sum();
        let average_resource_fraction = if self.regions.is_empty() {
            0.0
        } else {
            total_resource_fraction / self.regions.len() as f32
        };

        EcosystemSummary {
            tick: self.current_tick,
            total_population,
            species_count: self.species.len(),
            region_count: self.regions.len(),
            extinct_species,
            stressed_populations,
            average_resource_fraction,
        }
    }

    /// Project ecosystem state forward by a number of ticks.
    #[allow(clippy::cast_precision_loss)]
    pub fn project(&self, ticks_ahead: u32) -> EcosystemProjection {
        let mut sim = self.clone();
        for _ in 0..ticks_ahead {
            sim.tick();
        }

        let projected_populations: BTreeMap<_, _> = sim
            .populations
            .iter()
            .map(|(k, p)| (k.clone(), p.count))
            .collect();

        let mut extinction_risk = BTreeMap::new();
        for species_id in sim.species.keys() {
            let current_total = self.total_population_of_species(species_id);
            let projected_total = sim.total_population_of_species(species_id);

            let risk = if current_total == 0 || projected_total == 0 {
                1.0
            } else {
                let decline = 1.0 - (projected_total as f32 / current_total as f32);
                decline.clamp(0.0, 1.0)
            };
            extinction_risk.insert(species_id.clone(), risk);
        }

        let mut resource_depletion_risk = BTreeMap::new();
        for region_id in sim.regions.keys() {
            let current = self
                .regions
                .get(region_id)
                .map_or(0.0, EcosystemRegion::resource_fraction);
            let projected = sim
                .regions
                .get(region_id)
                .map_or(0.0, EcosystemRegion::resource_fraction);

            let risk = if current <= 0.0 {
                1.0
            } else {
                let decline: f32 = 1.0 - (projected / current);
                decline.clamp(0.0, 1.0)
            };
            resource_depletion_risk.insert(region_id.clone(), risk);
        }

        EcosystemProjection {
            target_tick: self.current_tick + u64::from(ticks_ahead),
            projected_populations,
            extinction_risk,
            resource_depletion_risk,
        }
    }

    /// Generate a deterministic fingerprint of current state.
    pub fn fingerprint(&self) -> EcosystemFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        hasher.update(&self.current_tick.to_le_bytes());

        for (id, pop) in &self.populations {
            hasher.update(id.species.as_str().as_bytes());
            hasher.update(id.region.as_str().as_bytes());
            hasher.update(&pop.count.to_le_bytes());
            hasher.update(&pop.stress.to_le_bytes());
        }

        for (id, region) in &self.regions {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&region.current_resources.to_le_bytes());
        }

        EcosystemFingerprint(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn species_id(s: &str) -> SpeciesId {
        SpeciesId::new(s)
    }

    fn region_id(s: &str) -> EcosystemRegionId {
        EcosystemRegionId::new(s)
    }

    fn make_prey() -> Species {
        Species::new("rabbit", "Rabbit", TrophicRole::PrimaryConsumer)
            .with_birth_rate(0.2)
            .with_death_rate(0.05)
            .with_max_density(500)
    }

    fn make_predator() -> Species {
        Species::new("fox", "Fox", TrophicRole::SecondaryConsumer)
            .with_birth_rate(0.05)
            .with_death_rate(0.03)
            .with_max_density(100)
    }

    fn make_competitor() -> Species {
        Species::new("deer", "Deer", TrophicRole::PrimaryConsumer)
            .with_birth_rate(0.15)
            .with_death_rate(0.04)
            .with_max_density(300)
    }

    fn make_region() -> EcosystemRegion {
        EcosystemRegion::new("forest", "Forest", 1000.0).with_regeneration_rate(0.1)
    }

    fn make_second_region() -> EcosystemRegion {
        EcosystemRegion::new("meadow", "Meadow", 800.0).with_regeneration_rate(0.15)
    }

    #[test]
    fn test_species_id() {
        let id = SpeciesId::new("wolf");
        assert_eq!(id.as_str(), "wolf");
    }

    #[test]
    fn test_region_id() {
        let id = EcosystemRegionId::new("forest");
        assert_eq!(id.as_str(), "forest");
    }

    #[test]
    fn test_population_key() {
        let key = PopulationKey::new(species_id("rabbit"), region_id("forest"));
        assert_eq!(key.species.as_str(), "rabbit");
        assert_eq!(key.region.as_str(), "forest");
    }

    #[test]
    fn test_species_builder() {
        let species = Species::new("test", "Test Species", TrophicRole::Producer)
            .with_birth_rate(0.3)
            .with_death_rate(0.1)
            .with_resource_consumption(2.0)
            .with_minimum_viable_population(20)
            .with_max_density(500);

        assert!(approx_eq(species.base_birth_rate, 0.3));
        assert!(approx_eq(species.base_death_rate, 0.1));
        assert!(approx_eq(species.resource_consumption, 2.0));
        assert_eq!(species.minimum_viable_population, 20);
        assert_eq!(species.max_density, 500);
    }

    #[test]
    fn test_population_state() {
        let mut pop = Population::new(species_id("rabbit"), region_id("forest"), 100);
        assert!(!pop.is_extinct());
        assert!(!pop.is_stressed());

        pop.stress = 0.6;
        assert!(pop.is_stressed());

        pop.count = 0;
        assert!(pop.is_extinct());
    }

    #[test]
    fn test_predator_prey_relation() {
        let rel = PredatorPreyRelation::new(species_id("fox"), species_id("rabbit"))
            .with_predation_rate(0.05)
            .with_conversion_efficiency(0.2);

        assert!(approx_eq(rel.predation_rate, 0.05));
        assert!(approx_eq(rel.conversion_efficiency, 0.2));
    }

    #[test]
    fn test_competitor_relation() {
        let rel =
            CompetitorRelation::new(species_id("rabbit"), species_id("deer")).with_coefficient(0.3);

        assert!(rel.involves(&species_id("rabbit")));
        assert!(rel.involves(&species_id("deer")));
        assert!(!rel.involves(&species_id("fox")));

        assert_eq!(rel.other(&species_id("rabbit")), Some(&species_id("deer")));
        assert_eq!(rel.other(&species_id("deer")), Some(&species_id("rabbit")));
        assert_eq!(rel.other(&species_id("fox")), None);
    }

    #[test]
    fn test_migration_corridor() {
        let corridor = MigrationCorridor::new(region_id("forest"), region_id("meadow"))
            .with_migration_rate(0.2)
            .with_allowed_species(vec![species_id("deer"), species_id("rabbit")]);

        assert!(corridor.allows(&species_id("rabbit")));
        assert!(corridor.allows(&species_id("deer")));
        assert!(!corridor.allows(&species_id("fox")));

        let open_corridor = MigrationCorridor::new(region_id("a"), region_id("b"));
        assert!(open_corridor.allows(&species_id("anything")));
    }

    #[test]
    fn test_region_resources() {
        let mut region = EcosystemRegion::new("test", "Test", 100.0).with_regeneration_rate(0.5);

        assert!(approx_eq(region.resource_fraction(), 1.0));

        let consumed = region.consume(30.0);
        assert!(approx_eq(consumed, 30.0));
        assert!(approx_eq(region.current_resources, 70.0));

        region.regenerate();
        assert!(approx_eq(region.current_resources, 85.0));

        let consumed = region.consume(200.0);
        assert!(approx_eq(consumed, 85.0));
        assert!(approx_eq(region.current_resources, 0.0));
    }

    #[test]
    fn test_simulator_basic() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        sim.add_species(make_prey());
        sim.add_region(make_region());
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));

        assert_eq!(sim.current_tick(), 0);
        assert!(sim.get_species(&species_id("rabbit")).is_some());
        assert!(sim.get_region(&region_id("forest")).is_some());
        assert_eq!(sim.total_population_of_species(&species_id("rabbit")), 100);
    }

    #[test]
    fn test_predator_prey_dynamics() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        sim.add_species(make_prey());
        sim.add_species(make_predator());
        sim.add_region(make_region());

        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            200,
        ));
        sim.add_population(Population::new(species_id("fox"), region_id("forest"), 20));

        sim.add_predator_prey(
            PredatorPreyRelation::new(species_id("fox"), species_id("rabbit"))
                .with_predation_rate(0.02)
                .with_conversion_efficiency(0.1),
        );

        let initial_rabbits = sim.total_population_of_species(&species_id("rabbit"));
        let initial_foxes = sim.total_population_of_species(&species_id("fox"));

        for _ in 0..10 {
            sim.tick();
        }

        let final_rabbits = sim.total_population_of_species(&species_id("rabbit"));
        let final_foxes = sim.total_population_of_species(&species_id("fox"));

        assert!(final_rabbits != initial_rabbits || final_foxes != initial_foxes);
    }

    #[test]
    fn test_carrying_capacity() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        let prey = make_prey().with_max_density(100);
        sim.add_species(prey);
        sim.add_region(make_region());

        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            150,
        ));

        let result = sim.tick();

        let overpop_event = result
            .events
            .iter()
            .any(|e| e.kind == EcosystemEventKind::Overpopulation);
        assert!(overpop_event);
    }

    #[test]
    fn test_resource_scarcity_starvation() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        let prey = make_prey().with_resource_consumption(10.0);
        sim.add_species(prey);
        sim.add_region(EcosystemRegion::new("barren", "Barren", 50.0).with_regeneration_rate(0.01));

        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("barren"),
            100,
        ));

        let mut had_starvation = false;
        for _ in 0..20 {
            let result = sim.tick();
            if result
                .events
                .iter()
                .any(|e| e.kind == EcosystemEventKind::Starvation)
            {
                had_starvation = true;
                break;
            }
        }

        assert!(had_starvation);
    }

    #[test]
    fn test_competition_pressure() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        sim.add_species(make_prey());
        sim.add_species(make_competitor());
        sim.add_region(make_region());

        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));
        sim.add_population(Population::new(
            species_id("deer"),
            region_id("forest"),
            100,
        ));

        sim.add_competitor(
            CompetitorRelation::new(species_id("rabbit"), species_id("deer")).with_coefficient(0.5),
        );

        let _initial_rabbits = sim.total_population_of_species(&species_id("rabbit"));

        for _ in 0..10 {
            sim.tick();
        }

        let with_competition = sim.total_population_of_species(&species_id("rabbit"));

        let mut sim2 = EcosystemSimulator::new(EcosystemConfig::default());
        sim2.add_species(make_prey());
        sim2.add_region(make_region());
        sim2.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));

        for _ in 0..10 {
            sim2.tick();
        }

        let without_competition = sim2.total_population_of_species(&species_id("rabbit"));

        assert!(
            with_competition <= without_competition,
            "Competition should reduce growth: {with_competition} vs {without_competition}"
        );
    }

    #[test]
    fn test_migration() {
        let config = EcosystemConfig::default()
            .with_migration_stress_threshold(0.3)
            .with_migration_cooldown(0);

        let mut sim = EcosystemSimulator::new(config);

        sim.add_species(make_prey());
        sim.add_region(EcosystemRegion::new("crowded", "Crowded", 10.0));
        sim.add_region(make_second_region());

        let mut pop = Population::new(species_id("rabbit"), region_id("crowded"), 100);
        pop.stress = 0.8;
        sim.add_population(pop);

        sim.add_corridor(
            MigrationCorridor::new(region_id("crowded"), region_id("meadow"))
                .with_migration_rate(0.5),
        );

        let result = sim.tick();

        let migration_events: Vec<_> = result
            .events
            .iter()
            .filter(|e| {
                e.kind == EcosystemEventKind::Migration
                    || e.kind == EcosystemEventKind::Colonization
            })
            .collect();

        assert!(!migration_events.is_empty(), "Expected migration events");

        let meadow_pop = sim.total_population_of_species(&species_id("rabbit"))
            - sim
                .get_population(&PopulationKey::new(
                    species_id("rabbit"),
                    region_id("crowded"),
                ))
                .map_or(0, |p| u64::from(p.count));

        assert!(meadow_pop > 0, "Expected population in meadow");
    }

    #[test]
    fn test_local_extinction() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        let prey = make_prey().with_birth_rate(0.0).with_death_rate(0.9);
        sim.add_species(prey);
        sim.add_region(make_region());

        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            10,
        ));

        let mut had_extinction = false;
        for _ in 0..10 {
            let result = sim.tick();
            if result
                .events
                .iter()
                .any(|e| e.kind == EcosystemEventKind::LocalExtinction)
            {
                had_extinction = true;
                break;
            }
        }

        assert!(had_extinction);
    }

    #[test]
    fn test_deterministic_tick_results() {
        let make_sim = || {
            let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
            sim.add_species(make_prey());
            sim.add_species(make_predator());
            sim.add_region(make_region());
            sim.add_population(Population::new(
                species_id("rabbit"),
                region_id("forest"),
                100,
            ));
            sim.add_population(Population::new(species_id("fox"), region_id("forest"), 20));
            sim.add_predator_prey(PredatorPreyRelation::new(
                species_id("fox"),
                species_id("rabbit"),
            ));
            sim
        };

        let mut sim1 = make_sim();
        let mut sim2 = make_sim();

        for _ in 0..20 {
            let r1 = sim1.tick();
            let r2 = sim2.tick();

            assert_eq!(r1.tick, r2.tick);
            assert_eq!(r1.births, r2.births);
            assert_eq!(r1.deaths, r2.deaths);
            assert_eq!(r1.events.len(), r2.events.len());
        }

        assert_eq!(sim1.fingerprint(), sim2.fingerprint());
    }

    #[test]
    fn test_fingerprint_changes_with_state() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
        sim.add_species(make_prey());
        sim.add_region(make_region());
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));

        let fp1 = sim.fingerprint();
        sim.tick();
        let fp2 = sim.fingerprint();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_summary() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
        sim.add_species(make_prey());
        sim.add_species(make_predator());
        sim.add_region(make_region());
        sim.add_region(make_second_region());
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));
        sim.add_population(Population::new(species_id("fox"), region_id("forest"), 20));

        let summary = sim.summary();

        assert_eq!(summary.tick, 0);
        assert_eq!(summary.total_population, 120);
        assert_eq!(summary.species_count, 2);
        assert_eq!(summary.region_count, 2);
        assert!(summary.extinct_species.is_empty());
    }

    #[test]
    fn test_projection() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
        sim.add_species(make_prey());
        sim.add_region(make_region());
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));

        let projection = sim.project(10);

        assert_eq!(projection.target_tick, 10);
        assert!(!projection.projected_populations.is_empty());
        assert!(!projection.extinction_risk.is_empty());
        assert!(!projection.resource_depletion_risk.is_empty());

        assert_eq!(sim.current_tick(), 0);
    }

    #[test]
    fn test_serde_species() {
        let species = make_prey();
        let json = serde_json::to_string(&species).unwrap();
        let restored: Species = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), species.id.as_str());
        assert!(approx_eq(restored.base_birth_rate, species.base_birth_rate));
    }

    #[test]
    fn test_serde_population() {
        let pop = Population::new(species_id("rabbit"), region_id("forest"), 100);
        let json = serde_json::to_string(&pop).unwrap();
        let restored: Population = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.key.species.as_str(), "rabbit");
        assert_eq!(restored.count, 100);
    }

    #[test]
    fn test_serde_ecosystem_simulator() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
        sim.add_species(make_prey());
        sim.add_species(make_predator());
        sim.add_region(make_region());
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));
        sim.add_population(Population::new(species_id("fox"), region_id("forest"), 20));
        sim.add_predator_prey(PredatorPreyRelation::new(
            species_id("fox"),
            species_id("rabbit"),
        ));

        for _ in 0..5 {
            sim.tick();
        }

        let bytes = bincode::serialize(&sim).unwrap();
        let restored: EcosystemSimulator = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.current_tick(), sim.current_tick());
        assert_eq!(restored.fingerprint(), sim.fingerprint());
    }

    #[test]
    fn test_serde_tick_result() {
        let result = EcosystemTickResult {
            tick: 5,
            events: vec![EcosystemEvent::new(
                5,
                EcosystemEventKind::Starvation,
                species_id("rabbit"),
                Some(region_id("forest")),
                "test event",
            )],
            births: 10,
            deaths: 5,
            migrations: 2,
        };

        let json = serde_json::to_string(&result).unwrap();
        let restored: EcosystemTickResult = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 5);
        assert_eq!(restored.events.len(), 1);
        assert_eq!(restored.births, 10);
    }

    #[test]
    fn test_serde_summary() {
        let summary = EcosystemSummary {
            tick: 10,
            total_population: 500,
            species_count: 3,
            region_count: 2,
            extinct_species: vec![species_id("dodo")],
            stressed_populations: 1,
            average_resource_fraction: 0.75,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: EcosystemSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 10);
        assert_eq!(restored.total_population, 500);
        assert_eq!(restored.extinct_species.len(), 1);
    }

    #[test]
    fn test_serde_projection() {
        let mut pops = BTreeMap::new();
        pops.insert(
            PopulationKey::new(species_id("rabbit"), region_id("forest")),
            150,
        );

        let mut extinction_risk = BTreeMap::new();
        extinction_risk.insert(species_id("rabbit"), 0.1);

        let mut depletion_risk = BTreeMap::new();
        depletion_risk.insert(region_id("forest"), 0.2);

        let projection = EcosystemProjection {
            target_tick: 20,
            projected_populations: pops,
            extinction_risk,
            resource_depletion_risk: depletion_risk,
        };

        let bytes = bincode::serialize(&projection).unwrap();
        let restored: EcosystemProjection = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.target_tick, 20);
        assert!(!restored.projected_populations.is_empty());
    }

    #[test]
    fn test_serde_fingerprint() {
        let fp = EcosystemFingerprint(0x1234_5678);
        let json = serde_json::to_string(&fp).unwrap();
        let restored: EcosystemFingerprint = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, fp);
    }

    #[test]
    fn test_ecosystem_config_builder() {
        let config = EcosystemConfig::default()
            .with_migration_stress_threshold(0.7)
            .with_migration_cooldown(10)
            .with_starvation_mortality_multiplier(3.0)
            .with_overcrowding_mortality_multiplier(2.0);

        assert!(approx_eq(config.migration_stress_threshold, 0.7));
        assert_eq!(config.migration_cooldown, 10);
        assert!(approx_eq(config.starvation_mortality_multiplier, 3.0));
        assert!(approx_eq(config.overcrowding_mortality_multiplier, 2.0));
    }

    #[test]
    fn test_queries() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
        sim.add_species(make_prey());
        sim.add_species(make_predator());
        sim.add_region(make_region());
        sim.add_region(make_second_region());
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("meadow"),
            50,
        ));
        sim.add_population(Population::new(species_id("fox"), region_id("forest"), 20));

        let forest_pops = sim.populations_in_region(&region_id("forest"));
        assert_eq!(forest_pops.len(), 2);

        let rabbit_pops = sim.populations_of_species(&species_id("rabbit"));
        assert_eq!(rabbit_pops.len(), 2);

        assert_eq!(sim.total_population_of_species(&species_id("rabbit")), 150);
        assert!(!sim.is_species_extinct(&species_id("rabbit")));
        assert!(sim.is_species_extinct(&species_id("wolf")));
    }

    #[test]
    fn test_recovery_event() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());

        let prey = make_prey()
            .with_birth_rate(0.3)
            .with_death_rate(0.01)
            .with_resource_consumption(0.01);
        sim.add_species(prey);
        sim.add_region(EcosystemRegion::new("lush", "Lush", 10000.0).with_regeneration_rate(0.5));

        let mut pop = Population::new(species_id("rabbit"), region_id("lush"), 100);
        pop.stress = 0.7;
        sim.add_population(pop);

        let mut had_recovery = false;
        for _ in 0..20 {
            let result = sim.tick();
            if result
                .events
                .iter()
                .any(|e| e.kind == EcosystemEventKind::Recovery)
            {
                had_recovery = true;
                break;
            }
        }

        assert!(had_recovery);
    }

    #[test]
    fn test_multiple_regions_independent() {
        let mut sim = EcosystemSimulator::new(EcosystemConfig::default());
        sim.add_species(make_prey());
        sim.add_region(make_region());
        sim.add_region(make_second_region());

        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("forest"),
            100,
        ));
        sim.add_population(Population::new(
            species_id("rabbit"),
            region_id("meadow"),
            100,
        ));

        for _ in 0..10 {
            sim.tick();
        }

        let forest_pop = sim
            .get_population(&PopulationKey::new(
                species_id("rabbit"),
                region_id("forest"),
            ))
            .unwrap()
            .count;
        let meadow_pop = sim
            .get_population(&PopulationKey::new(
                species_id("rabbit"),
                region_id("meadow"),
            ))
            .unwrap()
            .count;

        assert!(forest_pop > 0);
        assert!(meadow_pop > 0);
    }
}

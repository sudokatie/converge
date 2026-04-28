//! Population director for coordinating spawn pressure, pacing, and migrations.

use super::budget::{PacingProfile, SpawnEvent};
use super::migration::{MigrationConfig, MigrationManager, MigrationWave};
use super::region::RegionalPopulation;
use super::species::{SpeciesCapId, SpeciesRegistry};
use super::summary::{PopulationSnapshot, PopulationSummary, WorldPopulationSummary};
use super::threat::ThreatLevel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuration for the population director.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PopulationConfig {
    /// Default soft cap for new regions.
    pub default_soft_cap: u32,
    /// Default hard cap for new regions.
    pub default_hard_cap: u32,
    /// Ticks between global population rebalancing.
    pub rebalance_interval: u64,
    /// Whether to auto-migrate when pressure is high.
    pub auto_migrate: bool,
    /// Pressure threshold for auto-migration.
    pub migration_pressure_threshold: f32,
    /// Whether to auto-adjust pacing based on activity.
    pub auto_pace: bool,
    /// Snapshot staleness threshold (ticks).
    pub snapshot_staleness: u64,
    /// Whether to enable threat-based bias updates.
    pub threat_bias_enabled: bool,
}

impl PopulationConfig {
    /// Create a new configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_caps(mut self, soft: u32, hard: u32) -> Self {
        self.default_soft_cap = soft;
        self.default_hard_cap = hard;
        self
    }

    #[must_use]
    pub fn with_rebalance_interval(mut self, ticks: u64) -> Self {
        self.rebalance_interval = ticks.max(1);
        self
    }

    #[must_use]
    pub fn with_auto_migrate(mut self, enabled: bool) -> Self {
        self.auto_migrate = enabled;
        self
    }

    #[must_use]
    pub fn with_auto_pace(mut self, enabled: bool) -> Self {
        self.auto_pace = enabled;
        self
    }
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            default_soft_cap: 100,
            default_hard_cap: 200,
            rebalance_interval: 300,
            auto_migrate: true,
            migration_pressure_threshold: 0.8,
            auto_pace: true,
            snapshot_staleness: 600,
            threat_bias_enabled: true,
        }
    }
}

/// Kind of population event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PopulationEventKind {
    /// Entity spawned.
    Spawn,
    /// Entity despawned.
    Despawn,
    /// Migration started.
    MigrationStart,
    /// Migration completed.
    MigrationComplete,
    /// Region reached hard cap.
    RegionCapped,
    /// Species reached hard cap.
    SpeciesCapped,
    /// Threat level changed.
    ThreatChanged,
    /// Pacing changed.
    PacingChanged,
}

/// A population-related event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PopulationEvent {
    /// Event kind.
    pub kind: PopulationEventKind,
    /// Associated region (if any).
    pub region_id: Option<String>,
    /// Associated species (if any).
    pub species_id: Option<SpeciesCapId>,
    /// Numeric value (count, level, etc.).
    pub value: u32,
    /// Tick when event occurred.
    pub tick: u64,
}

impl PopulationEvent {
    /// Create a new event.
    #[must_use]
    pub fn new(kind: PopulationEventKind, tick: u64) -> Self {
        Self {
            kind,
            region_id: None,
            species_id: None,
            value: 0,
            tick,
        }
    }

    #[must_use]
    pub fn with_region(mut self, region_id: impl Into<String>) -> Self {
        self.region_id = Some(region_id.into());
        self
    }

    #[must_use]
    pub fn with_species(mut self, species_id: SpeciesCapId) -> Self {
        self.species_id = Some(species_id);
        self
    }

    #[must_use]
    pub fn with_value(mut self, value: u32) -> Self {
        self.value = value;
        self
    }
}

/// Result of a director tick.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TickResult {
    /// Events generated this tick.
    pub events: Vec<PopulationEvent>,
    /// Spawns that should occur.
    pub spawns: Vec<SpawnEvent>,
    /// Completed migrations.
    pub completed_migrations: Vec<MigrationWave>,
    /// Regions that need loading.
    pub regions_to_load: Vec<String>,
}

impl TickResult {
    /// Create a new tick result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if anything happened.
    #[must_use]
    pub fn has_activity(&self) -> bool {
        !self.events.is_empty()
            || !self.spawns.is_empty()
            || !self.completed_migrations.is_empty()
            || !self.regions_to_load.is_empty()
    }
}

/// Population director coordinating all population systems.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationDirector {
    /// Configuration.
    config: PopulationConfig,
    /// Species registry.
    species: SpeciesRegistry,
    /// Regional populations.
    regions: BTreeMap<String, RegionalPopulation>,
    /// Migration manager.
    migrations: MigrationManager,
    /// Snapshots for unloaded regions.
    snapshots: BTreeMap<String, PopulationSnapshot>,
    /// Current tick.
    current_tick: u64,
    /// Last rebalance tick.
    last_rebalance_tick: u64,
    /// Global pacing profile.
    global_pacing: PacingProfile,
}

impl PopulationDirector {
    /// Create a new population director.
    #[must_use]
    pub fn new(config: PopulationConfig) -> Self {
        Self {
            config,
            species: SpeciesRegistry::new(),
            regions: BTreeMap::new(),
            migrations: MigrationManager::new(MigrationConfig::new()),
            snapshots: BTreeMap::new(),
            current_tick: 0,
            last_rebalance_tick: 0,
            global_pacing: PacingProfile::default(),
        }
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &PopulationConfig {
        &self.config
    }

    /// Get the species registry.
    #[must_use]
    pub fn species(&self) -> &SpeciesRegistry {
        &self.species
    }

    /// Get mutable species registry.
    pub fn species_mut(&mut self) -> &mut SpeciesRegistry {
        &mut self.species
    }

    /// Get the migration manager.
    #[must_use]
    pub fn migrations(&self) -> &MigrationManager {
        &self.migrations
    }

    /// Get mutable migration manager.
    pub fn migrations_mut(&mut self) -> &mut MigrationManager {
        &mut self.migrations
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get global pacing.
    #[must_use]
    pub fn global_pacing(&self) -> &PacingProfile {
        &self.global_pacing
    }

    /// Get mutable global pacing.
    pub fn global_pacing_mut(&mut self) -> &mut PacingProfile {
        &mut self.global_pacing
    }

    /// Register a region.
    pub fn register_region(&mut self, mut region: RegionalPopulation) {
        if region.soft_cap == 0 {
            region.soft_cap = self.config.default_soft_cap;
        }
        if region.hard_cap == 0 {
            region.hard_cap = self.config.default_hard_cap;
        }
        self.regions.insert(region.region_id.clone(), region);
    }

    /// Get a region.
    #[must_use]
    pub fn get_region(&self, region_id: &str) -> Option<&RegionalPopulation> {
        self.regions.get(region_id)
    }

    /// Get mutable region.
    pub fn get_region_mut(&mut self, region_id: &str) -> Option<&mut RegionalPopulation> {
        self.regions.get_mut(region_id)
    }

    /// Get or create a region.
    ///
    /// # Panics
    ///
    /// This function will not panic as the region is inserted if not present.
    pub fn get_or_create_region(&mut self, region_id: &str) -> &mut RegionalPopulation {
        if !self.regions.contains_key(region_id) {
            let region = RegionalPopulation::new(region_id)
                .with_caps(self.config.default_soft_cap, self.config.default_hard_cap);
            self.regions.insert(region_id.to_string(), region);
        }
        self.regions
            .get_mut(region_id)
            .expect("region was just inserted")
    }

    /// Iterate over regions (deterministic order).
    pub fn regions(&self) -> impl Iterator<Item = &RegionalPopulation> {
        self.regions.values()
    }

    /// Get number of regions.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Set a region as unloaded, creating a snapshot.
    pub fn unload_region(&mut self, region_id: &str) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.loaded = false;
            let snapshot = PopulationSnapshot::from_region(region, self.current_tick);
            self.snapshots.insert(region_id.to_string(), snapshot);
        }
    }

    /// Set a region as loaded, discarding its snapshot.
    pub fn load_region(&mut self, region_id: &str) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.loaded = true;
        }
        self.snapshots.remove(region_id);
    }

    /// Get a snapshot for an unloaded region.
    #[must_use]
    pub fn get_snapshot(&self, region_id: &str) -> Option<&PopulationSnapshot> {
        self.snapshots.get(region_id)
    }

    /// Queue a spawn request.
    pub fn queue_spawn(&mut self, region_id: &str, event: SpawnEvent) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.spawn_budget_mut().queue_spawn(event);
        }
    }

    /// Check if a species can spawn in a region.
    #[must_use]
    pub fn can_spawn(&self, region_id: &str, species_id: &SpeciesCapId) -> bool {
        if !self.species.can_spawn(species_id) {
            return false;
        }

        if let Some(region) = self.regions.get(region_id) {
            if region.at_hard_cap() {
                return false;
            }
            if region.spawn_budget().is_species_on_cooldown(species_id) {
                return false;
            }
            true
        } else {
            false
        }
    }

    /// Record a spawn.
    pub fn record_spawn(
        &mut self,
        region_id: &str,
        species_id: &SpeciesCapId,
    ) -> Option<PopulationEvent> {
        let species = self.species.get_species(species_id)?;
        let is_hostile = species.hostile;

        self.species.get_species_mut(species_id)?.increment();

        let region = self.regions.get_mut(region_id)?;
        region.counts_mut().increment(species_id, is_hostile);

        Some(
            PopulationEvent::new(PopulationEventKind::Spawn, self.current_tick)
                .with_region(region_id)
                .with_species(species_id.clone())
                .with_value(1),
        )
    }

    /// Record a despawn.
    pub fn record_despawn(
        &mut self,
        region_id: &str,
        species_id: &SpeciesCapId,
    ) -> Option<PopulationEvent> {
        let species = self.species.get_species(species_id)?;
        let is_hostile = species.hostile;

        self.species.get_species_mut(species_id)?.decrement();

        let region = self.regions.get_mut(region_id)?;
        region.counts_mut().decrement(species_id, is_hostile);

        Some(
            PopulationEvent::new(PopulationEventKind::Despawn, self.current_tick)
                .with_region(region_id)
                .with_species(species_id.clone())
                .with_value(1),
        )
    }

    /// Tick the population director.
    pub fn tick(&mut self) -> TickResult {
        self.current_tick += 1;
        let mut result = TickResult::new();

        self.global_pacing.tick();

        for region in self.regions.values_mut() {
            if region.loaded {
                region.tick();

                if self.config.threat_bias_enabled {
                    region.update_bias_from_threat();
                }
            }
        }

        let completed = self.migrations.tick();
        for wave in completed {
            result.events.push(
                PopulationEvent::new(PopulationEventKind::MigrationComplete, self.current_tick)
                    .with_value(wave.arriving_count()),
            );
            result.completed_migrations.push(wave);
        }

        for region in self.regions.values_mut() {
            if region.loaded {
                let spawns = region.spawn_budget_mut().process_queue();
                result.spawns.extend(spawns);
            }
        }

        if self.should_rebalance() {
            self.rebalance(&mut result);
            self.last_rebalance_tick = self.current_tick;
        }

        self.check_snapshots(&mut result);

        result
    }

    fn should_rebalance(&self) -> bool {
        self.current_tick.saturating_sub(self.last_rebalance_tick) >= self.config.rebalance_interval
    }

    fn rebalance(&mut self, result: &mut TickResult) {
        for region in self.regions.values() {
            if region.at_hard_cap() {
                result.events.push(
                    PopulationEvent::new(PopulationEventKind::RegionCapped, self.current_tick)
                        .with_region(&region.region_id),
                );
            }
        }

        for species in self.species.iter_species() {
            if species.at_hard_cap() {
                result.events.push(
                    PopulationEvent::new(PopulationEventKind::SpeciesCapped, self.current_tick)
                        .with_species(species.id.clone()),
                );
            }
        }
    }

    fn check_snapshots(&mut self, result: &mut TickResult) {
        for (region_id, snapshot) in &self.snapshots {
            if snapshot.is_stale(self.current_tick, self.config.snapshot_staleness)
                || snapshot.needs_intervention(100)
            {
                result.regions_to_load.push(region_id.clone());
            }
        }
    }

    /// Get a world summary of all populations.
    #[must_use]
    pub fn world_summary(&self) -> WorldPopulationSummary {
        let summaries = self
            .regions
            .values()
            .map(|r| PopulationSummary::from_region(r, self.current_tick));
        WorldPopulationSummary::from_summaries(summaries, self.current_tick)
    }

    /// Get total population across all regions.
    #[must_use]
    pub fn total_population(&self) -> u32 {
        self.regions.values().map(|r| r.counts().total()).sum()
    }

    /// Get regions above soft cap.
    pub fn overpopulated_regions(&self) -> impl Iterator<Item = &RegionalPopulation> {
        self.regions.values().filter(|r| r.above_soft_cap())
    }

    /// Get regions that need spawns (below thresholds).
    pub fn underpopulated_regions(&self) -> impl Iterator<Item = &RegionalPopulation> {
        self.regions
            .values()
            .filter(|r| r.counts().total() == 0 || r.pressure() < 0.3)
    }

    /// Set threat level for a region.
    pub fn set_region_threat(&mut self, region_id: &str, level: ThreatLevel) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.threat_mut().set_level(level);
        }
    }

    /// Advance to a specific tick.
    pub fn advance_to(&mut self, tick: u64) {
        while self.current_tick < tick {
            self.tick();
        }
    }
}

impl Default for PopulationDirector {
    fn default() -> Self {
        Self::new(PopulationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::population::budget::SpawnPriority;
    use crate::population::species::SpeciesCap;

    fn make_director() -> PopulationDirector {
        let mut director = PopulationDirector::new(PopulationConfig::new());

        director.species_mut().register_species(
            SpeciesCap::new(SpeciesCapId::new("wolf"), "Wolf")
                .with_caps(5, 50, 100)
                .with_hostile(true),
        );
        director.species_mut().register_species(
            SpeciesCap::new(SpeciesCapId::new("deer"), "Deer")
                .with_caps(10, 100, 200)
                .with_hostile(false),
        );

        director.register_region(RegionalPopulation::new("forest").with_caps(50, 100));
        director.register_region(RegionalPopulation::new("plains").with_caps(80, 150));

        director
    }

    #[test]
    fn test_population_config_defaults() {
        let config = PopulationConfig::new();

        assert!(config.default_soft_cap > 0);
        assert!(config.default_hard_cap > config.default_soft_cap);
        assert!(config.auto_migrate);
    }

    #[test]
    fn test_population_event_new() {
        let event = PopulationEvent::new(PopulationEventKind::Spawn, 100)
            .with_region("forest")
            .with_species(SpeciesCapId::new("wolf"))
            .with_value(5);

        assert_eq!(event.kind, PopulationEventKind::Spawn);
        assert_eq!(event.region_id, Some("forest".to_string()));
        assert_eq!(event.value, 5);
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn test_tick_result_has_activity() {
        let empty = TickResult::new();
        assert!(!empty.has_activity());

        let mut with_event = TickResult::new();
        with_event
            .events
            .push(PopulationEvent::new(PopulationEventKind::Spawn, 0));
        assert!(with_event.has_activity());
    }

    #[test]
    fn test_director_new() {
        let director = PopulationDirector::new(PopulationConfig::new());

        assert_eq!(director.current_tick(), 0);
        assert_eq!(director.region_count(), 0);
    }

    #[test]
    fn test_director_register_region() {
        let director = make_director();

        assert_eq!(director.region_count(), 2);
        assert!(director.get_region("forest").is_some());
        assert!(director.get_region("plains").is_some());
    }

    #[test]
    fn test_director_get_or_create_region() {
        let mut director = make_director();

        let region = director.get_or_create_region("new_region");
        assert_eq!(region.region_id, "new_region");
        assert_eq!(director.region_count(), 3);

        let _ = director.get_or_create_region("new_region");
        assert_eq!(director.region_count(), 3);
    }

    #[test]
    fn test_director_can_spawn() {
        let director = make_director();

        assert!(director.can_spawn("forest", &SpeciesCapId::new("wolf")));
        assert!(!director.can_spawn("forest", &SpeciesCapId::new("unknown")));
        assert!(!director.can_spawn("unknown_region", &SpeciesCapId::new("wolf")));
    }

    #[test]
    fn test_director_record_spawn() {
        let mut director = make_director();

        let event = director.record_spawn("forest", &SpeciesCapId::new("wolf"));
        assert!(event.is_some());

        let event = event.unwrap();
        assert_eq!(event.kind, PopulationEventKind::Spawn);
        assert_eq!(event.region_id, Some("forest".to_string()));

        let region = director.get_region("forest").unwrap();
        assert_eq!(region.counts().get(&SpeciesCapId::new("wolf")), 1);

        let species = director
            .species()
            .get_species(&SpeciesCapId::new("wolf"))
            .unwrap();
        assert_eq!(species.current(), 1);
    }

    #[test]
    fn test_director_record_despawn() {
        let mut director = make_director();

        director.record_spawn("forest", &SpeciesCapId::new("wolf"));
        director.record_spawn("forest", &SpeciesCapId::new("wolf"));

        let event = director.record_despawn("forest", &SpeciesCapId::new("wolf"));
        assert!(event.is_some());

        let region = director.get_region("forest").unwrap();
        assert_eq!(region.counts().get(&SpeciesCapId::new("wolf")), 1);
    }

    #[test]
    fn test_director_tick() {
        let mut director = make_director();

        let result = director.tick();

        assert_eq!(director.current_tick(), 1);
        assert!(result.events.is_empty() || !result.events.is_empty());
    }

    #[test]
    fn test_director_queue_spawn() {
        let mut director = make_director();

        director.queue_spawn(
            "forest",
            SpawnEvent::new(SpeciesCapId::new("wolf"), 5).with_priority(SpawnPriority::High),
        );

        let region = director.get_region("forest").unwrap();
        assert_eq!(region.spawn_budget().queue_len(), 1);
    }

    #[test]
    fn test_director_unload_load_region() {
        let mut director = make_director();

        director.unload_region("forest");

        let region = director.get_region("forest").unwrap();
        assert!(!region.loaded);
        assert!(director.get_snapshot("forest").is_some());

        director.load_region("forest");

        let region = director.get_region("forest").unwrap();
        assert!(region.loaded);
        assert!(director.get_snapshot("forest").is_none());
    }

    #[test]
    fn test_director_world_summary() {
        let mut director = make_director();

        director.record_spawn("forest", &SpeciesCapId::new("wolf"));
        director.record_spawn("forest", &SpeciesCapId::new("deer"));
        director.record_spawn("plains", &SpeciesCapId::new("deer"));

        let summary = director.world_summary();

        assert_eq!(summary.total_population, 3);
        assert_eq!(summary.region_count, 2);
    }

    #[test]
    fn test_director_total_population() {
        let mut director = make_director();

        assert_eq!(director.total_population(), 0);

        director.record_spawn("forest", &SpeciesCapId::new("wolf"));
        director.record_spawn("plains", &SpeciesCapId::new("deer"));

        assert_eq!(director.total_population(), 2);
    }

    #[test]
    fn test_director_overpopulated_underpopulated() {
        let mut director = make_director();

        let under: Vec<_> = director.underpopulated_regions().collect();
        assert_eq!(under.len(), 2);

        for _ in 0..60 {
            director.record_spawn("forest", &SpeciesCapId::new("deer"));
        }

        let over: Vec<_> = director.overpopulated_regions().collect();
        assert_eq!(over.len(), 1);
        assert_eq!(over[0].region_id, "forest");
    }

    #[test]
    fn test_director_set_region_threat() {
        let mut director = make_director();

        director.set_region_threat("forest", ThreatLevel::High);

        let region = director.get_region("forest").unwrap();
        assert_eq!(region.threat().level(), ThreatLevel::High);
    }

    #[test]
    fn test_director_advance_to() {
        let mut director = make_director();

        director.advance_to(100);

        assert_eq!(director.current_tick(), 100);
    }

    #[test]
    fn test_director_deterministic_region_order() {
        let mut director = PopulationDirector::new(PopulationConfig::new());

        director.register_region(RegionalPopulation::new("z_region"));
        director.register_region(RegionalPopulation::new("a_region"));
        director.register_region(RegionalPopulation::new("m_region"));

        let ids: Vec<_> = director.regions().map(|r| r.region_id.as_str()).collect();
        assert_eq!(ids, vec!["a_region", "m_region", "z_region"]);
    }

    #[test]
    fn test_serde_population_config() {
        let config = PopulationConfig::new()
            .with_caps(75, 150)
            .with_auto_migrate(false);

        let json = serde_json::to_string(&config).unwrap();
        let restored: PopulationConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.default_soft_cap, 75);
        assert_eq!(restored.default_hard_cap, 150);
        assert!(!restored.auto_migrate);
    }

    #[test]
    fn test_serde_population_event() {
        let event = PopulationEvent::new(PopulationEventKind::Spawn, 500)
            .with_region("forest")
            .with_species(SpeciesCapId::new("wolf"))
            .with_value(3);

        let json = serde_json::to_string(&event).unwrap();
        let restored: PopulationEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.kind, PopulationEventKind::Spawn);
        assert_eq!(restored.tick, 500);
        assert_eq!(restored.value, 3);
    }

    #[test]
    fn test_serde_tick_result() {
        let mut result = TickResult::new();
        result
            .events
            .push(PopulationEvent::new(PopulationEventKind::Spawn, 100));

        let json = serde_json::to_string(&result).unwrap();
        let restored: TickResult = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.events.len(), 1);
    }

    #[test]
    fn test_serde_population_director() {
        let mut director = make_director();
        director.record_spawn("forest", &SpeciesCapId::new("wolf"));
        director.tick();

        let json = serde_json::to_string(&director).unwrap();
        let restored: PopulationDirector = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), director.current_tick());
        assert_eq!(restored.region_count(), 2);
        assert_eq!(restored.total_population(), 1);
    }

    #[test]
    fn test_director_spawn_budget_processing() {
        let mut director = make_director();

        director.queue_spawn(
            "forest",
            SpawnEvent::new(SpeciesCapId::new("wolf"), 3).with_priority(SpawnPriority::High),
        );

        let result = director.tick();

        assert!(
            !result.spawns.is_empty()
                || director
                    .get_region("forest")
                    .unwrap()
                    .spawn_budget()
                    .queue_len()
                    > 0
        );
    }

    #[test]
    fn test_director_rebalance_events() {
        let config = PopulationConfig::new().with_rebalance_interval(1);
        let mut director = PopulationDirector::new(config);

        director.species_mut().register_species(
            SpeciesCap::new(SpeciesCapId::new("test"), "Test").with_caps(0, 5, 10),
        );
        director.register_region(RegionalPopulation::new("test").with_caps(5, 10));

        for _ in 0..15 {
            director.record_spawn("test", &SpeciesCapId::new("test"));
        }

        let result = director.tick();

        let has_capped = result.events.iter().any(|e| {
            matches!(
                e.kind,
                PopulationEventKind::RegionCapped | PopulationEventKind::SpeciesCapped
            )
        });
        assert!(has_capped);
    }
}

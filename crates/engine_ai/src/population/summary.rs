//! Summaries and snapshots for population state, supporting unloaded-chunk simulation.

use super::budget::PacingIntensity;
use super::region::RegionalPopulation;
use super::threat::ThreatLevel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Population density classification.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RegionDensity {
    /// Region is empty.
    Empty,
    /// Very sparse population.
    Sparse,
    /// Low population.
    Low,
    /// Moderate population.
    #[default]
    Moderate,
    /// High population.
    High,
    /// Very high population.
    Crowded,
    /// At or near cap.
    Packed,
}

impl RegionDensity {
    /// Create from pressure value (0.0-1.0).
    #[must_use]
    pub fn from_pressure(pressure: f32) -> Self {
        let clamped = pressure.clamp(0.0, 1.0);
        if clamped < 0.01 {
            Self::Empty
        } else if clamped < 0.15 {
            Self::Sparse
        } else if clamped < 0.35 {
            Self::Low
        } else if clamped < 0.55 {
            Self::Moderate
        } else if clamped < 0.75 {
            Self::High
        } else if clamped < 0.9 {
            Self::Crowded
        } else {
            Self::Packed
        }
    }

    /// Convert to pressure estimate.
    #[must_use]
    pub fn to_pressure(self) -> f32 {
        match self {
            Self::Empty => 0.0,
            Self::Sparse => 0.1,
            Self::Low => 0.25,
            Self::Moderate => 0.45,
            Self::High => 0.65,
            Self::Crowded => 0.82,
            Self::Packed => 0.95,
        }
    }
}

/// Spawn pressure classification.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SpawnPressure {
    /// No spawn pressure, below minimum.
    Deficit,
    /// Low spawn pressure.
    Low,
    /// Normal spawn pressure.
    #[default]
    Normal,
    /// High spawn pressure, approaching cap.
    High,
    /// At cap, no more spawns.
    Capped,
}

impl SpawnPressure {
    /// Create from population pressure and whether below minimum.
    #[must_use]
    pub fn from_state(pressure: f32, below_minimum: bool) -> Self {
        if below_minimum {
            Self::Deficit
        } else if pressure >= 1.0 {
            Self::Capped
        } else if pressure > 0.8 {
            Self::High
        } else if pressure > 0.4 {
            Self::Normal
        } else {
            Self::Low
        }
    }

    /// Get spawn rate modifier.
    #[must_use]
    pub fn spawn_modifier(self) -> f32 {
        match self {
            Self::Deficit => 2.0,
            Self::Low => 1.5,
            Self::Normal => 1.0,
            Self::High => 0.5,
            Self::Capped => 0.0,
        }
    }

    /// Check if spawning is allowed.
    #[must_use]
    pub fn can_spawn(self) -> bool {
        self != Self::Capped
    }
}

/// Summary of population state for a region.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PopulationSummary {
    /// Region identifier.
    pub region_id: String,
    /// Total population.
    pub total_population: u32,
    /// Hostile population.
    pub hostile_population: u32,
    /// Passive population.
    pub passive_population: u32,
    /// Density classification.
    pub density: RegionDensity,
    /// Spawn pressure.
    pub spawn_pressure: SpawnPressure,
    /// Threat level.
    pub threat_level: ThreatLevel,
    /// Pacing intensity.
    pub pacing: PacingIntensity,
    /// Population pressure (0.0-1.0).
    pub pressure: f32,
    /// Whether region needs attention.
    pub needs_attention: bool,
    /// Tick when summary was computed.
    pub computed_at_tick: u64,
}

impl PopulationSummary {
    /// Create a new empty summary.
    #[must_use]
    pub fn new(region_id: impl Into<String>) -> Self {
        Self {
            region_id: region_id.into(),
            ..Default::default()
        }
    }

    /// Create from a regional population.
    #[must_use]
    pub fn from_region(region: &RegionalPopulation, tick: u64) -> Self {
        let counts = region.counts();
        let pressure = region.pressure();
        let below_min = counts.total() == 0;

        Self {
            region_id: region.region_id.clone(),
            total_population: counts.total(),
            hostile_population: counts.hostile(),
            passive_population: counts.passive(),
            density: RegionDensity::from_pressure(pressure),
            spawn_pressure: SpawnPressure::from_state(pressure, below_min),
            threat_level: region.threat().level(),
            pacing: region.spawn_budget().pacing().intensity(),
            pressure,
            needs_attention: region.at_hard_cap() || region.threat().is_dangerous(),
            computed_at_tick: tick,
        }
    }

    /// Get hostile ratio.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "population counts are bounded")]
    pub fn hostile_ratio(&self) -> f32 {
        if self.total_population == 0 {
            0.0
        } else {
            self.hostile_population as f32 / self.total_population as f32
        }
    }

    /// Check if the region is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_population == 0
    }

    /// Check if the region is packed.
    #[must_use]
    pub fn is_packed(&self) -> bool {
        self.density == RegionDensity::Packed
    }

    /// Merge with another summary (for aggregation).
    pub fn merge(&mut self, other: &Self) {
        self.total_population += other.total_population;
        self.hostile_population += other.hostile_population;
        self.passive_population += other.passive_population;

        let combined_pressure = f32::midpoint(self.pressure, other.pressure);
        self.pressure = combined_pressure;
        self.density = RegionDensity::from_pressure(combined_pressure);

        if other.threat_level > self.threat_level {
            self.threat_level = other.threat_level;
        }

        self.needs_attention = self.needs_attention || other.needs_attention;
    }
}

/// A snapshot of population state suitable for persistence or unloaded region simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationSnapshot {
    /// Summary of population state.
    pub summary: PopulationSummary,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
    /// Time acceleration factor for unloaded simulation.
    pub time_acceleration: f32,
    /// Estimated ticks until next significant event.
    pub ticks_until_event: Option<u64>,
    /// Whether this region should be loaded soon.
    pub should_load: bool,
    /// Per-species population estimates.
    species_estimates: BTreeMap<String, u32>,
}

impl PopulationSnapshot {
    /// Create a new snapshot.
    #[must_use]
    pub fn new(summary: PopulationSummary, tick: u64) -> Self {
        let should_load = summary.needs_attention;
        let ticks_until_event = Some(Self::estimate_ticks_until_event(&summary));

        Self {
            summary,
            snapshot_tick: tick,
            time_acceleration: 1.0,
            ticks_until_event,
            should_load,
            species_estimates: BTreeMap::new(),
        }
    }

    /// Create from a regional population.
    #[must_use]
    pub fn from_region(region: &RegionalPopulation, tick: u64) -> Self {
        let summary = PopulationSummary::from_region(region, tick);
        let mut snapshot = Self::new(summary, tick);

        for (species, count) in region.counts().iter() {
            snapshot
                .species_estimates
                .insert(species.as_str().to_string(), count);
        }

        snapshot
    }

    fn estimate_ticks_until_event(summary: &PopulationSummary) -> u64 {
        if summary.needs_attention {
            0
        } else if summary.spawn_pressure == SpawnPressure::High {
            100
        } else if summary.spawn_pressure == SpawnPressure::Deficit {
            50
        } else if summary.threat_level >= ThreatLevel::High {
            200
        } else {
            500
        }
    }

    /// Check if snapshot is stale.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Get the age of this snapshot in ticks.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }

    /// Check if intervention might be needed soon.
    #[must_use]
    pub fn needs_intervention(&self, tick_threshold: u64) -> bool {
        self.summary.needs_attention || self.ticks_until_event.is_some_and(|t| t < tick_threshold)
    }

    /// Estimate population after elapsed ticks (simple projection).
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        reason = "population projection bounded and clamped"
    )]
    pub fn project_population(&self, elapsed_ticks: u64) -> u32 {
        let rate = match self.summary.spawn_pressure {
            SpawnPressure::Deficit => 0.001,
            SpawnPressure::Low => 0.0005,
            SpawnPressure::Normal => 0.0,
            SpawnPressure::High => -0.0002,
            SpawnPressure::Capped => -0.0005,
        };

        let delta = (self.summary.total_population as f32 * rate * elapsed_ticks as f32) as i32;
        (self.summary.total_population as i32 + delta).max(0) as u32
    }

    /// Get species population estimate.
    #[must_use]
    pub fn get_species_estimate(&self, species: &str) -> u32 {
        self.species_estimates.get(species).copied().unwrap_or(0)
    }

    /// Set time acceleration.
    pub fn set_time_acceleration(&mut self, factor: f32) {
        self.time_acceleration = factor.max(0.0);
    }
}

/// Aggregated summary across multiple regions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldPopulationSummary {
    /// Total population across all regions.
    pub total_population: u32,
    /// Total hostile population.
    pub hostile_population: u32,
    /// Total passive population.
    pub passive_population: u32,
    /// Number of regions.
    pub region_count: u32,
    /// Number of regions needing attention.
    pub regions_needing_attention: u32,
    /// Per-region summaries.
    regions: BTreeMap<String, PopulationSummary>,
    /// Tick when computed.
    pub computed_at_tick: u64,
}

impl WorldPopulationSummary {
    /// Create a new world summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a region summary.
    pub fn add_region(&mut self, summary: PopulationSummary) {
        self.total_population += summary.total_population;
        self.hostile_population += summary.hostile_population;
        self.passive_population += summary.passive_population;
        self.region_count += 1;

        if summary.needs_attention {
            self.regions_needing_attention += 1;
        }

        self.regions.insert(summary.region_id.clone(), summary);
    }

    /// Get a region summary.
    #[must_use]
    pub fn get_region(&self, region_id: &str) -> Option<&PopulationSummary> {
        self.regions.get(region_id)
    }

    /// Iterate over region summaries (deterministic order).
    pub fn regions(&self) -> impl Iterator<Item = &PopulationSummary> {
        self.regions.values()
    }

    /// Get regions needing attention.
    pub fn attention_needed(&self) -> impl Iterator<Item = &PopulationSummary> {
        self.regions.values().filter(|s| s.needs_attention)
    }

    /// Get average pressure across all regions.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "region count is bounded")]
    pub fn average_pressure(&self) -> f32 {
        if self.region_count == 0 {
            return 0.0;
        }

        let total_pressure: f32 = self.regions.values().map(|s| s.pressure).sum();
        total_pressure / self.region_count as f32
    }

    /// Get hostile ratio across all regions.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "population counts are bounded")]
    pub fn hostile_ratio(&self) -> f32 {
        if self.total_population == 0 {
            0.0
        } else {
            self.hostile_population as f32 / self.total_population as f32
        }
    }

    /// Create from an iterator of region summaries.
    pub fn from_summaries(summaries: impl Iterator<Item = PopulationSummary>, tick: u64) -> Self {
        let mut world = Self::new();
        for summary in summaries {
            world.add_region(summary);
        }
        world.computed_at_tick = tick;
        world
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(id: &str, population: u32, cap: u32) -> RegionalPopulation {
        use crate::population::species::SpeciesCapId;

        let mut region = RegionalPopulation::new(id).with_caps(cap / 2, cap);
        region
            .counts_mut()
            .set(SpeciesCapId::new("test"), population, false);
        region
    }

    #[test]
    fn test_region_density_from_pressure() {
        assert_eq!(RegionDensity::from_pressure(0.0), RegionDensity::Empty);
        assert_eq!(RegionDensity::from_pressure(0.1), RegionDensity::Sparse);
        assert_eq!(RegionDensity::from_pressure(0.5), RegionDensity::Moderate);
        assert_eq!(RegionDensity::from_pressure(0.95), RegionDensity::Packed);
    }

    #[test]
    fn test_region_density_roundtrip() {
        for density in [
            RegionDensity::Empty,
            RegionDensity::Sparse,
            RegionDensity::Low,
            RegionDensity::Moderate,
            RegionDensity::High,
            RegionDensity::Crowded,
            RegionDensity::Packed,
        ] {
            let pressure = density.to_pressure();
            let restored = RegionDensity::from_pressure(pressure);
            assert_eq!(density, restored);
        }
    }

    #[test]
    fn test_spawn_pressure_from_state() {
        assert_eq!(SpawnPressure::from_state(0.3, true), SpawnPressure::Deficit);
        assert_eq!(SpawnPressure::from_state(0.3, false), SpawnPressure::Low);
        assert_eq!(SpawnPressure::from_state(0.6, false), SpawnPressure::Normal);
        assert_eq!(SpawnPressure::from_state(0.9, false), SpawnPressure::High);
        assert_eq!(SpawnPressure::from_state(1.0, false), SpawnPressure::Capped);
    }

    #[test]
    fn test_spawn_pressure_can_spawn() {
        assert!(SpawnPressure::Deficit.can_spawn());
        assert!(SpawnPressure::Normal.can_spawn());
        assert!(!SpawnPressure::Capped.can_spawn());
    }

    #[test]
    fn test_population_summary_new() {
        let summary = PopulationSummary::new("test_region");

        assert_eq!(summary.region_id, "test_region");
        assert_eq!(summary.total_population, 0);
        assert!(!summary.needs_attention);
    }

    #[test]
    fn test_population_summary_from_region() {
        let region = make_region("test", 50, 100);
        let summary = PopulationSummary::from_region(&region, 1000);

        assert_eq!(summary.region_id, "test");
        assert_eq!(summary.total_population, 50);
        assert!((summary.pressure - 0.5).abs() < f32::EPSILON);
        assert_eq!(summary.computed_at_tick, 1000);
    }

    #[test]
    fn test_population_summary_hostile_ratio() {
        let mut summary = PopulationSummary::new("test");
        summary.total_population = 100;
        summary.hostile_population = 25;
        summary.passive_population = 75;

        assert!((summary.hostile_ratio() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_population_summary_merge() {
        let mut s1 = PopulationSummary::new("a");
        s1.total_population = 50;
        s1.hostile_population = 20;
        s1.pressure = 0.5;

        let mut s2 = PopulationSummary::new("b");
        s2.total_population = 30;
        s2.hostile_population = 10;
        s2.pressure = 0.3;

        s1.merge(&s2);

        assert_eq!(s1.total_population, 80);
        assert_eq!(s1.hostile_population, 30);
        assert!((s1.pressure - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_population_snapshot_new() {
        let summary = PopulationSummary::new("test");
        let snapshot = PopulationSnapshot::new(summary, 500);

        assert_eq!(snapshot.snapshot_tick, 500);
        assert!(snapshot.ticks_until_event.is_some());
    }

    #[test]
    fn test_population_snapshot_from_region() {
        let region = make_region("test", 50, 100);
        let snapshot = PopulationSnapshot::from_region(&region, 1000);

        assert_eq!(snapshot.summary.total_population, 50);
        assert_eq!(snapshot.snapshot_tick, 1000);
    }

    #[test]
    fn test_population_snapshot_staleness() {
        let summary = PopulationSummary::new("test");
        let snapshot = PopulationSnapshot::new(summary, 100);

        assert!(!snapshot.is_stale(150, 100));
        assert!(snapshot.is_stale(250, 100));
    }

    #[test]
    fn test_population_snapshot_project_population() {
        let mut summary = PopulationSummary::new("test");
        summary.total_population = 100;
        summary.spawn_pressure = SpawnPressure::Deficit;

        let snapshot = PopulationSnapshot::new(summary, 0);
        let projected = snapshot.project_population(1000);

        assert!(projected > 100);
    }

    #[test]
    fn test_population_snapshot_needs_intervention() {
        let mut summary = PopulationSummary::new("test");
        summary.needs_attention = true;

        let snapshot = PopulationSnapshot::new(summary, 0);

        assert!(snapshot.needs_intervention(1000));
    }

    #[test]
    fn test_world_population_summary_new() {
        let world = WorldPopulationSummary::new();

        assert_eq!(world.total_population, 0);
        assert_eq!(world.region_count, 0);
    }

    #[test]
    fn test_world_population_summary_add_region() {
        let mut world = WorldPopulationSummary::new();

        let mut s1 = PopulationSummary::new("region_a");
        s1.total_population = 50;
        s1.hostile_population = 20;

        let mut s2 = PopulationSummary::new("region_b");
        s2.total_population = 30;
        s2.hostile_population = 10;
        s2.needs_attention = true;

        world.add_region(s1);
        world.add_region(s2);

        assert_eq!(world.total_population, 80);
        assert_eq!(world.hostile_population, 30);
        assert_eq!(world.region_count, 2);
        assert_eq!(world.regions_needing_attention, 1);
    }

    #[test]
    fn test_world_population_summary_get_region() {
        let mut world = WorldPopulationSummary::new();
        let summary = PopulationSummary::new("test_region");
        world.add_region(summary);

        assert!(world.get_region("test_region").is_some());
        assert!(world.get_region("other").is_none());
    }

    #[test]
    fn test_world_population_summary_average_pressure() {
        let mut world = WorldPopulationSummary::new();

        let mut s1 = PopulationSummary::new("a");
        s1.pressure = 0.4;

        let mut s2 = PopulationSummary::new("b");
        s2.pressure = 0.6;

        world.add_region(s1);
        world.add_region(s2);

        assert!((world.average_pressure() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_world_population_summary_from_summaries() {
        let summaries = vec![
            {
                let mut s = PopulationSummary::new("a");
                s.total_population = 50;
                s
            },
            {
                let mut s = PopulationSummary::new("b");
                s.total_population = 30;
                s
            },
        ];

        let world = WorldPopulationSummary::from_summaries(summaries.into_iter(), 1000);

        assert_eq!(world.total_population, 80);
        assert_eq!(world.region_count, 2);
        assert_eq!(world.computed_at_tick, 1000);
    }

    #[test]
    fn test_world_summary_deterministic_order() {
        let mut world = WorldPopulationSummary::new();

        world.add_region(PopulationSummary::new("z_region"));
        world.add_region(PopulationSummary::new("a_region"));
        world.add_region(PopulationSummary::new("m_region"));

        let ids: Vec<_> = world.regions().map(|s| s.region_id.as_str()).collect();
        assert_eq!(ids, vec!["a_region", "m_region", "z_region"]);
    }

    #[test]
    fn test_serde_population_summary() {
        let mut summary = PopulationSummary::new("test_region");
        summary.total_population = 75;
        summary.pressure = 0.75;
        summary.density = RegionDensity::High;

        let json = serde_json::to_string(&summary).unwrap();
        let restored: PopulationSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id, "test_region");
        assert_eq!(restored.total_population, 75);
        assert_eq!(restored.density, RegionDensity::High);
    }

    #[test]
    fn test_serde_population_snapshot() {
        let summary = PopulationSummary::new("test");
        let snapshot = PopulationSnapshot::new(summary, 1000);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: PopulationSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.snapshot_tick, 1000);
    }

    #[test]
    fn test_serde_world_summary() {
        let mut world = WorldPopulationSummary::new();
        world.add_region(PopulationSummary::new("region_a"));
        world.computed_at_tick = 500;

        let json = serde_json::to_string(&world).unwrap();
        let restored: WorldPopulationSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_count, 1);
        assert_eq!(restored.computed_at_tick, 500);
    }
}

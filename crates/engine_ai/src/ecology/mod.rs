//! Ecological simulation for food chains, resource zones, migration paths, and infestation fronts.
//!
//! Provides deterministic, data-driven simulation of ecosystem dynamics:
//!
//! - Trophic relationships and food chain modeling
//! - Resource zone distribution and renewal
//! - Migration paths for population movement
//! - Infestation front propagation and containment
//! - Summaries and projections for unloaded regions
//! - Stable fingerprints for determinism verification
//! - Resource sustainability tuning and harvest pressure tracking
//! - Multi-species population ecosystem balancing over time

pub mod ecosystem;
pub mod sustainability;

pub use ecosystem::{
    CompetitorRelation, EcosystemConfig, EcosystemEvent, EcosystemEventKind, EcosystemFingerprint,
    EcosystemProjection, EcosystemRegion, EcosystemRegionId, EcosystemSimulator, EcosystemSummary,
    EcosystemTickResult, MigrationCorridor, Population, PopulationKey, PredatorPreyRelation,
    Species, SpeciesId, TrophicRole,
};
pub use sustainability::{
    CarryingCapacityConfig, DepletionBehavior, DepletionProjection, HarvestPressure,
    RecoveryProjection, RegenerationMode, SustainabilityEvent, SustainabilityEventKind,
    SustainabilityFingerprint, SustainabilityPolicy, SustainabilityRating, SustainabilitySummary,
    SustainabilityTickResult, SustainabilityTracker,
};

use crate::population::SpeciesCapId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::Hash;

/// Identifier for a trophic level in a food chain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TrophicLevelId(pub String);

impl TrophicLevelId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for TrophicLevelId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for TrophicLevelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifier for a resource zone.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResourceZoneId(pub String);

impl ResourceZoneId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ResourceZoneId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for ResourceZoneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifier for a migration path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MigrationPathId(pub String);

impl MigrationPathId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for MigrationPathId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for MigrationPathId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifier for an infestation front.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InfestationFrontId(pub u64);

impl InfestationFrontId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for InfestationFrontId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "infestation:{}", self.0)
    }
}

/// Type of resource in a zone.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    #[default]
    Food,
    Water,
    Shelter,
    Breeding,
    Custom(String),
}

/// Trophic relationship between species.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrophicRelation {
    Predator,
    Prey,
    Competitor,
    Symbiont,
    Parasite,
    Host,
}

/// A link in a food chain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrophicLink {
    pub source: SpeciesCapId,
    pub target: SpeciesCapId,
    pub relation: TrophicRelation,
    pub strength: f32,
    pub min_ratio: f32,
    pub max_ratio: f32,
}

impl TrophicLink {
    #[must_use]
    pub fn new(source: SpeciesCapId, target: SpeciesCapId, relation: TrophicRelation) -> Self {
        Self {
            source,
            target,
            relation,
            strength: 1.0,
            min_ratio: 0.1,
            max_ratio: 10.0,
        }
    }

    #[must_use]
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 10.0);
        self
    }

    #[must_use]
    pub fn with_ratio_bounds(mut self, min: f32, max: f32) -> Self {
        self.min_ratio = min.max(0.01);
        self.max_ratio = max.max(self.min_ratio);
        self
    }
}

/// A food chain definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoodChain {
    pub id: TrophicLevelId,
    pub name: String,
    links: Vec<TrophicLink>,
    species: Vec<SpeciesCapId>,
}

impl FoodChain {
    #[must_use]
    pub fn new(id: TrophicLevelId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            links: Vec::new(),
            species: Vec::new(),
        }
    }

    pub fn add_link(&mut self, link: TrophicLink) {
        if !self.species.contains(&link.source) {
            self.species.push(link.source.clone());
            self.species.sort();
        }
        if !self.species.contains(&link.target) {
            self.species.push(link.target.clone());
            self.species.sort();
        }
        self.links.push(link);
    }

    #[must_use]
    pub fn links(&self) -> &[TrophicLink] {
        &self.links
    }

    #[must_use]
    pub fn species(&self) -> &[SpeciesCapId] {
        &self.species
    }

    pub fn predators_of(&self, species: &SpeciesCapId) -> impl Iterator<Item = &TrophicLink> {
        self.links.iter().filter(move |l| {
            &l.target == species && matches!(l.relation, TrophicRelation::Predator)
        })
    }

    pub fn prey_of(&self, species: &SpeciesCapId) -> impl Iterator<Item = &TrophicLink> {
        self.links.iter().filter(move |l| {
            &l.source == species && matches!(l.relation, TrophicRelation::Predator)
        })
    }
}

/// Resource zone state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceZone {
    pub id: ResourceZoneId,
    pub region: String,
    pub kind: ResourceKind,
    capacity: f32,
    current: f32,
    renewal_rate: f32,
    depletion_rate: f32,
    last_tick: u64,
}

impl ResourceZone {
    #[must_use]
    pub fn new(id: ResourceZoneId, region: impl Into<String>, kind: ResourceKind) -> Self {
        Self {
            id,
            region: region.into(),
            kind,
            capacity: 1000.0,
            current: 1000.0,
            renewal_rate: 1.0,
            depletion_rate: 0.0,
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: f32) -> Self {
        self.capacity = capacity.max(0.0);
        self.current = self.current.min(self.capacity);
        self
    }

    #[must_use]
    pub fn with_current(mut self, current: f32) -> Self {
        self.current = current.clamp(0.0, self.capacity);
        self
    }

    #[must_use]
    pub fn with_renewal_rate(mut self, rate: f32) -> Self {
        self.renewal_rate = rate.max(0.0);
        self
    }

    #[must_use]
    pub fn capacity(&self) -> f32 {
        self.capacity
    }

    #[must_use]
    pub fn current(&self) -> f32 {
        self.current
    }

    #[must_use]
    pub fn renewal_rate(&self) -> f32 {
        self.renewal_rate
    }

    #[must_use]
    pub fn availability(&self) -> f32 {
        if self.capacity > 0.0 {
            self.current / self.capacity
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn is_depleted(&self) -> bool {
        self.current < 0.01 * self.capacity
    }

    pub fn consume(&mut self, amount: f32) -> f32 {
        let consumed = amount.min(self.current);
        self.current -= consumed;
        consumed
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn tick(&mut self, current_tick: u64) {
        let elapsed = current_tick.saturating_sub(self.last_tick);
        if elapsed == 0 {
            return;
        }
        self.last_tick = current_tick;

        let renewal = self.renewal_rate * elapsed as f32;
        let headroom = self.capacity - self.current;
        self.current += renewal.min(headroom);
    }

    pub fn set_depletion_rate(&mut self, rate: f32) {
        self.depletion_rate = rate.max(0.0);
    }
}

/// Migration path between regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MigrationPath {
    pub id: MigrationPathId,
    pub source_region: String,
    pub destination_region: String,
    pub distance: f32,
    pub difficulty: f32,
    seasonal: bool,
    active_seasons: Vec<u8>,
    capacity: u32,
    species_affinity: BTreeMap<SpeciesCapId, f32>,
}

impl MigrationPath {
    #[must_use]
    pub fn new(
        id: MigrationPathId,
        source: impl Into<String>,
        destination: impl Into<String>,
    ) -> Self {
        Self {
            id,
            source_region: source.into(),
            destination_region: destination.into(),
            distance: 100.0,
            difficulty: 0.5,
            seasonal: false,
            active_seasons: vec![0, 1, 2, 3],
            capacity: 100,
            species_affinity: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance.max(1.0);
        self
    }

    #[must_use]
    pub fn with_difficulty(mut self, difficulty: f32) -> Self {
        self.difficulty = difficulty.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_seasonal(mut self, seasons: Vec<u8>) -> Self {
        self.seasonal = !seasons.is_empty();
        self.active_seasons = seasons;
        self
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    #[must_use]
    pub fn with_species_affinity(mut self, species: SpeciesCapId, affinity: f32) -> Self {
        self.species_affinity
            .insert(species, affinity.clamp(0.0, 2.0));
        self
    }

    #[must_use]
    pub fn is_active(&self, season: u8) -> bool {
        !self.seasonal || self.active_seasons.contains(&season)
    }

    #[must_use]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    #[must_use]
    pub fn affinity_for(&self, species: &SpeciesCapId) -> f32 {
        self.species_affinity.get(species).copied().unwrap_or(1.0)
    }

    #[must_use]
    pub fn travel_cost(&self, species: &SpeciesCapId) -> f32 {
        let base = self.distance * (1.0 + self.difficulty);
        base / self.affinity_for(species)
    }
}

/// Phase of an infestation front.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfestationPhase {
    #[default]
    Dormant,
    Emerging,
    Spreading,
    Peak,
    Declining,
    Contained,
}

impl InfestationPhase {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Emerging | Self::Spreading | Self::Peak)
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Contained)
    }

    #[must_use]
    pub fn spread_multiplier(self) -> f32 {
        match self {
            Self::Dormant | Self::Contained => 0.0,
            Self::Emerging => 0.3,
            Self::Spreading => 1.0,
            Self::Peak => 0.7,
            Self::Declining => 0.2,
        }
    }
}

/// Type of infestation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfestationType {
    Plague,
    #[default]
    Swarm,
    Blight,
    Fungal,
    Parasitic,
    Custom(String),
}

/// An infestation front spreading across regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InfestationFront {
    pub id: InfestationFrontId,
    pub infestation_type: InfestationType,
    phase: InfestationPhase,
    origin_region: String,
    affected_regions: Vec<String>,
    intensity: f32,
    spread_rate: f32,
    decay_rate: f32,
    start_tick: u64,
    last_spread_tick: u64,
    containment_level: f32,
}

impl InfestationFront {
    #[must_use]
    pub fn new(
        id: InfestationFrontId,
        origin: impl Into<String>,
        infestation_type: InfestationType,
        start_tick: u64,
    ) -> Self {
        let origin_region = origin.into();
        Self {
            id,
            infestation_type,
            phase: InfestationPhase::Emerging,
            origin_region: origin_region.clone(),
            affected_regions: vec![origin_region],
            intensity: 0.1,
            spread_rate: 0.05,
            decay_rate: 0.01,
            start_tick,
            last_spread_tick: start_tick,
            containment_level: 0.0,
        }
    }

    #[must_use]
    pub fn with_spread_rate(mut self, rate: f32) -> Self {
        self.spread_rate = rate.clamp(0.001, 1.0);
        self
    }

    #[must_use]
    pub fn with_decay_rate(mut self, rate: f32) -> Self {
        self.decay_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn phase(&self) -> InfestationPhase {
        self.phase
    }

    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin_region
    }

    #[must_use]
    pub fn affected_regions(&self) -> &[String] {
        &self.affected_regions
    }

    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    #[must_use]
    pub fn containment_level(&self) -> f32 {
        self.containment_level
    }

    #[must_use]
    pub fn is_affecting(&self, region: &str) -> bool {
        self.affected_regions.iter().any(|r| r == region)
    }

    pub fn apply_containment(&mut self, amount: f32) {
        self.containment_level = (self.containment_level + amount).clamp(0.0, 1.0);
    }

    pub fn spread_to(&mut self, region: impl Into<String>) {
        let region = region.into();
        if !self.affected_regions.contains(&region) {
            self.affected_regions.push(region);
            self.affected_regions.sort();
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn tick(&mut self, current_tick: u64, adjacent_regions: &[String]) {
        if self.phase.is_finished() {
            return;
        }

        let elapsed = current_tick.saturating_sub(self.last_spread_tick);
        if elapsed == 0 {
            return;
        }

        let effective_spread = self.spread_rate * self.phase.spread_multiplier();
        let growth = effective_spread * elapsed as f32 * (1.0 - self.containment_level);
        let decay = self.decay_rate * elapsed as f32 * (1.0 + self.containment_level);

        self.intensity = (self.intensity + growth - decay).clamp(0.0, 1.0);

        self.phase = if self.intensity < 0.05 || self.containment_level > 0.9 {
            InfestationPhase::Contained
        } else if self.intensity < 0.2 {
            if self.phase == InfestationPhase::Peak || self.phase == InfestationPhase::Spreading {
                InfestationPhase::Declining
            } else {
                InfestationPhase::Emerging
            }
        } else if self.intensity < 0.7 {
            InfestationPhase::Spreading
        } else {
            InfestationPhase::Peak
        };

        if self.phase.is_active() && effective_spread > 0.0 {
            let spread_chance = effective_spread * (1.0 - self.containment_level);
            for region in adjacent_regions {
                if !self.affected_regions.contains(region) && spread_chance > 0.3 {
                    self.affected_regions.push(region.clone());
                }
            }
            self.affected_regions.sort();
        }

        self.last_spread_tick = current_tick;
    }
}

/// Configuration for ecological simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "config struct with boolean options"
)]
pub struct EcologyConfig {
    pub tick_interval: u64,
    pub resource_renewal_enabled: bool,
    pub migration_enabled: bool,
    pub infestation_enabled: bool,
    pub food_chain_enabled: bool,
    pub max_infestations: u32,
    pub resource_depletion_threshold: f32,
    pub migration_pressure_threshold: f32,
}

impl Default for EcologyConfig {
    fn default() -> Self {
        Self {
            tick_interval: 10,
            resource_renewal_enabled: true,
            migration_enabled: true,
            infestation_enabled: true,
            food_chain_enabled: true,
            max_infestations: 5,
            resource_depletion_threshold: 0.1,
            migration_pressure_threshold: 0.8,
        }
    }
}

impl EcologyConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_tick_interval(mut self, interval: u64) -> Self {
        self.tick_interval = interval.max(1);
        self
    }

    #[must_use]
    pub fn with_resource_renewal(mut self, enabled: bool) -> Self {
        self.resource_renewal_enabled = enabled;
        self
    }

    #[must_use]
    pub fn with_migration(mut self, enabled: bool) -> Self {
        self.migration_enabled = enabled;
        self
    }

    #[must_use]
    pub fn with_infestation(mut self, enabled: bool) -> Self {
        self.infestation_enabled = enabled;
        self
    }

    #[must_use]
    pub fn with_food_chain(mut self, enabled: bool) -> Self {
        self.food_chain_enabled = enabled;
        self
    }
}

/// Event generated by ecological simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EcologyEventKind {
    ResourceDepleted {
        zone_id: ResourceZoneId,
    },
    ResourceRecovered {
        zone_id: ResourceZoneId,
    },
    MigrationTriggered {
        path_id: MigrationPathId,
        species: SpeciesCapId,
        count: u32,
    },
    InfestationStarted {
        front_id: InfestationFrontId,
        region: String,
    },
    InfestationSpread {
        front_id: InfestationFrontId,
        new_region: String,
    },
    InfestationContained {
        front_id: InfestationFrontId,
    },
    FoodChainImbalance {
        chain_id: TrophicLevelId,
        species: SpeciesCapId,
        pressure: f32,
    },
    PopulationCrash {
        species: SpeciesCapId,
        region: String,
        severity: f32,
    },
}

/// An ecological event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EcologyEvent {
    pub tick: u64,
    pub kind: EcologyEventKind,
}

impl EcologyEvent {
    #[must_use]
    pub fn new(tick: u64, kind: EcologyEventKind) -> Self {
        Self { tick, kind }
    }
}

/// Snapshot of ecological state for a region.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EcologySnapshot {
    pub region: String,
    pub tick: u64,
    pub resource_availability: f32,
    pub infestation_pressure: f32,
    pub food_chain_health: f32,
    pub migration_pressure: f32,
    pub species_counts: BTreeMap<SpeciesCapId, u32>,
}

impl EcologySnapshot {
    #[must_use]
    pub fn new(region: impl Into<String>, tick: u64) -> Self {
        Self {
            region: region.into(),
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn overall_health(&self) -> f32 {
        let resource_health = self.resource_availability;
        let infestation_health = 1.0 - self.infestation_pressure;
        let chain_health = self.food_chain_health;
        (resource_health + infestation_health + chain_health) / 3.0
    }
}

/// Summary of ecological state for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EcologySummary {
    pub region: String,
    pub tick: u64,
    pub health: f32,
    pub active_infestations: u32,
    pub depleted_resources: u32,
}

impl From<&EcologySnapshot> for EcologySummary {
    fn from(snapshot: &EcologySnapshot) -> Self {
        Self {
            region: snapshot.region.clone(),
            tick: snapshot.tick,
            health: snapshot.overall_health(),
            active_infestations: u32::from(snapshot.infestation_pressure > 0.1),
            depleted_resources: u32::from(snapshot.resource_availability < 0.2),
        }
    }
}

/// Fingerprint for ecological state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EcologyFingerprint(pub u32);

impl EcologyFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for EcologyFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "eco:{:08x}", self.0)
    }
}

/// Result of an ecological simulation tick.
#[derive(Clone, Debug, Default)]
pub struct EcologyTickResult {
    pub events: Vec<EcologyEvent>,
    pub migrations_triggered: u32,
    pub resources_updated: u32,
    pub infestations_spread: u32,
}

/// Manager for ecological simulation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EcologySimulator {
    config: EcologyConfig,
    food_chains: BTreeMap<TrophicLevelId, FoodChain>,
    resource_zones: BTreeMap<ResourceZoneId, ResourceZone>,
    migration_paths: BTreeMap<MigrationPathId, MigrationPath>,
    infestations: BTreeMap<InfestationFrontId, InfestationFront>,
    region_snapshots: BTreeMap<String, EcologySnapshot>,
    next_infestation_id: u64,
    current_tick: u64,
    current_season: u8,
}

impl EcologySimulator {
    #[must_use]
    pub fn new(config: EcologyConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn config(&self) -> &EcologyConfig {
        &self.config
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn current_season(&self) -> u8 {
        self.current_season
    }

    pub fn set_season(&mut self, season: u8) {
        self.current_season = season % 4;
    }

    pub fn register_food_chain(&mut self, chain: FoodChain) {
        self.food_chains.insert(chain.id.clone(), chain);
    }

    pub fn register_resource_zone(&mut self, zone: ResourceZone) {
        self.resource_zones.insert(zone.id.clone(), zone);
    }

    pub fn register_migration_path(&mut self, path: MigrationPath) {
        self.migration_paths.insert(path.id.clone(), path);
    }

    #[must_use]
    pub fn get_food_chain(&self, id: &TrophicLevelId) -> Option<&FoodChain> {
        self.food_chains.get(id)
    }

    #[must_use]
    pub fn get_resource_zone(&self, id: &ResourceZoneId) -> Option<&ResourceZone> {
        self.resource_zones.get(id)
    }

    pub fn get_resource_zone_mut(&mut self, id: &ResourceZoneId) -> Option<&mut ResourceZone> {
        self.resource_zones.get_mut(id)
    }

    #[must_use]
    pub fn get_migration_path(&self, id: &MigrationPathId) -> Option<&MigrationPath> {
        self.migration_paths.get(id)
    }

    #[must_use]
    pub fn get_infestation(&self, id: &InfestationFrontId) -> Option<&InfestationFront> {
        self.infestations.get(id)
    }

    pub fn food_chains(&self) -> impl Iterator<Item = &FoodChain> {
        self.food_chains.values()
    }

    pub fn resource_zones(&self) -> impl Iterator<Item = &ResourceZone> {
        self.resource_zones.values()
    }

    pub fn migration_paths(&self) -> impl Iterator<Item = &MigrationPath> {
        self.migration_paths.values()
    }

    pub fn active_infestations(&self) -> impl Iterator<Item = &InfestationFront> {
        self.infestations.values().filter(|i| i.phase().is_active())
    }

    pub fn zones_in_region(&self, region: &str) -> impl Iterator<Item = &ResourceZone> {
        self.resource_zones
            .values()
            .filter(move |z| z.region == region)
    }

    pub fn paths_from_region(&self, region: &str) -> impl Iterator<Item = &MigrationPath> {
        self.migration_paths
            .values()
            .filter(move |p| p.source_region == region || p.destination_region == region)
    }

    pub fn infestations_in_region(&self, region: &str) -> impl Iterator<Item = &InfestationFront> {
        self.infestations
            .values()
            .filter(move |i| i.is_affecting(region))
    }

    pub fn start_infestation(
        &mut self,
        origin: impl Into<String>,
        infestation_type: InfestationType,
    ) -> InfestationFrontId {
        let id = InfestationFrontId::new(self.next_infestation_id);
        self.next_infestation_id += 1;
        let front = InfestationFront::new(id, origin, infestation_type, self.current_tick);
        self.infestations.insert(id, front);
        id
    }

    pub fn contain_infestation(&mut self, id: &InfestationFrontId, amount: f32) {
        if let Some(front) = self.infestations.get_mut(id) {
            front.apply_containment(amount);
        }
    }

    #[must_use]
    pub fn snapshot_region(&self, region: &str) -> EcologySnapshot {
        self.region_snapshots
            .get(region)
            .cloned()
            .unwrap_or_else(|| EcologySnapshot::new(region, self.current_tick))
    }

    #[must_use]
    pub fn summary_region(&self, region: &str) -> EcologySummary {
        EcologySummary::from(&self.snapshot_region(region))
    }

    pub fn tick(&mut self) -> EcologyTickResult {
        self.current_tick += 1;
        let mut result = EcologyTickResult::default();

        if self.config.resource_renewal_enabled {
            for zone in self.resource_zones.values_mut() {
                let was_depleted = zone.is_depleted();
                zone.tick(self.current_tick);
                result.resources_updated += 1;

                if was_depleted && !zone.is_depleted() {
                    result.events.push(EcologyEvent::new(
                        self.current_tick,
                        EcologyEventKind::ResourceRecovered {
                            zone_id: zone.id.clone(),
                        },
                    ));
                } else if !was_depleted && zone.is_depleted() {
                    result.events.push(EcologyEvent::new(
                        self.current_tick,
                        EcologyEventKind::ResourceDepleted {
                            zone_id: zone.id.clone(),
                        },
                    ));
                }
            }
        }

        if self.config.infestation_enabled {
            let regions: Vec<String> = self
                .resource_zones
                .values()
                .map(|z| z.region.clone())
                .collect();

            let infestation_ids: Vec<InfestationFrontId> =
                self.infestations.keys().copied().collect();

            for id in infestation_ids {
                if let Some(front) = self.infestations.get_mut(&id) {
                    let was_active = front.phase().is_active();
                    let prev_regions = front.affected_regions().len();

                    let adjacent: Vec<String> = regions
                        .iter()
                        .filter(|r| !front.is_affecting(r))
                        .take(3)
                        .cloned()
                        .collect();

                    front.tick(self.current_tick, &adjacent);

                    let new_regions: Vec<String> = front
                        .affected_regions()
                        .iter()
                        .skip(prev_regions)
                        .cloned()
                        .collect();

                    for region in new_regions {
                        result.events.push(EcologyEvent::new(
                            self.current_tick,
                            EcologyEventKind::InfestationSpread {
                                front_id: id,
                                new_region: region,
                            },
                        ));
                        result.infestations_spread += 1;
                    }

                    if was_active && front.phase().is_finished() {
                        result.events.push(EcologyEvent::new(
                            self.current_tick,
                            EcologyEventKind::InfestationContained { front_id: id },
                        ));
                    }
                }
            }

            self.infestations.retain(|_, f| !f.phase().is_finished());
        }

        self.update_region_snapshots();
        result
    }

    #[allow(clippy::cast_precision_loss)]
    fn update_region_snapshots(&mut self) {
        let regions: Vec<String> = self
            .resource_zones
            .values()
            .map(|z| z.region.clone())
            .collect();

        for region in regions {
            let resource_availability: f32 = {
                let zones: Vec<&ResourceZone> = self.zones_in_region(&region).collect();
                if zones.is_empty() {
                    1.0
                } else {
                    zones.iter().map(|z| z.availability()).sum::<f32>() / zones.len() as f32
                }
            };

            let infestation_pressure: f32 = self
                .infestations_in_region(&region)
                .map(InfestationFront::intensity)
                .sum::<f32>()
                .min(1.0);

            let snapshot = EcologySnapshot {
                region: region.clone(),
                tick: self.current_tick,
                resource_availability,
                infestation_pressure,
                food_chain_health: 1.0,
                migration_pressure: 0.0,
                species_counts: BTreeMap::new(),
            };

            self.region_snapshots.insert(region, snapshot);
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> EcologyFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        hasher.update(&self.current_tick.to_le_bytes());
        hasher.update(&[self.current_season]);

        for (id, zone) in &self.resource_zones {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&zone.current().to_le_bytes());
        }

        for (id, front) in &self.infestations {
            hasher.update(&id.raw().to_le_bytes());
            hasher.update(&[front.phase() as u8]);
            hasher.update(&front.intensity().to_le_bytes());
        }

        EcologyFingerprint(hasher.finalize())
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn project(&self, ticks_ahead: u64, region: &str) -> EcologySnapshot {
        let mut snapshot = self.snapshot_region(region);
        snapshot.tick = self.current_tick + ticks_ahead;

        let zones: Vec<&ResourceZone> = self.zones_in_region(region).collect();
        if !zones.is_empty() {
            let avg_renewal: f32 =
                zones.iter().map(|z| z.renewal_rate()).sum::<f32>() / zones.len() as f32;
            let projected_gain = avg_renewal * ticks_ahead as f32;
            snapshot.resource_availability =
                (snapshot.resource_availability + projected_gain / 1000.0).min(1.0);
        }

        let active_count = self.infestations_in_region(region).count();
        if active_count > 0 {
            let decay_factor = 0.01 * ticks_ahead as f32;
            snapshot.infestation_pressure = (snapshot.infestation_pressure - decay_factor).max(0.0);
        }

        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zone(id: &str, region: &str) -> ResourceZone {
        ResourceZone::new(ResourceZoneId::new(id), region, ResourceKind::Food)
            .with_capacity(1000.0)
            .with_current(500.0)
            .with_renewal_rate(1.0)
    }

    fn make_path(id: &str) -> MigrationPath {
        MigrationPath::new(MigrationPathId::new(id), "region_a", "region_b")
            .with_distance(100.0)
            .with_difficulty(0.5)
    }

    fn make_chain(id: &str) -> FoodChain {
        FoodChain::new(TrophicLevelId::new(id), "Test Chain")
    }

    #[test]
    fn test_trophic_level_id() {
        let id = TrophicLevelId::new("forest_chain");
        assert_eq!(id.as_str(), "forest_chain");

        let id2: TrophicLevelId = "plains_chain".into();
        assert_eq!(id2.as_str(), "plains_chain");

        assert!(id < id2);
    }

    #[test]
    fn test_resource_zone_id() {
        let id = ResourceZoneId::new("berry_patch");
        assert_eq!(id.as_str(), "berry_patch");
        assert_eq!(format!("{id}"), "berry_patch");
    }

    #[test]
    fn test_migration_path_id() {
        let id = MigrationPathId::new("valley_corridor");
        assert_eq!(id.as_str(), "valley_corridor");
    }

    #[test]
    fn test_infestation_front_id() {
        let id = InfestationFrontId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "infestation:42");
    }

    #[test]
    fn test_trophic_link() {
        let link = TrophicLink::new(
            SpeciesCapId::new("wolf"),
            SpeciesCapId::new("deer"),
            TrophicRelation::Predator,
        )
        .with_strength(1.5)
        .with_ratio_bounds(0.05, 5.0);

        assert_eq!(link.source.as_str(), "wolf");
        assert_eq!(link.target.as_str(), "deer");
        assert!((link.strength - 1.5).abs() < f32::EPSILON);
        assert!((link.min_ratio - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn test_food_chain() {
        let mut chain = make_chain("forest");
        chain.add_link(TrophicLink::new(
            SpeciesCapId::new("wolf"),
            SpeciesCapId::new("deer"),
            TrophicRelation::Predator,
        ));
        chain.add_link(TrophicLink::new(
            SpeciesCapId::new("deer"),
            SpeciesCapId::new("grass"),
            TrophicRelation::Predator,
        ));

        assert_eq!(chain.links().len(), 2);
        assert_eq!(chain.species().len(), 3);

        let predators: Vec<_> = chain.predators_of(&SpeciesCapId::new("deer")).collect();
        assert_eq!(predators.len(), 1);
        assert_eq!(predators[0].source.as_str(), "wolf");

        let prey: Vec<_> = chain.prey_of(&SpeciesCapId::new("wolf")).collect();
        assert_eq!(prey.len(), 1);
        assert_eq!(prey[0].target.as_str(), "deer");
    }

    #[test]
    fn test_resource_zone() {
        let mut zone = make_zone("berries", "forest");
        assert!((zone.availability() - 0.5).abs() < f32::EPSILON);
        assert!(!zone.is_depleted());

        let consumed = zone.consume(400.0);
        assert!((consumed - 400.0).abs() < f32::EPSILON);
        assert!((zone.current() - 100.0).abs() < f32::EPSILON);

        zone.tick(10);
        assert!(zone.current() > 100.0);
    }

    #[test]
    fn test_resource_zone_depletion() {
        let mut zone = make_zone("scarce", "desert")
            .with_current(5.0)
            .with_renewal_rate(0.0);

        assert!(zone.is_depleted());
        zone.consume(10.0);
        assert!((zone.current() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_migration_path() {
        let path = make_path("corridor")
            .with_seasonal(vec![0, 2])
            .with_species_affinity(SpeciesCapId::new("bird"), 2.0);

        assert!(path.is_active(0));
        assert!(!path.is_active(1));
        assert!(path.is_active(2));

        assert!((path.affinity_for(&SpeciesCapId::new("bird")) - 2.0).abs() < f32::EPSILON);
        assert!((path.affinity_for(&SpeciesCapId::new("deer")) - 1.0).abs() < f32::EPSILON);

        let bird_cost = path.travel_cost(&SpeciesCapId::new("bird"));
        let deer_cost = path.travel_cost(&SpeciesCapId::new("deer"));
        assert!(bird_cost < deer_cost);
    }

    #[test]
    fn test_infestation_phase() {
        assert!(!InfestationPhase::Dormant.is_active());
        assert!(InfestationPhase::Spreading.is_active());
        assert!(InfestationPhase::Contained.is_finished());

        assert!((InfestationPhase::Spreading.spread_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((InfestationPhase::Dormant.spread_multiplier() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_infestation_front() {
        let mut front = InfestationFront::new(
            InfestationFrontId::new(1),
            "region_a",
            InfestationType::Swarm,
            0,
        )
        .with_spread_rate(0.1)
        .with_intensity(0.5);

        assert_eq!(front.phase(), InfestationPhase::Emerging);
        assert!(front.is_affecting("region_a"));
        assert!(!front.is_affecting("region_b"));

        front.spread_to("region_b");
        assert!(front.is_affecting("region_b"));
        assert_eq!(front.affected_regions().len(), 2);
    }

    #[test]
    fn test_infestation_containment() {
        let mut front = InfestationFront::new(
            InfestationFrontId::new(1),
            "origin",
            InfestationType::Blight,
            0,
        )
        .with_intensity(0.3);

        front.apply_containment(0.95);
        front.tick(100, &[]);

        assert!(front.containment_level() > 0.9);
        assert!(front.phase().is_finished());
    }

    #[test]
    fn test_ecology_config() {
        let config = EcologyConfig::new()
            .with_tick_interval(5)
            .with_infestation(false)
            .with_migration(false);

        assert_eq!(config.tick_interval, 5);
        assert!(!config.infestation_enabled);
        assert!(!config.migration_enabled);
        assert!(config.resource_renewal_enabled);
    }

    #[test]
    fn test_ecology_event() {
        let event = EcologyEvent::new(
            100,
            EcologyEventKind::ResourceDepleted {
                zone_id: ResourceZoneId::new("berries"),
            },
        );
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn test_ecology_snapshot() {
        let snapshot = EcologySnapshot {
            region: "forest".to_string(),
            tick: 100,
            resource_availability: 0.8,
            infestation_pressure: 0.2,
            food_chain_health: 0.9,
            migration_pressure: 0.1,
            species_counts: BTreeMap::new(),
        };

        let health = snapshot.overall_health();
        assert!(health > 0.7 && health < 0.9);

        let summary = EcologySummary::from(&snapshot);
        assert_eq!(summary.region, "forest");
        assert!((summary.health - health).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ecology_fingerprint() {
        let fp = EcologyFingerprint(0xDEAD_BEEF);
        assert_eq!(fp.raw(), 0xDEAD_BEEF);
        assert_eq!(format!("{fp}"), "eco:deadbeef");
    }

    #[test]
    fn test_simulator_basic() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        assert_eq!(sim.current_tick(), 0);

        sim.register_resource_zone(make_zone("berries", "forest"));
        sim.register_migration_path(make_path("corridor"));
        sim.register_food_chain(make_chain("forest"));

        assert!(
            sim.get_resource_zone(&ResourceZoneId::new("berries"))
                .is_some()
        );
        assert!(
            sim.get_migration_path(&MigrationPathId::new("corridor"))
                .is_some()
        );
        assert!(sim.get_food_chain(&TrophicLevelId::new("forest")).is_some());
    }

    #[test]
    fn test_simulator_tick() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        sim.register_resource_zone(make_zone("berries", "forest").with_current(50.0));

        let result = sim.tick();
        assert_eq!(sim.current_tick(), 1);
        assert!(result.resources_updated > 0);
    }

    #[test]
    fn test_simulator_infestation_lifecycle() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        sim.register_resource_zone(make_zone("berries", "forest"));

        let front_id = sim.start_infestation("forest", InfestationType::Swarm);
        assert!(sim.get_infestation(&front_id).is_some());

        for _ in 0..10 {
            sim.tick();
        }

        let front = sim.get_infestation(&front_id).unwrap();
        assert!(front.phase().is_active());

        sim.contain_infestation(&front_id, 0.95);
        for _ in 0..50 {
            sim.tick();
        }
    }

    #[test]
    fn test_simulator_season() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        assert_eq!(sim.current_season(), 0);

        sim.set_season(2);
        assert_eq!(sim.current_season(), 2);

        sim.set_season(5);
        assert_eq!(sim.current_season(), 1);
    }

    #[test]
    fn test_simulator_snapshot_and_projection() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        sim.register_resource_zone(make_zone("berries", "forest"));

        for _ in 0..5 {
            sim.tick();
        }

        let snapshot = sim.snapshot_region("forest");
        assert_eq!(snapshot.region, "forest");
        assert_eq!(snapshot.tick, 5);

        let projection = sim.project(100, "forest");
        assert_eq!(projection.tick, 105);
    }

    #[test]
    fn test_simulator_fingerprint_determinism() {
        let mut sim1 = EcologySimulator::new(EcologyConfig::new());
        let mut sim2 = EcologySimulator::new(EcologyConfig::new());

        sim1.register_resource_zone(make_zone("berries", "forest"));
        sim2.register_resource_zone(make_zone("berries", "forest"));

        for _ in 0..10 {
            sim1.tick();
            sim2.tick();
        }

        assert_eq!(sim1.fingerprint(), sim2.fingerprint());
    }

    #[test]
    fn test_simulator_queries() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        sim.register_resource_zone(make_zone("berries", "forest"));
        sim.register_resource_zone(make_zone("mushrooms", "forest"));
        sim.register_resource_zone(make_zone("cacti", "desert"));

        let forest_zones: Vec<_> = sim.zones_in_region("forest").collect();
        assert_eq!(forest_zones.len(), 2);

        let desert_zones: Vec<_> = sim.zones_in_region("desert").collect();
        assert_eq!(desert_zones.len(), 1);
    }

    #[test]
    fn test_serde_resource_zone() {
        let zone = make_zone("berries", "forest");
        let json = serde_json::to_string(&zone).unwrap();
        let restored: ResourceZone = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "berries");
        assert_eq!(restored.region, "forest");
        assert!((restored.capacity() - 1000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_migration_path() {
        let path = make_path("corridor").with_species_affinity(SpeciesCapId::new("bird"), 1.5);

        let json = serde_json::to_string(&path).unwrap();
        let restored: MigrationPath = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "corridor");
        assert!((restored.affinity_for(&SpeciesCapId::new("bird")) - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_infestation_front() {
        let front = InfestationFront::new(
            InfestationFrontId::new(42),
            "origin",
            InfestationType::Plague,
            100,
        )
        .with_intensity(0.6);

        let json = serde_json::to_string(&front).unwrap();
        let restored: InfestationFront = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.raw(), 42);
        assert!((restored.intensity() - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_ecology_simulator() {
        let mut sim = EcologySimulator::new(EcologyConfig::new());
        sim.register_resource_zone(make_zone("berries", "forest"));
        sim.register_migration_path(make_path("corridor"));
        sim.start_infestation("forest", InfestationType::Swarm);

        for _ in 0..5 {
            sim.tick();
        }

        let json = serde_json::to_string(&sim).unwrap();
        let restored: EcologySimulator = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), 5);
        assert!(
            restored
                .get_resource_zone(&ResourceZoneId::new("berries"))
                .is_some()
        );
        assert!(
            restored
                .get_migration_path(&MigrationPathId::new("corridor"))
                .is_some()
        );
        assert_eq!(restored.fingerprint(), sim.fingerprint());
    }

    #[test]
    fn test_serde_food_chain() {
        let mut chain = make_chain("forest");
        chain.add_link(TrophicLink::new(
            SpeciesCapId::new("wolf"),
            SpeciesCapId::new("deer"),
            TrophicRelation::Predator,
        ));

        let json = serde_json::to_string(&chain).unwrap();
        let restored: FoodChain = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "forest");
        assert_eq!(restored.links().len(), 1);
        assert_eq!(restored.species().len(), 2);
    }
}

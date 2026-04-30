//! Territory pressure model with sources, decay, contested fronts, expansion candidates, and nest
//! expansion systems.
//!
//! Provides deterministic, data-driven simulation of territorial dynamics:
//!
//! - Pressure sources from population, resources, threats, and faction activity
//! - Pressure decay over time with configurable rates
//! - Contested front tracking and resolution
//! - Expansion candidate identification and scoring
//! - Nest lifecycle management with health, capacity, and stage progression
//! - Nest expansion decisions based on territory pressure and nest state
//! - Unloaded-region projections and snapshots
//! - Stable fingerprints for determinism verification

use crate::faction::{FactionId, RegionId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Unique identifier for a pressure source.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PressureSourceId(pub String);

impl PressureSourceId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for PressureSourceId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for PressureSourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a contested front.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContestedFrontId(pub u64);

impl ContestedFrontId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ContestedFrontId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "front:{}", self.0)
    }
}

/// Kind of pressure source.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PressureKind {
    /// Pressure from population density.
    #[default]
    Population,
    /// Pressure from resource scarcity.
    ResourceScarcity,
    /// Pressure from resource abundance.
    ResourceAbundance,
    /// Pressure from external threats.
    Threat,
    /// Pressure from faction military activity.
    Military,
    /// Pressure from faction expansion goals.
    Expansion,
    /// Pressure from environmental hazards.
    Environmental,
    /// Pressure from infestation spread.
    Infestation,
}

impl PressureKind {
    #[must_use]
    #[expect(clippy::match_same_arms, reason = "distinct kinds, same values")]
    pub fn base_decay_rate(self) -> f32 {
        match self {
            Self::Population => 0.01,
            Self::ResourceScarcity => 0.02,
            Self::ResourceAbundance => 0.03,
            Self::Threat => 0.05,
            Self::Military => 0.04,
            Self::Expansion => 0.02,
            Self::Environmental => 0.01,
            Self::Infestation => 0.03,
        }
    }

    #[must_use]
    #[expect(clippy::match_same_arms, reason = "distinct kinds, same values")]
    pub fn propagation_factor(self) -> f32 {
        match self {
            Self::Population => 0.3,
            Self::ResourceScarcity => 0.5,
            Self::ResourceAbundance => 0.2,
            Self::Threat => 0.7,
            Self::Military => 0.8,
            Self::Expansion => 0.4,
            Self::Environmental => 0.2,
            Self::Infestation => 0.6,
        }
    }
}

/// A source of territorial pressure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureSource {
    pub id: PressureSourceId,
    pub region: RegionId,
    pub faction: Option<FactionId>,
    pub kind: PressureKind,
    magnitude: f32,
    decay_rate: f32,
    created_tick: u64,
    last_refresh_tick: u64,
    propagates: bool,
}

impl PressureSource {
    #[must_use]
    pub fn new(
        id: PressureSourceId,
        region: RegionId,
        kind: PressureKind,
        magnitude: f32,
        tick: u64,
    ) -> Self {
        Self {
            id,
            region,
            faction: None,
            kind,
            magnitude: magnitude.clamp(0.0, 10.0),
            decay_rate: kind.base_decay_rate(),
            created_tick: tick,
            last_refresh_tick: tick,
            propagates: true,
        }
    }

    #[must_use]
    pub fn with_faction(mut self, faction: FactionId) -> Self {
        self.faction = Some(faction);
        self
    }

    #[must_use]
    pub fn with_decay_rate(mut self, rate: f32) -> Self {
        self.decay_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_propagation(mut self, propagates: bool) -> Self {
        self.propagates = propagates;
        self
    }

    #[must_use]
    pub fn magnitude(&self) -> f32 {
        self.magnitude
    }

    #[must_use]
    pub fn decay_rate(&self) -> f32 {
        self.decay_rate
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick)
    }

    #[must_use]
    pub fn staleness(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_refresh_tick)
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.magnitude < 0.01
    }

    #[must_use]
    pub fn propagates(&self) -> bool {
        self.propagates
    }

    pub fn refresh(&mut self, magnitude: f32, tick: u64) {
        self.magnitude = magnitude.clamp(0.0, 10.0);
        self.last_refresh_tick = tick;
    }

    #[expect(clippy::cast_precision_loss, reason = "tick difference bounded")]
    pub fn decay(&mut self, elapsed_ticks: u64) {
        let decay = self.decay_rate * elapsed_ticks as f32;
        self.magnitude = (self.magnitude - decay).max(0.0);
    }

    #[must_use]
    pub fn propagated_magnitude(&self) -> f32 {
        self.magnitude * self.kind.propagation_factor()
    }
}

/// State of a contested front between factions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrontState {
    #[default]
    Forming,
    Active,
    Stalemate,
    Advancing,
    Retreating,
    Resolved,
}

impl FrontState {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Forming | Self::Active | Self::Advancing | Self::Retreating
        )
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Resolved)
    }
}

/// A contested front between two factions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContestedFront {
    pub id: ContestedFrontId,
    pub faction_a: FactionId,
    pub faction_b: FactionId,
    regions: BTreeSet<RegionId>,
    state: FrontState,
    pressure_a: f32,
    pressure_b: f32,
    created_tick: u64,
    last_update_tick: u64,
    resolution_threshold: f32,
}

impl ContestedFront {
    #[must_use]
    pub fn new(
        id: ContestedFrontId,
        faction_a: FactionId,
        faction_b: FactionId,
        initial_region: RegionId,
        tick: u64,
    ) -> Self {
        let mut regions = BTreeSet::new();
        regions.insert(initial_region);

        Self {
            id,
            faction_a,
            faction_b,
            regions,
            state: FrontState::Forming,
            pressure_a: 0.5,
            pressure_b: 0.5,
            created_tick: tick,
            last_update_tick: tick,
            resolution_threshold: 0.8,
        }
    }

    #[must_use]
    pub fn with_resolution_threshold(mut self, threshold: f32) -> Self {
        self.resolution_threshold = threshold.clamp(0.6, 0.99);
        self
    }

    #[must_use]
    pub fn state(&self) -> FrontState {
        self.state
    }

    pub fn regions(&self) -> impl Iterator<Item = &RegionId> {
        self.regions.iter()
    }

    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    #[must_use]
    pub fn includes_region(&self, region: &RegionId) -> bool {
        self.regions.contains(region)
    }

    #[must_use]
    pub fn pressure_a(&self) -> f32 {
        self.pressure_a
    }

    #[must_use]
    pub fn pressure_b(&self) -> f32 {
        self.pressure_b
    }

    #[must_use]
    pub fn balance(&self) -> f32 {
        let total = self.pressure_a + self.pressure_b;
        if total < 0.01 {
            0.5
        } else {
            self.pressure_a / total
        }
    }

    #[must_use]
    pub fn dominant_faction(&self) -> Option<&FactionId> {
        let balance = self.balance();
        if balance > 0.6 {
            Some(&self.faction_a)
        } else if balance < 0.4 {
            Some(&self.faction_b)
        } else {
            None
        }
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick)
    }

    pub fn add_region(&mut self, region: RegionId) {
        self.regions.insert(region);
    }

    pub fn remove_region(&mut self, region: &RegionId) -> bool {
        self.regions.remove(region)
    }

    pub fn update_pressure(&mut self, faction: &FactionId, pressure: f32, tick: u64) {
        if faction == &self.faction_a {
            self.pressure_a = pressure.clamp(0.0, 10.0);
        } else if faction == &self.faction_b {
            self.pressure_b = pressure.clamp(0.0, 10.0);
        }
        self.last_update_tick = tick;
        self.update_state();
    }

    fn update_state(&mut self) {
        let balance = self.balance();

        if balance > self.resolution_threshold || balance < 1.0 - self.resolution_threshold {
            self.state = FrontState::Resolved;
        } else if balance > 0.6 {
            self.state = FrontState::Advancing;
        } else if balance < 0.4 {
            self.state = FrontState::Retreating;
        } else if (balance - 0.5).abs() < 0.05 {
            self.state = FrontState::Stalemate;
        } else {
            self.state = FrontState::Active;
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "tick difference bounded")]
    pub fn decay(&mut self, elapsed_ticks: u64) {
        let decay = 0.01 * elapsed_ticks as f32;
        self.pressure_a = (self.pressure_a - decay).max(0.0);
        self.pressure_b = (self.pressure_b - decay).max(0.0);
        self.update_state();
    }
}

/// Score for an expansion candidate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExpansionCandidate {
    pub region: RegionId,
    pub faction: FactionId,
    pub score: f32,
    pub distance_from_core: u32,
    pub resource_value: f32,
    pub threat_level: f32,
    pub contestation: f32,
    pub computed_tick: u64,
}

impl ExpansionCandidate {
    #[must_use]
    pub fn new(region: RegionId, faction: FactionId, tick: u64) -> Self {
        Self {
            region,
            faction,
            score: 0.0,
            distance_from_core: 0,
            resource_value: 0.0,
            threat_level: 0.0,
            contestation: 0.0,
            computed_tick: tick,
        }
    }

    #[must_use]
    pub fn with_distance(mut self, distance: u32) -> Self {
        self.distance_from_core = distance;
        self
    }

    #[must_use]
    pub fn with_resource_value(mut self, value: f32) -> Self {
        self.resource_value = value.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_threat_level(mut self, threat: f32) -> Self {
        self.threat_level = threat.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_contestation(mut self, contestation: f32) -> Self {
        self.contestation = contestation.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "distance bounded")]
    pub fn compute_score(&self) -> f32 {
        let distance_penalty = (self.distance_from_core as f32 * 0.1).min(0.5);
        let threat_penalty = self.threat_level * 0.3;
        let contestation_penalty = self.contestation * 0.2;
        let resource_bonus = self.resource_value * 0.4;

        (1.0 + resource_bonus - distance_penalty - threat_penalty - contestation_penalty)
            .clamp(0.0, 2.0)
    }

    pub fn finalize(&mut self) {
        self.score = self.compute_score();
    }

    #[must_use]
    pub fn is_viable(&self) -> bool {
        self.score > 0.3 && self.contestation < 0.8
    }
}

impl Eq for ExpansionCandidate {}

impl PartialOrd for ExpansionCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpansionCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.region.cmp(&other.region))
    }
}

/// Configuration for pressure simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureConfig {
    pub decay_tick_interval: u64,
    pub propagation_enabled: bool,
    pub max_sources_per_region: usize,
    pub front_formation_threshold: f32,
    pub stalemate_duration_threshold: u64,
    pub expansion_score_threshold: f32,
}

impl Default for PressureConfig {
    fn default() -> Self {
        Self {
            decay_tick_interval: 10,
            propagation_enabled: true,
            max_sources_per_region: 20,
            front_formation_threshold: 0.3,
            stalemate_duration_threshold: 100,
            expansion_score_threshold: 0.4,
        }
    }
}

impl PressureConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_decay_interval(mut self, interval: u64) -> Self {
        self.decay_tick_interval = interval.max(1);
        self
    }

    #[must_use]
    pub fn with_propagation(mut self, enabled: bool) -> Self {
        self.propagation_enabled = enabled;
        self
    }

    #[must_use]
    pub fn with_max_sources(mut self, max: usize) -> Self {
        self.max_sources_per_region = max.max(1);
        self
    }
}

/// Event generated by pressure simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PressureEventKind {
    PressureApplied {
        source_id: PressureSourceId,
        region: RegionId,
        magnitude: f32,
    },
    PressureDecayed {
        source_id: PressureSourceId,
        region: RegionId,
        remaining: f32,
    },
    PressureExpired {
        source_id: PressureSourceId,
        region: RegionId,
    },
    PressurePropagated {
        source_id: PressureSourceId,
        from_region: RegionId,
        to_region: RegionId,
        magnitude: f32,
    },
    FrontFormed {
        front_id: ContestedFrontId,
        faction_a: FactionId,
        faction_b: FactionId,
        region: RegionId,
    },
    FrontExpanded {
        front_id: ContestedFrontId,
        new_region: RegionId,
    },
    FrontResolved {
        front_id: ContestedFrontId,
        winner: Option<FactionId>,
    },
    ExpansionCandidateIdentified {
        region: RegionId,
        faction: FactionId,
        score: f32,
    },
}

/// A pressure event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureEvent {
    pub tick: u64,
    pub kind: PressureEventKind,
}

impl PressureEvent {
    #[must_use]
    pub fn new(tick: u64, kind: PressureEventKind) -> Self {
        Self { tick, kind }
    }
}

/// Snapshot of pressure state for a region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionPressureSnapshot {
    pub region: RegionId,
    pub tick: u64,
    pub total_pressure: f32,
    pub pressure_by_kind: BTreeMap<PressureKind, f32>,
    pub pressure_by_faction: BTreeMap<FactionId, f32>,
    pub source_count: u32,
    pub contested: bool,
    pub dominant_faction: Option<FactionId>,
}

impl RegionPressureSnapshot {
    #[must_use]
    pub fn new(region: RegionId, tick: u64) -> Self {
        Self {
            region,
            tick,
            total_pressure: 0.0,
            pressure_by_kind: BTreeMap::new(),
            pressure_by_faction: BTreeMap::new(),
            source_count: 0,
            contested: false,
            dominant_faction: None,
        }
    }

    #[must_use]
    pub fn pressure_balance(&self) -> f32 {
        if self.pressure_by_faction.len() <= 1 {
            return 1.0;
        }

        let values: Vec<f32> = self.pressure_by_faction.values().copied().collect();
        let max = values.iter().copied().fold(0.0f32, f32::max);
        let total: f32 = values.iter().sum();

        if total < 0.01 { 1.0 } else { max / total }
    }
}

/// Summary for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PressureSummary {
    pub tick: u64,
    pub total_regions: u32,
    pub contested_regions: u32,
    pub active_fronts: u32,
    pub total_pressure: f32,
    pub dominant_pressure_kind: Option<PressureKind>,
}

impl From<&TerritoryPressureTracker> for PressureSummary {
    fn from(tracker: &TerritoryPressureTracker) -> Self {
        let mut pressure_by_kind: BTreeMap<PressureKind, f32> = BTreeMap::new();
        for source in tracker.sources.values() {
            *pressure_by_kind.entry(source.kind).or_insert(0.0) += source.magnitude();
        }

        let dominant_pressure_kind = pressure_by_kind
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| *k);

        Self {
            tick: tracker.current_tick,
            total_regions: u32::try_from(tracker.region_pressure.len()).unwrap_or(u32::MAX),
            contested_regions: u32::try_from(
                tracker
                    .fronts
                    .values()
                    .filter(|f| f.state().is_active())
                    .flat_map(ContestedFront::regions)
                    .collect::<BTreeSet<_>>()
                    .len(),
            )
            .unwrap_or(u32::MAX),
            active_fronts: u32::try_from(
                tracker
                    .fronts
                    .values()
                    .filter(|f| f.state().is_active())
                    .count(),
            )
            .unwrap_or(u32::MAX),
            total_pressure: tracker
                .sources
                .values()
                .map(PressureSource::magnitude)
                .sum(),
            dominant_pressure_kind,
        }
    }
}

/// Fingerprint for pressure state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PressureFingerprint(pub u32);

impl PressureFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for PressureFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pressure:{:08x}", self.0)
    }
}

/// Result of a pressure simulation tick.
#[derive(Clone, Debug, Default)]
pub struct PressureTickResult {
    pub events: Vec<PressureEvent>,
    pub sources_decayed: u32,
    pub sources_expired: u32,
    pub fronts_updated: u32,
    pub propagations: u32,
}

/// Projection of pressure state for unloaded regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureProjection {
    pub region: RegionId,
    pub projected_tick: u64,
    pub projected_pressure: f32,
    pub confidence: f32,
    pub trend: PressureTrend,
}

impl PressureProjection {
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "ticks_ahead bounded in practice"
    )]
    pub fn new(
        region: RegionId,
        current_tick: u64,
        ticks_ahead: u64,
        current_pressure: f32,
    ) -> Self {
        let projected_tick = current_tick + ticks_ahead;
        let decay_estimate = 0.01 * ticks_ahead as f32;
        let projected_pressure = (current_pressure - decay_estimate).max(0.0);
        let confidence = (1.0 - ticks_ahead as f32 / 1000.0).clamp(0.1, 1.0);

        let trend = if decay_estimate > current_pressure * 0.5 {
            PressureTrend::Declining
        } else if decay_estimate < current_pressure * 0.1 {
            PressureTrend::Stable
        } else {
            PressureTrend::Moderate
        };

        Self {
            region,
            projected_tick,
            projected_pressure,
            confidence,
            trend,
        }
    }
}

/// Trend of pressure change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PressureTrend {
    Rising,
    #[default]
    Stable,
    Moderate,
    Declining,
}

/// Unique identifier for a nest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NestId(pub String);

impl NestId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for NestId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for NestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nest:{}", self.0)
    }
}

/// Kind of nest structure.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum NestKind {
    /// Underground tunnel colony.
    #[default]
    Colony,
    /// Above-ground hive structure.
    Hive,
    /// Animal den or lair.
    Den,
    /// Underground burrow system.
    Burrow,
    /// Constructed mound.
    Mound,
    /// Organic growth cluster.
    Cluster,
    /// Aquatic nest.
    Aquatic,
}

impl NestKind {
    #[must_use]
    pub fn base_capacity(self) -> u32 {
        match self {
            Self::Colony => 500,
            Self::Hive => 1000,
            Self::Den => 20,
            Self::Burrow => 50,
            Self::Mound => 200,
            Self::Cluster => 100,
            Self::Aquatic => 150,
        }
    }

    #[must_use]
    pub fn expansion_rate(self) -> f32 {
        match self {
            Self::Hive => 0.03,
            Self::Den => 0.01,
            Self::Burrow => 0.015,
            Self::Mound => 0.025,
            Self::Cluster => 0.04,
            Self::Colony | Self::Aquatic => 0.02,
        }
    }

    #[must_use]
    pub fn decay_rate(self) -> f32 {
        match self {
            Self::Colony => 0.005,
            Self::Hive => 0.008,
            Self::Den => 0.003,
            Self::Burrow => 0.004,
            Self::Mound => 0.006,
            Self::Cluster => 0.01,
            Self::Aquatic => 0.007,
        }
    }
}

/// Stage of nest lifecycle.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum NestStage {
    /// Nest is being founded.
    #[default]
    Founding,
    /// Nest is establishing initial structure.
    Establishing,
    /// Nest is actively growing.
    Growing,
    /// Nest has reached maturity.
    Mature,
    /// Nest is declining.
    Declining,
    /// Nest is undergoing metamorphosis/transformation.
    Metamorphosis,
    /// Nest has collapsed or been abandoned.
    Collapsed,
}

impl NestStage {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Founding
                | Self::Establishing
                | Self::Growing
                | Self::Mature
                | Self::Metamorphosis
        )
    }

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Collapsed)
    }

    #[must_use]
    pub fn can_expand(self) -> bool {
        matches!(self, Self::Growing | Self::Mature)
    }

    #[must_use]
    pub fn health_modifier(self) -> f32 {
        match self {
            Self::Founding => 0.5,
            Self::Establishing => 0.7,
            Self::Growing | Self::Mature => 1.0,
            Self::Declining => 0.6,
            Self::Metamorphosis => 0.4,
            Self::Collapsed => 0.0,
        }
    }
}

/// A nest site with population, health, and expansion state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NestSite {
    pub id: NestId,
    pub region: RegionId,
    pub faction: Option<FactionId>,
    pub kind: NestKind,
    stage: NestStage,
    health: f32,
    capacity: u32,
    population: u32,
    spawn_pressure: f32,
    infestation_pressure: f32,
    maturity: f32,
    created_tick: u64,
    last_update_tick: u64,
}

impl NestSite {
    #[must_use]
    pub fn new(id: NestId, region: RegionId, kind: NestKind, tick: u64) -> Self {
        Self {
            id,
            region,
            faction: None,
            kind,
            stage: NestStage::Founding,
            health: 1.0,
            capacity: kind.base_capacity(),
            population: 0,
            spawn_pressure: 0.0,
            infestation_pressure: 0.0,
            maturity: 0.0,
            created_tick: tick,
            last_update_tick: tick,
        }
    }

    #[must_use]
    pub fn with_faction(mut self, faction: FactionId) -> Self {
        self.faction = Some(faction);
        self
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.capacity = capacity;
        self
    }

    #[must_use]
    pub fn with_population(mut self, population: u32) -> Self {
        self.population = population.min(self.capacity);
        self.update_spawn_pressure();
        self
    }

    #[must_use]
    pub fn stage(&self) -> NestStage {
        self.stage
    }

    #[must_use]
    pub fn health(&self) -> f32 {
        self.health
    }

    #[must_use]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    #[must_use]
    pub fn population(&self) -> u32 {
        self.population
    }

    #[must_use]
    pub fn spawn_pressure(&self) -> f32 {
        self.spawn_pressure
    }

    #[must_use]
    pub fn infestation_pressure(&self) -> f32 {
        self.infestation_pressure
    }

    #[must_use]
    pub fn maturity(&self) -> f32 {
        self.maturity
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick)
    }

    #[must_use]
    pub fn effective_health(&self) -> f32 {
        self.health * self.stage.health_modifier()
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn occupancy(&self) -> f32 {
        if self.capacity == 0 {
            0.0
        } else {
            self.population as f32 / self.capacity as f32
        }
    }

    #[must_use]
    pub fn expansion_potential(&self) -> f32 {
        if !self.stage.can_expand() {
            return 0.0;
        }

        let health_factor = self.effective_health();
        let occupancy_factor = self.occupancy();
        let maturity_factor = self.maturity.min(1.0);

        (health_factor * occupancy_factor * maturity_factor * self.kind.expansion_rate())
            .clamp(0.0, 1.0)
    }

    pub fn set_population(&mut self, population: u32) {
        self.population = population.min(self.capacity);
        self.update_spawn_pressure();
    }

    pub fn add_population(&mut self, amount: u32) {
        self.population = self.population.saturating_add(amount).min(self.capacity);
        self.update_spawn_pressure();
    }

    pub fn remove_population(&mut self, amount: u32) {
        self.population = self.population.saturating_sub(amount);
        self.update_spawn_pressure();
    }

    fn update_spawn_pressure(&mut self) {
        self.spawn_pressure = if self.occupancy() > 0.8 {
            ((self.occupancy() - 0.8) * 5.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn apply_infestation(&mut self, pressure: f32) {
        self.infestation_pressure = (self.infestation_pressure + pressure).clamp(0.0, 1.0);
        self.health = (self.health - pressure * 0.1).clamp(0.0, 1.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).clamp(0.0, 1.0);
    }

    pub fn damage(&mut self, amount: f32) {
        self.health = (self.health - amount).clamp(0.0, 1.0);
    }

    pub fn expand_capacity(&mut self, amount: u32) {
        self.capacity = self.capacity.saturating_add(amount);
    }

    #[expect(clippy::cast_precision_loss, reason = "elapsed ticks bounded")]
    pub fn tick(&mut self, elapsed_ticks: u64, current_tick: u64) -> Option<NestStageTransition> {
        self.last_update_tick = current_tick;

        let growth = self.kind.expansion_rate() * elapsed_ticks as f32;
        self.maturity = (self.maturity + growth).min(2.0);

        let decay = self.kind.decay_rate() * elapsed_ticks as f32;
        if self.population == 0 {
            self.health = (self.health - decay * 2.0).max(0.0);
        }

        self.infestation_pressure = (self.infestation_pressure - decay).max(0.0);

        let old_stage = self.stage;
        self.update_stage();

        if self.stage == old_stage {
            None
        } else {
            Some(NestStageTransition {
                nest_id: self.id.clone(),
                from: old_stage,
                to: self.stage,
                tick: current_tick,
            })
        }
    }

    fn update_stage(&mut self) {
        if self.health <= 0.0 || (self.population == 0 && self.maturity > 0.5) {
            self.stage = NestStage::Collapsed;
            return;
        }

        if self.stage == NestStage::Metamorphosis {
            if self.maturity >= 1.8 {
                self.stage = NestStage::Mature;
            }
            return;
        }

        self.stage = match self.maturity {
            m if m < 0.2 => NestStage::Founding,
            m if m < 0.4 => NestStage::Establishing,
            m if m < 0.8 => NestStage::Growing,
            m if m < 1.5 => NestStage::Mature,
            _ => {
                if self.health < 0.5 {
                    NestStage::Declining
                } else {
                    NestStage::Mature
                }
            }
        };
    }

    pub fn trigger_metamorphosis(&mut self) {
        if self.stage == NestStage::Mature {
            self.stage = NestStage::Metamorphosis;
            self.maturity = 1.5;
        }
    }
}

/// Transition between nest stages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestStageTransition {
    pub nest_id: NestId,
    pub from: NestStage,
    pub to: NestStage,
    pub tick: u64,
}

/// State of nest expansion efforts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NestExpansionState {
    pub nest_id: NestId,
    pub target_regions: Vec<RegionId>,
    pub expansion_progress: f32,
    pub blocked_by_fronts: Vec<ContestedFrontId>,
    pub competitor_nests: Vec<NestId>,
    pub started_tick: u64,
    pub last_progress_tick: u64,
}

impl NestExpansionState {
    #[must_use]
    pub fn new(nest_id: NestId, tick: u64) -> Self {
        Self {
            nest_id,
            target_regions: Vec::new(),
            expansion_progress: 0.0,
            blocked_by_fronts: Vec::new(),
            competitor_nests: Vec::new(),
            started_tick: tick,
            last_progress_tick: tick,
        }
    }

    #[must_use]
    pub fn with_targets(mut self, regions: Vec<RegionId>) -> Self {
        self.target_regions = regions;
        self
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by_fronts.is_empty()
    }

    #[must_use]
    pub fn is_contested(&self) -> bool {
        !self.competitor_nests.is_empty()
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.expansion_progress >= 1.0
    }

    pub fn add_blocker(&mut self, front_id: ContestedFrontId) {
        if !self.blocked_by_fronts.contains(&front_id) {
            self.blocked_by_fronts.push(front_id);
        }
    }

    pub fn remove_blocker(&mut self, front_id: &ContestedFrontId) {
        self.blocked_by_fronts.retain(|f| f != front_id);
    }

    pub fn add_competitor(&mut self, nest_id: NestId) {
        if !self.competitor_nests.contains(&nest_id) {
            self.competitor_nests.push(nest_id);
        }
    }

    pub fn advance(&mut self, amount: f32, tick: u64) {
        if !self.is_blocked() {
            self.expansion_progress = (self.expansion_progress + amount).min(1.0);
            self.last_progress_tick = tick;
        }
    }
}

/// Expansion candidate scored from nest state and territory pressure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NestExpansionCandidate {
    pub nest_id: NestId,
    pub target_region: RegionId,
    pub score: f32,
    pub nest_health: f32,
    pub nest_occupancy: f32,
    pub nest_maturity: f32,
    pub territory_pressure: f32,
    pub contested_front_penalty: f32,
    pub distance: u32,
    pub computed_tick: u64,
}

impl NestExpansionCandidate {
    #[must_use]
    pub fn new(nest_id: NestId, target_region: RegionId, tick: u64) -> Self {
        Self {
            nest_id,
            target_region,
            score: 0.0,
            nest_health: 1.0,
            nest_occupancy: 0.0,
            nest_maturity: 0.0,
            territory_pressure: 0.0,
            contested_front_penalty: 0.0,
            distance: 1,
            computed_tick: tick,
        }
    }

    #[must_use]
    pub fn with_nest_state(mut self, health: f32, occupancy: f32, maturity: f32) -> Self {
        self.nest_health = health.clamp(0.0, 1.0);
        self.nest_occupancy = occupancy.clamp(0.0, 1.0);
        self.nest_maturity = maturity.clamp(0.0, 2.0);
        self
    }

    #[must_use]
    pub fn with_territory_pressure(mut self, pressure: f32) -> Self {
        self.territory_pressure = pressure.clamp(0.0, 10.0);
        self
    }

    #[must_use]
    pub fn with_contested_penalty(mut self, penalty: f32) -> Self {
        self.contested_front_penalty = penalty.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_distance(mut self, distance: u32) -> Self {
        self.distance = distance;
        self
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "distance bounded")]
    pub fn compute_score(&self) -> f32 {
        let nest_factor =
            self.nest_health * self.nest_occupancy.max(0.3) * self.nest_maturity.min(1.0);
        let pressure_bonus = (self.territory_pressure * 0.1).min(0.3);
        let distance_penalty = (self.distance as f32 * 0.1).min(0.4);
        let front_penalty = self.contested_front_penalty * 0.5;

        (nest_factor + pressure_bonus - distance_penalty - front_penalty).clamp(0.0, 2.0)
    }

    pub fn finalize(&mut self) {
        self.score = self.compute_score();
    }

    #[must_use]
    pub fn is_viable(&self) -> bool {
        self.score > 0.2 && self.contested_front_penalty < 0.8
    }
}

impl Eq for NestExpansionCandidate {}

impl PartialOrd for NestExpansionCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NestExpansionCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.nest_id.cmp(&other.nest_id))
            .then_with(|| self.target_region.cmp(&other.target_region))
    }
}

/// Event kinds for nest expansion systems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NestEventKind {
    NestFounded {
        nest_id: NestId,
        region: RegionId,
        kind: NestKind,
    },
    StageChanged {
        nest_id: NestId,
        from: NestStage,
        to: NestStage,
    },
    PopulationChanged {
        nest_id: NestId,
        old_population: u32,
        new_population: u32,
    },
    HealthChanged {
        nest_id: NestId,
        old_health: f32,
        new_health: f32,
    },
    ExpansionStarted {
        nest_id: NestId,
        target_regions: Vec<RegionId>,
    },
    ExpansionBlocked {
        nest_id: NestId,
        front_id: ContestedFrontId,
    },
    ExpansionCompleted {
        nest_id: NestId,
        region: RegionId,
    },
    ExpansionFailed {
        nest_id: NestId,
        reason: ExpansionFailureReason,
    },
    NestCollapsed {
        nest_id: NestId,
        reason: CollapseReason,
    },
    InfestationSpread {
        from_nest: NestId,
        to_region: RegionId,
        pressure: f32,
    },
}

/// Reasons for expansion failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpansionFailureReason {
    InsufficientHealth,
    InsufficientPopulation,
    BlockedByFront,
    CompetitorPresent,
    RegionUnavailable,
}

/// Reasons for nest collapse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollapseReason {
    HealthDepleted,
    Abandoned,
    Overrun,
    EnvironmentalHazard,
}

/// A nest event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NestEvent {
    pub tick: u64,
    pub kind: NestEventKind,
}

impl NestEvent {
    #[must_use]
    pub fn new(tick: u64, kind: NestEventKind) -> Self {
        Self { tick, kind }
    }
}

/// Snapshot of nest state for unloaded regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NestSnapshot {
    pub nest_id: NestId,
    pub region: RegionId,
    pub kind: NestKind,
    pub stage: NestStage,
    pub health: f32,
    pub capacity: u32,
    pub population: u32,
    pub maturity: f32,
    pub spawn_pressure: f32,
    pub infestation_pressure: f32,
    pub snapshot_tick: u64,
}

impl From<&NestSite> for NestSnapshot {
    fn from(nest: &NestSite) -> Self {
        Self {
            nest_id: nest.id.clone(),
            region: nest.region.clone(),
            kind: nest.kind,
            stage: nest.stage,
            health: nest.health,
            capacity: nest.capacity,
            population: nest.population,
            maturity: nest.maturity,
            spawn_pressure: nest.spawn_pressure,
            infestation_pressure: nest.infestation_pressure,
            snapshot_tick: nest.last_update_tick,
        }
    }
}

impl NestSnapshot {
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "ticks bounded")]
    pub fn project(&self, ticks_ahead: u64, kind: NestKind) -> NestProjection {
        let decay = kind.decay_rate() * ticks_ahead as f32;
        let growth = kind.expansion_rate() * ticks_ahead as f32;

        let projected_health = (self.health - decay * 0.5).clamp(0.0, 1.0);
        let projected_maturity = (self.maturity + growth).min(2.0);

        let projected_stage = if projected_health <= 0.0 {
            NestStage::Collapsed
        } else {
            match projected_maturity {
                m if m < 0.2 => NestStage::Founding,
                m if m < 0.4 => NestStage::Establishing,
                m if m < 0.8 => NestStage::Growing,
                _ => {
                    if projected_health < 0.5 {
                        NestStage::Declining
                    } else {
                        NestStage::Mature
                    }
                }
            }
        };

        let confidence = (1.0 - ticks_ahead as f32 / 500.0).clamp(0.1, 1.0);

        NestProjection {
            nest_id: self.nest_id.clone(),
            projected_tick: self.snapshot_tick + ticks_ahead,
            projected_stage,
            projected_health,
            projected_maturity,
            projected_population: self.population,
            confidence,
        }
    }
}

/// Projection of nest state for unloaded regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NestProjection {
    pub nest_id: NestId,
    pub projected_tick: u64,
    pub projected_stage: NestStage,
    pub projected_health: f32,
    pub projected_maturity: f32,
    pub projected_population: u32,
    pub confidence: f32,
}

impl NestProjection {
    #[must_use]
    pub fn expansion_potential(&self) -> f32 {
        if !self.projected_stage.can_expand() {
            return 0.0;
        }

        let health_factor = self.projected_health * self.projected_stage.health_modifier();
        (health_factor * self.projected_maturity.min(1.0) * self.confidence).clamp(0.0, 1.0)
    }
}

/// Fingerprint for nest state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NestFingerprint(pub u32);

impl NestFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for NestFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nest:{:08x}", self.0)
    }
}

/// Summary of nest expansion state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NestExpansionSummary {
    pub tick: u64,
    pub total_nests: u32,
    pub active_nests: u32,
    pub expanding_nests: u32,
    pub collapsed_nests: u32,
    pub total_population: u64,
    pub total_capacity: u64,
    pub average_health: f32,
    pub active_expansions: u32,
    pub blocked_expansions: u32,
}

/// Result of a nest expansion tick.
#[derive(Clone, Debug, Default)]
pub struct NestTickResult {
    pub events: Vec<NestEvent>,
    pub nests_updated: u32,
    pub stage_transitions: u32,
    pub expansions_started: u32,
    pub expansions_completed: u32,
    pub expansions_blocked: u32,
    pub nests_collapsed: u32,
}

/// Tracker for nest expansion systems.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NestExpansionTracker {
    nests: BTreeMap<NestId, NestSite>,
    expansions: BTreeMap<NestId, NestExpansionState>,
    nest_regions: BTreeMap<RegionId, Vec<NestId>>,
    current_tick: u64,
    last_decay_tick: u64,
    decay_interval: u64,
}

impl NestExpansionTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nests: BTreeMap::new(),
            expansions: BTreeMap::new(),
            nest_regions: BTreeMap::new(),
            current_tick: 0,
            last_decay_tick: 0,
            decay_interval: 10,
        }
    }

    #[must_use]
    pub fn with_decay_interval(mut self, interval: u64) -> Self {
        self.decay_interval = interval.max(1);
        self
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn add_nest(&mut self, nest: NestSite) {
        let region = nest.region.clone();
        let id = nest.id.clone();
        self.nests.insert(id.clone(), nest);
        self.nest_regions.entry(region).or_default().push(id);
    }

    pub fn remove_nest(&mut self, id: &NestId) -> Option<NestSite> {
        if let Some(nest) = self.nests.remove(id) {
            if let Some(region_nests) = self.nest_regions.get_mut(&nest.region) {
                region_nests.retain(|n| n != id);
            }
            self.expansions.remove(id);
            Some(nest)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_nest(&self, id: &NestId) -> Option<&NestSite> {
        self.nests.get(id)
    }

    pub fn get_nest_mut(&mut self, id: &NestId) -> Option<&mut NestSite> {
        self.nests.get_mut(id)
    }

    #[must_use]
    pub fn get_expansion(&self, id: &NestId) -> Option<&NestExpansionState> {
        self.expansions.get(id)
    }

    pub fn nests_in_region(&self, region: &RegionId) -> impl Iterator<Item = &NestSite> {
        self.nest_regions
            .get(region)
            .into_iter()
            .flat_map(|ids| ids.iter().filter_map(|id| self.nests.get(id)))
    }

    pub fn active_nests(&self) -> impl Iterator<Item = &NestSite> {
        self.nests.values().filter(|n| n.stage().is_active())
    }

    pub fn expanding_nests(&self) -> impl Iterator<Item = (&NestSite, &NestExpansionState)> {
        self.expansions
            .iter()
            .filter_map(|(id, state)| self.nests.get(id).map(|nest| (nest, state)))
    }

    pub fn start_expansion(
        &mut self,
        nest_id: &NestId,
        target_regions: Vec<RegionId>,
    ) -> Option<&NestExpansionState> {
        let nest = self.nests.get(nest_id)?;
        if !nest.stage().can_expand() {
            return None;
        }

        let state = NestExpansionState::new(nest_id.clone(), self.current_tick)
            .with_targets(target_regions);
        self.expansions.insert(nest_id.clone(), state);
        self.expansions.get(nest_id)
    }

    pub fn cancel_expansion(&mut self, nest_id: &NestId) -> Option<NestExpansionState> {
        self.expansions.remove(nest_id)
    }

    pub fn tick(&mut self) -> NestTickResult {
        self.current_tick += 1;
        let mut result = NestTickResult::default();

        if self.current_tick - self.last_decay_tick >= self.decay_interval {
            let elapsed = self.current_tick - self.last_decay_tick;
            self.last_decay_tick = self.current_tick;

            let nest_ids: Vec<NestId> = self.nests.keys().cloned().collect();
            for id in nest_ids {
                if let Some(nest) = self.nests.get_mut(&id) {
                    if let Some(transition) = nest.tick(elapsed, self.current_tick) {
                        result.events.push(NestEvent::new(
                            self.current_tick,
                            NestEventKind::StageChanged {
                                nest_id: transition.nest_id,
                                from: transition.from,
                                to: transition.to,
                            },
                        ));
                        result.stage_transitions += 1;

                        if transition.to == NestStage::Collapsed {
                            result.nests_collapsed += 1;
                        }
                    }
                    result.nests_updated += 1;
                }
            }

            let expansion_ids: Vec<NestId> = self.expansions.keys().cloned().collect();
            for id in expansion_ids {
                if let Some(nest) = self.nests.get(&id) {
                    let progress = nest.expansion_potential() * 0.1;
                    if let Some(expansion) = self.expansions.get_mut(&id) {
                        let was_complete = expansion.is_complete();
                        expansion.advance(progress, self.current_tick);

                        if !was_complete && expansion.is_complete() {
                            result.expansions_completed += 1;
                        }
                    }
                }
            }

            self.nests.retain(|_, n| !n.stage().is_terminal());
            let remaining_nests: BTreeSet<NestId> = self.nests.keys().cloned().collect();
            self.expansions.retain(|id, _| remaining_nests.contains(id));
        }

        result
    }

    pub fn identify_expansion_candidates(
        &self,
        nest_id: &NestId,
        adjacent_regions: &[RegionId],
        pressure_tracker: &TerritoryPressureTracker,
    ) -> Vec<NestExpansionCandidate> {
        let nest = match self.nests.get(nest_id) {
            Some(n) if n.stage().can_expand() => n,
            _ => return Vec::new(),
        };

        let mut candidates = Vec::new();

        for region in adjacent_regions {
            let mut candidate =
                NestExpansionCandidate::new(nest_id.clone(), region.clone(), self.current_tick)
                    .with_nest_state(nest.effective_health(), nest.occupancy(), nest.maturity())
                    .with_territory_pressure(pressure_tracker.region_pressure(region))
                    .with_distance(1);

            if pressure_tracker.is_contested(region) {
                candidate = candidate.with_contested_penalty(0.5);
            }

            for other_nest in self.nests_in_region(region) {
                if other_nest.faction != nest.faction && other_nest.stage().is_active() {
                    let new_penalty = (candidate.contested_front_penalty + 0.3).min(1.0);
                    candidate = candidate.with_contested_penalty(new_penalty);
                }
            }

            candidate.finalize();

            if candidate.is_viable() {
                candidates.push(candidate);
            }
        }

        candidates.sort();
        candidates
    }

    #[must_use]
    pub fn snapshot_nest(&self, nest_id: &NestId) -> Option<NestSnapshot> {
        self.nests.get(nest_id).map(NestSnapshot::from)
    }

    pub fn snapshots(&self) -> impl Iterator<Item = NestSnapshot> + '_ {
        self.nests.values().map(NestSnapshot::from)
    }

    #[must_use]
    pub fn project_nest(&self, nest_id: &NestId, ticks_ahead: u64) -> Option<NestProjection> {
        let nest = self.nests.get(nest_id)?;
        Some(NestSnapshot::from(nest).project(ticks_ahead, nest.kind))
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts bounded by collection size"
    )]
    pub fn summary(&self) -> NestExpansionSummary {
        let active: Vec<_> = self
            .nests
            .values()
            .filter(|n| n.stage().is_active())
            .collect();
        let expanding: Vec<_> = active.iter().filter(|n| n.stage().can_expand()).collect();
        let collapsed = self
            .nests
            .values()
            .filter(|n| n.stage().is_terminal())
            .count();

        let total_population: u64 = self.nests.values().map(|n| u64::from(n.population())).sum();
        let total_capacity: u64 = self.nests.values().map(|n| u64::from(n.capacity())).sum();

        let health_sum: f32 = self.nests.values().map(NestSite::health).sum();
        let average_health = if self.nests.is_empty() {
            0.0
        } else {
            health_sum / self.nests.len() as f32
        };

        let blocked = self.expansions.values().filter(|e| e.is_blocked()).count();

        NestExpansionSummary {
            tick: self.current_tick,
            total_nests: u32::try_from(self.nests.len()).unwrap_or(u32::MAX),
            active_nests: u32::try_from(active.len()).unwrap_or(u32::MAX),
            expanding_nests: u32::try_from(expanding.len()).unwrap_or(u32::MAX),
            collapsed_nests: u32::try_from(collapsed).unwrap_or(u32::MAX),
            total_population,
            total_capacity,
            average_health,
            active_expansions: u32::try_from(self.expansions.len()).unwrap_or(u32::MAX),
            blocked_expansions: u32::try_from(blocked).unwrap_or(u32::MAX),
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> NestFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        hasher.update(&self.current_tick.to_le_bytes());

        for (id, nest) in &self.nests {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&[nest.stage() as u8]);
            hasher.update(&nest.health().to_le_bytes());
            hasher.update(&nest.population().to_le_bytes());
            hasher.update(&nest.maturity().to_le_bytes());
        }

        for (id, expansion) in &self.expansions {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&expansion.expansion_progress.to_le_bytes());
        }

        NestFingerprint(hasher.finalize())
    }
}

/// Main tracker for territory pressure.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerritoryPressureTracker {
    config: PressureConfig,
    sources: BTreeMap<PressureSourceId, PressureSource>,
    fronts: BTreeMap<ContestedFrontId, ContestedFront>,
    region_pressure: BTreeMap<RegionId, f32>,
    adjacent_regions: BTreeMap<RegionId, Vec<RegionId>>,
    next_front_id: u64,
    current_tick: u64,
    last_decay_tick: u64,
}

impl TerritoryPressureTracker {
    #[must_use]
    pub fn new(config: PressureConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn config(&self) -> &PressureConfig {
        &self.config
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn register_adjacent(&mut self, region: RegionId, adjacent: Vec<RegionId>) {
        self.adjacent_regions.insert(region, adjacent);
    }

    pub fn add_pressure_source(&mut self, source: PressureSource) {
        let region = source.region.clone();
        self.sources.insert(source.id.clone(), source);
        self.recalculate_region_pressure(&region);
    }

    pub fn remove_pressure_source(&mut self, id: &PressureSourceId) -> Option<PressureSource> {
        if let Some(source) = self.sources.remove(id) {
            self.recalculate_region_pressure(&source.region);
            Some(source)
        } else {
            None
        }
    }

    pub fn refresh_pressure_source(&mut self, id: &PressureSourceId, magnitude: f32) {
        if let Some(source) = self.sources.get_mut(id) {
            source.refresh(magnitude, self.current_tick);
            let region = source.region.clone();
            self.recalculate_region_pressure(&region);
        }
    }

    #[must_use]
    pub fn get_source(&self, id: &PressureSourceId) -> Option<&PressureSource> {
        self.sources.get(id)
    }

    #[must_use]
    pub fn get_front(&self, id: &ContestedFrontId) -> Option<&ContestedFront> {
        self.fronts.get(id)
    }

    pub fn get_front_mut(&mut self, id: &ContestedFrontId) -> Option<&mut ContestedFront> {
        self.fronts.get_mut(id)
    }

    #[must_use]
    pub fn region_pressure(&self, region: &RegionId) -> f32 {
        self.region_pressure.get(region).copied().unwrap_or(0.0)
    }

    pub fn sources_in_region(&self, region: &RegionId) -> impl Iterator<Item = &PressureSource> {
        self.sources.values().filter(move |s| &s.region == region)
    }

    pub fn fronts_in_region(&self, region: &RegionId) -> impl Iterator<Item = &ContestedFront> {
        self.fronts
            .values()
            .filter(move |f| f.includes_region(region))
    }

    pub fn active_fronts(&self) -> impl Iterator<Item = &ContestedFront> {
        self.fronts.values().filter(|f| f.state().is_active())
    }

    #[must_use]
    pub fn is_contested(&self, region: &RegionId) -> bool {
        self.fronts
            .values()
            .any(|f| f.state().is_active() && f.includes_region(region))
    }

    fn recalculate_region_pressure(&mut self, region: &RegionId) {
        let total: f32 = self
            .sources
            .values()
            .filter(|s| &s.region == region)
            .map(PressureSource::magnitude)
            .sum();
        self.region_pressure.insert(region.clone(), total);
    }

    pub fn create_front(
        &mut self,
        faction_a: FactionId,
        faction_b: FactionId,
        region: RegionId,
    ) -> ContestedFrontId {
        let id = ContestedFrontId::new(self.next_front_id);
        self.next_front_id += 1;
        let front = ContestedFront::new(id, faction_a, faction_b, region, self.current_tick);
        self.fronts.insert(id, front);
        id
    }

    pub fn tick(&mut self) -> PressureTickResult {
        self.current_tick += 1;
        let mut result = PressureTickResult::default();

        if self.current_tick - self.last_decay_tick >= self.config.decay_tick_interval {
            let elapsed = self.current_tick - self.last_decay_tick;
            self.last_decay_tick = self.current_tick;

            let source_ids: Vec<PressureSourceId> = self.sources.keys().cloned().collect();
            for id in source_ids {
                if let Some(source) = self.sources.get_mut(&id) {
                    source.decay(elapsed);
                    result.sources_decayed += 1;

                    if source.is_expired() {
                        result.events.push(PressureEvent::new(
                            self.current_tick,
                            PressureEventKind::PressureExpired {
                                source_id: id.clone(),
                                region: source.region.clone(),
                            },
                        ));
                        result.sources_expired += 1;
                    } else {
                        result.events.push(PressureEvent::new(
                            self.current_tick,
                            PressureEventKind::PressureDecayed {
                                source_id: id.clone(),
                                region: source.region.clone(),
                                remaining: source.magnitude(),
                            },
                        ));
                    }
                }
            }

            self.sources.retain(|_, s| !s.is_expired());

            let regions: Vec<RegionId> = self.region_pressure.keys().cloned().collect();
            for region in regions {
                self.recalculate_region_pressure(&region);
            }

            for front in self.fronts.values_mut() {
                front.decay(elapsed);
                result.fronts_updated += 1;
            }

            self.fronts.retain(|_, f| !f.state().is_finished());
        }

        result
    }

    #[must_use]
    pub fn snapshot_region(&self, region: &RegionId) -> RegionPressureSnapshot {
        let mut snapshot = RegionPressureSnapshot::new(region.clone(), self.current_tick);

        for source in self.sources_in_region(region) {
            snapshot.total_pressure += source.magnitude();
            *snapshot.pressure_by_kind.entry(source.kind).or_insert(0.0) += source.magnitude();
            if let Some(faction) = &source.faction {
                *snapshot
                    .pressure_by_faction
                    .entry(faction.clone())
                    .or_insert(0.0) += source.magnitude();
            }
            snapshot.source_count += 1;
        }

        snapshot.contested = self.is_contested(region);
        snapshot.dominant_faction = snapshot
            .pressure_by_faction
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(f, _)| f.clone());

        snapshot
    }

    #[must_use]
    pub fn summary(&self) -> PressureSummary {
        PressureSummary::from(self)
    }

    #[must_use]
    pub fn project(&self, region: &RegionId, ticks_ahead: u64) -> PressureProjection {
        let current_pressure = self.region_pressure(region);
        PressureProjection::new(
            region.clone(),
            self.current_tick,
            ticks_ahead,
            current_pressure,
        )
    }

    #[must_use]
    pub fn identify_expansion_candidates(&self, faction: &FactionId) -> Vec<ExpansionCandidate> {
        let owned_regions: BTreeSet<RegionId> = self
            .sources
            .values()
            .filter(|s| s.faction.as_ref() == Some(faction) && s.magnitude() > 0.5)
            .map(|s| s.region.clone())
            .collect();

        let mut candidates = Vec::new();

        for owned in &owned_regions {
            if let Some(adjacent) = self.adjacent_regions.get(owned) {
                for adj in adjacent {
                    if owned_regions.contains(adj) {
                        continue;
                    }

                    let mut candidate =
                        ExpansionCandidate::new(adj.clone(), faction.clone(), self.current_tick);

                    let distance = 1;
                    candidate = candidate.with_distance(distance);

                    let faction_pressure: f32 = self
                        .sources
                        .values()
                        .filter(|s| &s.region == adj && s.faction.as_ref() == Some(faction))
                        .map(PressureSource::magnitude)
                        .sum();

                    let other_pressure: f32 = self
                        .sources
                        .values()
                        .filter(|s| &s.region == adj && s.faction.as_ref() != Some(faction))
                        .map(PressureSource::magnitude)
                        .sum();

                    let total = faction_pressure + other_pressure;
                    let contestation = if total < 0.01 {
                        0.0
                    } else {
                        other_pressure / total
                    };

                    candidate = candidate.with_contestation(contestation);
                    candidate.finalize();

                    if candidate.is_viable() {
                        candidates.push(candidate);
                    }
                }
            }
        }

        candidates.sort();
        candidates
    }

    #[must_use]
    pub fn fingerprint(&self) -> PressureFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        hasher.update(&self.current_tick.to_le_bytes());

        for (id, source) in &self.sources {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&source.magnitude().to_le_bytes());
            hasher.update(&[source.kind as u8]);
        }

        for (id, front) in &self.fronts {
            hasher.update(&id.raw().to_le_bytes());
            hasher.update(&[front.state() as u8]);
            hasher.update(&front.pressure_a().to_le_bytes());
            hasher.update(&front.pressure_b().to_le_bytes());
        }

        PressureFingerprint(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source(id: &str, region: &str, kind: PressureKind, magnitude: f32) -> PressureSource {
        PressureSource::new(
            PressureSourceId::new(id),
            RegionId::new(region),
            kind,
            magnitude,
            0,
        )
    }

    #[test]
    fn test_pressure_source_id() {
        let id = PressureSourceId::new("pop_forest");
        assert_eq!(id.as_str(), "pop_forest");
        assert_eq!(format!("{id}"), "pop_forest");
    }

    #[test]
    fn test_contested_front_id() {
        let id = ContestedFrontId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "front:42");
    }

    #[test]
    fn test_pressure_kind_decay() {
        assert!(PressureKind::Population.base_decay_rate() > 0.0);
        assert!(
            PressureKind::Threat.base_decay_rate() > PressureKind::Population.base_decay_rate()
        );
    }

    #[test]
    fn test_pressure_kind_propagation() {
        assert!(
            PressureKind::Military.propagation_factor()
                > PressureKind::Population.propagation_factor()
        );
    }

    #[test]
    fn test_pressure_source_new() {
        let source = make_source("pop_1", "forest", PressureKind::Population, 5.0);
        assert_eq!(source.id.as_str(), "pop_1");
        assert!((source.magnitude() - 5.0).abs() < f32::EPSILON);
        assert!(!source.is_expired());
    }

    #[test]
    fn test_pressure_source_decay() {
        let mut source = make_source("pop_1", "forest", PressureKind::Population, 1.0);
        source.decay(50);
        assert!(source.magnitude() < 1.0);
    }

    #[test]
    fn test_pressure_source_expiration() {
        let mut source = make_source("pop_1", "forest", PressureKind::Population, 0.1);
        source.decay(100);
        assert!(source.is_expired());
    }

    #[test]
    fn test_pressure_source_refresh() {
        let mut source = make_source("pop_1", "forest", PressureKind::Population, 1.0);
        source.decay(50);
        let after_decay = source.magnitude();

        source.refresh(2.0, 100);
        assert!((source.magnitude() - 2.0).abs() < f32::EPSILON);
        assert!(source.magnitude() > after_decay);
    }

    #[test]
    fn test_pressure_source_with_faction() {
        let source = make_source("mil_1", "forest", PressureKind::Military, 3.0)
            .with_faction(FactionId::new("raiders"));
        assert_eq!(source.faction, Some(FactionId::new("raiders")));
    }

    #[test]
    fn test_front_state_active() {
        assert!(FrontState::Forming.is_active());
        assert!(FrontState::Active.is_active());
        assert!(!FrontState::Resolved.is_active());
        assert!(FrontState::Resolved.is_finished());
    }

    #[test]
    fn test_contested_front_new() {
        let front = ContestedFront::new(
            ContestedFrontId::new(1),
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
            0,
        );

        assert_eq!(front.state(), FrontState::Forming);
        assert!(front.includes_region(&RegionId::new("border")));
        assert_eq!(front.region_count(), 1);
    }

    #[test]
    fn test_contested_front_pressure() {
        let mut front = ContestedFront::new(
            ContestedFrontId::new(1),
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
            0,
        );

        front.update_pressure(&FactionId::new("a"), 3.0, 10);
        front.update_pressure(&FactionId::new("b"), 1.0, 10);

        assert!((front.pressure_a() - 3.0).abs() < f32::EPSILON);
        assert!((front.pressure_b() - 1.0).abs() < f32::EPSILON);
        assert!(front.balance() > 0.5);
        assert_eq!(front.dominant_faction(), Some(&FactionId::new("a")));
    }

    #[test]
    fn test_contested_front_resolution() {
        let mut front = ContestedFront::new(
            ContestedFrontId::new(1),
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
            0,
        );

        front.update_pressure(&FactionId::new("a"), 10.0, 10);
        front.update_pressure(&FactionId::new("b"), 1.0, 10);

        assert_eq!(front.state(), FrontState::Resolved);
    }

    #[test]
    fn test_contested_front_regions() {
        let mut front = ContestedFront::new(
            ContestedFrontId::new(1),
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
            0,
        );

        front.add_region(RegionId::new("expansion"));
        assert_eq!(front.region_count(), 2);

        front.remove_region(&RegionId::new("border"));
        assert_eq!(front.region_count(), 1);
    }

    #[test]
    fn test_expansion_candidate_score() {
        let mut candidate =
            ExpansionCandidate::new(RegionId::new("target"), FactionId::new("faction"), 0)
                .with_distance(2)
                .with_resource_value(0.8)
                .with_threat_level(0.2)
                .with_contestation(0.1);

        candidate.finalize();

        assert!(candidate.score > 0.0);
        assert!(candidate.is_viable());
    }

    #[test]
    fn test_expansion_candidate_ordering() {
        let mut c1 = ExpansionCandidate::new(RegionId::new("a"), FactionId::new("f"), 0)
            .with_resource_value(0.9);
        c1.finalize();

        let mut c2 = ExpansionCandidate::new(RegionId::new("b"), FactionId::new("f"), 0)
            .with_resource_value(0.3);
        c2.finalize();

        assert!(c1 < c2);
    }

    #[test]
    fn test_pressure_config() {
        let config = PressureConfig::new()
            .with_decay_interval(20)
            .with_propagation(false)
            .with_max_sources(50);

        assert_eq!(config.decay_tick_interval, 20);
        assert!(!config.propagation_enabled);
        assert_eq!(config.max_sources_per_region, 50);
    }

    #[test]
    fn test_pressure_event() {
        let event = PressureEvent::new(
            100,
            PressureEventKind::PressureApplied {
                source_id: PressureSourceId::new("test"),
                region: RegionId::new("forest"),
                magnitude: 5.0,
            },
        );
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn test_region_pressure_snapshot() {
        let snapshot = RegionPressureSnapshot::new(RegionId::new("test"), 100);
        assert_eq!(snapshot.region.as_str(), "test");
        assert_eq!(snapshot.tick, 100);
        assert!((snapshot.pressure_balance() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pressure_fingerprint() {
        let fp = PressureFingerprint(0xCAFE_BABE);
        assert_eq!(fp.raw(), 0xCAFE_BABE);
        assert_eq!(format!("{fp}"), "pressure:cafebabe");
    }

    #[test]
    fn test_pressure_projection() {
        let proj = PressureProjection::new(RegionId::new("test"), 100, 50, 5.0);
        assert_eq!(proj.projected_tick, 150);
        assert!(proj.projected_pressure < 5.0);
        assert!(proj.confidence > 0.0);
    }

    #[test]
    fn test_tracker_basic() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());
        assert_eq!(tracker.current_tick(), 0);

        tracker.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));

        assert!(
            tracker
                .get_source(&PressureSourceId::new("pop_1"))
                .is_some()
        );
        assert!((tracker.region_pressure(&RegionId::new("forest")) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tracker_multiple_sources() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        tracker.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));
        tracker.add_pressure_source(make_source("threat_1", "forest", PressureKind::Threat, 3.0));

        assert!((tracker.region_pressure(&RegionId::new("forest")) - 8.0).abs() < f32::EPSILON);

        let sources: Vec<_> = tracker
            .sources_in_region(&RegionId::new("forest"))
            .collect();
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn test_tracker_remove_source() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        tracker.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));
        tracker.remove_pressure_source(&PressureSourceId::new("pop_1"));

        assert!(
            tracker
                .get_source(&PressureSourceId::new("pop_1"))
                .is_none()
        );
        assert!((tracker.region_pressure(&RegionId::new("forest"))).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tracker_tick() {
        let mut tracker =
            TerritoryPressureTracker::new(PressureConfig::new().with_decay_interval(1));

        tracker.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));

        let initial = tracker.region_pressure(&RegionId::new("forest"));

        for _ in 0..20 {
            tracker.tick();
        }

        assert!(tracker.region_pressure(&RegionId::new("forest")) < initial);
    }

    #[test]
    fn test_tracker_front_creation() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        let front_id = tracker.create_front(
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
        );

        assert!(tracker.get_front(&front_id).is_some());
        assert!(tracker.is_contested(&RegionId::new("border")));
    }

    #[test]
    fn test_tracker_snapshot() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        tracker.add_pressure_source(
            make_source("pop_1", "forest", PressureKind::Population, 5.0)
                .with_faction(FactionId::new("settlers")),
        );

        let snapshot = tracker.snapshot_region(&RegionId::new("forest"));
        assert!((snapshot.total_pressure - 5.0).abs() < f32::EPSILON);
        assert_eq!(snapshot.source_count, 1);
    }

    #[test]
    fn test_tracker_fingerprint_determinism() {
        let mut tracker1 = TerritoryPressureTracker::new(PressureConfig::new());
        let mut tracker2 = TerritoryPressureTracker::new(PressureConfig::new());

        tracker1.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));
        tracker2.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));

        for _ in 0..10 {
            tracker1.tick();
            tracker2.tick();
        }

        assert_eq!(tracker1.fingerprint(), tracker2.fingerprint());
    }

    #[test]
    fn test_tracker_expansion_candidates() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        tracker.register_adjacent(
            RegionId::new("home"),
            vec![RegionId::new("target1"), RegionId::new("target2")],
        );

        tracker.add_pressure_source(
            make_source("pop_1", "home", PressureKind::Population, 5.0)
                .with_faction(FactionId::new("settlers")),
        );

        let candidates = tracker.identify_expansion_candidates(&FactionId::new("settlers"));
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_serde_pressure_source() {
        let source = make_source("pop_1", "forest", PressureKind::Population, 5.0)
            .with_faction(FactionId::new("settlers"));

        let json = serde_json::to_string(&source).unwrap();
        let restored: PressureSource = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "pop_1");
        assert!((restored.magnitude() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_contested_front() {
        let mut front = ContestedFront::new(
            ContestedFrontId::new(1),
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
            0,
        );
        front.add_region(RegionId::new("expansion"));

        let json = serde_json::to_string(&front).unwrap();
        let restored: ContestedFront = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.raw(), 1);
        assert_eq!(restored.region_count(), 2);
    }

    #[test]
    fn test_serde_expansion_candidate() {
        let mut candidate =
            ExpansionCandidate::new(RegionId::new("target"), FactionId::new("faction"), 100)
                .with_distance(2)
                .with_resource_value(0.7);
        candidate.finalize();

        let json = serde_json::to_string(&candidate).unwrap();
        let restored: ExpansionCandidate = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region.as_str(), "target");
        assert!(restored.score > 0.0);
    }

    #[test]
    fn test_serde_tracker() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        tracker.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));
        tracker.create_front(
            FactionId::new("a"),
            FactionId::new("b"),
            RegionId::new("border"),
        );

        for _ in 0..5 {
            tracker.tick();
        }

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: TerritoryPressureTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), 5);
        assert!(
            restored
                .get_source(&PressureSourceId::new("pop_1"))
                .is_some()
        );
        assert_eq!(restored.fingerprint(), tracker.fingerprint());
    }

    #[test]
    fn test_serde_pressure_summary() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());
        tracker.add_pressure_source(make_source(
            "pop_1",
            "forest",
            PressureKind::Population,
            5.0,
        ));

        let summary = tracker.summary();
        let json = serde_json::to_string(&summary).unwrap();
        let restored: PressureSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.total_regions, 1);
    }

    #[test]
    fn test_deterministic_ordering() {
        let mut tracker = TerritoryPressureTracker::new(PressureConfig::new());

        tracker.add_pressure_source(make_source(
            "z_source",
            "forest",
            PressureKind::Population,
            1.0,
        ));
        tracker.add_pressure_source(make_source(
            "a_source",
            "forest",
            PressureKind::Population,
            2.0,
        ));
        tracker.add_pressure_source(make_source(
            "m_source",
            "forest",
            PressureKind::Population,
            3.0,
        ));

        let ids: Vec<&str> = tracker
            .sources
            .keys()
            .map(PressureSourceId::as_str)
            .collect();

        assert_eq!(ids, vec!["a_source", "m_source", "z_source"]);
    }

    // Nest expansion tests

    fn make_nest(id: &str, region: &str, kind: NestKind) -> NestSite {
        NestSite::new(NestId::new(id), RegionId::new(region), kind, 0)
    }

    #[test]
    fn test_nest_id() {
        let id = NestId::new("colony_alpha");
        assert_eq!(id.as_str(), "colony_alpha");
        assert_eq!(format!("{id}"), "nest:colony_alpha");
    }

    #[test]
    fn test_nest_kind_properties() {
        assert!(NestKind::Hive.base_capacity() > NestKind::Den.base_capacity());
        assert!(NestKind::Cluster.expansion_rate() > NestKind::Den.expansion_rate());
        assert!(NestKind::Cluster.decay_rate() > NestKind::Den.decay_rate());
    }

    #[test]
    fn test_nest_stage_properties() {
        assert!(NestStage::Founding.is_active());
        assert!(NestStage::Mature.can_expand());
        assert!(!NestStage::Declining.can_expand());
        assert!(NestStage::Collapsed.is_terminal());
        assert!((NestStage::Growing.health_modifier() - 1.0).abs() < f32::EPSILON);
        assert!(NestStage::Declining.health_modifier() < 1.0);
    }

    #[test]
    fn test_nest_site_new() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony);
        assert_eq!(nest.id.as_str(), "colony_1");
        assert_eq!(nest.stage(), NestStage::Founding);
        assert!((nest.health() - 1.0).abs() < f32::EPSILON);
        assert_eq!(nest.capacity(), NestKind::Colony.base_capacity());
    }

    #[test]
    fn test_nest_site_population() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(100);

        assert_eq!(nest.population(), 100);

        nest.add_population(50);
        assert_eq!(nest.population(), 150);

        nest.remove_population(30);
        assert_eq!(nest.population(), 120);
    }

    #[test]
    fn test_nest_site_occupancy() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony)
            .with_capacity(100)
            .with_population(75);

        assert!((nest.occupancy() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_nest_site_spawn_pressure() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony).with_capacity(100);

        nest.set_population(70);
        assert!(nest.spawn_pressure() < f32::EPSILON);

        nest.set_population(90);
        assert!(nest.spawn_pressure() > 0.0);
    }

    #[test]
    fn test_nest_site_health_damage() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony);

        nest.damage(0.3);
        assert!((nest.health() - 0.7).abs() < f32::EPSILON);

        nest.heal(0.2);
        assert!((nest.health() - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_nest_site_infestation() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony);

        nest.apply_infestation(0.5);
        assert!((nest.infestation_pressure() - 0.5).abs() < f32::EPSILON);
        assert!(nest.health() < 1.0);
    }

    #[test]
    fn test_nest_site_lifecycle() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(100);

        assert_eq!(nest.stage(), NestStage::Founding);

        for i in 1..=100 {
            nest.tick(1, i);
        }

        assert!(nest.maturity() > 0.0);
        assert!(nest.stage() != NestStage::Founding);
    }

    #[test]
    fn test_nest_site_collapse() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony);

        nest.set_population(0);
        for i in 1..=500 {
            nest.tick(1, i);
        }

        assert_eq!(nest.stage(), NestStage::Collapsed);
    }

    #[test]
    fn test_nest_site_expansion_potential() {
        let mut nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(400);

        for i in 1..=50 {
            nest.tick(1, i);
        }

        if nest.stage().can_expand() {
            assert!(nest.expansion_potential() > 0.0);
        }
    }

    #[test]
    fn test_nest_stage_transition() {
        let transition = NestStageTransition {
            nest_id: NestId::new("colony_1"),
            from: NestStage::Growing,
            to: NestStage::Mature,
            tick: 100,
        };

        assert_eq!(transition.from, NestStage::Growing);
        assert_eq!(transition.to, NestStage::Mature);
    }

    #[test]
    fn test_nest_expansion_state() {
        let mut state = NestExpansionState::new(NestId::new("colony_1"), 0)
            .with_targets(vec![RegionId::new("forest"), RegionId::new("plains")]);

        assert!(!state.is_blocked());
        assert!(!state.is_complete());

        state.advance(0.5, 10);
        assert!((state.expansion_progress - 0.5).abs() < f32::EPSILON);

        state.add_blocker(ContestedFrontId::new(1));
        assert!(state.is_blocked());

        state.advance(0.5, 20);
        assert!((state.expansion_progress - 0.5).abs() < f32::EPSILON);

        state.remove_blocker(&ContestedFrontId::new(1));
        state.advance(0.5, 30);
        assert!(state.is_complete());
    }

    #[test]
    fn test_nest_expansion_candidate_score() {
        let mut candidate =
            NestExpansionCandidate::new(NestId::new("colony_1"), RegionId::new("target"), 0)
                .with_nest_state(1.0, 0.8, 1.0)
                .with_territory_pressure(2.0)
                .with_distance(1);

        candidate.finalize();

        assert!(candidate.score > 0.0);
        assert!(candidate.is_viable());
    }

    #[test]
    fn test_nest_expansion_candidate_contested_penalty() {
        let mut candidate =
            NestExpansionCandidate::new(NestId::new("colony_1"), RegionId::new("target"), 0)
                .with_nest_state(1.0, 0.8, 1.0)
                .with_contested_penalty(0.9);

        candidate.finalize();

        assert!(!candidate.is_viable());
    }

    #[test]
    fn test_nest_expansion_candidate_ordering() {
        let mut c1 = NestExpansionCandidate::new(NestId::new("a"), RegionId::new("target"), 0)
            .with_nest_state(1.0, 0.9, 1.0);
        c1.finalize();

        let mut c2 = NestExpansionCandidate::new(NestId::new("b"), RegionId::new("target"), 0)
            .with_nest_state(0.5, 0.5, 0.5);
        c2.finalize();

        assert!(c1 < c2);
    }

    #[test]
    fn test_nest_event() {
        let event = NestEvent::new(
            100,
            NestEventKind::NestFounded {
                nest_id: NestId::new("colony_1"),
                region: RegionId::new("forest"),
                kind: NestKind::Colony,
            },
        );
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn test_nest_snapshot() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(200);

        let snapshot = NestSnapshot::from(&nest);
        assert_eq!(snapshot.nest_id.as_str(), "colony_1");
        assert_eq!(snapshot.population, 200);
    }

    #[test]
    fn test_nest_snapshot_projection() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(200);

        let snapshot = NestSnapshot::from(&nest);
        let projection = snapshot.project(100, NestKind::Colony);

        assert_eq!(projection.nest_id.as_str(), "colony_1");
        assert!(projection.confidence > 0.0);
        assert!(projection.projected_maturity > snapshot.maturity);
    }

    #[test]
    fn test_nest_fingerprint() {
        let fp = NestFingerprint(0xDEAD_BEEF);
        assert_eq!(fp.raw(), 0xDEAD_BEEF);
        assert_eq!(format!("{fp}"), "nest:deadbeef");
    }

    #[test]
    fn test_nest_expansion_tracker_basic() {
        let mut tracker = NestExpansionTracker::new();
        assert_eq!(tracker.current_tick(), 0);

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony));

        assert!(tracker.get_nest(&NestId::new("colony_1")).is_some());
    }

    #[test]
    fn test_nest_expansion_tracker_regions() {
        let mut tracker = NestExpansionTracker::new();

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony));
        tracker.add_nest(make_nest("hive_1", "forest", NestKind::Hive));

        let nests: Vec<_> = tracker.nests_in_region(&RegionId::new("forest")).collect();
        assert_eq!(nests.len(), 2);
    }

    #[test]
    fn test_nest_expansion_tracker_remove() {
        let mut tracker = NestExpansionTracker::new();

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony));
        tracker.remove_nest(&NestId::new("colony_1"));

        assert!(tracker.get_nest(&NestId::new("colony_1")).is_none());
    }

    #[test]
    fn test_nest_expansion_tracker_tick() {
        let mut tracker = NestExpansionTracker::new().with_decay_interval(1);

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));

        for _ in 0..20 {
            tracker.tick();
        }

        let nest = tracker.get_nest(&NestId::new("colony_1")).unwrap();
        assert!(nest.maturity() > 0.0);
    }

    #[test]
    fn test_nest_expansion_tracker_start_expansion() {
        let mut tracker = NestExpansionTracker::new().with_decay_interval(1);

        let mut nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(400);

        for i in 1..=60 {
            nest.tick(1, i);
        }

        tracker.add_nest(nest);

        let can_expand = tracker
            .get_nest(&NestId::new("colony_1"))
            .unwrap()
            .stage()
            .can_expand();

        if can_expand {
            let expansion =
                tracker.start_expansion(&NestId::new("colony_1"), vec![RegionId::new("plains")]);
            assert!(expansion.is_some());
        }
    }

    #[test]
    fn test_nest_expansion_tracker_candidates() {
        let mut tracker = NestExpansionTracker::new().with_decay_interval(1);
        let pressure_tracker = TerritoryPressureTracker::new(PressureConfig::new());

        let mut nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(400);

        for i in 1..=60 {
            nest.tick(1, i);
        }

        tracker.add_nest(nest);

        let candidates = tracker.identify_expansion_candidates(
            &NestId::new("colony_1"),
            &[RegionId::new("plains"), RegionId::new("hills")],
            &pressure_tracker,
        );

        if tracker
            .get_nest(&NestId::new("colony_1"))
            .unwrap()
            .stage()
            .can_expand()
        {
            assert!(!candidates.is_empty());
        }
    }

    #[test]
    fn test_nest_expansion_tracker_summary() {
        let mut tracker = NestExpansionTracker::new();

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));
        tracker.add_nest(make_nest("hive_1", "plains", NestKind::Hive).with_population(200));

        let summary = tracker.summary();
        assert_eq!(summary.total_nests, 2);
        assert_eq!(summary.active_nests, 2);
        assert_eq!(summary.total_population, 300);
    }

    #[test]
    fn test_nest_expansion_tracker_fingerprint_determinism() {
        let mut tracker1 = NestExpansionTracker::new().with_decay_interval(1);
        let mut tracker2 = NestExpansionTracker::new().with_decay_interval(1);

        tracker1.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));
        tracker2.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));

        for _ in 0..10 {
            tracker1.tick();
            tracker2.tick();
        }

        assert_eq!(tracker1.fingerprint(), tracker2.fingerprint());
    }

    #[test]
    fn test_nest_expansion_tracker_projection() {
        let mut tracker = NestExpansionTracker::new();

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));

        let projection = tracker.project_nest(&NestId::new("colony_1"), 50);
        assert!(projection.is_some());

        let proj = projection.unwrap();
        assert!(proj.confidence > 0.0);
    }

    #[test]
    fn test_serde_nest_site() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony)
            .with_faction(FactionId::new("settlers"))
            .with_population(100);

        let json = serde_json::to_string(&nest).unwrap();
        let restored: NestSite = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "colony_1");
        assert_eq!(restored.population(), 100);
    }

    #[test]
    fn test_serde_nest_expansion_state() {
        let mut state = NestExpansionState::new(NestId::new("colony_1"), 0)
            .with_targets(vec![RegionId::new("target")]);
        state.advance(0.5, 10);

        let json = serde_json::to_string(&state).unwrap();
        let restored: NestExpansionState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nest_id.as_str(), "colony_1");
        assert!((restored.expansion_progress - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_nest_expansion_candidate() {
        let mut candidate =
            NestExpansionCandidate::new(NestId::new("colony_1"), RegionId::new("target"), 100)
                .with_nest_state(0.9, 0.8, 1.0);
        candidate.finalize();

        let json = serde_json::to_string(&candidate).unwrap();
        let restored: NestExpansionCandidate = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nest_id.as_str(), "colony_1");
        assert!(restored.score > 0.0);
    }

    #[test]
    fn test_serde_nest_snapshot() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(150);
        let snapshot = NestSnapshot::from(&nest);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: NestSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nest_id.as_str(), "colony_1");
        assert_eq!(restored.population, 150);
    }

    #[test]
    fn test_serde_nest_projection() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony);
        let snapshot = NestSnapshot::from(&nest);
        let projection = snapshot.project(50, NestKind::Colony);

        let json = serde_json::to_string(&projection).unwrap();
        let restored: NestProjection = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nest_id.as_str(), "colony_1");
    }

    #[test]
    fn test_serde_nest_expansion_tracker() {
        let mut tracker = NestExpansionTracker::new().with_decay_interval(1);

        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));

        for _ in 0..5 {
            tracker.tick();
        }

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: NestExpansionTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), 5);
        assert!(restored.get_nest(&NestId::new("colony_1")).is_some());
        assert_eq!(restored.fingerprint(), tracker.fingerprint());
    }

    #[test]
    fn test_serde_nest_expansion_summary() {
        let mut tracker = NestExpansionTracker::new();
        tracker.add_nest(make_nest("colony_1", "forest", NestKind::Colony).with_population(100));

        let summary = tracker.summary();
        let json = serde_json::to_string(&summary).unwrap();
        let restored: NestExpansionSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.total_nests, 1);
    }

    #[test]
    fn test_nest_deterministic_ordering() {
        let mut tracker = NestExpansionTracker::new();

        tracker.add_nest(make_nest("z_nest", "forest", NestKind::Colony));
        tracker.add_nest(make_nest("a_nest", "forest", NestKind::Colony));
        tracker.add_nest(make_nest("m_nest", "forest", NestKind::Colony));

        let ids: Vec<&str> = tracker.nests.keys().map(NestId::as_str).collect();

        assert_eq!(ids, vec!["a_nest", "m_nest", "z_nest"]);
    }

    #[test]
    fn test_nest_contested_front_interaction() {
        let mut pressure_tracker = TerritoryPressureTracker::new(PressureConfig::new());
        pressure_tracker.create_front(
            FactionId::new("faction_a"),
            FactionId::new("faction_b"),
            RegionId::new("contested_region"),
        );

        let mut nest_tracker = NestExpansionTracker::new();

        let mut nest = make_nest("colony_1", "home", NestKind::Colony)
            .with_faction(FactionId::new("faction_a"))
            .with_population(400);

        for i in 1..=60 {
            nest.tick(1, i);
        }

        nest_tracker.add_nest(nest);

        let candidates = nest_tracker.identify_expansion_candidates(
            &NestId::new("colony_1"),
            &[RegionId::new("contested_region")],
            &pressure_tracker,
        );

        if let Some(candidate) = candidates.first() {
            assert!(candidate.contested_front_penalty > 0.0);
        }
    }

    #[test]
    fn test_nest_unloaded_snapshot_behavior() {
        let nest = make_nest("colony_1", "forest", NestKind::Colony).with_population(200);

        let snapshot = NestSnapshot::from(&nest);

        // Use tick values that don't saturate the maturity cap (2.0).
        // Colony expansion_rate is 0.02, so 25 ticks -> 0.5, 50 ticks -> 1.0.
        let proj_25 = snapshot.project(25, NestKind::Colony);
        let proj_50 = snapshot.project(50, NestKind::Colony);

        assert!(proj_50.confidence < proj_25.confidence);
        assert!(proj_50.projected_maturity > proj_25.projected_maturity);
    }
}

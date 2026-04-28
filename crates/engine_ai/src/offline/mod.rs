//! Offline/cheap AI simulation for unloaded chunks and regions.
//!
//! Provides deterministic simulation of AI state without loading full entity data:
//!
//! - Composes existing cheap summaries (needs, sensors, factions, goals, population)
//! - Deterministic tick advancement with configurable time acceleration
//! - Staleness and attention reporting for region prioritization
//! - Per-region simulation budgets
//! - Aggregated offline events for deferred processing
//! - Load/unload handoff summaries for seamless transitions

use crate::faction::FactionSnapshot;
use crate::goal::GoalSnapshot;
use crate::needs::ColonySnapshot;
use crate::population::{PopulationSnapshot, PopulationSummary, ThreatLevel};
use crate::sensor::SensorSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique identifier for an offline region.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OfflineRegionId(String);

impl OfflineRegionId {
    /// Create a new region identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OfflineRegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Configuration for offline simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OfflineConfig {
    /// Maximum staleness (ticks) before a region snapshot must be refreshed.
    pub max_staleness: u64,
    /// Ticks ahead to check for attention needs.
    pub attention_horizon: u64,
    /// Default time acceleration for unloaded regions.
    pub default_time_acceleration: f32,
    /// Maximum time acceleration allowed.
    pub max_time_acceleration: f32,
    /// Minimum ticks between simulation steps.
    pub min_step_interval: u64,
    /// Maximum ticks to simulate in a single step.
    pub max_step_ticks: u64,
    /// Whether to generate events for offline changes.
    pub generate_events: bool,
    /// Threat level that triggers immediate attention.
    pub threat_attention_threshold: ThreatLevel,
    /// Population pressure that triggers attention.
    pub pressure_attention_threshold: f32,
}

impl Default for OfflineConfig {
    fn default() -> Self {
        Self {
            max_staleness: 1000,
            attention_horizon: 500,
            default_time_acceleration: 1.0,
            max_time_acceleration: 10.0,
            min_step_interval: 10,
            max_step_ticks: 500,
            generate_events: true,
            threat_attention_threshold: ThreatLevel::High,
            pressure_attention_threshold: 0.9,
        }
    }
}

impl OfflineConfig {
    /// Create a new config with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max staleness.
    #[must_use]
    pub fn with_max_staleness(mut self, ticks: u64) -> Self {
        self.max_staleness = ticks;
        self
    }

    /// Set attention horizon.
    #[must_use]
    pub fn with_attention_horizon(mut self, ticks: u64) -> Self {
        self.attention_horizon = ticks;
        self
    }

    /// Set default time acceleration.
    #[must_use]
    pub fn with_time_acceleration(mut self, factor: f32) -> Self {
        self.default_time_acceleration = factor.clamp(0.1, self.max_time_acceleration);
        self
    }

    /// Set max time acceleration.
    #[must_use]
    pub fn with_max_time_acceleration(mut self, factor: f32) -> Self {
        self.max_time_acceleration = factor.max(1.0);
        self
    }

    /// Set event generation.
    #[must_use]
    pub fn with_events(mut self, generate: bool) -> Self {
        self.generate_events = generate;
        self
    }

    /// Set pressure attention threshold.
    #[must_use]
    pub fn with_pressure_attention_threshold(mut self, threshold: f32) -> Self {
        self.pressure_attention_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

/// Per-region simulation budget.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionBudget {
    /// Ticks allocated for this region's simulation.
    pub tick_budget: u64,
    /// Maximum events to generate per step.
    pub max_events: u32,
    /// Priority weight (higher = more budget).
    pub priority: f32,
    /// Time acceleration override (if any).
    pub time_acceleration: Option<f32>,
    /// Whether simulation is paused.
    pub paused: bool,
}

impl Default for RegionBudget {
    fn default() -> Self {
        Self {
            tick_budget: 100,
            max_events: 10,
            priority: 1.0,
            time_acceleration: None,
            paused: false,
        }
    }
}

impl RegionBudget {
    /// Create a new budget with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tick budget.
    #[must_use]
    pub fn with_tick_budget(mut self, ticks: u64) -> Self {
        self.tick_budget = ticks;
        self
    }

    /// Set max events.
    #[must_use]
    pub fn with_max_events(mut self, max: u32) -> Self {
        self.max_events = max;
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority.max(0.0);
        self
    }

    /// Set time acceleration override.
    #[must_use]
    pub fn with_time_acceleration(mut self, factor: f32) -> Self {
        self.time_acceleration = Some(factor.max(0.1));
        self
    }

    /// Pause simulation.
    #[must_use]
    pub fn paused(mut self) -> Self {
        self.paused = true;
        self
    }

    /// Resume simulation.
    pub fn resume(&mut self) {
        self.paused = false;
    }
}

/// Event type for offline simulation changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OfflineEventKind {
    /// Population changed significantly.
    PopulationChange,
    /// Threat level changed.
    ThreatChange,
    /// Territory ownership changed.
    TerritoryChange,
    /// Faction state changed.
    FactionChange,
    /// Region needs immediate attention.
    AttentionRequired,
    /// Region became stale and needs refresh.
    SnapshotStale,
    /// Critical need threshold crossed.
    NeedsCritical,
    /// Goal state became urgent.
    GoalsUrgent,
    /// Sensor detected threat.
    ThreatDetected,
    /// Migration event occurred.
    MigrationEvent,
}

/// An event generated during offline simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OfflineEvent {
    /// Event kind.
    pub kind: OfflineEventKind,
    /// Region where event occurred.
    pub region_id: OfflineRegionId,
    /// Tick when event occurred.
    pub tick: u64,
    /// Optional description.
    pub description: Option<String>,
    /// Severity (0.0 = info, 1.0 = critical).
    pub severity: f32,
}

impl OfflineEvent {
    /// Create a new event.
    #[must_use]
    pub fn new(kind: OfflineEventKind, region_id: OfflineRegionId, tick: u64) -> Self {
        Self {
            kind,
            region_id,
            tick,
            description: None,
            severity: 0.5,
        }
    }

    /// Add description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set severity.
    #[must_use]
    pub fn with_severity(mut self, severity: f32) -> Self {
        self.severity = severity.clamp(0.0, 1.0);
        self
    }

    /// Check if this is a critical event.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.severity > 0.8
    }
}

/// Attention level for a region.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum AttentionLevel {
    /// No attention needed.
    #[default]
    None,
    /// Low attention (check when convenient).
    Low,
    /// Medium attention (check soon).
    Medium,
    /// High attention (check immediately).
    High,
    /// Critical attention (load region).
    Critical,
}

impl AttentionLevel {
    /// Convert to numeric priority.
    #[must_use]
    pub fn priority(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }

    /// Check if attention is needed.
    #[must_use]
    pub fn needs_attention(self) -> bool {
        self >= Self::Medium
    }

    /// Check if immediate action is required.
    #[must_use]
    pub fn is_urgent(self) -> bool {
        self >= Self::High
    }
}

/// Staleness information for a region.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StalenessInfo {
    /// Tick of last snapshot.
    pub last_snapshot_tick: u64,
    /// Tick of last simulation step.
    pub last_step_tick: u64,
    /// Current age in ticks.
    pub age: u64,
    /// Whether snapshot is stale.
    pub is_stale: bool,
    /// Estimated ticks until stale.
    pub ticks_until_stale: u64,
}

impl StalenessInfo {
    /// Create staleness info.
    #[must_use]
    pub fn new(snapshot_tick: u64, step_tick: u64, current_tick: u64, max_staleness: u64) -> Self {
        let age = current_tick.saturating_sub(snapshot_tick);
        let is_stale = age > max_staleness;
        let ticks_until_stale = max_staleness.saturating_sub(age);

        Self {
            last_snapshot_tick: snapshot_tick,
            last_step_tick: step_tick,
            age,
            is_stale,
            ticks_until_stale,
        }
    }

    /// Get freshness as a ratio (1.0 = fresh, 0.0 = stale).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "tick counts bounded")]
    pub fn freshness(&self, max_staleness: u64) -> f32 {
        if max_staleness == 0 {
            return 0.0;
        }
        1.0 - (self.age.min(max_staleness) as f32 / max_staleness as f32)
    }
}

/// Composite snapshot of all AI subsystems for a region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionSnapshot {
    /// Region identifier.
    pub region_id: OfflineRegionId,
    /// Needs snapshot (if available).
    pub needs: Option<ColonySnapshot>,
    /// Sensor snapshot (if available).
    pub sensors: Option<SensorSnapshot>,
    /// Faction snapshot (if available).
    pub faction: Option<FactionSnapshot>,
    /// Goal snapshot (if available).
    pub goals: Option<GoalSnapshot>,
    /// Population snapshot (if available).
    pub population: Option<PopulationSnapshot>,
    /// Tick when this composite was created.
    pub snapshot_tick: u64,
    /// Time acceleration factor.
    pub time_acceleration: f32,
}

impl RegionSnapshot {
    /// Create a new region snapshot.
    #[must_use]
    pub fn new(region_id: OfflineRegionId, tick: u64) -> Self {
        Self {
            region_id,
            needs: None,
            sensors: None,
            faction: None,
            goals: None,
            population: None,
            snapshot_tick: tick,
            time_acceleration: 1.0,
        }
    }

    /// Set needs snapshot.
    #[must_use]
    pub fn with_needs(mut self, snapshot: ColonySnapshot) -> Self {
        self.needs = Some(snapshot);
        self
    }

    /// Set sensor snapshot.
    #[must_use]
    pub fn with_sensors(mut self, snapshot: SensorSnapshot) -> Self {
        self.sensors = Some(snapshot);
        self
    }

    /// Set faction snapshot.
    #[must_use]
    pub fn with_faction(mut self, snapshot: FactionSnapshot) -> Self {
        self.faction = Some(snapshot);
        self
    }

    /// Set goals snapshot.
    #[must_use]
    pub fn with_goals(mut self, snapshot: GoalSnapshot) -> Self {
        self.goals = Some(snapshot);
        self
    }

    /// Set population snapshot.
    #[must_use]
    pub fn with_population(mut self, snapshot: PopulationSnapshot) -> Self {
        self.population = Some(snapshot);
        self
    }

    /// Set time acceleration.
    #[must_use]
    pub fn with_time_acceleration(mut self, factor: f32) -> Self {
        self.time_acceleration = factor.max(0.1);
        self
    }

    /// Check if the snapshot is stale.
    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.snapshot_tick) > max_staleness
    }

    /// Get the age of this snapshot.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.snapshot_tick)
    }

    /// Check if any subsystem needs attention.
    #[must_use]
    pub fn needs_attention(&self, attention_horizon: u64) -> bool {
        if let Some(needs) = &self.needs
            && needs.needs_attention(attention_horizon)
        {
            return true;
        }
        if let Some(sensors) = &self.sensors
            && sensors.needs_attention
        {
            return true;
        }
        if let Some(faction) = &self.faction
            && faction.any_needs_attention()
        {
            return true;
        }
        if let Some(goals) = &self.goals
            && goals.needs_attention
        {
            return true;
        }
        if let Some(population) = &self.population
            && population.summary.needs_attention
        {
            return true;
        }
        false
    }

    /// Get overall threat level.
    #[must_use]
    pub fn threat_level(&self) -> f32 {
        let mut max_threat = 0.0f32;

        if let Some(sensors) = &self.sensors {
            max_threat = max_threat.max(sensors.threat_level);
        }
        if let Some(faction) = &self.faction {
            max_threat = max_threat.max(faction.overall_threat);
        }
        if let Some(population) = &self.population {
            max_threat = max_threat.max(Self::threat_from_population(&population.summary));
        }

        max_threat
    }

    fn threat_from_population(summary: &PopulationSummary) -> f32 {
        match summary.threat_level {
            ThreatLevel::Safe => 0.0,
            ThreatLevel::Low => 0.25,
            ThreatLevel::Moderate => 0.5,
            ThreatLevel::High => 0.75,
            ThreatLevel::Extreme => 1.0,
        }
    }

    /// Determine attention level needed.
    #[must_use]
    pub fn attention_level(&self, config: &OfflineConfig, current_tick: u64) -> AttentionLevel {
        if self.is_stale(current_tick, config.max_staleness) {
            return AttentionLevel::High;
        }

        let threat = self.threat_level();
        if threat > 0.9 {
            return AttentionLevel::Critical;
        }
        if threat > 0.7 {
            return AttentionLevel::High;
        }

        if self.needs_attention(config.attention_horizon) {
            return AttentionLevel::Medium;
        }

        if let Some(population) = &self.population
            && population.summary.pressure > config.pressure_attention_threshold
        {
            return AttentionLevel::Medium;
        }

        if threat > 0.3 {
            return AttentionLevel::Low;
        }

        AttentionLevel::None
    }
}

/// Result of a simulation step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StepResult {
    /// Ticks actually simulated.
    pub ticks_simulated: u64,
    /// Events generated.
    pub events: Vec<OfflineEvent>,
    /// Whether region became stale.
    pub became_stale: bool,
    /// Whether attention is needed.
    pub needs_attention: bool,
    /// Current attention level.
    pub attention_level: AttentionLevel,
}

impl StepResult {
    /// Create a new step result.
    #[must_use]
    pub fn new(ticks: u64) -> Self {
        Self {
            ticks_simulated: ticks,
            ..Default::default()
        }
    }

    /// Add an event.
    pub fn add_event(&mut self, event: OfflineEvent) {
        self.events.push(event);
    }

    /// Set attention needed.
    pub fn set_attention(&mut self, level: AttentionLevel) {
        self.attention_level = level;
        self.needs_attention = level.needs_attention();
    }
}

/// State of an offline region simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OfflineRegionState {
    /// Region identifier.
    pub region_id: OfflineRegionId,
    /// Current snapshot.
    pub snapshot: RegionSnapshot,
    /// Current simulated tick.
    pub current_tick: u64,
    /// Per-region budget.
    pub budget: RegionBudget,
    /// Accumulated events.
    events: Vec<OfflineEvent>,
    /// Previous attention level.
    previous_attention: AttentionLevel,
    /// Ticks simulated since last snapshot.
    ticks_since_snapshot: u64,
}

impl OfflineRegionState {
    /// Create a new offline region state.
    #[must_use]
    pub fn new(region_id: OfflineRegionId, snapshot: RegionSnapshot, tick: u64) -> Self {
        Self {
            region_id,
            snapshot,
            current_tick: tick,
            budget: RegionBudget::new(),
            events: Vec::new(),
            previous_attention: AttentionLevel::None,
            ticks_since_snapshot: 0,
        }
    }

    /// Set the budget.
    #[must_use]
    pub fn with_budget(mut self, budget: RegionBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Update the snapshot.
    pub fn update_snapshot(&mut self, snapshot: RegionSnapshot, tick: u64) {
        self.snapshot = snapshot;
        self.current_tick = tick;
        self.ticks_since_snapshot = 0;
    }

    /// Get staleness info at the current region tick.
    #[must_use]
    pub fn staleness(&self, max_staleness: u64) -> StalenessInfo {
        StalenessInfo::new(
            self.snapshot.snapshot_tick,
            self.current_tick,
            self.current_tick,
            max_staleness,
        )
    }

    /// Get staleness info at a specific tick.
    #[must_use]
    pub fn staleness_at(&self, current_tick: u64, max_staleness: u64) -> StalenessInfo {
        StalenessInfo::new(
            self.snapshot.snapshot_tick,
            self.current_tick,
            current_tick,
            max_staleness,
        )
    }

    /// Get accumulated events and clear them.
    pub fn drain_events(&mut self) -> Vec<OfflineEvent> {
        std::mem::take(&mut self.events)
    }

    /// Get accumulated events without clearing.
    #[must_use]
    pub fn events(&self) -> &[OfflineEvent] {
        &self.events
    }

    /// Advance simulation by a number of ticks.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "tick calculations bounded"
    )]
    pub fn advance(&mut self, ticks: u64, config: &OfflineConfig) -> StepResult {
        if self.budget.paused {
            return StepResult::new(0);
        }

        let time_factor = self
            .budget
            .time_acceleration
            .unwrap_or(config.default_time_acceleration)
            .clamp(0.1, config.max_time_acceleration);

        let effective_ticks = ((ticks as f32 * time_factor) as u64)
            .min(config.max_step_ticks)
            .min(self.budget.tick_budget);

        if effective_ticks < config.min_step_interval {
            return StepResult::new(0);
        }

        self.current_tick += effective_ticks;
        self.ticks_since_snapshot += effective_ticks;

        let mut result = StepResult::new(effective_ticks);

        let staleness = self.staleness(config.max_staleness);
        if staleness.is_stale {
            result.became_stale = true;
            if config.generate_events {
                let event = OfflineEvent::new(
                    OfflineEventKind::SnapshotStale,
                    self.region_id.clone(),
                    self.current_tick,
                )
                .with_description("Snapshot needs refresh")
                .with_severity(0.6);
                result.add_event(event.clone());
                self.add_event_if_budget(event);
            }
        }

        self.check_and_generate_events(&mut result, config);

        let attention = self.snapshot.attention_level(config, self.current_tick);
        result.set_attention(attention);

        if attention > self.previous_attention && attention.is_urgent() && config.generate_events {
            let event = OfflineEvent::new(
                OfflineEventKind::AttentionRequired,
                self.region_id.clone(),
                self.current_tick,
            )
            .with_description(format!("Attention level increased to {attention:?}"))
            .with_severity(if attention == AttentionLevel::Critical {
                1.0
            } else {
                0.8
            });
            result.add_event(event.clone());
            self.add_event_if_budget(event);
        }

        self.previous_attention = attention;

        result
    }

    fn check_and_generate_events(&mut self, result: &mut StepResult, config: &OfflineConfig) {
        if !config.generate_events {
            return;
        }

        if let Some(needs) = &self.snapshot.needs
            && needs.summary.has_critical()
        {
            let event = OfflineEvent::new(
                OfflineEventKind::NeedsCritical,
                self.region_id.clone(),
                self.current_tick,
            )
            .with_severity(0.9);
            result.add_event(event.clone());
            self.add_event_if_budget(event);
        }

        if let Some(goals) = &self.snapshot.goals
            && goals.is_urgent()
        {
            let event = OfflineEvent::new(
                OfflineEventKind::GoalsUrgent,
                self.region_id.clone(),
                self.current_tick,
            )
            .with_severity(0.8);
            result.add_event(event.clone());
            self.add_event_if_budget(event);
        }

        if let Some(sensors) = &self.snapshot.sensors
            && sensors.is_dangerous()
        {
            let event = OfflineEvent::new(
                OfflineEventKind::ThreatDetected,
                self.region_id.clone(),
                self.current_tick,
            )
            .with_severity(0.85);
            result.add_event(event.clone());
            self.add_event_if_budget(event);
        }
    }

    fn add_event_if_budget(&mut self, event: OfflineEvent) {
        if self.events.len() < self.budget.max_events as usize {
            self.events.push(event);
        }
    }
}

/// Summary for handoff when loading a region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadHandoff {
    /// Region being loaded.
    pub region_id: OfflineRegionId,
    /// Last known snapshot.
    pub snapshot: RegionSnapshot,
    /// Accumulated offline events.
    pub events: Vec<OfflineEvent>,
    /// Ticks elapsed since snapshot.
    pub elapsed_ticks: u64,
    /// Projected population.
    pub projected_population: Option<u32>,
    /// Projected threat level.
    pub projected_threat: f32,
    /// Attention level at handoff.
    pub attention_level: AttentionLevel,
    /// Tick when handoff was created.
    pub handoff_tick: u64,
}

impl LoadHandoff {
    /// Create a load handoff from offline state.
    #[must_use]
    pub fn from_state(state: &OfflineRegionState, config: &OfflineConfig) -> Self {
        let elapsed = state.ticks_since_snapshot;
        let projected_population = state
            .snapshot
            .population
            .as_ref()
            .map(|p| p.project_population(elapsed));
        let projected_threat = state.snapshot.threat_level();
        let attention = state.snapshot.attention_level(config, state.current_tick);

        Self {
            region_id: state.region_id.clone(),
            snapshot: state.snapshot.clone(),
            events: state.events.clone(),
            elapsed_ticks: elapsed,
            projected_population,
            projected_threat,
            attention_level: attention,
            handoff_tick: state.current_tick,
        }
    }
}

/// Summary for handoff when unloading a region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnloadHandoff {
    /// Region being unloaded.
    pub region_id: OfflineRegionId,
    /// Final snapshot before unload.
    pub snapshot: RegionSnapshot,
    /// Budget to use for offline simulation.
    pub budget: RegionBudget,
    /// Tick when unloaded.
    pub unload_tick: u64,
    /// Reason for unload.
    pub reason: UnloadReason,
}

/// Reason for unloading a region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnloadReason {
    /// Normal distance-based unload.
    #[default]
    Distance,
    /// Memory pressure.
    MemoryPressure,
    /// Budget limit reached.
    BudgetLimit,
    /// Player request.
    PlayerRequest,
    /// Region became inactive.
    Inactive,
}

impl UnloadHandoff {
    /// Create an unload handoff.
    #[must_use]
    pub fn new(
        region_id: OfflineRegionId,
        snapshot: RegionSnapshot,
        budget: RegionBudget,
        tick: u64,
        reason: UnloadReason,
    ) -> Self {
        Self {
            region_id,
            snapshot,
            budget,
            unload_tick: tick,
            reason,
        }
    }

    /// Convert to offline region state.
    #[must_use]
    pub fn into_state(self) -> OfflineRegionState {
        OfflineRegionState::new(self.region_id, self.snapshot, self.unload_tick)
            .with_budget(self.budget)
    }
}

/// Manager for multiple offline regions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OfflineSimulator {
    /// Configuration.
    config: OfflineConfig,
    /// Offline regions by ID.
    regions: BTreeMap<OfflineRegionId, OfflineRegionState>,
    /// Current global tick.
    current_tick: u64,
    /// Total ticks simulated.
    total_ticks_simulated: u64,
}

impl OfflineSimulator {
    /// Create a new simulator.
    #[must_use]
    pub fn new(config: OfflineConfig) -> Self {
        Self {
            config,
            regions: BTreeMap::new(),
            current_tick: 0,
            total_ticks_simulated: 0,
        }
    }

    /// Get the config.
    #[must_use]
    pub fn config(&self) -> &OfflineConfig {
        &self.config
    }

    /// Get mutable config.
    pub fn config_mut(&mut self) -> &mut OfflineConfig {
        &mut self.config
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Set current tick.
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Get total ticks simulated.
    #[must_use]
    pub fn total_ticks_simulated(&self) -> u64 {
        self.total_ticks_simulated
    }

    /// Get number of offline regions.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Check if a region is being simulated.
    #[must_use]
    pub fn has_region(&self, id: &OfflineRegionId) -> bool {
        self.regions.contains_key(id)
    }

    /// Get a region state.
    #[must_use]
    pub fn get_region(&self, id: &OfflineRegionId) -> Option<&OfflineRegionState> {
        self.regions.get(id)
    }

    /// Get a mutable region state.
    pub fn get_region_mut(&mut self, id: &OfflineRegionId) -> Option<&mut OfflineRegionState> {
        self.regions.get_mut(id)
    }

    /// Add a region from an unload handoff.
    pub fn add_region(&mut self, handoff: UnloadHandoff) {
        let state = handoff.into_state();
        self.regions.insert(state.region_id.clone(), state);
    }

    /// Remove a region and create a load handoff.
    #[must_use]
    pub fn remove_region(&mut self, id: &OfflineRegionId) -> Option<LoadHandoff> {
        self.regions
            .remove(id)
            .map(|state| LoadHandoff::from_state(&state, &self.config))
    }

    /// Update a region's snapshot.
    pub fn update_snapshot(&mut self, id: &OfflineRegionId, snapshot: RegionSnapshot, tick: u64) {
        if let Some(state) = self.regions.get_mut(id) {
            state.update_snapshot(snapshot, tick);
        }
    }

    /// Update a region's budget.
    pub fn update_budget(&mut self, id: &OfflineRegionId, budget: RegionBudget) {
        if let Some(state) = self.regions.get_mut(id) {
            state.budget = budget;
        }
    }

    /// Advance all regions by the given ticks.
    pub fn advance_all(&mut self, ticks: u64) -> Vec<StepResult> {
        self.current_tick += ticks;

        let config = self.config.clone();
        let mut results = Vec::with_capacity(self.regions.len());

        for state in self.regions.values_mut() {
            let result = state.advance(ticks, &config);
            self.total_ticks_simulated += result.ticks_simulated;
            results.push(result);
        }

        results
    }

    /// Advance a single region.
    pub fn advance_region(&mut self, id: &OfflineRegionId, ticks: u64) -> Option<StepResult> {
        let config = self.config.clone();
        self.regions.get_mut(id).map(|state| {
            let result = state.advance(ticks, &config);
            self.total_ticks_simulated += result.ticks_simulated;
            result
        })
    }

    /// Get all regions needing attention.
    pub fn regions_needing_attention(&self) -> Vec<(&OfflineRegionId, AttentionLevel)> {
        self.regions
            .iter()
            .filter_map(|(id, state)| {
                let level = state
                    .snapshot
                    .attention_level(&self.config, self.current_tick);
                if level.needs_attention() {
                    Some((id, level))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get regions sorted by attention priority (highest first).
    pub fn regions_by_priority(&self) -> Vec<&OfflineRegionId> {
        let mut regions: Vec<_> = self.regions.keys().collect();
        regions.sort_by(|a, b| {
            let level_a = self.regions[*a]
                .snapshot
                .attention_level(&self.config, self.current_tick);
            let level_b = self.regions[*b]
                .snapshot
                .attention_level(&self.config, self.current_tick);
            level_b.priority().cmp(&level_a.priority())
        });
        regions
    }

    /// Get all stale regions.
    pub fn stale_regions(&self) -> Vec<&OfflineRegionId> {
        self.regions
            .iter()
            .filter(|(_, state)| {
                state
                    .staleness_at(self.current_tick, self.config.max_staleness)
                    .is_stale
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Drain all events from all regions.
    pub fn drain_all_events(&mut self) -> Vec<OfflineEvent> {
        let mut all_events = Vec::new();
        for state in self.regions.values_mut() {
            all_events.extend(state.drain_events());
        }
        all_events.sort_by_key(|e| e.tick);
        all_events
    }

    /// Iterate over all regions.
    pub fn iter(&self) -> impl Iterator<Item = (&OfflineRegionId, &OfflineRegionState)> {
        self.regions.iter()
    }

    /// Iterate over region IDs.
    pub fn region_ids(&self) -> impl Iterator<Item = &OfflineRegionId> {
        self.regions.keys()
    }
}

/// Aggregate summary of all offline regions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OfflineSummary {
    /// Number of offline regions.
    pub region_count: u32,
    /// Total population estimate.
    pub total_population: u32,
    /// Regions needing attention.
    pub regions_needing_attention: u32,
    /// Stale regions.
    pub stale_regions: u32,
    /// Average threat level.
    pub average_threat: f32,
    /// Highest attention level.
    pub max_attention: AttentionLevel,
    /// Total events pending.
    pub pending_events: u32,
    /// Computed at tick.
    pub computed_at_tick: u64,
}

impl OfflineSummary {
    /// Create summary from simulator.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "counts bounded"
    )]
    pub fn from_simulator(simulator: &OfflineSimulator) -> Self {
        let config = simulator.config();
        let current_tick = simulator.current_tick();

        let mut total_population = 0u32;
        let mut threat_sum = 0.0f32;
        let mut max_attention = AttentionLevel::None;
        let mut attention_count = 0u32;
        let mut stale_count = 0u32;
        let mut event_count = 0u32;

        for state in simulator.regions.values() {
            if let Some(pop) = &state.snapshot.population {
                total_population += pop.summary.total_population;
            }

            threat_sum += state.snapshot.threat_level();

            let attention = state.snapshot.attention_level(config, current_tick);
            if attention > max_attention {
                max_attention = attention;
            }
            if attention.needs_attention() {
                attention_count += 1;
            }

            if state.staleness(config.max_staleness).is_stale {
                stale_count += 1;
            }

            event_count += state.events.len() as u32;
        }

        let region_count = simulator.region_count() as u32;
        let average_threat = if region_count > 0 {
            threat_sum / region_count as f32
        } else {
            0.0
        };

        Self {
            region_count,
            total_population,
            regions_needing_attention: attention_count,
            stale_regions: stale_count,
            average_threat,
            max_attention,
            pending_events: event_count,
            computed_at_tick: current_tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::population::{RegionDensity, SpawnPressure};

    fn make_basic_snapshot(region_id: &str, tick: u64) -> RegionSnapshot {
        RegionSnapshot::new(OfflineRegionId::new(region_id), tick)
    }

    fn make_population_snapshot(total: u32, pressure: f32) -> PopulationSnapshot {
        let mut summary = PopulationSummary::new("test");
        summary.total_population = total;
        summary.pressure = pressure;
        summary.density = RegionDensity::from_pressure(pressure);
        summary.spawn_pressure = SpawnPressure::from_state(pressure, total == 0);
        PopulationSnapshot::new(summary, 0)
    }

    #[test]
    fn test_offline_region_id() {
        let id = OfflineRegionId::new("test_region");
        assert_eq!(id.as_str(), "test_region");
        assert_eq!(format!("{id}"), "test_region");
    }

    #[test]
    fn test_offline_config_default() {
        let config = OfflineConfig::default();
        assert_eq!(config.max_staleness, 1000);
        assert_eq!(config.attention_horizon, 500);
        assert!((config.default_time_acceleration - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_offline_config_builder() {
        let config = OfflineConfig::new()
            .with_max_staleness(2000)
            .with_attention_horizon(1000)
            .with_time_acceleration(2.0)
            .with_events(false);

        assert_eq!(config.max_staleness, 2000);
        assert_eq!(config.attention_horizon, 1000);
        assert!((config.default_time_acceleration - 2.0).abs() < f32::EPSILON);
        assert!(!config.generate_events);
    }

    #[test]
    fn test_region_budget_default() {
        let budget = RegionBudget::default();
        assert_eq!(budget.tick_budget, 100);
        assert_eq!(budget.max_events, 10);
        assert!((budget.priority - 1.0).abs() < f32::EPSILON);
        assert!(!budget.paused);
    }

    #[test]
    fn test_region_budget_builder() {
        let budget = RegionBudget::new()
            .with_tick_budget(200)
            .with_max_events(20)
            .with_priority(2.0)
            .with_time_acceleration(1.5)
            .paused();

        assert_eq!(budget.tick_budget, 200);
        assert_eq!(budget.max_events, 20);
        assert!((budget.priority - 2.0).abs() < f32::EPSILON);
        assert_eq!(budget.time_acceleration, Some(1.5));
        assert!(budget.paused);
    }

    #[test]
    fn test_offline_event() {
        let event = OfflineEvent::new(
            OfflineEventKind::PopulationChange,
            OfflineRegionId::new("test"),
            100,
        )
        .with_description("Population increased")
        .with_severity(0.7);

        assert_eq!(event.kind, OfflineEventKind::PopulationChange);
        assert_eq!(event.tick, 100);
        assert_eq!(event.description, Some("Population increased".to_string()));
        assert!((event.severity - 0.7).abs() < f32::EPSILON);
        assert!(!event.is_critical());
    }

    #[test]
    fn test_offline_event_critical() {
        let event = OfflineEvent::new(
            OfflineEventKind::ThreatDetected,
            OfflineRegionId::new("test"),
            0,
        )
        .with_severity(0.9);

        assert!(event.is_critical());
    }

    #[test]
    fn test_attention_level_ordering() {
        assert!(AttentionLevel::Critical > AttentionLevel::High);
        assert!(AttentionLevel::High > AttentionLevel::Medium);
        assert!(AttentionLevel::Medium > AttentionLevel::Low);
        assert!(AttentionLevel::Low > AttentionLevel::None);
    }

    #[test]
    fn test_attention_level_priority() {
        assert_eq!(AttentionLevel::None.priority(), 0);
        assert_eq!(AttentionLevel::Critical.priority(), 4);
    }

    #[test]
    fn test_attention_level_needs_attention() {
        assert!(!AttentionLevel::None.needs_attention());
        assert!(!AttentionLevel::Low.needs_attention());
        assert!(AttentionLevel::Medium.needs_attention());
        assert!(AttentionLevel::High.needs_attention());
        assert!(AttentionLevel::Critical.needs_attention());
    }

    #[test]
    fn test_staleness_info() {
        let info = StalenessInfo::new(100, 150, 200, 500);

        assert_eq!(info.last_snapshot_tick, 100);
        assert_eq!(info.age, 100);
        assert!(!info.is_stale);
        assert_eq!(info.ticks_until_stale, 400);
    }

    #[test]
    fn test_staleness_info_stale() {
        let info = StalenessInfo::new(100, 150, 700, 500);

        assert!(info.is_stale);
        assert_eq!(info.ticks_until_stale, 0);
    }

    #[test]
    fn test_staleness_freshness() {
        let fresh = StalenessInfo::new(100, 100, 100, 500);
        assert!((fresh.freshness(500) - 1.0).abs() < f32::EPSILON);

        let stale = StalenessInfo::new(100, 100, 600, 500);
        assert!((stale.freshness(500)).abs() < f32::EPSILON);

        let half = StalenessInfo::new(100, 100, 350, 500);
        assert!((half.freshness(500) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_region_snapshot_new() {
        let snapshot = make_basic_snapshot("region1", 100);

        assert_eq!(snapshot.region_id.as_str(), "region1");
        assert_eq!(snapshot.snapshot_tick, 100);
        assert!(snapshot.needs.is_none());
        assert!(snapshot.sensors.is_none());
    }

    #[test]
    fn test_region_snapshot_with_population() {
        let pop = make_population_snapshot(100, 0.5);
        let snapshot = make_basic_snapshot("test", 0).with_population(pop);

        assert!(snapshot.population.is_some());
        assert_eq!(snapshot.population.unwrap().summary.total_population, 100);
    }

    #[test]
    fn test_region_snapshot_staleness() {
        let snapshot = make_basic_snapshot("test", 100);

        assert!(!snapshot.is_stale(200, 500));
        assert!(snapshot.is_stale(700, 500));
        assert_eq!(snapshot.age(200), 100);
    }

    #[test]
    fn test_region_snapshot_attention_level_stale() {
        let config = OfflineConfig::new().with_max_staleness(100);
        let snapshot = make_basic_snapshot("test", 0);

        let level = snapshot.attention_level(&config, 200);
        assert_eq!(level, AttentionLevel::High);
    }

    #[test]
    fn test_step_result() {
        let mut result = StepResult::new(100);
        assert_eq!(result.ticks_simulated, 100);
        assert!(!result.needs_attention);

        result.set_attention(AttentionLevel::High);
        assert!(result.needs_attention);
        assert_eq!(result.attention_level, AttentionLevel::High);
    }

    #[test]
    fn test_offline_region_state_new() {
        let snapshot = make_basic_snapshot("test", 100);
        let state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100);

        assert_eq!(state.region_id.as_str(), "test");
        assert_eq!(state.current_tick, 100);
        assert!(state.events.is_empty());
    }

    #[test]
    fn test_offline_region_state_advance_paused() {
        let snapshot = make_basic_snapshot("test", 100);
        let mut state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100)
            .with_budget(RegionBudget::new().paused());

        let config = OfflineConfig::default();
        let result = state.advance(100, &config);

        assert_eq!(result.ticks_simulated, 0);
    }

    #[test]
    fn test_offline_region_state_advance() {
        let snapshot = make_basic_snapshot("test", 100);
        let mut state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100);

        let config = OfflineConfig::new().with_events(false);
        let result = state.advance(50, &config);

        assert_eq!(result.ticks_simulated, 50);
        assert_eq!(state.current_tick, 150);
    }

    #[test]
    fn test_offline_region_state_staleness() {
        let snapshot = make_basic_snapshot("test", 100);
        let state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100);

        let info = state.staleness(500);
        assert!(!info.is_stale);
    }

    #[test]
    fn test_offline_region_state_drain_events() {
        let snapshot = make_basic_snapshot("test", 0);
        let mut state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 0);

        let config = OfflineConfig::new().with_max_staleness(10);
        state.advance(50, &config);

        let events = state.drain_events();
        assert!(!events.is_empty());
        assert!(state.events().is_empty());
    }

    #[test]
    fn test_load_handoff() {
        let snapshot = make_basic_snapshot("test", 100);
        let state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100);
        let config = OfflineConfig::default();

        let handoff = LoadHandoff::from_state(&state, &config);

        assert_eq!(handoff.region_id.as_str(), "test");
        assert_eq!(handoff.handoff_tick, 100);
    }

    #[test]
    fn test_unload_handoff() {
        let snapshot = make_basic_snapshot("test", 100);
        let budget = RegionBudget::new().with_priority(2.0);
        let handoff = UnloadHandoff::new(
            OfflineRegionId::new("test"),
            snapshot,
            budget,
            100,
            UnloadReason::Distance,
        );

        assert_eq!(handoff.region_id.as_str(), "test");
        assert_eq!(handoff.reason, UnloadReason::Distance);

        let state = handoff.into_state();
        assert!((state.budget.priority - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_offline_simulator_new() {
        let config = OfflineConfig::default();
        let simulator = OfflineSimulator::new(config);

        assert_eq!(simulator.region_count(), 0);
        assert_eq!(simulator.current_tick(), 0);
    }

    #[test]
    fn test_offline_simulator_add_remove_region() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::default());

        let snapshot = make_basic_snapshot("region1", 0);
        let handoff = UnloadHandoff::new(
            OfflineRegionId::new("region1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        );

        simulator.add_region(handoff);
        assert_eq!(simulator.region_count(), 1);
        assert!(simulator.has_region(&OfflineRegionId::new("region1")));

        let load_handoff = simulator.remove_region(&OfflineRegionId::new("region1"));
        assert!(load_handoff.is_some());
        assert_eq!(simulator.region_count(), 0);
    }

    #[test]
    fn test_offline_simulator_advance_all() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::new().with_events(false));

        let snapshot1 = make_basic_snapshot("r1", 0);
        let snapshot2 = make_basic_snapshot("r2", 0);

        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot1,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r2"),
            snapshot2,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let results = simulator.advance_all(50);
        assert_eq!(results.len(), 2);
        assert_eq!(simulator.current_tick(), 50);
    }

    #[test]
    fn test_offline_simulator_advance_region() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::new().with_events(false));

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let result = simulator.advance_region(&OfflineRegionId::new("r1"), 50);
        assert!(result.is_some());
        assert_eq!(result.unwrap().ticks_simulated, 50);
    }

    #[test]
    fn test_offline_simulator_stale_regions() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::new().with_max_staleness(100));
        simulator.set_tick(200);

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let stale = simulator.stale_regions();
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn test_offline_simulator_regions_by_priority() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::new().with_max_staleness(1000));

        let snapshot1 = make_basic_snapshot("r1", 0);
        let snapshot2 =
            make_basic_snapshot("r2", 0).with_population(make_population_snapshot(100, 0.95));

        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot1,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r2"),
            snapshot2,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let priorities = simulator.regions_by_priority();
        assert_eq!(priorities.len(), 2);
    }

    #[test]
    fn test_offline_simulator_drain_events() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::new().with_max_staleness(10));

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        simulator.advance_all(50);
        let events = simulator.drain_all_events();
        assert!(!events.is_empty());
    }

    #[test]
    fn test_offline_summary() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::default());

        let pop = make_population_snapshot(100, 0.5);
        let snapshot = make_basic_snapshot("r1", 0).with_population(pop);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let summary = OfflineSummary::from_simulator(&simulator);
        assert_eq!(summary.region_count, 1);
        assert_eq!(summary.total_population, 100);
    }

    #[test]
    fn test_serde_offline_config() {
        let config = OfflineConfig::new()
            .with_max_staleness(2000)
            .with_time_acceleration(2.0);

        let json = serde_json::to_string(&config).unwrap();
        let restored: OfflineConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.max_staleness, 2000);
        assert!((restored.default_time_acceleration - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_region_budget() {
        let budget = RegionBudget::new().with_tick_budget(200).with_priority(1.5);

        let json = serde_json::to_string(&budget).unwrap();
        let restored: RegionBudget = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick_budget, 200);
        assert!((restored.priority - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_offline_event() {
        let event = OfflineEvent::new(
            OfflineEventKind::ThreatChange,
            OfflineRegionId::new("test"),
            100,
        )
        .with_severity(0.8);

        let json = serde_json::to_string(&event).unwrap();
        let restored: OfflineEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.kind, OfflineEventKind::ThreatChange);
        assert_eq!(restored.tick, 100);
    }

    #[test]
    fn test_serde_region_snapshot() {
        let snapshot = make_basic_snapshot("test", 100).with_time_acceleration(2.0);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: RegionSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id.as_str(), "test");
        assert_eq!(restored.snapshot_tick, 100);
    }

    #[test]
    fn test_serde_offline_region_state() {
        let snapshot = make_basic_snapshot("test", 100);
        let state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100);

        let json = serde_json::to_string(&state).unwrap();
        let restored: OfflineRegionState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id.as_str(), "test");
        assert_eq!(restored.current_tick, 100);
    }

    #[test]
    fn test_serde_load_handoff() {
        let snapshot = make_basic_snapshot("test", 100);
        let state = OfflineRegionState::new(OfflineRegionId::new("test"), snapshot, 100);
        let config = OfflineConfig::default();
        let handoff = LoadHandoff::from_state(&state, &config);

        let json = serde_json::to_string(&handoff).unwrap();
        let restored: LoadHandoff = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_id.as_str(), "test");
    }

    #[test]
    fn test_serde_unload_handoff() {
        let snapshot = make_basic_snapshot("test", 100);
        let handoff = UnloadHandoff::new(
            OfflineRegionId::new("test"),
            snapshot,
            RegionBudget::new(),
            100,
            UnloadReason::MemoryPressure,
        );

        let json = serde_json::to_string(&handoff).unwrap();
        let restored: UnloadHandoff = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.reason, UnloadReason::MemoryPressure);
    }

    #[test]
    fn test_serde_offline_simulator() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::default());
        simulator.set_tick(500);

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let json = serde_json::to_string(&simulator).unwrap();
        let restored: OfflineSimulator = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), 500);
        assert_eq!(restored.region_count(), 1);
    }

    #[test]
    fn test_serde_offline_summary() {
        let simulator = OfflineSimulator::new(OfflineConfig::default());
        let summary = OfflineSummary::from_simulator(&simulator);

        let json = serde_json::to_string(&summary).unwrap();
        let restored: OfflineSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.region_count, 0);
    }

    #[test]
    fn test_time_acceleration() {
        let mut simulator = OfflineSimulator::new(
            OfflineConfig::new()
                .with_time_acceleration(2.0)
                .with_events(false),
        );

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let results = simulator.advance_all(50);
        assert_eq!(results[0].ticks_simulated, 100);
    }

    #[test]
    fn test_budget_time_acceleration_override() {
        let mut simulator = OfflineSimulator::new(
            OfflineConfig::new()
                .with_time_acceleration(1.0)
                .with_events(false),
        );

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new().with_time_acceleration(3.0),
            0,
            UnloadReason::Distance,
        ));

        let results = simulator.advance_all(50);
        assert_eq!(results[0].ticks_simulated, 100);
    }

    #[test]
    fn test_max_step_ticks_limit() {
        let mut simulator = OfflineSimulator::new(
            OfflineConfig::new()
                .with_time_acceleration(10.0)
                .with_events(false),
        );

        let snapshot = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot,
            RegionBudget::new().with_tick_budget(1000),
            0,
            UnloadReason::Distance,
        ));

        let results = simulator.advance_all(100);
        assert!(results[0].ticks_simulated <= 500);
    }

    #[test]
    fn test_deterministic_advance() {
        let config = OfflineConfig::new().with_events(false);
        let snapshot = make_basic_snapshot("r1", 0);

        let mut state1 = OfflineRegionState::new(OfflineRegionId::new("r1"), snapshot.clone(), 0);
        let mut state2 = OfflineRegionState::new(OfflineRegionId::new("r1"), snapshot, 0);

        let result1 = state1.advance(100, &config);
        let result2 = state2.advance(100, &config);

        assert_eq!(result1.ticks_simulated, result2.ticks_simulated);
        assert_eq!(state1.current_tick, state2.current_tick);
    }

    #[test]
    fn test_update_snapshot() {
        let mut simulator = OfflineSimulator::new(OfflineConfig::default());

        let snapshot1 = make_basic_snapshot("r1", 0);
        simulator.add_region(UnloadHandoff::new(
            OfflineRegionId::new("r1"),
            snapshot1,
            RegionBudget::new(),
            0,
            UnloadReason::Distance,
        ));

        let snapshot2 = make_basic_snapshot("r1", 100);
        simulator.update_snapshot(&OfflineRegionId::new("r1"), snapshot2, 100);

        let state = simulator.get_region(&OfflineRegionId::new("r1")).unwrap();
        assert_eq!(state.snapshot.snapshot_tick, 100);
    }

    #[test]
    fn test_attention_detection() {
        let config = OfflineConfig::new().with_pressure_attention_threshold(0.8);

        let pop = make_population_snapshot(100, 0.9);
        let snapshot = make_basic_snapshot("r1", 0).with_population(pop);

        let level = snapshot.attention_level(&config, 0);
        assert!(level >= AttentionLevel::Medium);
    }
}

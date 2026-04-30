//! Resource sustainability tuning for ecological simulation.
//!
//! Provides explicit sustainability policies, harvest pressure tracking,
//! depletion/recovery projections, and sustainability ratings.

use super::ResourceZoneId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a resource zone regenerates over time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RegenerationMode {
    /// Fixed rate per tick regardless of current level.
    Constant { rate: f32 },
    /// Logistic growth: faster near inflection point, slower at extremes.
    Logistic { rate: f32, inflection: f32 },
    /// Seasonal variation with per-season multipliers (4 seasons).
    Seasonal {
        base_rate: f32,
        season_multipliers: [f32; 4],
    },
    /// No natural regeneration.
    Disabled,
}

impl Default for RegenerationMode {
    fn default() -> Self {
        Self::Constant { rate: 1.0 }
    }
}

impl RegenerationMode {
    #[must_use]
    pub fn compute_rate(&self, current: f32, capacity: f32, season: u8) -> f32 {
        match self {
            Self::Constant { rate } => *rate,
            Self::Logistic { rate, inflection } => {
                let fraction = if capacity > 0.0 {
                    current / capacity
                } else {
                    0.0
                };
                let growth = fraction * (1.0 - fraction) * 4.0;
                let scaled = if fraction < *inflection {
                    growth * (fraction / inflection)
                } else {
                    growth * ((1.0 - fraction) / (1.0 - inflection))
                };
                rate * scaled.max(0.0)
            }
            Self::Seasonal {
                base_rate,
                season_multipliers,
            } => {
                let idx = (season % 4) as usize;
                base_rate * season_multipliers[idx]
            }
            Self::Disabled => 0.0,
        }
    }

    #[must_use]
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }
}

/// How depletion affects the zone as resources drop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum DepletionBehavior {
    /// Harvest directly reduces resources 1:1.
    #[default]
    Linear,
    /// Depletion accelerates below a threshold.
    Accelerated { threshold: f32, multiplier: f32 },
    /// Some buffer absorbs initial depletion.
    Buffered { buffer: f32 },
}

impl DepletionBehavior {
    #[must_use]
    pub fn effective_depletion(&self, raw_amount: f32, current_fraction: f32) -> f32 {
        match self {
            Self::Linear => raw_amount,
            Self::Accelerated {
                threshold,
                multiplier,
            } => {
                if current_fraction < *threshold {
                    raw_amount * multiplier
                } else {
                    raw_amount
                }
            }
            Self::Buffered { buffer } => (raw_amount - buffer).max(0.0),
        }
    }
}

/// Per-zone sustainability policy configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SustainabilityPolicy {
    pub regeneration_mode: RegenerationMode,
    pub depletion_behavior: DepletionBehavior,
    /// Penalty multiplier applied when harvesting beyond sustainable yield.
    pub overharvest_penalty_rate: f32,
    /// Fraction of capacity below which recovery mode activates.
    pub recovery_threshold: f32,
    /// Fraction of renewal rate that counts as sustainable yield.
    pub sustainable_yield_fraction: f32,
    /// Minimum level for zone to be considered "recovered".
    pub recovered_threshold: f32,
    /// Whether overharvest penalties are applied.
    pub penalties_enabled: bool,
}

impl Default for SustainabilityPolicy {
    fn default() -> Self {
        Self {
            regeneration_mode: RegenerationMode::default(),
            depletion_behavior: DepletionBehavior::default(),
            overharvest_penalty_rate: 0.5,
            recovery_threshold: 0.2,
            sustainable_yield_fraction: 0.8,
            recovered_threshold: 0.7,
            penalties_enabled: true,
        }
    }
}

impl SustainabilityPolicy {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_regeneration_mode(mut self, mode: RegenerationMode) -> Self {
        self.regeneration_mode = mode;
        self
    }

    #[must_use]
    pub fn with_depletion_behavior(mut self, behavior: DepletionBehavior) -> Self {
        self.depletion_behavior = behavior;
        self
    }

    #[must_use]
    pub fn with_overharvest_penalty(mut self, rate: f32) -> Self {
        self.overharvest_penalty_rate = rate.clamp(0.0, 2.0);
        self
    }

    #[must_use]
    pub fn with_recovery_threshold(mut self, threshold: f32) -> Self {
        self.recovery_threshold = threshold.clamp(0.01, 0.9);
        self
    }

    #[must_use]
    pub fn with_sustainable_yield(mut self, fraction: f32) -> Self {
        self.sustainable_yield_fraction = fraction.clamp(0.1, 1.0);
        self
    }

    #[must_use]
    pub fn with_recovered_threshold(mut self, threshold: f32) -> Self {
        self.recovered_threshold = threshold.clamp(0.1, 1.0);
        self
    }

    #[must_use]
    pub fn with_penalties(mut self, enabled: bool) -> Self {
        self.penalties_enabled = enabled;
        self
    }

    #[must_use]
    pub fn compute_sustainable_yield(&self, renewal_rate: f32, capacity: f32) -> f32 {
        renewal_rate * self.sustainable_yield_fraction * capacity
    }
}

/// Carrying capacity behavior configuration.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CarryingCapacityConfig {
    /// Maximum fraction above base capacity (headroom).
    pub headroom: f32,
    /// Whether regeneration slows as capacity is approached.
    pub soft_cap: bool,
    /// Rate at which regeneration slows near capacity (0.0-1.0).
    pub soft_cap_rate: f32,
}

impl Default for CarryingCapacityConfig {
    fn default() -> Self {
        Self {
            headroom: 0.0,
            soft_cap: true,
            soft_cap_rate: 0.5,
        }
    }
}

impl CarryingCapacityConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_headroom(mut self, headroom: f32) -> Self {
        self.headroom = headroom.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_soft_cap(mut self, enabled: bool) -> Self {
        self.soft_cap = enabled;
        self
    }

    #[must_use]
    pub fn with_soft_cap_rate(mut self, rate: f32) -> Self {
        self.soft_cap_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn effective_capacity(&self, base_capacity: f32) -> f32 {
        base_capacity * (1.0 + self.headroom)
    }

    #[must_use]
    pub fn regeneration_modifier(&self, current_fraction: f32) -> f32 {
        if !self.soft_cap || current_fraction < 0.8 {
            1.0
        } else {
            let excess = (current_fraction - 0.8) / 0.2;
            1.0 - (excess * self.soft_cap_rate)
        }
    }
}

/// Tracks harvest pressure on a resource zone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HarvestPressure {
    zone_id: ResourceZoneId,
    window_ticks: u64,
    consumed_history: Vec<f32>,
    current_index: usize,
    total_consumed: f32,
    sustainable_limit: f32,
    last_tick: u64,
}

impl HarvestPressure {
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(zone_id: ResourceZoneId, window_ticks: u64, sustainable_limit: f32) -> Self {
        let window = window_ticks.max(1) as usize;
        Self {
            zone_id,
            window_ticks: window_ticks.max(1),
            consumed_history: vec![0.0; window],
            current_index: 0,
            total_consumed: 0.0,
            sustainable_limit: sustainable_limit.max(0.0),
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn zone_id(&self) -> &ResourceZoneId {
        &self.zone_id
    }

    #[must_use]
    pub fn window_ticks(&self) -> u64 {
        self.window_ticks
    }

    #[must_use]
    pub fn total_consumed(&self) -> f32 {
        self.total_consumed
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn average_consumption(&self) -> f32 {
        self.total_consumed / self.window_ticks as f32
    }

    #[must_use]
    pub fn sustainable_limit(&self) -> f32 {
        self.sustainable_limit
    }

    pub fn set_sustainable_limit(&mut self, limit: f32) {
        self.sustainable_limit = limit.max(0.0);
    }

    #[must_use]
    pub fn pressure_ratio(&self) -> f32 {
        if self.sustainable_limit > 0.0 {
            self.average_consumption() / self.sustainable_limit
        } else if self.total_consumed > 0.0 {
            f32::INFINITY
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn is_overharvested(&self) -> bool {
        self.pressure_ratio() > 1.0
    }

    #[must_use]
    pub fn is_unsustainable(&self) -> bool {
        self.pressure_ratio() > 1.5
    }

    pub fn record_harvest(&mut self, amount: f32, tick: u64) {
        let elapsed = tick.saturating_sub(self.last_tick);
        for _ in 0..elapsed.min(self.window_ticks) {
            self.advance_window();
        }
        self.consumed_history[self.current_index] += amount;
        self.total_consumed += amount;
        self.last_tick = tick;
    }

    fn advance_window(&mut self) {
        self.current_index = (self.current_index + 1) % self.consumed_history.len();
        self.total_consumed -= self.consumed_history[self.current_index];
        self.consumed_history[self.current_index] = 0.0;
    }

    pub fn tick(&mut self, current_tick: u64) {
        let elapsed = current_tick.saturating_sub(self.last_tick);
        for _ in 0..elapsed.min(self.window_ticks) {
            self.advance_window();
        }
        self.last_tick = current_tick;
    }

    pub fn reset(&mut self) {
        self.consumed_history.fill(0.0);
        self.current_index = 0;
        self.total_consumed = 0.0;
    }
}

/// Overall sustainability rating for a resource zone.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SustainabilityRating {
    Collapsed,
    Depleted,
    Overharvested,
    Stressed,
    #[default]
    Sustainable,
    Thriving,
}

impl SustainabilityRating {
    #[must_use]
    pub fn compute(current_fraction: f32, pressure_ratio: f32, recovery_threshold: f32) -> Self {
        if current_fraction < 0.01 {
            Self::Collapsed
        } else if current_fraction < recovery_threshold {
            Self::Depleted
        } else if pressure_ratio > 1.5 {
            Self::Overharvested
        } else if pressure_ratio > 1.0 {
            Self::Stressed
        } else if current_fraction > 0.8 && pressure_ratio < 0.5 {
            Self::Thriving
        } else {
            Self::Sustainable
        }
    }

    #[must_use]
    pub fn is_healthy(self) -> bool {
        matches!(self, Self::Thriving | Self::Sustainable)
    }

    #[must_use]
    pub fn needs_recovery(self) -> bool {
        matches!(self, Self::Collapsed | Self::Depleted)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Collapsed => "collapsed",
            Self::Depleted => "depleted",
            Self::Overharvested => "overharvested",
            Self::Stressed => "stressed",
            Self::Sustainable => "sustainable",
            Self::Thriving => "thriving",
        }
    }
}

impl std::fmt::Display for SustainabilityRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Projected time to resource depletion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DepletionProjection {
    pub zone_id: ResourceZoneId,
    pub current_level: f32,
    pub capacity: f32,
    pub net_change_per_tick: f32,
    pub ticks_to_depletion: Option<u64>,
    pub confidence: f32,
}

impl DepletionProjection {
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn compute(
        zone_id: ResourceZoneId,
        current_level: f32,
        capacity: f32,
        regeneration_rate: f32,
        consumption_rate: f32,
    ) -> Self {
        let net_change = regeneration_rate - consumption_rate;
        let ticks_to_depletion = if net_change >= 0.0 {
            None
        } else {
            let ticks = (current_level / (-net_change)).ceil();
            if ticks.is_finite() && ticks >= 0.0 {
                Some(ticks as u64)
            } else {
                None
            }
        };

        let confidence = if consumption_rate > 0.0 { 0.8 } else { 0.95 };

        Self {
            zone_id,
            current_level,
            capacity,
            net_change_per_tick: net_change,
            ticks_to_depletion,
            confidence,
        }
    }

    #[must_use]
    pub fn is_depleting(&self) -> bool {
        self.net_change_per_tick < 0.0
    }
}

/// Projected time to resource recovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryProjection {
    pub zone_id: ResourceZoneId,
    pub current_level: f32,
    pub target_level: f32,
    pub recovery_rate: f32,
    pub ticks_to_recovery: Option<u64>,
    pub confidence: f32,
}

impl RecoveryProjection {
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn compute(
        zone_id: ResourceZoneId,
        current_level: f32,
        target_level: f32,
        regeneration_rate: f32,
        consumption_rate: f32,
    ) -> Self {
        let net_recovery = regeneration_rate - consumption_rate;
        let deficit = target_level - current_level;

        let ticks_to_recovery = if deficit <= 0.0 {
            Some(0)
        } else if net_recovery <= 0.0 {
            None
        } else {
            let ticks = (deficit / net_recovery).ceil();
            if ticks.is_finite() && ticks >= 0.0 {
                Some(ticks as u64)
            } else {
                None
            }
        };

        let confidence = if net_recovery > 0.0 { 0.7 } else { 0.3 };

        Self {
            zone_id,
            current_level,
            target_level,
            recovery_rate: net_recovery,
            ticks_to_recovery,
            confidence,
        }
    }

    #[must_use]
    pub fn is_recovering(&self) -> bool {
        self.recovery_rate > 0.0
    }

    #[must_use]
    pub fn already_recovered(&self) -> bool {
        self.current_level >= self.target_level
    }
}

/// Summary of sustainability state for a zone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SustainabilitySummary {
    pub zone_id: ResourceZoneId,
    pub rating: SustainabilityRating,
    pub current_fraction: f32,
    pub harvest_pressure: f32,
    pub projected_depletion_ticks: Option<u64>,
    pub projected_recovery_ticks: Option<u64>,
    pub tick: u64,
}

impl SustainabilitySummary {
    #[must_use]
    pub fn new(zone_id: ResourceZoneId, tick: u64) -> Self {
        Self {
            zone_id,
            rating: SustainabilityRating::Sustainable,
            current_fraction: 1.0,
            harvest_pressure: 0.0,
            projected_depletion_ticks: None,
            projected_recovery_ticks: None,
            tick,
        }
    }
}

/// Events generated by sustainability tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SustainabilityEventKind {
    ZoneDepleted {
        zone_id: ResourceZoneId,
    },
    ZoneRecovered {
        zone_id: ResourceZoneId,
    },
    OverharvestStarted {
        zone_id: ResourceZoneId,
        pressure: f32,
    },
    OverharvestEnded {
        zone_id: ResourceZoneId,
    },
    UnsustainablePressure {
        zone_id: ResourceZoneId,
        pressure: f32,
    },
    RecoveryBegan {
        zone_id: ResourceZoneId,
        projected_ticks: u64,
    },
}

/// A sustainability event with tick timestamp.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SustainabilityEvent {
    pub tick: u64,
    pub kind: SustainabilityEventKind,
}

impl SustainabilityEvent {
    #[must_use]
    pub fn new(tick: u64, kind: SustainabilityEventKind) -> Self {
        Self { tick, kind }
    }
}

/// Fingerprint for sustainability state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SustainabilityFingerprint(pub u32);

impl SustainabilityFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for SustainabilityFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sust:{:08x}", self.0)
    }
}

/// Result of a sustainability tick.
#[derive(Clone, Debug, Default)]
pub struct SustainabilityTickResult {
    pub events: Vec<SustainabilityEvent>,
    pub zones_updated: u32,
    pub overharvested_zones: u32,
    pub recovering_zones: u32,
}

/// Previous state for detecting transitions.
#[derive(Clone, Debug, Default)]
struct ZoneTrackingState {
    was_depleted: bool,
    was_overharvested: bool,
    was_recovering: bool,
}

/// Manager for sustainability tracking across zones.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SustainabilityTracker {
    policies: BTreeMap<ResourceZoneId, SustainabilityPolicy>,
    capacity_configs: BTreeMap<ResourceZoneId, CarryingCapacityConfig>,
    pressure_trackers: BTreeMap<ResourceZoneId, HarvestPressure>,
    summaries: BTreeMap<ResourceZoneId, SustainabilitySummary>,
    current_tick: u64,
    default_policy: SustainabilityPolicy,
    default_capacity_config: CarryingCapacityConfig,
    pressure_window: u64,
}

impl SustainabilityTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pressure_window: 100,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_pressure_window(mut self, window: u64) -> Self {
        self.pressure_window = window.max(1);
        self
    }

    #[must_use]
    pub fn with_default_policy(mut self, policy: SustainabilityPolicy) -> Self {
        self.default_policy = policy;
        self
    }

    #[must_use]
    pub fn with_default_capacity_config(mut self, config: CarryingCapacityConfig) -> Self {
        self.default_capacity_config = config;
        self
    }

    pub fn set_policy(&mut self, zone_id: ResourceZoneId, policy: SustainabilityPolicy) {
        self.policies.insert(zone_id, policy);
    }

    pub fn set_capacity_config(&mut self, zone_id: ResourceZoneId, config: CarryingCapacityConfig) {
        self.capacity_configs.insert(zone_id, config);
    }

    #[must_use]
    pub fn policy(&self, zone_id: &ResourceZoneId) -> &SustainabilityPolicy {
        self.policies.get(zone_id).unwrap_or(&self.default_policy)
    }

    #[must_use]
    pub fn capacity_config(&self, zone_id: &ResourceZoneId) -> &CarryingCapacityConfig {
        self.capacity_configs
            .get(zone_id)
            .unwrap_or(&self.default_capacity_config)
    }

    pub fn register_zone(&mut self, zone_id: ResourceZoneId, sustainable_limit: f32) {
        if !self.pressure_trackers.contains_key(&zone_id) {
            self.pressure_trackers.insert(
                zone_id.clone(),
                HarvestPressure::new(zone_id.clone(), self.pressure_window, sustainable_limit),
            );
            self.summaries
                .insert(zone_id.clone(), SustainabilitySummary::new(zone_id, 0));
        }
    }

    pub fn record_harvest(&mut self, zone_id: &ResourceZoneId, amount: f32) {
        if let Some(tracker) = self.pressure_trackers.get_mut(zone_id) {
            tracker.record_harvest(amount, self.current_tick);
        }
    }

    #[must_use]
    pub fn pressure(&self, zone_id: &ResourceZoneId) -> Option<&HarvestPressure> {
        self.pressure_trackers.get(zone_id)
    }

    #[must_use]
    pub fn summary(&self, zone_id: &ResourceZoneId) -> Option<&SustainabilitySummary> {
        self.summaries.get(zone_id)
    }

    #[must_use]
    pub fn rating(&self, zone_id: &ResourceZoneId) -> SustainabilityRating {
        self.summaries
            .get(zone_id)
            .map_or(SustainabilityRating::Sustainable, |s| s.rating)
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn project_depletion(
        &self,
        zone_id: &ResourceZoneId,
        current: f32,
        capacity: f32,
        renewal_rate: f32,
    ) -> DepletionProjection {
        let consumption_rate = self
            .pressure_trackers
            .get(zone_id)
            .map_or(0.0, HarvestPressure::average_consumption);
        DepletionProjection::compute(
            zone_id.clone(),
            current,
            capacity,
            renewal_rate,
            consumption_rate,
        )
    }

    #[must_use]
    pub fn project_recovery(
        &self,
        zone_id: &ResourceZoneId,
        current: f32,
        target: f32,
        renewal_rate: f32,
    ) -> RecoveryProjection {
        let consumption_rate = self
            .pressure_trackers
            .get(zone_id)
            .map_or(0.0, HarvestPressure::average_consumption);
        RecoveryProjection::compute(
            zone_id.clone(),
            current,
            target,
            renewal_rate,
            consumption_rate,
        )
    }

    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    pub fn update_zone(
        &mut self,
        zone_id: &ResourceZoneId,
        current: f32,
        capacity: f32,
        renewal_rate: f32,
        events: &mut Vec<SustainabilityEvent>,
    ) {
        let policy = self.policy(zone_id).clone();
        let current_fraction = if capacity > 0.0 {
            current / capacity
        } else {
            0.0
        };

        if let Some(tracker) = self.pressure_trackers.get_mut(zone_id) {
            tracker.tick(self.current_tick);
            let sustainable_yield = policy.compute_sustainable_yield(renewal_rate, capacity);
            tracker.set_sustainable_limit(sustainable_yield / self.pressure_window as f32);
        }

        let pressure_ratio = self
            .pressure_trackers
            .get(zone_id)
            .map_or(0.0, HarvestPressure::pressure_ratio);

        let prev_state = self
            .summaries
            .get(zone_id)
            .map_or_else(ZoneTrackingState::default, |s| ZoneTrackingState {
                was_depleted: s.rating.needs_recovery(),
                was_overharvested: matches!(
                    s.rating,
                    SustainabilityRating::Overharvested | SustainabilityRating::Stressed
                ),
                was_recovering: s.projected_recovery_ticks.is_some(),
            });

        let rating = SustainabilityRating::compute(
            current_fraction,
            pressure_ratio,
            policy.recovery_threshold,
        );

        let depletion_proj = self.project_depletion(zone_id, current, capacity, renewal_rate);
        let target = capacity * policy.recovered_threshold;
        let recovery_proj = self.project_recovery(zone_id, current, target, renewal_rate);

        let is_depleted = rating.needs_recovery();
        let is_overharvested = matches!(
            rating,
            SustainabilityRating::Overharvested | SustainabilityRating::Stressed
        );
        let is_recovering = rating.needs_recovery() && recovery_proj.is_recovering();

        if !prev_state.was_depleted && is_depleted {
            events.push(SustainabilityEvent::new(
                self.current_tick,
                SustainabilityEventKind::ZoneDepleted {
                    zone_id: zone_id.clone(),
                },
            ));
        }

        if prev_state.was_depleted && !is_depleted && current_fraction >= policy.recovered_threshold
        {
            events.push(SustainabilityEvent::new(
                self.current_tick,
                SustainabilityEventKind::ZoneRecovered {
                    zone_id: zone_id.clone(),
                },
            ));
        }

        if !prev_state.was_overharvested && is_overharvested {
            events.push(SustainabilityEvent::new(
                self.current_tick,
                SustainabilityEventKind::OverharvestStarted {
                    zone_id: zone_id.clone(),
                    pressure: pressure_ratio,
                },
            ));
        }

        if prev_state.was_overharvested && !is_overharvested {
            events.push(SustainabilityEvent::new(
                self.current_tick,
                SustainabilityEventKind::OverharvestEnded {
                    zone_id: zone_id.clone(),
                },
            ));
        }

        if pressure_ratio > 1.5 {
            events.push(SustainabilityEvent::new(
                self.current_tick,
                SustainabilityEventKind::UnsustainablePressure {
                    zone_id: zone_id.clone(),
                    pressure: pressure_ratio,
                },
            ));
        }

        if !prev_state.was_recovering
            && is_recovering
            && let Some(ticks) = recovery_proj.ticks_to_recovery
        {
            events.push(SustainabilityEvent::new(
                self.current_tick,
                SustainabilityEventKind::RecoveryBegan {
                    zone_id: zone_id.clone(),
                    projected_ticks: ticks,
                },
            ));
        }

        let summary = SustainabilitySummary {
            zone_id: zone_id.clone(),
            rating,
            current_fraction,
            harvest_pressure: pressure_ratio,
            projected_depletion_ticks: depletion_proj.ticks_to_depletion,
            projected_recovery_ticks: recovery_proj.ticks_to_recovery,
            tick: self.current_tick,
        };
        self.summaries.insert(zone_id.clone(), summary);
    }

    pub fn tick(&mut self) -> SustainabilityTickResult {
        self.current_tick += 1;
        let mut result = SustainabilityTickResult::default();

        for tracker in self.pressure_trackers.values_mut() {
            tracker.tick(self.current_tick);
        }

        for summary in self.summaries.values() {
            result.zones_updated += 1;
            if matches!(
                summary.rating,
                SustainabilityRating::Overharvested | SustainabilityRating::Stressed
            ) {
                result.overharvested_zones += 1;
            }
            if summary.rating.needs_recovery() && summary.projected_recovery_ticks.is_some() {
                result.recovering_zones += 1;
            }
        }

        result
    }

    #[must_use]
    pub fn fingerprint(&self) -> SustainabilityFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        hasher.update(&self.current_tick.to_le_bytes());

        for (id, summary) in &self.summaries {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&[summary.rating as u8]);
            hasher.update(&summary.current_fraction.to_le_bytes());
            hasher.update(&summary.harvest_pressure.to_le_bytes());
        }

        SustainabilityFingerprint(hasher.finalize())
    }

    pub fn overharvested_zones(&self) -> impl Iterator<Item = &ResourceZoneId> {
        self.summaries.iter().filter_map(|(id, s)| {
            if matches!(
                s.rating,
                SustainabilityRating::Overharvested | SustainabilityRating::Stressed
            ) {
                Some(id)
            } else {
                None
            }
        })
    }

    pub fn depleted_zones(&self) -> impl Iterator<Item = &ResourceZoneId> {
        self.summaries.iter().filter_map(|(id, s)| {
            if s.rating.needs_recovery() {
                Some(id)
            } else {
                None
            }
        })
    }

    pub fn healthy_zones(&self) -> impl Iterator<Item = &ResourceZoneId> {
        self.summaries.iter().filter_map(|(id, s)| {
            if s.rating.is_healthy() {
                Some(id)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone_id(name: &str) -> ResourceZoneId {
        ResourceZoneId::new(name)
    }

    #[test]
    fn test_regeneration_mode_constant() {
        let mode = RegenerationMode::Constant { rate: 2.0 };
        assert!((mode.compute_rate(500.0, 1000.0, 0) - 2.0).abs() < f32::EPSILON);
        assert!((mode.compute_rate(100.0, 1000.0, 2) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_regeneration_mode_logistic() {
        let mode = RegenerationMode::Logistic {
            rate: 10.0,
            inflection: 0.5,
        };
        let mid_rate = mode.compute_rate(500.0, 1000.0, 0);
        let low_rate = mode.compute_rate(100.0, 1000.0, 0);
        let high_rate = mode.compute_rate(900.0, 1000.0, 0);
        assert!(mid_rate > low_rate);
        assert!(mid_rate > high_rate);
    }

    #[test]
    fn test_regeneration_mode_seasonal() {
        let mode = RegenerationMode::Seasonal {
            base_rate: 1.0,
            season_multipliers: [1.0, 0.5, 2.0, 0.25],
        };
        assert!((mode.compute_rate(500.0, 1000.0, 0) - 1.0).abs() < f32::EPSILON);
        assert!((mode.compute_rate(500.0, 1000.0, 1) - 0.5).abs() < f32::EPSILON);
        assert!((mode.compute_rate(500.0, 1000.0, 2) - 2.0).abs() < f32::EPSILON);
        assert!((mode.compute_rate(500.0, 1000.0, 3) - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_regeneration_mode_disabled() {
        let mode = RegenerationMode::Disabled;
        assert!(mode.is_disabled());
        assert!((mode.compute_rate(500.0, 1000.0, 0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_depletion_behavior_linear() {
        let behavior = DepletionBehavior::Linear;
        assert!((behavior.effective_depletion(10.0, 0.5) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_depletion_behavior_accelerated() {
        let behavior = DepletionBehavior::Accelerated {
            threshold: 0.3,
            multiplier: 2.0,
        };
        assert!((behavior.effective_depletion(10.0, 0.5) - 10.0).abs() < f32::EPSILON);
        assert!((behavior.effective_depletion(10.0, 0.2) - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_depletion_behavior_buffered() {
        let behavior = DepletionBehavior::Buffered { buffer: 5.0 };
        assert!((behavior.effective_depletion(10.0, 0.5) - 5.0).abs() < f32::EPSILON);
        assert!((behavior.effective_depletion(3.0, 0.5) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sustainability_policy_builder() {
        let policy = SustainabilityPolicy::new()
            .with_regeneration_mode(RegenerationMode::Disabled)
            .with_overharvest_penalty(1.5)
            .with_recovery_threshold(0.3)
            .with_sustainable_yield(0.9);

        assert!(policy.regeneration_mode.is_disabled());
        assert!((policy.overharvest_penalty_rate - 1.5).abs() < f32::EPSILON);
        assert!((policy.recovery_threshold - 0.3).abs() < f32::EPSILON);
        assert!((policy.sustainable_yield_fraction - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sustainable_yield_computation() {
        let policy = SustainabilityPolicy::new().with_sustainable_yield(0.8);
        let yield_amount = policy.compute_sustainable_yield(1.0, 1000.0);
        assert!((yield_amount - 800.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_carrying_capacity_config() {
        let config = CarryingCapacityConfig::new()
            .with_headroom(0.2)
            .with_soft_cap(true)
            .with_soft_cap_rate(0.5);

        assert!((config.effective_capacity(1000.0) - 1200.0).abs() < f32::EPSILON);
        assert!((config.regeneration_modifier(0.5) - 1.0).abs() < f32::EPSILON);
        assert!(config.regeneration_modifier(0.9) < 1.0);
    }

    #[test]
    fn test_harvest_pressure_tracking() {
        let mut pressure = HarvestPressure::new(zone_id("test"), 10, 10.0);

        pressure.record_harvest(5.0, 1);
        pressure.record_harvest(5.0, 2);
        assert!((pressure.total_consumed() - 10.0).abs() < f32::EPSILON);
        assert!((pressure.average_consumption() - 1.0).abs() < f32::EPSILON);
        assert!((pressure.pressure_ratio() - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_harvest_pressure_overharvest() {
        let mut pressure = HarvestPressure::new(zone_id("test"), 10, 1.0);

        for tick in 1..=10 {
            pressure.record_harvest(2.0, tick);
        }

        assert!(pressure.is_overharvested());
    }

    #[test]
    fn test_harvest_pressure_window_sliding() {
        let mut pressure = HarvestPressure::new(zone_id("test"), 5, 10.0);

        pressure.record_harvest(10.0, 1);
        assert!((pressure.total_consumed() - 10.0).abs() < f32::EPSILON);

        pressure.tick(7);
        assert!(pressure.total_consumed() < 10.0);
    }

    #[test]
    fn test_sustainability_rating_compute() {
        assert_eq!(
            SustainabilityRating::compute(0.005, 0.0, 0.2),
            SustainabilityRating::Collapsed
        );
        assert_eq!(
            SustainabilityRating::compute(0.1, 0.0, 0.2),
            SustainabilityRating::Depleted
        );
        assert_eq!(
            SustainabilityRating::compute(0.5, 2.0, 0.2),
            SustainabilityRating::Overharvested
        );
        assert_eq!(
            SustainabilityRating::compute(0.5, 1.2, 0.2),
            SustainabilityRating::Stressed
        );
        assert_eq!(
            SustainabilityRating::compute(0.5, 0.5, 0.2),
            SustainabilityRating::Sustainable
        );
        assert_eq!(
            SustainabilityRating::compute(0.9, 0.3, 0.2),
            SustainabilityRating::Thriving
        );
    }

    #[test]
    fn test_sustainability_rating_predicates() {
        assert!(SustainabilityRating::Thriving.is_healthy());
        assert!(SustainabilityRating::Sustainable.is_healthy());
        assert!(!SustainabilityRating::Stressed.is_healthy());
        assert!(SustainabilityRating::Depleted.needs_recovery());
        assert!(SustainabilityRating::Collapsed.needs_recovery());
        assert!(!SustainabilityRating::Stressed.needs_recovery());
    }

    #[test]
    fn test_depletion_projection() {
        let proj = DepletionProjection::compute(zone_id("test"), 100.0, 1000.0, 1.0, 2.0);

        assert!(proj.is_depleting());
        assert!(proj.ticks_to_depletion.is_some());
        assert_eq!(proj.ticks_to_depletion.unwrap(), 100);
    }

    #[test]
    fn test_depletion_projection_sustainable() {
        let proj = DepletionProjection::compute(zone_id("test"), 500.0, 1000.0, 2.0, 1.0);

        assert!(!proj.is_depleting());
        assert!(proj.ticks_to_depletion.is_none());
    }

    #[test]
    fn test_recovery_projection() {
        let proj = RecoveryProjection::compute(zone_id("test"), 200.0, 700.0, 10.0, 5.0);

        assert!(proj.is_recovering());
        assert!(proj.ticks_to_recovery.is_some());
        assert_eq!(proj.ticks_to_recovery.unwrap(), 100);
    }

    #[test]
    fn test_recovery_projection_already_recovered() {
        let proj = RecoveryProjection::compute(zone_id("test"), 800.0, 700.0, 10.0, 5.0);

        assert!(proj.already_recovered());
        assert_eq!(proj.ticks_to_recovery, Some(0));
    }

    #[test]
    fn test_sustainability_tracker_basic() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("forest"), 10.0);

        assert!(tracker.pressure(&zone_id("forest")).is_some());
        assert!(tracker.summary(&zone_id("forest")).is_some());
    }

    #[test]
    fn test_sustainability_tracker_harvest_recording() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("forest"), 10.0);

        tracker.record_harvest(&zone_id("forest"), 5.0);
        tracker.record_harvest(&zone_id("forest"), 5.0);

        let pressure = tracker.pressure(&zone_id("forest")).unwrap();
        assert!((pressure.total_consumed() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sustainability_tracker_zone_update() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("forest"), 10.0);

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("forest"), 800.0, 1000.0, 1.0, &mut events);

        let summary = tracker.summary(&zone_id("forest")).unwrap();
        assert!((summary.current_fraction - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sustainability_tracker_depletion_event() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("forest"), 10.0);

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("forest"), 500.0, 1000.0, 1.0, &mut events);
        events.clear();

        tracker.update_zone(&zone_id("forest"), 50.0, 1000.0, 1.0, &mut events);

        assert!(events.iter().any(|e| matches!(
            &e.kind,
            SustainabilityEventKind::ZoneDepleted { zone_id } if zone_id.as_str() == "forest"
        )));
    }

    #[test]
    fn test_sustainability_tracker_recovery_event() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("forest"), 10.0);

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("forest"), 100.0, 1000.0, 1.0, &mut events);
        events.clear();

        tracker.update_zone(&zone_id("forest"), 800.0, 1000.0, 1.0, &mut events);

        assert!(events.iter().any(|e| matches!(
            &e.kind,
            SustainabilityEventKind::ZoneRecovered { zone_id } if zone_id.as_str() == "forest"
        )));
    }

    #[test]
    fn test_sustainability_tracker_overharvest_event() {
        let mut tracker = SustainabilityTracker::new().with_pressure_window(10);
        tracker.register_zone(zone_id("forest"), 1.0);

        for tick in 1..=10 {
            tracker.record_harvest(&zone_id("forest"), 100.0);
            tracker.current_tick = tick;
        }

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("forest"), 500.0, 1000.0, 1.0, &mut events);

        assert!(
            events
                .iter()
                .any(|e| matches!(&e.kind, SustainabilityEventKind::OverharvestStarted { .. }))
        );
    }

    #[test]
    fn test_sustainability_tracker_disabled_regeneration() {
        let mut tracker = SustainabilityTracker::new();
        let policy = SustainabilityPolicy::new().with_regeneration_mode(RegenerationMode::Disabled);
        tracker.set_policy(zone_id("desert"), policy);
        tracker.register_zone(zone_id("desert"), 0.0);

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("desert"), 500.0, 1000.0, 0.0, &mut events);

        let summary = tracker.summary(&zone_id("desert")).unwrap();
        assert!(
            summary.projected_recovery_ticks.is_none()
                || matches!(summary.rating, SustainabilityRating::Sustainable)
        );
    }

    #[test]
    fn test_sustainability_tracker_fingerprint_determinism() {
        let mut tracker1 = SustainabilityTracker::new();
        let mut tracker2 = SustainabilityTracker::new();

        tracker1.register_zone(zone_id("forest"), 10.0);
        tracker2.register_zone(zone_id("forest"), 10.0);

        let mut e1 = Vec::new();
        let mut e2 = Vec::new();

        tracker1.update_zone(&zone_id("forest"), 500.0, 1000.0, 1.0, &mut e1);
        tracker2.update_zone(&zone_id("forest"), 500.0, 1000.0, 1.0, &mut e2);

        tracker1.tick();
        tracker2.tick();

        assert_eq!(tracker1.fingerprint(), tracker2.fingerprint());
    }

    #[test]
    fn test_sustainability_fingerprint_display() {
        let fp = SustainabilityFingerprint(0xCAFE_BABE);
        assert_eq!(format!("{fp}"), "sust:cafebabe");
    }

    #[test]
    fn test_serde_sustainability_policy() {
        let policy = SustainabilityPolicy::new()
            .with_regeneration_mode(RegenerationMode::Logistic {
                rate: 5.0,
                inflection: 0.4,
            })
            .with_overharvest_penalty(1.2);

        let json = serde_json::to_string(&policy).unwrap();
        let restored: SustainabilityPolicy = serde_json::from_str(&json).unwrap();

        assert!((restored.overharvest_penalty_rate - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_harvest_pressure() {
        let mut pressure = HarvestPressure::new(zone_id("test"), 10, 5.0);
        pressure.record_harvest(3.0, 1);

        let json = serde_json::to_string(&pressure).unwrap();
        let restored: HarvestPressure = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.zone_id().as_str(), "test");
        assert!((restored.total_consumed() - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_sustainability_summary() {
        let summary = SustainabilitySummary {
            zone_id: zone_id("forest"),
            rating: SustainabilityRating::Stressed,
            current_fraction: 0.4,
            harvest_pressure: 1.3,
            projected_depletion_ticks: Some(50),
            projected_recovery_ticks: None,
            tick: 100,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: SustainabilitySummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.rating, SustainabilityRating::Stressed);
        assert_eq!(restored.projected_depletion_ticks, Some(50));
    }

    #[test]
    fn test_serde_sustainability_tracker() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("forest"), 10.0);
        tracker.record_harvest(&zone_id("forest"), 5.0);

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("forest"), 500.0, 1000.0, 1.0, &mut events);
        tracker.tick();

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: SustainabilityTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), tracker.current_tick());
        assert_eq!(restored.fingerprint(), tracker.fingerprint());
    }

    #[test]
    fn test_serde_sustainability_event() {
        let event = SustainabilityEvent::new(
            42,
            SustainabilityEventKind::OverharvestStarted {
                zone_id: zone_id("test"),
                pressure: 1.5,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        let restored: SustainabilityEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 42);
    }

    #[test]
    fn test_zone_queries() {
        let mut tracker = SustainabilityTracker::new();
        tracker.register_zone(zone_id("healthy"), 10.0);
        tracker.register_zone(zone_id("stressed"), 10.0);
        tracker.register_zone(zone_id("depleted"), 10.0);

        let mut events = Vec::new();
        tracker.update_zone(&zone_id("healthy"), 900.0, 1000.0, 1.0, &mut events);
        tracker.update_zone(&zone_id("stressed"), 500.0, 1000.0, 1.0, &mut events);
        tracker.update_zone(&zone_id("depleted"), 50.0, 1000.0, 1.0, &mut events);

        let healthy: Vec<_> = tracker.healthy_zones().collect();
        let depleted: Vec<_> = tracker.depleted_zones().collect();

        assert!(healthy.iter().any(|z| z.as_str() == "healthy"));
        assert!(depleted.iter().any(|z| z.as_str() == "depleted"));
    }
}

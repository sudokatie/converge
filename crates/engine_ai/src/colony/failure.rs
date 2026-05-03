//! Colony failure cascade system for crisis propagation and mitigation.

use super::ids::{FailureId, ResourceId, ShelterId, StorageNodeId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Severity level of a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FailureSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
    Catastrophic,
}

impl FailureSeverity {
    #[must_use]
    pub fn can_escalate(self) -> bool {
        !matches!(self, Self::Catastrophic)
    }

    #[must_use]
    pub fn escalate(self) -> Self {
        match self {
            Self::Minor => Self::Moderate,
            Self::Moderate => Self::Major,
            Self::Major => Self::Critical,
            Self::Critical | Self::Catastrophic => Self::Catastrophic,
        }
    }

    #[must_use]
    pub fn deescalate(self) -> Self {
        match self {
            Self::Minor | Self::Moderate => Self::Minor,
            Self::Major => Self::Moderate,
            Self::Critical => Self::Major,
            Self::Catastrophic => Self::Critical,
        }
    }

    #[must_use]
    pub fn cascade_probability(self) -> f32 {
        match self {
            Self::Minor => 0.1,
            Self::Moderate => 0.25,
            Self::Major => 0.5,
            Self::Critical => 0.75,
            Self::Catastrophic => 0.95,
        }
    }

    #[must_use]
    pub fn recovery_difficulty(self) -> f32 {
        match self {
            Self::Minor => 0.2,
            Self::Moderate => 0.4,
            Self::Major => 0.6,
            Self::Critical => 0.85,
            Self::Catastrophic => 1.0,
        }
    }

    #[must_use]
    pub fn priority_score(self) -> i32 {
        match self {
            Self::Minor => 10,
            Self::Moderate => 30,
            Self::Major => 60,
            Self::Critical => 90,
            Self::Catastrophic => 100,
        }
    }
}

/// Type of failure trigger.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureTrigger {
    ResourceShortage { resource: ResourceId, deficit: u32 },
    Overcrowding { shelter: ShelterId, excess: u32 },
    LowShelterValue { shelter: ShelterId, rating: u32 },
    BlockedLogistics { node: StorageNodeId },
    MoraleCrisis { level: u32 },
    PanicPressure { level: u32 },
    DependencyFailure { source: FailureId },
    SystemOverload { system: String },
    EnvironmentalHazard { hazard: String },
    ExternalThreat { threat: String },
}

impl FailureTrigger {
    #[must_use]
    pub fn base_severity(&self) -> FailureSeverity {
        match self {
            Self::ResourceShortage { deficit, .. } => {
                if *deficit > 100 {
                    FailureSeverity::Critical
                } else if *deficit > 50 {
                    FailureSeverity::Major
                } else if *deficit > 20 {
                    FailureSeverity::Moderate
                } else {
                    FailureSeverity::Minor
                }
            }
            Self::Overcrowding { excess, .. } => {
                if *excess > 50 {
                    FailureSeverity::Major
                } else if *excess > 20 {
                    FailureSeverity::Moderate
                } else {
                    FailureSeverity::Minor
                }
            }
            Self::LowShelterValue { rating, .. } => {
                if *rating < 20 {
                    FailureSeverity::Critical
                } else if *rating < 40 {
                    FailureSeverity::Major
                } else {
                    FailureSeverity::Moderate
                }
            }
            Self::BlockedLogistics { .. } | Self::SystemOverload { .. } => FailureSeverity::Major,
            Self::MoraleCrisis { level } => {
                if *level > 80 {
                    FailureSeverity::Critical
                } else if *level > 50 {
                    FailureSeverity::Major
                } else {
                    FailureSeverity::Moderate
                }
            }
            Self::PanicPressure { level } => {
                if *level > 80 {
                    FailureSeverity::Critical
                } else if *level > 60 {
                    FailureSeverity::Major
                } else {
                    FailureSeverity::Moderate
                }
            }
            Self::DependencyFailure { .. } => FailureSeverity::Moderate,
            Self::EnvironmentalHazard { .. } | Self::ExternalThreat { .. } => {
                FailureSeverity::Critical
            }
        }
    }

    #[must_use]
    pub fn can_cascade(&self) -> bool {
        !matches!(self, Self::DependencyFailure { .. })
    }
}

/// Status of a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureStatus {
    Active,
    Escalating,
    Contained,
    Mitigating,
    Resolved,
    Suppressed,
}

impl FailureStatus {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active | Self::Escalating)
    }

    #[must_use]
    pub fn is_resolved(self) -> bool {
        matches!(self, Self::Resolved | Self::Suppressed)
    }

    #[must_use]
    pub fn can_escalate(self) -> bool {
        matches!(self, Self::Active | Self::Escalating)
    }
}

/// A failure event in the colony.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Failure {
    pub id: FailureId,
    pub trigger: FailureTrigger,
    pub severity: FailureSeverity,
    pub status: FailureStatus,
    pub created_tick: u64,
    pub escalated_tick: Option<u64>,
    pub contained_tick: Option<u64>,
    pub resolved_tick: Option<u64>,
    pub cascade_source: Option<FailureId>,
    pub cascaded_to: BTreeSet<FailureId>,
    pub escalation_pressure: u32,
    pub mitigation_progress: f32,
    pub affected_population: u32,
    pub damage_accumulated: u32,
}

impl Failure {
    #[must_use]
    pub fn new(id: FailureId, trigger: FailureTrigger, created_tick: u64) -> Self {
        let severity = trigger.base_severity();
        Self {
            id,
            trigger,
            severity,
            status: FailureStatus::Active,
            created_tick,
            escalated_tick: None,
            contained_tick: None,
            resolved_tick: None,
            cascade_source: None,
            cascaded_to: BTreeSet::new(),
            escalation_pressure: 0,
            mitigation_progress: 0.0,
            affected_population: 0,
            damage_accumulated: 0,
        }
    }

    #[must_use]
    pub fn with_severity(mut self, severity: FailureSeverity) -> Self {
        self.severity = severity;
        self
    }

    #[must_use]
    pub fn with_source(mut self, source: FailureId) -> Self {
        self.cascade_source = Some(source);
        self
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> u64 {
        let end = self.resolved_tick.unwrap_or(current_tick);
        end.saturating_sub(self.created_tick)
    }

    #[must_use]
    pub fn is_cascaded(&self) -> bool {
        self.cascade_source.is_some()
    }

    #[must_use]
    pub fn has_cascaded(&self) -> bool {
        !self.cascaded_to.is_empty()
    }

    #[must_use]
    pub fn cascade_count(&self) -> usize {
        self.cascaded_to.len()
    }

    #[must_use]
    pub fn should_escalate(&self) -> bool {
        self.status.can_escalate()
            && self.severity.can_escalate()
            && self.escalation_pressure >= 100
    }

    pub fn add_escalation_pressure(&mut self, amount: u32) {
        if self.status.can_escalate() {
            self.escalation_pressure = self.escalation_pressure.saturating_add(amount);
        }
    }

    pub fn escalate(&mut self, tick: u64) {
        if self.severity.can_escalate() {
            self.severity = self.severity.escalate();
            self.escalated_tick = Some(tick);
            self.status = FailureStatus::Escalating;
            self.escalation_pressure = 0;
        }
    }

    pub fn contain(&mut self, tick: u64) {
        self.status = FailureStatus::Contained;
        self.contained_tick = Some(tick);
    }

    pub fn begin_mitigation(&mut self) {
        if self.status == FailureStatus::Contained {
            self.status = FailureStatus::Mitigating;
        }
    }

    pub fn add_mitigation(&mut self, amount: f32) -> bool {
        self.mitigation_progress = (self.mitigation_progress + amount).min(1.0);
        self.mitigation_progress >= 1.0
    }

    pub fn resolve(&mut self, tick: u64) {
        self.status = FailureStatus::Resolved;
        self.resolved_tick = Some(tick);
        self.mitigation_progress = 1.0;
    }

    pub fn suppress(&mut self, tick: u64) {
        self.status = FailureStatus::Suppressed;
        self.resolved_tick = Some(tick);
    }

    pub fn add_damage(&mut self, amount: u32) {
        self.damage_accumulated = self.damage_accumulated.saturating_add(amount);
    }

    pub fn add_affected(&mut self, count: u32) {
        self.affected_population = self.affected_population.saturating_add(count);
    }

    pub fn record_cascade(&mut self, target: FailureId) {
        self.cascaded_to.insert(target);
    }
}

/// Configuration for failure cascade behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CascadeConfig {
    pub max_cascade_depth: u32,
    pub cascade_delay_ticks: u64,
    pub escalation_threshold: u32,
    pub auto_mitigation_enabled: bool,
    pub mitigation_rate: f32,
    pub max_active_failures: usize,
    pub event_log_capacity: usize,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self {
            max_cascade_depth: 5,
            cascade_delay_ticks: 10,
            escalation_threshold: 100,
            auto_mitigation_enabled: false,
            mitigation_rate: 0.01,
            max_active_failures: 100,
            event_log_capacity: 500,
        }
    }
}

impl CascadeConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_cascade_depth(mut self, depth: u32) -> Self {
        self.max_cascade_depth = depth;
        self
    }

    #[must_use]
    pub fn with_event_log_capacity(mut self, capacity: usize) -> Self {
        self.event_log_capacity = capacity;
        self
    }
}

/// Event types for failure cascade system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureEvent {
    pub tick: u64,
    pub kind: FailureEventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FailureEventKind {
    FailureTriggered {
        failure: FailureId,
        severity: FailureSeverity,
    },
    FailureEscalated {
        failure: FailureId,
        from: FailureSeverity,
        to: FailureSeverity,
    },
    FailureCascaded {
        source: FailureId,
        target: FailureId,
    },
    FailureContained {
        failure: FailureId,
    },
    FailureMitigating {
        failure: FailureId,
        progress: f32,
    },
    FailureResolved {
        failure: FailureId,
        duration: u64,
    },
    FailureSuppressed {
        failure: FailureId,
    },
    CascadeBlocked {
        source: FailureId,
        reason: String,
    },
    MassResolution {
        count: u32,
    },
}

impl FailureEvent {
    #[must_use]
    pub fn new(tick: u64, kind: FailureEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn failure_triggered(tick: u64, failure: FailureId, severity: FailureSeverity) -> Self {
        Self::new(
            tick,
            FailureEventKind::FailureTriggered { failure, severity },
        )
    }

    #[must_use]
    pub fn failure_escalated(
        tick: u64,
        failure: FailureId,
        from: FailureSeverity,
        to: FailureSeverity,
    ) -> Self {
        Self::new(
            tick,
            FailureEventKind::FailureEscalated { failure, from, to },
        )
    }

    #[must_use]
    pub fn failure_cascaded(tick: u64, source: FailureId, target: FailureId) -> Self {
        Self::new(tick, FailureEventKind::FailureCascaded { source, target })
    }

    #[must_use]
    pub fn failure_resolved(tick: u64, failure: FailureId, duration: u64) -> Self {
        Self::new(
            tick,
            FailureEventKind::FailureResolved { failure, duration },
        )
    }

    #[must_use]
    pub fn involves_failure(&self, failure: FailureId) -> bool {
        match &self.kind {
            FailureEventKind::FailureTriggered { failure: f, .. }
            | FailureEventKind::FailureEscalated { failure: f, .. }
            | FailureEventKind::FailureContained { failure: f }
            | FailureEventKind::FailureMitigating { failure: f, .. }
            | FailureEventKind::FailureResolved { failure: f, .. }
            | FailureEventKind::FailureSuppressed { failure: f }
            | FailureEventKind::CascadeBlocked { source: f, .. } => *f == failure,
            FailureEventKind::FailureCascaded { source, target } => {
                *source == failure || *target == failure
            }
            FailureEventKind::MassResolution { .. } => false,
        }
    }
}

/// Registry for failure tracking.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FailureRegistry {
    failures: BTreeMap<FailureId, Failure>,
    next_id: u64,
}

impl FailureRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, trigger: FailureTrigger, created_tick: u64) -> FailureId {
        let id = FailureId::new(self.next_id);
        self.next_id += 1;
        let failure = Failure::new(id, trigger, created_tick);
        self.failures.insert(id, failure);
        id
    }

    pub fn register(&mut self, failure: Failure) {
        let id = failure.id;
        self.failures.insert(id, failure);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: FailureId) -> Option<Failure> {
        self.failures.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: FailureId) -> Option<&Failure> {
        self.failures.get(&id)
    }

    pub fn get_mut(&mut self, id: FailureId) -> Option<&mut Failure> {
        self.failures.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Failure> {
        self.failures.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Failure> {
        self.failures.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.failures.len()
    }

    pub fn active(&self) -> impl Iterator<Item = &Failure> {
        self.failures.values().filter(|f| f.status.is_active())
    }

    pub fn resolved(&self) -> impl Iterator<Item = &Failure> {
        self.failures.values().filter(|f| f.status.is_resolved())
    }

    pub fn by_severity(&self, severity: FailureSeverity) -> impl Iterator<Item = &Failure> {
        self.failures
            .values()
            .filter(move |f| f.severity == severity)
    }

    pub fn cascaded_from(&self, source: FailureId) -> impl Iterator<Item = &Failure> {
        self.failures
            .values()
            .filter(move |f| f.cascade_source == Some(source))
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active().count()
    }

    pub fn priority_sorted(&self) -> Vec<&Failure> {
        let mut active: Vec<_> = self.active().collect();
        active.sort_by(|a, b| {
            b.severity
                .priority_score()
                .cmp(&a.severity.priority_score())
                .then_with(|| a.created_tick.cmp(&b.created_tick))
                .then_with(|| a.id.cmp(&b.id))
        });
        active
    }

    #[expect(clippy::missing_panics_doc, reason = "ids come from resolved iterator")]
    pub fn cleanup_resolved(&mut self, max_resolved: usize) {
        let mut resolved: Vec<_> = self.resolved().map(|f| f.id).collect();
        if resolved.len() > max_resolved {
            resolved.sort_by(|a, b| {
                let fa = self.get(*a).expect("id from resolved");
                let fb = self.get(*b).expect("id from resolved");
                fa.resolved_tick.cmp(&fb.resolved_tick)
            });
            let to_remove = resolved.len() - max_resolved;
            for id in resolved.into_iter().take(to_remove) {
                self.failures.remove(&id);
            }
        }
    }
}

/// Mitigation action for a failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MitigationAction {
    pub name: String,
    pub description: String,
    pub effectiveness: f32,
    pub cost: u32,
    pub duration_ticks: u64,
    pub applicable_triggers: Vec<String>,
}

impl MitigationAction {
    #[must_use]
    pub fn new(name: impl Into<String>, effectiveness: f32) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            effectiveness: effectiveness.clamp(0.0, 1.0),
            cost: 0,
            duration_ticks: 10,
            applicable_triggers: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    #[must_use]
    pub fn with_cost(mut self, cost: u32) -> Self {
        self.cost = cost;
        self
    }

    #[must_use]
    pub fn with_duration(mut self, ticks: u64) -> Self {
        self.duration_ticks = ticks;
        self
    }
}

/// Suggest mitigations for a failure.
#[must_use]
#[expect(clippy::too_many_lines, reason = "match arms for each trigger type")]
pub fn suggest_mitigations(failure: &Failure) -> Vec<MitigationAction> {
    let mut actions = Vec::new();

    match &failure.trigger {
        FailureTrigger::ResourceShortage { resource, .. } => {
            actions.push(
                MitigationAction::new("Emergency Rationing", 0.6)
                    .with_description(format!("Reduce consumption of {}", resource.as_str()))
                    .with_duration(20),
            );
            actions.push(
                MitigationAction::new("Emergency Production", 0.8)
                    .with_description(format!("Prioritize production of {}", resource.as_str()))
                    .with_cost(50)
                    .with_duration(30),
            );
        }
        FailureTrigger::Overcrowding { .. } => {
            actions.push(
                MitigationAction::new("Emergency Relocation", 0.7)
                    .with_description("Move population to available shelters")
                    .with_duration(15),
            );
            actions.push(
                MitigationAction::new("Temporary Housing", 0.5)
                    .with_description("Deploy emergency shelters")
                    .with_cost(100)
                    .with_duration(25),
            );
        }
        FailureTrigger::LowShelterValue { .. } => {
            actions.push(
                MitigationAction::new("Emergency Repairs", 0.6)
                    .with_description("Perform critical repairs")
                    .with_cost(75)
                    .with_duration(20),
            );
            actions.push(
                MitigationAction::new("Evacuation", 0.9)
                    .with_description("Evacuate to safe shelter")
                    .with_duration(10),
            );
        }
        FailureTrigger::BlockedLogistics { .. } => {
            actions.push(
                MitigationAction::new("Clear Blockage", 0.8)
                    .with_description("Remove logistics obstruction")
                    .with_cost(30)
                    .with_duration(15),
            );
            actions.push(
                MitigationAction::new("Alternative Route", 0.6)
                    .with_description("Establish backup route")
                    .with_duration(25),
            );
        }
        FailureTrigger::MoraleCrisis { .. } | FailureTrigger::PanicPressure { .. } => {
            actions.push(
                MitigationAction::new("Public Address", 0.4)
                    .with_description("Calm the population")
                    .with_duration(5),
            );
            actions.push(
                MitigationAction::new("Emergency Supplies", 0.6)
                    .with_description("Distribute comfort supplies")
                    .with_cost(50)
                    .with_duration(10),
            );
        }
        FailureTrigger::DependencyFailure { .. } => {
            actions.push(
                MitigationAction::new("Address Root Cause", 0.7)
                    .with_description("Resolve the source failure first")
                    .with_duration(30),
            );
        }
        FailureTrigger::SystemOverload { .. } => {
            actions.push(
                MitigationAction::new("Load Shedding", 0.7)
                    .with_description("Reduce system load")
                    .with_duration(10),
            );
            actions.push(
                MitigationAction::new("System Restart", 0.9)
                    .with_description("Restart affected systems")
                    .with_cost(20)
                    .with_duration(15),
            );
        }
        FailureTrigger::EnvironmentalHazard { .. } | FailureTrigger::ExternalThreat { .. } => {
            actions.push(
                MitigationAction::new("Shelter In Place", 0.6)
                    .with_description("Protect population in shelters")
                    .with_duration(30),
            );
            actions.push(
                MitigationAction::new("Emergency Response", 0.8)
                    .with_description("Deploy response teams")
                    .with_cost(100)
                    .with_duration(20),
            );
        }
    }

    actions.sort_by(|a, b| {
        b.effectiveness
            .partial_cmp(&a.effectiveness)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    actions
}

/// Summary of failure cascade state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FailureSummary {
    pub tick: u64,
    pub total_failures: u32,
    pub active_failures: u32,
    pub resolved_failures: u32,
    pub by_severity: BTreeMap<String, u32>,
    pub total_cascades: u32,
    pub total_damage: u32,
    pub total_affected: u32,
    pub average_resolution_time: f32,
}

impl FailureSummary {
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
    pub fn from_registry(registry: &FailureRegistry, tick: u64) -> Self {
        let total = registry.count() as u32;
        let active = registry.active_count() as u32;
        let resolved = registry.resolved().count() as u32;

        let mut by_severity: BTreeMap<String, u32> = BTreeMap::new();
        for failure in registry.iter() {
            *by_severity
                .entry(format!("{:?}", failure.severity))
                .or_insert(0) += 1;
        }

        let total_cascades = registry.iter().map(Failure::cascade_count).sum::<usize>() as u32;

        let total_damage: u32 = registry.iter().map(|f| f.damage_accumulated).sum();
        let total_affected: u32 = registry.iter().map(|f| f.affected_population).sum();

        let resolved_with_time: Vec<_> = registry
            .resolved()
            .filter_map(|f| f.resolved_tick.map(|t| t.saturating_sub(f.created_tick)))
            .collect();

        let avg_resolution = if resolved_with_time.is_empty() {
            0.0
        } else {
            let sum: u64 = resolved_with_time.iter().sum();
            sum as f32 / resolved_with_time.len() as f32
        };

        Self {
            tick,
            total_failures: total,
            active_failures: active,
            resolved_failures: resolved,
            by_severity,
            total_cascades,
            total_damage,
            total_affected,
            average_resolution_time: avg_resolution,
        }
    }

    #[must_use]
    pub fn stability_score(&self) -> f32 {
        if self.total_failures == 0 {
            return 1.0;
        }
        #[expect(clippy::cast_precision_loss, reason = "bounded values")]
        {
            1.0 - (self.active_failures as f32 / self.total_failures as f32).min(1.0)
        }
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.total_failures.to_le_bytes());
        hasher.update(&self.active_failures.to_le_bytes());
        hasher.update(&self.total_cascades.to_le_bytes());
        hasher.update(&self.total_damage.to_le_bytes());
        hasher.finalize()
    }
}

/// Projection of future failure state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureProjection {
    pub base_tick: u64,
    pub projected_tick: u64,
    pub estimated_resolutions: u32,
    pub estimated_escalations: u32,
    pub estimated_cascades: u32,
    pub risk_score: f32,
    pub confidence: f32,
}

impl FailureProjection {
    #[must_use]
    pub fn new(base_tick: u64, projected_tick: u64) -> Self {
        Self {
            base_tick,
            projected_tick,
            estimated_resolutions: 0,
            estimated_escalations: 0,
            estimated_cascades: 0,
            risk_score: 0.0,
            confidence: 1.0,
        }
    }

    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "bounded values"
    )]
    pub fn from_registry(registry: &FailureRegistry, base_tick: u64, projected_tick: u64) -> Self {
        let ticks_ahead = projected_tick.saturating_sub(base_tick);
        let active: Vec<_> = registry.active().collect();

        let estimated_resolutions = active
            .iter()
            .filter(|f| f.mitigation_progress > 0.5)
            .count() as u32;

        let estimated_escalations = active.iter().filter(|f| f.should_escalate()).count() as u32;

        let cascade_risk: f32 = active
            .iter()
            .filter(|f| f.trigger.can_cascade())
            .map(|f| f.severity.cascade_probability())
            .sum();

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "risk is non-negative sum and bounded"
        )]
        let estimated_cascades = (cascade_risk * 0.5) as u32;

        let risk = if active.is_empty() {
            0.0
        } else {
            let severity_sum: f32 = active
                .iter()
                .map(|f| f.severity.priority_score() as f32 / 100.0)
                .sum();
            (severity_sum / active.len() as f32).min(1.0)
        };

        let confidence = if ticks_ahead > 1000 { 0.3 } else { 0.8 };

        Self {
            base_tick,
            projected_tick,
            estimated_resolutions,
            estimated_escalations,
            estimated_cascades,
            risk_score: risk,
            confidence,
        }
    }
}

/// Fingerprint for failure state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FailureFingerprint(pub u32);

impl FailureFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn from_registry(registry: &FailureRegistry, tick: u64) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&tick.to_le_bytes());
        hasher.update(&(registry.count() as u64).to_le_bytes());
        hasher.update(&(registry.active_count() as u64).to_le_bytes());

        for failure in registry.iter() {
            hasher.update(&failure.id.raw().to_le_bytes());
            hasher.update(
                &failure
                    .severity
                    .priority_score()
                    .unsigned_abs()
                    .to_le_bytes(),
            );
            hasher.update(&u8::from(failure.status.is_active()).to_le_bytes());
        }

        Self(hasher.finalize())
    }
}

impl std::fmt::Display for FailureFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failure:{:08x}", self.0)
    }
}

/// Bounded event log for failure events.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FailureEventLog {
    events: VecDeque<FailureEvent>,
    capacity: usize,
}

impl FailureEventLog {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, event: FailureEvent) {
        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &FailureEvent> {
        self.events.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn for_failure(&self, failure: FailureId) -> impl Iterator<Item = &FailureEvent> {
        self.events
            .iter()
            .filter(move |e| e.involves_failure(failure))
    }

    pub fn since_tick(&self, tick: u64) -> impl Iterator<Item = &FailureEvent> {
        self.events.iter().filter(move |e| e.tick >= tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_escalation() {
        let minor = FailureSeverity::Minor;
        assert!(minor.can_escalate());
        assert_eq!(minor.escalate(), FailureSeverity::Moderate);

        let catastrophic = FailureSeverity::Catastrophic;
        assert!(!catastrophic.can_escalate());
        assert_eq!(catastrophic.escalate(), FailureSeverity::Catastrophic);
    }

    #[test]
    fn test_severity_deescalation() {
        let critical = FailureSeverity::Critical;
        assert_eq!(critical.deescalate(), FailureSeverity::Major);

        let minor = FailureSeverity::Minor;
        assert_eq!(minor.deescalate(), FailureSeverity::Minor);
    }

    #[test]
    fn test_failure_trigger_severity() {
        let minor_shortage = FailureTrigger::ResourceShortage {
            resource: ResourceId::new("food"),
            deficit: 10,
        };
        assert_eq!(minor_shortage.base_severity(), FailureSeverity::Minor);

        let major_shortage = FailureTrigger::ResourceShortage {
            resource: ResourceId::new("food"),
            deficit: 75,
        };
        assert_eq!(major_shortage.base_severity(), FailureSeverity::Major);
    }

    #[test]
    fn test_failure_lifecycle() {
        let trigger = FailureTrigger::ResourceShortage {
            resource: ResourceId::new("water"),
            deficit: 50,
        };
        let mut failure = Failure::new(FailureId::new(1), trigger, 0);

        assert_eq!(failure.status, FailureStatus::Active);
        assert!(failure.status.is_active());

        failure.contain(10);
        assert_eq!(failure.status, FailureStatus::Contained);

        failure.begin_mitigation();
        assert_eq!(failure.status, FailureStatus::Mitigating);

        failure.add_mitigation(0.5);
        assert!(!failure.add_mitigation(0.4));
        assert!(failure.add_mitigation(0.2));

        failure.resolve(20);
        assert!(failure.status.is_resolved());
        assert_eq!(failure.duration(20), 20);
    }

    #[test]
    fn test_failure_escalation() {
        let trigger = FailureTrigger::MoraleCrisis { level: 60 };
        let mut failure = Failure::new(FailureId::new(1), trigger, 0);

        assert!(!failure.should_escalate());

        failure.add_escalation_pressure(100);
        assert!(failure.should_escalate());

        let old_severity = failure.severity;
        failure.escalate(10);
        assert_eq!(failure.severity, old_severity.escalate());
        assert_eq!(failure.escalation_pressure, 0);
    }

    #[test]
    fn test_failure_cascade_tracking() {
        let mut failure = Failure::new(
            FailureId::new(1),
            FailureTrigger::SystemOverload {
                system: "power".into(),
            },
            0,
        );

        assert!(!failure.has_cascaded());

        failure.record_cascade(FailureId::new(2));
        failure.record_cascade(FailureId::new(3));

        assert!(failure.has_cascaded());
        assert_eq!(failure.cascade_count(), 2);
    }

    #[test]
    fn test_failure_registry() {
        let mut registry = FailureRegistry::new();

        let id1 = registry.create(
            FailureTrigger::Overcrowding {
                shelter: ShelterId::new(1),
                excess: 30,
            },
            0,
        );

        let _id2 = registry.create(
            FailureTrigger::BlockedLogistics {
                node: StorageNodeId::new(1),
            },
            5,
        );

        assert_eq!(registry.count(), 2);
        assert_eq!(registry.active_count(), 2);

        registry.get_mut(id1).unwrap().resolve(10);
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn test_failure_registry_priority_sorted() {
        let mut registry = FailureRegistry::new();

        registry.create(
            FailureTrigger::ResourceShortage {
                resource: ResourceId::new("food"),
                deficit: 10,
            },
            0,
        );

        registry.create(
            FailureTrigger::EnvironmentalHazard {
                hazard: "radiation".into(),
            },
            0,
        );

        let sorted = registry.priority_sorted();
        assert!(sorted[0].severity >= sorted[1].severity);
    }

    #[test]
    fn test_suggest_mitigations() {
        let failure = Failure::new(
            FailureId::new(1),
            FailureTrigger::ResourceShortage {
                resource: ResourceId::new("oxygen"),
                deficit: 100,
            },
            0,
        );

        let mitigations = suggest_mitigations(&failure);
        assert!(!mitigations.is_empty());
        assert!(mitigations[0].effectiveness >= mitigations.last().unwrap().effectiveness);
    }

    #[test]
    fn test_failure_summary() {
        let mut registry = FailureRegistry::new();

        registry.create(FailureTrigger::MoraleCrisis { level: 50 }, 0);

        let id = registry.create(FailureTrigger::PanicPressure { level: 70 }, 5);

        registry.get_mut(id).unwrap().resolve(15);

        let summary = FailureSummary::from_registry(&registry, 20);

        assert_eq!(summary.total_failures, 2);
        assert_eq!(summary.active_failures, 1);
        assert_eq!(summary.resolved_failures, 1);
        assert!(summary.stability_score() > 0.0);
    }

    #[test]
    fn test_failure_projection() {
        let mut registry = FailureRegistry::new();

        registry.create(
            FailureTrigger::SystemOverload {
                system: "life_support".into(),
            },
            0,
        );

        let projection = FailureProjection::from_registry(&registry, 0, 100);

        assert_eq!(projection.base_tick, 0);
        assert_eq!(projection.projected_tick, 100);
        assert!(projection.confidence > 0.0);
    }

    #[test]
    fn test_failure_fingerprint() {
        let mut registry = FailureRegistry::new();
        registry.create(
            FailureTrigger::ExternalThreat {
                threat: "meteor".into(),
            },
            0,
        );

        let fp1 = FailureFingerprint::from_registry(&registry, 0);
        let fp2 = FailureFingerprint::from_registry(&registry, 0);

        assert_eq!(fp1, fp2);
        assert_eq!(format!("{fp1}"), format!("failure:{:08x}", fp1.raw()));
    }

    #[test]
    fn test_failure_event_log() {
        let mut log = FailureEventLog::new(3);

        log.push(FailureEvent::failure_triggered(
            0,
            FailureId::new(1),
            FailureSeverity::Minor,
        ));
        log.push(FailureEvent::failure_triggered(
            1,
            FailureId::new(2),
            FailureSeverity::Major,
        ));
        log.push(FailureEvent::failure_triggered(
            2,
            FailureId::new(3),
            FailureSeverity::Critical,
        ));
        log.push(FailureEvent::failure_triggered(
            3,
            FailureId::new(4),
            FailureSeverity::Minor,
        ));

        assert_eq!(log.len(), 3);

        let first = log.iter().next().unwrap();
        assert_eq!(first.tick, 1);
    }

    #[test]
    fn test_event_log_filtering() {
        let mut log = FailureEventLog::new(10);

        log.push(FailureEvent::failure_triggered(
            0,
            FailureId::new(1),
            FailureSeverity::Minor,
        ));
        log.push(FailureEvent::failure_escalated(
            5,
            FailureId::new(1),
            FailureSeverity::Minor,
            FailureSeverity::Moderate,
        ));
        log.push(FailureEvent::failure_resolved(10, FailureId::new(1), 10));
        log.push(FailureEvent::failure_triggered(
            3,
            FailureId::new(2),
            FailureSeverity::Major,
        ));

        let failure1_events: Vec<_> = log.for_failure(FailureId::new(1)).collect();
        assert_eq!(failure1_events.len(), 3);

        let recent_events: Vec<_> = log.since_tick(5).collect();
        assert_eq!(recent_events.len(), 2);
    }

    #[test]
    fn test_cascade_config() {
        let config = CascadeConfig::new()
            .with_max_cascade_depth(3)
            .with_event_log_capacity(1000);

        assert_eq!(config.max_cascade_depth, 3);
        assert_eq!(config.event_log_capacity, 1000);
    }

    #[test]
    fn test_serde_failure() {
        let failure = Failure::new(
            FailureId::new(42),
            FailureTrigger::ResourceShortage {
                resource: ResourceId::new("fuel"),
                deficit: 50,
            },
            100,
        )
        .with_severity(FailureSeverity::Major);

        let json = serde_json::to_string(&failure).unwrap();
        let restored: Failure = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, failure.id);
        assert_eq!(restored.severity, FailureSeverity::Major);
    }

    #[test]
    fn test_serde_summary() {
        let summary = FailureSummary {
            tick: 500,
            total_failures: 10,
            active_failures: 3,
            resolved_failures: 7,
            by_severity: BTreeMap::new(),
            total_cascades: 2,
            total_damage: 100,
            total_affected: 50,
            average_resolution_time: 25.0,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: FailureSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, summary);
    }

    #[test]
    fn test_bincode_failure() {
        let failure = Failure::new(
            FailureId::new(99),
            FailureTrigger::PanicPressure { level: 80 },
            200,
        );

        let bytes = bincode::serialize(&failure).unwrap();
        let restored: Failure = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id.raw(), 99);
        assert_eq!(restored.created_tick, 200);
    }

    #[test]
    fn test_bincode_event() {
        let event = FailureEvent::failure_cascaded(100, FailureId::new(1), FailureId::new(2));

        let bytes = bincode::serialize(&event).unwrap();
        let restored: FailureEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 100);
    }

    #[test]
    fn test_bincode_summary() {
        let mut registry = FailureRegistry::new();
        registry.create(FailureTrigger::MoraleCrisis { level: 50 }, 0);
        let summary = FailureSummary::from_registry(&registry, 100);

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: FailureSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 100);
        assert_eq!(restored.total_failures, 1);
    }

    #[test]
    fn test_bincode_projection() {
        let projection = FailureProjection::new(100, 500);

        let bytes = bincode::serialize(&projection).unwrap();
        let restored: FailureProjection = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.base_tick, 100);
        assert_eq!(restored.projected_tick, 500);
    }

    #[test]
    fn test_bincode_fingerprint() {
        let fp = FailureFingerprint(0xDEAD_BEEF);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: FailureFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.raw(), 0xDEAD_BEEF);
    }

    #[test]
    fn test_mitigation_action() {
        let action = MitigationAction::new("Test Action", 0.75)
            .with_description("Test description")
            .with_cost(100)
            .with_duration(20);

        assert_eq!(action.name, "Test Action");
        assert!((action.effectiveness - 0.75).abs() < 0.001);
        assert_eq!(action.cost, 100);
        assert_eq!(action.duration_ticks, 20);
    }
}

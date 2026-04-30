//! State types for the emergency response system.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::ids::{
    ContainmentZoneId, EmergencyId, EmergencyTypeId, RegionId, ResponderId, ResponderRoleId,
    ResponseActionId, ResponsePlanId, ShelterZoneId,
};

/// Severity level of an emergency.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum EmergencySeverity {
    #[default]
    Minor,
    Moderate,
    Major,
    Critical,
    Catastrophic,
}

impl EmergencySeverity {
    #[must_use]
    pub fn escalation_threshold(self) -> u32 {
        match self {
            Self::Minor => 50,
            Self::Moderate => 100,
            Self::Major => 200,
            Self::Critical => 400,
            Self::Catastrophic => u32::MAX,
        }
    }

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
    pub fn responder_multiplier(self) -> f32 {
        match self {
            Self::Minor => 1.0,
            Self::Moderate => 1.5,
            Self::Major => 2.0,
            Self::Critical => 3.0,
            Self::Catastrophic => 5.0,
        }
    }

    #[must_use]
    pub fn priority_score(self) -> u32 {
        match self {
            Self::Minor => 10,
            Self::Moderate => 30,
            Self::Major => 60,
            Self::Critical => 90,
            Self::Catastrophic => 100,
        }
    }
}

/// Type of emergency incident.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EmergencyKind {
    Fire,
    Breach,
    Stampede,
    Infestation,
}

impl EmergencyKind {
    #[must_use]
    pub fn default_severity(self) -> EmergencySeverity {
        match self {
            Self::Breach => EmergencySeverity::Major,
            Self::Fire | Self::Stampede => EmergencySeverity::Moderate,
            Self::Infestation => EmergencySeverity::Minor,
        }
    }

    #[must_use]
    pub fn spread_rate(self) -> f32 {
        match self {
            Self::Fire => 1.5,
            Self::Breach => 0.0,
            Self::Stampede => 2.0,
            Self::Infestation => 0.8,
        }
    }

    #[must_use]
    pub fn requires_evacuation(self) -> bool {
        matches!(self, Self::Fire | Self::Breach | Self::Stampede)
    }

    #[must_use]
    pub fn requires_containment(self) -> bool {
        matches!(self, Self::Fire | Self::Infestation)
    }

    #[must_use]
    pub fn requires_suppression(self) -> bool {
        matches!(self, Self::Fire | Self::Infestation)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fire => "fire",
            Self::Breach => "breach",
            Self::Stampede => "stampede",
            Self::Infestation => "infestation",
        }
    }
}

/// Status of an emergency incident.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum EmergencyStatus {
    #[default]
    Detected,
    Assessed,
    ResponseActive,
    Contained,
    Resolved,
    Escalated,
}

impl EmergencyStatus {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Detected | Self::Assessed | Self::ResponseActive | Self::Escalated
        )
    }

    #[must_use]
    pub fn is_resolved(self) -> bool {
        matches!(self, Self::Resolved)
    }

    #[must_use]
    pub fn can_assign_responders(self) -> bool {
        matches!(
            self,
            Self::Detected | Self::Assessed | Self::ResponseActive | Self::Escalated
        )
    }
}

/// An emergency incident.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Emergency {
    pub id: EmergencyId,
    pub kind: EmergencyKind,
    pub type_id: EmergencyTypeId,
    pub status: EmergencyStatus,
    pub severity: EmergencySeverity,
    pub origin_region: RegionId,
    pub affected_regions: BTreeSet<RegionId>,
    pub intensity: f32,
    pub spread_progress: f32,
    pub containment_level: f32,
    pub damage_accumulated: u32,
    pub casualties: u32,
    pub detected_tick: u64,
    pub assessed_tick: Option<u64>,
    pub resolved_tick: Option<u64>,
    pub escalation_pressure: u32,
    pub active_plan: Option<ResponsePlanId>,
    pub assigned_responders: BTreeSet<ResponderId>,
}

impl Emergency {
    #[must_use]
    pub fn new(
        id: EmergencyId,
        kind: EmergencyKind,
        origin_region: RegionId,
        detected_tick: u64,
    ) -> Self {
        let mut affected_regions = BTreeSet::new();
        affected_regions.insert(origin_region);

        Self {
            id,
            kind,
            type_id: EmergencyTypeId::new(kind.as_str()),
            status: EmergencyStatus::Detected,
            severity: kind.default_severity(),
            origin_region,
            affected_regions,
            intensity: 1.0,
            spread_progress: 0.0,
            containment_level: 0.0,
            damage_accumulated: 0,
            casualties: 0,
            detected_tick,
            assessed_tick: None,
            resolved_tick: None,
            escalation_pressure: 0,
            active_plan: None,
            assigned_responders: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn with_severity(mut self, severity: EmergencySeverity) -> Self {
        self.severity = severity;
        self
    }

    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 10.0);
        self
    }

    pub fn assess(&mut self, tick: u64) {
        if self.status == EmergencyStatus::Detected {
            self.status = EmergencyStatus::Assessed;
            self.assessed_tick = Some(tick);
        }
    }

    pub fn activate_response(&mut self, plan: ResponsePlanId) {
        if matches!(
            self.status,
            EmergencyStatus::Detected | EmergencyStatus::Assessed
        ) {
            self.status = EmergencyStatus::ResponseActive;
            self.active_plan = Some(plan);
        }
    }

    pub fn escalate(&mut self) {
        if self.severity.can_escalate() {
            self.severity = self.severity.escalate();
            self.status = EmergencyStatus::Escalated;
            self.escalation_pressure = 0;
        }
    }

    pub fn contain(&mut self) {
        if self.status.is_active() {
            self.status = EmergencyStatus::Contained;
        }
    }

    pub fn resolve(&mut self, tick: u64) {
        self.status = EmergencyStatus::Resolved;
        self.resolved_tick = Some(tick);
        self.intensity = 0.0;
    }

    pub fn assign_responder(&mut self, responder: ResponderId) {
        self.assigned_responders.insert(responder);
    }

    pub fn unassign_responder(&mut self, responder: ResponderId) {
        self.assigned_responders.remove(&responder);
    }

    pub fn spread_to(&mut self, region: RegionId) {
        self.affected_regions.insert(region);
    }

    pub fn add_damage(&mut self, damage: u32) {
        self.damage_accumulated = self.damage_accumulated.saturating_add(damage);
    }

    pub fn add_casualties(&mut self, count: u32) {
        self.casualties = self.casualties.saturating_add(count);
    }

    pub fn add_escalation_pressure(&mut self, pressure: u32) {
        self.escalation_pressure = self.escalation_pressure.saturating_add(pressure);
    }

    pub fn add_containment(&mut self, amount: f32) {
        self.containment_level = (self.containment_level + amount).clamp(0.0, 1.0);
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.status.is_resolved()
    }

    #[must_use]
    pub fn should_escalate(&self) -> bool {
        self.severity.can_escalate()
            && self.escalation_pressure >= self.severity.escalation_threshold()
    }

    #[must_use]
    pub fn is_contained(&self) -> bool {
        self.containment_level >= 1.0 || self.status == EmergencyStatus::Contained
    }

    #[must_use]
    pub fn responder_count(&self) -> usize {
        self.assigned_responders.len()
    }

    #[must_use]
    pub fn region_count(&self) -> usize {
        self.affected_regions.len()
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> u64 {
        if let Some(resolved) = self.resolved_tick {
            resolved.saturating_sub(self.detected_tick)
        } else {
            current_tick.saturating_sub(self.detected_tick)
        }
    }
}

/// Status of a responder.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ResponderStatus {
    #[default]
    Available,
    Assigned,
    EnRoute,
    OnScene,
    Performing,
    Incapacitated,
    Resting,
}

impl ResponderStatus {
    #[must_use]
    pub fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }

    #[must_use]
    pub fn is_deployable(self) -> bool {
        matches!(self, Self::Available | Self::Resting)
    }

    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Assigned | Self::EnRoute | Self::OnScene | Self::Performing
        )
    }

    #[must_use]
    pub fn can_perform_action(self) -> bool {
        matches!(self, Self::OnScene | Self::Performing)
    }
}

/// An emergency responder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Responder {
    pub id: ResponderId,
    pub name: String,
    pub status: ResponderStatus,
    pub roles: BTreeSet<ResponderRoleId>,
    pub current_region: Option<RegionId>,
    pub target_region: Option<RegionId>,
    pub assigned_emergency: Option<EmergencyId>,
    pub current_action: Option<ResponseActionId>,
    pub effectiveness: f32,
    pub fatigue: f32,
    pub created_tick: u64,
    pub total_responses: u32,
    pub total_actions_completed: u32,
}

impl Responder {
    #[must_use]
    pub fn new(id: ResponderId, name: impl Into<String>, created_tick: u64) -> Self {
        Self {
            id,
            name: name.into(),
            status: ResponderStatus::Available,
            roles: BTreeSet::new(),
            current_region: None,
            target_region: None,
            assigned_emergency: None,
            current_action: None,
            effectiveness: 1.0,
            fatigue: 0.0,
            created_tick,
            total_responses: 0,
            total_actions_completed: 0,
        }
    }

    #[must_use]
    pub fn with_region(mut self, region: RegionId) -> Self {
        self.current_region = Some(region);
        self
    }

    #[must_use]
    pub fn with_role(mut self, role: ResponderRoleId) -> Self {
        self.roles.insert(role);
        self
    }

    #[must_use]
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = ResponderRoleId>) -> Self {
        self.roles.extend(roles);
        self
    }

    pub fn assign_to(&mut self, emergency: EmergencyId, target_region: RegionId) {
        self.assigned_emergency = Some(emergency);
        self.target_region = Some(target_region);
        self.status = ResponderStatus::Assigned;
        self.total_responses += 1;
    }

    pub fn set_en_route(&mut self) {
        if self.status == ResponderStatus::Assigned {
            self.status = ResponderStatus::EnRoute;
        }
    }

    pub fn arrive_on_scene(&mut self) {
        if self.status == ResponderStatus::EnRoute {
            self.status = ResponderStatus::OnScene;
            self.current_region = self.target_region;
        }
    }

    pub fn start_action(&mut self, action: ResponseActionId) {
        if self.status == ResponderStatus::OnScene {
            self.status = ResponderStatus::Performing;
            self.current_action = Some(action);
        }
    }

    pub fn complete_action(&mut self) {
        if self.status == ResponderStatus::Performing {
            self.status = ResponderStatus::OnScene;
            self.current_action = None;
            self.total_actions_completed += 1;
        }
    }

    pub fn release(&mut self) {
        self.assigned_emergency = None;
        self.target_region = None;
        self.current_action = None;
        self.status = ResponderStatus::Available;
    }

    pub fn incapacitate(&mut self) {
        self.status = ResponderStatus::Incapacitated;
        self.current_action = None;
    }

    pub fn recover(&mut self) {
        if self.status == ResponderStatus::Incapacitated {
            self.status = ResponderStatus::Resting;
            self.fatigue = 0.5;
        }
    }

    pub fn rest(&mut self) {
        if self.status == ResponderStatus::Resting && self.fatigue <= 0.0 {
            self.status = ResponderStatus::Available;
        }
    }

    pub fn add_fatigue(&mut self, amount: f32) {
        self.fatigue = (self.fatigue + amount).clamp(0.0, 1.0);
    }

    pub fn reduce_fatigue(&mut self, amount: f32) {
        self.fatigue = (self.fatigue - amount).max(0.0);
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.status.is_available()
    }

    #[must_use]
    pub fn is_deployable(&self) -> bool {
        self.status.is_deployable() && self.fatigue < 0.9
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    #[must_use]
    pub fn has_role(&self, role: &ResponderRoleId) -> bool {
        self.roles.contains(role)
    }

    #[must_use]
    pub fn effective_work_rate(&self) -> f32 {
        self.effectiveness * (1.0 - self.fatigue * 0.5)
    }
}

/// Type of response action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResponseActionKind {
    Evacuate,
    Shelter,
    Suppress,
    SealBreach,
    ContainInfestation,
    Triage,
    DirectCrowd,
    Scout,
}

impl ResponseActionKind {
    #[must_use]
    pub fn base_work_required(self) -> u32 {
        match self {
            Self::Scout => 20,
            Self::DirectCrowd => 30,
            Self::Shelter => 40,
            Self::Evacuate => 50,
            Self::Triage => 60,
            Self::SealBreach => 80,
            Self::ContainInfestation => 90,
            Self::Suppress => 100,
        }
    }

    #[must_use]
    pub fn required_role(self) -> Option<&'static str> {
        match self {
            Self::Suppress => Some("firefighter"),
            Self::Triage => Some("medic"),
            Self::SealBreach => Some("engineer"),
            Self::ContainInfestation => Some("hazmat"),
            Self::Scout => Some("scout"),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_immediate(self) -> bool {
        matches!(self, Self::Scout | Self::DirectCrowd)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Evacuate => "evacuate",
            Self::Shelter => "shelter",
            Self::Suppress => "suppress",
            Self::SealBreach => "seal_breach",
            Self::ContainInfestation => "contain_infestation",
            Self::Triage => "triage",
            Self::DirectCrowd => "direct_crowd",
            Self::Scout => "scout",
        }
    }
}

/// Status of a response action.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ActionStatus {
    #[default]
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl ActionStatus {
    #[must_use]
    pub fn is_available(self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Assigned | Self::InProgress)
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// A response action instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseAction {
    pub id: ResponseActionId,
    pub kind: ResponseActionKind,
    pub status: ActionStatus,
    pub emergency: EmergencyId,
    pub plan: ResponsePlanId,
    pub target_region: RegionId,
    pub assigned_responders: BTreeSet<ResponderId>,
    pub work_required: u32,
    pub work_done: u32,
    pub priority: u32,
    pub created_tick: u64,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    pub blocked_by: BTreeSet<ResponseActionId>,
}

impl ResponseAction {
    #[must_use]
    pub fn new(
        id: ResponseActionId,
        kind: ResponseActionKind,
        emergency: EmergencyId,
        plan: ResponsePlanId,
        target_region: RegionId,
        created_tick: u64,
    ) -> Self {
        Self {
            id,
            kind,
            status: ActionStatus::Pending,
            emergency,
            plan,
            target_region,
            assigned_responders: BTreeSet::new(),
            work_required: kind.base_work_required(),
            work_done: 0,
            priority: 50,
            created_tick,
            started_tick: None,
            completed_tick: None,
            blocked_by: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_work_required(mut self, work: u32) -> Self {
        self.work_required = work;
        self
    }

    pub fn assign_responder(&mut self, responder: ResponderId) {
        self.assigned_responders.insert(responder);
        if self.status == ActionStatus::Pending {
            self.status = ActionStatus::Assigned;
        }
    }

    pub fn unassign_responder(&mut self, responder: ResponderId) {
        self.assigned_responders.remove(&responder);
        if self.assigned_responders.is_empty() && self.status == ActionStatus::Assigned {
            self.status = ActionStatus::Pending;
        }
    }

    pub fn start(&mut self, tick: u64) {
        if self.status == ActionStatus::Assigned && !self.is_blocked() {
            self.status = ActionStatus::InProgress;
            self.started_tick = Some(tick);
        }
    }

    pub fn add_work(&mut self, amount: u32) -> bool {
        self.work_done = self.work_done.saturating_add(amount);
        self.work_done >= self.work_required
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = ActionStatus::Completed;
        self.completed_tick = Some(tick);
    }

    pub fn fail(&mut self, tick: u64) {
        self.status = ActionStatus::Failed;
        self.completed_tick = Some(tick);
    }

    pub fn cancel(&mut self, tick: u64) {
        self.status = ActionStatus::Cancelled;
        self.completed_tick = Some(tick);
    }

    pub fn add_blocker(&mut self, blocker: ResponseActionId) {
        self.blocked_by.insert(blocker);
    }

    pub fn remove_blocker(&mut self, blocker: ResponseActionId) {
        self.blocked_by.remove(&blocker);
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty()
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.status.is_available() && !self.is_blocked()
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.status.is_finished()
    }

    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.work_required == 0 {
            return 1.0;
        }
        #[expect(clippy::cast_precision_loss, reason = "work values are bounded")]
        let progress = self.work_done as f32 / self.work_required as f32;
        progress.clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn remaining_work(&self) -> u32 {
        self.work_required.saturating_sub(self.work_done)
    }
}

/// A shelter/safe zone for evacuees.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShelterZone {
    pub id: ShelterZoneId,
    pub region: RegionId,
    pub capacity: u32,
    pub current_occupancy: u32,
    pub is_active: bool,
    pub supplies_level: f32,
}

impl ShelterZone {
    #[must_use]
    pub fn new(id: ShelterZoneId, region: RegionId, capacity: u32) -> Self {
        Self {
            id,
            region,
            capacity,
            current_occupancy: 0,
            is_active: true,
            supplies_level: 1.0,
        }
    }

    #[must_use]
    pub fn available_capacity(&self) -> u32 {
        self.capacity.saturating_sub(self.current_occupancy)
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current_occupancy >= self.capacity
    }

    #[must_use]
    pub fn can_accept(&self, count: u32) -> bool {
        self.is_active && self.available_capacity() >= count
    }

    pub fn admit(&mut self, count: u32) {
        self.current_occupancy = (self.current_occupancy + count).min(self.capacity);
    }

    pub fn release(&mut self, count: u32) {
        self.current_occupancy = self.current_occupancy.saturating_sub(count);
    }
}

/// A containment zone for emergencies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContainmentZone {
    pub id: ContainmentZoneId,
    pub emergency: EmergencyId,
    pub perimeter_regions: BTreeSet<RegionId>,
    pub contained_regions: BTreeSet<RegionId>,
    pub integrity: f32,
    pub established_tick: u64,
}

impl ContainmentZone {
    #[must_use]
    pub fn new(
        id: ContainmentZoneId,
        emergency: EmergencyId,
        perimeter: impl IntoIterator<Item = RegionId>,
        contained: impl IntoIterator<Item = RegionId>,
        established_tick: u64,
    ) -> Self {
        Self {
            id,
            emergency,
            perimeter_regions: perimeter.into_iter().collect(),
            contained_regions: contained.into_iter().collect(),
            integrity: 1.0,
            established_tick,
        }
    }

    pub fn breach(&mut self, amount: f32) {
        self.integrity = (self.integrity - amount).max(0.0);
    }

    pub fn reinforce(&mut self, amount: f32) {
        self.integrity = (self.integrity + amount).min(1.0);
    }

    #[must_use]
    pub fn is_holding(&self) -> bool {
        self.integrity > 0.0
    }

    #[must_use]
    pub fn is_breached(&self) -> bool {
        self.integrity <= 0.0
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_emergency_severity_escalation() {
        assert_eq!(
            EmergencySeverity::Minor.escalate(),
            EmergencySeverity::Moderate
        );
        assert_eq!(
            EmergencySeverity::Critical.escalate(),
            EmergencySeverity::Catastrophic
        );
        assert_eq!(
            EmergencySeverity::Catastrophic.escalate(),
            EmergencySeverity::Catastrophic
        );
        assert!(!EmergencySeverity::Catastrophic.can_escalate());
    }

    #[test]
    fn test_emergency_severity_deescalation() {
        assert_eq!(
            EmergencySeverity::Major.deescalate(),
            EmergencySeverity::Moderate
        );
        assert_eq!(
            EmergencySeverity::Minor.deescalate(),
            EmergencySeverity::Minor
        );
    }

    #[test]
    fn test_emergency_kind_properties() {
        assert!(EmergencyKind::Fire.requires_evacuation());
        assert!(EmergencyKind::Fire.requires_containment());
        assert!(EmergencyKind::Fire.requires_suppression());

        assert!(EmergencyKind::Breach.requires_evacuation());
        assert!(!EmergencyKind::Breach.requires_containment());

        assert!(EmergencyKind::Stampede.requires_evacuation());
        assert!(!EmergencyKind::Stampede.requires_containment());

        assert!(!EmergencyKind::Infestation.requires_evacuation());
        assert!(EmergencyKind::Infestation.requires_containment());
    }

    #[test]
    fn test_emergency_creation() {
        let emergency = Emergency::new(
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(5),
            100,
        );

        assert_eq!(emergency.kind, EmergencyKind::Fire);
        assert_eq!(emergency.status, EmergencyStatus::Detected);
        assert_eq!(emergency.severity, EmergencySeverity::Moderate);
        assert!(emergency.affected_regions.contains(&RegionId::new(5)));
        assert_eq!(emergency.detected_tick, 100);
    }

    #[test]
    fn test_emergency_lifecycle() {
        let mut emergency = Emergency::new(
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            0,
        );

        emergency.assess(10);
        assert_eq!(emergency.status, EmergencyStatus::Assessed);
        assert_eq!(emergency.assessed_tick, Some(10));

        let plan = ResponsePlanId::new(1);
        emergency.activate_response(plan);
        assert_eq!(emergency.status, EmergencyStatus::ResponseActive);
        assert_eq!(emergency.active_plan, Some(plan));

        emergency.contain();
        assert_eq!(emergency.status, EmergencyStatus::Contained);

        emergency.resolve(100);
        assert_eq!(emergency.status, EmergencyStatus::Resolved);
        assert_eq!(emergency.resolved_tick, Some(100));
    }

    #[test]
    fn test_emergency_escalation() {
        let mut emergency = Emergency::new(
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            0,
        )
        .with_severity(EmergencySeverity::Minor);

        assert!(!emergency.should_escalate());

        emergency.add_escalation_pressure(50);
        assert!(emergency.should_escalate());

        emergency.escalate();
        assert_eq!(emergency.severity, EmergencySeverity::Moderate);
        assert_eq!(emergency.status, EmergencyStatus::Escalated);
        assert_eq!(emergency.escalation_pressure, 0);
    }

    #[test]
    fn test_responder_creation() {
        let responder = Responder::new(ResponderId::new(1), "Firefighter Bob", 0)
            .with_region(RegionId::new(1))
            .with_role(ResponderRoleId::new("firefighter"));

        assert_eq!(responder.name, "Firefighter Bob");
        assert!(responder.is_available());
        assert!(responder.has_role(&ResponderRoleId::new("firefighter")));
    }

    #[test]
    fn test_responder_assignment_cycle() {
        let mut responder = Responder::new(ResponderId::new(1), "Test", 0);

        responder.assign_to(EmergencyId::new(1), RegionId::new(5));
        assert_eq!(responder.status, ResponderStatus::Assigned);
        assert_eq!(responder.assigned_emergency, Some(EmergencyId::new(1)));

        responder.set_en_route();
        assert_eq!(responder.status, ResponderStatus::EnRoute);

        responder.arrive_on_scene();
        assert_eq!(responder.status, ResponderStatus::OnScene);
        assert_eq!(responder.current_region, Some(RegionId::new(5)));

        let action = ResponseActionId::new(1);
        responder.start_action(action);
        assert_eq!(responder.status, ResponderStatus::Performing);

        responder.complete_action();
        assert_eq!(responder.status, ResponderStatus::OnScene);
        assert_eq!(responder.total_actions_completed, 1);

        responder.release();
        assert!(responder.is_available());
    }

    #[test]
    fn test_responder_fatigue() {
        let mut responder = Responder::new(ResponderId::new(1), "Test", 0);

        assert!(responder.is_deployable());

        responder.add_fatigue(0.95);
        assert!(!responder.is_deployable());

        responder.reduce_fatigue(0.5);
        assert!(responder.is_deployable());
    }

    #[test]
    fn test_response_action_creation() {
        let action = ResponseAction::new(
            ResponseActionId::new(1),
            ResponseActionKind::Suppress,
            EmergencyId::new(1),
            ResponsePlanId::new(1),
            RegionId::new(5),
            0,
        );

        assert_eq!(action.kind, ResponseActionKind::Suppress);
        assert_eq!(action.work_required, 100);
        assert!(action.is_available());
    }

    #[test]
    fn test_response_action_progress() {
        let mut action = ResponseAction::new(
            ResponseActionId::new(1),
            ResponseActionKind::Scout,
            EmergencyId::new(1),
            ResponsePlanId::new(1),
            RegionId::new(1),
            0,
        );

        action.assign_responder(ResponderId::new(1));
        action.start(10);
        assert_eq!(action.status, ActionStatus::InProgress);

        assert!(!action.add_work(10));
        assert!((action.progress() - 0.5).abs() < 0.001);

        assert!(action.add_work(10));
        action.complete(20);
        assert!(action.is_finished());
    }

    #[test]
    fn test_response_action_blocking() {
        let mut action = ResponseAction::new(
            ResponseActionId::new(2),
            ResponseActionKind::Suppress,
            EmergencyId::new(1),
            ResponsePlanId::new(1),
            RegionId::new(1),
            0,
        );

        action.add_blocker(ResponseActionId::new(1));
        assert!(action.is_blocked());
        assert!(!action.is_available());

        action.remove_blocker(ResponseActionId::new(1));
        assert!(!action.is_blocked());
        assert!(action.is_available());
    }

    #[test]
    fn test_shelter_zone() {
        let mut shelter = ShelterZone::new(ShelterZoneId::new(1), RegionId::new(1), 50);

        assert_eq!(shelter.available_capacity(), 50);
        assert!(shelter.can_accept(30));

        shelter.admit(30);
        assert_eq!(shelter.current_occupancy, 30);
        assert_eq!(shelter.available_capacity(), 20);

        shelter.admit(30);
        assert_eq!(shelter.current_occupancy, 50);
        assert!(shelter.is_full());

        shelter.release(10);
        assert_eq!(shelter.current_occupancy, 40);
    }

    #[test]
    fn test_containment_zone() {
        let mut zone = ContainmentZone::new(
            ContainmentZoneId::new(1),
            EmergencyId::new(1),
            [RegionId::new(1), RegionId::new(2)],
            [RegionId::new(3)],
            0,
        );

        assert!(zone.is_holding());
        assert!(!zone.is_breached());

        zone.breach(0.5);
        assert!((zone.integrity - 0.5).abs() < 0.001);
        assert!(zone.is_holding());

        zone.breach(0.6);
        assert!(zone.is_breached());

        zone.reinforce(0.3);
        assert!((zone.integrity - 0.3).abs() < 0.001);
        assert!(zone.is_holding());
    }

    #[test]
    fn test_serde_coverage() {
        let severity = EmergencySeverity::Major;
        let json = serde_json::to_string(&severity).unwrap();
        let restored: EmergencySeverity = serde_json::from_str(&json).unwrap();
        assert_eq!(severity, restored);

        let kind = EmergencyKind::Fire;
        let json = serde_json::to_string(&kind).unwrap();
        let restored: EmergencyKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, restored);

        let emergency = Emergency::new(
            EmergencyId::new(1),
            EmergencyKind::Breach,
            RegionId::new(1),
            100,
        );
        let json = serde_json::to_string(&emergency).unwrap();
        let restored: Emergency = serde_json::from_str(&json).unwrap();
        assert_eq!(emergency, restored);

        let responder =
            Responder::new(ResponderId::new(1), "Test", 0).with_role(ResponderRoleId::new("medic"));
        let json = serde_json::to_string(&responder).unwrap();
        let restored: Responder = serde_json::from_str(&json).unwrap();
        assert_eq!(responder, restored);

        let action = ResponseAction::new(
            ResponseActionId::new(1),
            ResponseActionKind::Triage,
            EmergencyId::new(1),
            ResponsePlanId::new(1),
            RegionId::new(1),
            0,
        );
        let json = serde_json::to_string(&action).unwrap();
        let restored: ResponseAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, restored);
    }
}

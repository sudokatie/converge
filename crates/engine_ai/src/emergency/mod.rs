//! Emergency response system for incident management.
//!
//! Provides deterministic simulation of emergency response:
//!
//! - Emergency detection and assessment (fire, breach, stampede, infestation)
//! - Response plan creation with ordered action dependencies
//! - Responder assignment and progress tracking
//! - Severity escalation and containment
//! - Shelter and containment zone management
//! - Event emission for emergency lifecycle
//! - Snapshots and projections for state inspection
//! - Stable fingerprints for determinism verification

mod events;
mod ids;
mod plan;
mod state;

pub use events::{EmergencyEvent, EmergencyEventKind};
pub use ids::{
    ContainmentZoneId, EmergencyId, EmergencyTypeId, RegionId, ResponderId, ResponderRoleId,
    ResponseActionId, ResponsePlanId, ResponseProtocolId, ShelterZoneId,
};
pub use plan::{Assignment, PlanStatus, ResponsePlan, create_standard_plan};
pub use state::{
    ActionStatus, ContainmentZone, Emergency, EmergencyKind, EmergencySeverity, EmergencyStatus,
    Responder, ResponderStatus, ResponseAction, ResponseActionKind, ShelterZone,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Snapshot of emergency response system state at a point in time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EmergencySnapshot {
    pub tick: u64,
    pub total_emergencies: u32,
    pub active_emergencies: u32,
    pub resolved_emergencies: u32,
    pub total_responders: u32,
    pub available_responders: u32,
    pub active_responders: u32,
    pub total_actions: u32,
    pub pending_actions: u32,
    pub completed_actions: u32,
    pub total_casualties: u32,
    pub total_damage: u32,
    pub emergencies_by_kind: BTreeMap<String, u32>,
    pub emergencies_by_severity: BTreeMap<String, u32>,
}

impl EmergencySnapshot {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded by game limits")]
    pub fn response_rate(&self) -> f32 {
        if self.total_emergencies == 0 {
            return 0.0;
        }
        self.resolved_emergencies as f32 / self.total_emergencies as f32
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded by game limits")]
    pub fn responder_utilization(&self) -> f32 {
        if self.total_responders == 0 {
            return 0.0;
        }
        self.active_responders as f32 / self.total_responders as f32
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.total_emergencies.to_le_bytes());
        hasher.update(&self.active_emergencies.to_le_bytes());
        hasher.update(&self.resolved_emergencies.to_le_bytes());
        hasher.update(&self.total_responders.to_le_bytes());
        hasher.update(&self.total_casualties.to_le_bytes());
        hasher.update(&self.total_damage.to_le_bytes());
        hasher.finalize()
    }
}

/// Summary of emergency response system state for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EmergencySummary {
    pub tick: u64,
    pub active_count: u32,
    pub resolved_count: u32,
    pub responder_count: u32,
    pub response_rate: f32,
    pub utilization: f32,
}

impl From<&EmergencySnapshot> for EmergencySummary {
    fn from(snapshot: &EmergencySnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            active_count: snapshot.active_emergencies,
            resolved_count: snapshot.resolved_emergencies,
            responder_count: snapshot.total_responders,
            response_rate: snapshot.response_rate(),
            utilization: snapshot.responder_utilization(),
        }
    }
}

/// Fingerprint for emergency response system state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmergencyFingerprint(pub u32);

impl EmergencyFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for EmergencyFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "emergency:{:08x}", self.0)
    }
}

/// Projection of future emergency response state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmergencyProjection {
    pub base_tick: u64,
    pub projected_tick: u64,
    pub estimated_resolutions: u32,
    pub estimated_escalations: u32,
    pub estimated_casualties: u32,
    pub risk_score: f32,
}

impl EmergencyProjection {
    #[must_use]
    pub fn new(base_tick: u64, projected_tick: u64) -> Self {
        Self {
            base_tick,
            projected_tick,
            estimated_resolutions: 0,
            estimated_escalations: 0,
            estimated_casualties: 0,
            risk_score: 0.0,
        }
    }

    #[must_use]
    pub fn with_risk_score(mut self, score: f32) -> Self {
        self.risk_score = score.clamp(0.0, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_incident_lifecycle() {
        let region = RegionId::new(1);
        let mut emergency = Emergency::new(EmergencyId::new(1), EmergencyKind::Fire, region, 0);

        assert_eq!(emergency.kind, EmergencyKind::Fire);
        assert_eq!(emergency.status, EmergencyStatus::Detected);
        assert_eq!(emergency.severity, EmergencySeverity::Moderate);

        emergency.assess(10);
        assert_eq!(emergency.status, EmergencyStatus::Assessed);

        let plan_id = ResponsePlanId::new(1);
        emergency.activate_response(plan_id);
        assert_eq!(emergency.status, EmergencyStatus::ResponseActive);

        emergency.contain();
        assert_eq!(emergency.status, EmergencyStatus::Contained);

        emergency.resolve(100);
        assert!(emergency.is_resolved());
        assert_eq!(emergency.duration(100), 100);
    }

    #[test]
    fn test_breach_incident_lifecycle() {
        let region = RegionId::new(2);
        let emergency = Emergency::new(EmergencyId::new(2), EmergencyKind::Breach, region, 0);

        assert_eq!(emergency.kind, EmergencyKind::Breach);
        assert_eq!(emergency.severity, EmergencySeverity::Major);
        assert!(emergency.kind.requires_evacuation());
        assert!(!emergency.kind.requires_containment());

        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            emergency.id,
            EmergencyKind::Breach,
            region,
            0,
        );

        let has_seal = plan
            .actions
            .values()
            .any(|a| a.kind == ResponseActionKind::SealBreach);
        assert!(has_seal);
    }

    #[test]
    fn test_stampede_incident_lifecycle() {
        let region = RegionId::new(3);
        let emergency = Emergency::new(EmergencyId::new(3), EmergencyKind::Stampede, region, 0);

        assert_eq!(emergency.kind, EmergencyKind::Stampede);
        assert!(emergency.kind.requires_evacuation());
        assert!(!emergency.kind.requires_containment());
        assert!((emergency.kind.spread_rate() - 2.0).abs() < 0.001);

        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            emergency.id,
            EmergencyKind::Stampede,
            region,
            0,
        );

        let has_crowd_control = plan
            .actions
            .values()
            .any(|a| a.kind == ResponseActionKind::DirectCrowd);
        assert!(has_crowd_control);
    }

    #[test]
    fn test_infestation_incident_lifecycle() {
        let region = RegionId::new(4);
        let emergency = Emergency::new(EmergencyId::new(4), EmergencyKind::Infestation, region, 0);

        assert_eq!(emergency.kind, EmergencyKind::Infestation);
        assert_eq!(emergency.severity, EmergencySeverity::Minor);
        assert!(!emergency.kind.requires_evacuation());
        assert!(emergency.kind.requires_containment());

        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            emergency.id,
            EmergencyKind::Infestation,
            region,
            0,
        );

        let has_contain = plan
            .actions
            .values()
            .any(|a| a.kind == ResponseActionKind::ContainInfestation);
        assert!(has_contain);
    }

    #[test]
    fn test_response_plan_action_ordering() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            0,
        );

        assert!(plan.action_count() >= 3);

        let first_action_id = plan.action_order[0];
        let first_action = plan.get_action(first_action_id).unwrap();
        assert_eq!(first_action.kind, ResponseActionKind::Scout);
        assert!(!first_action.is_blocked());

        let second_action_id = plan.action_order[1];
        let second_action = plan.get_action(second_action_id).unwrap();
        assert!(second_action.blocked_by.contains(&first_action_id));
    }

    #[test]
    fn test_responder_assignment_and_progress() {
        let mut responder = Responder::new(ResponderId::new(1), "Firefighter", 0)
            .with_region(RegionId::new(1))
            .with_role(ResponderRoleId::new("firefighter"));

        assert!(responder.is_available());
        assert!(responder.has_role(&ResponderRoleId::new("firefighter")));

        responder.assign_to(EmergencyId::new(1), RegionId::new(5));
        assert_eq!(responder.status, ResponderStatus::Assigned);
        assert_eq!(responder.total_responses, 1);

        responder.set_en_route();
        assert_eq!(responder.status, ResponderStatus::EnRoute);

        responder.arrive_on_scene();
        assert_eq!(responder.status, ResponderStatus::OnScene);
        assert_eq!(responder.current_region, Some(RegionId::new(5)));

        let action_id = ResponseActionId::new(1);
        responder.start_action(action_id);
        assert_eq!(responder.status, ResponderStatus::Performing);
        assert_eq!(responder.current_action, Some(action_id));

        responder.complete_action();
        assert_eq!(responder.status, ResponderStatus::OnScene);
        assert_eq!(responder.total_actions_completed, 1);

        responder.release();
        assert!(responder.is_available());
    }

    #[test]
    fn test_response_action_progress() {
        let mut action = ResponseAction::new(
            ResponseActionId::new(1),
            ResponseActionKind::Suppress,
            EmergencyId::new(1),
            ResponsePlanId::new(1),
            RegionId::new(1),
            0,
        );

        assert_eq!(action.work_required, 100);
        assert!(action.is_available());

        action.assign_responder(ResponderId::new(1));
        assert_eq!(action.status, ActionStatus::Assigned);

        action.start(10);
        assert_eq!(action.status, ActionStatus::InProgress);

        assert!(!action.add_work(50));
        assert!((action.progress() - 0.5).abs() < 0.001);
        assert_eq!(action.remaining_work(), 50);

        assert!(action.add_work(50));
        action.complete(20);
        assert!(action.is_finished());
    }

    #[test]
    fn test_severity_escalation() {
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

        let prev_severity = emergency.severity;
        emergency.escalate();
        assert_eq!(emergency.severity, EmergencySeverity::Moderate);
        assert!(emergency.severity > prev_severity);
        assert_eq!(emergency.status, EmergencyStatus::Escalated);
        assert_eq!(emergency.escalation_pressure, 0);

        let mut catastrophic = emergency.clone();
        catastrophic.severity = EmergencySeverity::Catastrophic;
        assert!(!catastrophic.severity.can_escalate());
    }

    #[test]
    fn test_snapshot_creation() {
        let mut snapshot = EmergencySnapshot::new(100);
        snapshot.total_emergencies = 10;
        snapshot.active_emergencies = 3;
        snapshot.resolved_emergencies = 7;
        snapshot.total_responders = 20;
        snapshot.available_responders = 8;
        snapshot.active_responders = 12;

        assert!((snapshot.response_rate() - 0.7).abs() < 0.001);
        assert!((snapshot.responder_utilization() - 0.6).abs() < 0.001);

        let checksum1 = snapshot.checksum();
        let checksum2 = snapshot.checksum();
        assert_eq!(checksum1, checksum2);

        snapshot.total_casualties = 1;
        let checksum3 = snapshot.checksum();
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_projection_creation() {
        let projection = EmergencyProjection::new(100, 200).with_risk_score(0.75);

        assert_eq!(projection.base_tick, 100);
        assert_eq!(projection.projected_tick, 200);
        assert!((projection.risk_score - 0.75).abs() < 0.001);

        let clamped = EmergencyProjection::new(0, 100).with_risk_score(1.5);
        assert!((clamped.risk_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_serde_roundtrip_emergency() {
        let emergency = Emergency::new(
            EmergencyId::new(42),
            EmergencyKind::Fire,
            RegionId::new(1),
            100,
        )
        .with_severity(EmergencySeverity::Major)
        .with_intensity(5.0);

        let json = serde_json::to_string(&emergency).unwrap();
        let restored: Emergency = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, emergency.id);
        assert_eq!(restored.kind, emergency.kind);
        assert_eq!(restored.severity, emergency.severity);
        assert!((restored.intensity - emergency.intensity).abs() < 0.001);
    }

    #[test]
    fn test_serde_roundtrip_responder() {
        let responder = Responder::new(ResponderId::new(1), "Test Responder", 0)
            .with_region(RegionId::new(5))
            .with_role(ResponderRoleId::new("medic"))
            .with_role(ResponderRoleId::new("hazmat"));

        let json = serde_json::to_string(&responder).unwrap();
        let restored: Responder = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, responder.id);
        assert_eq!(restored.name, responder.name);
        assert_eq!(restored.roles, responder.roles);
    }

    #[test]
    fn test_serde_roundtrip_plan() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Breach,
            RegionId::new(1),
            0,
        );

        let json = serde_json::to_string(&plan).unwrap();
        let restored: ResponsePlan = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, plan.id);
        assert_eq!(restored.action_count(), plan.action_count());
        assert_eq!(restored.action_order, plan.action_order);
    }

    #[test]
    fn test_serde_roundtrip_snapshot() {
        let mut snapshot = EmergencySnapshot::new(500);
        snapshot.total_emergencies = 25;
        snapshot.total_casualties = 3;
        snapshot.emergencies_by_kind.insert("fire".into(), 10);
        snapshot.emergencies_by_severity.insert("major".into(), 5);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: EmergencySnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, snapshot);
        assert_eq!(restored.checksum(), snapshot.checksum());
    }

    #[test]
    fn test_serde_roundtrip_projection() {
        let projection = EmergencyProjection {
            base_tick: 100,
            projected_tick: 500,
            estimated_resolutions: 5,
            estimated_escalations: 2,
            estimated_casualties: 1,
            risk_score: 0.35,
        };

        let json = serde_json::to_string(&projection).unwrap();
        let restored: EmergencyProjection = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, projection);
    }

    #[test]
    fn test_serde_roundtrip_fingerprint() {
        let fp = EmergencyFingerprint(0xDEAD_BEEF);

        let json = serde_json::to_string(&fp).unwrap();
        let restored: EmergencyFingerprint = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, fp);
        assert_eq!(format!("{fp}"), "emergency:deadbeef");
    }

    #[test]
    fn test_fingerprint_stability() {
        let fp1 = EmergencyFingerprint(0x1234_5678);
        let fp2 = EmergencyFingerprint(0x1234_5678);
        let fp3 = EmergencyFingerprint(0x8765_4321);

        assert_eq!(fp1, fp2);
        assert_ne!(fp1, fp3);
        assert_eq!(fp1.raw(), 0x1234_5678);
    }

    #[test]
    fn test_snapshot_checksum_deterministic() {
        let snapshot1 = EmergencySnapshot {
            tick: 100,
            total_emergencies: 5,
            active_emergencies: 2,
            resolved_emergencies: 3,
            total_responders: 10,
            available_responders: 4,
            active_responders: 6,
            total_actions: 20,
            pending_actions: 5,
            completed_actions: 15,
            total_casualties: 1,
            total_damage: 500,
            emergencies_by_kind: BTreeMap::new(),
            emergencies_by_severity: BTreeMap::new(),
        };

        let snapshot2 = snapshot1.clone();

        assert_eq!(snapshot1.checksum(), snapshot2.checksum());
    }

    #[test]
    fn test_emergency_event_creation() {
        let event = EmergencyEvent::emergency_detected(
            100,
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(5),
            EmergencySeverity::Major,
        );

        assert_eq!(event.tick, 100);
        assert!(event.involves_emergency(EmergencyId::new(1)));
        assert!(!event.involves_emergency(EmergencyId::new(2)));
    }

    #[test]
    fn test_emergency_event_escalation() {
        let event = EmergencyEvent::emergency_escalated(
            50,
            EmergencyId::new(1),
            EmergencySeverity::Minor,
            EmergencySeverity::Major,
        );

        assert_eq!(event.tick, 50);
        assert!(event.involves_emergency(EmergencyId::new(1)));

        let json = serde_json::to_string(&event).unwrap();
        let restored: EmergencyEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }

    #[test]
    fn test_shelter_zone_capacity() {
        let mut shelter = ShelterZone::new(ShelterZoneId::new(1), RegionId::new(1), 100);

        assert_eq!(shelter.available_capacity(), 100);
        assert!(shelter.can_accept(50));
        assert!(!shelter.is_full());

        shelter.admit(60);
        assert_eq!(shelter.current_occupancy, 60);
        assert_eq!(shelter.available_capacity(), 40);

        shelter.admit(50);
        assert_eq!(shelter.current_occupancy, 100);
        assert!(shelter.is_full());
        assert!(!shelter.can_accept(1));

        shelter.release(30);
        assert_eq!(shelter.current_occupancy, 70);
    }

    #[test]
    fn test_containment_zone_integrity() {
        let mut zone = ContainmentZone::new(
            ContainmentZoneId::new(1),
            EmergencyId::new(1),
            [RegionId::new(1), RegionId::new(2)],
            [RegionId::new(3)],
            0,
        );

        assert!(zone.is_holding());
        assert!(!zone.is_breached());
        assert!((zone.integrity - 1.0).abs() < 0.001);

        zone.breach(0.3);
        assert!((zone.integrity - 0.7).abs() < 0.001);
        assert!(zone.is_holding());

        zone.breach(0.8);
        assert!(zone.is_breached());

        zone.reinforce(0.5);
        assert!((zone.integrity - 0.5).abs() < 0.001);
        assert!(zone.is_holding());
    }

    #[test]
    fn test_responder_fatigue_system() {
        let mut responder = Responder::new(ResponderId::new(1), "Tired Worker", 0);

        assert!(responder.is_deployable());
        assert!((responder.fatigue - 0.0).abs() < 0.001);

        responder.add_fatigue(0.5);
        assert!((responder.fatigue - 0.5).abs() < 0.001);
        assert!(responder.is_deployable());

        responder.add_fatigue(0.5);
        assert!(!responder.is_deployable());

        responder.reduce_fatigue(0.2);
        assert!(responder.is_deployable());

        assert!(responder.effective_work_rate() < 1.0);
    }

    #[test]
    fn test_action_blocking_dependencies() {
        let mut action = ResponseAction::new(
            ResponseActionId::new(3),
            ResponseActionKind::Suppress,
            EmergencyId::new(1),
            ResponsePlanId::new(1),
            RegionId::new(1),
            0,
        );

        assert!(!action.is_blocked());
        assert!(action.is_available());

        action.add_blocker(ResponseActionId::new(1));
        action.add_blocker(ResponseActionId::new(2));
        assert!(action.is_blocked());
        assert!(!action.is_available());

        action.remove_blocker(ResponseActionId::new(1));
        assert!(action.is_blocked());

        action.remove_blocker(ResponseActionId::new(2));
        assert!(!action.is_blocked());
        assert!(action.is_available());
    }

    #[test]
    fn test_emergency_spread_tracking() {
        let mut emergency = Emergency::new(
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            0,
        );

        assert_eq!(emergency.region_count(), 1);
        assert!(emergency.affected_regions.contains(&RegionId::new(1)));

        emergency.spread_to(RegionId::new(2));
        emergency.spread_to(RegionId::new(3));
        assert_eq!(emergency.region_count(), 3);

        emergency.add_damage(100);
        emergency.add_casualties(2);
        assert_eq!(emergency.damage_accumulated, 100);
        assert_eq!(emergency.casualties, 2);
    }

    #[test]
    fn test_summary_from_snapshot_conversion() {
        let mut snapshot = EmergencySnapshot::new(200);
        snapshot.active_emergencies = 5;
        snapshot.resolved_emergencies = 10;
        snapshot.total_emergencies = 15;
        snapshot.total_responders = 20;
        snapshot.active_responders = 15;

        let summary = EmergencySummary::from(&snapshot);

        assert_eq!(summary.tick, 200);
        assert_eq!(summary.active_count, 5);
        assert_eq!(summary.resolved_count, 10);
        assert_eq!(summary.responder_count, 20);
        assert!((summary.response_rate - (10.0 / 15.0)).abs() < 0.001);
        assert!((summary.utilization - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_all_emergency_kinds() {
        let kinds = [
            EmergencyKind::Fire,
            EmergencyKind::Breach,
            EmergencyKind::Stampede,
            EmergencyKind::Infestation,
        ];

        for kind in kinds {
            let emergency = Emergency::new(EmergencyId::new(1), kind, RegionId::new(1), 0);
            assert_eq!(emergency.kind, kind);

            let plan = create_standard_plan(
                ResponsePlanId::new(1),
                EmergencyId::new(1),
                kind,
                RegionId::new(1),
                0,
            );
            assert!(plan.action_count() > 0);

            let json = serde_json::to_string(&kind).unwrap();
            let restored: EmergencyKind = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, kind);
        }
    }

    #[test]
    fn test_all_severity_levels() {
        let severities = [
            EmergencySeverity::Minor,
            EmergencySeverity::Moderate,
            EmergencySeverity::Major,
            EmergencySeverity::Critical,
            EmergencySeverity::Catastrophic,
        ];

        for (i, severity) in severities.iter().enumerate() {
            if i < severities.len() - 1 {
                assert!(severity.can_escalate());
                assert_eq!(severity.escalate(), severities[i + 1]);
            }

            if i > 0 {
                assert_eq!(severity.deescalate(), severities[i - 1]);
            }

            assert!(severity.responder_multiplier() > 0.0);
            assert!(severity.priority_score() > 0);
        }
    }

    #[test]
    fn test_all_action_kinds() {
        let action_kinds = [
            ResponseActionKind::Evacuate,
            ResponseActionKind::Shelter,
            ResponseActionKind::Suppress,
            ResponseActionKind::SealBreach,
            ResponseActionKind::ContainInfestation,
            ResponseActionKind::Triage,
            ResponseActionKind::DirectCrowd,
            ResponseActionKind::Scout,
        ];

        for kind in action_kinds {
            let action = ResponseAction::new(
                ResponseActionId::new(1),
                kind,
                EmergencyId::new(1),
                ResponsePlanId::new(1),
                RegionId::new(1),
                0,
            );

            assert!(action.work_required > 0);

            let json = serde_json::to_string(&kind).unwrap();
            let restored: ResponseActionKind = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, kind);
        }
    }
}

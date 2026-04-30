//! Response plan management for emergencies.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::ids::{
    EmergencyId, RegionId, ResponderId, ResponseActionId, ResponsePlanId, ResponseProtocolId,
};
use super::state::{EmergencyKind, ResponseAction, ResponseActionKind};

/// Status of a response plan.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PlanStatus {
    #[default]
    Drafting,
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl PlanStatus {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    #[must_use]
    pub fn can_modify(self) -> bool {
        matches!(self, Self::Drafting | Self::Paused)
    }
}

/// A response plan for handling an emergency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponsePlan {
    pub id: ResponsePlanId,
    pub emergency: EmergencyId,
    pub protocol: ResponseProtocolId,
    pub status: PlanStatus,
    pub actions: BTreeMap<ResponseActionId, ResponseAction>,
    pub action_order: Vec<ResponseActionId>,
    pub assigned_responders: BTreeSet<ResponderId>,
    pub created_tick: u64,
    pub activated_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    next_action_id: u64,
}

impl ResponsePlan {
    #[must_use]
    pub fn new(
        id: ResponsePlanId,
        emergency: EmergencyId,
        protocol: ResponseProtocolId,
        created_tick: u64,
    ) -> Self {
        Self {
            id,
            emergency,
            protocol,
            status: PlanStatus::Drafting,
            actions: BTreeMap::new(),
            action_order: Vec::new(),
            assigned_responders: BTreeSet::new(),
            created_tick,
            activated_tick: None,
            completed_tick: None,
            next_action_id: 1,
        }
    }

    pub fn add_action(
        &mut self,
        kind: ResponseActionKind,
        target_region: RegionId,
    ) -> ResponseActionId {
        let action_id = ResponseActionId::new(self.next_action_id);
        self.next_action_id += 1;

        let action = ResponseAction::new(
            action_id,
            kind,
            self.emergency,
            self.id,
            target_region,
            self.created_tick,
        );

        self.actions.insert(action_id, action);
        self.action_order.push(action_id);
        action_id
    }

    pub fn add_action_with_blocker(
        &mut self,
        kind: ResponseActionKind,
        target_region: RegionId,
        blocked_by: ResponseActionId,
    ) -> ResponseActionId {
        let action_id = self.add_action(kind, target_region);
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.add_blocker(blocked_by);
        }
        action_id
    }

    pub fn activate(&mut self, tick: u64) {
        if self.status == PlanStatus::Drafting || self.status == PlanStatus::Paused {
            self.status = PlanStatus::Active;
            self.activated_tick = Some(tick);
        }
    }

    pub fn pause(&mut self) {
        if self.status == PlanStatus::Active {
            self.status = PlanStatus::Paused;
        }
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = PlanStatus::Completed;
        self.completed_tick = Some(tick);
    }

    pub fn fail(&mut self, tick: u64) {
        self.status = PlanStatus::Failed;
        self.completed_tick = Some(tick);
    }

    pub fn cancel(&mut self, tick: u64) {
        self.status = PlanStatus::Cancelled;
        self.completed_tick = Some(tick);
    }

    pub fn assign_responder(&mut self, responder: ResponderId) {
        self.assigned_responders.insert(responder);
    }

    pub fn unassign_responder(&mut self, responder: ResponderId) {
        self.assigned_responders.remove(&responder);
    }

    #[must_use]
    pub fn get_action(&self, id: ResponseActionId) -> Option<&ResponseAction> {
        self.actions.get(&id)
    }

    pub fn get_action_mut(&mut self, id: ResponseActionId) -> Option<&mut ResponseAction> {
        self.actions.get_mut(&id)
    }

    #[must_use]
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    pub fn pending_actions(&self) -> impl Iterator<Item = &ResponseAction> {
        self.actions.values().filter(|a| a.is_available())
    }

    pub fn active_actions(&self) -> impl Iterator<Item = &ResponseAction> {
        self.actions.values().filter(|a| a.is_active())
    }

    pub fn completed_actions(&self) -> impl Iterator<Item = &ResponseAction> {
        self.actions.values().filter(|a| a.is_finished())
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "action counts bounded")]
    pub fn progress(&self) -> f32 {
        if self.actions.is_empty() {
            return 1.0;
        }
        let completed = self.actions.values().filter(|a| a.is_finished()).count();
        completed as f32 / self.actions.len() as f32
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.actions.is_empty() && self.actions.values().all(ResponseAction::is_finished)
    }

    #[must_use]
    pub fn responder_count(&self) -> usize {
        self.assigned_responders.len()
    }
}

/// Assignment of a responder to an action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub responder: ResponderId,
    pub action: ResponseActionId,
    pub plan: ResponsePlanId,
    pub emergency: EmergencyId,
    pub assigned_tick: u64,
}

impl Assignment {
    #[must_use]
    pub fn new(
        responder: ResponderId,
        action: ResponseActionId,
        plan: ResponsePlanId,
        emergency: EmergencyId,
        assigned_tick: u64,
    ) -> Self {
        Self {
            responder,
            action,
            plan,
            emergency,
            assigned_tick,
        }
    }
}

/// Creates a standard response plan for a given emergency kind.
#[must_use]
pub fn create_standard_plan(
    plan_id: ResponsePlanId,
    emergency_id: EmergencyId,
    kind: EmergencyKind,
    origin_region: RegionId,
    created_tick: u64,
) -> ResponsePlan {
    let protocol = ResponseProtocolId::new(format!("standard_{}", kind.as_str()));
    let mut plan = ResponsePlan::new(plan_id, emergency_id, protocol, created_tick);

    match kind {
        EmergencyKind::Fire => {
            let scout = plan.add_action(ResponseActionKind::Scout, origin_region);
            let evacuate =
                plan.add_action_with_blocker(ResponseActionKind::Evacuate, origin_region, scout);
            plan.add_action_with_blocker(ResponseActionKind::Suppress, origin_region, evacuate);
            plan.add_action(ResponseActionKind::Triage, origin_region);
        }
        EmergencyKind::Breach => {
            let scout = plan.add_action(ResponseActionKind::Scout, origin_region);
            let evacuate =
                plan.add_action_with_blocker(ResponseActionKind::Evacuate, origin_region, scout);
            plan.add_action_with_blocker(ResponseActionKind::SealBreach, origin_region, evacuate);
            plan.add_action(ResponseActionKind::Triage, origin_region);
        }
        EmergencyKind::Stampede => {
            plan.add_action(ResponseActionKind::DirectCrowd, origin_region);
            plan.add_action(ResponseActionKind::Shelter, origin_region);
            plan.add_action(ResponseActionKind::Triage, origin_region);
        }
        EmergencyKind::Infestation => {
            let scout = plan.add_action(ResponseActionKind::Scout, origin_region);
            plan.add_action_with_blocker(
                ResponseActionKind::ContainInfestation,
                origin_region,
                scout,
            );
            plan.add_action(ResponseActionKind::Suppress, origin_region);
        }
    }

    plan
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_creation() {
        let plan = ResponsePlan::new(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            ResponseProtocolId::new("test"),
            0,
        );

        assert_eq!(plan.status, PlanStatus::Drafting);
        assert_eq!(plan.action_count(), 0);
    }

    #[test]
    fn test_plan_add_actions() {
        let mut plan = ResponsePlan::new(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            ResponseProtocolId::new("test"),
            0,
        );

        let action1 = plan.add_action(ResponseActionKind::Scout, RegionId::new(1));
        let action2 =
            plan.add_action_with_blocker(ResponseActionKind::Evacuate, RegionId::new(1), action1);

        assert_eq!(plan.action_count(), 2);
        assert!(plan.get_action(action2).unwrap().is_blocked());
    }

    #[test]
    fn test_plan_lifecycle() {
        let mut plan = ResponsePlan::new(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            ResponseProtocolId::new("test"),
            0,
        );

        plan.add_action(ResponseActionKind::Scout, RegionId::new(1));

        plan.activate(10);
        assert_eq!(plan.status, PlanStatus::Active);
        assert_eq!(plan.activated_tick, Some(10));

        plan.pause();
        assert_eq!(plan.status, PlanStatus::Paused);

        plan.activate(15);
        assert_eq!(plan.status, PlanStatus::Active);

        plan.complete(100);
        assert_eq!(plan.status, PlanStatus::Completed);
        assert_eq!(plan.completed_tick, Some(100));
    }

    #[test]
    fn test_plan_progress() {
        let mut plan = ResponsePlan::new(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            ResponseProtocolId::new("test"),
            0,
        );

        plan.add_action(ResponseActionKind::Scout, RegionId::new(1));
        plan.add_action(ResponseActionKind::Evacuate, RegionId::new(1));

        assert!((plan.progress() - 0.0).abs() < f32::EPSILON);

        let action_ids: Vec<_> = plan.action_order.clone();
        plan.get_action_mut(action_ids[0]).unwrap().complete(10);

        assert!((plan.progress() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_standard_fire_plan() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            0,
        );

        assert_eq!(plan.action_count(), 4);
        assert!(plan.protocol.as_str().contains("fire"));
    }

    #[test]
    fn test_standard_breach_plan() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Breach,
            RegionId::new(1),
            0,
        );

        assert_eq!(plan.action_count(), 4);
        let has_seal = plan
            .actions
            .values()
            .any(|a| a.kind == ResponseActionKind::SealBreach);
        assert!(has_seal);
    }

    #[test]
    fn test_standard_stampede_plan() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Stampede,
            RegionId::new(1),
            0,
        );

        let has_direct_crowd = plan
            .actions
            .values()
            .any(|a| a.kind == ResponseActionKind::DirectCrowd);
        assert!(has_direct_crowd);
    }

    #[test]
    fn test_standard_infestation_plan() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Infestation,
            RegionId::new(1),
            0,
        );

        let has_contain = plan
            .actions
            .values()
            .any(|a| a.kind == ResponseActionKind::ContainInfestation);
        assert!(has_contain);
    }

    #[test]
    fn test_assignment_creation() {
        let assignment = Assignment::new(
            ResponderId::new(1),
            ResponseActionId::new(1),
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            100,
        );

        assert_eq!(assignment.responder, ResponderId::new(1));
        assert_eq!(assignment.assigned_tick, 100);
    }

    #[test]
    fn test_plan_serde() {
        let plan = create_standard_plan(
            ResponsePlanId::new(1),
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            0,
        );

        let json = serde_json::to_string(&plan).unwrap();
        let restored: ResponsePlan = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, plan.id);
        assert_eq!(restored.action_count(), plan.action_count());
    }
}

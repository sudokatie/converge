//! Event types for the emergency response system.

use serde::{Deserialize, Serialize};

use super::ids::{EmergencyId, RegionId, ResponderId, ResponseActionId, ResponsePlanId};
use super::state::{EmergencyKind, EmergencySeverity, ResponseActionKind};

/// Kind of emergency event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmergencyEventKind {
    EmergencyDetected {
        emergency: EmergencyId,
        kind: EmergencyKind,
        region: RegionId,
        severity: EmergencySeverity,
    },
    EmergencyAssessed {
        emergency: EmergencyId,
    },
    EmergencyEscalated {
        emergency: EmergencyId,
        from_severity: EmergencySeverity,
        to_severity: EmergencySeverity,
    },
    EmergencyDeescalated {
        emergency: EmergencyId,
        from_severity: EmergencySeverity,
        to_severity: EmergencySeverity,
    },
    EmergencyContained {
        emergency: EmergencyId,
    },
    EmergencyResolved {
        emergency: EmergencyId,
        duration_ticks: u64,
    },
    EmergencySpread {
        emergency: EmergencyId,
        to_region: RegionId,
    },
    PlanCreated {
        plan: ResponsePlanId,
        emergency: EmergencyId,
    },
    PlanActivated {
        plan: ResponsePlanId,
    },
    PlanCompleted {
        plan: ResponsePlanId,
    },
    PlanFailed {
        plan: ResponsePlanId,
        reason: String,
    },
    ActionStarted {
        action: ResponseActionId,
        kind: ResponseActionKind,
        region: RegionId,
    },
    ActionProgress {
        action: ResponseActionId,
        progress: f32,
    },
    ActionCompleted {
        action: ResponseActionId,
        duration_ticks: u64,
    },
    ActionFailed {
        action: ResponseActionId,
        reason: String,
    },
    ResponderAssigned {
        responder: ResponderId,
        emergency: EmergencyId,
        action: ResponseActionId,
    },
    ResponderArrived {
        responder: ResponderId,
        region: RegionId,
    },
    ResponderReleased {
        responder: ResponderId,
        emergency: EmergencyId,
    },
    ResponderIncapacitated {
        responder: ResponderId,
    },
    CasualtyReported {
        emergency: EmergencyId,
        count: u32,
    },
    DamageReported {
        emergency: EmergencyId,
        amount: u32,
    },
}

/// An event in the emergency response system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmergencyEvent {
    pub tick: u64,
    pub kind: EmergencyEventKind,
}

impl EmergencyEvent {
    #[must_use]
    pub fn new(tick: u64, kind: EmergencyEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn emergency_detected(
        tick: u64,
        emergency: EmergencyId,
        kind: EmergencyKind,
        region: RegionId,
        severity: EmergencySeverity,
    ) -> Self {
        Self::new(
            tick,
            EmergencyEventKind::EmergencyDetected {
                emergency,
                kind,
                region,
                severity,
            },
        )
    }

    #[must_use]
    pub fn emergency_escalated(
        tick: u64,
        emergency: EmergencyId,
        from: EmergencySeverity,
        to: EmergencySeverity,
    ) -> Self {
        Self::new(
            tick,
            EmergencyEventKind::EmergencyEscalated {
                emergency,
                from_severity: from,
                to_severity: to,
            },
        )
    }

    #[must_use]
    pub fn emergency_deescalated(
        tick: u64,
        emergency: EmergencyId,
        from: EmergencySeverity,
        to: EmergencySeverity,
    ) -> Self {
        Self::new(
            tick,
            EmergencyEventKind::EmergencyDeescalated {
                emergency,
                from_severity: from,
                to_severity: to,
            },
        )
    }

    #[must_use]
    pub fn emergency_resolved(tick: u64, emergency: EmergencyId, duration: u64) -> Self {
        Self::new(
            tick,
            EmergencyEventKind::EmergencyResolved {
                emergency,
                duration_ticks: duration,
            },
        )
    }

    #[must_use]
    pub fn action_completed(tick: u64, action: ResponseActionId, duration: u64) -> Self {
        Self::new(
            tick,
            EmergencyEventKind::ActionCompleted {
                action,
                duration_ticks: duration,
            },
        )
    }

    #[must_use]
    pub fn responder_assigned(
        tick: u64,
        responder: ResponderId,
        emergency: EmergencyId,
        action: ResponseActionId,
    ) -> Self {
        Self::new(
            tick,
            EmergencyEventKind::ResponderAssigned {
                responder,
                emergency,
                action,
            },
        )
    }

    #[must_use]
    pub fn involves_emergency(&self, emergency: EmergencyId) -> bool {
        match &self.kind {
            EmergencyEventKind::EmergencyDetected { emergency: e, .. }
            | EmergencyEventKind::EmergencyAssessed { emergency: e }
            | EmergencyEventKind::EmergencyEscalated { emergency: e, .. }
            | EmergencyEventKind::EmergencyDeescalated { emergency: e, .. }
            | EmergencyEventKind::EmergencyContained { emergency: e }
            | EmergencyEventKind::EmergencyResolved { emergency: e, .. }
            | EmergencyEventKind::EmergencySpread { emergency: e, .. }
            | EmergencyEventKind::PlanCreated { emergency: e, .. }
            | EmergencyEventKind::ResponderAssigned { emergency: e, .. }
            | EmergencyEventKind::ResponderReleased { emergency: e, .. }
            | EmergencyEventKind::CasualtyReported { emergency: e, .. }
            | EmergencyEventKind::DamageReported { emergency: e, .. } => *e == emergency,
            _ => false,
        }
    }

    #[must_use]
    pub fn involves_responder(&self, responder: ResponderId) -> bool {
        match &self.kind {
            EmergencyEventKind::ResponderAssigned { responder: r, .. }
            | EmergencyEventKind::ResponderArrived { responder: r, .. }
            | EmergencyEventKind::ResponderReleased { responder: r, .. }
            | EmergencyEventKind::ResponderIncapacitated { responder: r } => *r == responder,
            _ => false,
        }
    }

    #[must_use]
    pub fn involves_action(&self, action: ResponseActionId) -> bool {
        match &self.kind {
            EmergencyEventKind::ActionStarted { action: a, .. }
            | EmergencyEventKind::ActionProgress { action: a, .. }
            | EmergencyEventKind::ActionCompleted { action: a, .. }
            | EmergencyEventKind::ActionFailed { action: a, .. }
            | EmergencyEventKind::ResponderAssigned { action: a, .. } => *a == action,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = EmergencyEvent::emergency_detected(
            100,
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(5),
            EmergencySeverity::Moderate,
        );

        assert_eq!(event.tick, 100);
        assert!(matches!(
            event.kind,
            EmergencyEventKind::EmergencyDetected { .. }
        ));
    }

    #[test]
    fn test_event_involves_emergency() {
        let event = EmergencyEvent::emergency_detected(
            0,
            EmergencyId::new(1),
            EmergencyKind::Fire,
            RegionId::new(1),
            EmergencySeverity::Minor,
        );

        assert!(event.involves_emergency(EmergencyId::new(1)));
        assert!(!event.involves_emergency(EmergencyId::new(2)));
    }

    #[test]
    fn test_event_involves_responder() {
        let event = EmergencyEvent::responder_assigned(
            0,
            ResponderId::new(5),
            EmergencyId::new(1),
            ResponseActionId::new(1),
        );

        assert!(event.involves_responder(ResponderId::new(5)));
        assert!(!event.involves_responder(ResponderId::new(6)));
    }

    #[test]
    fn test_event_involves_action() {
        let event = EmergencyEvent::action_completed(0, ResponseActionId::new(3), 50);

        assert!(event.involves_action(ResponseActionId::new(3)));
        assert!(!event.involves_action(ResponseActionId::new(4)));
    }

    #[test]
    fn test_event_serde() {
        let event = EmergencyEvent::emergency_escalated(
            100,
            EmergencyId::new(1),
            EmergencySeverity::Minor,
            EmergencySeverity::Moderate,
        );

        let json = serde_json::to_string(&event).unwrap();
        let restored: EmergencyEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, restored);
    }
}

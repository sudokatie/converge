//! Lifecycle event types.

use super::state::{GrowthPhase, LifecycleId};
use serde::{Deserialize, Serialize};

/// Kind of lifecycle event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LifecycleEventKind {
    /// Egg was laid/spawned.
    EggLaid { id: LifecycleId },
    /// Egg began hatching process.
    HatchingStarted { id: LifecycleId },
    /// Creature successfully hatched from egg.
    Hatched {
        id: LifecycleId,
        initial_phase: GrowthPhase,
    },
    /// Egg failed to hatch (died).
    HatchingFailed { id: LifecycleId },
    /// Creature transitioned to a new growth phase.
    PhaseTransition {
        id: LifecycleId,
        from: GrowthPhase,
        to: GrowthPhase,
    },
    /// Creature began metamorphosis.
    MetamorphosisStarted {
        id: LifecycleId,
        from_phase: GrowthPhase,
    },
    /// Creature completed metamorphosis.
    MetamorphosisCompleted {
        id: LifecycleId,
        result_phase: GrowthPhase,
    },
    /// Creature died during metamorphosis.
    MetamorphosisFailed { id: LifecycleId },
    /// Creature died of natural causes (age, elder decline).
    NaturalDeath { id: LifecycleId, age: u64 },
    /// Creature spawned directly as living.
    Spawned { id: LifecycleId, phase: GrowthPhase },
    /// Corpse spawned.
    CorpseCreated { id: LifecycleId, biomass: f32 },
    /// Corpse fully decayed and was removed.
    CorpseDecayed { id: LifecycleId },
    /// Biomass released from decaying corpse.
    BiomassReleased {
        id: LifecycleId,
        amount: f32,
        remaining: f32,
    },
}

/// A lifecycle event with timestamp.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub tick: u64,
    pub kind: LifecycleEventKind,
}

impl LifecycleEvent {
    #[must_use]
    pub fn new(tick: u64, kind: LifecycleEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn id(&self) -> Option<LifecycleId> {
        match &self.kind {
            LifecycleEventKind::EggLaid { id }
            | LifecycleEventKind::HatchingStarted { id }
            | LifecycleEventKind::Hatched { id, .. }
            | LifecycleEventKind::HatchingFailed { id }
            | LifecycleEventKind::PhaseTransition { id, .. }
            | LifecycleEventKind::MetamorphosisStarted { id, .. }
            | LifecycleEventKind::MetamorphosisCompleted { id, .. }
            | LifecycleEventKind::MetamorphosisFailed { id }
            | LifecycleEventKind::NaturalDeath { id, .. }
            | LifecycleEventKind::Spawned { id, .. }
            | LifecycleEventKind::CorpseCreated { id, .. }
            | LifecycleEventKind::CorpseDecayed { id }
            | LifecycleEventKind::BiomassReleased { id, .. } => Some(*id),
        }
    }

    #[must_use]
    pub fn is_birth(&self) -> bool {
        matches!(
            self.kind,
            LifecycleEventKind::Hatched { .. } | LifecycleEventKind::Spawned { .. }
        )
    }

    #[must_use]
    pub fn is_death(&self) -> bool {
        matches!(
            self.kind,
            LifecycleEventKind::NaturalDeath { .. }
                | LifecycleEventKind::HatchingFailed { .. }
                | LifecycleEventKind::MetamorphosisFailed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_event_new() {
        let event = LifecycleEvent::new(
            100,
            LifecycleEventKind::EggLaid {
                id: LifecycleId::new(1),
            },
        );
        assert_eq!(event.tick, 100);
        assert_eq!(event.id(), Some(LifecycleId::new(1)));
    }

    #[test]
    fn test_lifecycle_event_is_birth() {
        let hatched = LifecycleEvent::new(
            0,
            LifecycleEventKind::Hatched {
                id: LifecycleId::new(1),
                initial_phase: GrowthPhase::Juvenile,
            },
        );
        assert!(hatched.is_birth());

        let spawned = LifecycleEvent::new(
            0,
            LifecycleEventKind::Spawned {
                id: LifecycleId::new(2),
                phase: GrowthPhase::Adult,
            },
        );
        assert!(spawned.is_birth());

        let egg = LifecycleEvent::new(
            0,
            LifecycleEventKind::EggLaid {
                id: LifecycleId::new(3),
            },
        );
        assert!(!egg.is_birth());
    }

    #[test]
    fn test_lifecycle_event_is_death() {
        let death = LifecycleEvent::new(
            100,
            LifecycleEventKind::NaturalDeath {
                id: LifecycleId::new(1),
                age: 5000,
            },
        );
        assert!(death.is_death());

        let hatch_fail = LifecycleEvent::new(
            50,
            LifecycleEventKind::HatchingFailed {
                id: LifecycleId::new(2),
            },
        );
        assert!(hatch_fail.is_death());

        let transition = LifecycleEvent::new(
            200,
            LifecycleEventKind::PhaseTransition {
                id: LifecycleId::new(3),
                from: GrowthPhase::Juvenile,
                to: GrowthPhase::Adult,
            },
        );
        assert!(!transition.is_death());
    }

    #[test]
    fn test_lifecycle_event_kind_serde() {
        let kind = LifecycleEventKind::Hatched {
            id: LifecycleId::new(42),
            initial_phase: GrowthPhase::Juvenile,
        };
        let json = serde_json::to_string(&kind).unwrap();
        let restored: LifecycleEventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, kind);
    }

    #[test]
    fn test_lifecycle_event_serde() {
        let event = LifecycleEvent::new(
            500,
            LifecycleEventKind::PhaseTransition {
                id: LifecycleId::new(1),
                from: GrowthPhase::Juvenile,
                to: GrowthPhase::Adult,
            },
        );
        let json = serde_json::to_string(&event).unwrap();
        let restored: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tick, 500);
    }
}

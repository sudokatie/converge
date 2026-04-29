//! Trigger types for block behavior activation.

use serde::{Deserialize, Serialize};

use crate::chunk::BlockId;
use crate::environment::HazardKind;

/// Events that can trigger block behavior evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BehaviorTrigger {
    /// Player uses/interacts with the block.
    Use,

    /// Block is being mined/destroyed.
    Mine,

    /// Block was just placed.
    Place,

    /// A neighbor block changed (placed, removed, or modified).
    NeighborChanged {
        /// Filter for specific neighbor directions (-1, 0, 1 for each axis; None = any).
        direction: Option<(i8, i8, i8)>,
        /// Filter for specific block type that appeared.
        block_filter: Option<BlockId>,
    },

    /// Periodic tick update.
    Tick {
        /// Minimum ticks since last trigger (0 = every tick).
        interval: u32,
    },

    /// Block exposed to environmental hazard.
    HazardExposure {
        /// Which hazard kind triggers this (None = any hazard).
        hazard_kind: Option<HazardKind>,
        /// Minimum intensity threshold (0.0-1.0).
        min_intensity: f32,
    },

    /// Block contacted by fluid.
    FluidContact {
        /// Which fluid kind triggers this.
        fluid_kind: u8,
    },

    /// Automation signal received on the block.
    SignalReceived {
        /// Minimum signal strength (0 = any non-zero).
        min_strength: i32,
    },

    /// Entity collides with the block.
    EntityCollision {
        /// Filter for entity kind (0 = any, other values are entity type IDs).
        entity_kind: u32,
    },

    /// Random tick (for growth, decay, etc.).
    RandomTick {
        /// Probability 0.0-1.0 that this trigger activates on any given random tick.
        probability: f32,
    },
}

impl BehaviorTrigger {
    /// Check if this trigger matches the given event.
    #[must_use]
    pub fn matches(&self, event: &TriggerEventKind) -> bool {
        match (self, event) {
            (Self::Use, TriggerEventKind::Use)
            | (Self::Mine, TriggerEventKind::Mine)
            | (Self::Place, TriggerEventKind::Place) => true,

            (
                Self::NeighborChanged {
                    direction,
                    block_filter,
                },
                TriggerEventKind::NeighborChanged {
                    direction: dir,
                    new_block,
                },
            ) => {
                let dir_match = direction.is_none_or(|d| d == *dir);
                let block_match = block_filter.is_none_or(|b| b == *new_block);
                dir_match && block_match
            }

            (
                Self::Tick { interval },
                TriggerEventKind::Tick {
                    ticks_since_last, ..
                },
            ) => *ticks_since_last >= *interval,

            (
                Self::HazardExposure {
                    hazard_kind,
                    min_intensity,
                },
                TriggerEventKind::HazardExposure { kind, intensity },
            ) => {
                let kind_match = hazard_kind.is_none_or(|k| k == *kind);
                kind_match && *intensity >= *min_intensity
            }

            (Self::FluidContact { fluid_kind }, TriggerEventKind::FluidContact { kind, .. }) => {
                *fluid_kind == *kind
            }

            (
                Self::SignalReceived { min_strength },
                TriggerEventKind::SignalReceived { strength },
            ) => *strength >= *min_strength,

            (Self::EntityCollision { entity_kind }, TriggerEventKind::EntityCollision { kind }) => {
                *entity_kind == 0 || *entity_kind == *kind
            }

            (Self::RandomTick { .. }, TriggerEventKind::RandomTick { activated }) => *activated,

            _ => false,
        }
    }

    /// Get the discriminant for deterministic ordering.
    #[must_use]
    pub const fn discriminant(&self) -> u8 {
        match self {
            Self::Use => 0,
            Self::Mine => 1,
            Self::Place => 2,
            Self::NeighborChanged { .. } => 3,
            Self::Tick { .. } => 4,
            Self::HazardExposure { .. } => 5,
            Self::FluidContact { .. } => 6,
            Self::SignalReceived { .. } => 7,
            Self::EntityCollision { .. } => 8,
            Self::RandomTick { .. } => 9,
        }
    }

    /// Feed trigger data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "enum indices fit in u32")]
    pub fn feed_checksum(&self, hasher: &mut crate::ChecksumBuilder) {
        hasher.feed_u32(u32::from(self.discriminant()));

        match self {
            Self::Use | Self::Mine | Self::Place => {}

            Self::NeighborChanged {
                direction,
                block_filter,
            } => {
                if let Some((dx, dy, dz)) = direction {
                    hasher.feed_u32(1);
                    hasher.feed_i32(i32::from(*dx));
                    hasher.feed_i32(i32::from(*dy));
                    hasher.feed_i32(i32::from(*dz));
                } else {
                    hasher.feed_u32(0);
                }
                if let Some(b) = block_filter {
                    hasher.feed_u32(1);
                    hasher.feed_u32(u32::from(b.raw()));
                } else {
                    hasher.feed_u32(0);
                }
            }

            Self::Tick { interval } => {
                hasher.feed_u32(*interval);
            }

            Self::HazardExposure {
                hazard_kind,
                min_intensity,
            } => {
                if let Some(k) = hazard_kind {
                    hasher.feed_u32(1);
                    hasher.feed_u32(k.as_index() as u32);
                } else {
                    hasher.feed_u32(0);
                }
                hasher.feed_f32(*min_intensity);
            }

            Self::FluidContact { fluid_kind } => {
                hasher.feed_u32(u32::from(*fluid_kind));
            }

            Self::SignalReceived { min_strength } => {
                hasher.feed_i32(*min_strength);
            }

            Self::EntityCollision { entity_kind } => {
                hasher.feed_u32(*entity_kind);
            }

            Self::RandomTick { probability } => {
                hasher.feed_f32(*probability);
            }
        }
    }
}

/// Runtime trigger event data passed to the evaluator.
#[derive(Clone, Debug, PartialEq)]
pub enum TriggerEventKind {
    /// Player use interaction.
    Use,

    /// Mining event.
    Mine,

    /// Placement event.
    Place,

    /// Neighbor change event.
    NeighborChanged {
        direction: (i8, i8, i8),
        new_block: BlockId,
    },

    /// Tick event.
    Tick {
        current_tick: u64,
        ticks_since_last: u32,
    },

    /// Hazard exposure event.
    HazardExposure { kind: HazardKind, intensity: f32 },

    /// Fluid contact event.
    FluidContact { kind: u8, level: f32 },

    /// Signal received event.
    SignalReceived { strength: i32 },

    /// Entity collision event.
    EntityCollision { kind: u32 },

    /// Random tick event (with pre-computed activation).
    RandomTick { activated: bool },
}

impl TriggerEventKind {
    /// Get the discriminant for matching.
    #[must_use]
    pub const fn discriminant(&self) -> u8 {
        match self {
            Self::Use => 0,
            Self::Mine => 1,
            Self::Place => 2,
            Self::NeighborChanged { .. } => 3,
            Self::Tick { .. } => 4,
            Self::HazardExposure { .. } => 5,
            Self::FluidContact { .. } => 6,
            Self::SignalReceived { .. } => 7,
            Self::EntityCollision { .. } => 8,
            Self::RandomTick { .. } => 9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::FluidKind;

    #[test]
    fn trigger_use_matches() {
        let trigger = BehaviorTrigger::Use;
        assert!(trigger.matches(&TriggerEventKind::Use));
        assert!(!trigger.matches(&TriggerEventKind::Mine));
    }

    #[test]
    fn trigger_neighbor_any_direction() {
        let trigger = BehaviorTrigger::NeighborChanged {
            direction: None,
            block_filter: None,
        };
        let event = TriggerEventKind::NeighborChanged {
            direction: (1, 0, 0),
            new_block: BlockId(5),
        };
        assert!(trigger.matches(&event));
    }

    #[test]
    fn trigger_neighbor_specific_direction() {
        let trigger = BehaviorTrigger::NeighborChanged {
            direction: Some((0, 1, 0)),
            block_filter: None,
        };
        let event_match = TriggerEventKind::NeighborChanged {
            direction: (0, 1, 0),
            new_block: BlockId(5),
        };
        let event_no_match = TriggerEventKind::NeighborChanged {
            direction: (1, 0, 0),
            new_block: BlockId(5),
        };
        assert!(trigger.matches(&event_match));
        assert!(!trigger.matches(&event_no_match));
    }

    #[test]
    fn trigger_tick_interval() {
        let trigger = BehaviorTrigger::Tick { interval: 10 };
        let event_enough = TriggerEventKind::Tick {
            current_tick: 100,
            ticks_since_last: 15,
        };
        let event_not_enough = TriggerEventKind::Tick {
            current_tick: 100,
            ticks_since_last: 5,
        };
        assert!(trigger.matches(&event_enough));
        assert!(!trigger.matches(&event_not_enough));
    }

    #[test]
    fn trigger_hazard_intensity() {
        let trigger = BehaviorTrigger::HazardExposure {
            hazard_kind: Some(HazardKind::Fire),
            min_intensity: 0.5,
        };
        let event_strong = TriggerEventKind::HazardExposure {
            kind: HazardKind::Fire,
            intensity: 0.8,
        };
        let event_weak = TriggerEventKind::HazardExposure {
            kind: HazardKind::Fire,
            intensity: 0.3,
        };
        let event_wrong_kind = TriggerEventKind::HazardExposure {
            kind: HazardKind::Frost,
            intensity: 0.8,
        };
        assert!(trigger.matches(&event_strong));
        assert!(!trigger.matches(&event_weak));
        assert!(!trigger.matches(&event_wrong_kind));
    }

    #[test]
    #[expect(clippy::cast_possible_truncation)]
    fn trigger_fluid_contact() {
        let trigger = BehaviorTrigger::FluidContact {
            fluid_kind: FluidKind::Water.as_index() as u8,
        };
        let event_match = TriggerEventKind::FluidContact {
            kind: FluidKind::Water.as_index() as u8,
            level: 1.0,
        };
        let event_no_match = TriggerEventKind::FluidContact {
            kind: FluidKind::Lava.as_index() as u8,
            level: 1.0,
        };
        assert!(trigger.matches(&event_match));
        assert!(!trigger.matches(&event_no_match));
    }

    #[test]
    fn trigger_entity_any() {
        let trigger = BehaviorTrigger::EntityCollision { entity_kind: 0 };
        let event = TriggerEventKind::EntityCollision { kind: 42 };
        assert!(trigger.matches(&event));
    }

    #[test]
    fn trigger_entity_specific() {
        let trigger = BehaviorTrigger::EntityCollision { entity_kind: 42 };
        let event_match = TriggerEventKind::EntityCollision { kind: 42 };
        let event_no_match = TriggerEventKind::EntityCollision { kind: 99 };
        assert!(trigger.matches(&event_match));
        assert!(!trigger.matches(&event_no_match));
    }

    #[test]
    fn serde_round_trip() {
        let triggers = [
            BehaviorTrigger::Use,
            BehaviorTrigger::Mine,
            BehaviorTrigger::Place,
            BehaviorTrigger::NeighborChanged {
                direction: Some((1, 0, -1)),
                block_filter: Some(BlockId(10)),
            },
            BehaviorTrigger::Tick { interval: 20 },
            BehaviorTrigger::HazardExposure {
                hazard_kind: Some(HazardKind::Fire),
                min_intensity: 0.25,
            },
            BehaviorTrigger::FluidContact { fluid_kind: 1 },
            BehaviorTrigger::SignalReceived { min_strength: 5 },
            BehaviorTrigger::EntityCollision { entity_kind: 100 },
            BehaviorTrigger::RandomTick { probability: 0.1 },
        ];

        for trigger in &triggers {
            let json = serde_json::to_string(trigger).unwrap();
            let recovered: BehaviorTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(*trigger, recovered);
        }
    }
}

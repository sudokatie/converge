//! Actions and effects produced by block behaviors.

use serde::{Deserialize, Serialize};

use crate::chunk::BlockId;
use crate::environment::{FluidKind, HazardKind};

/// Actions that can be emitted when a behavior activates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BehaviorAction {
    /// Transform this block into another type.
    TransformBlock { new_block: BlockId },

    /// Remove this block (set to air).
    DestroyBlock,

    /// Spawn an item drop.
    DropItem { item_id: u32, count: u8 },

    /// Emit a particle effect.
    EmitParticle { particle_id: u32, count: u8 },

    /// Play a sound.
    PlaySound { sound_id: u32 },

    /// Emit an automation signal.
    EmitSignal { strength: i32 },

    /// Apply damage to nearby entities.
    DamageEntities { radius: f32, damage: f32 },

    /// Spawn a hazard at this location.
    SpawnHazard { kind: HazardKind, intensity: f32 },

    /// Spawn fluid at this location.
    SpawnFluid { kind: FluidKind, volume: f32 },

    /// Set block metadata.
    SetMetadata { key: String, value: i32 },

    /// Increment block metadata (with wrapping).
    IncrementMetadata { key: String, amount: i32, max: i32 },

    /// Trigger a neighbor update cascade.
    NotifyNeighbors,

    /// Queue this block for another tick evaluation.
    ScheduleTick { delay: u32 },

    /// Chain to another node in the graph.
    ChainToNode { node_id: u32 },

    /// Apply an effect to colliding entity.
    ApplyEntityEffect { effect_id: u32, duration: u32 },

    /// Emit a custom event for external systems.
    EmitEvent { event_kind: u32, payload: Vec<u8> },
}

impl BehaviorAction {
    /// Get the discriminant for deterministic ordering.
    #[must_use]
    pub const fn discriminant(&self) -> u8 {
        match self {
            Self::TransformBlock { .. } => 0,
            Self::DestroyBlock => 1,
            Self::DropItem { .. } => 2,
            Self::EmitParticle { .. } => 3,
            Self::PlaySound { .. } => 4,
            Self::EmitSignal { .. } => 5,
            Self::DamageEntities { .. } => 6,
            Self::SpawnHazard { .. } => 7,
            Self::SpawnFluid { .. } => 8,
            Self::SetMetadata { .. } => 9,
            Self::IncrementMetadata { .. } => 10,
            Self::NotifyNeighbors => 11,
            Self::ScheduleTick { .. } => 12,
            Self::ChainToNode { .. } => 13,
            Self::ApplyEntityEffect { .. } => 14,
            Self::EmitEvent { .. } => 15,
        }
    }

    /// Check if this action modifies block state.
    #[must_use]
    pub const fn modifies_block(&self) -> bool {
        matches!(
            self,
            Self::TransformBlock { .. }
                | Self::DestroyBlock
                | Self::SetMetadata { .. }
                | Self::IncrementMetadata { .. }
        )
    }

    /// Check if this action affects other blocks.
    #[must_use]
    pub const fn affects_neighbors(&self) -> bool {
        matches!(
            self,
            Self::NotifyNeighbors
                | Self::SpawnHazard { .. }
                | Self::SpawnFluid { .. }
                | Self::DamageEntities { .. }
        )
    }

    /// Check if this is a chain action.
    #[must_use]
    pub const fn is_chain(&self) -> bool {
        matches!(self, Self::ChainToNode { .. })
    }

    /// Feed action data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "enum indices fit in u32")]
    pub fn feed_checksum(&self, hasher: &mut crate::ChecksumBuilder) {
        hasher.feed_u32(u32::from(self.discriminant()));

        match self {
            Self::TransformBlock { new_block } => {
                hasher.feed_u32(u32::from(new_block.raw()));
            }

            Self::DestroyBlock | Self::NotifyNeighbors => {}

            Self::DropItem { item_id, count } => {
                hasher.feed_u32(*item_id);
                hasher.feed_u32(u32::from(*count));
            }

            Self::EmitParticle { particle_id, count } => {
                hasher.feed_u32(*particle_id);
                hasher.feed_u32(u32::from(*count));
            }

            Self::PlaySound { sound_id } => {
                hasher.feed_u32(*sound_id);
            }

            Self::EmitSignal { strength } => {
                hasher.feed_i32(*strength);
            }

            Self::DamageEntities { radius, damage } => {
                hasher.feed_f32(*radius);
                hasher.feed_f32(*damage);
            }

            Self::SpawnHazard { kind, intensity } => {
                hasher.feed_u32(kind.as_index() as u32);
                hasher.feed_f32(*intensity);
            }

            Self::SpawnFluid { kind, volume } => {
                hasher.feed_u32(kind.as_index() as u32);
                hasher.feed_f32(*volume);
            }

            Self::SetMetadata { key, value } => {
                hasher.feed_str(key);
                hasher.feed_i32(*value);
            }

            Self::IncrementMetadata { key, amount, max } => {
                hasher.feed_str(key);
                hasher.feed_i32(*amount);
                hasher.feed_i32(*max);
            }

            Self::ScheduleTick { delay } => {
                hasher.feed_u32(*delay);
            }

            Self::ChainToNode { node_id } => {
                hasher.feed_u32(*node_id);
            }

            Self::ApplyEntityEffect {
                effect_id,
                duration,
            } => {
                hasher.feed_u32(*effect_id);
                hasher.feed_u32(*duration);
            }

            Self::EmitEvent {
                event_kind,
                payload,
            } => {
                hasher.feed_u32(*event_kind);
                hasher.feed_bytes(payload);
            }
        }
    }
}

/// Kind of effect produced by behavior evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectKind {
    /// Block transformation occurred.
    BlockChanged,
    /// Block was destroyed.
    BlockDestroyed,
    /// Item was dropped.
    ItemDropped,
    /// Particle effect emitted.
    ParticleEmitted,
    /// Sound played.
    SoundPlayed,
    /// Signal emitted.
    SignalEmitted,
    /// Entities damaged.
    EntitiesDamaged,
    /// Hazard spawned.
    HazardSpawned,
    /// Fluid spawned.
    FluidSpawned,
    /// Metadata changed.
    MetadataChanged,
    /// Neighbors notified.
    NeighborsNotified,
    /// Tick scheduled.
    TickScheduled,
    /// Entity effect applied.
    EntityEffectApplied,
    /// Custom event emitted.
    EventEmitted,
}

/// A concrete effect produced by evaluating a behavior action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehaviorEffect {
    /// The kind of effect.
    pub kind: EffectKind,
    /// Source node that produced this effect.
    pub source_node: u32,
    /// Additional data depending on effect kind.
    pub data: EffectData,
}

/// Effect-specific data payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EffectData {
    /// No additional data.
    None,
    /// Block ID for transformations.
    Block(BlockId),
    /// Item drop data.
    ItemDrop { item_id: u32, count: u8 },
    /// Particle emission data.
    Particle { particle_id: u32, count: u8 },
    /// Sound ID.
    Sound(u32),
    /// Signal strength.
    Signal(i32),
    /// Damage data.
    Damage { radius: f32, damage: f32 },
    /// Hazard data.
    Hazard { kind: HazardKind, intensity: f32 },
    /// Fluid data.
    Fluid { kind: FluidKind, volume: f32 },
    /// Metadata change.
    Metadata { key: String, value: i32 },
    /// Tick delay.
    TickDelay(u32),
    /// Entity effect data.
    EntityEffect { effect_id: u32, duration: u32 },
    /// Custom event data.
    Event { event_kind: u32, payload: Vec<u8> },
}

impl BehaviorEffect {
    /// Create a block changed effect.
    #[must_use]
    pub fn block_changed(source_node: u32, new_block: BlockId) -> Self {
        Self {
            kind: EffectKind::BlockChanged,
            source_node,
            data: EffectData::Block(new_block),
        }
    }

    /// Create a block destroyed effect.
    #[must_use]
    pub fn block_destroyed(source_node: u32) -> Self {
        Self {
            kind: EffectKind::BlockDestroyed,
            source_node,
            data: EffectData::None,
        }
    }

    /// Create an item dropped effect.
    #[must_use]
    pub fn item_dropped(source_node: u32, item_id: u32, count: u8) -> Self {
        Self {
            kind: EffectKind::ItemDropped,
            source_node,
            data: EffectData::ItemDrop { item_id, count },
        }
    }

    /// Create a signal emitted effect.
    #[must_use]
    pub fn signal_emitted(source_node: u32, strength: i32) -> Self {
        Self {
            kind: EffectKind::SignalEmitted,
            source_node,
            data: EffectData::Signal(strength),
        }
    }

    /// Feed effect data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "enum indices fit in u32")]
    pub fn feed_checksum(&self, hasher: &mut crate::ChecksumBuilder) {
        hasher.feed_u32(self.kind as u32);
        hasher.feed_u32(self.source_node);

        match &self.data {
            EffectData::None => {}
            EffectData::Block(id) => {
                hasher.feed_u32(u32::from(id.raw()));
            }
            EffectData::ItemDrop { item_id, count } => {
                hasher.feed_u32(*item_id);
                hasher.feed_u32(u32::from(*count));
            }
            EffectData::Particle { particle_id, count } => {
                hasher.feed_u32(*particle_id);
                hasher.feed_u32(u32::from(*count));
            }
            EffectData::Sound(id) => {
                hasher.feed_u32(*id);
            }
            EffectData::Signal(strength) => {
                hasher.feed_i32(*strength);
            }
            EffectData::Damage { radius, damage } => {
                hasher.feed_f32(*radius);
                hasher.feed_f32(*damage);
            }
            EffectData::Hazard { kind, intensity } => {
                hasher.feed_u32(kind.as_index() as u32);
                hasher.feed_f32(*intensity);
            }
            EffectData::Fluid { kind, volume } => {
                hasher.feed_u32(kind.as_index() as u32);
                hasher.feed_f32(*volume);
            }
            EffectData::Metadata { key, value } => {
                hasher.feed_str(key);
                hasher.feed_i32(*value);
            }
            EffectData::TickDelay(delay) => {
                hasher.feed_u32(*delay);
            }
            EffectData::EntityEffect {
                effect_id,
                duration,
            } => {
                hasher.feed_u32(*effect_id);
                hasher.feed_u32(*duration);
            }
            EffectData::Event {
                event_kind,
                payload,
            } => {
                hasher.feed_u32(*event_kind);
                hasher.feed_bytes(payload);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_modifies_block() {
        assert!(
            BehaviorAction::TransformBlock {
                new_block: BlockId(1)
            }
            .modifies_block()
        );
        assert!(BehaviorAction::DestroyBlock.modifies_block());
        assert!(
            BehaviorAction::SetMetadata {
                key: "test".into(),
                value: 1
            }
            .modifies_block()
        );
        assert!(!BehaviorAction::PlaySound { sound_id: 1 }.modifies_block());
    }

    #[test]
    fn action_affects_neighbors() {
        assert!(BehaviorAction::NotifyNeighbors.affects_neighbors());
        assert!(
            BehaviorAction::SpawnHazard {
                kind: HazardKind::Fire,
                intensity: 0.5
            }
            .affects_neighbors()
        );
        assert!(!BehaviorAction::PlaySound { sound_id: 1 }.affects_neighbors());
    }

    #[test]
    fn action_is_chain() {
        assert!(BehaviorAction::ChainToNode { node_id: 5 }.is_chain());
        assert!(!BehaviorAction::DestroyBlock.is_chain());
    }

    #[test]
    fn effect_creation() {
        let effect = BehaviorEffect::block_changed(1, BlockId(42));
        assert_eq!(effect.kind, EffectKind::BlockChanged);
        assert_eq!(effect.source_node, 1);

        let effect = BehaviorEffect::signal_emitted(2, 15);
        assert_eq!(effect.kind, EffectKind::SignalEmitted);
        if let EffectData::Signal(s) = effect.data {
            assert_eq!(s, 15);
        } else {
            panic!("wrong data type");
        }
    }

    #[test]
    fn serde_round_trip_action() {
        let actions = [
            BehaviorAction::TransformBlock {
                new_block: BlockId(10),
            },
            BehaviorAction::DestroyBlock,
            BehaviorAction::DropItem {
                item_id: 5,
                count: 3,
            },
            BehaviorAction::EmitSignal { strength: 15 },
            BehaviorAction::SpawnHazard {
                kind: HazardKind::Fire,
                intensity: 0.8,
            },
            BehaviorAction::SetMetadata {
                key: "state".into(),
                value: 2,
            },
            BehaviorAction::ChainToNode { node_id: 42 },
        ];

        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let recovered: BehaviorAction = serde_json::from_str(&json).unwrap();
            assert_eq!(*action, recovered);
        }
    }

    #[test]
    fn serde_round_trip_effect() {
        let effects = [
            BehaviorEffect::block_changed(1, BlockId(5)),
            BehaviorEffect::block_destroyed(2),
            BehaviorEffect::item_dropped(3, 100, 5),
            BehaviorEffect::signal_emitted(4, 10),
        ];

        for effect in &effects {
            let json = serde_json::to_string(effect).unwrap();
            let recovered: BehaviorEffect = serde_json::from_str(&json).unwrap();
            assert_eq!(*effect, recovered);
        }
    }
}

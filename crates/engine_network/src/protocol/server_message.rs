//! Messages sent from server to client.

use engine_core::coords::{ChunkPos, WorldPos};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Types of entities that can be synchronized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    Player,
    Pig,
    Cow,
    Sheep,
    Chicken,
    Zombie,
    Skeleton,
    Spider,
    Creeper,
    DroppedItem,
}

/// Snapshot of a single entity's state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySnapshot {
    /// Entity network ID.
    pub id: u64,
    /// Entity type.
    pub kind: EntityKind,
    /// World position.
    pub position: Vec3,
    /// Rotation quaternion.
    pub rotation: Quat,
    /// Linear velocity.
    pub velocity: Vec3,
    /// Health (for creatures).
    pub health: Option<f32>,
}

/// Complete world snapshot for a client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// Server tick number.
    pub tick: u64,
    /// Last acknowledged client input sequence.
    pub ack_sequence: u32,
    /// Player's authoritative position.
    pub player_position: Vec3,
    /// Player's authoritative velocity.
    pub player_velocity: Vec3,
    /// Nearby entity states.
    pub entities: Vec<EntitySnapshot>,
}

/// Messages sent from server to client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Full world snapshot (sent at tick rate).
    Snapshot(WorldSnapshot),
    
    /// Entity spawned in view.
    EntitySpawn {
        /// Network entity ID.
        id: u64,
        /// Entity type.
        kind: EntityKind,
        /// Initial position.
        position: Vec3,
        /// Initial rotation.
        rotation: Quat,
    },
    
    /// Entity despawned or left view.
    EntityDespawn {
        /// Network entity ID.
        id: u64,
    },
    
    /// Entity position/rotation update (for entities not in snapshot).
    EntityUpdate {
        /// Network entity ID.
        id: u64,
        /// New position.
        position: Vec3,
        /// New rotation.
        rotation: Quat,
    },
    
    /// Block changed in world.
    BlockChange {
        /// World position of change.
        pos: WorldPos,
        /// New block type.
        block: u16,
    },
    
    /// Chunk data response.
    ChunkData {
        /// Chunk position.
        pos: ChunkPos,
        /// Compressed chunk data.
        data: Vec<u8>,
    },
    
    /// Chat message received.
    ChatReceive {
        /// Sender name.
        sender: String,
        /// Message content.
        message: String,
    },
    
    /// Player joined notification.
    PlayerJoined {
        /// Player's network ID.
        id: u64,
        /// Player's display name.
        name: String,
    },
    
    /// Player left notification.
    PlayerLeft {
        /// Player's network ID.
        id: u64,
    },
    
    /// Connection accepted with player's assigned ID.
    Welcome {
        /// Assigned player ID.
        player_id: u64,
        /// Server tick rate in Hz.
        tick_rate: u32,
        /// World seed.
        seed: u64,
    },
}

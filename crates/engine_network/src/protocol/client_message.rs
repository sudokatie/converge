//! Messages sent from client to server.

use engine_core::coords::{ChunkPos, WorldPos};
use serde::{Deserialize, Serialize};

use glam::Vec3;

/// Input state sent from client each tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputState {
    /// Movement direction (normalized or zero).
    pub movement: Vec3,
    /// Whether jump is pressed.
    pub jump: bool,
    /// Whether sprint is pressed.
    pub sprint: bool,
    /// Camera yaw in radians.
    pub yaw: f32,
    /// Camera pitch in radians.
    pub pitch: f32,
    /// Sequence number for reconciliation.
    pub sequence: u32,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            movement: Vec3::ZERO,
            jump: false,
            sprint: false,
            yaw: 0.0,
            pitch: 0.0,
            sequence: 0,
        }
    }
}

/// Messages sent from client to server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Player input state (sent every tick).
    Input(InputState),
    
    /// Request to place a block.
    BlockPlace {
        /// World position to place at.
        pos: WorldPos,
        /// Block type to place.
        block: u16,
    },
    
    /// Request to break a block.
    BlockBreak {
        /// World position to break.
        pos: WorldPos,
    },
    
    /// Send a chat message.
    ChatSend {
        /// Message content.
        message: String,
    },
    
    /// Request chunk data.
    ChunkRequest {
        /// Chunk position to request.
        pos: ChunkPos,
    },
}

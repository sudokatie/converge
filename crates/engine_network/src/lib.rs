//! Networking system for the Lattice game engine.
//!
//! Provides client-server architecture, state synchronization, and prediction.
//!
//! # Architecture
//!
//! The networking system uses a client-server model with:
//! - **Transport**: UDP-based networking using renet
//! - **Protocol**: Defined message types for client/server communication
//! - **Channels**: Unreliable (inputs/snapshots), reliable (chat/blocks), chunk data
//! - **Prediction**: Client-side movement prediction with server reconciliation
//!
//! # Example
//!
//! ```ignore
//! // Server
//! let mut server = GameServer::new(27015)?;
//! loop {
//!     server.update(dt);
//!     for (client_id, msg) in server.receive() {
//!         // Handle messages
//!     }
//!     server.broadcast(&ServerMessage::Snapshot(snapshot))?;
//!     server.send_packets();
//! }
//!
//! // Client
//! let mut client = GameClient::connect("127.0.0.1:27015")?;
//! let mut predictor = MovementPredictor::new();
//! loop {
//!     client.update(dt);
//!     
//!     // Send input with sequence number
//!     let seq = predictor.next_sequence();
//!     let input = InputState { sequence: seq, ..input };
//!     client.send(&ClientMessage::Input(input.clone()))?;
//!     
//!     // Apply locally and record
//!     apply_movement(&mut state, &input, dt);
//!     predictor.record_input(input, dt, &state);
//!     
//!     // Handle server messages
//!     for msg in client.receive() {
//!         if let ServerMessage::Snapshot(snap) = msg {
//!             // Reconcile with server state
//!             if let Some(replay) = predictor.reconcile(&snap.auth_state) {
//!                 for record in replay {
//!                     apply_movement(&mut state, &record.input, record.dt);
//!                 }
//!                 predictor.apply_replay_result(state.clone());
//!             }
//!         }
//!     }
//!     
//!     client.send_packets();
//! }
//! ```

pub mod prediction;
pub mod protocol;
pub mod sync;
pub mod transport;

pub use prediction::{InputRecord, MovementPredictor, PredictedState};
pub use protocol::{ClientMessage, EntityKind, ServerMessage, WorldSnapshot};
pub use sync::{
    ChunkPriority, ChunkRequest, ClientChunkSync, InterpolatedState, InterpolationBuffer,
    ServerChunkSync,
};
pub use transport::{ClientId, GameClient, GameServer, DEFAULT_PORT};

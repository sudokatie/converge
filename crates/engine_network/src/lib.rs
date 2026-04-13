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
//! loop {
//!     client.update(dt);
//!     for msg in client.receive() {
//!         // Handle messages
//!     }
//!     client.send(&ClientMessage::Input(input))?;
//!     client.send_packets();
//! }
//! ```

pub mod protocol;
pub mod sync;
pub mod transport;

pub use protocol::{ClientMessage, EntityKind, ServerMessage, WorldSnapshot};
pub use sync::{InterpolatedState, InterpolationBuffer};
pub use transport::{ClientId, GameClient, GameServer, DEFAULT_PORT};

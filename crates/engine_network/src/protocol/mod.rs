//! Network protocol messages for client-server communication.

pub mod client_message;
pub mod server_message;

pub use client_message::{ClientMessage, InputState};
pub use server_message::{EntityKind, EntitySnapshot, ServerMessage, WorldSnapshot};

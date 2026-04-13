//! Network protocol messages for client-server communication.

mod client_message;
mod server_message;

pub use client_message::{ClientMessage, InputState};
pub use server_message::{EntityKind, ServerMessage, WorldSnapshot};

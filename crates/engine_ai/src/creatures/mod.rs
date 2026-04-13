//! Creature behavior systems.

mod hostile;
mod passive;

pub use hostile::{HostileAI, HostileAction, HostileState};
pub use passive::{PassiveAI, PassiveState};

//! Client-side prediction and server reconciliation.
//!
//! Implements client-side prediction to hide network latency and
//! server reconciliation to correct mispredictions.

mod movement;

pub use movement::{InputRecord, MovementPredictor, PredictedState};

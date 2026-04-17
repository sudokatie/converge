//! AI systems for creature behavior.

mod herd;
mod lod;

pub use herd::{
    HerdResult, HerdState, calculate_herd_behavior, cohesion_force, find_herd_leader,
    separation_force, HERD_FOLLOW_DISTANCE, HERD_FOLLOW_SPEED, HERD_MIN_DISTANCE,
};
pub use lod::{
    AiLodLevel, AiLodManager, AiLodState, FULL_AI_DISTANCE, SIMPLIFIED_AI_DISTANCE,
};

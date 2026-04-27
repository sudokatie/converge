//! AI systems for creature behavior.

mod herd;
mod lod;
mod ranged;

pub use herd::{
    HERD_FOLLOW_DISTANCE, HERD_FOLLOW_SPEED, HERD_MIN_DISTANCE, HerdResult, HerdState,
    calculate_herd_behavior, cohesion_force, find_herd_leader, separation_force,
};
pub use lod::{AiLodLevel, AiLodManager, AiLodState, FULL_AI_DISTANCE, SIMPLIFIED_AI_DISTANCE};
pub use ranged::{
    PROJECTILE_DAMAGE, PROJECTILE_LIFETIME, PROJECTILE_SPEED, Projectile, RANGED_ATTACK_COOLDOWN,
    RANGED_ATTACK_RANGE, RangedAttacker, RangedState,
};

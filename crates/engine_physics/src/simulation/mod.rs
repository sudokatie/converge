//! Physics simulation systems.

mod fall_damage;
mod player_movement;

pub use fall_damage::{
    DROWNING_DAMAGE_PER_SEC, DROWNING_GRACE_PERIOD, DrowningTracker, FALL_DAMAGE_PER_BLOCK,
    FALL_DAMAGE_THRESHOLD, FallDamageTracker, MAX_AIR_SUPPLY, calculate_fall_damage,
};
pub use player_movement::{PlayerPhysics, VoxelQuery};

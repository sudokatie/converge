//! Physics simulation systems.

mod fall_damage;
mod player_movement;

pub use fall_damage::{
    calculate_fall_damage, DrowningTracker, FallDamageTracker, DROWNING_DAMAGE_PER_SEC,
    DROWNING_GRACE_PERIOD, FALL_DAMAGE_PER_BLOCK, FALL_DAMAGE_THRESHOLD, MAX_AIR_SUPPLY,
};
pub use player_movement::{PlayerPhysics, VoxelQuery};

//! Entity factory functions.

mod creature;
mod player;
mod spawning;

pub use creature::{query_creatures, query_hostile, query_passive, spawn_creature, Creature, CreatureKind};
pub use player::spawn_player;
pub use spawning::{
    BiomeType, SpawnResult, SpawnSystem, MAX_SPAWN_DISTANCE, MAX_SPAWN_ATTEMPTS,
    MIN_SPAWN_DISTANCE, POPULATION_CAP, SPAWN_CHECK_INTERVAL,
};

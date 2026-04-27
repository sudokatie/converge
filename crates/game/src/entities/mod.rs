//! Entity factory functions.

mod creature;
mod player;
mod spawning;

pub use creature::{
    Creature, CreatureKind, query_creatures, query_hostile, query_passive, spawn_creature,
};
pub use player::spawn_player;
pub use spawning::{
    BiomeType, MAX_SPAWN_ATTEMPTS, MAX_SPAWN_DISTANCE, MIN_SPAWN_DISTANCE, POPULATION_CAP,
    SPAWN_CHECK_INTERVAL, SpawnResult, SpawnSystem,
};

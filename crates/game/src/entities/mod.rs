//! Entity factory functions.

mod creature;
mod player;

pub use creature::{query_creatures, query_hostile, query_passive, spawn_creature, Creature, CreatureKind};
pub use player::spawn_player;

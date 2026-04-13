//! Audio system for the Lattice game engine.
//!
//! Provides 3D positional audio, music, and sound effects using kira.

mod manager;
mod sound;

pub use manager::{AudioManager, VolumeCategory};
pub use sound::{SoundId, SoundRegistry};

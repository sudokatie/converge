//! Audio system for the Lattice game engine.
//!
//! Provides 3D positional audio, music, and sound effects using kira.

mod effects;
mod manager;
mod sound;

pub use effects::{
    AmbientSoundEvent, AudioEffectsExt, BlockSoundEvent, CombatSoundEvent, SoundEffects,
    SurfaceType, UiSoundEvent,
};
pub use manager::{AudioManager, VolumeCategory};
pub use sound::{SoundId, SoundRegistry};

//! Audio system for the Lattice game engine.
//!
//! Provides 3D positional audio, music, and sound effects using kira.

mod effects;
mod manager;
mod music;
mod sound;

pub use effects::{
    AmbientSoundEvent, AudioEffectsExt, BlockSoundEvent, CombatSoundEvent, SoundEffects,
    SurfaceType, UiSoundEvent,
};
pub use manager::{AudioManager, VolumeCategory};
pub use music::{MusicPlayRequest, MusicPlayer, MusicRegistry, MusicState, MusicTrack, TrackId};
pub use sound::{SoundId, SoundRegistry};

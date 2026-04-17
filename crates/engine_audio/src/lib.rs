//! Audio system for the Lattice game engine.
//!
//! Provides 3D positional audio, music, and sound effects using kira.

mod combat_music;
mod effects;
mod manager;
mod music;
mod sound;
mod sound_pool;

pub use combat_music::{
    CombatMusicController, CombatMusicState, COMBAT_COOLDOWN_SECS, COMBAT_RADIUS,
};
pub use effects::{
    AmbientSoundEvent, AudioEffectsExt, BlockSoundEvent, CombatSoundEvent, SoundEffects,
    SurfaceType, UiSoundEvent,
};
pub use manager::{AudioManager, VolumeCategory};
pub use music::{MusicPlayRequest, MusicPlayer, MusicRegistry, MusicState, MusicTrack, TrackId};
pub use sound::{SoundId, SoundRegistry};
pub use sound_pool::{PoolPick, SoundPool, SoundPoolRegistry, MAX_POOL_SIZE};

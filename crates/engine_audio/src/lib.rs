//! Audio system for the Lattice game engine.
//!
//! Provides 3D positional audio, music, and sound effects using kira.

mod ambient;
mod combat_music;
mod effects;
mod manager;
mod music;
mod sound;
mod sound_pool;

pub use ambient::{
    AMBIENT_CHECK_INTERVAL, AMBIENT_PLAY_CHANCE, AmbientBiome, AmbientSound, AmbientSoundController,
};
pub use combat_music::{
    COMBAT_COOLDOWN_SECS, COMBAT_RADIUS, CombatMusicController, CombatMusicState,
};
pub use effects::{
    AmbientSoundEvent, AudioEffectsExt, BlockSoundEvent, CombatSoundEvent, SoundEffects,
    SurfaceType, UiSoundEvent,
};
pub use manager::{AudioManager, VolumeCategory};
pub use music::{MusicPlayRequest, MusicPlayer, MusicRegistry, MusicState, MusicTrack, TrackId};
pub use sound::{SoundId, SoundRegistry};
pub use sound_pool::{MAX_POOL_SIZE, PoolPick, SoundPool, SoundPoolRegistry};

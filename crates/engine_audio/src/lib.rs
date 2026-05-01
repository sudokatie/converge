//! Audio system for the Lattice game engine.
//!
//! Provides 3D positional audio, music, and sound effects using kira.
//!
//! # Modules
//!
//! - [`occlusion`]: Audio obstruction/occlusion with material-aware propagation.
//! - [`reverb`]: Material-aware reverb zones and acoustic regions.

mod ambient;
mod combat_music;
mod dynamic_music;
mod effects;
mod manager;
mod music;
pub mod occlusion;
pub mod reverb;
mod sound;
mod sound_pool;

pub use ambient::{
    AMBIENT_CHECK_INTERVAL, AMBIENT_PLAY_CHANCE, AmbientBiome, AmbientSound, AmbientSoundController,
};
pub use combat_music::{
    COMBAT_COOLDOWN_SECS, COMBAT_RADIUS, CombatMusicController, CombatMusicState,
};
pub use dynamic_music::{
    DEFAULT_FADE_IN_SECS, DEFAULT_FADE_OUT_SECS, DEFAULT_HYSTERESIS_SECS, DynamicMusicController,
    EnvironmentPressure, LAYER_ACTIVE_THRESHOLD, LayerConfig, LayerMix, LayerProfile, LayerState,
    MAX_ACTIVE_LAYERS, MixLayer, MusicLayerKind, compute_mix_fingerprint,
    compute_profile_fingerprint, deserialize_mix, deserialize_pressure, deserialize_profile,
    serialize_mix, serialize_pressure, serialize_profile,
};
pub use effects::{
    AmbientSoundEvent, AudioEffectsExt, BlockSoundEvent, CombatSoundEvent, SoundEffects,
    SurfaceType, UiSoundEvent,
};
pub use manager::{AudioManager, VolumeCategory};
pub use music::{MusicPlayRequest, MusicPlayer, MusicRegistry, MusicState, MusicTrack, TrackId};
pub use sound::{SoundId, SoundRegistry};
pub use sound_pool::{MAX_POOL_SIZE, PoolPick, SoundPool, SoundPoolRegistry};

pub use occlusion::{
    AcousticMaterial, MaterialProfile, MaterialStackSummary, ObstructionPath, OcclusionResult,
    OcclusionSample, compute_occlusion, compute_path_fingerprint, deserialize_path,
    deserialize_result, serialize_path, serialize_result,
};
pub use reverb::{
    ReverbConfig, ReverbPreset, ReverbSample, ReverbZone, ReverbZoneId, ReverbZoneRegistry,
    ZoneShape, compute_registry_fingerprint, compute_zone_fingerprint, deserialize_registry,
    deserialize_zone, sample_reverb, sample_reverb_priority, serialize_registry, serialize_zone,
};

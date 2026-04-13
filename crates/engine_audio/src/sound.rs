//! Sound definitions and registry.

use std::collections::HashMap;
use std::path::Path;

use kira::sound::static_sound::StaticSoundData;
use thiserror::Error;

/// Sound identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(pub u16);

impl SoundId {
    // Common sound IDs
    pub const BLOCK_PLACE: SoundId = SoundId(1);
    pub const BLOCK_BREAK: SoundId = SoundId(2);
    pub const FOOTSTEP_GRASS: SoundId = SoundId(10);
    pub const FOOTSTEP_STONE: SoundId = SoundId(11);
    pub const FOOTSTEP_WOOD: SoundId = SoundId(12);
    pub const FOOTSTEP_SAND: SoundId = SoundId(13);
    pub const HIT_PLAYER: SoundId = SoundId(20);
    pub const HIT_CREATURE: SoundId = SoundId(21);
    pub const DEATH: SoundId = SoundId(22);
    pub const EAT: SoundId = SoundId(30);
    pub const PICKUP: SoundId = SoundId(31);
    pub const UI_CLICK: SoundId = SoundId(40);
    pub const UI_OPEN: SoundId = SoundId(41);
    pub const UI_CLOSE: SoundId = SoundId(42);
}

/// Errors that can occur when loading sounds.
#[derive(Debug, Error)]
pub enum SoundError {
    #[error("Failed to load sound file: {0}")]
    LoadFailed(String),
    #[error("Sound not found: {0:?}")]
    NotFound(SoundId),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A loaded sound ready for playback.
#[derive(Clone)]
pub struct LoadedSound {
    /// The sound data.
    pub data: StaticSoundData,
    /// Default volume (0.0 to 1.0).
    pub default_volume: f32,
    /// Whether this is a looping sound.
    pub looping: bool,
}

impl LoadedSound {
    /// Create a new loaded sound.
    pub fn new(data: StaticSoundData) -> Self {
        Self {
            data,
            default_volume: 1.0,
            looping: false,
        }
    }

    /// Set default volume.
    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.default_volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Set as looping.
    #[must_use]
    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }
}

/// Registry of loaded sounds.
pub struct SoundRegistry {
    sounds: HashMap<SoundId, LoadedSound>,
}

impl SoundRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sounds: HashMap::new(),
        }
    }

    /// Load a sound from a file path.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be loaded.
    pub fn load(&mut self, id: SoundId, path: &Path) -> Result<(), SoundError> {
        let data = StaticSoundData::from_file(path)
            .map_err(|e| SoundError::LoadFailed(format!("{}: {}", path.display(), e)))?;

        self.sounds.insert(id, LoadedSound::new(data));
        Ok(())
    }

    /// Load a sound with custom settings.
    pub fn load_with_settings(
        &mut self,
        id: SoundId,
        path: &Path,
        volume: f32,
        looping: bool,
    ) -> Result<(), SoundError> {
        let data = StaticSoundData::from_file(path)
            .map_err(|e| SoundError::LoadFailed(format!("{}: {}", path.display(), e)))?;

        let mut sound = LoadedSound::new(data).with_volume(volume);
        if looping {
            sound = sound.looping();
        }

        self.sounds.insert(id, sound);
        Ok(())
    }

    /// Register a pre-loaded sound.
    pub fn register(&mut self, id: SoundId, sound: LoadedSound) {
        self.sounds.insert(id, sound);
    }

    /// Get a sound by ID.
    #[must_use]
    pub fn get(&self, id: SoundId) -> Option<&LoadedSound> {
        self.sounds.get(&id)
    }

    /// Check if a sound is registered.
    #[must_use]
    pub fn contains(&self, id: SoundId) -> bool {
        self.sounds.contains_key(&id)
    }

    /// Number of registered sounds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// Clear all sounds.
    pub fn clear(&mut self) {
        self.sounds.clear();
    }
}

impl Default for SoundRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_id_constants() {
        assert_eq!(SoundId::BLOCK_PLACE.0, 1);
        assert_eq!(SoundId::UI_CLICK.0, 40);
    }

    #[test]
    fn test_sound_registry_new() {
        let registry = SoundRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_sound_registry_contains() {
        let registry = SoundRegistry::new();
        assert!(!registry.contains(SoundId::BLOCK_PLACE));
    }

    // Note: Loading actual audio files would require test assets
    // These tests verify the API structure
}

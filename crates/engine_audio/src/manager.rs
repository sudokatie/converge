//! Audio manager for playing sounds and music.

use glam::Vec3;
use kira::{
    manager::{AudioManager as KiraManager, AudioManagerSettings, backend::DefaultBackend},
    sound::static_sound::StaticSoundHandle,
    tween::{Tween, Value},
    Volume,
};
use std::collections::HashMap;
use thiserror::Error;

use crate::sound::{SoundId, SoundRegistry};

/// Volume categories for separate volume controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VolumeCategory {
    /// Master volume (affects all sounds).
    Master,
    /// Sound effects volume.
    Effects,
    /// Music volume.
    Music,
    /// Ambient sounds volume.
    Ambient,
    /// UI sounds volume.
    Ui,
}

impl Default for VolumeCategory {
    fn default() -> Self {
        Self::Effects
    }
}

/// Errors from audio operations.
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to initialize audio backend")]
    InitFailed,
    #[error("Sound not found: {0:?}")]
    SoundNotFound(SoundId),
    #[error("Playback error")]
    PlaybackError,
}

/// Listener position and orientation for 3D audio.
#[derive(Clone, Debug, Default)]
pub struct Listener {
    /// World position.
    pub position: Vec3,
    /// Forward direction (normalized).
    pub forward: Vec3,
    /// Up direction (normalized).
    pub up: Vec3,
}

impl Listener {
    /// Create a new listener.
    #[must_use]
    pub fn new(position: Vec3, forward: Vec3) -> Self {
        Self {
            position,
            forward: forward.normalize_or_zero(),
            up: Vec3::Y,
        }
    }

    /// Update listener transform.
    pub fn set_transform(&mut self, position: Vec3, forward: Vec3) {
        self.position = position;
        self.forward = forward.normalize_or_zero();
    }
}

/// Playing sound instance.
struct PlayingSound {
    handle: StaticSoundHandle,
    category: VolumeCategory,
}

/// Main audio manager.
pub struct AudioManager {
    /// Kira audio manager.
    manager: Option<KiraManager>,
    /// Sound registry.
    sounds: SoundRegistry,
    /// Volume levels per category.
    volumes: HashMap<VolumeCategory, f32>,
    /// Currently playing sounds.
    playing: Vec<PlayingSound>,
    /// Listener position for 3D audio.
    listener: Listener,
    /// Whether audio is enabled.
    enabled: bool,
}

impl AudioManager {
    /// Create a new audio manager.
    ///
    /// # Errors
    ///
    /// Returns error if audio backend fails to initialize.
    pub fn new() -> Result<Self, AudioError> {
        let settings = AudioManagerSettings::default();
        let manager = KiraManager::<DefaultBackend>::new(settings)
            .map_err(|_| AudioError::InitFailed)?;

        let mut volumes = HashMap::new();
        volumes.insert(VolumeCategory::Master, 1.0);
        volumes.insert(VolumeCategory::Effects, 1.0);
        volumes.insert(VolumeCategory::Music, 0.7);
        volumes.insert(VolumeCategory::Ambient, 0.8);
        volumes.insert(VolumeCategory::Ui, 1.0);

        Ok(Self {
            manager: Some(manager),
            sounds: SoundRegistry::new(),
            volumes,
            playing: Vec::new(),
            listener: Listener::default(),
            enabled: true,
        })
    }

    /// Create a dummy audio manager (no sound output).
    #[must_use]
    pub fn dummy() -> Self {
        let mut volumes = HashMap::new();
        volumes.insert(VolumeCategory::Master, 1.0);
        volumes.insert(VolumeCategory::Effects, 1.0);
        volumes.insert(VolumeCategory::Music, 0.7);
        volumes.insert(VolumeCategory::Ambient, 0.8);
        volumes.insert(VolumeCategory::Ui, 1.0);

        Self {
            manager: None,
            sounds: SoundRegistry::new(),
            volumes,
            playing: Vec::new(),
            listener: Listener::default(),
            enabled: false,
        }
    }

    /// Check if audio is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.manager.is_some()
    }

    /// Enable or disable audio.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if audio is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.manager.is_some()
    }

    /// Get the sound registry for loading sounds.
    #[must_use]
    pub fn sounds(&self) -> &SoundRegistry {
        &self.sounds
    }

    /// Get mutable sound registry.
    pub fn sounds_mut(&mut self) -> &mut SoundRegistry {
        &mut self.sounds
    }

    /// Set volume for a category.
    pub fn set_volume(&mut self, category: VolumeCategory, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.volumes.insert(category, volume);

        // Update playing sounds in this category
        self.update_playing_volumes();
    }

    /// Get volume for a category.
    #[must_use]
    pub fn volume(&self, category: VolumeCategory) -> f32 {
        *self.volumes.get(&category).unwrap_or(&1.0)
    }

    /// Calculate effective volume for a category.
    fn effective_volume(&self, category: VolumeCategory) -> f32 {
        let master = self.volume(VolumeCategory::Master);
        let category_vol = self.volume(category);
        master * category_vol
    }

    /// Update volumes for currently playing sounds.
    fn update_playing_volumes(&mut self) {
        // Pre-compute volumes for each category
        let master = self.volume(VolumeCategory::Master);
        let effects = master * self.volume(VolumeCategory::Effects);
        let music = master * self.volume(VolumeCategory::Music);
        let ambient = master * self.volume(VolumeCategory::Ambient);
        let ui = master * self.volume(VolumeCategory::Ui);

        for playing in &mut self.playing {
            let vol = match playing.category {
                VolumeCategory::Master => master,
                VolumeCategory::Effects => effects,
                VolumeCategory::Music => music,
                VolumeCategory::Ambient => ambient,
                VolumeCategory::Ui => ui,
            };
            let _ = playing.handle.set_volume(Volume::Amplitude(vol as f64), Tween::default());
        }
    }

    /// Set listener position and direction.
    pub fn set_listener(&mut self, position: Vec3, forward: Vec3) {
        self.listener.set_transform(position, forward);
    }

    /// Get listener position.
    #[must_use]
    pub fn listener(&self) -> &Listener {
        &self.listener
    }

    /// Play a sound.
    ///
    /// Returns handle ID or None if sound not found.
    pub fn play(&mut self, id: SoundId) -> Option<usize> {
        self.play_with_category(id, VolumeCategory::Effects)
    }

    /// Play a sound with specific category.
    pub fn play_with_category(&mut self, id: SoundId, category: VolumeCategory) -> Option<usize> {
        if !self.is_enabled() {
            return None;
        }

        // Get sound data and calculate volume before borrowing manager
        let sound = self.sounds.get(id)?;
        let base_volume = sound.default_volume;
        let data = sound.data.clone();
        let effective_vol = self.effective_volume(category);

        let manager = self.manager.as_mut()?;
        let volume = effective_vol * base_volume;
        let mut play_data = data;
        play_data.settings.volume = Value::Fixed(Volume::Amplitude(volume as f64));

        let handle = manager.play(play_data).ok()?;
        let idx = self.playing.len();
        self.playing.push(PlayingSound { handle, category });

        Some(idx)
    }

    /// Play a sound at a 3D position.
    ///
    /// Volume is attenuated based on distance from listener.
    pub fn play_at(&mut self, id: SoundId, position: Vec3) -> Option<usize> {
        if !self.is_enabled() {
            return None;
        }

        // Calculate distance-based attenuation
        let distance = self.listener.position.distance(position);
        let attenuation = calculate_attenuation(distance);

        if attenuation < 0.01 {
            return None; // Too quiet to play
        }

        // Get sound data and calculate volume before borrowing manager
        let sound = self.sounds.get(id)?;
        let base_volume = sound.default_volume;
        let data = sound.data.clone();
        let effective_vol = self.effective_volume(VolumeCategory::Effects);

        let manager = self.manager.as_mut()?;
        let volume = effective_vol * base_volume * attenuation;
        let mut play_data = data;
        play_data.settings.volume = Value::Fixed(Volume::Amplitude(volume as f64));

        // TODO: Add panning based on position relative to listener

        let handle = manager.play(play_data).ok()?;
        let idx = self.playing.len();
        self.playing.push(PlayingSound {
            handle,
            category: VolumeCategory::Effects,
        });

        Some(idx)
    }

    /// Stop all sounds.
    pub fn stop_all(&mut self) {
        for playing in &mut self.playing {
            let _ = playing.handle.stop(Tween::default());
        }
        self.playing.clear();
    }

    /// Clean up finished sounds.
    pub fn cleanup(&mut self) {
        self.playing.retain(|p| p.handle.state() != kira::sound::PlaybackState::Stopped);
    }

    /// Number of currently playing sounds.
    #[must_use]
    pub fn playing_count(&self) -> usize {
        self.playing.len()
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::dummy())
    }
}

/// Calculate distance-based attenuation.
///
/// Uses inverse distance model with reference distance of 1 and max distance of 50.
fn calculate_attenuation(distance: f32) -> f32 {
    const REF_DISTANCE: f32 = 1.0;
    const MAX_DISTANCE: f32 = 50.0;
    const ROLLOFF: f32 = 1.0;

    if distance <= REF_DISTANCE {
        1.0
    } else if distance >= MAX_DISTANCE {
        0.0
    } else {
        REF_DISTANCE / (REF_DISTANCE + ROLLOFF * (distance - REF_DISTANCE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_category_default() {
        assert_eq!(VolumeCategory::default(), VolumeCategory::Effects);
    }

    #[test]
    fn test_listener_new() {
        let listener = Listener::new(Vec3::new(1.0, 2.0, 3.0), Vec3::X);
        assert_eq!(listener.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((listener.forward - Vec3::X).length() < 0.001);
    }

    #[test]
    fn test_attenuation_at_ref_distance() {
        assert!((calculate_attenuation(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_attenuation_at_max_distance() {
        assert!((calculate_attenuation(50.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_attenuation_falloff() {
        let near = calculate_attenuation(5.0);
        let far = calculate_attenuation(20.0);
        assert!(near > far);
    }

    #[test]
    fn test_dummy_manager() {
        let manager = AudioManager::dummy();
        assert!(!manager.is_available());
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_volume_settings() {
        let mut manager = AudioManager::dummy();

        manager.set_volume(VolumeCategory::Master, 0.5);
        assert!((manager.volume(VolumeCategory::Master) - 0.5).abs() < 0.001);

        // Clamp to valid range
        manager.set_volume(VolumeCategory::Effects, 1.5);
        assert!((manager.volume(VolumeCategory::Effects) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_set_listener() {
        let mut manager = AudioManager::dummy();
        manager.set_listener(Vec3::new(10.0, 5.0, 10.0), Vec3::Z);

        assert_eq!(manager.listener().position, Vec3::new(10.0, 5.0, 10.0));
    }

    #[test]
    fn test_play_on_dummy() {
        let mut manager = AudioManager::dummy();
        // Should return None, not crash
        assert!(manager.play(SoundId::BLOCK_PLACE).is_none());
    }
}

//! Music playback system with crossfading.
//!
//! Manages background music tracks with smooth transitions.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use kira::sound::static_sound::StaticSoundData;
use thiserror::Error;

/// Music track identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TrackId(pub u16);

impl TrackId {
    // Common track IDs
    pub const MENU: TrackId = TrackId(1);
    pub const OVERWORLD_DAY: TrackId = TrackId(10);
    pub const OVERWORLD_NIGHT: TrackId = TrackId(11);
    pub const UNDERGROUND: TrackId = TrackId(20);
    pub const COMBAT: TrackId = TrackId(30);
    pub const BOSS: TrackId = TrackId(31);
    pub const VICTORY: TrackId = TrackId(40);
    pub const DEATH: TrackId = TrackId(41);
}

/// Default crossfade duration.
pub const DEFAULT_CROSSFADE: Duration = Duration::from_secs(2);

/// Errors from music operations.
#[derive(Debug, Error)]
pub enum MusicError {
    #[error("Failed to load music file: {0}")]
    LoadFailed(String),
    #[error("Track not found: {0:?}")]
    TrackNotFound(TrackId),
    #[error("Playback error")]
    PlaybackError,
}

/// State of music playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicState {
    /// No music playing.
    Stopped,
    /// Music playing normally.
    Playing,
    /// Crossfading between tracks.
    Crossfading,
    /// Music paused.
    Paused,
}

/// A loaded music track.
#[derive(Clone)]
pub struct MusicTrack {
    /// Track data.
    pub data: StaticSoundData,
    /// Display name.
    pub name: String,
    /// Whether this track loops.
    pub looping: bool,
}

impl MusicTrack {
    /// Create a new music track.
    #[must_use]
    pub fn new(data: StaticSoundData, name: impl Into<String>) -> Self {
        Self {
            data,
            name: name.into(),
            looping: true,
        }
    }

    /// Set looping behavior.
    #[must_use]
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}

/// Registry of music tracks.
pub struct MusicRegistry {
    tracks: HashMap<TrackId, MusicTrack>,
}

impl MusicRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
        }
    }

    /// Load a track from a file.
    pub fn load(&mut self, id: TrackId, path: &Path, name: &str) -> Result<(), MusicError> {
        let data = StaticSoundData::from_file(path)
            .map_err(|e| MusicError::LoadFailed(format!("{}: {}", path.display(), e)))?;

        self.tracks.insert(id, MusicTrack::new(data, name));
        Ok(())
    }

    /// Register a pre-loaded track.
    pub fn register(&mut self, id: TrackId, track: MusicTrack) {
        self.tracks.insert(id, track);
    }

    /// Get a track by ID.
    #[must_use]
    pub fn get(&self, id: TrackId) -> Option<&MusicTrack> {
        self.tracks.get(&id)
    }

    /// Check if a track exists.
    #[must_use]
    pub fn contains(&self, id: TrackId) -> bool {
        self.tracks.contains_key(&id)
    }

    /// Number of loaded tracks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

impl Default for MusicRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Music playback request returned by MusicPlayer.
#[derive(Clone, Debug)]
pub struct MusicPlayRequest {
    /// Track ID to play.
    pub track_id: TrackId,
    /// Sound data to play.
    pub data: StaticSoundData,
    /// Target volume.
    pub volume: f32,
    /// Crossfade duration (if crossfading).
    pub crossfade: Option<Duration>,
}

/// Music state manager.
///
/// This is a state-only manager - actual playback is handled by AudioManager.
/// Call `request_play` to get data to pass to AudioManager.
pub struct MusicPlayer {
    /// Track registry.
    tracks: MusicRegistry,
    /// Currently playing track ID.
    current_track: Option<TrackId>,
    /// Current playback state.
    state: MusicState,
    /// Master music volume.
    volume: f32,
    /// Crossfade duration.
    crossfade_duration: Duration,
}

impl MusicPlayer {
    /// Create a new music player.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracks: MusicRegistry::new(),
            current_track: None,
            state: MusicState::Stopped,
            volume: 0.7,
            crossfade_duration: DEFAULT_CROSSFADE,
        }
    }

    /// Get the track registry.
    #[must_use]
    pub fn tracks(&self) -> &MusicRegistry {
        &self.tracks
    }

    /// Get mutable track registry.
    pub fn tracks_mut(&mut self) -> &mut MusicRegistry {
        &mut self.tracks
    }

    /// Get current playback state.
    #[must_use]
    pub fn state(&self) -> MusicState {
        self.state
    }

    /// Get currently playing track.
    #[must_use]
    pub fn current_track(&self) -> Option<TrackId> {
        self.current_track
    }

    /// Set music volume (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Get current volume.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Set crossfade duration.
    pub fn set_crossfade_duration(&mut self, duration: Duration) {
        self.crossfade_duration = duration;
    }

    /// Get crossfade duration.
    #[must_use]
    pub fn crossfade_duration(&self) -> Duration {
        self.crossfade_duration
    }

    /// Request to play a track immediately.
    ///
    /// Returns the play request data, or None if track not found.
    pub fn request_play(&mut self, id: TrackId) -> Option<MusicPlayRequest> {
        let track = self.tracks.get(id)?;

        let mut data = track.data.clone();
        if track.looping {
            data = data.loop_region(..);
        }

        self.current_track = Some(id);
        self.state = MusicState::Playing;

        Some(MusicPlayRequest {
            track_id: id,
            data,
            volume: self.volume,
            crossfade: None,
        })
    }

    /// Request to crossfade to a new track.
    ///
    /// Returns the play request data, or None if same track or not found.
    pub fn request_crossfade(&mut self, id: TrackId) -> Option<MusicPlayRequest> {
        // If same track, do nothing
        if self.current_track == Some(id) {
            return None;
        }

        let track = self.tracks.get(id)?;

        let mut data = track.data.clone();
        if track.looping {
            data = data.loop_region(..);
        }

        self.current_track = Some(id);
        self.state = MusicState::Crossfading;

        Some(MusicPlayRequest {
            track_id: id,
            data,
            volume: self.volume,
            crossfade: Some(self.crossfade_duration),
        })
    }

    /// Mark music as stopped.
    pub fn stop(&mut self) {
        self.current_track = None;
        self.state = MusicState::Stopped;
    }

    /// Mark music as paused.
    pub fn pause(&mut self) {
        self.state = MusicState::Paused;
    }

    /// Mark music as resumed.
    pub fn resume(&mut self) {
        if self.current_track.is_some() {
            self.state = MusicState::Playing;
        }
    }

    /// Mark crossfade as complete.
    pub fn crossfade_complete(&mut self) {
        if self.state == MusicState::Crossfading {
            self.state = MusicState::Playing;
        }
    }

    /// Mark track as finished.
    pub fn track_finished(&mut self) {
        self.current_track = None;
        self.state = MusicState::Stopped;
    }

    /// Check if music is currently playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        matches!(self.state, MusicState::Playing | MusicState::Crossfading)
    }
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_id_constants() {
        assert_eq!(TrackId::MENU.0, 1);
        assert_eq!(TrackId::OVERWORLD_DAY.0, 10);
        assert_eq!(TrackId::COMBAT.0, 30);
    }

    #[test]
    fn test_music_registry_new() {
        let registry = MusicRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_music_registry_contains() {
        let registry = MusicRegistry::new();
        assert!(!registry.contains(TrackId::MENU));
    }

    #[test]
    fn test_music_player_new() {
        let player = MusicPlayer::new();
        assert_eq!(player.state(), MusicState::Stopped);
        assert!(player.current_track().is_none());
    }

    #[test]
    fn test_music_player_volume() {
        let mut player = MusicPlayer::new();

        player.set_volume(0.5);
        assert!((player.volume() - 0.5).abs() < 0.001);

        // Clamp to valid range
        player.set_volume(1.5);
        assert!((player.volume() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_music_player_crossfade_duration() {
        let mut player = MusicPlayer::new();
        player.set_crossfade_duration(Duration::from_secs(3));
        // Just verify it doesn't crash
    }

    #[test]
    fn test_music_state_transitions() {
        // Verify enum variants exist
        assert_ne!(MusicState::Stopped, MusicState::Playing);
        assert_ne!(MusicState::Playing, MusicState::Crossfading);
        assert_ne!(MusicState::Paused, MusicState::Stopped);
    }

    #[test]
    fn test_is_playing() {
        let player = MusicPlayer::new();
        assert!(!player.is_playing());
    }
}

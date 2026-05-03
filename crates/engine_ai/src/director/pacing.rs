//! Adaptive pacing profiles for director AI.

use super::ids::PacingProfileId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Level of pacing intensity.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PacingLevel {
    /// Very easy, minimal challenges.
    Calm,
    /// Light challenges, recovery time.
    Easy,
    /// Balanced challenges and rest.
    #[default]
    Normal,
    /// Increased pressure and tension.
    Tense,
    /// High pressure, frequent challenges.
    Intense,
    /// Maximum pressure, survival mode.
    Extreme,
}

impl PacingLevel {
    #[must_use]
    pub fn intensity(self) -> f32 {
        match self {
            Self::Calm => 0.1,
            Self::Easy => 0.3,
            Self::Normal => 0.5,
            Self::Tense => 0.7,
            Self::Intense => 0.85,
            Self::Extreme => 1.0,
        }
    }

    #[must_use]
    pub fn from_intensity(intensity: f32) -> Self {
        match intensity {
            x if x < 0.2 => Self::Calm,
            x if x < 0.4 => Self::Easy,
            x if x < 0.6 => Self::Normal,
            x if x < 0.8 => Self::Tense,
            x if x < 0.95 => Self::Intense,
            _ => Self::Extreme,
        }
    }

    #[must_use]
    pub fn spawn_rate_multiplier(self) -> f32 {
        match self {
            Self::Calm => 0.3,
            Self::Easy => 0.6,
            Self::Normal => 1.0,
            Self::Tense => 1.3,
            Self::Intense => 1.6,
            Self::Extreme => 2.0,
        }
    }

    #[must_use]
    pub fn event_frequency_multiplier(self) -> f32 {
        match self {
            Self::Calm => 0.2,
            Self::Easy => 0.5,
            Self::Normal => 1.0,
            Self::Tense => 1.5,
            Self::Intense => 2.0,
            Self::Extreme => 3.0,
        }
    }

    #[must_use]
    pub fn resource_pressure_multiplier(self) -> f32 {
        match self {
            Self::Calm => 0.5,
            Self::Easy => 0.75,
            Self::Normal => 1.0,
            Self::Tense => 1.2,
            Self::Intense => 1.5,
            Self::Extreme => 2.0,
        }
    }
}

/// Definition of a pacing profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacingProfileDef {
    pub id: PacingProfileId,
    pub name: String,
    /// Target intensity (0.0 to 1.0).
    pub target_intensity: f32,
    /// Minimum intensity bound.
    pub min_intensity: f32,
    /// Maximum intensity bound.
    pub max_intensity: f32,
    /// Rate of intensity change per tick.
    pub change_rate: f32,
    /// How quickly to return to target.
    pub recovery_rate: f32,
    /// Multiplier for spawn events.
    pub spawn_multiplier: f32,
    /// Multiplier for disaster frequency.
    pub disaster_multiplier: f32,
    /// Grace period after disasters (ticks).
    pub post_disaster_grace: u64,
}

impl PacingProfileDef {
    #[must_use]
    pub fn new(id: impl Into<PacingProfileId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            target_intensity: 0.5,
            min_intensity: 0.1,
            max_intensity: 1.0,
            change_rate: 0.01,
            recovery_rate: 0.005,
            spawn_multiplier: 1.0,
            disaster_multiplier: 1.0,
            post_disaster_grace: 500,
        }
    }

    #[must_use]
    pub fn with_target_intensity(mut self, intensity: f32) -> Self {
        self.target_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_bounds(mut self, min: f32, max: f32) -> Self {
        self.min_intensity = min.clamp(0.0, 1.0);
        self.max_intensity = max.clamp(0.0, 1.0).max(self.min_intensity);
        self
    }

    #[must_use]
    pub fn with_change_rate(mut self, rate: f32) -> Self {
        self.change_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_multipliers(mut self, spawn: f32, disaster: f32) -> Self {
        self.spawn_multiplier = spawn.max(0.0);
        self.disaster_multiplier = disaster.max(0.0);
        self
    }

    #[must_use]
    pub fn with_post_disaster_grace(mut self, ticks: u64) -> Self {
        self.post_disaster_grace = ticks;
        self
    }
}

/// Preset pacing profiles.
pub mod presets {
    use super::PacingProfileDef;

    #[must_use]
    pub fn peaceful() -> PacingProfileDef {
        PacingProfileDef::new("peaceful", "Peaceful")
            .with_target_intensity(0.2)
            .with_bounds(0.05, 0.4)
            .with_multipliers(0.5, 0.3)
            .with_post_disaster_grace(1000)
    }

    #[must_use]
    pub fn normal() -> PacingProfileDef {
        PacingProfileDef::new("normal", "Normal")
            .with_target_intensity(0.5)
            .with_bounds(0.2, 0.8)
            .with_multipliers(1.0, 1.0)
            .with_post_disaster_grace(500)
    }

    #[must_use]
    pub fn challenging() -> PacingProfileDef {
        PacingProfileDef::new("challenging", "Challenging")
            .with_target_intensity(0.7)
            .with_bounds(0.4, 0.95)
            .with_multipliers(1.3, 1.5)
            .with_post_disaster_grace(300)
    }

    #[must_use]
    pub fn survival() -> PacingProfileDef {
        PacingProfileDef::new("survival", "Survival")
            .with_target_intensity(0.85)
            .with_bounds(0.6, 1.0)
            .with_multipliers(1.5, 2.0)
            .with_post_disaster_grace(100)
    }

    #[must_use]
    pub fn all_presets() -> Vec<PacingProfileDef> {
        vec![peaceful(), normal(), challenging(), survival()]
    }
}

/// Current pacing state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacingState {
    /// Active profile ID.
    pub profile_id: PacingProfileId,
    /// Current intensity (0.0 to 1.0).
    pub current_intensity: f32,
    /// Target intensity for this moment.
    pub target_intensity: f32,
    /// Current pacing level.
    pub level: PacingLevel,
    /// Ticks until grace period ends.
    pub grace_period_remaining: u64,
    /// Whether intensity is locked (manual override).
    pub locked: bool,
    /// Last adjustment tick.
    pub last_adjustment_tick: u64,
    /// Adjustment history for smoothing.
    smoothed_intensity: f32,
}

impl PacingState {
    #[must_use]
    pub fn new(profile_id: impl Into<PacingProfileId>, initial_intensity: f32) -> Self {
        let intensity = initial_intensity.clamp(0.0, 1.0);
        Self {
            profile_id: profile_id.into(),
            current_intensity: intensity,
            target_intensity: intensity,
            level: PacingLevel::from_intensity(intensity),
            grace_period_remaining: 0,
            locked: false,
            last_adjustment_tick: 0,
            smoothed_intensity: intensity,
        }
    }

    #[must_use]
    pub fn is_in_grace_period(&self) -> bool {
        self.grace_period_remaining > 0
    }

    pub fn start_grace_period(&mut self, ticks: u64) {
        self.grace_period_remaining = ticks;
    }

    pub fn tick_grace_period(&mut self) {
        self.grace_period_remaining = self.grace_period_remaining.saturating_sub(1);
    }

    pub fn lock(&mut self) {
        self.locked = true;
    }

    pub fn unlock(&mut self) {
        self.locked = false;
    }

    pub fn set_target(&mut self, target: f32) {
        self.target_intensity = target.clamp(0.0, 1.0);
    }

    pub fn adjust_toward_target(
        &mut self,
        rate: f32,
        min: f32,
        max: f32,
        smoothing: f32,
        tick: u64,
    ) {
        if self.locked {
            return;
        }

        let effective_target = if self.is_in_grace_period() {
            self.current_intensity.min(self.target_intensity)
        } else {
            self.target_intensity
        };

        let diff = effective_target - self.current_intensity;
        let adjustment = diff.signum() * diff.abs().min(rate);

        self.current_intensity = (self.current_intensity + adjustment).clamp(min, max);
        self.smoothed_intensity =
            self.smoothed_intensity * (1.0 - smoothing) + self.current_intensity * smoothing;
        self.level = PacingLevel::from_intensity(self.smoothed_intensity);
        self.last_adjustment_tick = tick;
    }

    #[must_use]
    pub fn effective_intensity(&self) -> f32 {
        self.smoothed_intensity
    }
}

impl Default for PacingState {
    fn default() -> Self {
        Self::new("normal", 0.5)
    }
}

/// Registry for pacing profiles.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PacingProfileRegistry {
    profiles: BTreeMap<PacingProfileId, PacingProfileDef>,
}

impl PacingProfileRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_presets() -> Self {
        let mut registry = Self::new();
        for preset in presets::all_presets() {
            registry.register(preset);
        }
        registry
    }

    pub fn register(&mut self, profile: PacingProfileDef) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    #[must_use]
    pub fn get(&self, id: &PacingProfileId) -> Option<&PacingProfileDef> {
        self.profiles.get(id)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.profiles.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PacingProfileDef> {
        self.profiles.values()
    }
}

/// Summary of pacing state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PacingSummary {
    pub tick: u64,
    pub profile_id: PacingProfileId,
    pub current_intensity: f32,
    pub effective_intensity: f32,
    pub level: PacingLevel,
    pub in_grace_period: bool,
    pub locked: bool,
}

impl PacingSummary {
    #[must_use]
    pub fn from_state(state: &PacingState, tick: u64) -> Self {
        Self {
            tick,
            profile_id: state.profile_id.clone(),
            current_intensity: state.current_intensity,
            effective_intensity: state.effective_intensity(),
            level: state.level,
            in_grace_period: state.is_in_grace_period(),
            locked: state.locked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacing_level_intensity() {
        assert!(PacingLevel::Calm.intensity() < PacingLevel::Normal.intensity());
        assert!(PacingLevel::Normal.intensity() < PacingLevel::Extreme.intensity());
    }

    #[test]
    fn test_pacing_level_from_intensity() {
        assert_eq!(PacingLevel::from_intensity(0.1), PacingLevel::Calm);
        assert_eq!(PacingLevel::from_intensity(0.5), PacingLevel::Normal);
        assert_eq!(PacingLevel::from_intensity(0.99), PacingLevel::Extreme);
    }

    #[test]
    fn test_pacing_level_multipliers() {
        assert!(PacingLevel::Calm.spawn_rate_multiplier() < 1.0);
        assert!((PacingLevel::Normal.spawn_rate_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!(PacingLevel::Extreme.spawn_rate_multiplier() > 1.0);
    }

    #[test]
    fn test_pacing_profile_def_new() {
        let profile = PacingProfileDef::new("test", "Test Profile")
            .with_target_intensity(0.6)
            .with_bounds(0.3, 0.9);

        assert_eq!(profile.id.as_str(), "test");
        assert!((profile.target_intensity - 0.6).abs() < f32::EPSILON);
        assert!((profile.min_intensity - 0.3).abs() < f32::EPSILON);
        assert!((profile.max_intensity - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pacing_presets() {
        let presets = presets::all_presets();
        assert!(!presets.is_empty());

        let peaceful = presets::peaceful();
        assert!(peaceful.target_intensity < 0.5);

        let survival = presets::survival();
        assert!(survival.target_intensity > 0.5);
    }

    #[test]
    fn test_pacing_state_new() {
        let state = PacingState::new("normal", 0.5);

        assert_eq!(state.profile_id.as_str(), "normal");
        assert!((state.current_intensity - 0.5).abs() < f32::EPSILON);
        assert_eq!(state.level, PacingLevel::Normal);
        assert!(!state.locked);
    }

    #[test]
    fn test_pacing_state_grace_period() {
        let mut state = PacingState::new("normal", 0.5);

        assert!(!state.is_in_grace_period());

        state.start_grace_period(100);
        assert!(state.is_in_grace_period());
        assert_eq!(state.grace_period_remaining, 100);

        state.tick_grace_period();
        assert_eq!(state.grace_period_remaining, 99);
    }

    #[test]
    fn test_pacing_state_lock() {
        let mut state = PacingState::new("normal", 0.5);

        state.lock();
        assert!(state.locked);

        state.set_target(0.8);
        state.adjust_toward_target(0.1, 0.0, 1.0, 0.2, 100);
        assert!((state.current_intensity - 0.5).abs() < f32::EPSILON);

        state.unlock();
        state.adjust_toward_target(0.1, 0.0, 1.0, 0.2, 200);
        assert!(state.current_intensity > 0.5);
    }

    #[test]
    fn test_pacing_state_adjust_toward_target() {
        let mut state = PacingState::new("normal", 0.3);

        state.set_target(0.7);
        for i in 0..50 {
            state.adjust_toward_target(0.02, 0.1, 0.9, 0.3, i);
        }

        assert!(state.current_intensity > 0.5);
        assert!(state.current_intensity <= 0.9);
    }

    #[test]
    fn test_pacing_state_grace_period_effect() {
        let mut state = PacingState::new("normal", 0.7);

        state.set_target(0.9);
        state.start_grace_period(10);

        state.adjust_toward_target(0.1, 0.0, 1.0, 0.2, 100);
        assert!(state.current_intensity <= 0.7);
    }

    #[test]
    fn test_pacing_profile_registry() {
        let registry = PacingProfileRegistry::with_presets();

        assert!(registry.count() >= 4);
        assert!(registry.get(&PacingProfileId::new("normal")).is_some());
        assert!(registry.get(&PacingProfileId::new("peaceful")).is_some());
    }

    #[test]
    fn test_pacing_summary() {
        let state = PacingState::new("challenging", 0.7);
        let summary = PacingSummary::from_state(&state, 100);

        assert_eq!(summary.profile_id.as_str(), "challenging");
        assert!((summary.current_intensity - 0.7).abs() < f32::EPSILON);
        assert!(!summary.in_grace_period);
    }

    #[test]
    fn test_serde_pacing_profile_def() {
        let profile = PacingProfileDef::new("test", "Test")
            .with_target_intensity(0.65)
            .with_multipliers(1.2, 1.5);

        let json = serde_json::to_string(&profile).unwrap();
        let restored: PacingProfileDef = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "test");
        assert!((restored.target_intensity - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_pacing_state() {
        let mut state = PacingState::new("survival", 0.8);
        state.start_grace_period(50);

        let bytes = bincode::serialize(&state).unwrap();
        let restored: PacingState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.profile_id.as_str(), "survival");
        assert!(restored.is_in_grace_period());
    }

    #[test]
    fn test_bincode_pacing_summary() {
        let summary = PacingSummary {
            tick: 500,
            profile_id: PacingProfileId::new("test"),
            current_intensity: 0.6,
            effective_intensity: 0.58,
            level: PacingLevel::Normal,
            in_grace_period: false,
            locked: false,
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: PacingSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 500);
        assert!((restored.current_intensity - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_pacing_profile_registry() {
        let registry = PacingProfileRegistry::with_presets();

        let bytes = bincode::serialize(&registry).unwrap();
        let restored: PacingProfileRegistry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.count(), registry.count());
    }
}

//! Ambient sound controller.
//!
//! Plays biome-appropriate ambient sounds (wind, birds, cave drips)
//! on a randomized schedule per spec 9.4.1.

use rand::Rng;

/// Default ambient check interval (seconds).
pub const AMBIENT_CHECK_INTERVAL: f32 = 8.0;

/// Chance of playing an ambient sound each check (0.0-1.0).
pub const AMBIENT_PLAY_CHANCE: f32 = 0.4;

/// Biome type for ambient sound selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmbientBiome {
    Plains,
    Forest,
    Desert,
    Mountains,
    Ocean,
    Swamp,
    Cave,
    Nether,
}

/// Ambient sound entry.
#[derive(Debug, Clone)]
pub struct AmbientSound {
    /// Display name.
    pub name: String,
    /// Sound ID in the registry.
    pub sound_id: u16,
    /// Volume (0.0-1.0).
    pub volume: f32,
}

/// Ambient sound controller state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientState {
    /// Playing an ambient sound.
    Playing,
    /// Waiting for next check.
    Waiting,
}

/// Manages ambient sound playback.
#[derive(Debug, Clone)]
pub struct AmbientSoundController {
    /// Current biome.
    current_biome: AmbientBiome,
    /// Time since last check.
    time_since_check: f32,
    /// Check interval.
    check_interval: f32,
    /// Whether ambient sounds are enabled.
    enabled: bool,
    /// Whether the player is underground.
    underground: bool,
    /// Time of day (0.0-1.0).
    time_of_day: f32,
}

impl Default for AmbientSoundController {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientSoundController {
    /// Create a new ambient sound controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_biome: AmbientBiome::Plains,
            time_since_check: 0.0,
            check_interval: AMBIENT_CHECK_INTERVAL,
            enabled: true,
            underground: false,
            time_of_day: 0.5,
        }
    }

    /// Set the current biome.
    pub fn set_biome(&mut self, biome: AmbientBiome) {
        self.current_biome = biome;
    }

    /// Set whether the player is underground.
    pub fn set_underground(&mut self, underground: bool) {
        self.underground = underground;
    }

    /// Set the time of day (0.0-1.0).
    pub fn set_time_of_day(&mut self, t: f32) {
        self.time_of_day = t.rem_euclid(1.0);
    }

    /// Enable or disable ambient sounds.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if ambient sounds are enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the list of ambient sounds for the current context.
    #[must_use]
    pub fn current_ambient_sounds(&self) -> Vec<AmbientSound> {
        if self.underground {
            return vec![
                AmbientSound { name: "cave_drip".into(), sound_id: 60, volume: 0.3 },
                AmbientSound { name: "cave_wind".into(), sound_id: 61, volume: 0.15 },
                AmbientSound { name: "lava_bubble".into(), sound_id: 62, volume: 0.2 },
            ];
        }

        let is_night = self.time_of_day < 0.25 || self.time_of_day > 0.75;

        match self.current_biome {
            AmbientBiome::Plains => {
                let mut sounds = vec![
                    AmbientSound { name: "wind".into(), sound_id: 51, volume: 0.2 },
                ];
                if !is_night {
                    sounds.push(AmbientSound { name: "bird".into(), sound_id: 63, volume: 0.25 });
                } else {
                    sounds.push(AmbientSound { name: "cricket".into(), sound_id: 64, volume: 0.15 });
                }
                sounds
            }
            AmbientBiome::Forest => {
                let mut sounds = vec![
                    AmbientSound { name: "wind_leaves".into(), sound_id: 65, volume: 0.25 },
                ];
                if !is_night {
                    sounds.push(AmbientSound { name: "bird".into(), sound_id: 63, volume: 0.3 });
                    sounds.push(AmbientSound { name: "woodpecker".into(), sound_id: 66, volume: 0.15 });
                } else {
                    sounds.push(AmbientSound { name: "owl".into(), sound_id: 67, volume: 0.2 });
                }
                sounds
            }
            AmbientBiome::Desert => {
                vec![
                    AmbientSound { name: "desert_wind".into(), sound_id: 68, volume: 0.3 },
                ]
            }
            AmbientBiome::Mountains => {
                vec![
                    AmbientSound { name: "mountain_wind".into(), sound_id: 69, volume: 0.35 },
                    AmbientSound { name: "eagle".into(), sound_id: 70, volume: 0.1 },
                ]
            }
            AmbientBiome::Ocean => {
                vec![
                    AmbientSound { name: "waves".into(), sound_id: 71, volume: 0.4 },
                    AmbientSound { name: "seagull".into(), sound_id: 72, volume: 0.15 },
                ]
            }
            AmbientBiome::Swamp => {
                vec![
                    AmbientSound { name: "swamp_bug".into(), sound_id: 73, volume: 0.2 },
                    AmbientSound { name: "frog".into(), sound_id: 74, volume: 0.15 },
                ]
            }
            AmbientBiome::Cave => {
                vec![
                    AmbientSound { name: "cave_drip".into(), sound_id: 60, volume: 0.3 },
                    AmbientSound { name: "cave_wind".into(), sound_id: 61, volume: 0.15 },
                ]
            }
            AmbientBiome::Nether => {
                vec![
                    AmbientSound { name: "nether_ambient".into(), sound_id: 75, volume: 0.3 },
                ]
            }
        }
    }

    /// Tick the ambient controller.
    ///
    /// Returns a sound to play if one should be triggered.
    #[must_use]
    pub fn tick(&mut self, dt: f32) -> Option<AmbientSound> {
        if !self.enabled {
            return None;
        }

        self.time_since_check += dt;

        if self.time_since_check < self.check_interval {
            return None;
        }

        self.time_since_check = 0.0;

        // Random chance to play
        let mut rng = rand::thread_rng();
        let roll: f32 = rng.gen_range(0.0..1.0);
        if roll > AMBIENT_PLAY_CHANCE {
            return None;
        }

        let sounds = self.current_ambient_sounds();
        if sounds.is_empty() {
            return None;
        }

        // Pick a random sound from the list
        let index = rng.gen_range(0..sounds.len());
        Some(sounds[index].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_controller() {
        let ctrl = AmbientSoundController::new();
        assert!(ctrl.is_enabled());
        assert!(!ctrl.underground);
    }

    #[test]
    fn test_plains_has_wind_and_birds() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_time_of_day(0.5); // Daytime
        let sounds = ctrl.current_ambient_sounds();
        assert!(sounds.iter().any(|s| s.name == "wind"));
        assert!(sounds.iter().any(|s| s.name == "bird"));
    }

    #[test]
    fn test_plains_night_has_crickets() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_time_of_day(0.0); // Night
        let sounds = ctrl.current_ambient_sounds();
        assert!(sounds.iter().any(|s| s.name == "cricket"));
        assert!(!sounds.iter().any(|s| s.name == "bird"));
    }

    #[test]
    fn test_underground_has_cave_sounds() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_underground(true);
        let sounds = ctrl.current_ambient_sounds();
        assert!(sounds.iter().any(|s| s.name == "cave_drip"));
    }

    #[test]
    fn test_ocean_has_waves() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_biome(AmbientBiome::Ocean);
        let sounds = ctrl.current_ambient_sounds();
        assert!(sounds.iter().any(|s| s.name == "waves"));
    }

    #[test]
    fn test_desert_has_wind() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_biome(AmbientBiome::Desert);
        let sounds = ctrl.current_ambient_sounds();
        assert!(sounds.iter().any(|s| s.name == "desert_wind"));
    }

    #[test]
    fn test_tick_returns_none_before_interval() {
        let mut ctrl = AmbientSoundController::new();
        assert!(ctrl.tick(2.0).is_none());
    }

    #[test]
    fn test_tick_disabled() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_enabled(false);
        assert!(ctrl.tick(10.0).is_none());
    }

    #[test]
    fn test_biome_variants() {
        assert_ne!(AmbientBiome::Plains, AmbientBiome::Forest);
        assert_ne!(AmbientBiome::Cave, AmbientBiome::Nether);
    }

    #[test]
    fn test_time_of_day_wraps() {
        let mut ctrl = AmbientSoundController::new();
        ctrl.set_time_of_day(1.5);
        assert!((ctrl.time_of_day - 0.5).abs() < 0.001);
    }
}

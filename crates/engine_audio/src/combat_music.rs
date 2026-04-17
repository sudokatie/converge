//! Combat music trigger system.
//!
//! Detects combat situations and triggers combat music with
//! a cooldown to avoid frequent track switching.

/// Default combat detection radius.
pub const COMBAT_RADIUS: f32 = 20.0;

/// Time without combat before reverting to ambient music.
pub const COMBAT_COOLDOWN_SECS: f32 = 5.0;

/// Combat music state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatMusicState {
    /// No combat, ambient music playing.
    Peaceful,
    /// Combat detected, combat music playing.
    InCombat,
    /// Combat ended, waiting for cooldown before reverting.
    Cooldown,
}

/// Tracks combat activity and triggers music changes.
#[derive(Debug, Clone)]
pub struct CombatMusicController {
    /// Current state.
    state: CombatMusicState,
    /// Time since last combat event (seconds).
    time_since_combat: f32,
    /// Cooldown duration before reverting to peaceful music.
    cooldown_duration: f32,
    /// Number of hostile entities currently in combat range.
    nearby_hostiles: u32,
    /// Whether combat music is enabled.
    enabled: bool,
}

impl Default for CombatMusicController {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatMusicController {
    /// Create a new combat music controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: CombatMusicState::Peaceful,
            time_since_combat: 0.0,
            cooldown_duration: COMBAT_COOLDOWN_SECS,
            nearby_hostiles: 0,
            enabled: true,
        }
    }

    /// Get the current combat music state.
    #[must_use]
    pub fn state(&self) -> CombatMusicState {
        self.state
    }

    /// Check if combat music is currently playing.
    #[must_use]
    pub fn is_in_combat(&self) -> bool {
        self.state == CombatMusicState::InCombat
    }

    /// Enable or disable combat music.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.state = CombatMusicState::Peaceful;
            self.nearby_hostiles = 0;
        }
    }

    /// Set the cooldown duration.
    pub fn set_cooldown(&mut self, duration: f32) {
        self.cooldown_duration = duration.max(0.0);
    }

    /// Update the number of nearby hostile entities.
    ///
    /// Call this each frame or when entities move.
    pub fn set_nearby_hostiles(&mut self, count: u32) {
        self.nearby_hostiles = count;
    }

    /// Report a combat event (player attacked or attacking).
    ///
    /// Immediately triggers combat music if enabled.
    pub fn combat_event(&mut self) {
        if !self.enabled {
            return;
        }
        self.time_since_combat = 0.0;
        self.state = CombatMusicState::InCombat;
    }

    /// Report player damage (triggers combat music).
    pub fn player_damaged(&mut self) {
        self.combat_event();
    }

    /// Update each frame.
    ///
    /// Returns the desired music state based on combat activity.
    pub fn tick(&mut self, dt: f32) -> CombatMusicState {
        if !self.enabled {
            return CombatMusicState::Peaceful;
        }

        match self.state {
            CombatMusicState::Peaceful => {
                // Check if hostiles are nearby
                if self.nearby_hostiles > 0 {
                    self.state = CombatMusicState::InCombat;
                    self.time_since_combat = 0.0;
                }
            }
            CombatMusicState::InCombat => {
                self.time_since_combat += dt;

                // If no hostiles and cooldown started
                if self.nearby_hostiles == 0 && self.time_since_combat > 0.0 {
                    self.state = CombatMusicState::Cooldown;
                }

                // Reset timer if still in combat
                if self.nearby_hostiles > 0 {
                    self.time_since_combat = 0.0;
                }
            }
            CombatMusicState::Cooldown => {
                self.time_since_combat += dt;

                // Re-enter combat if hostiles appear again
                if self.nearby_hostiles > 0 {
                    self.state = CombatMusicState::InCombat;
                    self.time_since_combat = 0.0;
                } else if self.time_since_combat >= self.cooldown_duration {
                    // Cooldown elapsed, back to peaceful
                    self.state = CombatMusicState::Peaceful;
                    self.time_since_combat = 0.0;
                }
            }
        }

        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_controller_is_peaceful() {
        let ctrl = CombatMusicController::new();
        assert_eq!(ctrl.state(), CombatMusicState::Peaceful);
        assert!(ctrl.enabled);
    }

    #[test]
    fn test_combat_event_triggers() {
        let mut ctrl = CombatMusicController::new();
        ctrl.combat_event();
        assert_eq!(ctrl.state(), CombatMusicState::InCombat);
    }

    #[test]
    fn test_player_damage_triggers() {
        let mut ctrl = CombatMusicController::new();
        ctrl.player_damaged();
        assert_eq!(ctrl.state(), CombatMusicState::InCombat);
    }

    #[test]
    fn test_nearby_hostiles_triggers() {
        let mut ctrl = CombatMusicController::new();
        ctrl.set_nearby_hostiles(3);
        ctrl.tick(0.0);
        assert_eq!(ctrl.state(), CombatMusicState::InCombat);
    }

    #[test]
    fn test_cooldown_before_peaceful() {
        let mut ctrl = CombatMusicController::new();
        ctrl.combat_event();
        assert_eq!(ctrl.state(), CombatMusicState::InCombat);

        // No more hostiles, tick some time
        ctrl.set_nearby_hostiles(0);
        let state = ctrl.tick(1.0);
        assert_eq!(state, CombatMusicState::Cooldown);

        // Not enough time yet
        let state = ctrl.tick(3.0);
        assert_eq!(state, CombatMusicState::Cooldown);

        // Cooldown elapsed
        let state = ctrl.tick(1.5);
        assert_eq!(state, CombatMusicState::Peaceful);
    }

    #[test]
    fn test_combat_restarts_cooldown() {
        let mut ctrl = CombatMusicController::new();
        ctrl.combat_event();
        ctrl.set_nearby_hostiles(0);
        ctrl.tick(2.0); // In cooldown

        // New combat event
        ctrl.combat_event();
        assert_eq!(ctrl.state(), CombatMusicState::InCombat);
    }

    #[test]
    fn test_hostiles_reappear_during_cooldown() {
        let mut ctrl = CombatMusicController::new();
        ctrl.combat_event();
        ctrl.set_nearby_hostiles(0);
        ctrl.tick(2.0); // Cooldown

        // Hostiles come back
        ctrl.set_nearby_hostiles(1);
        let state = ctrl.tick(0.0);
        assert_eq!(state, CombatMusicState::InCombat);
    }

    #[test]
    fn test_disable_combat_music() {
        let mut ctrl = CombatMusicController::new();
        ctrl.combat_event();
        ctrl.set_enabled(false);
        assert_eq!(ctrl.state(), CombatMusicState::Peaceful);
    }

    #[test]
    fn test_disabled_ignores_events() {
        let mut ctrl = CombatMusicController::new();
        ctrl.set_enabled(false);
        ctrl.combat_event();
        assert_eq!(ctrl.state(), CombatMusicState::Peaceful);
    }

    #[test]
    fn test_custom_cooldown() {
        let mut ctrl = CombatMusicController::new();
        ctrl.set_cooldown(10.0);
        ctrl.combat_event();
        ctrl.set_nearby_hostiles(0);
        ctrl.tick(1.0); // Cooldown

        let state = ctrl.tick(8.0); // 9 total, less than 10
        assert_eq!(state, CombatMusicState::Cooldown);

        let state = ctrl.tick(2.0); // 11 total, past cooldown
        assert_eq!(state, CombatMusicState::Peaceful);
    }

    #[test]
    fn test_is_in_combat() {
        let mut ctrl = CombatMusicController::new();
        assert!(!ctrl.is_in_combat());
        ctrl.combat_event();
        assert!(ctrl.is_in_combat());
    }
}

//! Hunger system for players.

use serde::{Deserialize, Serialize};

/// Default maximum hunger (20 = 10 drumsticks).
pub const DEFAULT_MAX_HUNGER: f32 = 20.0;

/// Hunger threshold below which sprinting is disabled.
pub const SPRINT_THRESHOLD: f32 = 6.0;

/// Hunger threshold below which damage is taken.
pub const DAMAGE_THRESHOLD: f32 = 0.0;

/// Base hunger drain rate (per second).
pub const BASE_DRAIN_RATE: f32 = 1.0 / 60.0; // 1 point per minute

/// Sprint multiplier for hunger drain.
pub const SPRINT_MULTIPLIER: f32 = 2.0;

/// Hunger component for players.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hunger {
    /// Current hunger level.
    current: f32,
    /// Maximum hunger level.
    max: f32,
    /// Hidden saturation buffer (drains before hunger).
    saturation: f32,
    /// Time since last damage from starvation.
    starvation_timer: f32,
}

impl Hunger {
    /// Create hunger with specified maximum.
    #[must_use]
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            saturation: 5.0, // Start with some saturation
            starvation_timer: 0.0,
        }
    }

    /// Create hunger with default maximum (20).
    #[must_use]
    pub fn default_player() -> Self {
        Self::new(DEFAULT_MAX_HUNGER)
    }

    /// Get current hunger.
    #[must_use]
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Get maximum hunger.
    #[must_use]
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Get current saturation.
    #[must_use]
    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    /// Get hunger as fraction (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            return 0.0;
        }
        (self.current / self.max).clamp(0.0, 1.0)
    }

    /// Get hunger as half-drumsticks (0 to max*2).
    #[must_use]
    pub fn half_drumsticks(&self) -> u32 {
        (self.current * 2.0).round().max(0.0) as u32
    }

    /// Check if hunger is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// Check if player can sprint.
    #[must_use]
    pub fn can_sprint(&self) -> bool {
        self.current >= SPRINT_THRESHOLD
    }

    /// Check if player should take starvation damage.
    #[must_use]
    pub fn should_damage(&self) -> bool {
        self.current <= DAMAGE_THRESHOLD
    }

    /// Check if hunger is low (for UI warning).
    #[must_use]
    pub fn is_low(&self) -> bool {
        self.current <= 3.0
    }

    /// Update hunger over time.
    ///
    /// Returns true if starvation damage should be applied.
    pub fn tick(&mut self, dt: f32, sprinting: bool) -> bool {
        let drain = BASE_DRAIN_RATE * dt * if sprinting { SPRINT_MULTIPLIER } else { 1.0 };

        // Drain saturation first
        if self.saturation > 0.0 {
            self.saturation = (self.saturation - drain).max(0.0);
        } else {
            // Then drain hunger
            self.current = (self.current - drain).max(0.0);
        }

        // Track starvation damage
        if self.should_damage() {
            self.starvation_timer += dt;
            if self.starvation_timer >= 1.0 {
                self.starvation_timer = 0.0;
                return true; // Should apply 1 damage
            }
        } else {
            self.starvation_timer = 0.0;
        }

        false
    }

    /// Eat food to restore hunger and saturation.
    ///
    /// Returns actual amount of hunger restored.
    pub fn eat(&mut self, food: f32, saturation: f32) -> f32 {
        if food <= 0.0 {
            return 0.0;
        }

        let old = self.current;
        self.current = (self.current + food).min(self.max);

        // Saturation is capped at current hunger
        self.saturation = (self.saturation + saturation).min(self.current);

        self.current - old
    }

    /// Fully restore hunger and saturation.
    pub fn restore(&mut self) {
        self.current = self.max;
        self.saturation = self.max;
        self.starvation_timer = 0.0;
    }

    /// Reset for respawn.
    pub fn respawn(&mut self) {
        self.restore();
    }
}

impl Default for Hunger {
    fn default() -> Self {
        Self::default_player()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hunger() {
        let hunger = Hunger::new(20.0);
        assert_eq!(hunger.current(), 20.0);
        assert_eq!(hunger.max(), 20.0);
        assert!(hunger.is_full());
        assert!(hunger.can_sprint());
        assert!(!hunger.should_damage());
    }

    #[test]
    fn test_fraction() {
        let mut hunger = Hunger::new(20.0);
        assert!((hunger.fraction() - 1.0).abs() < f32::EPSILON);

        // Drain some hunger (exhaust saturation first)
        hunger.saturation = 0.0;
        hunger.current = 10.0;
        assert!((hunger.fraction() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_half_drumsticks() {
        let hunger = Hunger::new(20.0);
        assert_eq!(hunger.half_drumsticks(), 40);

        let mut hunger = Hunger::new(10.0);
        hunger.current = 7.5;
        assert_eq!(hunger.half_drumsticks(), 15);
    }

    #[test]
    fn test_eat() {
        let mut hunger = Hunger::new(20.0);
        hunger.current = 10.0;
        hunger.saturation = 0.0;

        let restored = hunger.eat(5.0, 3.0);
        assert_eq!(restored, 5.0);
        assert_eq!(hunger.current(), 15.0);
        assert_eq!(hunger.saturation(), 3.0);
    }

    #[test]
    fn test_eat_caps_at_max() {
        let mut hunger = Hunger::new(20.0);
        hunger.current = 18.0;
        hunger.saturation = 0.0;

        let restored = hunger.eat(10.0, 10.0);
        assert_eq!(restored, 2.0); // Only 2 was needed
        assert!(hunger.is_full());
        // Saturation capped at current hunger (which is now 20)
        assert_eq!(hunger.saturation(), 10.0); // Added 10, capped at current=20
    }

    #[test]
    fn test_tick_drains_saturation_first() {
        let mut hunger = Hunger::new(20.0);
        hunger.saturation = 5.0;
        let initial_hunger = hunger.current();

        // Tick for 60 seconds (should drain about 1 point)
        for _ in 0..60 {
            hunger.tick(1.0, false);
        }

        // Saturation should have drained first
        assert!(hunger.saturation < 5.0);
        // Hunger might be slightly drained if saturation exhausted
    }

    #[test]
    fn test_tick_sprinting_drains_faster() {
        let mut hunger1 = Hunger::new(20.0);
        hunger1.saturation = 0.0;
        hunger1.current = 20.0;

        let mut hunger2 = Hunger::new(20.0);
        hunger2.saturation = 0.0;
        hunger2.current = 20.0;

        // Tick both for 60 seconds
        for _ in 0..60 {
            hunger1.tick(1.0, false);
            hunger2.tick(1.0, true);
        }

        // Sprinting should drain twice as fast
        assert!(hunger2.current() < hunger1.current());
    }

    #[test]
    fn test_can_sprint() {
        let mut hunger = Hunger::new(20.0);
        assert!(hunger.can_sprint());

        hunger.current = SPRINT_THRESHOLD;
        assert!(hunger.can_sprint());

        hunger.current = SPRINT_THRESHOLD - 0.1;
        assert!(!hunger.can_sprint());
    }

    #[test]
    fn test_starvation() {
        let mut hunger = Hunger::new(20.0);
        hunger.current = 0.0;
        hunger.saturation = 0.0;

        assert!(hunger.should_damage());

        // Starvation damage every second
        let damage1 = hunger.tick(0.5, false);
        assert!(!damage1);

        let damage2 = hunger.tick(0.6, false);
        assert!(damage2); // Should trigger damage
    }

    #[test]
    fn test_restore() {
        let mut hunger = Hunger::new(20.0);
        hunger.current = 5.0;
        hunger.saturation = 0.0;
        hunger.starvation_timer = 0.5;

        hunger.restore();
        assert!(hunger.is_full());
        assert_eq!(hunger.saturation(), 20.0);
    }

    #[test]
    fn test_is_low() {
        let mut hunger = Hunger::new(20.0);
        assert!(!hunger.is_low());

        hunger.current = 3.0;
        assert!(hunger.is_low());

        hunger.current = 2.0;
        assert!(hunger.is_low());
    }
}

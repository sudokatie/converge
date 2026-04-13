//! Health system for players and creatures.

use serde::{Deserialize, Serialize};

/// Default maximum health (20 = 10 hearts).
pub const DEFAULT_MAX_HEALTH: f32 = 20.0;

/// Damage sources for tracking and effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageSource {
    /// Fall damage from height.
    Fall,
    /// Attack from entity.
    Attack,
    /// Drowning underwater.
    Drowning,
    /// Fire/lava damage.
    Fire,
    /// Starvation (hunger depleted).
    Starvation,
    /// Environmental hazard.
    Environment,
    /// Void damage (falling out of world).
    Void,
}

/// Health component for entities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Health {
    /// Current health points.
    current: f32,
    /// Maximum health points.
    max: f32,
    /// Invincibility timer (seconds remaining).
    invincibility: f32,
    /// Last damage source (for death message).
    last_damage_source: Option<DamageSource>,
}

impl Health {
    /// Create health with specified maximum.
    #[must_use]
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            invincibility: 0.0,
            last_damage_source: None,
        }
    }

    /// Create health with default maximum (20).
    #[must_use]
    pub fn default_player() -> Self {
        Self::new(DEFAULT_MAX_HEALTH)
    }

    /// Get current health.
    #[must_use]
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Get maximum health.
    #[must_use]
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Set maximum health.
    ///
    /// If current health exceeds new max, it's clamped.
    pub fn set_max(&mut self, max: f32) {
        self.max = max.max(1.0);
        if self.current > self.max {
            self.current = self.max;
        }
    }

    /// Get health as fraction (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            return 0.0;
        }
        (self.current / self.max).clamp(0.0, 1.0)
    }

    /// Get health as half-hearts (0 to max*2).
    #[must_use]
    pub fn half_hearts(&self) -> u32 {
        (self.current * 2.0).round().max(0.0) as u32
    }

    /// Check if dead.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    /// Check if alive.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    /// Check if full health.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// Check if invincible.
    #[must_use]
    pub fn is_invincible(&self) -> bool {
        self.invincibility > 0.0
    }

    /// Get remaining invincibility time.
    #[must_use]
    pub fn invincibility_remaining(&self) -> f32 {
        self.invincibility
    }

    /// Get last damage source.
    #[must_use]
    pub fn last_damage_source(&self) -> Option<DamageSource> {
        self.last_damage_source
    }

    /// Apply damage.
    ///
    /// Returns true if entity died from this damage.
    /// Damage is ignored if invincible (returns false).
    pub fn damage(&mut self, amount: f32, source: DamageSource) -> bool {
        if amount <= 0.0 {
            return false;
        }

        if self.is_invincible() {
            return false;
        }

        if self.is_dead() {
            return false;
        }

        self.current = (self.current - amount).max(0.0);
        self.last_damage_source = Some(source);

        // Set invincibility after taking damage
        self.invincibility = 0.5; // Half second

        self.current <= 0.0
    }

    /// Apply damage ignoring invincibility (for void, starvation, etc.).
    ///
    /// Returns true if entity died.
    pub fn damage_absolute(&mut self, amount: f32, source: DamageSource) -> bool {
        if amount <= 0.0 {
            return false;
        }

        if self.is_dead() {
            return false;
        }

        self.current = (self.current - amount).max(0.0);
        self.last_damage_source = Some(source);

        self.current <= 0.0
    }

    /// Heal by amount.
    ///
    /// Returns actual amount healed.
    pub fn heal(&mut self, amount: f32) -> f32 {
        if amount <= 0.0 || self.is_dead() {
            return 0.0;
        }

        let old = self.current;
        self.current = (self.current + amount).min(self.max);
        self.current - old
    }

    /// Fully restore health.
    pub fn restore(&mut self) {
        self.current = self.max;
        self.invincibility = 0.0;
        self.last_damage_source = None;
    }

    /// Reset for respawn.
    pub fn respawn(&mut self) {
        self.restore();
    }

    /// Update invincibility timer.
    pub fn tick(&mut self, dt: f32) {
        if self.invincibility > 0.0 {
            self.invincibility = (self.invincibility - dt).max(0.0);
        }
    }

    /// Set invincibility duration.
    pub fn set_invincibility(&mut self, duration: f32) {
        self.invincibility = duration.max(0.0);
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::default_player()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_health() {
        let health = Health::new(20.0);
        assert_eq!(health.current(), 20.0);
        assert_eq!(health.max(), 20.0);
        assert!(health.is_full());
        assert!(health.is_alive());
        assert!(!health.is_dead());
    }

    #[test]
    fn test_fraction() {
        let mut health = Health::new(20.0);
        assert!((health.fraction() - 1.0).abs() < f32::EPSILON);

        health.damage(10.0, DamageSource::Attack);
        assert!((health.fraction() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_half_hearts() {
        let health = Health::new(20.0);
        assert_eq!(health.half_hearts(), 40);

        let mut health = Health::new(10.0);
        assert_eq!(health.half_hearts(), 20);

        health.damage(2.5, DamageSource::Attack);
        assert_eq!(health.half_hearts(), 15);
    }

    #[test]
    fn test_damage() {
        let mut health = Health::new(20.0);

        let died = health.damage(5.0, DamageSource::Attack);
        assert!(!died);
        assert_eq!(health.current(), 15.0);
        assert_eq!(health.last_damage_source(), Some(DamageSource::Attack));
    }

    #[test]
    fn test_damage_kills() {
        let mut health = Health::new(20.0);

        let died = health.damage(25.0, DamageSource::Fall);
        assert!(died);
        assert!(health.is_dead());
        assert_eq!(health.current(), 0.0);
    }

    #[test]
    fn test_invincibility() {
        let mut health = Health::new(20.0);

        // First damage applies and sets invincibility
        health.damage(5.0, DamageSource::Attack);
        assert!(health.is_invincible());
        assert_eq!(health.current(), 15.0);

        // Second damage blocked by invincibility
        health.damage(5.0, DamageSource::Attack);
        assert_eq!(health.current(), 15.0); // Unchanged

        // Tick down invincibility
        health.tick(0.6);
        assert!(!health.is_invincible());

        // Now damage applies
        health.damage(5.0, DamageSource::Attack);
        assert_eq!(health.current(), 10.0);
    }

    #[test]
    fn test_damage_absolute() {
        let mut health = Health::new(20.0);

        // Take damage to trigger invincibility
        health.damage(5.0, DamageSource::Attack);
        assert!(health.is_invincible());

        // Absolute damage ignores invincibility
        health.damage_absolute(5.0, DamageSource::Void);
        assert_eq!(health.current(), 10.0);
    }

    #[test]
    fn test_heal() {
        let mut health = Health::new(20.0);
        health.damage_absolute(15.0, DamageSource::Attack);
        assert_eq!(health.current(), 5.0);

        let healed = health.heal(10.0);
        assert_eq!(healed, 10.0);
        assert_eq!(health.current(), 15.0);

        // Heal caps at max
        let healed = health.heal(10.0);
        assert_eq!(healed, 5.0);
        assert!(health.is_full());
    }

    #[test]
    fn test_heal_when_dead() {
        let mut health = Health::new(20.0);
        health.damage_absolute(25.0, DamageSource::Void);
        assert!(health.is_dead());

        // Can't heal when dead
        let healed = health.heal(10.0);
        assert_eq!(healed, 0.0);
        assert!(health.is_dead());
    }

    #[test]
    fn test_restore() {
        let mut health = Health::new(20.0);
        health.damage_absolute(15.0, DamageSource::Attack);
        health.set_invincibility(1.0);

        health.restore();
        assert!(health.is_full());
        assert!(!health.is_invincible());
        assert!(health.last_damage_source().is_none());
    }

    #[test]
    fn test_set_max() {
        let mut health = Health::new(20.0);
        assert_eq!(health.max(), 20.0);

        // Increase max
        health.set_max(40.0);
        assert_eq!(health.max(), 40.0);
        assert_eq!(health.current(), 20.0); // Current unchanged

        // Decrease max below current
        health.set_max(10.0);
        assert_eq!(health.max(), 10.0);
        assert_eq!(health.current(), 10.0); // Current clamped
    }

    #[test]
    fn test_negative_amounts_ignored() {
        let mut health = Health::new(20.0);

        // Negative damage does nothing
        let died = health.damage(-5.0, DamageSource::Attack);
        assert!(!died);
        assert_eq!(health.current(), 20.0);

        // Negative heal does nothing
        health.damage_absolute(10.0, DamageSource::Attack);
        let healed = health.heal(-5.0);
        assert_eq!(healed, 0.0);
        assert_eq!(health.current(), 10.0);
    }
}

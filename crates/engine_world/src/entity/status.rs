//! Status effects system for entities
//! 
//! Implements spec 6.5.3 - status effects with duration, level, and tick behavior.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Types of status effects that can be applied to entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusEffectType {
    /// Deals damage over time
    Poison,
    /// Restores health over time
    Regeneration,
    /// Increases movement speed
    Speed,
    /// Decreases movement speed
    Slowness,
    /// Increases mining/attack speed
    Haste,
    /// Decreases mining/attack speed
    MiningFatigue,
    /// Increases melee damage
    Strength,
    /// Decreases melee damage
    Weakness,
}

impl StatusEffectType {
    /// Whether this effect is beneficial to the entity
    pub fn is_beneficial(&self) -> bool {
        matches!(
            self,
            StatusEffectType::Regeneration
                | StatusEffectType::Speed
                | StatusEffectType::Haste
                | StatusEffectType::Strength
        )
    }

    /// Whether this effect is harmful to the entity
    pub fn is_harmful(&self) -> bool {
        matches!(
            self,
            StatusEffectType::Poison
                | StatusEffectType::Slowness
                | StatusEffectType::MiningFatigue
                | StatusEffectType::Weakness
        )
    }

    /// Get the tick interval for this effect (how often tick_effect runs)
    pub fn tick_interval(&self) -> Duration {
        match self {
            StatusEffectType::Poison => Duration::from_millis(1250),
            StatusEffectType::Regeneration => Duration::from_millis(2500),
            _ => Duration::from_millis(1000),
        }
    }
}

/// A status effect instance applied to an entity
#[derive(Debug, Clone)]
pub struct StatusEffect {
    /// The type of effect
    pub effect_type: StatusEffectType,
    /// Effect level/amplifier (0 = level 1, 1 = level 2, etc.)
    pub level: u8,
    /// Total duration of the effect
    pub duration: Duration,
    /// When the effect was applied
    pub applied_at: Instant,
    /// When the effect last ticked
    pub last_tick: Instant,
    /// Whether to show particles
    pub show_particles: bool,
    /// Whether this effect came from a beacon
    pub ambient: bool,
}

impl StatusEffect {
    /// Create a new status effect
    pub fn new(effect_type: StatusEffectType, level: u8, duration: Duration) -> Self {
        let now = Instant::now();
        Self {
            effect_type,
            level,
            duration,
            applied_at: now,
            last_tick: now,
            show_particles: true,
            ambient: false,
        }
    }

    /// Create an ambient (beacon) effect
    pub fn ambient(effect_type: StatusEffectType, level: u8, duration: Duration) -> Self {
        let mut effect = Self::new(effect_type, level, duration);
        effect.ambient = true;
        effect.show_particles = false;
        effect
    }

    /// Check if the effect has expired
    pub fn is_expired(&self) -> bool {
        self.applied_at.elapsed() >= self.duration
    }

    /// Get remaining duration
    pub fn remaining_duration(&self) -> Duration {
        self.duration.saturating_sub(self.applied_at.elapsed())
    }

    /// Check if the effect should tick now
    pub fn should_tick(&self) -> bool {
        self.last_tick.elapsed() >= self.effect_type.tick_interval()
    }

    /// Apply the tick effect and return the result
    pub fn tick_effect(&mut self) -> TickResult {
        if !self.should_tick() {
            return TickResult::NoOp;
        }
        
        self.last_tick = Instant::now();
        let amplifier = self.level as f32 + 1.0;
        
        match self.effect_type {
            StatusEffectType::Poison => {
                // Poison deals 1 damage per level every 1.25 seconds
                // Cannot kill (leaves at 1 HP)
                TickResult::Damage {
                    amount: amplifier,
                    can_kill: false,
                }
            }
            StatusEffectType::Regeneration => {
                // Regeneration heals 1 HP per level every 2.5 seconds
                TickResult::Heal {
                    amount: amplifier,
                }
            }
            StatusEffectType::Speed => {
                // Speed increases movement by 20% per level
                TickResult::SpeedModifier {
                    multiplier: 1.0 + (0.2 * amplifier),
                }
            }
            StatusEffectType::Slowness => {
                // Slowness decreases movement by 15% per level
                TickResult::SpeedModifier {
                    multiplier: 1.0 - (0.15 * amplifier).min(0.85),
                }
            }
            StatusEffectType::Haste => {
                // Haste increases mining speed by 20% per level
                TickResult::MiningSpeedModifier {
                    multiplier: 1.0 + (0.2 * amplifier),
                }
            }
            StatusEffectType::MiningFatigue => {
                // Mining fatigue decreases mining speed significantly
                let reduction = match self.level {
                    0 => 0.3,
                    1 => 0.09,
                    2 => 0.0027,
                    _ => 0.00081,
                };
                TickResult::MiningSpeedModifier {
                    multiplier: reduction,
                }
            }
            StatusEffectType::Strength => {
                // Strength increases melee damage by 3 per level
                TickResult::DamageModifier {
                    flat_bonus: 3.0 * amplifier,
                }
            }
            StatusEffectType::Weakness => {
                // Weakness decreases melee damage by 4 per level
                TickResult::DamageModifier {
                    flat_bonus: -4.0 * amplifier,
                }
            }
        }
    }
}

/// Result of a status effect tick
#[derive(Debug, Clone, PartialEq)]
pub enum TickResult {
    /// No operation this tick
    NoOp,
    /// Deal damage to the entity
    Damage { amount: f32, can_kill: bool },
    /// Heal the entity
    Heal { amount: f32 },
    /// Modify movement speed
    SpeedModifier { multiplier: f32 },
    /// Modify mining speed
    MiningSpeedModifier { multiplier: f32 },
    /// Modify melee damage (additive)
    DamageModifier { flat_bonus: f32 },
}

/// Manager for all status effects on an entity
#[derive(Debug, Default)]
pub struct StatusEffectManager {
    effects: HashMap<StatusEffectType, StatusEffect>,
}

impl StatusEffectManager {
    /// Create a new empty manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a status effect, replacing any existing effect of the same type
    /// if the new one is stronger or has longer duration
    pub fn apply(&mut self, effect: StatusEffect) {
        let should_apply = match self.effects.get(&effect.effect_type) {
            None => true,
            Some(existing) => {
                effect.level > existing.level
                    || (effect.level == existing.level
                        && effect.remaining_duration() > existing.remaining_duration())
            }
        };

        if should_apply {
            self.effects.insert(effect.effect_type, effect);
        }
    }

    /// Remove a specific effect type
    pub fn remove(&mut self, effect_type: StatusEffectType) -> Option<StatusEffect> {
        self.effects.remove(&effect_type)
    }

    /// Clear all effects
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Clear all harmful effects
    pub fn clear_harmful(&mut self) {
        self.effects.retain(|_, e| !e.effect_type.is_harmful());
    }

    /// Check if entity has a specific effect
    pub fn has_effect(&self, effect_type: StatusEffectType) -> bool {
        self.effects.contains_key(&effect_type)
    }

    /// Get a specific effect
    pub fn get_effect(&self, effect_type: StatusEffectType) -> Option<&StatusEffect> {
        self.effects.get(&effect_type)
    }

    /// Update all effects and return tick results
    pub fn update(&mut self) -> Vec<TickResult> {
        // Remove expired effects
        self.effects.retain(|_, e| !e.is_expired());

        // Tick all remaining effects
        self.effects
            .values_mut()
            .map(|e| e.tick_effect())
            .filter(|r| *r != TickResult::NoOp)
            .collect()
    }

    /// Get all active effects
    pub fn active_effects(&self) -> impl Iterator<Item = &StatusEffect> {
        self.effects.values()
    }

    /// Calculate total speed modifier from all effects
    pub fn total_speed_modifier(&self) -> f32 {
        let mut modifier = 1.0;
        for effect in self.effects.values() {
            let amplifier = effect.level as f32 + 1.0;
            match effect.effect_type {
                StatusEffectType::Speed => modifier *= 1.0 + (0.2 * amplifier),
                StatusEffectType::Slowness => modifier *= 1.0 - (0.15 * amplifier).min(0.85),
                _ => {}
            }
        }
        modifier.max(0.0)
    }

    /// Calculate total damage modifier from all effects
    pub fn total_damage_modifier(&self) -> f32 {
        let mut modifier = 0.0;
        for effect in self.effects.values() {
            let amplifier = effect.level as f32 + 1.0;
            match effect.effect_type {
                StatusEffectType::Strength => modifier += 3.0 * amplifier,
                StatusEffectType::Weakness => modifier -= 4.0 * amplifier,
                _ => {}
            }
        }
        modifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_creation() {
        let effect = StatusEffect::new(
            StatusEffectType::Poison,
            0,
            Duration::from_secs(30),
        );
        assert_eq!(effect.effect_type, StatusEffectType::Poison);
        assert_eq!(effect.level, 0);
        assert!(!effect.is_expired());
    }

    #[test]
    fn test_effect_expiry() {
        let effect = StatusEffect::new(
            StatusEffectType::Speed,
            0,
            Duration::from_millis(1),
        );
        std::thread::sleep(Duration::from_millis(5));
        assert!(effect.is_expired());
    }

    #[test]
    fn test_manager_apply() {
        let mut manager = StatusEffectManager::new();
        manager.apply(StatusEffect::new(
            StatusEffectType::Strength,
            1,
            Duration::from_secs(60),
        ));
        assert!(manager.has_effect(StatusEffectType::Strength));
    }

    #[test]
    fn test_speed_modifier() {
        let mut manager = StatusEffectManager::new();
        manager.apply(StatusEffect::new(
            StatusEffectType::Speed,
            0,
            Duration::from_secs(60),
        ));
        let modifier = manager.total_speed_modifier();
        assert!((modifier - 1.2).abs() < 0.01);
    }
}

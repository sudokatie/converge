//! Combat system for melee attacks and damage.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::health::{DamageSource, Health};

/// Default attack cooldown in seconds.
pub const ATTACK_COOLDOWN: f32 = 0.5;

/// Default invincibility duration after being hit.
pub const DAMAGE_INVINCIBILITY: f32 = 0.5;

/// Default knockback velocity (blocks/second).
pub const KNOCKBACK_SPEED: f32 = 5.0;

/// Knockback upward component.
pub const KNOCKBACK_UPWARD: f32 = 3.0;

/// Combat stats for an entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatStats {
    /// Base attack damage.
    pub attack_damage: f32,
    /// Attack cooldown time in seconds.
    pub attack_cooldown: f32,
    /// Knockback resistance (0.0 = full knockback, 1.0 = immune).
    pub knockback_resistance: f32,
    /// Attack reach in blocks.
    pub attack_reach: f32,
}

impl Default for CombatStats {
    fn default() -> Self {
        Self {
            attack_damage: 1.0,
            attack_cooldown: ATTACK_COOLDOWN,
            knockback_resistance: 0.0,
            attack_reach: 3.0,
        }
    }
}

impl CombatStats {
    /// Create stats for a player (fist damage).
    #[must_use]
    pub fn player() -> Self {
        Self {
            attack_damage: 1.0,
            attack_cooldown: ATTACK_COOLDOWN,
            knockback_resistance: 0.0,
            attack_reach: 4.0,
        }
    }

    /// Create stats for a zombie.
    #[must_use]
    pub fn zombie() -> Self {
        Self {
            attack_damage: 3.0,
            attack_cooldown: 1.0,
            knockback_resistance: 0.0,
            attack_reach: 2.0,
        }
    }

    /// Create stats for a skeleton.
    #[must_use]
    pub fn skeleton() -> Self {
        Self {
            attack_damage: 2.0,
            attack_cooldown: 1.5,
            knockback_resistance: 0.0,
            attack_reach: 2.0,
        }
    }

    /// Create stats with custom damage.
    #[must_use]
    pub fn with_damage(mut self, damage: f32) -> Self {
        self.attack_damage = damage;
        self
    }

    /// Create stats with custom cooldown.
    #[must_use]
    pub fn with_cooldown(mut self, cooldown: f32) -> Self {
        self.attack_cooldown = cooldown;
        self
    }

    /// Create stats with custom reach.
    #[must_use]
    pub fn with_reach(mut self, reach: f32) -> Self {
        self.attack_reach = reach;
        self
    }

    /// Create stats with knockback resistance.
    #[must_use]
    pub fn with_knockback_resistance(mut self, resistance: f32) -> Self {
        self.knockback_resistance = resistance.clamp(0.0, 1.0);
        self
    }
}

/// Tracks attack cooldown for an entity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AttackCooldown {
    /// Time remaining on cooldown.
    remaining: f32,
    /// Maximum cooldown (for UI display).
    max: f32,
}

impl AttackCooldown {
    /// Create a new attack cooldown tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if attack is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Get cooldown progress (0.0 = just attacked, 1.0 = ready).
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.max <= 0.0 {
            return 1.0;
        }
        1.0 - (self.remaining / self.max).clamp(0.0, 1.0)
    }

    /// Start cooldown.
    pub fn start(&mut self, duration: f32) {
        self.remaining = duration;
        self.max = duration;
    }

    /// Update cooldown timer.
    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    /// Reset cooldown (ready immediately).
    pub fn reset(&mut self) {
        self.remaining = 0.0;
        self.max = 0.0;
    }
}

/// Result of an attack attempt.
#[derive(Clone, Debug)]
pub struct AttackResult {
    /// Whether the attack connected.
    pub hit: bool,
    /// Damage dealt (after modifiers).
    pub damage: f32,
    /// Whether the target died.
    pub killed: bool,
    /// Knockback velocity to apply to target.
    pub knockback: Vec3,
}

impl AttackResult {
    /// Create a miss result.
    #[must_use]
    pub fn miss() -> Self {
        Self {
            hit: false,
            damage: 0.0,
            killed: false,
            knockback: Vec3::ZERO,
        }
    }

    /// Create a hit result.
    #[must_use]
    pub fn hit(damage: f32, killed: bool, knockback: Vec3) -> Self {
        Self {
            hit: true,
            damage,
            killed,
            knockback,
        }
    }
}

/// Calculate knockback direction and magnitude.
#[must_use]
pub fn calculate_knockback(
    attacker_pos: Vec3,
    target_pos: Vec3,
    knockback_resistance: f32,
) -> Vec3 {
    // Direction away from attacker
    let horizontal = Vec3::new(
        target_pos.x - attacker_pos.x,
        0.0,
        target_pos.z - attacker_pos.z,
    )
    .normalize_or_zero();

    let knockback = horizontal * KNOCKBACK_SPEED + Vec3::new(0.0, KNOCKBACK_UPWARD, 0.0);

    // Apply resistance
    knockback * (1.0 - knockback_resistance)
}

/// Attempt a melee attack.
///
/// Returns the attack result. The caller is responsible for applying
/// damage to health and knockback to velocity.
#[must_use]
pub fn attempt_attack(
    attacker_pos: Vec3,
    attacker_stats: &CombatStats,
    attacker_cooldown: &mut AttackCooldown,
    target_pos: Vec3,
    target_health: &mut Health,
    target_stats: &CombatStats,
) -> AttackResult {
    // Check cooldown
    if !attacker_cooldown.is_ready() {
        return AttackResult::miss();
    }

    // Check range
    let distance = attacker_pos.distance(target_pos);
    if distance > attacker_stats.attack_reach {
        return AttackResult::miss();
    }

    // Start cooldown
    attacker_cooldown.start(attacker_stats.attack_cooldown);

    // Apply damage
    let killed = target_health.damage(attacker_stats.attack_damage, DamageSource::Attack);

    // Calculate knockback
    let knockback = if !target_health.is_dead() {
        calculate_knockback(attacker_pos, target_pos, target_stats.knockback_resistance)
    } else {
        Vec3::ZERO
    };

    AttackResult::hit(attacker_stats.attack_damage, killed, knockback)
}

/// Check if an attack can be made (range and cooldown check only).
#[must_use]
pub fn can_attack(
    attacker_pos: Vec3,
    attacker_stats: &CombatStats,
    cooldown: &AttackCooldown,
    target_pos: Vec3,
) -> bool {
    if !cooldown.is_ready() {
        return false;
    }

    let distance = attacker_pos.distance(target_pos);
    distance <= attacker_stats.attack_reach
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_stats_default() {
        let stats = CombatStats::default();
        assert_eq!(stats.attack_damage, 1.0);
        assert_eq!(stats.attack_cooldown, ATTACK_COOLDOWN);
    }

    #[test]
    fn test_combat_stats_presets() {
        let zombie = CombatStats::zombie();
        assert_eq!(zombie.attack_damage, 3.0);

        let skeleton = CombatStats::skeleton();
        assert_eq!(skeleton.attack_damage, 2.0);
    }

    #[test]
    fn test_combat_stats_builder() {
        let stats = CombatStats::default()
            .with_damage(5.0)
            .with_cooldown(1.0)
            .with_reach(4.0)
            .with_knockback_resistance(0.5);

        assert_eq!(stats.attack_damage, 5.0);
        assert_eq!(stats.attack_cooldown, 1.0);
        assert_eq!(stats.attack_reach, 4.0);
        assert_eq!(stats.knockback_resistance, 0.5);
    }

    #[test]
    fn test_attack_cooldown() {
        let mut cooldown = AttackCooldown::new();
        assert!(cooldown.is_ready());
        assert_eq!(cooldown.progress(), 1.0);

        cooldown.start(1.0);
        assert!(!cooldown.is_ready());
        assert_eq!(cooldown.progress(), 0.0);

        cooldown.tick(0.5);
        assert!(!cooldown.is_ready());
        assert!((cooldown.progress() - 0.5).abs() < 0.01);

        cooldown.tick(0.6);
        assert!(cooldown.is_ready());
        assert_eq!(cooldown.progress(), 1.0);
    }

    #[test]
    fn test_knockback_calculation() {
        let attacker = Vec3::ZERO;
        let target = Vec3::new(5.0, 0.0, 0.0);

        let knockback = calculate_knockback(attacker, target, 0.0);

        // Should push away (positive X)
        assert!(knockback.x > 0.0);
        // Should have upward component
        assert!(knockback.y > 0.0);
    }

    #[test]
    fn test_knockback_resistance() {
        let attacker = Vec3::ZERO;
        let target = Vec3::new(5.0, 0.0, 0.0);

        let full_knockback = calculate_knockback(attacker, target, 0.0);
        let half_knockback = calculate_knockback(attacker, target, 0.5);
        let no_knockback = calculate_knockback(attacker, target, 1.0);

        assert!(half_knockback.length() < full_knockback.length());
        assert_eq!(no_knockback, Vec3::ZERO);
    }

    #[test]
    fn test_attempt_attack_success() {
        let attacker_pos = Vec3::ZERO;
        let attacker_stats = CombatStats::default().with_damage(5.0);
        let mut attacker_cooldown = AttackCooldown::new();

        let target_pos = Vec3::new(2.0, 0.0, 0.0);
        let mut target_health = Health::new(20.0);
        let target_stats = CombatStats::default();

        let result = attempt_attack(
            attacker_pos,
            &attacker_stats,
            &mut attacker_cooldown,
            target_pos,
            &mut target_health,
            &target_stats,
        );

        assert!(result.hit);
        assert_eq!(result.damage, 5.0);
        assert!(!result.killed);
        assert_eq!(target_health.current(), 15.0);
        assert!(!attacker_cooldown.is_ready());
    }

    #[test]
    fn test_attempt_attack_out_of_range() {
        let attacker_pos = Vec3::ZERO;
        let attacker_stats = CombatStats::default();
        let mut attacker_cooldown = AttackCooldown::new();

        let target_pos = Vec3::new(10.0, 0.0, 0.0); // Far away
        let mut target_health = Health::new(20.0);
        let target_stats = CombatStats::default();

        let result = attempt_attack(
            attacker_pos,
            &attacker_stats,
            &mut attacker_cooldown,
            target_pos,
            &mut target_health,
            &target_stats,
        );

        assert!(!result.hit);
        assert_eq!(target_health.current(), 20.0); // Unchanged
    }

    #[test]
    fn test_attempt_attack_on_cooldown() {
        let attacker_pos = Vec3::ZERO;
        let attacker_stats = CombatStats::default();
        let mut attacker_cooldown = AttackCooldown::new();
        attacker_cooldown.start(1.0); // On cooldown

        let target_pos = Vec3::new(2.0, 0.0, 0.0);
        let mut target_health = Health::new(20.0);
        let target_stats = CombatStats::default();

        let result = attempt_attack(
            attacker_pos,
            &attacker_stats,
            &mut attacker_cooldown,
            target_pos,
            &mut target_health,
            &target_stats,
        );

        assert!(!result.hit);
        assert_eq!(target_health.current(), 20.0); // Unchanged
    }

    #[test]
    fn test_attack_kills_target() {
        let attacker_pos = Vec3::ZERO;
        let attacker_stats = CombatStats::default().with_damage(25.0);
        let mut attacker_cooldown = AttackCooldown::new();

        let target_pos = Vec3::new(2.0, 0.0, 0.0);
        let mut target_health = Health::new(20.0);
        let target_stats = CombatStats::default();

        let result = attempt_attack(
            attacker_pos,
            &attacker_stats,
            &mut attacker_cooldown,
            target_pos,
            &mut target_health,
            &target_stats,
        );

        assert!(result.hit);
        assert!(result.killed);
        assert!(target_health.is_dead());
        // No knockback on kill
        assert_eq!(result.knockback, Vec3::ZERO);
    }

    #[test]
    fn test_can_attack() {
        let attacker_pos = Vec3::ZERO;
        let attacker_stats = CombatStats::default();
        let cooldown = AttackCooldown::new();

        // In range
        assert!(can_attack(
            attacker_pos,
            &attacker_stats,
            &cooldown,
            Vec3::new(2.0, 0.0, 0.0)
        ));

        // Out of range
        assert!(!can_attack(
            attacker_pos,
            &attacker_stats,
            &cooldown,
            Vec3::new(10.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn test_attack_result_constructors() {
        let miss = AttackResult::miss();
        assert!(!miss.hit);

        let hit = AttackResult::hit(5.0, false, Vec3::ONE);
        assert!(hit.hit);
        assert_eq!(hit.damage, 5.0);
        assert!(!hit.killed);
    }
}

//! Ranged attack system for hostile creatures.
//!
//! Implements spec 8.3.2: skeleton ranged attacks with projectile
//! spawning, cooldown, and aim prediction.

use glam::Vec3;

/// Default ranged attack range (blocks).
pub const RANGED_ATTACK_RANGE: f32 = 16.0;

/// Default ranged attack cooldown (seconds).
pub const RANGED_ATTACK_COOLDOWN: f32 = 2.0;

/// Projectile speed (blocks per second).
pub const PROJECTILE_SPEED: f32 = 20.0;

/// Projectile lifetime (seconds).
pub const PROJECTILE_LIFETIME: f32 = 5.0;

/// Damage dealt by a projectile.
pub const PROJECTILE_DAMAGE: f32 = 3.0;

/// Ranged attack state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangedState {
    /// Ready to fire.
    Ready,
    /// On cooldown after firing.
    Cooldown,
}

/// Manages ranged attack logic for a creature.
#[derive(Debug, Clone)]
pub struct RangedAttacker {
    /// Attack range.
    pub range: f32,
    /// Cooldown duration.
    pub cooldown: f32,
    /// Damage per hit.
    pub damage: f32,
    /// Time remaining on cooldown.
    cooldown_remaining: f32,
    /// Current state.
    state: RangedState,
}

impl Default for RangedAttacker {
    fn default() -> Self {
        Self::new()
    }
}

impl RangedAttacker {
    /// Create a new ranged attacker with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            range: RANGED_ATTACK_RANGE,
            cooldown: RANGED_ATTACK_COOLDOWN,
            damage: PROJECTILE_DAMAGE,
            cooldown_remaining: 0.0,
            state: RangedState::Ready,
        }
    }

    /// Create a ranged attacker with custom parameters.
    #[must_use]
    pub fn with_params(range: f32, cooldown: f32, damage: f32) -> Self {
        Self {
            range,
            cooldown,
            damage,
            cooldown_remaining: 0.0,
            state: RangedState::Ready,
        }
    }

    /// Get the current state.
    #[must_use]
    pub fn state(&self) -> RangedState {
        self.state
    }

    /// Check if ready to fire.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.state == RangedState::Ready
    }

    /// Check if target is within range.
    #[must_use]
    pub fn is_in_range(&self, attacker_pos: Vec3, target_pos: Vec3) -> bool {
        attacker_pos.distance(target_pos) <= self.range
    }

    /// Calculate aim direction toward a target.
    ///
    /// Includes basic prediction based on target velocity.
    #[must_use]
    pub fn aim_direction(
        attacker_pos: Vec3,
        target_pos: Vec3,
        target_velocity: Vec3,
    ) -> Vec3 {
        let to_target = target_pos - attacker_pos;
        let distance = to_target.length();

        if distance < 0.1 {
            return Vec3::ZERO;
        }

        // Predict where target will be when projectile arrives
        let travel_time = distance / PROJECTILE_SPEED;
        let predicted_pos = target_pos + target_velocity * travel_time;

        (predicted_pos - attacker_pos).normalize()
    }

    /// Attempt to fire a projectile.
    ///
    /// Returns the projectile spawn info if firing succeeds.
    pub fn fire(&mut self, attacker_pos: Vec3, target_pos: Vec3, target_velocity: Vec3) -> Option<Projectile> {
        if !self.is_ready() || !self.is_in_range(attacker_pos, target_pos) {
            return None;
        }

        let direction = Self::aim_direction(attacker_pos, target_pos, target_velocity);

        self.state = RangedState::Cooldown;
        self.cooldown_remaining = self.cooldown;

        Some(Projectile {
            position: attacker_pos,
            velocity: direction * PROJECTILE_SPEED,
            damage: self.damage,
            lifetime: PROJECTILE_LIFETIME,
            age: 0.0,
        })
    }

    /// Update cooldown.
    pub fn tick(&mut self, dt: f32) {
        if self.state == RangedState::Cooldown {
            self.cooldown_remaining -= dt;
            if self.cooldown_remaining <= 0.0 {
                self.cooldown_remaining = 0.0;
                self.state = RangedState::Ready;
            }
        }
    }

    /// Get cooldown progress (0.0 to 1.0).
    #[must_use]
    pub fn cooldown_progress(&self) -> f32 {
        if self.cooldown <= 0.0 {
            return 1.0;
        }
        1.0 - (self.cooldown_remaining / self.cooldown).clamp(0.0, 1.0)
    }
}

/// A projectile in flight.
#[derive(Debug, Clone)]
pub struct Projectile {
    /// Current position.
    pub position: Vec3,
    /// Velocity vector.
    pub velocity: Vec3,
    /// Damage on hit.
    pub damage: f32,
    /// Maximum lifetime.
    pub lifetime: f32,
    /// Current age.
    pub age: f32,
}

impl Projectile {
    /// Update projectile position.
    ///
    /// Returns true if the projectile has expired.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.position += self.velocity * dt;
        self.age += dt;
        self.age >= self.lifetime
    }

    /// Check if this projectile hits a target at the given position.
    #[must_use]
    pub fn hits(&self, target_pos: Vec3, hit_radius: f32) -> bool {
        self.position.distance(target_pos) <= hit_radius
    }

    /// Create a skeleton attacker.
    #[must_use]
    pub fn skeleton_attacker() -> RangedAttacker {
        RangedAttacker::with_params(RANGED_ATTACK_RANGE, RANGED_ATTACK_COOLDOWN, PROJECTILE_DAMAGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_attacker_is_ready() {
        let attacker = RangedAttacker::new();
        assert!(attacker.is_ready());
        assert_eq!(attacker.state(), RangedState::Ready);
    }

    #[test]
    fn test_in_range() {
        let attacker = RangedAttacker::new();
        assert!(attacker.is_in_range(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)));
        assert!(!attacker.is_in_range(Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0)));
    }

    #[test]
    fn test_fire_creates_projectile() {
        let mut attacker = RangedAttacker::new();
        let proj = attacker.fire(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

        assert!(proj.is_some());
        assert_eq!(attacker.state(), RangedState::Cooldown);
    }

    #[test]
    fn test_fire_out_of_range() {
        let mut attacker = RangedAttacker::new();
        let proj = attacker.fire(Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
        assert!(proj.is_none());
    }

    #[test]
    fn test_fire_on_cooldown() {
        let mut attacker = RangedAttacker::new();
        attacker.fire(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

        // Second fire should fail
        let proj = attacker.fire(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);
        assert!(proj.is_none());
    }

    #[test]
    fn test_cooldown_expires() {
        let mut attacker = RangedAttacker::new();
        attacker.fire(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

        attacker.tick(RANGED_ATTACK_COOLDOWN);
        assert!(attacker.is_ready());
    }

    #[test]
    fn test_cooldown_progress() {
        let mut attacker = RangedAttacker::new();
        attacker.fire(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

        assert!((attacker.cooldown_progress() - 0.0).abs() < 0.001);

        attacker.tick(1.0);
        assert!(attacker.cooldown_progress() > 0.0 && attacker.cooldown_progress() < 1.0);
    }

    #[test]
    fn test_aim_direction() {
        let dir = RangedAttacker::aim_direction(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::ZERO,
        );
        assert!((dir - Vec3::X).length() < 0.01);
    }

    #[test]
    fn test_aim_with_moving_target() {
        let dir = RangedAttacker::aim_direction(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 5.0), // Moving in Z
        );
        // Should lead the target
        assert!(dir.z > 0.0, "Should aim ahead of moving target");
    }

    #[test]
    fn test_projectile_tick() {
        let mut proj = Projectile {
            position: Vec3::ZERO,
            velocity: Vec3::X * PROJECTILE_SPEED,
            damage: PROJECTILE_DAMAGE,
            lifetime: PROJECTILE_LIFETIME,
            age: 0.0,
        };

        assert!(!proj.tick(1.0));
        assert!(proj.position.x > 0.0);
    }

    #[test]
    fn test_projectile_expiry() {
        let mut proj = Projectile {
            position: Vec3::ZERO,
            velocity: Vec3::X,
            damage: 1.0,
            lifetime: 1.0,
            age: 0.0,
        };

        assert!(proj.tick(1.5));
    }

    #[test]
    fn test_projectile_hit_detection() {
        let proj = Projectile {
            position: Vec3::new(5.0, 0.0, 0.0),
            velocity: Vec3::X,
            damage: 1.0,
            lifetime: 5.0,
            age: 0.0,
        };

        assert!(proj.hits(Vec3::new(5.5, 0.0, 0.0), 1.0));
        assert!(!proj.hits(Vec3::new(10.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn test_skeleton_attacker() {
        let attacker = Projectile::skeleton_attacker();
        assert!(attacker.is_ready());
        assert_eq!(attacker.damage, PROJECTILE_DAMAGE);
    }
}

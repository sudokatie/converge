//! Hostile creature AI - state machine for aggressive mobs.
//!
//! Hostile creatures (zombies, skeletons, spiders, creepers) patrol their
//! home area, detect players, chase them down, and attack.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Detection range in blocks.
pub const DETECTION_RANGE: f32 = 16.0;

/// Maximum chase distance from home before giving up.
pub const MAX_CHASE_DISTANCE: f32 = 32.0;

/// Attack range in blocks.
pub const ATTACK_RANGE: f32 = 2.0;

/// Attack cooldown in seconds.
pub const ATTACK_COOLDOWN: f32 = 1.0;

/// Patrol radius around home.
pub const PATROL_RADIUS: f32 = 8.0;

/// Time to stay alert after losing sight.
pub const ALERT_DURATION: f32 = 5.0;

/// State of a hostile creature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostileState {
    /// Patrolling around home area.
    Patrol,
    /// Heard/saw something, investigating.
    Alert,
    /// Actively chasing a target.
    Chase,
    /// In attack range, attacking.
    Attack,
    /// Returning home after losing target.
    Returning,
}

impl Default for HostileState {
    fn default() -> Self {
        Self::Patrol
    }
}

/// Result of an AI update tick.
#[derive(Clone, Debug, Default)]
pub struct HostileAction {
    /// Desired movement direction (normalized or zero).
    pub movement: Vec3,
    /// Whether to perform an attack this tick.
    pub should_attack: bool,
    /// Current facing direction (for rendering).
    pub look_direction: Vec3,
}

/// AI component for hostile creatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostileAI {
    /// Current behavior state.
    state: HostileState,
    /// Home position (spawn point).
    home: Vec3,
    /// Current patrol target.
    patrol_target: Option<Vec3>,
    /// Last known target position.
    last_target_pos: Option<Vec3>,
    /// Timer for current state.
    timer: f32,
    /// Attack cooldown timer.
    attack_cooldown: f32,
    /// Time since target was last seen.
    time_since_seen: f32,
}

impl HostileAI {
    /// Create a new hostile AI at the given home position.
    #[must_use]
    pub fn new(home: Vec3) -> Self {
        Self {
            state: HostileState::Patrol,
            home,
            patrol_target: None,
            last_target_pos: None,
            timer: 0.0,
            attack_cooldown: 0.0,
            time_since_seen: 0.0,
        }
    }

    /// Get the current state.
    #[must_use]
    pub fn state(&self) -> HostileState {
        self.state
    }

    /// Get the home position.
    #[must_use]
    pub fn home(&self) -> Vec3 {
        self.home
    }

    /// Set home position.
    pub fn set_home(&mut self, home: Vec3) {
        self.home = home;
    }

    /// Check if target is within detection range.
    #[must_use]
    pub fn can_detect(&self, self_pos: Vec3, target_pos: Vec3) -> bool {
        let distance = self_pos.distance(target_pos);
        distance <= DETECTION_RANGE
    }

    /// Check if target is within attack range.
    #[must_use]
    pub fn can_attack(&self, self_pos: Vec3, target_pos: Vec3) -> bool {
        let distance = self_pos.distance(target_pos);
        distance <= ATTACK_RANGE
    }

    /// Check if we're too far from home.
    #[must_use]
    pub fn too_far_from_home(&self, self_pos: Vec3) -> bool {
        let distance = self_pos.distance(self.home);
        distance > MAX_CHASE_DISTANCE
    }

    /// Check line of sight to target (simplified - just distance check).
    /// In a full implementation, this would raycast through blocks.
    #[must_use]
    pub fn has_line_of_sight(&self, self_pos: Vec3, target_pos: Vec3) -> bool {
        // Simplified: just check distance and height difference
        let distance = self_pos.distance(target_pos);
        let height_diff = (self_pos.y - target_pos.y).abs();

        distance <= DETECTION_RANGE && height_diff < 4.0
    }

    /// Update the AI state machine.
    ///
    /// # Arguments
    /// * `self_pos` - Current position of this creature
    /// * `target_pos` - Position of potential target (player), or None
    /// * `dt` - Delta time in seconds
    /// * `rng_value` - Random value 0-1 for patrol decisions
    ///
    /// Returns the action to take this tick.
    pub fn update(
        &mut self,
        self_pos: Vec3,
        target_pos: Option<Vec3>,
        dt: f32,
        rng_value: f32,
    ) -> HostileAction {
        // Update cooldowns
        self.timer = (self.timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);

        // Check for target detection
        let target_visible = target_pos
            .map(|tp| self.has_line_of_sight(self_pos, tp))
            .unwrap_or(false);

        if target_visible {
            self.time_since_seen = 0.0;
            self.last_target_pos = target_pos;
        } else {
            self.time_since_seen += dt;
        }

        match self.state {
            HostileState::Patrol => self.update_patrol(self_pos, target_pos, target_visible, rng_value),
            HostileState::Alert => self.update_alert(self_pos, target_pos, target_visible),
            HostileState::Chase => self.update_chase(self_pos, target_pos, target_visible),
            HostileState::Attack => self.update_attack(self_pos, target_pos, target_visible),
            HostileState::Returning => self.update_returning(self_pos, target_pos, target_visible, rng_value),
        }
    }

    fn update_patrol(
        &mut self,
        self_pos: Vec3,
        target_pos: Option<Vec3>,
        target_visible: bool,
        rng_value: f32,
    ) -> HostileAction {
        // Check for target detection
        if target_visible {
            self.state = HostileState::Alert;
            self.timer = ALERT_DURATION;
            return self.look_at(self_pos, target_pos.unwrap());
        }

        // Patrol behavior
        if self.patrol_target.is_none() || self.timer <= 0.0 {
            // Pick new patrol target
            self.patrol_target = Some(self.random_patrol_point(rng_value));
            self.timer = 10.0;
        }

        if let Some(target) = self.patrol_target {
            let to_target = target - self_pos;
            let dist = Vec3::new(to_target.x, 0.0, to_target.z).length();

            if dist < 1.0 {
                // Reached patrol point
                self.patrol_target = None;
                self.timer = 2.0 + rng_value * 3.0; // Wait before next patrol
                return HostileAction::default();
            }

            let dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();
            return HostileAction {
                movement: dir,
                should_attack: false,
                look_direction: dir,
            };
        }

        HostileAction::default()
    }

    fn update_alert(
        &mut self,
        self_pos: Vec3,
        target_pos: Option<Vec3>,
        target_visible: bool,
    ) -> HostileAction {
        if target_visible {
            if let Some(tp) = target_pos {
                if self.can_detect(self_pos, tp) {
                    // Target confirmed, start chase
                    self.state = HostileState::Chase;
                    return self.move_toward(self_pos, tp);
                }
            }
        }

        // Look around at last known position
        if let Some(last_pos) = self.last_target_pos {
            let action = self.look_at(self_pos, last_pos);

            // Move toward last known position
            let to_last = last_pos - self_pos;
            let dist = Vec3::new(to_last.x, 0.0, to_last.z).length();

            if dist > 2.0 {
                let dir = Vec3::new(to_last.x, 0.0, to_last.z).normalize_or_zero();
                return HostileAction {
                    movement: dir * 0.5, // Move slower while alert
                    ..action
                };
            }
        }

        // Alert timer expired - return to patrol
        if self.time_since_seen > ALERT_DURATION {
            self.state = HostileState::Patrol;
            self.last_target_pos = None;
        }

        HostileAction::default()
    }

    fn update_chase(
        &mut self,
        self_pos: Vec3,
        target_pos: Option<Vec3>,
        target_visible: bool,
    ) -> HostileAction {
        // Check if too far from home
        if self.too_far_from_home(self_pos) {
            self.state = HostileState::Returning;
            return self.move_toward(self_pos, self.home);
        }

        // Lost sight of target
        if !target_visible {
            if self.time_since_seen > ALERT_DURATION {
                self.state = HostileState::Returning;
                return self.move_toward(self_pos, self.home);
            }
            // Move to last known position
            if let Some(last_pos) = self.last_target_pos {
                return self.move_toward(self_pos, last_pos);
            }
        }

        if let Some(tp) = target_pos {
            // Check for attack range
            if self.can_attack(self_pos, tp) {
                self.state = HostileState::Attack;
                return self.update_attack(self_pos, target_pos, target_visible);
            }

            // Chase the target
            return self.move_toward(self_pos, tp);
        }

        HostileAction::default()
    }

    fn update_attack(
        &mut self,
        self_pos: Vec3,
        target_pos: Option<Vec3>,
        target_visible: bool,
    ) -> HostileAction {
        if let Some(tp) = target_pos {
            // Target moved out of range
            if !self.can_attack(self_pos, tp) {
                self.state = HostileState::Chase;
                return self.move_toward(self_pos, tp);
            }

            // Attack if cooldown ready
            let should_attack = self.attack_cooldown <= 0.0;
            if should_attack {
                self.attack_cooldown = ATTACK_COOLDOWN;
            }

            return HostileAction {
                movement: Vec3::ZERO,
                should_attack,
                look_direction: (tp - self_pos).normalize_or_zero(),
            };
        }

        // Lost target
        if !target_visible {
            self.state = HostileState::Alert;
            self.timer = ALERT_DURATION;
        }

        HostileAction::default()
    }

    fn update_returning(
        &mut self,
        self_pos: Vec3,
        target_pos: Option<Vec3>,
        target_visible: bool,
        rng_value: f32,
    ) -> HostileAction {
        // Check if we spot target while returning
        if target_visible {
            if let Some(tp) = target_pos {
                // Only re-engage if not too far from home
                if !self.too_far_from_home(self_pos) {
                    self.state = HostileState::Chase;
                    return self.move_toward(self_pos, tp);
                }
            }
        }

        // Move toward home
        let to_home = self.home - self_pos;
        let dist = Vec3::new(to_home.x, 0.0, to_home.z).length();

        if dist < 2.0 {
            // Reached home, resume patrol
            self.state = HostileState::Patrol;
            self.patrol_target = None;
            self.timer = rng_value * 3.0;
            return HostileAction::default();
        }

        self.move_toward(self_pos, self.home)
    }

    fn random_patrol_point(&self, rng_value: f32) -> Vec3 {
        let angle = rng_value * std::f32::consts::TAU;
        let distance = PATROL_RADIUS * 0.3 + rng_value * PATROL_RADIUS * 0.7;

        Vec3::new(
            self.home.x + angle.cos() * distance,
            self.home.y,
            self.home.z + angle.sin() * distance,
        )
    }

    fn move_toward(&self, self_pos: Vec3, target: Vec3) -> HostileAction {
        let to_target = target - self_pos;
        let dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();

        HostileAction {
            movement: dir,
            should_attack: false,
            look_direction: dir,
        }
    }

    fn look_at(&self, self_pos: Vec3, target: Vec3) -> HostileAction {
        let to_target = target - self_pos;
        let dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();

        HostileAction {
            movement: Vec3::ZERO,
            should_attack: false,
            look_direction: dir,
        }
    }

    /// Force return to home.
    pub fn force_return(&mut self) {
        self.state = HostileState::Returning;
        self.last_target_pos = None;
    }

    /// Reset to patrol state at home.
    pub fn reset(&mut self) {
        self.state = HostileState::Patrol;
        self.patrol_target = None;
        self.last_target_pos = None;
        self.timer = 0.0;
        self.attack_cooldown = 0.0;
        self.time_since_seen = 0.0;
    }
}

impl Default for HostileAI {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hostile_ai() {
        let home = Vec3::new(10.0, 64.0, 10.0);
        let ai = HostileAI::new(home);

        assert_eq!(ai.state(), HostileState::Patrol);
        assert_eq!(ai.home(), home);
    }

    #[test]
    fn test_detection_range() {
        let ai = HostileAI::new(Vec3::ZERO);
        let self_pos = Vec3::ZERO;

        // Within range
        assert!(ai.can_detect(self_pos, Vec3::new(10.0, 0.0, 0.0)));

        // Outside range
        assert!(!ai.can_detect(self_pos, Vec3::new(20.0, 0.0, 0.0)));
    }

    #[test]
    fn test_attack_range() {
        let ai = HostileAI::new(Vec3::ZERO);
        let self_pos = Vec3::ZERO;

        assert!(ai.can_attack(self_pos, Vec3::new(1.5, 0.0, 0.0)));
        assert!(!ai.can_attack(self_pos, Vec3::new(3.0, 0.0, 0.0)));
    }

    #[test]
    fn test_patrol_to_alert_on_detection() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        let self_pos = Vec3::ZERO;
        let target_pos = Some(Vec3::new(10.0, 0.0, 0.0));

        ai.update(self_pos, target_pos, 0.1, 0.5);

        assert_eq!(ai.state(), HostileState::Alert);
    }

    #[test]
    fn test_alert_to_chase() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Alert;
        ai.timer = ALERT_DURATION;

        let self_pos = Vec3::ZERO;
        let target_pos = Some(Vec3::new(10.0, 0.0, 0.0));

        ai.update(self_pos, target_pos, 0.1, 0.5);

        assert_eq!(ai.state(), HostileState::Chase);
    }

    #[test]
    fn test_chase_to_attack() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Chase;

        let self_pos = Vec3::ZERO;
        let target_pos = Some(Vec3::new(1.0, 0.0, 0.0)); // Within attack range

        ai.update(self_pos, target_pos, 0.1, 0.5);

        assert_eq!(ai.state(), HostileState::Attack);
    }

    #[test]
    fn test_attack_cooldown() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Attack;
        ai.attack_cooldown = 0.0;

        let self_pos = Vec3::ZERO;
        let target_pos = Some(Vec3::new(1.0, 0.0, 0.0));

        let action1 = ai.update(self_pos, target_pos, 0.1, 0.5);
        assert!(action1.should_attack);
        assert!(ai.attack_cooldown > 0.0);

        // Second attack should be on cooldown
        let action2 = ai.update(self_pos, target_pos, 0.1, 0.5);
        assert!(!action2.should_attack);
    }

    #[test]
    fn test_give_up_when_too_far() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Chase;

        let self_pos = Vec3::new(50.0, 0.0, 0.0); // Far from home
        let target_pos = Some(Vec3::new(60.0, 0.0, 0.0));

        ai.update(self_pos, target_pos, 0.1, 0.5);

        assert_eq!(ai.state(), HostileState::Returning);
    }

    #[test]
    fn test_returning_resumes_patrol() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Returning;

        let self_pos = Vec3::new(0.5, 0.0, 0.5); // Near home

        ai.update(self_pos, None, 0.1, 0.5);

        assert_eq!(ai.state(), HostileState::Patrol);
    }

    #[test]
    fn test_reset() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Chase;
        ai.attack_cooldown = 1.0;
        ai.last_target_pos = Some(Vec3::ONE);

        ai.reset();

        assert_eq!(ai.state(), HostileState::Patrol);
        assert!(ai.last_target_pos.is_none());
        assert_eq!(ai.attack_cooldown, 0.0);
    }

    #[test]
    fn test_movement_direction() {
        let mut ai = HostileAI::new(Vec3::ZERO);
        ai.state = HostileState::Chase;

        let self_pos = Vec3::ZERO;
        let target_pos = Some(Vec3::new(10.0, 0.0, 0.0));

        let action = ai.update(self_pos, target_pos, 0.1, 0.5);

        // Should move toward target (positive X)
        assert!(action.movement.x > 0.0);
        assert!(action.movement.length() > 0.9); // Normalized
    }
}

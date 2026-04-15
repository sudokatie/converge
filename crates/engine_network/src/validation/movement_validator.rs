//! Movement validation for anti-cheat
//! 
//! Validates player movement to detect speed hacks, teleportation, and fly hacks.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{ValidationConfig, ValidationResult};

/// Position in 3D space
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance_squared(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn distance(&self, other: &Position) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn horizontal_distance(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }
}

/// Player movement state for validation
#[derive(Debug)]
struct PlayerMovementState {
    last_position: Position,
    last_update: Instant,
    on_ground: bool,
    violations: u32,
    last_violation: Option<Instant>,
    flying_allowed: bool,
}

impl PlayerMovementState {
    fn new(position: Position) -> Self {
        Self {
            last_position: position,
            last_update: Instant::now(),
            on_ground: true,
            violations: 0,
            last_violation: None,
            flying_allowed: false,
        }
    }
}

/// Validates player movement to detect cheats
pub struct MovementValidator {
    config: ValidationConfig,
    players: HashMap<u64, PlayerMovementState>,
}

impl MovementValidator {
    /// Create a new movement validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            players: HashMap::new(),
        }
    }

    /// Register a new player
    pub fn register_player(&mut self, player_id: u64, position: Position) {
        self.players.insert(player_id, PlayerMovementState::new(position));
    }

    /// Remove a player
    pub fn remove_player(&mut self, player_id: u64) {
        self.players.remove(&player_id);
    }

    /// Set whether a player is allowed to fly
    pub fn set_flying_allowed(&mut self, player_id: u64, allowed: bool) {
        if let Some(state) = self.players.get_mut(&player_id) {
            state.flying_allowed = allowed;
        }
    }

    /// Validate a movement update
    pub fn validate_movement(
        &mut self,
        player_id: u64,
        new_position: Position,
        on_ground: bool,
    ) -> ValidationResult {
        let state = match self.players.get_mut(&player_id) {
            Some(s) => s,
            None => {
                // Unknown player, register them
                self.players.insert(player_id, PlayerMovementState::new(new_position));
                return ValidationResult::Valid;
            }
        };

        let now = Instant::now();
        let dt = now.duration_since(state.last_update).as_secs_f64();
        
        // Avoid division by zero for very fast updates
        if dt < 0.001 {
            return ValidationResult::Valid;
        }

        let distance = state.last_position.distance(&new_position);
        let horizontal_distance = state.last_position.horizontal_distance(&new_position);
        let vertical_distance = (new_position.y - state.last_position.y).abs();

        // Calculate speeds
        let speed = distance / dt;
        let horizontal_speed = horizontal_distance / dt;
        let vertical_speed = vertical_distance / dt;

        let result = self.check_movement(
            state,
            speed,
            horizontal_speed,
            vertical_speed,
            new_position,
            on_ground,
        );

        // Update state
        state.last_position = new_position;
        state.last_update = now;
        state.on_ground = on_ground;

        // Handle violations
        if matches!(result, ValidationResult::Invalid { .. }) {
            state.violations += 1;
            state.last_violation = Some(now);

            if state.violations >= self.config.violation_threshold {
                return ValidationResult::Kick {
                    reason: format!(
                        "Too many movement violations ({})",
                        state.violations
                    ),
                };
            }
        } else {
            // Decay violations over time
            if let Some(last_violation) = state.last_violation {
                if now.duration_since(last_violation) > Duration::from_secs(30) {
                    state.violations = state.violations.saturating_sub(1);
                    if state.violations == 0 {
                        state.last_violation = None;
                    }
                }
            }
        }

        result
    }

    fn check_movement(
        &self,
        state: &PlayerMovementState,
        speed: f64,
        horizontal_speed: f64,
        vertical_speed: f64,
        new_position: Position,
        on_ground: bool,
    ) -> ValidationResult {
        let max_speed = self.config.max_speed as f64 + self.config.position_tolerance as f64;
        let max_vertical = self.config.max_vertical_speed as f64 + self.config.position_tolerance as f64;

        // Check for teleportation (instant large distance)
        if speed > max_speed * 5.0 {
            return ValidationResult::Invalid {
                reason: format!("Teleportation detected: speed={:.2} blocks/s", speed),
            };
        }

        // Check horizontal speed
        if horizontal_speed > max_speed {
            return ValidationResult::Invalid {
                reason: format!(
                    "Speed hack detected: {:.2} blocks/s (max: {:.2})",
                    horizontal_speed, max_speed
                ),
            };
        }

        // Check vertical speed (going up without flying)
        if vertical_speed > max_vertical && new_position.y > state.last_position.y {
            if !state.flying_allowed {
                return ValidationResult::Suspicious {
                    reason: format!(
                        "High vertical speed: {:.2} blocks/s",
                        vertical_speed
                    ),
                };
            }
        }

        // Check for flying (not on ground for extended period without permission)
        if !on_ground && !state.on_ground && !state.flying_allowed {
            // This would need more context (time in air, etc.)
            // For now just log it
        }

        ValidationResult::Valid
    }

    /// Get violation count for a player
    pub fn get_violations(&self, player_id: u64) -> u32 {
        self.players
            .get(&player_id)
            .map(|s| s.violations)
            .unwrap_or(0)
    }

    /// Reset violations for a player
    pub fn reset_violations(&mut self, player_id: u64) {
        if let Some(state) = self.players.get_mut(&player_id) {
            state.violations = 0;
            state.last_violation = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_movement() {
        let mut validator = MovementValidator::new(ValidationConfig::default());
        validator.register_player(1, Position::new(0.0, 64.0, 0.0));
        
        // Small movement should be valid
        let result = validator.validate_movement(
            1,
            Position::new(0.1, 64.0, 0.1),
            true,
        );
        assert!(result.is_valid());
    }

    #[test]
    fn test_teleport_detection() {
        let mut validator = MovementValidator::new(ValidationConfig::default());
        validator.register_player(1, Position::new(0.0, 64.0, 0.0));
        
        // Wait a tiny bit to ensure dt > 0
        std::thread::sleep(Duration::from_millis(10));
        
        // Large instant movement should be invalid
        let result = validator.validate_movement(
            1,
            Position::new(1000.0, 64.0, 1000.0),
            true,
        );
        assert!(!result.is_valid());
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0.0, 0.0, 0.0);
        let p2 = Position::new(3.0, 4.0, 0.0);
        assert!((p1.distance(&p2) - 5.0).abs() < 0.001);
    }
}

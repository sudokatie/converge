//! Fall damage calculation.
//!
//! Implements spec 6.5.1: fall damage of 1 per block above 3 blocks fallen.
//! Also provides drowning damage tracking.

/// Minimum fall distance before damage applies (in blocks).
pub const FALL_DAMAGE_THRESHOLD: f32 = 3.0;

/// Damage per block fallen above the threshold.
pub const FALL_DAMAGE_PER_BLOCK: f32 = 1.0;

/// Drowning damage per second when underwater.
pub const DROWNING_DAMAGE_PER_SEC: f32 = 2.0;

/// Seconds of air before drowning starts.
pub const DROWNING_GRACE_PERIOD: f32 = 10.0;

/// Maximum air supply in seconds.
pub const MAX_AIR_SUPPLY: f32 = 10.0;

/// Tracks fall state and calculates damage on landing.
#[derive(Clone, Debug)]
pub struct FallDamageTracker {
    /// Whether the player was in the air last frame.
    was_in_air: bool,
    /// Highest Y position during current fall.
    fall_start_y: f32,
    /// Pending fall damage to apply (set on landing, consumed by game code).
    pending_damage: f32,
}

impl Default for FallDamageTracker {
    fn default() -> Self {
        Self {
            was_in_air: false,
            fall_start_y: 0.0,
            pending_damage: 0.0,
        }
    }
}

impl FallDamageTracker {
    /// Create a new fall damage tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update fall damage state each frame.
    ///
    /// Call every frame with the player's current on_ground state and Y position.
    /// Returns the fall damage taken this frame (non-zero only on the landing frame).
    pub fn update(&mut self, on_ground: bool, position_y: f32) -> f32 {
        if !on_ground {
            // In the air
            if !self.was_in_air {
                // Just left the ground - start tracking fall
                self.fall_start_y = position_y;
            } else if position_y > self.fall_start_y {
                // Moving up (jump, slab, etc.) - reset start
                self.fall_start_y = position_y;
            }
            self.was_in_air = true;
            self.pending_damage = 0.0;
            0.0
        } else if self.was_in_air {
            // Just landed
            let fall_distance = self.fall_start_y - position_y;
            self.pending_damage = calculate_fall_damage(fall_distance);
            self.was_in_air = false;
            self.pending_damage
        } else {
            // On ground, was on ground
            self.pending_damage = 0.0;
            0.0
        }
    }

    /// Get pending fall damage (consumed by caller).
    #[must_use]
    pub fn take_pending_damage(&mut self) -> f32 {
        let damage = self.pending_damage;
        self.pending_damage = 0.0;
        damage
    }

    /// Check if currently tracking a fall.
    #[must_use]
    pub fn is_falling(&self) -> bool {
        self.was_in_air
    }

    /// Get the fall start height.
    #[must_use]
    pub fn fall_start_y(&self) -> f32 {
        self.fall_start_y
    }
}

/// Calculate fall damage from fall distance.
///
/// Damage = max(0, fall_distance - FALL_DAMAGE_THRESHOLD) * FALL_DAMAGE_PER_BLOCK
#[must_use]
pub fn calculate_fall_damage(fall_distance: f32) -> f32 {
    if fall_distance <= FALL_DAMAGE_THRESHOLD {
        0.0
    } else {
        (fall_distance - FALL_DAMAGE_THRESHOLD) * FALL_DAMAGE_PER_BLOCK
    }
}

/// Tracks drowning state when the player is underwater.
#[derive(Clone, Debug)]
pub struct DrowningTracker {
    /// Current air supply in seconds.
    air_supply: f32,
    /// Whether the player is currently submerged.
    is_submerged: bool,
    /// Pending drowning damage this frame.
    pending_damage: f32,
}

impl Default for DrowningTracker {
    fn default() -> Self {
        Self {
            air_supply: MAX_AIR_SUPPLY,
            is_submerged: false,
            pending_damage: 0.0,
        }
    }
}

impl DrowningTracker {
    /// Create a new drowning tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update drowning state each frame.
    ///
    /// Call every frame with whether the player's head is in water.
    pub fn update(&mut self, head_in_water: bool, dt: f32) -> f32 {
        self.is_submerged = head_in_water;
        self.pending_damage = 0.0;

        if head_in_water {
            self.air_supply -= dt;
            if self.air_supply <= 0.0 {
                self.air_supply = 0.0;
                // Drowning damage
                self.pending_damage = DROWNING_DAMAGE_PER_SEC * dt;
            }
        } else {
            // Recover air
            self.air_supply = (self.air_supply + dt * 2.0).min(MAX_AIR_SUPPLY);
        }

        self.pending_damage
    }

    /// Get current air supply (0.0 to MAX_AIR_SUPPLY).
    #[must_use]
    pub fn air_supply(&self) -> f32 {
        self.air_supply
    }

    /// Get air supply as fraction (0.0 to 1.0).
    #[must_use]
    pub fn air_fraction(&self) -> f32 {
        self.air_supply / MAX_AIR_SUPPLY
    }

    /// Check if currently drowning (air depleted).
    #[must_use]
    pub fn is_drowning(&self) -> bool {
        self.air_supply <= 0.0 && self.is_submerged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_damage_short_fall() {
        assert_eq!(calculate_fall_damage(0.0), 0.0);
        assert_eq!(calculate_fall_damage(1.0), 0.0);
        assert_eq!(calculate_fall_damage(2.9), 0.0);
        assert_eq!(calculate_fall_damage(3.0), 0.0);
    }

    #[test]
    fn test_damage_above_threshold() {
        // 4 blocks: 4 - 3 = 1 damage
        assert!((calculate_fall_damage(4.0) - 1.0).abs() < 0.001);
        // 7 blocks: 7 - 3 = 4 damage
        assert!((calculate_fall_damage(7.0) - 4.0).abs() < 0.001);
        // 13 blocks: 13 - 3 = 10 damage (lethal)
        assert!((calculate_fall_damage(13.0) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_negative_fall_distance() {
        // Should not happen in practice, but handle gracefully
        assert_eq!(calculate_fall_damage(-1.0), 0.0);
    }

    #[test]
    fn test_fall_tracker_no_damage_ground() {
        let mut tracker = FallDamageTracker::new();

        // Start on ground, stay on ground
        let damage = tracker.update(true, 10.0);
        assert_eq!(damage, 0.0);
    }

    #[test]
    fn test_fall_tracker_damage_on_landing() {
        let mut tracker = FallDamageTracker::new();

        // Walk off edge (y=10)
        let damage = tracker.update(false, 10.0);
        assert_eq!(damage, 0.0); // No damage yet

        // Falling (y=8)
        let damage = tracker.update(false, 8.0);
        assert_eq!(damage, 0.0);

        // Land (y=4, fell 6 blocks = 3 damage)
        let damage = tracker.update(true, 4.0);
        assert!((damage - 3.0).abs() < 0.001, "Expected 3 damage from 6-block fall, got {damage}");
    }

    #[test]
    fn test_fall_tracker_no_damage_short_fall() {
        let mut tracker = FallDamageTracker::new();

        // Walk off small ledge (y=10)
        tracker.update(false, 10.0);

        // Land quickly (y=8, only 2 blocks = no damage)
        let damage = tracker.update(true, 8.0);
        assert_eq!(damage, 0.0);
    }

    #[test]
    fn test_fall_tracker_resets_on_new_fall() {
        let mut tracker = FallDamageTracker::new();

        // First fall
        tracker.update(false, 20.0);
        let damage1 = tracker.update(true, 10.0); // 10 blocks = 7 damage
        assert!((damage1 - 7.0).abs() < 0.001);

        // Second fall from lower height
        tracker.update(false, 14.0);
        let damage2 = tracker.update(true, 12.0); // 2 blocks = no damage
        assert_eq!(damage2, 0.0);
    }

    #[test]
    fn test_fall_tracker_jump_resets_start() {
        let mut tracker = FallDamageTracker::new();

        // Fall from y=10
        tracker.update(false, 10.0);
        // But going up (jumping) resets start
        tracker.update(false, 12.0);
        // Land at y=9, only fell 3 blocks from y=12
        let damage = tracker.update(true, 9.0); // 3 blocks = no damage
        assert_eq!(damage, 0.0);
    }

    #[test]
    fn test_pending_damage_consumed() {
        let mut tracker = FallDamageTracker::new();
        tracker.update(false, 10.0);
        tracker.update(true, 4.0); // 6 blocks = 3 damage

        let damage = tracker.take_pending_damage();
        assert!((damage - 3.0).abs() < 0.001);

        // Should be consumed
        assert_eq!(tracker.take_pending_damage(), 0.0);
    }

    #[test]
    fn test_drowning_no_damage_with_air() {
        let mut tracker = DrowningTracker::new();

        // Underwater for 5 seconds (within grace period)
        for _ in 0..50 {
            let damage = tracker.update(true, 0.1);
            assert_eq!(damage, 0.0, "No damage while air supply remains");
        }

        assert!(tracker.air_supply() > 0.0, "Should still have air");
        assert!(!tracker.is_drowning());
    }

    #[test]
    fn test_drowning_damage_after_air_depleted() {
        let mut tracker = DrowningTracker::new();

        // Use up all air (10 seconds)
        for _ in 0..100 {
            tracker.update(true, 0.1);
        }

        assert!(tracker.air_supply() <= 0.0, "Air should be depleted");
        assert!(tracker.is_drowning());

        // Now should be taking damage
        let damage = tracker.update(true, 0.1);
        assert!(damage > 0.0, "Should take drowning damage");
    }

    #[test]
    fn test_drowning_air_recovery() {
        let mut tracker = DrowningTracker::new();

        // Deplete air
        for _ in 0..100 {
            tracker.update(true, 0.1);
        }

        // Surface and recover
        for _ in 0..50 {
            tracker.update(false, 0.1);
        }

        assert!(tracker.air_supply() > 0.0, "Air should recover on surface");
        assert!(!tracker.is_drowning());
    }

    #[test]
    fn test_drowning_air_fraction() {
        let tracker = DrowningTracker::new();
        assert!((tracker.air_fraction() - 1.0).abs() < 0.001, "Should start with full air");
    }
}

//! Contact and environment state for the character controller.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Environment type the character is currently in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentType {
    /// Standard air environment.
    #[default]
    Air,
    /// Submerged in liquid.
    Liquid,
    /// Zero-gravity zone.
    ZeroGravity,
    /// High-pressure atmosphere.
    HighPressure,
}

/// Packed surface contact flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactFlags(u8);

impl ContactFlags {
    const GROUND: u8 = 1 << 0;
    const CEILING: u8 = 1 << 1;
    const WALL: u8 = 1 << 2;
    const CLIMBABLE: u8 = 1 << 3;
    const LIQUID: u8 = 1 << 4;

    #[must_use]
    pub fn on_ground(self) -> bool {
        self.0 & Self::GROUND != 0
    }

    pub fn set_ground(&mut self, v: bool) {
        if v {
            self.0 |= Self::GROUND;
        } else {
            self.0 &= !Self::GROUND;
        }
    }

    #[must_use]
    pub fn on_ceiling(self) -> bool {
        self.0 & Self::CEILING != 0
    }

    pub fn set_ceiling(&mut self, v: bool) {
        if v {
            self.0 |= Self::CEILING;
        } else {
            self.0 &= !Self::CEILING;
        }
    }

    #[must_use]
    pub fn on_wall(self) -> bool {
        self.0 & Self::WALL != 0
    }

    pub fn set_wall(&mut self, v: bool) {
        if v {
            self.0 |= Self::WALL;
        } else {
            self.0 &= !Self::WALL;
        }
    }

    #[must_use]
    pub fn on_climbable(self) -> bool {
        self.0 & Self::CLIMBABLE != 0
    }

    pub fn set_climbable(&mut self, v: bool) {
        if v {
            self.0 |= Self::CLIMBABLE;
        } else {
            self.0 &= !Self::CLIMBABLE;
        }
    }

    #[must_use]
    pub fn in_liquid(self) -> bool {
        self.0 & Self::LIQUID != 0
    }

    pub fn set_liquid(&mut self, v: bool) {
        if v {
            self.0 |= Self::LIQUID;
        } else {
            self.0 &= !Self::LIQUID;
        }
    }
}

/// Contact state describing the character's relationship to surfaces.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactState {
    /// Surface contact flags (ground, ceiling, wall, climbable, liquid).
    pub flags: ContactFlags,
    /// Liquid depth (`0.0` = not in liquid, `1.0` = fully submerged).
    pub liquid_depth: f32,
    /// Ground normal (if `on_ground`).
    pub ground_normal: Vec3,
    /// Wall normal (if `on_wall`).
    pub wall_normal: Vec3,
    /// The current environment type.
    pub environment: EnvironmentType,
}

impl ContactState {
    /// Create a new contact state with no contacts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ground_normal: Vec3::Y,
            wall_normal: Vec3::ZERO,
            ..Default::default()
        }
    }

    /// Create a grounded contact state.
    #[must_use]
    pub fn grounded() -> Self {
        let mut flags = ContactFlags::default();
        flags.set_ground(true);
        Self {
            flags,
            ground_normal: Vec3::Y,
            ..Default::default()
        }
    }

    /// Create a grounded contact state with a custom normal.
    #[must_use]
    pub fn grounded_with_normal(normal: Vec3) -> Self {
        let mut flags = ContactFlags::default();
        flags.set_ground(true);
        Self {
            flags,
            ground_normal: normal.normalize_or_zero(),
            ..Default::default()
        }
    }

    /// Create a swimming contact state.
    #[must_use]
    pub fn swimming(depth: f32) -> Self {
        let mut flags = ContactFlags::default();
        flags.set_liquid(true);
        Self {
            flags,
            liquid_depth: depth.clamp(0.0, 1.0),
            environment: EnvironmentType::Liquid,
            ground_normal: Vec3::Y,
            ..Default::default()
        }
    }

    /// Create a climbing contact state.
    #[must_use]
    pub fn climbing(wall_normal: Vec3) -> Self {
        let mut flags = ContactFlags::default();
        flags.set_wall(true);
        flags.set_climbable(true);
        Self {
            flags,
            wall_normal: wall_normal.normalize_or_zero(),
            ground_normal: Vec3::Y,
            ..Default::default()
        }
    }

    /// Create a zero-G contact state.
    #[must_use]
    pub fn zero_g() -> Self {
        Self {
            environment: EnvironmentType::ZeroGravity,
            ground_normal: Vec3::Y,
            ..Default::default()
        }
    }

    /// Whether the character is on the ground.
    #[must_use]
    pub fn on_ground(&self) -> bool {
        self.flags.on_ground()
    }

    /// Whether the character is touching a ceiling.
    #[must_use]
    pub fn on_ceiling(&self) -> bool {
        self.flags.on_ceiling()
    }

    /// Whether the character is touching a wall.
    #[must_use]
    pub fn on_wall(&self) -> bool {
        self.flags.on_wall()
    }

    /// Whether the character is on a climbable surface.
    #[must_use]
    pub fn on_climbable(&self) -> bool {
        self.flags.on_climbable()
    }

    /// Whether the character is in liquid.
    #[must_use]
    pub fn in_liquid(&self) -> bool {
        self.flags.in_liquid()
    }

    /// Whether the character is airborne (not on ground, wall, or in liquid).
    #[must_use]
    pub fn is_airborne(&self) -> bool {
        !self.on_ground() && !self.on_wall() && !self.in_liquid()
    }

    /// Whether the character can jump from current contact.
    #[must_use]
    pub fn can_jump(&self) -> bool {
        self.on_ground()
    }

    /// Whether the character should use swimming physics.
    #[must_use]
    pub fn should_swim(&self) -> bool {
        self.in_liquid() && self.liquid_depth > 0.5
    }

    /// Whether the character has any wall contact.
    #[must_use]
    pub fn has_wall_contact(&self) -> bool {
        self.on_wall() && self.wall_normal.length_squared() > 0.0001
    }

    /// Get the effective up direction based on ground normal.
    #[must_use]
    pub fn up_direction(&self) -> Vec3 {
        if self.ground_normal.length_squared() > 0.0001 {
            self.ground_normal
        } else {
            Vec3::Y
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_airborne() {
        let contact = ContactState::new();
        assert!(contact.is_airborne());
        assert!(!contact.can_jump());
    }

    #[test]
    fn grounded_contact() {
        let contact = ContactState::grounded();
        assert!(contact.on_ground());
        assert!(!contact.is_airborne());
        assert!(contact.can_jump());
    }

    #[test]
    fn swimming_contact() {
        let contact = ContactState::swimming(0.8);
        assert!(contact.in_liquid());
        assert!(contact.should_swim());
        assert!((contact.liquid_depth - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn shallow_water_no_swim() {
        let contact = ContactState::swimming(0.3);
        assert!(contact.in_liquid());
        assert!(!contact.should_swim());
    }

    #[test]
    fn climbing_contact() {
        let contact = ContactState::climbing(Vec3::X);
        assert!(contact.on_wall());
        assert!(contact.on_climbable());
        assert!(contact.has_wall_contact());
    }

    #[test]
    fn zero_g_contact() {
        let contact = ContactState::zero_g();
        assert_eq!(contact.environment, EnvironmentType::ZeroGravity);
        assert!(contact.is_airborne());
    }

    #[test]
    fn up_direction_from_normal() {
        let contact = ContactState::grounded_with_normal(Vec3::new(0.0, 1.0, 0.1).normalize());
        let up = contact.up_direction();
        assert!(up.y > 0.9);
    }
}

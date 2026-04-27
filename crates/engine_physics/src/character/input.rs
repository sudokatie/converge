//! Input state for the character controller.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Packed button state flags for character input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputActions(u8);

impl InputActions {
    const JUMP: u8 = 1 << 0;
    const SPRINT: u8 = 1 << 1;
    const CROUCH: u8 = 1 << 2;
    const PRIMARY: u8 = 1 << 3;
    const SECONDARY: u8 = 1 << 4;

    #[must_use]
    pub fn jump(self) -> bool {
        self.0 & Self::JUMP != 0
    }

    pub fn set_jump(&mut self, v: bool) {
        if v {
            self.0 |= Self::JUMP;
        } else {
            self.0 &= !Self::JUMP;
        }
    }

    #[must_use]
    pub fn sprint(self) -> bool {
        self.0 & Self::SPRINT != 0
    }

    pub fn set_sprint(&mut self, v: bool) {
        if v {
            self.0 |= Self::SPRINT;
        } else {
            self.0 &= !Self::SPRINT;
        }
    }

    #[must_use]
    pub fn crouch(self) -> bool {
        self.0 & Self::CROUCH != 0
    }

    pub fn set_crouch(&mut self, v: bool) {
        if v {
            self.0 |= Self::CROUCH;
        } else {
            self.0 &= !Self::CROUCH;
        }
    }

    #[must_use]
    pub fn primary_action(self) -> bool {
        self.0 & Self::PRIMARY != 0
    }

    pub fn set_primary_action(&mut self, v: bool) {
        if v {
            self.0 |= Self::PRIMARY;
        } else {
            self.0 &= !Self::PRIMARY;
        }
    }

    #[must_use]
    pub fn secondary_action(self) -> bool {
        self.0 & Self::SECONDARY != 0
    }

    pub fn set_secondary_action(&mut self, v: bool) {
        if v {
            self.0 |= Self::SECONDARY;
        } else {
            self.0 &= !Self::SECONDARY;
        }
    }
}

/// Input state representing player intentions for a single tick.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterInput {
    /// Movement direction in local space (x = right, y = up, z = forward).
    /// Should be normalized or zero.
    pub movement: Vec3,
    /// Button action states (jump, sprint, crouch, primary/secondary actions).
    pub actions: InputActions,
    /// Look direction (normalized, used for climbing orientation).
    pub look_direction: Vec3,
}

impl CharacterInput {
    /// Create a new input state with no inputs active.
    #[must_use]
    pub fn new() -> Self {
        Self {
            look_direction: Vec3::NEG_Z,
            ..Default::default()
        }
    }

    /// Create input for horizontal movement only.
    #[must_use]
    pub fn horizontal(x: f32, z: f32) -> Self {
        Self {
            movement: Vec3::new(x, 0.0, z).normalize_or_zero(),
            look_direction: Vec3::NEG_Z,
            ..Default::default()
        }
    }

    /// Create input for 3D movement (swimming/climbing/zero-G).
    #[must_use]
    pub fn full_3d(x: f32, y: f32, z: f32) -> Self {
        Self {
            movement: Vec3::new(x, y, z).normalize_or_zero(),
            look_direction: Vec3::NEG_Z,
            ..Default::default()
        }
    }

    /// Set movement direction.
    #[must_use]
    pub fn with_movement(mut self, movement: Vec3) -> Self {
        self.movement = movement.normalize_or_zero();
        self
    }

    /// Set jump state.
    #[must_use]
    pub fn with_jump(mut self, jump: bool) -> Self {
        self.actions.set_jump(jump);
        self
    }

    /// Set sprint state.
    #[must_use]
    pub fn with_sprint(mut self, sprint: bool) -> Self {
        self.actions.set_sprint(sprint);
        self
    }

    /// Set crouch state.
    #[must_use]
    pub fn with_crouch(mut self, crouch: bool) -> Self {
        self.actions.set_crouch(crouch);
        self
    }

    /// Set primary action state.
    #[must_use]
    pub fn with_primary_action(mut self, pressed: bool) -> Self {
        self.actions.set_primary_action(pressed);
        self
    }

    /// Set secondary action state.
    #[must_use]
    pub fn with_secondary_action(mut self, pressed: bool) -> Self {
        self.actions.set_secondary_action(pressed);
        self
    }

    /// Set look direction.
    #[must_use]
    pub fn with_look_direction(mut self, direction: Vec3) -> Self {
        self.look_direction = direction.normalize_or_zero();
        if self.look_direction == Vec3::ZERO {
            self.look_direction = Vec3::NEG_Z;
        }
        self
    }

    /// Whether any movement input is active.
    #[must_use]
    pub fn has_movement(&self) -> bool {
        self.movement.length_squared() > 0.0001
    }

    /// Get the horizontal movement component.
    #[must_use]
    pub fn horizontal_movement(&self) -> Vec3 {
        Vec3::new(self.movement.x, 0.0, self.movement.z)
    }

    /// Get the vertical movement component.
    #[must_use]
    pub fn vertical_movement(&self) -> f32 {
        self.movement.y
    }

    /// Whether jump is pressed.
    #[must_use]
    pub fn jump(&self) -> bool {
        self.actions.jump()
    }

    /// Whether sprint is active.
    #[must_use]
    pub fn sprint(&self) -> bool {
        self.actions.sprint()
    }

    /// Whether crouch is pressed.
    #[must_use]
    pub fn crouch(&self) -> bool {
        self.actions.crouch()
    }

    /// Whether primary action is pressed.
    #[must_use]
    pub fn primary_action(&self) -> bool {
        self.actions.primary_action()
    }

    /// Whether secondary action is pressed.
    #[must_use]
    pub fn secondary_action(&self) -> bool {
        self.actions.secondary_action()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_input_is_neutral() {
        let input = CharacterInput::new();
        assert!(!input.has_movement());
        assert!(!input.jump());
        assert!(!input.sprint());
        assert!(!input.crouch());
    }

    #[test]
    fn horizontal_movement() {
        let input = CharacterInput::horizontal(1.0, 0.0);
        assert!(input.has_movement());
        assert!(input.horizontal_movement().x > 0.0);
        assert!(input.vertical_movement().abs() < f32::EPSILON);
    }

    #[test]
    fn full_3d_movement() {
        let input = CharacterInput::full_3d(0.0, 1.0, 0.0);
        assert!(input.has_movement());
        assert!(input.vertical_movement() > 0.0);
    }

    #[test]
    fn builder_pattern() {
        let input = CharacterInput::new()
            .with_movement(Vec3::new(0.0, 0.0, 1.0))
            .with_jump(true)
            .with_sprint(true);

        assert!(input.has_movement());
        assert!(input.jump());
        assert!(input.sprint());
    }

    #[test]
    fn movement_normalization() {
        let input = CharacterInput::horizontal(10.0, 0.0);
        let len = input.movement.length();
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn zero_movement_normalized() {
        let input = CharacterInput::horizontal(0.0, 0.0);
        assert!(input.movement.abs_diff_eq(Vec3::ZERO, f32::EPSILON));
        assert!(!input.has_movement());
    }
}

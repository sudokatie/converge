//! Action mapping for input abstraction.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::InputState;

/// Game actions that can be bound to keys/buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    Jump,
    Crouch,
    Sprint,
    Attack,
    UseItem,
    Interact,
    Inventory,
    Pause,
    Chat,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    Hotbar9,
}

/// A key or mouse button binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyBinding {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl From<KeyCode> for KeyBinding {
    fn from(key: KeyCode) -> Self {
        KeyBinding::Key(key)
    }
}

impl From<MouseButton> for KeyBinding {
    fn from(button: MouseButton) -> Self {
        KeyBinding::Mouse(button)
    }
}

/// Error type for action map operations.
#[derive(Debug, Error)]
pub enum ActionMapError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialize(#[from] ron::Error),
    #[error("Deserialization error: {0}")]
    Deserialize(#[from] ron::de::SpannedError),
}

/// Maps actions to input bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMap {
    bindings: HashMap<Action, Vec<KeyBinding>>,
}

impl ActionMap {
    /// Create an empty action map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Create an action map with default bindings.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut map = Self::new();

        // Movement
        map.bind(Action::MoveForward, KeyCode::KeyW);
        map.bind(Action::MoveBack, KeyCode::KeyS);
        map.bind(Action::MoveLeft, KeyCode::KeyA);
        map.bind(Action::MoveRight, KeyCode::KeyD);
        map.bind(Action::Jump, KeyCode::Space);
        map.bind(Action::Crouch, KeyCode::ControlLeft);
        map.bind(Action::Sprint, KeyCode::ShiftLeft);

        // Actions
        map.bind(Action::Attack, MouseButton::Left);
        map.bind(Action::UseItem, MouseButton::Right);
        map.bind(Action::Interact, KeyCode::KeyE);

        // UI
        map.bind(Action::Inventory, KeyCode::Tab);
        map.bind(Action::Pause, KeyCode::Escape);
        map.bind(Action::Chat, KeyCode::KeyT);

        // Hotbar
        map.bind(Action::Hotbar1, KeyCode::Digit1);
        map.bind(Action::Hotbar2, KeyCode::Digit2);
        map.bind(Action::Hotbar3, KeyCode::Digit3);
        map.bind(Action::Hotbar4, KeyCode::Digit4);
        map.bind(Action::Hotbar5, KeyCode::Digit5);
        map.bind(Action::Hotbar6, KeyCode::Digit6);
        map.bind(Action::Hotbar7, KeyCode::Digit7);
        map.bind(Action::Hotbar8, KeyCode::Digit8);
        map.bind(Action::Hotbar9, KeyCode::Digit9);

        map
    }

    /// Bind an input to an action.
    pub fn bind(&mut self, action: Action, binding: impl Into<KeyBinding>) {
        let binding = binding.into();
        self.bindings
            .entry(action)
            .or_default()
            .push(binding);
    }

    /// Remove a binding from an action.
    pub fn unbind(&mut self, action: Action, binding: impl Into<KeyBinding>) {
        let binding = binding.into();
        if let Some(bindings) = self.bindings.get_mut(&action) {
            bindings.retain(|b| *b != binding);
        }
    }

    /// Clear all bindings for an action.
    pub fn clear(&mut self, action: Action) {
        self.bindings.remove(&action);
    }

    /// Get all bindings for an action.
    #[must_use]
    pub fn get_bindings(&self, action: Action) -> &[KeyBinding] {
        self.bindings.get(&action).map_or(&[], |v| v.as_slice())
    }

    /// Check if an action was pressed this frame.
    #[must_use]
    pub fn is_action_pressed(&self, action: Action, input: &InputState) -> bool {
        self.get_bindings(action).iter().any(|binding| match binding {
            KeyBinding::Key(key) => input.is_key_pressed(*key),
            KeyBinding::Mouse(btn) => input.is_mouse_button_pressed(*btn),
        })
    }

    /// Check if an action is currently held.
    #[must_use]
    pub fn is_action_held(&self, action: Action, input: &InputState) -> bool {
        self.get_bindings(action).iter().any(|binding| match binding {
            KeyBinding::Key(key) => input.is_key_held(*key),
            KeyBinding::Mouse(btn) => input.is_mouse_button_held(*btn),
        })
    }

    /// Check if an action was released this frame.
    #[must_use]
    pub fn is_action_released(&self, action: Action, input: &InputState) -> bool {
        self.get_bindings(action).iter().any(|binding| match binding {
            KeyBinding::Key(key) => input.is_key_released(*key),
            KeyBinding::Mouse(btn) => input.is_mouse_button_released(*btn),
        })
    }

    /// Save the action map to a file (RON format).
    ///
    /// # Errors
    /// Returns an error if the file cannot be written or serialization fails.
    pub fn save(&self, path: &Path) -> Result<(), ActionMapError> {
        let contents = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Load an action map from a file (RON format).
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or deserialization fails.
    pub fn load(path: &Path) -> Result<Self, ActionMapError> {
        let contents = fs::read_to_string(path)?;
        let map = ron::from_str(&contents)?;
        Ok(map)
    }
}

impl Default for ActionMap {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::WindowEvent;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_input_with_key(key: KeyCode) -> InputState {
        let mut input = InputState::new();
        input.update(&WindowEvent::KeyboardInput { key, pressed: true });
        input
    }

    #[test]
    fn test_default_wasd() {
        let map = ActionMap::with_defaults();
        let input = make_input_with_key(KeyCode::KeyW);
        assert!(map.is_action_pressed(Action::MoveForward, &input));
        assert!(map.is_action_held(Action::MoveForward, &input));
    }

    #[test]
    fn test_rebind() {
        let mut map = ActionMap::with_defaults();
        let input_w = make_input_with_key(KeyCode::KeyW);
        let input_up = make_input_with_key(KeyCode::ArrowUp);

        // Default: W moves forward
        assert!(map.is_action_held(Action::MoveForward, &input_w));
        assert!(!map.is_action_held(Action::MoveForward, &input_up));

        // Add arrow key binding
        map.bind(Action::MoveForward, KeyCode::ArrowUp);
        assert!(map.is_action_held(Action::MoveForward, &input_up));

        // Remove W binding
        map.unbind(Action::MoveForward, KeyCode::KeyW);
        assert!(!map.is_action_held(Action::MoveForward, &input_w));
        assert!(map.is_action_held(Action::MoveForward, &input_up));
    }

    #[test]
    fn test_multiple_bindings() {
        let mut map = ActionMap::new();
        map.bind(Action::Jump, KeyCode::Space);
        map.bind(Action::Jump, KeyCode::KeyJ);

        let input_space = make_input_with_key(KeyCode::Space);
        let input_j = make_input_with_key(KeyCode::KeyJ);

        assert!(map.is_action_held(Action::Jump, &input_space));
        assert!(map.is_action_held(Action::Jump, &input_j));
    }

    #[test]
    fn test_save_load() {
        let mut map = ActionMap::new();
        map.bind(Action::Attack, KeyCode::KeyF);
        map.bind(Action::Jump, KeyCode::Space);

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"// placeholder").unwrap();
        let path = temp.path();

        map.save(path).unwrap();
        let loaded = ActionMap::load(path).unwrap();

        assert_eq!(loaded.get_bindings(Action::Attack).len(), 1);
        assert_eq!(loaded.get_bindings(Action::Jump).len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut map = ActionMap::with_defaults();
        assert!(!map.get_bindings(Action::MoveForward).is_empty());
        map.clear(Action::MoveForward);
        assert!(map.get_bindings(Action::MoveForward).is_empty());
    }
}

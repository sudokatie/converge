//! Input state tracking.

use std::collections::HashSet;

use glam::Vec2;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::WindowEvent;

/// Tracks keyboard and mouse input state.
///
/// Update with window events each frame, then call `end_frame()`.
pub struct InputState {
    // Keyboard
    keys_held: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,

    // Mouse
    mouse_position: Vec2,
    mouse_delta: Vec2,
    mouse_buttons_held: HashSet<MouseButton>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_released: HashSet<MouseButton>,
    scroll_delta: Vec2,

    // Previous frame state for delta calculation
    prev_mouse_position: Vec2,

    // Window focus
    focused: bool,
}

impl InputState {
    /// Create a new input state tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            mouse_buttons_held: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_released: HashSet::new(),
            scroll_delta: Vec2::ZERO,
            prev_mouse_position: Vec2::ZERO,
            focused: true,
        }
    }

    /// Process a window event to update input state.
    pub fn update(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { key, pressed } => {
                if *pressed {
                    if !self.keys_held.contains(key) {
                        self.keys_pressed.insert(*key);
                    }
                    self.keys_held.insert(*key);
                } else {
                    self.keys_held.remove(key);
                    self.keys_released.insert(*key);
                }
            }
            WindowEvent::MouseMoved { x, y } => {
                self.mouse_position = Vec2::new(*x as f32, *y as f32);
            }
            WindowEvent::MouseButton { button, pressed } => {
                if *pressed {
                    if !self.mouse_buttons_held.contains(button) {
                        self.mouse_buttons_pressed.insert(*button);
                    }
                    self.mouse_buttons_held.insert(*button);
                } else {
                    self.mouse_buttons_held.remove(button);
                    self.mouse_buttons_released.insert(*button);
                }
            }
            WindowEvent::MouseScroll { delta_x, delta_y } => {
                self.scroll_delta += Vec2::new(*delta_x as f32, *delta_y as f32);
            }
            WindowEvent::Focused => {
                self.focused = true;
            }
            WindowEvent::Unfocused => {
                self.focused = false;
                // Clear all held state when focus is lost
                self.keys_held.clear();
                self.mouse_buttons_held.clear();
            }
            _ => {}
        }
    }

    /// Call at the end of each frame to reset per-frame state.
    pub fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_buttons_pressed.clear();
        self.mouse_buttons_released.clear();
        self.mouse_delta = self.mouse_position - self.prev_mouse_position;
        self.prev_mouse_position = self.mouse_position;
        self.scroll_delta = Vec2::ZERO;
    }

    /// Check if a key was pressed this frame (not held from before).
    #[must_use]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key is currently held down.
    #[must_use]
    pub fn is_key_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    /// Check if a key was released this frame.
    #[must_use]
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Get the current mouse position in window coordinates.
    #[must_use]
    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    /// Get the mouse movement since last frame.
    #[must_use]
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    /// Check if a mouse button was pressed this frame.
    #[must_use]
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    /// Check if a mouse button is currently held.
    #[must_use]
    pub fn is_mouse_button_held(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
    }

    /// Check if a mouse button was released this frame.
    #[must_use]
    pub fn is_mouse_button_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_released.contains(&button)
    }

    /// Get the scroll wheel delta this frame.
    #[must_use]
    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    /// Check if the window currently has focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Check if any key is currently held.
    #[must_use]
    pub fn any_key_held(&self) -> bool {
        !self.keys_held.is_empty()
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_press() {
        let mut input = InputState::new();

        // Key not pressed initially
        assert!(!input.is_key_pressed(KeyCode::KeyW));
        assert!(!input.is_key_held(KeyCode::KeyW));

        // Press key
        input.update(&WindowEvent::KeyboardInput {
            key: KeyCode::KeyW,
            pressed: true,
        });

        assert!(input.is_key_pressed(KeyCode::KeyW));
        assert!(input.is_key_held(KeyCode::KeyW));

        // After end_frame, pressed clears but held remains
        input.end_frame();
        assert!(!input.is_key_pressed(KeyCode::KeyW));
        assert!(input.is_key_held(KeyCode::KeyW));
    }

    #[test]
    fn test_key_release() {
        let mut input = InputState::new();

        // Press then release
        input.update(&WindowEvent::KeyboardInput {
            key: KeyCode::Space,
            pressed: true,
        });
        input.end_frame();
        input.update(&WindowEvent::KeyboardInput {
            key: KeyCode::Space,
            pressed: false,
        });

        assert!(!input.is_key_held(KeyCode::Space));
        assert!(input.is_key_released(KeyCode::Space));

        input.end_frame();
        assert!(!input.is_key_released(KeyCode::Space));
    }

    #[test]
    fn test_mouse_position() {
        let mut input = InputState::new();

        input.update(&WindowEvent::MouseMoved { x: 100.0, y: 200.0 });
        assert_eq!(input.mouse_position(), Vec2::new(100.0, 200.0));
    }

    #[test]
    fn test_mouse_delta() {
        let mut input = InputState::new();

        input.update(&WindowEvent::MouseMoved { x: 100.0, y: 100.0 });
        input.end_frame();
        input.update(&WindowEvent::MouseMoved { x: 150.0, y: 120.0 });
        input.end_frame();

        assert_eq!(input.mouse_delta(), Vec2::new(50.0, 20.0));
    }

    #[test]
    fn test_mouse_buttons() {
        let mut input = InputState::new();

        input.update(&WindowEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        });

        assert!(input.is_mouse_button_pressed(MouseButton::Left));
        assert!(input.is_mouse_button_held(MouseButton::Left));

        input.end_frame();
        assert!(!input.is_mouse_button_pressed(MouseButton::Left));
        assert!(input.is_mouse_button_held(MouseButton::Left));
    }

    #[test]
    fn test_scroll() {
        let mut input = InputState::new();

        input.update(&WindowEvent::MouseScroll {
            delta_x: 0.0,
            delta_y: 3.0,
        });

        assert_eq!(input.scroll_delta(), Vec2::new(0.0, 3.0));

        input.end_frame();
        assert_eq!(input.scroll_delta(), Vec2::ZERO);
    }

    #[test]
    fn test_focus_clears_held() {
        let mut input = InputState::new();

        input.update(&WindowEvent::KeyboardInput {
            key: KeyCode::KeyA,
            pressed: true,
        });
        assert!(input.is_key_held(KeyCode::KeyA));

        // Lose focus
        input.update(&WindowEvent::Unfocused);
        assert!(!input.is_key_held(KeyCode::KeyA));
    }
}

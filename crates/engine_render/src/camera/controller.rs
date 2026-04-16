//! First-person camera controller.

use engine_core::platform::{Action, ActionMap, InputState};
use glam::Vec2;

use super::Camera;

/// First-person camera controller settings.
#[derive(Debug, Clone)]
pub struct ControllerSettings {
    /// Mouse sensitivity (radians per pixel).
    pub sensitivity: f32,
    /// Movement speed (units per second).
    pub move_speed: f32,
    /// Sprint multiplier.
    pub sprint_multiplier: f32,
    /// Invert Y axis.
    pub invert_y: bool,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.002,
            move_speed: 10.0,
            sprint_multiplier: 2.0,
            invert_y: false,
        }
    }
}

/// First-person camera controller.
///
/// Handles mouse look and WASD movement.
pub struct FirstPersonController {
    /// Controller settings.
    pub settings: ControllerSettings,
    /// Current pitch angle (radians).
    pitch: f32,
    /// Current yaw angle (radians).
    yaw: f32,
}

impl FirstPersonController {
    /// Create a new controller with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            settings: ControllerSettings::default(),
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    /// Create a controller with custom settings.
    #[must_use]
    pub fn with_settings(settings: ControllerSettings) -> Self {
        Self {
            settings,
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    /// Get the current pitch angle.
    #[must_use]
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Get the current yaw angle.
    #[must_use]
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Get the rotation quaternion.
    #[must_use]
    pub fn rotation(&self) -> glam::Quat {
        glam::Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }

    /// Update internal rotation state based on input.
    ///
    /// This updates the controller's pitch and yaw based on mouse input.
    /// Use `rotation()` to get the resulting quaternion to apply to a camera.
    ///
    /// # Arguments
    /// * `dt` - Delta time in seconds (unused for mouse look, included for API consistency)
    /// * `input` - Current input state
    pub fn update(&mut self, dt: f32, input: &InputState) {
        let _ = dt; // Mouse look is not time-dependent

        let mouse_delta = input.mouse_delta();
        if mouse_delta.length_squared() == 0.0 {
            return;
        }

        let sensitivity = self.settings.sensitivity;
        let y_mult = if self.settings.invert_y { 1.0 } else { -1.0 };

        self.yaw -= mouse_delta.x * sensitivity;
        self.pitch += mouse_delta.y * sensitivity * y_mult;

        // Clamp pitch to +/- 89 degrees
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        self.pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Update the camera based on input (using action map).
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `input` - Current input state
    /// * `actions` - Action map for key bindings
    /// * `dt` - Delta time in seconds
    pub fn update_with_actions(&mut self, camera: &mut Camera, input: &InputState, actions: &ActionMap, dt: f32) {
        // Mouse look
        let mouse_delta = input.mouse_delta();
        self.process_mouse(camera, mouse_delta);

        // Keyboard movement
        self.process_movement(camera, input, actions, dt);
    }

    /// Update with raw input (no action map).
    pub fn update_raw(&mut self, camera: &mut Camera, input: &InputState, dt: f32) {
        use winit::keyboard::KeyCode;

        let mouse_delta = input.mouse_delta();
        self.process_mouse(camera, mouse_delta);

        // Direct key checks
        let mut movement = glam::Vec3::ZERO;
        if input.is_key_held(KeyCode::KeyW) {
            movement.z -= 1.0;
        }
        if input.is_key_held(KeyCode::KeyS) {
            movement.z += 1.0;
        }
        if input.is_key_held(KeyCode::KeyA) {
            movement.x -= 1.0;
        }
        if input.is_key_held(KeyCode::KeyD) {
            movement.x += 1.0;
        }
        if input.is_key_held(KeyCode::Space) {
            movement.y += 1.0;
        }
        if input.is_key_held(KeyCode::ControlLeft) {
            movement.y -= 1.0;
        }

        if movement.length_squared() > 0.0 {
            let speed = if input.is_key_held(KeyCode::ShiftLeft) {
                self.settings.move_speed * self.settings.sprint_multiplier
            } else {
                self.settings.move_speed
            };

            // Transform movement to world space (only yaw, not pitch)
            let yaw_rotation = glam::Quat::from_rotation_y(self.yaw);
            let world_movement = yaw_rotation * movement.normalize();
            camera.position += world_movement * speed * dt;
        }
    }

    /// Process mouse movement for camera rotation.
    fn process_mouse(&mut self, camera: &mut Camera, delta: Vec2) {
        if delta.length_squared() == 0.0 {
            return;
        }

        let sensitivity = self.settings.sensitivity;
        let y_mult = if self.settings.invert_y { 1.0 } else { -1.0 };

        self.yaw -= delta.x * sensitivity;
        self.pitch += delta.y * sensitivity * y_mult;

        // Clamp pitch to +/- 89 degrees
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        self.pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        // Build rotation from Euler angles
        camera.rotation = self.rotation();
    }

    /// Process keyboard movement.
    fn process_movement(&mut self, camera: &mut Camera, input: &InputState, actions: &ActionMap, dt: f32) {
        let mut movement = glam::Vec3::ZERO;

        if actions.is_action_held(Action::MoveForward, input) {
            movement.z -= 1.0;
        }
        if actions.is_action_held(Action::MoveBack, input) {
            movement.z += 1.0;
        }
        if actions.is_action_held(Action::MoveLeft, input) {
            movement.x -= 1.0;
        }
        if actions.is_action_held(Action::MoveRight, input) {
            movement.x += 1.0;
        }
        if actions.is_action_held(Action::Jump, input) {
            movement.y += 1.0;
        }
        if actions.is_action_held(Action::Crouch, input) {
            movement.y -= 1.0;
        }

        if movement.length_squared() > 0.0 {
            let speed = if actions.is_action_held(Action::Sprint, input) {
                self.settings.move_speed * self.settings.sprint_multiplier
            } else {
                self.settings.move_speed
            };

            // Transform movement to world space (only yaw, not pitch)
            let yaw_rotation = glam::Quat::from_rotation_y(self.yaw);
            let world_movement = yaw_rotation * movement.normalize();
            camera.position += world_movement * speed * dt;
        }
    }
}

impl Default for FirstPersonController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_x_changes_yaw() {
        let mut controller = FirstPersonController::new();
        let mut camera = Camera::new();

        let initial_yaw = controller.yaw();
        controller.process_mouse(&mut camera, Vec2::new(100.0, 0.0));

        assert_ne!(controller.yaw(), initial_yaw);
    }

    #[test]
    fn test_mouse_y_changes_pitch() {
        let mut controller = FirstPersonController::new();
        let mut camera = Camera::new();

        let initial_pitch = controller.pitch();
        controller.process_mouse(&mut camera, Vec2::new(0.0, 100.0));

        assert_ne!(controller.pitch(), initial_pitch);
    }

    #[test]
    fn test_pitch_clamped() {
        let mut controller = FirstPersonController::new();
        let mut camera = Camera::new();

        // Move mouse up a lot
        for _ in 0..100 {
            controller.process_mouse(&mut camera, Vec2::new(0.0, -500.0));
        }

        // Should be clamped to ~89 degrees
        assert!(controller.pitch() < std::f32::consts::FRAC_PI_2);
        assert!(controller.pitch() > 1.5); // About 85+ degrees

        // Move mouse down a lot
        for _ in 0..200 {
            controller.process_mouse(&mut camera, Vec2::new(0.0, 500.0));
        }

        // Should be clamped to ~-89 degrees
        assert!(controller.pitch() > -std::f32::consts::FRAC_PI_2);
        assert!(controller.pitch() < -1.5);
    }

    #[test]
    fn test_rotation_quaternion() {
        let mut controller = FirstPersonController::new();
        let mut camera = Camera::new();

        controller.process_mouse(&mut camera, Vec2::new(50.0, 30.0));

        let rot = controller.rotation();
        assert!(rot.is_normalized());
        assert_eq!(camera.rotation, rot);
    }

    #[test]
    fn test_update_changes_rotation() {
        use engine_core::platform::WindowEvent;

        let mut controller = FirstPersonController::new();
        let mut input = InputState::new();

        // Simulate mouse movement
        input.update(&WindowEvent::MouseMoved { x: 100.0, y: 100.0 });
        input.end_frame();
        input.update(&WindowEvent::MouseMoved { x: 200.0, y: 150.0 });
        input.end_frame();

        let initial_yaw = controller.yaw();
        let initial_pitch = controller.pitch();

        controller.update(0.016, &input);

        // Rotation should have changed
        assert_ne!(controller.yaw(), initial_yaw);
        assert_ne!(controller.pitch(), initial_pitch);
    }

    #[test]
    fn test_update_rotation_is_normalized() {
        use engine_core::platform::WindowEvent;

        let mut controller = FirstPersonController::new();
        let mut input = InputState::new();

        input.update(&WindowEvent::MouseMoved { x: 0.0, y: 0.0 });
        input.end_frame();
        input.update(&WindowEvent::MouseMoved { x: 300.0, y: 200.0 });
        input.end_frame();

        controller.update(0.016, &input);

        let rot = controller.rotation();
        assert!(rot.is_normalized());
    }
}


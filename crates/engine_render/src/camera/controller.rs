//! First-person camera controller.
//!
//! Task 1.14 implementation - placeholder for now.

use engine_core::platform::InputState;
use glam::Vec2;

use super::Camera;

/// First-person camera controller settings.
#[derive(Debug, Clone)]
pub struct ControllerSettings {
    /// Mouse sensitivity.
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

    /// Update the camera based on input.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `input` - Current input state
    /// * `dt` - Delta time in seconds
    #[allow(unused_variables)]
    pub fn update(&mut self, camera: &mut Camera, input: &InputState, dt: f32) {
        // Mouse look
        let mouse_delta = input.mouse_delta();
        self.process_mouse(camera, mouse_delta);

        // Keyboard movement - will be implemented in Task 1.14
        // For now, just a stub
    }

    /// Process mouse movement for camera rotation.
    fn process_mouse(&mut self, camera: &mut Camera, delta: Vec2) {
        let sensitivity = self.settings.sensitivity;
        let y_mult = if self.settings.invert_y { 1.0 } else { -1.0 };

        self.yaw -= delta.x * sensitivity;
        self.pitch += delta.y * sensitivity * y_mult;

        // Clamp pitch to prevent gimbal lock
        self.pitch = self.pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);

        // Build rotation from Euler angles
        camera.rotation = glam::Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);
    }
}

impl Default for FirstPersonController {
    fn default() -> Self {
        Self::new()
    }
}

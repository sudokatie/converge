//! Main character controller implementation.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{
    CharacterInput, ContactState, MovementConfig, MovementEvent, MovementMode, MovementOutput,
    TetherState,
};

/// Linear interpolation helper.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Character controller state and physics simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterController {
    /// Current velocity.
    pub velocity: Vec3,
    /// Current movement mode.
    pub mode: MovementMode,
    /// Whether on ground (walking mode).
    pub on_ground: bool,
    /// Movement configuration.
    pub config: MovementConfig,
    /// Optional tether state.
    pub tether: Option<TetherState>,
    /// Climbing surface normal (if climbing).
    pub climb_normal: Vec3,
    /// Time since last mode change (for cooldowns).
    pub mode_change_cooldown: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            mode: MovementMode::Walking,
            on_ground: false,
            config: MovementConfig::default(),
            tether: None,
            climb_normal: Vec3::ZERO,
            mode_change_cooldown: 0.0,
        }
    }
}

impl CharacterController {
    /// Create a new character controller with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a character controller with custom configuration.
    #[must_use]
    pub fn with_config(config: MovementConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Get the current horizontal speed.
    #[must_use]
    pub fn horizontal_speed(&self) -> f32 {
        Vec3::new(self.velocity.x, 0.0, self.velocity.z).length()
    }

    /// Get the current total speed.
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.velocity.length()
    }

    /// Set the movement mode directly.
    pub fn set_mode(&mut self, mode: MovementMode) {
        self.mode = mode;
        self.mode_change_cooldown = 0.2;
    }

    /// Attach a tether to an anchor point.
    pub fn attach_tether(&mut self, anchor: Vec3, character_pos: Vec3) {
        let tether = TetherState::from_character_position(character_pos, anchor);
        self.tether = Some(tether);
        self.mode = MovementMode::Tethered;
    }

    /// Detach the current tether.
    pub fn detach_tether(&mut self) {
        self.tether = None;
        if self.mode == MovementMode::Tethered {
            self.mode = MovementMode::Walking;
        }
    }

    /// Update the character controller for one tick.
    ///
    /// This is the main entry point that dispatches to mode-specific physics.
    /// Returns deterministic output including new position, velocity, and events.
    #[must_use]
    pub fn update(
        &mut self,
        position: Vec3,
        input: &CharacterInput,
        contact: &ContactState,
        gravity: Vec3,
        dt: f32,
    ) -> MovementOutput {
        self.mode_change_cooldown = (self.mode_change_cooldown - dt).max(0.0);

        let mut output = match self.mode {
            MovementMode::Walking => self.update_walking(position, input, contact, gravity, dt),
            MovementMode::Swimming => self.update_swimming(position, input, contact, gravity, dt),
            MovementMode::Climbing => self.update_climbing(position, input, contact, dt),
            MovementMode::ZeroG => self.update_zero_g(position, input, contact, dt),
            MovementMode::Tethered => self.update_tethered(position, input, contact, gravity, dt),
        };

        self.check_mode_transitions(input, contact, &mut output);
        output
    }

    /// Update walking mode physics.
    fn update_walking(
        &mut self,
        position: Vec3,
        input: &CharacterInput,
        contact: &ContactState,
        gravity: Vec3,
        dt: f32,
    ) -> MovementOutput {
        let cfg = &self.config.walking;
        let mut events = Vec::new();

        let was_on_ground = self.on_ground;
        self.on_ground = contact.on_ground();

        if !was_on_ground && self.on_ground {
            events.push(MovementEvent::Landed {
                impact_velocity: -self.velocity.y,
            });
        }

        self.velocity += gravity * cfg.gravity_scale * dt;

        let speed = if input.sprint() {
            cfg.move_speed * cfg.sprint_multiplier
        } else {
            cfg.move_speed
        };

        let target_velocity = Vec3::new(input.movement.x * speed, 0.0, input.movement.z * speed);

        let control = if self.on_ground {
            cfg.ground_friction
        } else {
            cfg.air_friction * cfg.air_control
        };

        self.velocity.x = lerp(self.velocity.x, target_velocity.x, control * dt);
        self.velocity.z = lerp(self.velocity.z, target_velocity.z, control * dt);

        if input.jump() && self.on_ground {
            self.velocity.y = cfg.jump_impulse;
            self.on_ground = false;
            events.push(MovementEvent::Jumped);
        }

        if self.on_ground && self.velocity.y < 0.0 {
            self.velocity.y = 0.0;
        }

        let new_position = position + self.velocity * dt;

        let mut output = MovementOutput::new(new_position, self.velocity, self.mode);
        output.on_ground = self.on_ground;
        output.events = events;
        output
    }

    /// Update swimming mode physics.
    fn update_swimming(
        &mut self,
        position: Vec3,
        input: &CharacterInput,
        contact: &ContactState,
        gravity: Vec3,
        dt: f32,
    ) -> MovementOutput {
        let cfg = &self.config.swimming;
        let mut events = Vec::new();

        let speed = if input.sprint() {
            cfg.swim_speed * cfg.sprint_multiplier
        } else {
            cfg.swim_speed
        };

        let mut target_velocity = input.movement * speed;

        if input.jump() {
            target_velocity.y += speed * cfg.vertical_control;
        }
        if input.crouch() {
            target_velocity.y -= speed * cfg.vertical_control;
        }

        let buoyancy_force = Vec3::new(0.0, cfg.buoyancy, 0.0);
        let depth_factor = contact.liquid_depth.clamp(0.0, 1.0);
        self.velocity += buoyancy_force * depth_factor * dt;

        let gravity_in_water = gravity * 0.2 * (1.0 - depth_factor);
        self.velocity += gravity_in_water * dt;

        self.velocity = Vec3::new(
            lerp(self.velocity.x, target_velocity.x, cfg.drag * dt),
            lerp(self.velocity.y, target_velocity.y, cfg.drag * dt),
            lerp(self.velocity.z, target_velocity.z, cfg.drag * dt),
        );

        let new_position = position + self.velocity * dt;

        if !contact.in_liquid() {
            events.push(MovementEvent::ExitedWater);
        }

        let mut output = MovementOutput::new(new_position, self.velocity, self.mode);
        output.events = events;
        output
    }

    /// Update climbing mode physics.
    fn update_climbing(
        &mut self,
        position: Vec3,
        input: &CharacterInput,
        contact: &ContactState,
        dt: f32,
    ) -> MovementOutput {
        let cfg = &self.config.climbing;
        let mut events = Vec::new();

        if !contact.on_climbable() {
            events.push(MovementEvent::ReleasedSurface);
            self.velocity = Vec3::ZERO;
            let output = MovementOutput::new(position, self.velocity, self.mode);
            return output;
        }

        let wall_normal = if contact.wall_normal.length_squared() > 0.0001 {
            contact.wall_normal
        } else {
            self.climb_normal
        };
        self.climb_normal = wall_normal;

        let up = Vec3::Y;
        let right = up.cross(wall_normal).normalize_or_zero();

        let target_velocity = Vec3::new(
            right.x * input.movement.x * cfg.strafe_speed
                + up.x * input.movement.y * cfg.climb_speed,
            right.y * input.movement.x * cfg.strafe_speed
                + up.y * input.movement.y * cfg.climb_speed,
            right.z * input.movement.x * cfg.strafe_speed
                + up.z * input.movement.y * cfg.climb_speed,
        );

        if input.has_movement() {
            self.velocity = Vec3::new(
                lerp(self.velocity.x, target_velocity.x, cfg.acceleration * dt),
                lerp(self.velocity.y, target_velocity.y, cfg.acceleration * dt),
                lerp(self.velocity.z, target_velocity.z, cfg.acceleration * dt),
            );
        } else {
            self.velocity = Vec3::new(
                lerp(self.velocity.x, 0.0, cfg.friction * dt),
                lerp(self.velocity.y, 0.0, cfg.friction * dt),
                lerp(self.velocity.z, 0.0, cfg.friction * dt),
            );
        }

        if input.secondary_action() {
            events.push(MovementEvent::ReleasedSurface);
        }

        let new_position = position + self.velocity * dt;

        let mut output = MovementOutput::new(new_position, self.velocity, self.mode);
        output.events = events;
        output
    }

    /// Update zero-G mode physics.
    fn update_zero_g(
        &mut self,
        position: Vec3,
        input: &CharacterInput,
        contact: &ContactState,
        dt: f32,
    ) -> MovementOutput {
        let cfg = &self.config.zero_g;
        let mut events = Vec::new();

        if input.primary_action() && (contact.on_ground() || contact.on_wall()) {
            let push_dir = if input.has_movement() {
                input.movement
            } else if contact.on_ground() {
                Vec3::Y
            } else {
                contact.wall_normal
            };

            self.velocity += push_dir.normalize_or_zero() * cfg.push_speed;
            events.push(MovementEvent::PushedOff {
                direction: push_dir.normalize_or_zero(),
            });
        }

        self.velocity *= 1.0 - cfg.damping * dt;

        let speed = self.velocity.length();
        if speed > cfg.max_velocity {
            self.velocity = self.velocity.normalize_or_zero() * cfg.max_velocity;
        }

        let new_position = position + self.velocity * dt;

        let mut output = MovementOutput::new(new_position, self.velocity, self.mode);
        output.events = events;
        output
    }

    /// Update tethered mode physics.
    fn update_tethered(
        &mut self,
        position: Vec3,
        input: &CharacterInput,
        _contact: &ContactState,
        gravity: Vec3,
        dt: f32,
    ) -> MovementOutput {
        let cfg = &self.config.tethered;
        let mut events = Vec::new();

        let Some(tether) = &mut self.tether else {
            return MovementOutput::new(position, self.velocity, self.mode);
        };

        self.velocity += gravity * cfg.gravity_scale * dt;

        if input.jump() {
            tether.retract(cfg.reel_speed * dt, cfg.min_length);
        }
        if input.crouch() {
            tether.extend(cfg.reel_speed * dt, cfg.max_length);
        }

        let to_anchor = tether.anchor - position;
        let tangent_speed = 2.0;
        let tangent = Vec3::new(-to_anchor.z, 0.0, to_anchor.x).normalize_or_zero();
        self.velocity += tangent * input.movement.x * tangent_speed * dt;

        self.velocity *= 1.0 - cfg.swing_damping * dt;

        self.velocity = tether.constrain_velocity(position, self.velocity, cfg.stiffness, dt);

        let mut new_position = position + self.velocity * dt;
        let was_taut = tether.taut;
        tether.taut = tether.exceeds_length(new_position);

        if tether.taut {
            new_position = tether.constrain_position(new_position);
            if !was_taut {
                events.push(MovementEvent::TetherMaxLength);
            }
        }

        if input.secondary_action() {
            events.push(MovementEvent::TetherDetached);
        }

        let mut output = MovementOutput::new(new_position, self.velocity, self.mode);
        output.events = events;
        output
    }

    /// Check for automatic mode transitions based on contact state.
    fn check_mode_transitions(
        &mut self,
        input: &CharacterInput,
        contact: &ContactState,
        output: &mut MovementOutput,
    ) {
        if self.mode_change_cooldown > 0.0 {
            return;
        }

        let old_mode = self.mode;

        match self.mode {
            MovementMode::Walking => {
                if contact.should_swim() {
                    self.mode = MovementMode::Swimming;
                    output.events.push(MovementEvent::EnteredWater);
                } else if contact.on_climbable() && input.primary_action() {
                    self.mode = MovementMode::Climbing;
                    self.climb_normal = contact.wall_normal;
                    output.events.push(MovementEvent::GrabbedSurface {
                        normal: contact.wall_normal,
                    });
                }
            }
            MovementMode::Swimming => {
                if !contact.in_liquid() {
                    self.mode = MovementMode::Walking;
                    output.events.push(MovementEvent::ExitedWater);
                }
            }
            MovementMode::Climbing => {
                if !contact.on_climbable() || input.secondary_action() {
                    self.mode = MovementMode::Walking;
                }
            }
            MovementMode::ZeroG => {
                if contact.environment != super::contact::EnvironmentType::ZeroGravity {
                    self.mode = MovementMode::Walking;
                }
            }
            MovementMode::Tethered => {
                if input.secondary_action() || self.tether.is_none() {
                    self.tether = None;
                    self.mode = MovementMode::Walking;
                }
            }
        }

        if self.mode != old_mode {
            output.mode = self.mode;
            output.events.push(MovementEvent::ModeChanged {
                from: old_mode,
                to: self.mode,
            });
            self.mode_change_cooldown = 0.2;
        }
    }

    /// Force mode transition to zero-G (for entering zero-G zones).
    pub fn enter_zero_g(&mut self) {
        if self.mode != MovementMode::Tethered {
            self.mode = MovementMode::ZeroG;
            self.mode_change_cooldown = 0.2;
        }
    }

    /// Force mode transition out of zero-G.
    pub fn exit_zero_g(&mut self) {
        if self.mode == MovementMode::ZeroG {
            self.mode = MovementMode::Walking;
            self.mode_change_cooldown = 0.2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.016;
    const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

    #[test]
    fn default_controller() {
        let controller = CharacterController::new();
        assert_eq!(controller.mode, MovementMode::Walking);
        assert_eq!(controller.velocity, Vec3::ZERO);
        assert!(!controller.on_ground);
    }

    #[test]
    fn walking_gravity() {
        let mut controller = CharacterController::new();
        let input = CharacterInput::new();
        let contact = ContactState::new();

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert!(output.velocity.y < 0.0);
        assert!(output.position.y < 0.0);
    }

    #[test]
    fn walking_ground_stops_fall() {
        let mut controller = CharacterController::new();
        controller.velocity = Vec3::new(0.0, -10.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::grounded();

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert!(output.on_ground);
        assert!(output.velocity.y >= 0.0);
    }

    #[test]
    fn walking_jump() {
        let mut controller = CharacterController::new();
        controller.on_ground = true;

        let input = CharacterInput::new().with_jump(true);
        let contact = ContactState::grounded();

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert!(output.jumped());
        assert!(output.velocity.y > 0.0);
    }

    #[test]
    fn walking_no_double_jump() {
        let mut controller = CharacterController::new();
        controller.on_ground = false;
        controller.velocity.y = 5.0;

        let input = CharacterInput::new().with_jump(true);
        let contact = ContactState::new();

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert!(!output.jumped());
    }

    #[test]
    fn walking_movement() {
        let mut controller = CharacterController::new();
        controller.on_ground = true;

        let input = CharacterInput::horizontal(0.0, 1.0);
        let contact = ContactState::grounded();

        for _ in 0..60 {
            let _ = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);
        }

        assert!(controller.velocity.z > 0.0);
    }

    #[test]
    fn walking_sprint_faster() {
        let mut normal = CharacterController::new();
        let mut sprinting = CharacterController::new();
        normal.on_ground = true;
        sprinting.on_ground = true;

        let normal_input = CharacterInput::horizontal(0.0, 1.0);
        let sprint_input = CharacterInput::horizontal(0.0, 1.0).with_sprint(true);
        let contact = ContactState::grounded();

        for _ in 0..60 {
            let _ = normal.update(Vec3::ZERO, &normal_input, &contact, GRAVITY, DT);
            let _ = sprinting.update(Vec3::ZERO, &sprint_input, &contact, GRAVITY, DT);
        }

        assert!(sprinting.horizontal_speed() > normal.horizontal_speed());
    }

    #[test]
    fn swimming_buoyancy() {
        let mut controller = CharacterController::with_config(MovementConfig::default());
        controller.mode = MovementMode::Swimming;
        controller.velocity = Vec3::new(0.0, -5.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::swimming(1.0);

        for _ in 0..60 {
            let _ = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);
        }

        assert!(controller.velocity.y > -5.0);
    }

    #[test]
    fn swimming_drag() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::Swimming;
        controller.velocity = Vec3::new(10.0, 0.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::swimming(1.0);

        let initial_speed = controller.horizontal_speed();
        for _ in 0..60 {
            let _ = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);
        }

        assert!(controller.horizontal_speed() < initial_speed);
    }

    #[test]
    fn swimming_vertical_control() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::Swimming;

        let input = CharacterInput::new().with_jump(true);
        let contact = ContactState::swimming(1.0);

        for _ in 0..30 {
            let _ = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);
        }

        assert!(controller.velocity.y > 0.0);
    }

    #[test]
    fn climbing_vertical() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::Climbing;

        let input = CharacterInput::full_3d(0.0, 1.0, 0.0);
        let contact = ContactState::climbing(Vec3::NEG_Z);

        for _ in 0..30 {
            let _ = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);
        }

        assert!(controller.velocity.y > 0.0);
    }

    #[test]
    fn climbing_no_gravity() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::Climbing;

        let input = CharacterInput::new();
        let contact = ContactState::climbing(Vec3::NEG_Z);

        let output = controller.update(Vec3::new(0.0, 10.0, 0.0), &input, &contact, GRAVITY, DT);

        assert!(output.velocity.y.abs() < 0.1);
    }

    #[test]
    fn zero_g_push_off() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::ZeroG;

        let input = CharacterInput::new().with_primary_action(true);
        let contact = ContactState::grounded();

        let output = controller.update(Vec3::ZERO, &input, &contact, Vec3::ZERO, DT);

        assert!(output.velocity.length() > 0.0);
        assert!(output.has_event(|e| matches!(e, MovementEvent::PushedOff { .. })));
    }

    #[test]
    fn zero_g_damping() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::ZeroG;
        controller.velocity = Vec3::new(10.0, 0.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::zero_g();

        let initial_speed = controller.speed();
        for _ in 0..60 {
            let _ = controller.update(Vec3::ZERO, &input, &contact, Vec3::ZERO, DT);
        }

        assert!(controller.speed() < initial_speed);
    }

    #[test]
    fn zero_g_max_velocity() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::ZeroG;
        controller.velocity = Vec3::new(100.0, 0.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::zero_g();

        let _ = controller.update(Vec3::ZERO, &input, &contact, Vec3::ZERO, DT);

        assert!(controller.speed() <= controller.config.zero_g.max_velocity);
    }

    #[test]
    fn tether_attach() {
        let mut controller = CharacterController::new();
        controller.attach_tether(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);

        assert!(controller.tether.is_some());
        assert_eq!(controller.mode, MovementMode::Tethered);

        let tether = controller.tether.as_ref().unwrap();
        assert!((tether.length - 10.0).abs() < 0.001);
    }

    #[test]
    fn tether_length_enforcement() {
        let mut controller = CharacterController::new();
        controller.attach_tether(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        controller.velocity = Vec3::new(10.0, 0.0, 0.0);

        let input = CharacterInput::new();
        let contact = ContactState::new();

        let output = controller.update(Vec3::new(5.0, 0.0, 0.0), &input, &contact, GRAVITY, DT);

        let distance = output.position.length();
        let tether = controller.tether.as_ref().unwrap();
        assert!(distance <= tether.length + 0.1);
    }

    #[test]
    fn tether_detach() {
        let mut controller = CharacterController::new();
        controller.attach_tether(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        controller.detach_tether();

        assert!(controller.tether.is_none());
        assert_eq!(controller.mode, MovementMode::Walking);
    }

    #[test]
    fn mode_transition_to_swimming() {
        let mut controller = CharacterController::new();

        let input = CharacterInput::new();
        let contact = ContactState::swimming(0.8);

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert_eq!(controller.mode, MovementMode::Swimming);
        assert!(output.mode_changed());
    }

    #[test]
    fn mode_transition_from_swimming() {
        let mut controller = CharacterController::new();
        controller.mode = MovementMode::Swimming;
        controller.mode_change_cooldown = 0.0;

        let input = CharacterInput::new();
        let contact = ContactState::grounded();

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert_eq!(controller.mode, MovementMode::Walking);
        assert!(output.mode_changed());
    }

    #[test]
    fn gravity_vector_application() {
        let mut controller = CharacterController::new();

        let custom_gravity = Vec3::new(5.0, -15.0, 0.0);
        let input = CharacterInput::new();
        let contact = ContactState::new();

        let output = controller.update(Vec3::ZERO, &input, &contact, custom_gravity, DT);

        assert!(output.velocity.x > 0.0);
        assert!(output.velocity.y < 0.0);
    }

    #[test]
    fn landing_event() {
        let mut controller = CharacterController::new();
        controller.velocity = Vec3::new(0.0, -10.0, 0.0);
        controller.on_ground = false;

        let input = CharacterInput::new();
        let contact = ContactState::grounded();

        let output = controller.update(Vec3::ZERO, &input, &contact, GRAVITY, DT);

        assert!(output.landed());
    }
}

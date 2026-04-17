//! Core ECS components for game entities.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use engine_ai::{BehaviorTree, Blackboard};

/// Transform component - position and rotation in world space.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    /// World position.
    pub position: Vec3,
    /// World rotation.
    pub rotation: Quat,
}

impl Transform {
    /// Create a transform at the given position with no rotation.
    #[must_use]
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
        }
    }

    /// Create a transform with position and rotation.
    #[must_use]
    pub fn new(position: Vec3, rotation: Quat) -> Self {
        Self { position, rotation }
    }

    /// Get the forward direction (-Z in local space).
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation * -Vec3::Z
    }

    /// Get the right direction (+X in local space).
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Get the up direction (+Y in local space).
    #[must_use]
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

/// Velocity component - linear and angular velocity.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Velocity {
    /// Linear velocity (units per second).
    pub linear: Vec3,
    /// Angular velocity (radians per second around each axis).
    pub angular: Vec3,
}

impl Velocity {
    /// Create a velocity with only linear component.
    #[must_use]
    pub fn linear(velocity: Vec3) -> Self {
        Self {
            linear: velocity,
            angular: Vec3::ZERO,
        }
    }

    /// Create zero velocity.
    #[must_use]
    pub fn zero() -> Self {
        Self::default()
    }

    /// Get the speed (magnitude of linear velocity).
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.linear.length()
    }
}

/// Shape types for colliders.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ColliderShape {
    /// Axis-aligned box with half extents.
    Box { half_extents: Vec3 },
    /// Sphere with radius.
    Sphere { radius: f32 },
    /// Capsule aligned to Y axis.
    Capsule { height: f32, radius: f32 },
}

impl ColliderShape {
    /// Create a box collider.
    #[must_use]
    pub fn cuboid(half_x: f32, half_y: f32, half_z: f32) -> Self {
        Self::Box {
            half_extents: Vec3::new(half_x, half_y, half_z),
        }
    }

    /// Create a sphere collider.
    #[must_use]
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    /// Create a capsule collider (Y-aligned).
    #[must_use]
    pub fn capsule(height: f32, radius: f32) -> Self {
        Self::Capsule { height, radius }
    }
}

/// Collider component for physics interactions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Collider {
    /// Collision shape.
    pub shape: ColliderShape,
    /// Offset from entity position.
    pub offset: Vec3,
}

impl Collider {
    /// Create a collider with no offset.
    #[must_use]
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            offset: Vec3::ZERO,
        }
    }

    /// Create a collider with offset.
    #[must_use]
    pub fn with_offset(shape: ColliderShape, offset: Vec3) -> Self {
        Self { shape, offset }
    }

    /// Create a player-sized capsule collider (1.8m tall, 0.3m radius).
    #[must_use]
    pub fn player() -> Self {
        Self::new(ColliderShape::capsule(1.8, 0.3))
    }
}

/// Player component - marks entity as player-controlled.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Player {
    /// Camera pitch angle (radians, -PI/2 to PI/2).
    pub pitch: f32,
    /// Camera yaw angle (radians).
    pub yaw: f32,
}

impl Player {
    /// Create a new player with default orientation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the camera rotation quaternion.
    #[must_use]
    pub fn camera_rotation(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }

    /// Get the look direction.
    #[must_use]
    pub fn look_direction(&self) -> Vec3 {
        self.camera_rotation() * -Vec3::Z
    }

    /// Add to pitch, clamping to valid range.
    pub fn add_pitch(&mut self, delta: f32) {
        const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = (self.pitch + delta).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Add to yaw, wrapping around.
    pub fn add_yaw(&mut self, delta: f32) {
        self.yaw = (self.yaw + delta) % std::f32::consts::TAU;
    }
}

/// Network identifier component for multiplayer synchronization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkId {
    /// Unique network identifier.
    pub id: u64,
}

impl NetworkId {
    /// Create a new network ID.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}

/// Controller type for entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControllerKind {
    /// Controlled by local player input.
    Player,
    /// Controlled by AI behavior tree.
    AI,
    /// Controlled by network synchronization.
    Network,
}

/// Controller component - determines how an entity is controlled.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Controller {
    /// The type of controller.
    pub kind: ControllerKind,
}

impl Controller {
    /// Create a player controller.
    #[must_use]
    pub fn player() -> Self {
        Self {
            kind: ControllerKind::Player,
        }
    }

    /// Create an AI controller.
    #[must_use]
    pub fn ai() -> Self {
        Self {
            kind: ControllerKind::AI,
        }
    }

    /// Create a network controller.
    #[must_use]
    pub fn network() -> Self {
        Self {
            kind: ControllerKind::Network,
        }
    }

    /// Check if this is a player controller.
    #[must_use]
    pub fn is_player(&self) -> bool {
        self.kind == ControllerKind::Player
    }

    /// Check if this is an AI controller.
    #[must_use]
    pub fn is_ai(&self) -> bool {
        self.kind == ControllerKind::AI
    }

    /// Check if this is a network controller.
    #[must_use]
    pub fn is_network(&self) -> bool {
        self.kind == ControllerKind::Network
    }
}

/// AI brain component for entities controlled by behavior trees.
///
/// Contains the behavior tree and blackboard for AI decision making.
pub struct AIBrain {
    /// Behavior tree for AI decision making.
    pub behavior: BehaviorTree,
    /// Shared blackboard for storing AI state.
    pub blackboard: Blackboard,
}

impl AIBrain {
    /// Create a new AI brain with the given behavior tree.
    pub fn new(behavior: BehaviorTree) -> Self {
        Self {
            blackboard: Blackboard::new(),
            behavior,
        }
    }

    /// Create an AI brain with existing blackboard.
    pub fn with_blackboard(behavior: BehaviorTree, blackboard: Blackboard) -> Self {
        Self {
            behavior,
            blackboard,
        }
    }

    /// Tick the behavior tree.
    pub fn tick(&mut self) -> engine_ai::NodeStatus {
        self.behavior.tick()
    }

    /// Reset the behavior tree.
    pub fn reset(&mut self) {
        self.behavior.reset();
    }
}

impl std::fmt::Debug for AIBrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AIBrain")
            .field("behavior", &"BehaviorTree")
            .field("blackboard", &self.blackboard)
            .finish()
    }
}

/// Marker component for entities pending destruction.
///
/// Entities with this component will be destroyed at the end of the frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct PendingDestroy;

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn test_transform_directions() {
        let transform = Transform::default();
        assert!((transform.forward() - (-Vec3::Z)).length() < 0.001);
        assert!((transform.right() - Vec3::X).length() < 0.001);
        assert!((transform.up() - Vec3::Y).length() < 0.001);
    }

    #[test]
    fn test_transform_rotated() {
        let transform = Transform::new(Vec3::ZERO, Quat::from_rotation_y(PI));
        assert!((transform.forward() - Vec3::Z).length() < 0.001);
    }

    #[test]
    fn test_velocity_speed() {
        let velocity = Velocity::linear(Vec3::new(3.0, 4.0, 0.0));
        assert!((velocity.speed() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_player_pitch_clamp() {
        let mut player = Player::new();
        player.add_pitch(PI); // Try to look straight up and beyond
        assert!(player.pitch < FRAC_PI_2);
        assert!(player.pitch > FRAC_PI_2 - 0.1);
    }

    #[test]
    fn test_player_yaw_wrap() {
        let mut player = Player::new();
        player.add_yaw(3.0 * PI);
        assert!(player.yaw >= 0.0);
        assert!(player.yaw < std::f32::consts::TAU);
    }

    #[test]
    fn test_collider_player() {
        let collider = Collider::player();
        match collider.shape {
            ColliderShape::Capsule { height, radius } => {
                assert!((height - 1.8).abs() < 0.001);
                assert!((radius - 0.3).abs() < 0.001);
            }
            _ => panic!("Expected capsule shape"),
        }
    }

    #[test]
    fn test_network_id() {
        let id = NetworkId::new(12345);
        assert_eq!(id.id, 12345);
    }

    #[test]
    fn test_controller_types() {
        let player = Controller::player();
        assert!(player.is_player());
        assert!(!player.is_ai());
        assert!(!player.is_network());

        let ai = Controller::ai();
        assert!(!ai.is_player());
        assert!(ai.is_ai());
        assert!(!ai.is_network());

        let network = Controller::network();
        assert!(!network.is_player());
        assert!(!network.is_ai());
        assert!(network.is_network());
    }

    #[test]
    fn test_controller_kind_equality() {
        assert_eq!(ControllerKind::Player, ControllerKind::Player);
        assert_ne!(ControllerKind::Player, ControllerKind::AI);
    }
}

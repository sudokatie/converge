//! Entity serialization for save/load.
//!
//! Implements spec 6.1.3: serialization with serde for persistence/network.
//! Saves entity components to JSON, restores them on load.

use std::collections::HashMap;
use std::path::Path;

use glam::{Quat, Vec3};
use hecs::World;
use serde::{Deserialize, Serialize};

use crate::ecs::{Collider, ColliderShape, Controller, ControllerKind, NetworkId, Transform, Velocity};
use crate::entities::CreatureKind;
use crate::survival::Health;

/// Serialized entity data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEntity {
    /// Entity type for reconstruction.
    pub entity_type: SerializedEntityType,
    /// Position.
    pub position: [f32; 3],
    /// Rotation (quaternion as [x, y, z, w]).
    pub rotation: [f32; 4],
    /// Linear velocity.
    pub velocity: [f32; 3],
    /// Health current.
    pub health_current: f32,
    /// Health max.
    pub health_max: f32,
    /// Creature kind (if creature).
    pub creature_kind: Option<CreatureKind>,
    /// Controller kind (if player).
    pub controller_kind: Option<ControllerKind>,
    /// Network ID (if networked).
    pub network_id: Option<u64>,
    /// Collider shape (if collidable).
    pub collider_shape: Option<ColliderShapeSerde>,
}

/// Serializable collider shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderShapeSerde {
    /// Axis-aligned bounding box with half-extents.
    Box { half_x: f32, half_y: f32, half_z: f32 },
    /// Sphere with radius.
    Sphere { radius: f32 },
    /// Capsule with height and radius.
    Capsule { height: f32, radius: f32 },
}

impl From<&ColliderShape> for ColliderShapeSerde {
    fn from(shape: &ColliderShape) -> Self {
        match shape {
            ColliderShape::Box { half_extents } => ColliderShapeSerde::Box {
                half_x: half_extents.x,
                half_y: half_extents.y,
                half_z: half_extents.z,
            },
            ColliderShape::Sphere { radius } => ColliderShapeSerde::Sphere { radius: *radius },
            ColliderShape::Capsule { height, radius } => ColliderShapeSerde::Capsule { height: *height, radius: *radius },
        }
    }
}

/// Entity type for deserialization routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SerializedEntityType {
    /// Player entity.
    Player,
    /// Creature entity.
    Creature,
    /// Generic entity.
    Other(String),
}

/// Save data for all entities in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySaveData {
    /// Save format version.
    pub version: u32,
    /// List of serialized entities.
    pub entities: Vec<SerializedEntity>,
}

impl EntitySaveData {
    /// Current save format version.
    pub const VERSION: u32 = 1;

    /// Create new empty save data.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            entities: Vec::new(),
        }
    }

    /// Serialize all entities from the ECS world.
    #[must_use]
    pub fn from_world(world: &World) -> Self {
        let mut data = Self::new();

        for (id, (transform, velocity)) in world.query::<(&Transform, &Velocity)>().iter() {
            let health_data = world.get::<&Health>(id).ok().map(|h| (h.current(), h.max()));
            let creature: Option<CreatureKind> = world.get::<&CreatureKind>(id).ok().map(|c| *c);
            let controller = world.get::<&Controller>(id).ok();
            let net_id = world.get::<&NetworkId>(id).ok();
            let collider = world.get::<&Collider>(id).ok();

            let entity_type = if controller.is_some() {
                SerializedEntityType::Player
            } else if creature.is_some() {
                SerializedEntityType::Creature
            } else {
                SerializedEntityType::Other("unknown".to_string())
            };

            let (health_current, health_max) = health_data.unwrap_or((20.0, 20.0));

            data.entities.push(SerializedEntity {
                entity_type,
                position: [transform.position.x, transform.position.y, transform.position.z],
                rotation: [transform.rotation.x, transform.rotation.y, transform.rotation.z, transform.rotation.w],
                velocity: [velocity.linear.x, velocity.linear.y, velocity.linear.z],
                health_current,
                health_max,
                creature_kind: creature,
                controller_kind: controller.map(|c| c.kind),
                network_id: net_id.map(|n| n.id),
                collider_shape: collider.map(|c| ColliderShapeSerde::from(&c.shape)),
            });
        }

        data
    }

    /// Save to a JSON file.
    ///
    /// # Errors
    ///
    /// Returns error if serialization or file write fails.
    pub fn save_to_file(&self, path: &Path) -> Result<(), EntitySaveError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| EntitySaveError::SerializationFailed(e.to_string()))?;
        std::fs::write(path, json)
            .map_err(|e| EntitySaveError::IoError(e.to_string()))
    }

    /// Load from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns error if file read or deserialization fails.
    pub fn load_from_file(path: &Path) -> Result<Self, EntitySaveError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| EntitySaveError::IoError(e.to_string()))?;
        let data: Self = serde_json::from_str(&content)
            .map_err(|e| EntitySaveError::DeserializationFailed(e.to_string()))?;

        if data.version != Self::VERSION {
            return Err(EntitySaveError::VersionMismatch {
                expected: Self::VERSION,
                found: data.version,
            });
        }

        Ok(data)
    }

    /// Number of entities in save data.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Check if save data is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

impl Default for EntitySaveData {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from entity save/load operations.
#[derive(Debug, thiserror::Error)]
pub enum EntitySaveError {
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World as HecsWorld;

    fn make_test_world() -> HecsWorld {
        let mut world = HecsWorld::new();

        // Spawn a player
        world.spawn((
            Transform {
                position: Vec3::new(100.0, 64.0, 200.0),
                rotation: Quat::IDENTITY,
            },
            Velocity::default(),
            Health::new(20.0),
            Controller { kind: ControllerKind::Player },
            NetworkId { id: 42 },
        ));

        // Spawn a creature
        world.spawn((
            Transform {
                position: Vec3::new(50.0, 64.0, 50.0),
                rotation: Quat::from_rotation_y(1.5),
            },
            Velocity {
                linear: Vec3::new(1.0, 0.0, 0.0),
                angular: Vec3::ZERO,
            },
            Health::new(10.0),
            CreatureKind::Pig,
        ));

        world
    }

    #[test]
    fn test_serialize_world() {
        let world = make_test_world();
        let data = EntitySaveData::from_world(&world);

        assert_eq!(data.entities.len(), 2);
        assert_eq!(data.version, EntitySaveData::VERSION);
    }

    #[test]
    fn test_player_serialization() {
        let world = make_test_world();
        let data = EntitySaveData::from_world(&world);

        let player = data.entities.iter()
            .find(|e| e.entity_type == SerializedEntityType::Player)
            .unwrap();

        assert_eq!(player.position, [100.0, 64.0, 200.0]);
        assert_eq!(player.health_current, 20.0);
        assert_eq!(player.network_id, Some(42));
        assert_eq!(player.controller_kind, Some(ControllerKind::Player));
    }

    #[test]
    fn test_creature_serialization() {
        let world = make_test_world();
        let data = EntitySaveData::from_world(&world);

        let creature = data.entities.iter()
            .find(|e| e.entity_type == SerializedEntityType::Creature)
            .unwrap();

        assert_eq!(creature.position, [50.0, 64.0, 50.0]);
        assert_eq!(creature.creature_kind, Some(CreatureKind::Pig));
        assert_eq!(creature.velocity, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_save_and_load_file() {
        let dir = std::env::temp_dir().join("lattice_entity_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("entities.json");

        let world = make_test_world();
        let data = EntitySaveData::from_world(&world);

        data.save_to_file(&path).unwrap();
        let loaded = EntitySaveData::load_from_file(&path).unwrap();

        assert_eq!(loaded.entities.len(), 2);
        assert_eq!(loaded.version, EntitySaveData::VERSION);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_version_mismatch() {
        let dir = std::env::temp_dir().join("lattice_version_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bad_version.json");

        let bad_data = r#"{"version": 999, "entities": []}"#;
        std::fs::write(&path, bad_data).unwrap();

        let result = EntitySaveData::load_from_file(&path);
        assert!(matches!(result, Err(EntitySaveError::VersionMismatch { .. })));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_empty_save_data() {
        let data = EntitySaveData::new();
        assert!(data.is_empty());
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_collider_shape_serde() {
        let box_shape = ColliderShape::Box { half_extents: glam::Vec3::new(0.3, 0.9, 0.3) };
        let serde_shape = ColliderShapeSerde::from(&box_shape);
        match serde_shape {
            ColliderShapeSerde::Box { half_x, half_y, half_z } => {
                assert!((half_x - 0.3).abs() < 0.001);
                assert!((half_y - 0.9).abs() < 0.001);
            }
            _ => panic!("Expected box shape"),
        }
    }

    #[test]
    fn test_entity_type_equality() {
        assert_eq!(SerializedEntityType::Player, SerializedEntityType::Player);
        assert_ne!(SerializedEntityType::Player, SerializedEntityType::Creature);
    }

    #[test]
    fn test_json_roundtrip() {
        let world = make_test_world();
        let data = EntitySaveData::from_world(&world);

        let json = serde_json::to_string(&data).unwrap();
        let parsed: EntitySaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.entities.len(), data.entities.len());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = EntitySaveData::load_from_file(Path::new("/nonexistent/path.json"));
        assert!(result.is_err());
    }
}

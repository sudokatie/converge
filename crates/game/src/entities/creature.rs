//! Creature entities for the game world.
//!
//! Creatures include passive animals (pigs, cows, sheep) and hostile mobs (zombies, skeletons).

use glam::{Quat, Vec3};
use hecs::{Entity, World};
use serde::{Deserialize, Serialize};

use crate::ecs::{Collider, ColliderShape, Transform, Velocity};
use crate::survival::Health;

/// Type of creature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreatureKind {
    // Passive animals
    Pig,
    Cow,
    Sheep,
    Chicken,

    // Hostile creatures
    Zombie,
    Skeleton,
    Spider,
    Creeper,
}

impl CreatureKind {
    /// Check if this creature is hostile.
    #[must_use]
    pub fn is_hostile(self) -> bool {
        matches!(
            self,
            CreatureKind::Zombie
                | CreatureKind::Skeleton
                | CreatureKind::Spider
                | CreatureKind::Creeper
        )
    }

    /// Check if this creature is passive.
    #[must_use]
    pub fn is_passive(self) -> bool {
        !self.is_hostile()
    }

    /// Get the maximum health for this creature type.
    #[must_use]
    pub fn max_health(self) -> f32 {
        match self {
            CreatureKind::Pig => 10.0,
            CreatureKind::Cow => 10.0,
            CreatureKind::Sheep => 8.0,
            CreatureKind::Chicken => 4.0,
            CreatureKind::Zombie => 20.0,
            CreatureKind::Skeleton => 20.0,
            CreatureKind::Spider => 16.0,
            CreatureKind::Creeper => 20.0,
        }
    }

    /// Get the collider dimensions for this creature type.
    #[must_use]
    pub fn collider(self) -> ColliderShape {
        match self {
            CreatureKind::Pig => ColliderShape::Box {
                half_extents: Vec3::new(0.45, 0.45, 0.45),
            },
            CreatureKind::Cow => ColliderShape::Box {
                half_extents: Vec3::new(0.45, 0.7, 0.45),
            },
            CreatureKind::Sheep => ColliderShape::Box {
                half_extents: Vec3::new(0.45, 0.65, 0.45),
            },
            CreatureKind::Chicken => ColliderShape::Box {
                half_extents: Vec3::new(0.2, 0.35, 0.2),
            },
            CreatureKind::Zombie => ColliderShape::Capsule {
                height: 1.8,
                radius: 0.3,
            },
            CreatureKind::Skeleton => ColliderShape::Capsule {
                height: 1.8,
                radius: 0.3,
            },
            CreatureKind::Spider => ColliderShape::Box {
                half_extents: Vec3::new(0.7, 0.45, 0.7),
            },
            CreatureKind::Creeper => ColliderShape::Capsule {
                height: 1.7,
                radius: 0.3,
            },
        }
    }

    /// Get the movement speed for this creature type.
    #[must_use]
    pub fn move_speed(self) -> f32 {
        match self {
            CreatureKind::Pig => 2.5,
            CreatureKind::Cow => 2.0,
            CreatureKind::Sheep => 2.3,
            CreatureKind::Chicken => 3.0,
            CreatureKind::Zombie => 2.3,
            CreatureKind::Skeleton => 2.5,
            CreatureKind::Spider => 3.0,
            CreatureKind::Creeper => 2.5,
        }
    }

    /// Get the attack damage for hostile creatures.
    #[must_use]
    pub fn attack_damage(self) -> f32 {
        match self {
            CreatureKind::Pig => 0.0,
            CreatureKind::Cow => 0.0,
            CreatureKind::Sheep => 0.0,
            CreatureKind::Chicken => 0.0,
            CreatureKind::Zombie => 3.0,
            CreatureKind::Skeleton => 2.0, // Ranged
            CreatureKind::Spider => 2.0,
            CreatureKind::Creeper => 0.0, // Explodes instead
        }
    }

    /// Get display name.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            CreatureKind::Pig => "Pig",
            CreatureKind::Cow => "Cow",
            CreatureKind::Sheep => "Sheep",
            CreatureKind::Chicken => "Chicken",
            CreatureKind::Zombie => "Zombie",
            CreatureKind::Skeleton => "Skeleton",
            CreatureKind::Spider => "Spider",
            CreatureKind::Creeper => "Creeper",
        }
    }
}

/// Marker component for creatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Creature {
    /// Type of creature.
    pub kind: CreatureKind,
    /// Movement speed multiplier.
    pub speed: f32,
    /// Attack damage multiplier.
    pub damage: f32,
}

impl Creature {
    /// Create a new creature component.
    #[must_use]
    pub fn new(kind: CreatureKind) -> Self {
        Self {
            kind,
            speed: kind.move_speed(),
            damage: kind.attack_damage(),
        }
    }
}

/// Spawn a creature entity in the world.
///
/// Returns the entity ID of the spawned creature.
pub fn spawn_creature(world: &mut World, kind: CreatureKind, position: Vec3) -> Entity {
    let transform = Transform {
        position,
        rotation: Quat::IDENTITY,
    };

    let velocity = Velocity {
        linear: Vec3::ZERO,
        angular: Vec3::ZERO,
    };

    let collider = Collider {
        shape: kind.collider(),
        offset: Vec3::ZERO,
    };

    let health = Health::new(kind.max_health());
    let creature = Creature::new(kind);

    world.spawn((transform, velocity, collider, health, creature))
}

/// Query all creatures in the world, returning collected results.
#[must_use]
pub fn query_creatures(world: &World) -> Vec<(Entity, CreatureKind, Vec3)> {
    world
        .query::<(&Creature, &Transform)>()
        .iter()
        .map(|(entity, (creature, transform))| (entity, creature.kind, transform.position))
        .collect()
}

/// Query hostile creatures.
#[must_use]
pub fn query_hostile(world: &World) -> Vec<(Entity, CreatureKind, Vec3)> {
    world
        .query::<(&Creature, &Transform)>()
        .iter()
        .filter(|(_, (creature, _))| creature.kind.is_hostile())
        .map(|(entity, (creature, transform))| (entity, creature.kind, transform.position))
        .collect()
}

/// Query passive creatures.
#[must_use]
pub fn query_passive(world: &World) -> Vec<(Entity, CreatureKind, Vec3)> {
    world
        .query::<(&Creature, &Transform)>()
        .iter()
        .filter(|(_, (creature, _))| creature.kind.is_passive())
        .map(|(entity, (creature, transform))| (entity, creature.kind, transform.position))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_kind_hostile() {
        assert!(CreatureKind::Zombie.is_hostile());
        assert!(CreatureKind::Skeleton.is_hostile());
        assert!(CreatureKind::Spider.is_hostile());
        assert!(CreatureKind::Creeper.is_hostile());

        assert!(!CreatureKind::Pig.is_hostile());
        assert!(!CreatureKind::Cow.is_hostile());
        assert!(!CreatureKind::Sheep.is_hostile());
        assert!(!CreatureKind::Chicken.is_hostile());
    }

    #[test]
    fn test_creature_kind_passive() {
        assert!(CreatureKind::Pig.is_passive());
        assert!(CreatureKind::Cow.is_passive());
        assert!(CreatureKind::Sheep.is_passive());
        assert!(CreatureKind::Chicken.is_passive());

        assert!(!CreatureKind::Zombie.is_passive());
    }

    #[test]
    fn test_spawn_creature() {
        let mut world = World::new();
        let pos = Vec3::new(10.0, 64.0, 10.0);

        let entity = spawn_creature(&mut world, CreatureKind::Pig, pos);

        // Verify all components exist
        assert!(world.get::<&Transform>(entity).is_ok());
        assert!(world.get::<&Velocity>(entity).is_ok());
        assert!(world.get::<&Collider>(entity).is_ok());
        assert!(world.get::<&Health>(entity).is_ok());
        assert!(world.get::<&Creature>(entity).is_ok());

        // Check creature type
        let creature = world.get::<&Creature>(entity).unwrap();
        assert_eq!(creature.kind, CreatureKind::Pig);

        // Check health
        let health = world.get::<&Health>(entity).unwrap();
        assert_eq!(health.max(), 10.0);
    }

    #[test]
    fn test_spawn_hostile() {
        let mut world = World::new();

        let zombie = spawn_creature(&mut world, CreatureKind::Zombie, Vec3::ZERO);

        let creature = world.get::<&Creature>(zombie).unwrap();
        assert!(creature.kind.is_hostile());
        assert_eq!(creature.damage, 3.0);

        let health = world.get::<&Health>(zombie).unwrap();
        assert_eq!(health.max(), 20.0);
    }

    #[test]
    fn test_query_creatures() {
        let mut world = World::new();

        spawn_creature(&mut world, CreatureKind::Pig, Vec3::ZERO);
        spawn_creature(&mut world, CreatureKind::Zombie, Vec3::ONE);
        spawn_creature(&mut world, CreatureKind::Cow, Vec3::X);

        let all = query_creatures(&world);
        assert_eq!(all.len(), 3);

        let hostile = query_hostile(&world);
        assert_eq!(hostile.len(), 1);

        let passive = query_passive(&world);
        assert_eq!(passive.len(), 2);
    }

    #[test]
    fn test_creature_stats() {
        // Verify different creatures have different stats
        assert!(CreatureKind::Cow.max_health() > CreatureKind::Chicken.max_health());
        assert!(CreatureKind::Zombie.attack_damage() > CreatureKind::Pig.attack_damage());
        assert!(CreatureKind::Chicken.move_speed() > CreatureKind::Cow.move_speed());
    }

    #[test]
    fn test_display_names() {
        assert_eq!(CreatureKind::Pig.display_name(), "Pig");
        assert_eq!(CreatureKind::Zombie.display_name(), "Zombie");
        assert_eq!(CreatureKind::Creeper.display_name(), "Creeper");
    }
}

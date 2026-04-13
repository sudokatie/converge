//! Player entity spawning.

use glam::Vec3;
use hecs::{Entity, World};

use crate::ecs::{Collider, Player, Transform, Velocity};

/// Spawn a player entity at the given position.
///
/// Creates an entity with:
/// - Transform at the given position
/// - Zero velocity
/// - Player-sized capsule collider (1.8m tall, 0.3m radius)
/// - Player component for camera control
///
/// Returns the entity handle.
pub fn spawn_player(world: &mut World, position: Vec3) -> Entity {
    world.spawn((
        Transform::from_position(position),
        Velocity::zero(),
        Collider::player(),
        Player::new(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_player() {
        let mut world = World::new();
        let position = Vec3::new(10.0, 64.0, 20.0);

        let entity = spawn_player(&mut world, position);

        // Verify entity exists and has all components
        assert!(world.contains(entity));

        let transform = world.get::<&Transform>(entity).unwrap();
        assert!((transform.position - position).length() < 0.001);

        let velocity = world.get::<&Velocity>(entity).unwrap();
        assert!(velocity.speed() < 0.001);

        assert!(world.get::<&Collider>(entity).is_ok());
        assert!(world.get::<&Player>(entity).is_ok());
    }

    #[test]
    fn test_spawn_multiple_players() {
        let mut world = World::new();

        let p1 = spawn_player(&mut world, Vec3::new(0.0, 0.0, 0.0));
        let p2 = spawn_player(&mut world, Vec3::new(100.0, 0.0, 0.0));

        assert_ne!(p1, p2);
        assert!(world.contains(p1));
        assert!(world.contains(p2));
    }
}

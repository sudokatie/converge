//! ECS components and systems.

mod components;
mod entity_save;

pub use components::{
    AIBrain, Collider, ColliderShape, Controller, ControllerKind, NetworkId, PendingDestroy,
    Player, Transform, Velocity,
};
pub use entity_save::{EntitySaveData, EntitySaveError, SerializedEntity, SerializedEntityType};

use hecs::{Entity, World};

/// System for deferred entity destruction.
///
/// Entities marked with `PendingDestroy` are collected and despawned at the end of the frame.
/// This prevents issues with removing entities while iterating.
pub struct DeferredDestruction {
    /// Entities queued for destruction.
    pending: Vec<Entity>,
}

impl DeferredDestruction {
    /// Create a new deferred destruction system.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Mark an entity for destruction at the end of the frame.
    pub fn mark(&mut self, entity: Entity) {
        if !self.pending.contains(&entity) {
            self.pending.push(entity);
        }
    }

    /// Collect all entities with `PendingDestroy` component.
    pub fn collect(&mut self, world: &World) {
        for (entity, _) in world.query::<&PendingDestroy>().iter() {
            if !self.pending.contains(&entity) {
                self.pending.push(entity);
            }
        }
    }

    /// Sweep and destroy all pending entities.
    ///
    /// Call this at the end of the frame after all systems have run.
    /// Returns the number of entities destroyed.
    pub fn sweep(&mut self, world: &mut World) -> usize {
        let count = self.pending.len();
        for entity in self.pending.drain(..) {
            let _ = world.despawn(entity);
        }
        count
    }

    /// Mark then sweep in one call.
    ///
    /// Convenience method that collects all `PendingDestroy` entities and destroys them.
    pub fn process(&mut self, world: &mut World) -> usize {
        self.collect(world);
        self.sweep(world)
    }

    /// Get the number of entities pending destruction.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if there are any pending destructions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

impl Default for DeferredDestruction {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deferred_destruction_mark() {
        let mut world = World::new();
        let entity = world.spawn((Transform::default(),));

        let mut destruction = DeferredDestruction::new();
        destruction.mark(entity);

        assert_eq!(destruction.pending_count(), 1);
        assert!(!destruction.is_empty());
    }

    #[test]
    fn test_deferred_destruction_sweep() {
        let mut world = World::new();
        let entity = world.spawn((Transform::default(),));

        let mut destruction = DeferredDestruction::new();
        destruction.mark(entity);

        let count = destruction.sweep(&mut world);
        assert_eq!(count, 1);
        assert!(destruction.is_empty());
        assert!(!world.contains(entity));
    }

    #[test]
    fn test_deferred_destruction_collect() {
        let mut world = World::new();
        world.spawn((Transform::default(), PendingDestroy));
        world.spawn((Transform::default(),)); // Not pending

        let mut destruction = DeferredDestruction::new();
        destruction.collect(&world);

        assert_eq!(destruction.pending_count(), 1);
    }

    #[test]
    fn test_deferred_destruction_process() {
        let mut world = World::new();
        let entity1 = world.spawn((Transform::default(), PendingDestroy));
        let entity2 = world.spawn((Transform::default(),));

        let mut destruction = DeferredDestruction::new();
        let count = destruction.process(&mut world);

        assert_eq!(count, 1);
        assert!(!world.contains(entity1));
        assert!(world.contains(entity2));
    }

    #[test]
    fn test_no_duplicate_marks() {
        let mut world = World::new();
        let entity = world.spawn((Transform::default(),));

        let mut destruction = DeferredDestruction::new();
        destruction.mark(entity);
        destruction.mark(entity);
        destruction.mark(entity);

        assert_eq!(destruction.pending_count(), 1);
    }
}

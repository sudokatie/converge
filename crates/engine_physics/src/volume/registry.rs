//! Volume registry for managing and sampling physics volumes.

use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{
    BlendMode, EntitySample, EntryEvent, ExitEvent, PhysicsLaws, PhysicsVolume, VolumeConfig,
    VolumeEvents, VolumeId,
};

/// Result of sampling a position against the registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleResult {
    /// The blended physics laws at this position.
    pub laws: PhysicsLaws,
    /// IDs of volumes containing this position (highest priority first).
    pub volume_ids: Vec<VolumeId>,
    /// Penetration depths for each volume.
    pub penetrations: Vec<f32>,
}

impl Default for SampleResult {
    fn default() -> Self {
        Self {
            laws: PhysicsLaws::empty(),
            volume_ids: Vec::new(),
            penetrations: Vec::new(),
        }
    }
}

impl SampleResult {
    /// Returns whether the position is inside any volume.
    #[must_use]
    pub fn is_in_volume(&self) -> bool {
        !self.volume_ids.is_empty()
    }

    /// Returns the primary (highest priority) volume.
    #[must_use]
    pub fn primary_volume(&self) -> Option<VolumeId> {
        self.volume_ids.first().copied()
    }
}

/// Registry for managing physics volumes and sampling entities.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VolumeRegistry {
    volumes: HashMap<VolumeId, PhysicsVolume>,
    entity_volumes: HashMap<u64, Vec<VolumeId>>,
}

impl VolumeRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            volumes: HashMap::new(),
            entity_volumes: HashMap::new(),
        }
    }

    /// Registers a volume.
    pub fn register(&mut self, volume: PhysicsVolume) {
        self.volumes.insert(volume.id(), volume);
    }

    /// Unregisters a volume by ID.
    pub fn unregister(&mut self, id: VolumeId) -> Option<PhysicsVolume> {
        self.volumes.remove(&id)
    }

    /// Returns a volume by ID.
    #[must_use]
    pub fn get(&self, id: VolumeId) -> Option<&PhysicsVolume> {
        self.volumes.get(&id)
    }

    /// Returns a mutable reference to a volume by ID.
    pub fn get_mut(&mut self, id: VolumeId) -> Option<&mut PhysicsVolume> {
        self.volumes.get_mut(&id)
    }

    /// Returns the number of registered volumes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.volumes.len()
    }

    /// Returns whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.volumes.is_empty()
    }

    /// Returns an iterator over all volumes.
    pub fn iter(&self) -> impl Iterator<Item = &PhysicsVolume> {
        self.volumes.values()
    }

    /// Samples physics laws at a position.
    #[must_use]
    pub fn sample_position(&self, position: Vec3, layer: u32) -> SampleResult {
        let mut containing: Vec<(VolumeId, i32, f32, &PhysicsLaws)> = Vec::new();

        for volume in self.volumes.values() {
            if !volume.is_enabled() || !volume.applies_to_layer(layer) {
                continue;
            }
            let penetration = volume.penetration_depth(position);
            if penetration > 0.0 {
                containing.push((volume.id(), volume.priority(), penetration, volume.laws()));
            }
        }

        if containing.is_empty() {
            return SampleResult::default();
        }

        containing.sort_by(|a, b| b.1.cmp(&a.1));

        let volume_ids: Vec<_> = containing.iter().map(|(id, _, _, _)| *id).collect();
        let penetrations: Vec<_> = containing.iter().map(|(_, _, p, _)| *p).collect();

        let laws = if let Some(primary) = self.volumes.get(&volume_ids[0]) {
            Self::blend_laws(&containing, primary.config())
        } else {
            PhysicsLaws::empty()
        };

        SampleResult {
            laws,
            volume_ids,
            penetrations,
        }
    }

    /// Samples an entity and generates entry/exit events.
    pub fn sample_entity(&mut self, sample: &EntitySample, events: &mut VolumeEvents) {
        let result = self.sample_position(sample.position, sample.layer);
        let prev_volumes = self.entity_volumes.get(&sample.entity_id).cloned();

        for &volume_id in &result.volume_ids {
            let was_inside = prev_volumes
                .as_ref()
                .is_some_and(|v| v.contains(&volume_id));
            if !was_inside {
                let penetration = result
                    .volume_ids
                    .iter()
                    .position(|&id| id == volume_id)
                    .and_then(|idx| result.penetrations.get(idx).copied())
                    .unwrap_or(0.0);
                events.push_entry(
                    EntryEvent::new(volume_id, sample.entity_id)
                        .with_entry_speed(sample.speed())
                        .with_penetration(penetration),
                );
            }
        }

        if let Some(prev) = &prev_volumes {
            for &volume_id in prev {
                if !result.volume_ids.contains(&volume_id) {
                    events.push_exit(
                        ExitEvent::new(volume_id, sample.entity_id).with_exit_speed(sample.speed()),
                    );
                }
            }
        }

        if result.volume_ids.is_empty() {
            self.entity_volumes.remove(&sample.entity_id);
        } else {
            self.entity_volumes
                .insert(sample.entity_id, result.volume_ids);
        }
    }

    /// Clears all entity tracking state.
    pub fn clear_entity_state(&mut self) {
        self.entity_volumes.clear();
    }

    fn blend_laws(
        containing: &[(VolumeId, i32, f32, &PhysicsLaws)],
        primary_config: &VolumeConfig,
    ) -> PhysicsLaws {
        if containing.is_empty() {
            return PhysicsLaws::empty();
        }

        let (_, _, _, primary_laws) = &containing[0];

        match primary_config.blend_mode {
            BlendMode::Replace => **primary_laws,
            BlendMode::Blend => {
                let total_penetration: f32 = containing.iter().map(|(_, _, p, _)| p).sum();
                if total_penetration <= 0.0 {
                    return **primary_laws;
                }
                let mut result = PhysicsLaws::empty();
                for (_, _, penetration, laws) in containing {
                    let weight = penetration / total_penetration;
                    result = result.blend(laws, weight);
                }
                result
            }
            BlendMode::Additive | BlendMode::Multiply => {
                let mut result = **primary_laws;
                for (_, _, _, laws) in containing.iter().skip(1) {
                    result = result.merge(laws);
                }
                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::math::Aabb;

    use crate::volume::VolumeShape;

    fn test_volume(id: u64, center: Vec3, half_extents: Vec3) -> PhysicsVolume {
        PhysicsVolume::new(
            VolumeId::new(id),
            VolumeShape::aabb(Aabb::from_center_half_extents(center, half_extents)),
            PhysicsLaws::low_gravity(),
        )
    }

    #[test]
    fn register_and_get() {
        let mut registry = VolumeRegistry::new();
        let volume = test_volume(1, Vec3::ZERO, Vec3::ONE);
        registry.register(volume);

        assert_eq!(registry.len(), 1);
        assert!(registry.get(VolumeId::new(1)).is_some());
        assert!(registry.get(VolumeId::new(2)).is_none());
    }

    #[test]
    fn unregister() {
        let mut registry = VolumeRegistry::new();
        registry.register(test_volume(1, Vec3::ZERO, Vec3::ONE));

        let removed = registry.unregister(VolumeId::new(1));
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn sample_empty_registry() {
        let registry = VolumeRegistry::new();
        let result = registry.sample_position(Vec3::ZERO, 0);
        assert!(!result.is_in_volume());
    }

    #[test]
    fn sample_inside_volume() {
        let mut registry = VolumeRegistry::new();
        registry.register(test_volume(1, Vec3::ZERO, Vec3::ONE));

        let result = registry.sample_position(Vec3::ZERO, 0);
        assert!(result.is_in_volume());
        assert_eq!(result.primary_volume(), Some(VolumeId::new(1)));
    }

    #[test]
    fn sample_outside_volume() {
        let mut registry = VolumeRegistry::new();
        registry.register(test_volume(1, Vec3::ZERO, Vec3::ONE));

        let result = registry.sample_position(Vec3::splat(10.0), 0);
        assert!(!result.is_in_volume());
    }

    #[test]
    fn sample_entity_entry_event() {
        let mut registry = VolumeRegistry::new();
        registry.register(test_volume(1, Vec3::ZERO, Vec3::ONE));

        let mut events = VolumeEvents::new();
        let sample = EntitySample::new(42, Vec3::ZERO);
        registry.sample_entity(&sample, &mut events);

        assert_eq!(events.len(), 1);
        assert!(events.iter().next().unwrap().is_entry());
    }

    #[test]
    fn sample_entity_exit_event() {
        let mut registry = VolumeRegistry::new();
        registry.register(test_volume(1, Vec3::ZERO, Vec3::ONE));

        let mut events = VolumeEvents::new();

        let enter = EntitySample::new(42, Vec3::ZERO);
        registry.sample_entity(&enter, &mut events);
        events.clear();

        let exit = EntitySample::new(42, Vec3::splat(10.0));
        registry.sample_entity(&exit, &mut events);

        assert_eq!(events.len(), 1);
        assert!(events.iter().next().unwrap().is_exit());
    }

    #[test]
    fn priority_ordering() {
        let mut registry = VolumeRegistry::new();

        let low_priority = test_volume(1, Vec3::ZERO, Vec3::splat(2.0));
        let high_priority = test_volume(2, Vec3::ZERO, Vec3::ONE)
            .with_config(VolumeConfig::default().with_priority(10));

        registry.register(low_priority);
        registry.register(high_priority);

        let result = registry.sample_position(Vec3::ZERO, 0);
        assert_eq!(result.primary_volume(), Some(VolumeId::new(2)));
    }

    #[test]
    fn layer_filtering() {
        let mut registry = VolumeRegistry::new();
        let volume = test_volume(1, Vec3::ZERO, Vec3::ONE)
            .with_config(VolumeConfig::default().with_layer_mask(0b0010));
        registry.register(volume);

        let result_layer0 = registry.sample_position(Vec3::ZERO, 0);
        assert!(!result_layer0.is_in_volume());

        let result_layer1 = registry.sample_position(Vec3::ZERO, 1);
        assert!(result_layer1.is_in_volume());
    }
}

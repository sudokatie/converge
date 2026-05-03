//! Entity sampling within physics volumes.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{PhysicsLaws, VolumeId};

/// A sample point for an entity within volumes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntitySample {
    /// Entity identifier.
    pub entity_id: u64,
    /// World-space position.
    pub position: Vec3,
    /// Current velocity.
    pub velocity: Vec3,
    /// Physics layer for filtering.
    pub layer: u32,
}

impl EntitySample {
    /// Creates a new entity sample.
    #[must_use]
    pub const fn new(entity_id: u64, position: Vec3) -> Self {
        Self {
            entity_id,
            position,
            velocity: Vec3::ZERO,
            layer: 0,
        }
    }

    /// Builder: sets velocity.
    #[must_use]
    pub const fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    /// Builder: sets layer.
    #[must_use]
    pub const fn with_layer(mut self, layer: u32) -> Self {
        self.layer = layer;
        self
    }

    /// Returns the speed (velocity magnitude).
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.velocity.length()
    }
}

/// Result of sampling volumes for an entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampledLaws {
    /// The final blended physics laws.
    pub laws: PhysicsLaws,
    /// IDs of volumes affecting this entity (highest priority first).
    pub volume_ids: Vec<VolumeId>,
    /// Penetration depths for each volume (same order as `volume_ids`).
    pub penetrations: Vec<f32>,
}

impl Default for SampledLaws {
    fn default() -> Self {
        Self {
            laws: PhysicsLaws::empty(),
            volume_ids: Vec::new(),
            penetrations: Vec::new(),
        }
    }
}

impl SampledLaws {
    /// Creates empty sampled laws.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            laws: PhysicsLaws::empty(),
            volume_ids: Vec::new(),
            penetrations: Vec::new(),
        }
    }

    /// Creates sampled laws from a single volume.
    #[must_use]
    pub fn single(volume_id: VolumeId, laws: PhysicsLaws, penetration: f32) -> Self {
        Self {
            laws,
            volume_ids: vec![volume_id],
            penetrations: vec![penetration],
        }
    }

    /// Returns whether this entity is in any volume.
    #[must_use]
    pub fn is_in_volume(&self) -> bool {
        !self.volume_ids.is_empty()
    }

    /// Returns the number of volumes affecting this entity.
    #[must_use]
    pub fn volume_count(&self) -> usize {
        self.volume_ids.len()
    }

    /// Returns the primary (highest priority) volume ID.
    #[must_use]
    pub fn primary_volume(&self) -> Option<VolumeId> {
        self.volume_ids.first().copied()
    }

    /// Returns the maximum penetration depth.
    #[must_use]
    pub fn max_penetration(&self) -> f32 {
        self.penetrations
            .iter()
            .copied()
            .max_by(f32::total_cmp)
            .unwrap_or(0.0)
    }

    /// Returns whether a specific volume affects this entity.
    #[must_use]
    pub fn is_in(&self, volume_id: VolumeId) -> bool {
        self.volume_ids.contains(&volume_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn entity_sample_default() {
        let sample = EntitySample::new(42, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(sample.entity_id, 42);
        assert_eq!(sample.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(sample.velocity, Vec3::ZERO);
        assert_eq!(sample.layer, 0);
    }

    #[test]
    fn entity_sample_builder() {
        let sample = EntitySample::new(1, Vec3::ZERO)
            .with_velocity(Vec3::new(3.0, 4.0, 0.0))
            .with_layer(5);

        assert_relative_eq!(sample.speed(), 5.0);
        assert_eq!(sample.layer, 5);
    }

    #[test]
    fn sampled_laws_empty() {
        let sampled = SampledLaws::empty();
        assert!(!sampled.is_in_volume());
        assert_eq!(sampled.volume_count(), 0);
        assert!(sampled.primary_volume().is_none());
    }

    #[test]
    fn sampled_laws_single() {
        let sampled = SampledLaws::single(VolumeId::new(1), PhysicsLaws::low_gravity(), 0.5);
        assert!(sampled.is_in_volume());
        assert_eq!(sampled.volume_count(), 1);
        assert_eq!(sampled.primary_volume(), Some(VolumeId::new(1)));
        assert_relative_eq!(sampled.max_penetration(), 0.5);
    }

    #[test]
    fn sampled_laws_is_in() {
        let sampled = SampledLaws {
            laws: PhysicsLaws::default(),
            volume_ids: vec![VolumeId::new(1), VolumeId::new(3)],
            penetrations: vec![0.5, 1.0],
        };

        assert!(sampled.is_in(VolumeId::new(1)));
        assert!(!sampled.is_in(VolumeId::new(2)));
        assert!(sampled.is_in(VolumeId::new(3)));
    }

    #[test]
    fn max_penetration() {
        let sampled = SampledLaws {
            laws: PhysicsLaws::default(),
            volume_ids: vec![VolumeId::new(1), VolumeId::new(2), VolumeId::new(3)],
            penetrations: vec![0.5, 2.0, 1.0],
        };

        assert_relative_eq!(sampled.max_penetration(), 2.0);
    }

    #[test]
    fn serialization() {
        let sampled = SampledLaws::single(VolumeId::new(5), PhysicsLaws::underwater(), 1.5);
        let json = serde_json::to_string(&sampled).unwrap();
        let recovered: SampledLaws = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.primary_volume(), Some(VolumeId::new(5)));
    }
}

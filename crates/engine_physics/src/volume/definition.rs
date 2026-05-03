//! Physics volume definition.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{
    CollisionModifier, MaterialModifier, PhysicsLaws, VolumeConfig, VolumeFingerprint, VolumeId,
    VolumeShape,
};

/// A physics volume with custom laws for a spatial region.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsVolume {
    /// Unique identifier.
    id: VolumeId,
    /// Spatial bounds.
    shape: VolumeShape,
    /// Physics parameter overrides.
    laws: PhysicsLaws,
    /// Volume behavior configuration.
    config: VolumeConfig,
    /// Optional collision modifications.
    collision_modifier: Option<CollisionModifier>,
    /// Optional material modifications.
    material_modifier: Option<MaterialModifier>,
    /// Optional user-defined tag for filtering.
    tag: u32,
}

impl PhysicsVolume {
    /// Creates a new physics volume.
    #[must_use]
    pub fn new(id: VolumeId, shape: VolumeShape, laws: PhysicsLaws) -> Self {
        Self {
            id,
            shape,
            laws,
            config: VolumeConfig::default(),
            collision_modifier: None,
            material_modifier: None,
            tag: 0,
        }
    }

    /// Creates a volume with all parameters.
    #[must_use]
    pub fn with_all(
        id: VolumeId,
        shape: VolumeShape,
        laws: PhysicsLaws,
        config: VolumeConfig,
        collision_modifier: Option<CollisionModifier>,
        material_modifier: Option<MaterialModifier>,
    ) -> Self {
        Self {
            id,
            shape,
            laws,
            config,
            collision_modifier,
            material_modifier,
            tag: 0,
        }
    }

    /// Builder: sets configuration.
    #[must_use]
    pub fn with_config(mut self, config: VolumeConfig) -> Self {
        self.config = config;
        self
    }

    /// Builder: sets collision modifier.
    #[must_use]
    pub fn with_collision_modifier(mut self, modifier: CollisionModifier) -> Self {
        self.collision_modifier = Some(modifier);
        self
    }

    /// Builder: sets material modifier.
    #[must_use]
    pub fn with_material_modifier(mut self, modifier: MaterialModifier) -> Self {
        self.material_modifier = Some(modifier);
        self
    }

    /// Builder: sets tag.
    #[must_use]
    pub const fn with_tag(mut self, tag: u32) -> Self {
        self.tag = tag;
        self
    }

    /// Returns the volume ID.
    #[must_use]
    pub const fn id(&self) -> VolumeId {
        self.id
    }

    /// Returns the volume shape.
    #[must_use]
    pub const fn shape(&self) -> &VolumeShape {
        &self.shape
    }

    /// Returns the physics laws.
    #[must_use]
    pub const fn laws(&self) -> &PhysicsLaws {
        &self.laws
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &VolumeConfig {
        &self.config
    }

    /// Returns the collision modifier if set.
    #[must_use]
    pub const fn collision_modifier(&self) -> Option<&CollisionModifier> {
        self.collision_modifier.as_ref()
    }

    /// Returns the material modifier if set.
    #[must_use]
    pub const fn material_modifier(&self) -> Option<&MaterialModifier> {
        self.material_modifier.as_ref()
    }

    /// Returns the tag.
    #[must_use]
    pub const fn tag(&self) -> u32 {
        self.tag
    }

    /// Returns whether this volume is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Returns the priority.
    #[must_use]
    pub const fn priority(&self) -> i32 {
        self.config.priority
    }

    /// Returns whether a point is inside this volume.
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.config.enabled && self.shape.contains_point(point)
    }

    /// Returns the penetration depth for a point (0 if outside).
    #[must_use]
    pub fn penetration_depth(&self, point: Vec3) -> f32 {
        if !self.config.enabled {
            return 0.0;
        }
        self.shape.penetration_depth(point)
    }

    /// Returns the center of the volume.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        self.shape.center()
    }

    /// Returns whether this volume applies to the given layer.
    #[must_use]
    pub const fn applies_to_layer(&self, layer: u32) -> bool {
        self.config.applies_to_layer(layer)
    }

    /// Sets the enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Sets the shape.
    pub fn set_shape(&mut self, shape: VolumeShape) {
        self.shape = shape;
    }

    /// Sets the physics laws.
    pub fn set_laws(&mut self, laws: PhysicsLaws) {
        self.laws = laws;
    }

    /// Sets the priority.
    pub fn set_priority(&mut self, priority: i32) {
        self.config.priority = priority;
    }

    /// Computes a stable fingerprint for this volume.
    #[must_use]
    pub fn fingerprint(&self) -> VolumeFingerprint {
        VolumeFingerprint::from_volume(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use engine_core::math::Aabb;

    fn test_volume() -> PhysicsVolume {
        PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::aabb(Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE)),
            PhysicsLaws::low_gravity(),
        )
    }

    #[test]
    fn create_volume() {
        let volume = test_volume();
        assert_eq!(volume.id(), VolumeId::new(1));
        assert!(volume.is_enabled());
        assert_eq!(volume.priority(), 0);
    }

    #[test]
    fn builder_chain() {
        let volume = PhysicsVolume::new(
            VolumeId::new(2),
            VolumeShape::sphere_centered(Vec3::ZERO, 5.0),
            PhysicsLaws::underwater(),
        )
        .with_config(VolumeConfig::default().with_priority(10))
        .with_collision_modifier(CollisionModifier::soft())
        .with_material_modifier(MaterialModifier::sticky())
        .with_tag(42);

        assert_eq!(volume.priority(), 10);
        assert!(volume.collision_modifier().is_some());
        assert!(volume.material_modifier().is_some());
        assert_eq!(volume.tag(), 42);
    }

    #[test]
    fn contains_point() {
        let volume = test_volume();
        assert!(volume.contains_point(Vec3::ZERO));
        assert!(volume.contains_point(Vec3::splat(0.5)));
        assert!(!volume.contains_point(Vec3::splat(2.0)));
    }

    #[test]
    fn disabled_volume_contains_nothing() {
        let mut volume = test_volume();
        volume.set_enabled(false);
        assert!(!volume.contains_point(Vec3::ZERO));
        assert_relative_eq!(volume.penetration_depth(Vec3::ZERO), 0.0);
    }

    #[test]
    fn penetration_depth() {
        let volume = PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::sphere_centered(Vec3::ZERO, 2.0),
            PhysicsLaws::default(),
        );
        assert_relative_eq!(volume.penetration_depth(Vec3::ZERO), 2.0);
        assert_relative_eq!(volume.penetration_depth(Vec3::new(1.0, 0.0, 0.0)), 1.0);
    }

    #[test]
    fn layer_filtering() {
        let volume = PhysicsVolume::new(
            VolumeId::new(1),
            VolumeShape::default(),
            PhysicsLaws::default(),
        )
        .with_config(VolumeConfig::default().with_layer_mask(0b0101));

        assert!(volume.applies_to_layer(0));
        assert!(!volume.applies_to_layer(1));
        assert!(volume.applies_to_layer(2));
    }

    #[test]
    fn mutation() {
        let mut volume = test_volume();
        volume.set_priority(5);
        volume.set_laws(PhysicsLaws::zero_gravity());
        assert_eq!(volume.priority(), 5);
        assert!(volume.laws().gravity.is_some());
    }

    #[test]
    fn serialization() {
        let volume = test_volume()
            .with_config(VolumeConfig::default().with_priority(7))
            .with_tag(99);

        let json = serde_json::to_string(&volume).unwrap();
        let recovered: PhysicsVolume = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.id(), volume.id());
        assert_eq!(recovered.priority(), 7);
        assert_eq!(recovered.tag(), 99);
    }

    #[test]
    fn bincode_serialization() {
        let volume = test_volume();
        let bytes = bincode::serialize(&volume).unwrap();
        let recovered: PhysicsVolume = bincode::deserialize(&bytes).unwrap();
        assert_eq!(recovered.id(), volume.id());
    }
}

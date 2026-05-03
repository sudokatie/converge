//! Collision and material modifiers for volumes.

use serde::{Deserialize, Serialize};

/// Modifies collision behavior within a volume.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollisionModifier {
    /// Collision response scale (0 = no collision, 1 = normal, >1 = amplified).
    pub response_scale: f32,
    /// Penetration resolution scale.
    pub penetration_scale: f32,
    /// Whether to disable entity-entity collisions.
    pub disable_entity_collisions: bool,
    /// Whether to disable entity-world collisions.
    pub disable_world_collisions: bool,
    /// Contact offset adjustment.
    pub contact_offset: f32,
}

impl Default for CollisionModifier {
    fn default() -> Self {
        Self {
            response_scale: 1.0,
            penetration_scale: 1.0,
            disable_entity_collisions: false,
            disable_world_collisions: false,
            contact_offset: 0.0,
        }
    }
}

impl CollisionModifier {
    /// Creates a collision modifier with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            response_scale: 1.0,
            penetration_scale: 1.0,
            disable_entity_collisions: false,
            disable_world_collisions: false,
            contact_offset: 0.0,
        }
    }

    /// Creates a no-collision modifier (ghost volume).
    #[must_use]
    pub const fn ghost() -> Self {
        Self {
            response_scale: 0.0,
            penetration_scale: 0.0,
            disable_entity_collisions: true,
            disable_world_collisions: true,
            contact_offset: 0.0,
        }
    }

    /// Creates a soft collision modifier.
    #[must_use]
    pub const fn soft() -> Self {
        Self {
            response_scale: 0.3,
            penetration_scale: 0.5,
            disable_entity_collisions: false,
            disable_world_collisions: false,
            contact_offset: 0.0,
        }
    }

    /// Builder: sets response scale.
    #[must_use]
    pub const fn with_response_scale(mut self, scale: f32) -> Self {
        self.response_scale = scale;
        self
    }

    /// Builder: sets penetration scale.
    #[must_use]
    pub const fn with_penetration_scale(mut self, scale: f32) -> Self {
        self.penetration_scale = scale;
        self
    }

    /// Builder: disables entity collisions.
    #[must_use]
    pub const fn with_entity_collisions_disabled(mut self) -> Self {
        self.disable_entity_collisions = true;
        self
    }

    /// Builder: disables world collisions.
    #[must_use]
    pub const fn with_world_collisions_disabled(mut self) -> Self {
        self.disable_world_collisions = true;
        self
    }

    /// Builder: sets contact offset.
    #[must_use]
    pub const fn with_contact_offset(mut self, offset: f32) -> Self {
        self.contact_offset = offset;
        self
    }

    /// Returns whether collisions are effectively disabled.
    #[must_use]
    pub const fn is_ghost(&self) -> bool {
        self.disable_entity_collisions && self.disable_world_collisions
    }

    /// Returns whether any collision modification is active.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        (self.response_scale - 1.0).abs() > f32::EPSILON
            || (self.penetration_scale - 1.0).abs() > f32::EPSILON
            || self.disable_entity_collisions
            || self.disable_world_collisions
            || self.contact_offset.abs() > f32::EPSILON
    }
}

/// Modifies material properties within a volume.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialModifier {
    /// Friction coefficient override (None = no override).
    pub friction: Option<f32>,
    /// Restitution override (None = no override).
    pub restitution: Option<f32>,
    /// Density modifier (multiplied with entity density).
    pub density_scale: f32,
    /// Surface type identifier for audio/visual effects.
    pub surface_type: u32,
}

impl Default for MaterialModifier {
    fn default() -> Self {
        Self {
            friction: None,
            restitution: None,
            density_scale: 1.0,
            surface_type: 0,
        }
    }
}

impl MaterialModifier {
    /// Creates a material modifier with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            friction: None,
            restitution: None,
            density_scale: 1.0,
            surface_type: 0,
        }
    }

    /// Creates an icy surface modifier.
    #[must_use]
    pub const fn ice() -> Self {
        Self {
            friction: Some(0.05),
            restitution: Some(0.1),
            density_scale: 1.0,
            surface_type: 1,
        }
    }

    /// Creates a sticky surface modifier.
    #[must_use]
    pub const fn sticky() -> Self {
        Self {
            friction: Some(2.0),
            restitution: Some(0.0),
            density_scale: 1.0,
            surface_type: 2,
        }
    }

    /// Creates a bouncy surface modifier.
    #[must_use]
    pub const fn bouncy() -> Self {
        Self {
            friction: Some(0.3),
            restitution: Some(1.2),
            density_scale: 1.0,
            surface_type: 3,
        }
    }

    /// Builder: sets friction override.
    #[must_use]
    pub const fn with_friction(mut self, friction: f32) -> Self {
        self.friction = Some(friction);
        self
    }

    /// Builder: sets restitution override.
    #[must_use]
    pub const fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = Some(restitution);
        self
    }

    /// Builder: sets density scale.
    #[must_use]
    pub const fn with_density_scale(mut self, scale: f32) -> Self {
        self.density_scale = scale;
        self
    }

    /// Builder: sets surface type.
    #[must_use]
    pub const fn with_surface_type(mut self, surface: u32) -> Self {
        self.surface_type = surface;
        self
    }

    /// Returns whether any material property is overridden.
    #[must_use]
    pub fn has_overrides(&self) -> bool {
        self.friction.is_some()
            || self.restitution.is_some()
            || (self.density_scale - 1.0).abs() > f32::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn collision_modifier_default() {
        let modifier = CollisionModifier::default();
        assert!(!modifier.is_ghost());
        assert!(!modifier.is_modified());
    }

    #[test]
    fn collision_modifier_ghost() {
        let modifier = CollisionModifier::ghost();
        assert!(modifier.is_ghost());
        assert!(modifier.is_modified());
        assert_relative_eq!(modifier.response_scale, 0.0);
    }

    #[test]
    fn collision_modifier_builder() {
        let modifier = CollisionModifier::new()
            .with_response_scale(0.5)
            .with_penetration_scale(0.8)
            .with_contact_offset(0.01);

        assert!(modifier.is_modified());
        assert_relative_eq!(modifier.response_scale, 0.5);
        assert_relative_eq!(modifier.penetration_scale, 0.8);
    }

    #[test]
    fn material_modifier_default() {
        let modifier = MaterialModifier::default();
        assert!(!modifier.has_overrides());
    }

    #[test]
    fn material_modifier_ice() {
        let modifier = MaterialModifier::ice();
        assert!(modifier.has_overrides());
        assert_relative_eq!(modifier.friction.unwrap(), 0.05);
    }

    #[test]
    fn material_modifier_builder() {
        let modifier = MaterialModifier::new()
            .with_friction(0.7)
            .with_restitution(0.3)
            .with_surface_type(5);

        assert!(modifier.has_overrides());
        assert_eq!(modifier.surface_type, 5);
    }

    #[test]
    fn collision_serialization() {
        let modifier = CollisionModifier::soft();
        let json = serde_json::to_string(&modifier).unwrap();
        let recovered: CollisionModifier = serde_json::from_str(&json).unwrap();
        assert_relative_eq!(recovered.response_scale, modifier.response_scale);
    }

    #[test]
    fn material_serialization() {
        let modifier = MaterialModifier::bouncy();
        let json = serde_json::to_string(&modifier).unwrap();
        let recovered: MaterialModifier = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.restitution, modifier.restitution);
    }
}

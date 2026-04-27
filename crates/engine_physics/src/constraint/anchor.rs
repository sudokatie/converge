//! Constraint anchor points and endpoints.
//!
//! Anchors define where constraints attach: either to a fixed world position
//! or to a local point on a body that moves with it.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::body::{BodyId, BodySnapshot};

/// A point in world space used as a fixed anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldAnchor {
    /// Fixed position in world coordinates.
    pub position: Vec3,
}

impl WorldAnchor {
    /// Creates a new world anchor at the given position.
    #[must_use]
    pub const fn new(position: Vec3) -> Self {
        Self { position }
    }

    /// Returns the world-space position (always the stored position).
    #[must_use]
    pub const fn world_position(&self) -> Vec3 {
        self.position
    }
}

impl From<Vec3> for WorldAnchor {
    fn from(position: Vec3) -> Self {
        Self::new(position)
    }
}

/// A point attached to a body in its local coordinate frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BodyAnchor {
    /// The body this anchor is attached to.
    pub body_id: BodyId,
    /// Position in the body's local coordinate frame.
    pub local_offset: Vec3,
}

impl BodyAnchor {
    /// Creates a new body anchor at the body's center of mass.
    #[must_use]
    pub const fn new(body_id: BodyId) -> Self {
        Self {
            body_id,
            local_offset: Vec3::ZERO,
        }
    }

    /// Creates a body anchor with a local offset from the center of mass.
    #[must_use]
    pub const fn with_offset(body_id: BodyId, local_offset: Vec3) -> Self {
        Self {
            body_id,
            local_offset,
        }
    }

    /// Builder: sets the local offset.
    #[must_use]
    pub const fn offset(mut self, local_offset: Vec3) -> Self {
        self.local_offset = local_offset;
        self
    }

    /// Computes the world-space position given the body's current state.
    #[must_use]
    pub fn world_position(&self, body: &BodySnapshot) -> Vec3 {
        body.local_to_world(self.local_offset)
    }

    /// Computes the world-space velocity at this anchor point.
    #[must_use]
    pub fn world_velocity(&self, body: &BodySnapshot) -> Vec3 {
        body.velocity_at_point(self.world_position(body))
    }
}

/// An endpoint for a constraint: either a world anchor or a body anchor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConstraintEndpoint {
    /// Fixed point in world space.
    World(WorldAnchor),
    /// Point attached to a body.
    Body(BodyAnchor),
}

impl Default for ConstraintEndpoint {
    fn default() -> Self {
        Self::World(WorldAnchor::default())
    }
}

impl ConstraintEndpoint {
    /// Creates a world-space endpoint.
    #[must_use]
    pub const fn world(position: Vec3) -> Self {
        Self::World(WorldAnchor::new(position))
    }

    /// Creates a body endpoint at the center of mass.
    #[must_use]
    pub const fn body(body_id: BodyId) -> Self {
        Self::Body(BodyAnchor::new(body_id))
    }

    /// Creates a body endpoint with a local offset.
    #[must_use]
    pub const fn body_offset(body_id: BodyId, local_offset: Vec3) -> Self {
        Self::Body(BodyAnchor::with_offset(body_id, local_offset))
    }

    /// Returns the body ID if this is a body endpoint.
    #[must_use]
    pub const fn body_id(&self) -> Option<BodyId> {
        match self {
            Self::World(_) => None,
            Self::Body(anchor) => Some(anchor.body_id),
        }
    }

    /// Returns whether this is a world anchor.
    #[must_use]
    pub const fn is_world(&self) -> bool {
        matches!(self, Self::World(_))
    }

    /// Returns whether this is a body anchor.
    #[must_use]
    pub const fn is_body(&self) -> bool {
        matches!(self, Self::Body(_))
    }

    /// Computes the world-space position, using the body snapshot if needed.
    #[must_use]
    pub fn world_position(&self, body: Option<&BodySnapshot>) -> Vec3 {
        match self {
            Self::World(anchor) => anchor.world_position(),
            Self::Body(anchor) => body.map_or(Vec3::ZERO, |b| anchor.world_position(b)),
        }
    }

    /// Computes the world-space velocity, using the body snapshot if needed.
    #[must_use]
    pub fn world_velocity(&self, body: Option<&BodySnapshot>) -> Vec3 {
        match self {
            Self::World(_) => Vec3::ZERO,
            Self::Body(anchor) => body.map_or(Vec3::ZERO, |b| anchor.world_velocity(b)),
        }
    }

    /// Returns the inverse mass contribution from this endpoint.
    #[must_use]
    pub fn inv_mass(&self, body: Option<&BodySnapshot>) -> f32 {
        match self {
            Self::World(_) => 0.0,
            Self::Body(_) => body.map_or(0.0, |b| b.inv_mass),
        }
    }
}

impl From<Vec3> for ConstraintEndpoint {
    fn from(position: Vec3) -> Self {
        Self::world(position)
    }
}

impl From<BodyId> for ConstraintEndpoint {
    fn from(body_id: BodyId) -> Self {
        Self::body(body_id)
    }
}

impl From<WorldAnchor> for ConstraintEndpoint {
    fn from(anchor: WorldAnchor) -> Self {
        Self::World(anchor)
    }
}

impl From<BodyAnchor> for ConstraintEndpoint {
    fn from(anchor: BodyAnchor) -> Self {
        Self::Body(anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Quat;

    fn assert_vec3_eq(a: Vec3, b: Vec3) {
        assert_relative_eq!(a.x, b.x, epsilon = 1e-6);
        assert_relative_eq!(a.y, b.y, epsilon = 1e-6);
        assert_relative_eq!(a.z, b.z, epsilon = 1e-6);
    }

    #[test]
    fn world_anchor_position() {
        let anchor = WorldAnchor::new(Vec3::new(1.0, 2.0, 3.0));
        assert_vec3_eq(anchor.world_position(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn body_anchor_at_center() {
        let body = BodySnapshot::new(Vec3::new(5.0, 0.0, 0.0));
        let anchor = BodyAnchor::new(BodyId::new(1));
        assert_vec3_eq(anchor.world_position(&body), Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn body_anchor_with_offset() {
        let body = BodySnapshot::new(Vec3::new(5.0, 0.0, 0.0));
        let anchor = BodyAnchor::with_offset(BodyId::new(1), Vec3::new(1.0, 0.0, 0.0));
        assert_vec3_eq(anchor.world_position(&body), Vec3::new(6.0, 0.0, 0.0));
    }

    #[test]
    fn body_anchor_with_rotation() {
        use std::f32::consts::FRAC_PI_2;
        let body = BodySnapshot::new(Vec3::ZERO).with_orientation(Quat::from_rotation_z(FRAC_PI_2));
        let anchor = BodyAnchor::with_offset(BodyId::new(1), Vec3::new(1.0, 0.0, 0.0));
        let pos = anchor.world_position(&body);
        assert_relative_eq!(pos.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(pos.y, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn endpoint_world_variant() {
        let endpoint = ConstraintEndpoint::world(Vec3::new(1.0, 2.0, 3.0));
        assert!(endpoint.is_world());
        assert!(!endpoint.is_body());
        assert!(endpoint.body_id().is_none());
        assert_vec3_eq(endpoint.world_position(None), Vec3::new(1.0, 2.0, 3.0));
        assert_vec3_eq(endpoint.world_velocity(None), Vec3::ZERO);
        assert_relative_eq!(endpoint.inv_mass(None), 0.0);
    }

    #[test]
    fn endpoint_body_variant() {
        let body = BodySnapshot::new(Vec3::new(1.0, 0.0, 0.0))
            .with_mass(2.0)
            .with_linear_velocity(Vec3::new(0.0, 1.0, 0.0));
        let endpoint = ConstraintEndpoint::body(BodyId::new(42));
        assert!(!endpoint.is_world());
        assert!(endpoint.is_body());
        assert_eq!(endpoint.body_id(), Some(BodyId::new(42)));
        assert_vec3_eq(
            endpoint.world_position(Some(&body)),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert_vec3_eq(
            endpoint.world_velocity(Some(&body)),
            Vec3::new(0.0, 1.0, 0.0),
        );
        assert_relative_eq!(endpoint.inv_mass(Some(&body)), 0.5);
    }

    #[test]
    fn endpoint_from_vec3() {
        let endpoint: ConstraintEndpoint = Vec3::new(1.0, 2.0, 3.0).into();
        assert!(endpoint.is_world());
    }

    #[test]
    fn endpoint_from_body_id() {
        let endpoint: ConstraintEndpoint = BodyId::new(5).into();
        assert!(endpoint.is_body());
        assert_eq!(endpoint.body_id(), Some(BodyId::new(5)));
    }

    #[test]
    fn anchor_serialization() {
        let world = WorldAnchor::new(Vec3::new(1.0, 2.0, 3.0));
        let json = serde_json::to_string(&world).unwrap();
        let recovered: WorldAnchor = serde_json::from_str(&json).unwrap();
        assert_vec3_eq(recovered.position, world.position);

        let body = BodyAnchor::with_offset(BodyId::new(10), Vec3::X);
        let json = serde_json::to_string(&body).unwrap();
        let recovered: BodyAnchor = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.body_id, body.body_id);
        assert_vec3_eq(recovered.local_offset, body.local_offset);
    }

    #[test]
    fn endpoint_serialization() {
        let endpoint = ConstraintEndpoint::body_offset(BodyId::new(7), Vec3::Y);
        let json = serde_json::to_string(&endpoint).unwrap();
        let recovered: ConstraintEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, endpoint);
    }
}

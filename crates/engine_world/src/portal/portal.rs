//! Portal definition linking two endpoints.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::endpoint::PortalEndpoint;
use super::id::PortalId;
use super::transform::PortalTransform;

/// Traversal direction through a portal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortalSide {
    /// Traversing from endpoint A to endpoint B.
    AtoB,
    /// Traversing from endpoint B to endpoint A.
    BtoA,
}

impl PortalSide {
    /// Get the opposite side.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::AtoB => Self::BtoA,
            Self::BtoA => Self::AtoB,
        }
    }
}

/// Flags for portal behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "these are independent behavioral flags, not state encoding"
)]
pub struct PortalFlags {
    /// Portal is active and can be traversed.
    pub active: bool,
    /// Portal is bidirectional (can traverse either way).
    pub bidirectional: bool,
    /// Portal preserves momentum direction through traversal.
    pub preserve_momentum: bool,
    /// Portal clips geometry that crosses it.
    pub clips_geometry: bool,
    /// Portal renders the destination view.
    pub renders_through: bool,
}

impl Default for PortalFlags {
    fn default() -> Self {
        Self {
            active: true,
            bidirectional: true,
            preserve_momentum: true,
            clips_geometry: true,
            renders_through: true,
        }
    }
}

/// A portal connecting two spatial regions.
///
/// Portals enable non-euclidean traversal by linking two endpoints
/// in potentially different zones. When an entity crosses the portal
/// plane, it is teleported and transformed to the other side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Portal {
    /// Unique identifier for this portal.
    pub id: PortalId,
    /// First endpoint of the portal.
    pub endpoint_a: PortalEndpoint,
    /// Second endpoint of the portal.
    pub endpoint_b: PortalEndpoint,
    /// Transform applied when traversing A to B.
    pub transform_a_to_b: PortalTransform,
    /// Behavioral flags.
    pub flags: PortalFlags,
    /// Optional name for debugging.
    pub name: Option<String>,
}

impl Portal {
    /// Create a new portal between two endpoints.
    #[must_use]
    pub fn new(id: PortalId, endpoint_a: PortalEndpoint, endpoint_b: PortalEndpoint) -> Self {
        let transform_a_to_b = PortalTransform::between_frames(
            endpoint_a.position,
            endpoint_a.forward,
            endpoint_a.up,
            endpoint_b.position,
            endpoint_b.forward,
            endpoint_b.up,
        );

        Self {
            id,
            endpoint_a,
            endpoint_b,
            transform_a_to_b,
            flags: PortalFlags::default(),
            name: None,
        }
    }

    /// Create a portal with explicit transform.
    #[must_use]
    pub fn with_transform(
        id: PortalId,
        endpoint_a: PortalEndpoint,
        endpoint_b: PortalEndpoint,
        transform: PortalTransform,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            transform_a_to_b: transform,
            flags: PortalFlags::default(),
            name: None,
        }
    }

    /// Set a name for debugging.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set portal flags.
    #[must_use]
    pub fn with_flags(mut self, flags: PortalFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Check if the portal is currently traversable.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.flags.active && self.endpoint_a.active && self.endpoint_b.active
    }

    /// Check if traversal in a given direction is allowed.
    #[must_use]
    pub fn can_traverse(&self, side: PortalSide) -> bool {
        if !self.is_active() {
            return false;
        }
        match side {
            PortalSide::AtoB => true,
            PortalSide::BtoA => self.flags.bidirectional,
        }
    }

    /// Get the endpoint for a given traversal direction.
    #[must_use]
    pub fn entry_endpoint(&self, side: PortalSide) -> &PortalEndpoint {
        match side {
            PortalSide::AtoB => &self.endpoint_a,
            PortalSide::BtoA => &self.endpoint_b,
        }
    }

    /// Get the exit endpoint for a given traversal direction.
    #[must_use]
    pub fn exit_endpoint(&self, side: PortalSide) -> &PortalEndpoint {
        match side {
            PortalSide::AtoB => &self.endpoint_b,
            PortalSide::BtoA => &self.endpoint_a,
        }
    }

    /// Get the transform for a given traversal direction.
    #[must_use]
    pub fn transform(&self, side: PortalSide) -> PortalTransform {
        match side {
            PortalSide::AtoB => self.transform_a_to_b,
            PortalSide::BtoA => self.transform_a_to_b.inverted(),
        }
    }

    /// Transform a position through the portal.
    #[must_use]
    pub fn transform_point(&self, point: Vec3, side: PortalSide) -> Vec3 {
        self.transform(side).transform_point(point)
    }

    /// Transform a direction through the portal.
    #[must_use]
    pub fn transform_direction(&self, direction: Vec3, side: PortalSide) -> Vec3 {
        self.transform(side).transform_direction(direction)
    }

    /// Determine which side of the portal a point should enter from.
    ///
    /// Returns None if the point is not in front of either endpoint.
    #[must_use]
    pub fn determine_entry_side(&self, point: Vec3) -> Option<PortalSide> {
        let in_front_a = self.endpoint_a.is_in_front(point);
        let in_front_b = self.endpoint_b.is_in_front(point);

        match (in_front_a, in_front_b) {
            (true, false) => Some(PortalSide::AtoB),
            (false, true) if self.flags.bidirectional => Some(PortalSide::BtoA),
            _ => None,
        }
    }

    /// Check if a point is near the portal plane on either side.
    #[must_use]
    pub fn is_near(&self, point: Vec3, threshold: f32) -> bool {
        let dist_a = self.endpoint_a.signed_distance_to_plane(point).abs();
        let dist_b = self.endpoint_b.signed_distance_to_plane(point).abs();
        dist_a < threshold || dist_b < threshold
    }

    /// Get the approximate center between both endpoints.
    ///
    /// This is useful for sorting portals by distance when endpoints
    /// are in the same zone.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        (self.endpoint_a.position + self.endpoint_b.position) * 0.5
    }

    /// Get the smaller of the two endpoint areas.
    #[must_use]
    pub fn min_area(&self) -> f32 {
        self.endpoint_a.area().min(self.endpoint_b.area())
    }

    /// Recompute the transform based on current endpoint positions.
    pub fn update_transform(&mut self) {
        self.transform_a_to_b = PortalTransform::between_frames(
            self.endpoint_a.position,
            self.endpoint_a.forward,
            self.endpoint_a.up,
            self.endpoint_b.position,
            self.endpoint_b.forward,
            self.endpoint_b.up,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::id::ZoneId;
    use super::*;
    use approx::assert_relative_eq;

    fn test_portal() -> Portal {
        let endpoint_a =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 3.0);
        let endpoint_b = PortalEndpoint::rectangle(
            ZoneId::new(0, 1),
            Vec3::new(100.0, 0.0, 0.0),
            -Vec3::Z,
            Vec3::Y,
            2.0,
            3.0,
        );
        Portal::new(PortalId::new(0, 0), endpoint_a, endpoint_b)
    }

    #[test]
    fn portal_creation() {
        let portal = test_portal();
        assert!(portal.is_active());
        assert!(portal.can_traverse(PortalSide::AtoB));
        assert!(portal.can_traverse(PortalSide::BtoA));
    }

    #[test]
    fn unidirectional_portal() {
        let mut portal = test_portal();
        portal.flags.bidirectional = false;
        assert!(portal.can_traverse(PortalSide::AtoB));
        assert!(!portal.can_traverse(PortalSide::BtoA));
    }

    #[test]
    fn inactive_portal() {
        let mut portal = test_portal();
        portal.flags.active = false;
        assert!(!portal.is_active());
        assert!(!portal.can_traverse(PortalSide::AtoB));
    }

    #[test]
    fn transform_point_through_portal() {
        let portal = test_portal();
        let point = Vec3::new(0.0, 1.0, 0.5);
        let transformed = portal.transform_point(point, PortalSide::AtoB);
        assert_relative_eq!(transformed.x, 100.0, epsilon = 0.1);
    }

    #[test]
    fn roundtrip_transform() {
        let portal = test_portal();
        let point = Vec3::new(0.5, 0.5, 0.1);
        let to_b = portal.transform_point(point, PortalSide::AtoB);
        let back = portal.transform_point(to_b, PortalSide::BtoA);
        assert_relative_eq!(back.x, point.x, epsilon = 1e-4);
        assert_relative_eq!(back.y, point.y, epsilon = 1e-4);
        assert_relative_eq!(back.z, point.z, epsilon = 1e-4);
    }

    #[test]
    fn determine_entry_side() {
        let portal = test_portal();
        let in_front_a = Vec3::new(0.0, 0.0, 1.0);
        let in_front_b = Vec3::new(100.0, 0.0, -1.0);

        assert_eq!(
            portal.determine_entry_side(in_front_a),
            Some(PortalSide::AtoB)
        );
        assert_eq!(
            portal.determine_entry_side(in_front_b),
            Some(PortalSide::BtoA)
        );
    }

    #[test]
    fn side_opposite() {
        assert_eq!(PortalSide::AtoB.opposite(), PortalSide::BtoA);
        assert_eq!(PortalSide::BtoA.opposite(), PortalSide::AtoB);
    }

    #[test]
    fn entry_exit_endpoints() {
        let portal = test_portal();
        assert_eq!(
            portal.entry_endpoint(PortalSide::AtoB).zone,
            portal.endpoint_a.zone
        );
        assert_eq!(
            portal.exit_endpoint(PortalSide::AtoB).zone,
            portal.endpoint_b.zone
        );
    }

    #[test]
    fn is_near() {
        let portal = test_portal();
        let near_a = Vec3::new(0.0, 0.0, 0.5);
        let far = Vec3::new(50.0, 0.0, 50.0);
        assert!(portal.is_near(near_a, 1.0));
        assert!(!portal.is_near(far, 1.0));
    }

    #[test]
    fn with_name() {
        let portal = test_portal().with_name("test_portal");
        assert_eq!(portal.name, Some("test_portal".to_string()));
    }

    #[test]
    fn serde_roundtrip() {
        let portal = test_portal().with_name("named");
        let serialized = bincode::serialize(&portal).unwrap();
        let deserialized: Portal = bincode::deserialize(&serialized).unwrap();
        assert_eq!(portal.id, deserialized.id);
        assert_eq!(portal.name, deserialized.name);
        assert_eq!(portal.flags, deserialized.flags);
    }
}

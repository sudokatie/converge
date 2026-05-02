//! Portal endpoint definition.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::id::ZoneId;

/// Shape of a portal opening.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortalShape {
    /// Rectangular portal with width and height.
    #[default]
    Rectangle,
    /// Circular portal with radius.
    Circle,
    /// Arbitrary convex polygon.
    Polygon,
}

/// One side of a portal connection.
///
/// A portal has two endpoints, each residing in a zone.
/// The endpoint defines the spatial extent and orientation
/// of the portal opening within its zone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalEndpoint {
    /// Zone containing this endpoint.
    pub zone: ZoneId,
    /// Center position of the portal in zone-local coordinates.
    pub position: Vec3,
    /// Forward direction (normal to portal plane, pointing into the portal).
    pub forward: Vec3,
    /// Up direction for portal orientation.
    pub up: Vec3,
    /// Shape of the portal opening.
    pub shape: PortalShape,
    /// Half-extents for rectangular portals, or radius for circular.
    pub half_extents: Vec3,
    /// Whether this endpoint is currently active.
    pub active: bool,
}

impl PortalEndpoint {
    /// Create a new rectangular portal endpoint.
    #[must_use]
    pub fn rectangle(
        zone: ZoneId,
        position: Vec3,
        forward: Vec3,
        up: Vec3,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            zone,
            position,
            forward: forward.normalize(),
            up: up.normalize(),
            shape: PortalShape::Rectangle,
            half_extents: Vec3::new(width * 0.5, height * 0.5, 0.0),
            active: true,
        }
    }

    /// Create a new circular portal endpoint.
    #[must_use]
    pub fn circle(zone: ZoneId, position: Vec3, forward: Vec3, up: Vec3, radius: f32) -> Self {
        Self {
            zone,
            position,
            forward: forward.normalize(),
            up: up.normalize(),
            shape: PortalShape::Circle,
            half_extents: Vec3::splat(radius),
            active: true,
        }
    }

    /// Get the right direction of the portal frame.
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.up.cross(self.forward).normalize()
    }

    /// Get the width of the portal (for rectangular) or diameter (for circular).
    #[must_use]
    pub fn width(&self) -> f32 {
        self.half_extents.x * 2.0
    }

    /// Get the height of the portal (for rectangular) or diameter (for circular).
    #[must_use]
    pub fn height(&self) -> f32 {
        self.half_extents.y * 2.0
    }

    /// Get the approximate area of the portal opening.
    #[must_use]
    pub fn area(&self) -> f32 {
        match self.shape {
            PortalShape::Circle => std::f32::consts::PI * self.half_extents.x * self.half_extents.x,
            PortalShape::Rectangle | PortalShape::Polygon => self.width() * self.height(),
        }
    }

    /// Check if a point is in front of the portal plane.
    #[must_use]
    pub fn is_in_front(&self, point: Vec3) -> bool {
        let to_point = point - self.position;
        to_point.dot(self.forward) > 0.0
    }

    /// Compute signed distance from a point to the portal plane.
    #[must_use]
    pub fn signed_distance_to_plane(&self, point: Vec3) -> f32 {
        let to_point = point - self.position;
        to_point.dot(self.forward)
    }

    /// Project a point onto the portal plane.
    #[must_use]
    pub fn project_to_plane(&self, point: Vec3) -> Vec3 {
        let dist = self.signed_distance_to_plane(point);
        point - self.forward * dist
    }

    /// Convert a world point to portal-local coordinates (u, v, depth).
    #[must_use]
    pub fn to_local(&self, point: Vec3) -> Vec3 {
        let relative = point - self.position;
        let right = self.right();
        Vec3::new(
            relative.dot(right),
            relative.dot(self.up),
            relative.dot(self.forward),
        )
    }

    /// Convert portal-local coordinates back to world position.
    #[must_use]
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        let right = self.right();
        self.position + right * local.x + self.up * local.y + self.forward * local.z
    }

    /// Check if a point (projected to portal plane) is within the portal bounds.
    #[must_use]
    pub fn contains_projected(&self, point: Vec3) -> bool {
        let local = self.to_local(point);
        match self.shape {
            PortalShape::Circle => {
                let r = self.half_extents.x;
                local.x * local.x + local.y * local.y <= r * r
            }
            PortalShape::Rectangle | PortalShape::Polygon => {
                local.x.abs() <= self.half_extents.x && local.y.abs() <= self.half_extents.y
            }
        }
    }

    /// Get corner positions for rectangular portals.
    #[must_use]
    pub fn corners(&self) -> [Vec3; 4] {
        let right = self.right();
        let hw = self.half_extents.x;
        let hh = self.half_extents.y;
        [
            self.position - right * hw - self.up * hh,
            self.position + right * hw - self.up * hh,
            self.position + right * hw + self.up * hh,
            self.position - right * hw + self.up * hh,
        ]
    }

    /// Set active state.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl Default for PortalEndpoint {
    fn default() -> Self {
        Self {
            zone: ZoneId::from_raw(0),
            position: Vec3::ZERO,
            forward: Vec3::Z,
            up: Vec3::Y,
            shape: PortalShape::Rectangle,
            half_extents: Vec3::new(1.0, 2.0, 0.0),
            active: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn rectangle_dimensions() {
        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 4.0, 3.0);
        assert_relative_eq!(endpoint.width(), 4.0, epsilon = 1e-5);
        assert_relative_eq!(endpoint.height(), 3.0, epsilon = 1e-5);
        assert_relative_eq!(endpoint.area(), 12.0, epsilon = 1e-5);
    }

    #[test]
    fn circle_dimensions() {
        let endpoint = PortalEndpoint::circle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 5.0);
        assert_relative_eq!(endpoint.width(), 10.0, epsilon = 1e-5);
        let expected_area = std::f32::consts::PI * 25.0;
        assert_relative_eq!(endpoint.area(), expected_area, epsilon = 1e-4);
    }

    #[test]
    fn right_direction() {
        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 2.0);
        let right = endpoint.right();
        assert_relative_eq!(right.x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(right.y, 0.0, epsilon = 1e-5);
        assert_relative_eq!(right.z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn in_front_detection() {
        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 2.0);
        assert!(endpoint.is_in_front(Vec3::new(0.0, 0.0, 1.0)));
        assert!(!endpoint.is_in_front(Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn local_world_roundtrip() {
        let endpoint = PortalEndpoint::rectangle(
            ZoneId::new(0, 0),
            Vec3::new(10.0, 5.0, 0.0),
            Vec3::Z,
            Vec3::Y,
            2.0,
            2.0,
        );
        let point = Vec3::new(12.0, 7.0, 3.0);
        let local = endpoint.to_local(point);
        let recovered = endpoint.to_world(local);
        assert_relative_eq!(recovered.x, point.x, epsilon = 1e-4);
        assert_relative_eq!(recovered.y, point.y, epsilon = 1e-4);
        assert_relative_eq!(recovered.z, point.z, epsilon = 1e-4);
    }

    #[test]
    fn contains_projected_rectangle() {
        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 2.0);
        let inside = Vec3::new(0.5, 0.5, 0.0);
        let outside = Vec3::new(2.0, 0.0, 0.0);
        assert!(endpoint.contains_projected(inside));
        assert!(!endpoint.contains_projected(outside));
    }

    #[test]
    fn contains_projected_circle() {
        let endpoint = PortalEndpoint::circle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 1.0);
        let inside = Vec3::new(0.5, 0.5, 0.0);
        let outside = Vec3::new(1.0, 1.0, 0.0);
        assert!(endpoint.contains_projected(inside));
        assert!(!endpoint.contains_projected(outside));
    }

    #[test]
    fn corners() {
        let endpoint =
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 2.0);
        let corners = endpoint.corners();
        assert_eq!(corners.len(), 4);
        assert_relative_eq!(corners[0].x, -1.0, epsilon = 1e-5);
        assert_relative_eq!(corners[0].y, -1.0, epsilon = 1e-5);
        assert_relative_eq!(corners[2].x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(corners[2].y, 1.0, epsilon = 1e-5);
    }

    #[test]
    fn serde_roundtrip() {
        let endpoint = PortalEndpoint::rectangle(
            ZoneId::new(1, 2),
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::Z,
            Vec3::Y,
            4.0,
            3.0,
        );
        let serialized = bincode::serialize(&endpoint).unwrap();
        let deserialized: PortalEndpoint = bincode::deserialize(&serialized).unwrap();
        assert_eq!(endpoint.zone, deserialized.zone);
        assert_eq!(endpoint.shape, deserialized.shape);
    }
}

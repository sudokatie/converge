//! Non-euclidean traversal primitives for portal-based world connectivity.
//!
//! This module provides the foundation for non-euclidean geometry in the world
//! system, enabling:
//!
//! - **Portals**: Spatial connections between zones with arbitrary transforms
//! - **Portal Graph**: Zone connectivity graph with traversal queries
//! - **Traversal Paths**: Computed paths through portal networks with cumulative transforms
//! - **Fingerprinting**: Deterministic checksums for verification
//!
//! # Architecture
//!
//! The world is divided into **zones** (spatially distinct regions) connected by
//! **portals** (spatial wormholes). Each portal has two **endpoints**, one in each
//! zone, and a **transform** that maps coordinates from one side to the other.
//!
//! This enables:
//! - Rooms larger on the inside
//! - Impossible spaces (overlapping volumes)
//! - Seamless teleportation with consistent physics
//! - Recursive spaces (portal loops)
//!
//! # Example
//!
//! ```ignore
//! use engine_world::portal::*;
//! use glam::Vec3;
//!
//! // Create a portal graph
//! let mut graph = PortalGraph::new();
//!
//! // Add zones
//! graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new().with_name("room_a"));
//! graph.add_zone(ZoneId::new(0, 1), ZoneMetadata::new().with_name("room_b"));
//!
//! // Create a portal between them
//! let portal = Portal::new(
//!     PortalId::new(0, 0),
//!     PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 3.0),
//!     PortalEndpoint::rectangle(ZoneId::new(0, 1), Vec3::new(100.0, 0.0, 0.0), -Vec3::Z, Vec3::Y, 2.0, 3.0),
//! );
//! graph.add_portal(portal);
//!
//! // Query paths
//! let result = ReachabilityQuery::new(ZoneId::new(0, 0))
//!     .with_max_depth(5)
//!     .execute(&graph);
//!
//! // Transform a point through the path
//! if let Some(path) = result.shortest_path_to(ZoneId::new(0, 1)) {
//!     let local_point = Vec3::new(1.0, 0.0, 0.0);
//!     let in_room_b = path.transform_point(local_point);
//! }
//! ```

mod endpoint;
mod fingerprint;
mod graph;
mod id;
#[expect(
    clippy::module_inception,
    reason = "portal.rs contains the main Portal struct"
)]
mod portal;
mod transform;
mod traversal;
mod validation;

pub use endpoint::{PortalEndpoint, PortalShape};
pub use fingerprint::{PortalFingerprint, PortalFingerprintBuilder, PortalGraphChecksum};
pub use graph::{PortalGraph, ZoneMetadata};
pub use id::{PortalId, TraversalId, ZoneId};
pub use portal::{Portal, PortalFlags, PortalSide};
pub use transform::PortalTransform;
pub use traversal::{
    PathfindQuery, ReachabilityQuery, TraversalConfig, TraversalPath, TraversalResult,
    TraversalStats, TraversalStep, ZoneDistanceMap,
};
pub use validation::{PortalValidationError, PortalValidationErrors};

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn public_exports_available() {
        let _ = PortalId::new(0, 0);
        let _ = ZoneId::new(0, 0);
        let _ = TraversalId::from_raw(0);
        let _ = PortalTransform::identity();
        let _ = PortalShape::Rectangle;
        let _ = PortalSide::AtoB;
        let _ = PortalFlags::default();
        let _ = ZoneMetadata::new();
        let _ = TraversalConfig::default();
        let _ = PortalFingerprint::from_raw(0);
        let _ = PortalGraphChecksum::new(0, 0, 0);
    }

    #[test]
    fn basic_workflow() {
        let mut graph = PortalGraph::new();
        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new().with_name("start"));
        graph.add_zone(ZoneId::new(0, 1), ZoneMetadata::new().with_name("end"));

        let portal = Portal::new(
            PortalId::new(0, 0),
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 2.0, 3.0),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 1),
                Vec3::new(100.0, 0.0, 0.0),
                -Vec3::Z,
                Vec3::Y,
                2.0,
                3.0,
            ),
        );
        graph.add_portal(portal);

        let errors = graph.validate();
        assert!(errors.is_empty());

        let result = ReachabilityQuery::new(ZoneId::new(0, 0))
            .with_max_depth(5)
            .execute(&graph);

        assert!(result.reached(ZoneId::new(0, 1)));
        assert_eq!(result.stats.zones_visited, 2);

        let fp = graph.fingerprint();
        assert_ne!(fp.value(), 0);
    }

    #[test]
    #[expect(clippy::cast_precision_loss, reason = "small test indices")]
    fn pathfind_workflow() {
        let mut graph = PortalGraph::new();
        for i in 0..5 {
            graph.add_zone(ZoneId::new(0, i), ZoneMetadata::new());
        }

        for i in 0..4 {
            let portal = Portal::new(
                PortalId::new(0, i),
                PortalEndpoint::rectangle(
                    ZoneId::new(0, i),
                    Vec3::new(i as f32 * 10.0, 0.0, 5.0),
                    Vec3::Z,
                    Vec3::Y,
                    2.0,
                    3.0,
                ),
                PortalEndpoint::rectangle(
                    ZoneId::new(0, i + 1),
                    Vec3::new((i + 1) as f32 * 10.0, 0.0, 0.0),
                    -Vec3::Z,
                    Vec3::Y,
                    2.0,
                    3.0,
                ),
            );
            graph.add_portal(portal);
        }

        let path = PathfindQuery::new(ZoneId::new(0, 0), ZoneId::new(0, 4)).execute(&graph);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.depth(), 4);
    }

    #[test]
    fn transform_workflow() {
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
        let portal = Portal::new(PortalId::new(0, 0), endpoint_a, endpoint_b);

        let point = Vec3::new(0.5, 0.5, 0.0);
        let transformed = portal.transform_point(point, PortalSide::AtoB);

        assert!(transformed.x > 50.0);
    }

    #[test]
    fn validation_workflow() {
        let mut graph = PortalGraph::new();
        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new());

        let same_zone_portal = Portal::new(
            PortalId::new(0, 0),
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 1.0, 1.0),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 0),
                Vec3::new(10.0, 0.0, 0.0),
                -Vec3::Z,
                Vec3::Y,
                1.0,
                1.0,
            ),
        );
        graph.add_portal(same_zone_portal);

        let errors = graph.validate();
        assert!(!errors.is_empty());
        assert!(errors.any(|e| matches!(e, PortalValidationError::SameZoneEndpoints { .. })));
    }

    #[test]
    fn fingerprint_determinism() {
        let mut graph1 = PortalGraph::new();
        let mut graph2 = PortalGraph::new();

        for graph in [&mut graph1, &mut graph2] {
            graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new());
            graph.add_zone(ZoneId::new(0, 1), ZoneMetadata::new());
            graph.add_portal(Portal::new(
                PortalId::new(0, 0),
                PortalEndpoint::rectangle(
                    ZoneId::new(0, 0),
                    Vec3::ZERO,
                    Vec3::Z,
                    Vec3::Y,
                    2.0,
                    3.0,
                ),
                PortalEndpoint::rectangle(
                    ZoneId::new(0, 1),
                    Vec3::new(10.0, 0.0, 0.0),
                    -Vec3::Z,
                    Vec3::Y,
                    2.0,
                    3.0,
                ),
            ));
        }

        assert!(graph1.fingerprint().matches(&graph2.fingerprint()));
        assert!(graph1.checksum().matches(&graph2.checksum()));
    }

    #[test]
    fn serde_workflow() {
        let mut graph = PortalGraph::new();
        graph.add_zone(ZoneId::new(0, 0), ZoneMetadata::new().with_name("test"));
        graph.add_portal(Portal::new(
            PortalId::new(0, 0),
            PortalEndpoint::rectangle(ZoneId::new(0, 0), Vec3::ZERO, Vec3::Z, Vec3::Y, 1.0, 1.0),
            PortalEndpoint::rectangle(
                ZoneId::new(0, 1),
                Vec3::new(10.0, 0.0, 0.0),
                -Vec3::Z,
                Vec3::Y,
                1.0,
                1.0,
            ),
        ));

        let json = serde_json::to_string(&graph).unwrap();
        let recovered: PortalGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph.portal_count(), recovered.portal_count());

        let bincode_bytes = bincode::serialize(&graph).unwrap();
        let recovered: PortalGraph = bincode::deserialize(&bincode_bytes).unwrap();
        assert!(graph.fingerprint().matches(&recovered.fingerprint()));
    }
}

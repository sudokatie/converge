//! Portal traversal query APIs.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::graph::PortalGraph;
use super::id::{PortalId, TraversalId, ZoneId};
use super::portal::PortalSide;
use super::transform::PortalTransform;

/// Configuration for traversal queries.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraversalConfig {
    /// Maximum depth to traverse (portal crossings).
    pub max_depth: u32,
    /// Whether to allow revisiting zones.
    pub allow_cycles: bool,
    /// Maximum total zones to visit.
    pub max_zones: u32,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 16,
            allow_cycles: false,
            max_zones: 256,
        }
    }
}

impl TraversalConfig {
    /// Create a new config with specified max depth.
    #[must_use]
    pub fn with_max_depth(depth: u32) -> Self {
        Self {
            max_depth: depth,
            ..Default::default()
        }
    }

    /// Allow cycles in traversal.
    #[must_use]
    pub fn with_cycles(mut self) -> Self {
        self.allow_cycles = true;
        self
    }
}

/// A single step in a traversal path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalStep {
    /// The portal crossed.
    pub portal_id: PortalId,
    /// The direction of traversal.
    pub side: PortalSide,
    /// The zone entered after crossing.
    pub entered_zone: ZoneId,
    /// Depth at this step (number of portals crossed).
    pub depth: u32,
}

/// A path through the portal graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TraversalPath {
    /// Unique ID for this path.
    pub id: TraversalId,
    /// Starting zone.
    pub start_zone: ZoneId,
    /// Ending zone.
    pub end_zone: ZoneId,
    /// Steps in the path.
    pub steps: Vec<TraversalStep>,
    /// Cumulative transform from start to end.
    pub transform: PortalTransform,
}

impl TraversalPath {
    /// Create a new empty path starting in a zone.
    #[must_use]
    pub fn new(id: TraversalId, start_zone: ZoneId) -> Self {
        Self {
            id,
            start_zone,
            end_zone: start_zone,
            steps: Vec::new(),
            transform: PortalTransform::identity(),
        }
    }

    /// Add a step to the path.
    pub fn push_step(&mut self, step: TraversalStep, portal_transform: PortalTransform) {
        self.end_zone = step.entered_zone;
        self.transform = self.transform.then(&portal_transform);
        self.steps.push(step);
    }

    /// Get the depth (number of portals crossed).
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "step count fits in u32")]
    pub fn depth(&self) -> u32 {
        self.steps.len() as u32
    }

    /// Check if the path is empty (no portals crossed).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get all zones visited in order.
    #[must_use]
    pub fn zones_visited(&self) -> Vec<ZoneId> {
        let mut zones = vec![self.start_zone];
        for step in &self.steps {
            zones.push(step.entered_zone);
        }
        zones
    }

    /// Transform a point from the start zone's coordinate system to the end zone's.
    #[must_use]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.transform.transform_point(point)
    }

    /// Transform a direction from the start zone's coordinate system to the end zone's.
    #[must_use]
    pub fn transform_direction(&self, direction: Vec3) -> Vec3 {
        self.transform.transform_direction(direction)
    }
}

/// Result of a traversal query.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TraversalResult {
    /// All discovered paths.
    pub paths: Vec<TraversalPath>,
    /// Zones visited during traversal.
    pub visited_zones: BTreeSet<ZoneId>,
    /// Statistics about the traversal.
    pub stats: TraversalStats,
}

impl TraversalResult {
    /// Get paths that end at a specific zone.
    #[must_use]
    pub fn paths_to(&self, zone: ZoneId) -> Vec<&TraversalPath> {
        self.paths.iter().filter(|p| p.end_zone == zone).collect()
    }

    /// Get the shortest path to a zone.
    #[must_use]
    pub fn shortest_path_to(&self, zone: ZoneId) -> Option<&TraversalPath> {
        self.paths_to(zone).into_iter().min_by_key(|p| p.depth())
    }

    /// Check if a zone was reached.
    #[must_use]
    pub fn reached(&self, zone: ZoneId) -> bool {
        self.visited_zones.contains(&zone)
    }
}

/// Statistics about a traversal operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalStats {
    /// Number of zones visited.
    pub zones_visited: u32,
    /// Number of portals considered.
    pub portals_considered: u32,
    /// Maximum depth reached.
    pub max_depth_reached: u32,
    /// Number of paths found.
    pub paths_found: u32,
}

/// Query for zones reachable within a depth limit.
#[derive(Clone, Debug)]
pub struct ReachabilityQuery {
    /// Starting zone.
    pub start_zone: ZoneId,
    /// Configuration.
    pub config: TraversalConfig,
}

impl ReachabilityQuery {
    /// Create a new reachability query.
    #[must_use]
    pub fn new(start_zone: ZoneId) -> Self {
        Self {
            start_zone,
            config: TraversalConfig::default(),
        }
    }

    /// Set max depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Execute the query.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts fit in u32")]
    pub fn execute(&self, graph: &PortalGraph) -> TraversalResult {
        let mut result = TraversalResult::default();
        let mut visited = BTreeSet::new();
        let mut queue: VecDeque<(ZoneId, u32, TraversalPath)> = VecDeque::new();
        let mut next_path_id: u64 = 0;

        let initial_path = TraversalPath::new(TraversalId::from_raw(next_path_id), self.start_zone);
        next_path_id += 1;
        visited.insert(self.start_zone);
        queue.push_back((self.start_zone, 0, initial_path));

        while let Some((zone, depth, path)) = queue.pop_front() {
            if depth > 0 {
                result.paths.push(path.clone());
            }

            if depth >= self.config.max_depth {
                result.stats.max_depth_reached = result.stats.max_depth_reached.max(depth);
                continue;
            }

            if visited.len() as u32 >= self.config.max_zones {
                break;
            }

            for portal in graph.zone_portals(zone) {
                result.stats.portals_considered += 1;

                let Some(side) = graph.portal_side_from_zone(portal.id, zone) else {
                    continue;
                };

                if !portal.can_traverse(side) {
                    continue;
                }

                let exit_zone = portal.exit_endpoint(side).zone;

                if !self.config.allow_cycles && visited.contains(&exit_zone) {
                    continue;
                }

                visited.insert(exit_zone);

                let step = TraversalStep {
                    portal_id: portal.id,
                    side,
                    entered_zone: exit_zone,
                    depth: depth + 1,
                };

                let mut new_path =
                    TraversalPath::new(TraversalId::from_raw(next_path_id), self.start_zone);
                next_path_id += 1;

                for s in &path.steps {
                    let Some(p) = graph.portal(s.portal_id) else {
                        continue;
                    };
                    new_path.push_step(*s, p.transform(s.side));
                }
                let portal_transform = portal.transform(side);
                new_path.push_step(step, portal_transform);

                queue.push_back((exit_zone, depth + 1, new_path));
            }
        }

        result.visited_zones = visited;
        result.stats.zones_visited = result.visited_zones.len() as u32;
        result.stats.paths_found = result.paths.len() as u32;
        result
    }
}

/// Query for the shortest path between two zones.
#[derive(Clone, Debug)]
pub struct PathfindQuery {
    /// Starting zone.
    pub start_zone: ZoneId,
    /// Target zone.
    pub target_zone: ZoneId,
    /// Configuration.
    pub config: TraversalConfig,
}

impl PathfindQuery {
    /// Create a new pathfinding query.
    #[must_use]
    pub fn new(start_zone: ZoneId, target_zone: ZoneId) -> Self {
        Self {
            start_zone,
            target_zone,
            config: TraversalConfig::default(),
        }
    }

    /// Set max depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Execute the query and return the shortest path if found.
    #[must_use]
    pub fn execute(&self, graph: &PortalGraph) -> Option<TraversalPath> {
        if self.start_zone == self.target_zone {
            return Some(TraversalPath::new(
                TraversalId::from_raw(0),
                self.start_zone,
            ));
        }

        let mut visited = BTreeSet::new();
        let mut queue: VecDeque<(ZoneId, u32, TraversalPath)> = VecDeque::new();
        let mut next_path_id: u64 = 0;

        let initial_path = TraversalPath::new(TraversalId::from_raw(next_path_id), self.start_zone);
        next_path_id += 1;
        visited.insert(self.start_zone);
        queue.push_back((self.start_zone, 0, initial_path));

        while let Some((zone, depth, path)) = queue.pop_front() {
            if depth >= self.config.max_depth {
                continue;
            }

            for portal in graph.zone_portals(zone) {
                let Some(side) = graph.portal_side_from_zone(portal.id, zone) else {
                    continue;
                };

                if !portal.can_traverse(side) {
                    continue;
                }

                let exit_zone = portal.exit_endpoint(side).zone;

                if visited.contains(&exit_zone) {
                    continue;
                }

                visited.insert(exit_zone);

                let step = TraversalStep {
                    portal_id: portal.id,
                    side,
                    entered_zone: exit_zone,
                    depth: depth + 1,
                };

                let mut new_path =
                    TraversalPath::new(TraversalId::from_raw(next_path_id), self.start_zone);
                next_path_id += 1;

                for s in &path.steps {
                    let Some(p) = graph.portal(s.portal_id) else {
                        continue;
                    };
                    new_path.push_step(*s, p.transform(s.side));
                }
                let portal_transform = portal.transform(side);
                new_path.push_step(step, portal_transform);

                if exit_zone == self.target_zone {
                    return Some(new_path);
                }

                queue.push_back((exit_zone, depth + 1, new_path));
            }
        }

        None
    }
}

/// Zones reachable from a point within a given number of portal crossings.
#[derive(Clone, Debug)]
pub struct ZoneDistanceMap {
    /// Starting zone.
    pub start_zone: ZoneId,
    /// Distance (portal crossings) to each reachable zone.
    pub distances: BTreeMap<ZoneId, u32>,
}

impl ZoneDistanceMap {
    /// Compute zone distances from a starting zone.
    #[must_use]
    pub fn compute(graph: &PortalGraph, start_zone: ZoneId, max_depth: u32) -> Self {
        let mut distances = BTreeMap::new();
        let mut queue = VecDeque::new();

        distances.insert(start_zone, 0);
        queue.push_back((start_zone, 0u32));

        while let Some((zone, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for portal in graph.zone_portals(zone) {
                let Some(side) = graph.portal_side_from_zone(portal.id, zone) else {
                    continue;
                };

                if !portal.can_traverse(side) {
                    continue;
                }

                let exit_zone = portal.exit_endpoint(side).zone;

                if distances.contains_key(&exit_zone) {
                    continue;
                }

                distances.insert(exit_zone, depth + 1);
                queue.push_back((exit_zone, depth + 1));
            }
        }

        Self {
            start_zone,
            distances,
        }
    }

    /// Get distance to a zone.
    #[must_use]
    pub fn distance_to(&self, zone: ZoneId) -> Option<u32> {
        self.distances.get(&zone).copied()
    }

    /// Check if a zone is reachable.
    #[must_use]
    pub fn is_reachable(&self, zone: ZoneId) -> bool {
        self.distances.contains_key(&zone)
    }

    /// Get all zones at a specific distance.
    #[must_use]
    pub fn zones_at_distance(&self, distance: u32) -> Vec<ZoneId> {
        self.distances
            .iter()
            .filter(|&(_, d)| *d == distance)
            .map(|(&z, _)| z)
            .collect()
    }

    /// Get the maximum distance to any reachable zone.
    #[must_use]
    pub fn max_distance(&self) -> u32 {
        self.distances.values().copied().max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::endpoint::PortalEndpoint;
    use crate::portal::graph::ZoneMetadata;
    use crate::portal::portal::Portal;

    #[expect(clippy::cast_precision_loss, reason = "small test indices")]
    fn linear_graph() -> PortalGraph {
        let mut graph = PortalGraph::new();
        for i in 0..4 {
            graph.add_zone(ZoneId::new(0, i), ZoneMetadata::new());
        }

        for i in 0..3 {
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
        graph
    }

    #[test]
    fn reachability_finds_zones() {
        let graph = linear_graph();
        let result = ReachabilityQuery::new(ZoneId::new(0, 0))
            .with_max_depth(10)
            .execute(&graph);

        assert!(result.reached(ZoneId::new(0, 1)));
        assert!(result.reached(ZoneId::new(0, 2)));
        assert!(result.reached(ZoneId::new(0, 3)));
        assert_eq!(result.stats.zones_visited, 4);
    }

    #[test]
    fn reachability_respects_depth() {
        let graph = linear_graph();
        let result = ReachabilityQuery::new(ZoneId::new(0, 0))
            .with_max_depth(1)
            .execute(&graph);

        assert!(result.reached(ZoneId::new(0, 1)));
        assert!(!result.reached(ZoneId::new(0, 2)));
    }

    #[test]
    fn pathfind_finds_path() {
        let graph = linear_graph();
        let path = PathfindQuery::new(ZoneId::new(0, 0), ZoneId::new(0, 3)).execute(&graph);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.depth(), 3);
        assert_eq!(path.end_zone, ZoneId::new(0, 3));
    }

    #[test]
    fn pathfind_same_zone() {
        let graph = linear_graph();
        let path = PathfindQuery::new(ZoneId::new(0, 0), ZoneId::new(0, 0)).execute(&graph);

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.is_empty());
    }

    #[test]
    fn pathfind_unreachable() {
        let graph = linear_graph();
        let path = PathfindQuery::new(ZoneId::new(0, 0), ZoneId::new(0, 3))
            .with_max_depth(2)
            .execute(&graph);

        assert!(path.is_none());
    }

    #[test]
    fn distance_map() {
        let graph = linear_graph();
        let map = ZoneDistanceMap::compute(&graph, ZoneId::new(0, 0), 10);

        assert_eq!(map.distance_to(ZoneId::new(0, 0)), Some(0));
        assert_eq!(map.distance_to(ZoneId::new(0, 1)), Some(1));
        assert_eq!(map.distance_to(ZoneId::new(0, 2)), Some(2));
        assert_eq!(map.distance_to(ZoneId::new(0, 3)), Some(3));
        assert_eq!(map.max_distance(), 3);
    }

    #[test]
    fn distance_map_zones_at_distance() {
        let graph = linear_graph();
        let map = ZoneDistanceMap::compute(&graph, ZoneId::new(0, 0), 10);

        let at_1 = map.zones_at_distance(1);
        assert_eq!(at_1.len(), 1);
        assert!(at_1.contains(&ZoneId::new(0, 1)));
    }

    #[test]
    fn traversal_path_transform() {
        let graph = linear_graph();
        let path = PathfindQuery::new(ZoneId::new(0, 0), ZoneId::new(0, 1))
            .execute(&graph)
            .unwrap();

        let point = Vec3::ZERO;
        let transformed = path.transform_point(point);
        assert!(transformed.x > 0.0);
    }

    #[test]
    fn traversal_path_zones_visited() {
        let graph = linear_graph();
        let path = PathfindQuery::new(ZoneId::new(0, 0), ZoneId::new(0, 2))
            .execute(&graph)
            .unwrap();

        let zones = path.zones_visited();
        assert_eq!(zones.len(), 3);
        assert_eq!(zones[0], ZoneId::new(0, 0));
        assert_eq!(zones[1], ZoneId::new(0, 1));
        assert_eq!(zones[2], ZoneId::new(0, 2));
    }

    #[test]
    fn serde_roundtrip() {
        let config = TraversalConfig::with_max_depth(8).with_cycles();
        let json = serde_json::to_string(&config).unwrap();
        let recovered: TraversalConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }
}

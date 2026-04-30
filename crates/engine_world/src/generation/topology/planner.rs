//! Topology planner for deterministic topology generation.

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "deliberate rng-to-float conversions and array indexing"
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::annotation::{
    HazardType, MissionHook, ResourceType, TopologyAnnotation, TopologyAnnotations,
};
use super::cell::{CellQuery, CellState, TopologyCell};
use super::config::{ConfigError, TopologyConfig};
use super::fingerprint::{
    FingerprintBuilder, NodeData, SegmentData, TopologyChecksum, TopologyFingerprint,
};
use super::kind::TopologyKind;
use super::node::{NodeId, NodeRole, TopologyNode};
use super::query::{PathQuery, QueryResult};
use super::segment::{SegmentId, SegmentKind, TopologySegment};

/// Summary of a topology planner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlannerSummary {
    /// Topology kind.
    pub kind: TopologyKind,
    /// Number of nodes.
    pub node_count: usize,
    /// Number of segments.
    pub segment_count: usize,
    /// Number of annotations.
    pub annotation_count: usize,
    /// Whether there is an entry node.
    pub has_entry: bool,
    /// Whether there is an exit node.
    pub has_exit: bool,
    /// Maximum depth from entry.
    pub max_depth: u32,
    /// Total volume (approximate).
    pub total_volume: f32,
}

/// Deterministic topology planner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyPlanner {
    /// Configuration used for generation.
    config: TopologyConfig,
    /// Nodes indexed by ID.
    nodes: BTreeMap<NodeId, TopologyNode>,
    /// Segments indexed by ID.
    segments: BTreeMap<SegmentId, TopologySegment>,
    /// Adjacency list (node -> outgoing segments).
    adjacency: BTreeMap<NodeId, Vec<SegmentId>>,
    /// Annotations.
    annotations: TopologyAnnotations,
    /// Entry node ID.
    entry: Option<NodeId>,
    /// Exit node ID.
    exit: Option<NodeId>,
    /// Next node ID.
    next_node_id: u64,
    /// Next segment ID.
    next_segment_id: u64,
}

impl TopologyPlanner {
    /// Generate a topology from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn generate(config: &TopologyConfig) -> Result<Self, ConfigError> {
        config.validate()?;

        let mut planner = Self {
            config: config.clone(),
            nodes: BTreeMap::new(),
            segments: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            annotations: TopologyAnnotations::new(),
            entry: None,
            exit: None,
            next_node_id: 0,
            next_segment_id: 0,
        };

        planner.generate_topology(config);
        planner.assign_roles();
        planner.calculate_depths();

        if config.enable_hazards {
            planner.generate_hazards(config);
        }
        if config.enable_resources {
            planner.generate_resources(config);
        }
        if config.enable_mission_hooks {
            planner.generate_mission_hooks(config);
        }

        Ok(planner)
    }

    fn generate_topology(&mut self, config: &TopologyConfig) {
        let seed = config.seed;
        let node_count = config.node_count as usize;

        let mut rng_state = seed;
        let mut rng = || {
            rng_state = rng_state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            rng_state
        };

        let entry_id = self.add_node([0.0, 0.0, 0.0], config);
        self.entry = Some(entry_id);
        self.nodes.get_mut(&entry_id).unwrap().role = NodeRole::Entry;

        let mut current = entry_id;
        let mut depth = 0_u32;

        while self.nodes.len() < node_count {
            depth += 1;
            let should_branch = (rng() as f32 / u64::MAX as f32) < config.branch_probability;

            let direction_count = if should_branch { 2 } else { 1 };

            for _ in 0..direction_count {
                if self.nodes.len() >= node_count {
                    break;
                }

                let angle = (rng() % 360) as f32 * std::f32::consts::PI / 180.0;
                let distance = lerp(
                    config.min_width * 2.0,
                    config.max_width * 3.0,
                    rng() as f32 / u64::MAX as f32,
                );

                let current_node = self.nodes.get(&current).unwrap();
                let current_pos = current_node.position;

                let new_pos = match config.kind {
                    TopologyKind::Trench => {
                        let y_change = -((rng() % 10) as f32) - 5.0;
                        [
                            current_pos[0] + angle.cos() * distance,
                            current_pos[1] + y_change,
                            current_pos[2] + angle.sin() * distance,
                        ]
                    }
                    TopologyKind::HollowSphere => {
                        let phi = (rng() as f32 / u64::MAX as f32) * std::f32::consts::PI;
                        let theta = angle;
                        let r = config.max_width * 5.0;
                        [
                            r * phi.sin() * theta.cos(),
                            r * phi.cos(),
                            r * phi.sin() * theta.sin(),
                        ]
                    }
                    TopologyKind::IceTunnel | TopologyKind::StationDeck => [
                        current_pos[0] + angle.cos() * distance,
                        current_pos[1] + ((rng() % 3) as f32 - 1.0) * 3.0,
                        current_pos[2] + angle.sin() * distance,
                    ],
                };

                let new_id = self.add_node(new_pos, config);
                self.nodes.get_mut(&new_id).unwrap().depth = depth;
                self.add_segment(current, new_id, config, &mut rng);
            }

            let node_ids: Vec<_> = self.nodes.keys().copied().collect();
            if !node_ids.is_empty() {
                let idx = (rng() as usize) % node_ids.len();
                current = node_ids[idx];
            }

            if (rng() as f32 / u64::MAX as f32) < config.loop_probability && self.nodes.len() > 3 {
                let a_idx = (rng() as usize) % node_ids.len();
                let b_idx = (rng() as usize) % node_ids.len();
                if a_idx != b_idx {
                    let a = node_ids[a_idx];
                    let b = node_ids[b_idx];
                    if !self.are_connected(a, b) {
                        self.add_segment(a, b, config, &mut rng);
                    }
                }
            }
        }

        let mut max_depth_node = entry_id;
        let mut max_depth_val = 0;
        for (id, node) in &self.nodes {
            if node.depth > max_depth_val {
                max_depth_val = node.depth;
                max_depth_node = *id;
            }
        }
        self.exit = Some(max_depth_node);
        self.nodes.get_mut(&max_depth_node).unwrap().role = NodeRole::Exit;
    }

    fn add_node(&mut self, position: [f32; 3], config: &TopologyConfig) -> NodeId {
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        let rng_val = self.next_node_id.wrapping_mul(31337);
        let radius = lerp(
            config.min_width,
            config.max_width,
            rng_val as f32 / u64::MAX as f32,
        );
        let height = lerp(
            config.min_height,
            config.max_height,
            rng_val.wrapping_mul(65537) as f32 / u64::MAX as f32,
        );

        let node = TopologyNode::new(id, position, radius, height);
        self.nodes.insert(id, node);
        self.adjacency.insert(id, Vec::new());
        id
    }

    fn add_segment(
        &mut self,
        from: NodeId,
        to: NodeId,
        config: &TopologyConfig,
        rng: &mut impl FnMut() -> u64,
    ) -> SegmentId {
        let id = SegmentId::new(self.next_segment_id);
        self.next_segment_id += 1;

        let from_node = self.nodes.get(&from).unwrap();
        let to_node = self.nodes.get(&to).unwrap();

        let dx = to_node.position[0] - from_node.position[0];
        let dy = to_node.position[1] - from_node.position[1];
        let dz = to_node.position[2] - from_node.position[2];
        let length = (dx * dx + dy * dy + dz * dz).sqrt();

        let kind = match config.kind {
            TopologyKind::Trench => {
                if dy.abs() > length * 0.5 {
                    SegmentKind::Shaft
                } else {
                    SegmentKind::Open
                }
            }
            TopologyKind::IceTunnel => SegmentKind::Tunnel,
            TopologyKind::StationDeck => {
                let r = rng() % 10;
                if r < 3 {
                    SegmentKind::Airlock
                } else {
                    SegmentKind::Corridor
                }
            }
            TopologyKind::HollowSphere => SegmentKind::Bridge,
        };

        let width = lerp(
            config.min_width * 0.3,
            config.max_width * 0.5,
            rng() as f32 / u64::MAX as f32,
        );
        let height = lerp(
            config.min_height * 0.5,
            config.max_height * 0.7,
            rng() as f32 / u64::MAX as f32,
        );

        let segment = TopologySegment::new(id, from, to, length)
            .with_kind(kind)
            .with_dimensions(width, height);

        self.segments.insert(id, segment);
        self.adjacency.entry(from).or_default().push(id);
        self.adjacency.entry(to).or_default().push(id);

        id
    }

    fn assign_roles(&mut self) {
        for (id, segs) in &self.adjacency {
            let count = segs.len();
            if let Some(node) = self.nodes.get_mut(id)
                && node.role == NodeRole::Standard
            {
                node.role = match count {
                    0 | 1 => NodeRole::DeadEnd,
                    2 => NodeRole::Standard,
                    _ => NodeRole::Junction,
                };
            }
        }

        for node in self.nodes.values_mut() {
            if node.radius > self.config.max_width * 0.8
                && (node.role == NodeRole::Standard || node.role == NodeRole::Junction)
            {
                node.role = NodeRole::Chamber;
            }
        }
    }

    fn calculate_depths(&mut self) {
        let Some(entry) = self.entry else { return };

        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((entry, 0_u32));

        while let Some((node_id, depth)) = queue.pop_front() {
            if !visited.insert(node_id) {
                continue;
            }
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.depth = depth;
            }
            for neighbor in self.neighbor_ids(node_id) {
                if !visited.contains(&neighbor) {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }
    }

    fn generate_hazards(&mut self, config: &TopologyConfig) {
        let mut rng_state = config.seed.wrapping_add(111);
        let mut rng = || {
            rng_state = rng_state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            rng_state
        };

        let hazard_types = [
            HazardType::Unstable,
            HazardType::Toxic,
            HazardType::Freezing,
            HazardType::Radiation,
            HazardType::Creatures,
        ];

        for id in self.nodes.keys() {
            if (rng() as f32 / u64::MAX as f32) < config.hazard_probability {
                let hazard = hazard_types[(rng() as usize) % hazard_types.len()];
                let intensity = ((rng() % 8) + 3) as u8;
                self.annotations
                    .add(TopologyAnnotation::node_hazard(*id, hazard, intensity));
            }
        }

        for id in self.segments.keys() {
            if (rng() as f32 / u64::MAX as f32) < config.hazard_probability * 0.5 {
                let hazard = hazard_types[(rng() as usize) % hazard_types.len()];
                let intensity = ((rng() % 5) + 1) as u8;
                self.annotations
                    .add(TopologyAnnotation::segment_hazard(*id, hazard, intensity));
            }
        }
    }

    fn generate_resources(&mut self, config: &TopologyConfig) {
        let mut rng_state = config.seed.wrapping_add(222);
        let mut rng = || {
            rng_state = rng_state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            rng_state
        };

        let resource_types = [
            ResourceType::Ore,
            ResourceType::Crystal,
            ResourceType::Fuel,
            ResourceType::Water,
            ResourceType::Mineral,
            ResourceType::Salvage,
        ];

        for id in self.nodes.keys() {
            if (rng() as f32 / u64::MAX as f32) < config.resource_probability {
                let resource = resource_types[(rng() as usize) % resource_types.len()];
                let quantity = ((rng() % 100) + 10) as u32;
                self.annotations
                    .add(TopologyAnnotation::node_resource(*id, resource, quantity));
            }
        }
    }

    fn generate_mission_hooks(&mut self, config: &TopologyConfig) {
        let mut rng_state = config.seed.wrapping_add(333);
        let mut rng = || {
            rng_state = rng_state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            rng_state
        };

        let hook_types = [
            MissionHook::Pickup,
            MissionHook::Terminal,
            MissionHook::Encounter,
            MissionHook::Secret,
        ];

        for id in self.nodes.keys() {
            if (rng() as f32 / u64::MAX as f32) < config.mission_hook_probability {
                let hook = hook_types[(rng() as usize) % hook_types.len()];
                self.annotations
                    .add(TopologyAnnotation::node_mission(*id, hook));
            }
        }

        if let Some(exit) = self.exit {
            self.annotations.add(TopologyAnnotation::node_mission(
                exit,
                MissionHook::Objective,
            ));
        }
    }

    fn are_connected(&self, a: NodeId, b: NodeId) -> bool {
        self.adjacency.get(&a).is_some_and(|segs| {
            segs.iter()
                .any(|s| self.segments.get(s).is_some_and(|seg| seg.connects(a, b)))
        })
    }

    fn neighbor_ids(&self, node: NodeId) -> Vec<NodeId> {
        self.adjacency
            .get(&node)
            .map(|segs| {
                segs.iter()
                    .filter_map(|s| self.segments.get(s).and_then(|seg| seg.other_end(node)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the entry node ID.
    #[must_use]
    pub fn entry_node(&self) -> Option<NodeId> {
        self.entry
    }

    /// Get the exit node ID.
    #[must_use]
    pub fn exit_node(&self) -> Option<NodeId> {
        self.exit
    }

    /// Get a node by ID.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&TopologyNode> {
        self.nodes.get(&id)
    }

    /// Get a segment by ID.
    #[must_use]
    pub fn segment(&self, id: SegmentId) -> Option<&TopologySegment> {
        self.segments.get(&id)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &TopologyNode> {
        self.nodes.values()
    }

    /// Get all segments.
    pub fn segments(&self) -> impl Iterator<Item = &TopologySegment> {
        self.segments.values()
    }

    /// Get neighbor nodes of a node.
    #[must_use]
    pub fn neighbors(&self, node: NodeId) -> QueryResult<NodeId> {
        QueryResult::complete(self.neighbor_ids(node))
    }

    /// Get segments connected to a node.
    #[must_use]
    pub fn segments_from(&self, node: NodeId) -> QueryResult<SegmentId> {
        QueryResult::complete(self.adjacency.get(&node).cloned().unwrap_or_default())
    }

    /// Find shortest path between two nodes.
    #[must_use]
    pub fn shortest_path(&self, from: NodeId, to: NodeId) -> PathQuery {
        if from == to {
            return PathQuery::complete(vec![from], 0);
        }

        let mut dist: BTreeMap<NodeId, u32> = BTreeMap::new();
        let mut prev: BTreeMap<NodeId, NodeId> = BTreeMap::new();
        let mut queue = VecDeque::new();

        dist.insert(from, 0);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if current == to {
                break;
            }

            let current_dist = dist.get(&current).copied().unwrap_or(u32::MAX);

            for seg_id in self.adjacency.get(&current).unwrap_or(&Vec::new()) {
                let Some(seg) = self.segments.get(seg_id) else {
                    continue;
                };
                let Some(neighbor) = seg.other_end(current) else {
                    continue;
                };

                let new_dist = current_dist.saturating_add(seg.cost);
                if new_dist < dist.get(&neighbor).copied().unwrap_or(u32::MAX) {
                    dist.insert(neighbor, new_dist);
                    prev.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        let Some(&total_cost) = dist.get(&to) else {
            return PathQuery::empty();
        };

        let mut path = Vec::new();
        let mut current = to;
        while current != from {
            path.push(current);
            let Some(&p) = prev.get(&current) else {
                return PathQuery::empty();
            };
            current = p;
        }
        path.push(from);
        path.reverse();

        PathQuery::complete(path, total_cost)
    }

    /// Get nodes reachable from a starting node.
    #[must_use]
    pub fn reachable_from(&self, start: NodeId) -> QueryResult<NodeId> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            for neighbor in self.neighbor_ids(node) {
                if !visited.contains(&neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        QueryResult::complete(visited.into_iter().collect())
    }

    /// Get nodes at a specific depth.
    #[must_use]
    pub fn nodes_at_depth(&self, depth: u32) -> QueryResult<NodeId> {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.depth == depth)
            .map(|(id, _)| *id)
            .collect();
        QueryResult::complete(nodes)
    }

    /// Get dead end nodes.
    #[must_use]
    pub fn dead_ends(&self) -> QueryResult<NodeId> {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.role == NodeRole::DeadEnd)
            .map(|(id, _)| *id)
            .collect();
        QueryResult::complete(nodes)
    }

    /// Get junction nodes.
    #[must_use]
    pub fn junctions(&self) -> QueryResult<NodeId> {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.role == NodeRole::Junction)
            .map(|(id, _)| *id)
            .collect();
        QueryResult::complete(nodes)
    }

    /// Get chamber nodes.
    #[must_use]
    pub fn chambers(&self) -> QueryResult<NodeId> {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.role == NodeRole::Chamber)
            .map(|(id, _)| *id)
            .collect();
        QueryResult::complete(nodes)
    }

    /// Get annotations.
    #[must_use]
    pub fn annotations(&self) -> &TopologyAnnotations {
        &self.annotations
    }

    /// Sample a cell at a position.
    #[must_use]
    pub fn sample_cell(&self, world_pos: [f32; 3]) -> TopologyCell {
        let query = CellQuery::new(self.config.cell_size);
        let grid_pos = query.world_to_grid(world_pos);

        for (id, node) in &self.nodes {
            if node.contains(world_pos) {
                return TopologyCell::open(grid_pos)
                    .with_node(*id)
                    .with_state(CellState::Open);
            }
        }

        for (id, seg) in &self.segments {
            let from = self.nodes.get(&seg.from);
            let to = self.nodes.get(&seg.to);
            if let (Some(from_node), Some(to_node)) = (from, to)
                && is_point_in_segment(world_pos, from_node, to_node, seg)
            {
                return TopologyCell::open(grid_pos)
                    .with_segment(*id)
                    .with_state(CellState::Open);
            }
        }

        TopologyCell::solid(grid_pos)
    }

    /// Get a summary of the topology.
    #[must_use]
    pub fn summary(&self) -> PlannerSummary {
        let max_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);
        let total_volume: f32 = self.nodes.values().map(TopologyNode::volume).sum::<f32>()
            + self
                .segments
                .values()
                .map(TopologySegment::volume)
                .sum::<f32>();

        PlannerSummary {
            kind: self.config.kind,
            node_count: self.nodes.len(),
            segment_count: self.segments.len(),
            annotation_count: self.annotations.len(),
            has_entry: self.entry.is_some(),
            has_exit: self.exit.is_some(),
            max_depth,
            total_volume,
        }
    }

    /// Compute a fingerprint of the topology.
    #[must_use]
    pub fn fingerprint(&self) -> TopologyFingerprint {
        let mut builder = FingerprintBuilder::new();
        builder.feed_u64(self.config.seed);
        builder.feed_u8(self.config.kind.as_raw());
        builder.feed_u32(self.nodes.len() as u32);
        builder.feed_u32(self.segments.len() as u32);

        for node in self.nodes.values() {
            builder.feed_node(&NodeData {
                id: node.id.value(),
                role: node.role.as_raw(),
                x: node.position[0],
                y: node.position[1],
                z: node.position[2],
                radius: node.radius,
                height: node.height,
                depth: node.depth,
            });
        }

        for seg in self.segments.values() {
            builder.feed_segment(&SegmentData {
                id: seg.id.value(),
                from: seg.from.value(),
                to: seg.to.value(),
                kind: seg.kind.as_raw(),
                width: seg.width,
                height: seg.height,
                length: seg.length,
                bidirectional: seg.bidirectional,
                cost: seg.cost,
            });
        }

        builder.build()
    }

    /// Compute a checksum of the topology.
    #[must_use]
    pub fn checksum(&self) -> TopologyChecksum {
        let structure = self.fingerprint().value();

        let mut builder = FingerprintBuilder::new();
        builder.feed_u32(self.annotations.len() as u32);
        for ann in self.annotations.all() {
            match ann {
                TopologyAnnotation::NodeHazard {
                    node,
                    hazard,
                    intensity,
                } => {
                    builder.feed_u8(0);
                    builder.feed_u64(node.value());
                    builder.feed_u8(hazard.as_raw());
                    builder.feed_u8(*intensity);
                }
                TopologyAnnotation::SegmentHazard {
                    segment,
                    hazard,
                    intensity,
                } => {
                    builder.feed_u8(1);
                    builder.feed_u64(segment.value());
                    builder.feed_u8(hazard.as_raw());
                    builder.feed_u8(*intensity);
                }
                TopologyAnnotation::NodeResource {
                    node,
                    resource,
                    quantity,
                } => {
                    builder.feed_u8(2);
                    builder.feed_u64(node.value());
                    builder.feed_u8(resource.as_raw());
                    builder.feed_u32(*quantity);
                }
                TopologyAnnotation::NodeMission { node, hook } => {
                    builder.feed_u8(3);
                    builder.feed_u64(node.value());
                    builder.feed_u8(hook.as_raw());
                }
            }
        }
        let annotations = builder.build().value();

        TopologyChecksum::new(structure, annotations)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn is_point_in_segment(
    point: [f32; 3],
    from_node: &TopologyNode,
    to_node: &TopologyNode,
    seg: &TopologySegment,
) -> bool {
    let fx = from_node.position[0];
    let fy = from_node.position[1];
    let fz = from_node.position[2];
    let tx = to_node.position[0];
    let ty = to_node.position[1];
    let tz = to_node.position[2];

    let dx = tx - fx;
    let dy = ty - fy;
    let dz = tz - fz;
    let len_sq = dx * dx + dy * dy + dz * dz;

    if len_sq < 0.001 {
        return false;
    }

    let t = ((point[0] - fx) * dx + (point[1] - fy) * dy + (point[2] - fz) * dz) / len_sq;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }

    let closest = [fx + t * dx, fy + t * dy, fz + t * dz];
    let dist_sq = (point[0] - closest[0]).powi(2)
        + (point[1] - closest[1]).powi(2)
        + (point[2] - closest[2]).powi(2);

    let half_width = seg.width / 2.0;
    dist_sq <= half_width * half_width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_trench() {
        let config = TopologyConfig::small(42, TopologyKind::Trench);
        let planner = TopologyPlanner::generate(&config).unwrap();

        assert!(planner.entry_node().is_some());
        assert!(planner.exit_node().is_some());
        assert!(planner.nodes().count() >= 2);
        assert!(planner.segments().count() >= 1);
    }

    #[test]
    fn generate_ice_tunnel() {
        let config = TopologyConfig::small(42, TopologyKind::IceTunnel);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let summary = planner.summary();
        assert_eq!(summary.kind, TopologyKind::IceTunnel);
        assert!(summary.has_entry);
        assert!(summary.has_exit);
    }

    #[test]
    fn generate_station_deck() {
        let config = TopologyConfig::small(42, TopologyKind::StationDeck);
        let planner = TopologyPlanner::generate(&config).unwrap();

        assert!(planner.entry_node().is_some());
        assert!(
            planner
                .segments()
                .any(|s| s.kind == SegmentKind::Corridor || s.kind == SegmentKind::Airlock)
        );
    }

    #[test]
    fn generate_hollow_sphere() {
        let config = TopologyConfig::small(42, TopologyKind::HollowSphere);
        let planner = TopologyPlanner::generate(&config).unwrap();

        assert!(planner.entry_node().is_some());
        assert!(planner.exit_node().is_some());
    }

    #[test]
    fn deterministic_generation() {
        let config1 = TopologyConfig::medium(12345, TopologyKind::Trench);
        let planner1 = TopologyPlanner::generate(&config1).unwrap();

        let config2 = TopologyConfig::medium(12345, TopologyKind::Trench);
        let planner2 = TopologyPlanner::generate(&config2).unwrap();

        assert!(planner1.fingerprint().matches(&planner2.fingerprint()));
        assert!(planner1.checksum().matches(&planner2.checksum()));
    }

    #[test]
    fn different_seeds_different_results() {
        let config1 = TopologyConfig::small(111, TopologyKind::IceTunnel);
        let planner1 = TopologyPlanner::generate(&config1).unwrap();

        let config2 = TopologyConfig::small(222, TopologyKind::IceTunnel);
        let planner2 = TopologyPlanner::generate(&config2).unwrap();

        assert!(!planner1.fingerprint().matches(&planner2.fingerprint()));
    }

    #[test]
    fn shortest_path() {
        let config = TopologyConfig::small(42, TopologyKind::Trench);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let entry = planner.entry_node().unwrap();
        let exit = planner.exit_node().unwrap();

        let path = planner.shortest_path(entry, exit);
        assert!(path.complete);
        assert!(!path.is_empty());
        assert_eq!(path.start(), Some(entry));
        assert_eq!(path.end(), Some(exit));
    }

    #[test]
    fn reachable_from_entry() {
        let config = TopologyConfig::small(42, TopologyKind::Trench);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let entry = planner.entry_node().unwrap();
        let reachable = planner.reachable_from(entry);

        assert!(reachable.contains(&entry));
        assert!(planner.exit_node().is_some_and(|e| reachable.contains(&e)));
    }

    #[test]
    fn query_helpers() {
        let config =
            TopologyConfig::medium(42, TopologyKind::IceTunnel).with_branch_probability(0.5);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let dead_ends = planner.dead_ends();
        let junctions = planner.junctions();

        assert!(dead_ends.len() + junctions.len() <= planner.nodes().count());
    }

    #[test]
    fn annotations_generated() {
        let config = TopologyConfig::medium(42, TopologyKind::StationDeck)
            .with_hazards(true, 0.3)
            .with_resources(true, 0.3)
            .with_mission_hooks(true, 0.2);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let annotations = planner.annotations();
        assert!(!annotations.is_empty());
        assert!(annotations.hazards().count() > 0 || annotations.resources().count() > 0);
    }

    #[test]
    fn sample_cell() {
        let config = TopologyConfig::small(42, TopologyKind::Trench);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let entry = planner.entry_node().unwrap();
        let entry_node = planner.node(entry).unwrap();

        let cell = planner.sample_cell(entry_node.position);
        assert_eq!(cell.state, CellState::Open);
        assert_eq!(cell.node, Some(entry));

        let far_cell = planner.sample_cell([10000.0, 10000.0, 10000.0]);
        assert_eq!(far_cell.state, CellState::Solid);
    }

    #[test]
    fn validation_errors() {
        let bad_config = TopologyConfig::new(1, TopologyKind::Trench).with_node_count(1);
        let result = TopologyPlanner::generate(&bad_config);
        assert!(result.is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let config = TopologyConfig::small(42, TopologyKind::IceTunnel);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let json = serde_json::to_string(&planner).unwrap();
        let recovered: TopologyPlanner = serde_json::from_str(&json).unwrap();

        assert!(planner.fingerprint().matches(&recovered.fingerprint()));
        assert!(planner.checksum().matches(&recovered.checksum()));
    }

    #[test]
    fn fingerprint_stable() {
        let config = TopologyConfig::medium(99999, TopologyKind::HollowSphere);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let fp1 = planner.fingerprint();
        let fp2 = planner.fingerprint();
        let fp3 = planner.fingerprint();

        assert!(fp1.matches(&fp2));
        assert!(fp2.matches(&fp3));
    }

    #[test]
    fn checksum_components() {
        let config = TopologyConfig::small(42, TopologyKind::Trench).with_hazards(true, 0.5);
        let planner = TopologyPlanner::generate(&config).unwrap();

        let checksum = planner.checksum();

        let config_no_hazards =
            TopologyConfig::small(42, TopologyKind::Trench).with_hazards(false, 0.0);
        let planner_no_hazards = TopologyPlanner::generate(&config_no_hazards).unwrap();

        let checksum_no_hazards = planner_no_hazards.checksum();

        assert!(checksum.structure_matches(&checksum_no_hazards));
        assert!(!checksum.matches(&checksum_no_hazards));
    }
}

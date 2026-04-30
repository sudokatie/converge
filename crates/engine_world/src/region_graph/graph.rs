//! Core region graph structure with generation and queries.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

use super::annotation::{HazardAnnotation, ResourceAnnotation};
use super::config::RegionGraphConfig;
use super::edge::RegionEdge;
use super::edge_id::EdgeId;
use super::edge_kind::EdgeKind;
use super::fingerprint::{FingerprintBuilder, GraphChecksum, GraphFingerprint};
use super::gate::{GateRequirement, ProgressionTier};
use super::region::RegionNode;
use super::region_id::RegionId;
use super::region_kind::{RegionKind, RegionTag};

/// A region graph representing macro-scale structure and progression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionGraph {
    /// Graph configuration.
    config: RegionGraphConfig,
    /// Region nodes indexed by ID.
    regions: BTreeMap<RegionId, RegionNode>,
    /// Edges indexed by ID.
    edges: BTreeMap<EdgeId, RegionEdge>,
    /// Adjacency list: region -> outgoing edges.
    adjacency: BTreeMap<RegionId, BTreeSet<EdgeId>>,
    /// Spawn region ID.
    spawn: Option<RegionId>,
    /// Goal region ID.
    goal: Option<RegionId>,
    /// Critical path (ordered region IDs).
    critical_path: Vec<RegionId>,
    /// Next region sequence number.
    next_region_seq: u32,
    /// Next edge sequence number.
    next_edge_seq: u32,
}

impl RegionGraph {
    /// Create a new empty graph.
    #[must_use]
    pub fn new(config: RegionGraphConfig) -> Self {
        Self {
            config,
            regions: BTreeMap::new(),
            edges: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            spawn: None,
            goal: None,
            critical_path: Vec::new(),
            next_region_seq: 0,
            next_edge_seq: 0,
        }
    }

    /// Generate a graph from configuration.
    #[must_use]
    pub fn generate(config: RegionGraphConfig) -> Self {
        let mut graph = Self::new(config);
        graph.do_generate();
        graph
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &RegionGraphConfig {
        &self.config
    }

    /// Get the spawn region.
    #[must_use]
    pub fn spawn(&self) -> Option<RegionId> {
        self.spawn
    }

    /// Get the goal region.
    #[must_use]
    pub fn goal(&self) -> Option<RegionId> {
        self.goal
    }

    /// Get the critical path.
    #[must_use]
    pub fn critical_path(&self) -> &[RegionId] {
        &self.critical_path
    }

    /// Get region count.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get edge count.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get a region by ID.
    #[must_use]
    pub fn region(&self, id: RegionId) -> Option<&RegionNode> {
        self.regions.get(&id)
    }

    /// Get a mutable region by ID.
    pub fn region_mut(&mut self, id: RegionId) -> Option<&mut RegionNode> {
        self.regions.get_mut(&id)
    }

    /// Get an edge by ID.
    #[must_use]
    pub fn edge(&self, id: EdgeId) -> Option<&RegionEdge> {
        self.edges.get(&id)
    }

    /// Get a mutable edge by ID.
    pub fn edge_mut(&mut self, id: EdgeId) -> Option<&mut RegionEdge> {
        self.edges.get_mut(&id)
    }

    /// Iterate all regions in deterministic order.
    pub fn regions(&self) -> impl Iterator<Item = &RegionNode> {
        self.regions.values()
    }

    /// Iterate all edges in deterministic order.
    pub fn edges(&self) -> impl Iterator<Item = &RegionEdge> {
        self.edges.values()
    }

    /// Add a region to the graph.
    pub fn add_region(&mut self, mut region: RegionNode) -> RegionId {
        let id = self.allocate_region_id();
        region.id = id;
        self.adjacency.insert(id, BTreeSet::new());
        self.regions.insert(id, region);
        id
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, mut edge: RegionEdge) -> EdgeId {
        let id = self.allocate_edge_id();
        edge.id = id;

        if let Some(adj) = self.adjacency.get_mut(&edge.from) {
            adj.insert(id);
        }
        if edge.bidirectional
            && let Some(adj) = self.adjacency.get_mut(&edge.to)
        {
            adj.insert(id);
        }

        self.edges.insert(id, edge);
        id
    }

    /// Get neighbors of a region.
    #[must_use]
    pub fn neighbors(&self, region: RegionId) -> Vec<RegionId> {
        let mut neighbors = BTreeSet::new();
        if let Some(edge_ids) = self.adjacency.get(&region) {
            for edge_id in edge_ids {
                if let Some(edge) = self.edges.get(edge_id)
                    && let Some(dest) = edge.destination_from(region)
                {
                    neighbors.insert(dest);
                }
            }
        }
        neighbors.into_iter().collect()
    }

    /// Get edges from a region.
    #[must_use]
    pub fn edges_from(&self, region: RegionId) -> Vec<&RegionEdge> {
        let mut result = Vec::new();
        if let Some(edge_ids) = self.adjacency.get(&region) {
            for edge_id in edge_ids {
                if let Some(edge) = self.edges.get(edge_id)
                    && edge.can_traverse_from(region)
                {
                    result.push(edge);
                }
            }
        }
        result.sort();
        result
    }

    /// Get accessible neighbors at a given tier.
    #[must_use]
    pub fn accessible_neighbors(&self, region: RegionId, tier: ProgressionTier) -> Vec<RegionId> {
        let mut neighbors = Vec::new();
        for edge in self.edges_from(region) {
            if !edge.is_accessible(tier) {
                continue;
            }
            if let Some(dest) = edge.destination_from(region)
                && let Some(dest_region) = self.regions.get(&dest)
                && dest_region.is_accessible(tier)
            {
                neighbors.push(dest);
            }
        }
        neighbors.sort();
        neighbors.dedup();
        neighbors
    }

    /// Find all regions reachable from a starting region.
    #[must_use]
    pub fn reachable_from(&self, start: RegionId, tier: ProgressionTier) -> BTreeSet<RegionId> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();

        if self.regions.contains_key(&start) {
            queue.push_back(start);
            visited.insert(start);
        }

        while let Some(current) = queue.pop_front() {
            for neighbor in self.accessible_neighbors(current, tier) {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        visited
    }

    /// Find shortest path between two regions.
    #[must_use]
    #[expect(
        clippy::items_after_statements,
        reason = "helper struct scoped to function"
    )]
    pub fn shortest_path(
        &self,
        from: RegionId,
        to: RegionId,
        tier: ProgressionTier,
    ) -> Option<Vec<RegionId>> {
        if from == to {
            return Some(vec![from]);
        }
        if !self.regions.contains_key(&from) || !self.regions.contains_key(&to) {
            return None;
        }

        #[derive(Eq, PartialEq)]
        struct State {
            cost: u32,
            region: RegionId,
        }

        impl Ord for State {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                other
                    .cost
                    .cmp(&self.cost)
                    .then_with(|| self.region.cmp(&other.region))
            }
        }

        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut dist: BTreeMap<RegionId, u32> = BTreeMap::new();
        let mut prev: BTreeMap<RegionId, RegionId> = BTreeMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(from, 0);
        heap.push(State {
            cost: 0,
            region: from,
        });

        while let Some(State { cost, region }) = heap.pop() {
            if region == to {
                let mut path = vec![to];
                let mut current = to;
                while let Some(&p) = prev.get(&current) {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some(path);
            }

            if cost > *dist.get(&region).unwrap_or(&u32::MAX) {
                continue;
            }

            for edge in self.edges_from(region) {
                if let Some(edge_cost) = edge.effective_cost(tier)
                    && let Some(next) = edge.destination_from(region)
                    && let Some(next_region) = self.regions.get(&next)
                {
                    if !next_region.is_accessible(tier) {
                        continue;
                    }
                    let next_cost = cost + edge_cost;
                    if next_cost < *dist.get(&next).unwrap_or(&u32::MAX) {
                        dist.insert(next, next_cost);
                        prev.insert(next, region);
                        heap.push(State {
                            cost: next_cost,
                            region: next,
                        });
                    }
                }
            }
        }

        None
    }

    /// Get regions by tier.
    #[must_use]
    pub fn regions_by_tier(&self, tier: ProgressionTier) -> Vec<RegionId> {
        self.regions
            .iter()
            .filter(|(_, r)| r.tier == tier)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get tier summary.
    #[must_use]
    pub fn tier_summary(&self) -> BTreeMap<ProgressionTier, TierSummary> {
        let mut summaries: BTreeMap<ProgressionTier, TierSummary> = BTreeMap::new();

        for region in self.regions.values() {
            let summary = summaries.entry(region.tier).or_default();
            summary.region_count += 1;

            if region.is_critical() {
                summary.critical_count += 1;
            }
            if region.is_dead_end() {
                summary.dead_end_count += 1;
            }
            if region.is_branch() {
                summary.branch_count += 1;
            }

            summary.total_resources += region.total_resources();
            if region.has_active_hazards() {
                summary.hazard_count += 1;
            }
        }

        summaries
    }

    /// Get dead end regions.
    #[must_use]
    pub fn dead_ends(&self) -> Vec<RegionId> {
        self.regions
            .iter()
            .filter(|(_, r)| r.is_dead_end())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get branch point regions.
    #[must_use]
    pub fn branch_points(&self) -> Vec<RegionId> {
        self.regions
            .iter()
            .filter(|(_, r)| r.is_branch())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get chokepoint regions.
    #[must_use]
    pub fn chokepoints(&self) -> Vec<RegionId> {
        self.regions
            .iter()
            .filter(|(_, r)| r.is_chokepoint())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Compute graph fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> GraphFingerprint {
        let mut builder = FingerprintBuilder::new();

        builder.feed_u64(self.config.seed);
        builder.feed_u32(self.config.region_count);

        for (id, region) in &self.regions {
            builder.feed_region(
                id.raw(),
                region.kind.as_raw(),
                region.tier.value(),
                region.tags.len(),
            );
        }

        for (id, edge) in &self.edges {
            builder.feed_edge(
                id.raw(),
                edge.from.raw(),
                edge.to.raw(),
                edge.kind.as_raw(),
                edge.cost,
                edge.bidirectional,
            );
        }

        builder.build()
    }

    /// Compute graph checksum including state.
    #[must_use]
    pub fn checksum(&self) -> GraphChecksum {
        let structure = self.fingerprint().value();

        let mut state_builder = FingerprintBuilder::new();
        for region in self.regions.values() {
            state_builder.feed_bool(region.visited);
            state_builder.feed_bool(region.visible);
            if let Some(gate) = &region.gate {
                state_builder.feed_bool(gate.unlocked);
            }
        }
        for edge in self.edges.values() {
            state_builder.feed_bool(edge.passable);
            if let Some(gate) = &edge.gate {
                state_builder.feed_bool(gate.unlocked);
            }
        }
        let state = state_builder.build().value();

        GraphChecksum::new(structure, state)
    }

    /// Get graph summary.
    #[must_use]
    pub fn summary(&self) -> GraphSummary {
        GraphSummary {
            region_count: self.regions.len(),
            edge_count: self.edges.len(),
            tier_count: self
                .regions
                .values()
                .map(|r| r.tier.value())
                .max()
                .map_or(0, |t| t + 1),
            dead_end_count: self.dead_ends().len(),
            branch_count: self.branch_points().len(),
            chokepoint_count: self.chokepoints().len(),
            critical_path_length: self.critical_path.len(),
            has_spawn: self.spawn.is_some(),
            has_goal: self.goal.is_some(),
        }
    }

    fn allocate_region_id(&mut self) -> RegionId {
        #[expect(clippy::cast_possible_truncation, reason = "seed always fits in u32")]
        let id = RegionId::new(self.config.seed as u32, self.next_region_seq);
        self.next_region_seq += 1;
        id
    }

    fn allocate_edge_id(&mut self) -> EdgeId {
        #[expect(clippy::cast_possible_truncation, reason = "seed always fits in u32")]
        let id = EdgeId::new(self.config.seed as u32, self.next_edge_seq);
        self.next_edge_seq += 1;
        id
    }

    #[expect(clippy::too_many_lines, reason = "generation logic is cohesive")]
    fn do_generate(&mut self) {
        let seed = self.config.seed;
        let region_count = self.config.region_count as usize;
        let tier_count = self.config.tier_count;

        if region_count < 2 {
            return;
        }

        let mut rng = SimpleRng::new(seed);

        let spawn_id = self.add_region(
            RegionNode::new(RegionId::default(), RegionKind::Spawn)
                .with_name("Spawn")
                .with_tier(0)
                .with_tag(RegionTag::Critical)
                .with_tag(RegionTag::Safe),
        );
        self.spawn = Some(spawn_id);

        let goal_id = self.add_region(
            RegionNode::new(RegionId::default(), RegionKind::Goal)
                .with_name("Goal")
                .with_tier(tier_count.saturating_sub(1))
                .with_tag(RegionTag::Critical),
        );
        self.goal = Some(goal_id);

        let mut critical_ids = vec![spawn_id];

        let critical_count = if self.config.create_critical_path {
            (self.config.min_critical_path_length as usize)
                .min(region_count - 2)
                .max(1)
        } else {
            0
        };

        for i in 0..critical_count {
            #[expect(clippy::cast_possible_truncation, reason = "tier index always fits")]
            let tier = ((i + 1) * (tier_count as usize) / (critical_count + 1)) as u8;
            let kind = self.config.kind_weights.select(rng.next_u32());

            let region = RegionNode::new(RegionId::default(), kind)
                .with_name(format!("Critical-{}", i + 1))
                .with_tier(tier)
                .with_tag(RegionTag::Critical)
                .with_position(rng.next_i32_range(-100, 100), rng.next_i32_range(-100, 100));

            let id = self.add_region(region);
            critical_ids.push(id);
        }

        critical_ids.push(goal_id);

        for window in critical_ids.windows(2) {
            let from = window[0];
            let to = window[1];
            let edge_kind = Self::random_edge_kind(&mut rng);
            self.add_edge(RegionEdge::new(EdgeId::default(), from, to, edge_kind));
        }

        self.critical_path.clone_from(&critical_ids);

        let remaining = region_count.saturating_sub(critical_ids.len());
        for i in 0..remaining {
            #[expect(clippy::cast_possible_truncation, reason = "tier index always fits")]
            let tier = (rng.next_u32() % u32::from(tier_count)) as u8;
            let kind = self.config.kind_weights.select(rng.next_u32());

            let is_dead_end = rng.next_f32() < self.config.dead_end_probability;
            let is_branch = !is_dead_end && rng.next_f32() < self.config.branch_probability;

            let mut region = RegionNode::new(RegionId::default(), kind)
                .with_name(format!("Region-{}", i + 1))
                .with_tier(tier)
                .with_position(rng.next_i32_range(-100, 100), rng.next_i32_range(-100, 100));

            if is_dead_end {
                region.tags.insert(RegionTag::DeadEnd);
            }
            if is_branch {
                region.tags.insert(RegionTag::Branch);
            }

            if self.config.enable_hazards && rng.next_f32() < self.config.hazard_probability {
                let severity = (rng.next_u32() % 8) as u8 + 2;
                region
                    .hazards
                    .push(HazardAnnotation::new("generic", severity));
                region.tags.insert(RegionTag::Dangerous);
            }

            if self.config.enable_resources && rng.next_f32() < self.config.resource_probability {
                let quantity = (rng.next_u32() % 100) + 10;
                let quality = (rng.next_u32() % 4) as u8;
                region
                    .resources
                    .push(ResourceAnnotation::new("generic", quantity).with_quality(quality));
                region.tags.insert(RegionTag::Loot);
            }

            if tier > 0 && rng.next_f32() < 0.3 {
                region.gate = Some(GateRequirement::tier(tier));
                region.tags.insert(RegionTag::Locked);
            }

            let id = self.add_region(region);

            let connect_to = if critical_ids.is_empty() {
                spawn_id
            } else {
                let tier_regions: Vec<_> = self
                    .regions
                    .iter()
                    .filter(|(rid, r)| **rid != id && r.tier <= ProgressionTier::new(tier))
                    .map(|(rid, _)| *rid)
                    .collect();

                if tier_regions.is_empty() {
                    spawn_id
                } else {
                    let idx = (rng.next_u32() as usize) % tier_regions.len();
                    tier_regions[idx]
                }
            };

            let edge_kind = Self::random_edge_kind(&mut rng);
            self.add_edge(RegionEdge::new(
                EdgeId::default(),
                connect_to,
                id,
                edge_kind,
            ));

            if !is_dead_end && rng.next_f32() < self.config.loop_probability {
                let candidates: Vec<_> = self
                    .regions
                    .keys()
                    .filter(|rid| {
                        **rid != id && **rid != connect_to && !self.are_connected(id, **rid)
                    })
                    .copied()
                    .collect();

                if !candidates.is_empty() {
                    let idx = (rng.next_u32() as usize) % candidates.len();
                    let loop_to = candidates[idx];
                    let edge_kind = Self::random_edge_kind(&mut rng);
                    self.add_edge(RegionEdge::new(EdgeId::default(), id, loop_to, edge_kind));
                }
            }
        }

        self.compute_chokepoints();
    }

    fn random_edge_kind(rng: &mut SimpleRng) -> EdgeKind {
        let kinds = [
            EdgeKind::Path,
            EdgeKind::Corridor,
            EdgeKind::Tunnel,
            EdgeKind::Bridge,
            EdgeKind::Ladder,
        ];
        let idx = (rng.next_u32() as usize) % kinds.len();
        kinds[idx]
    }

    fn are_connected(&self, a: RegionId, b: RegionId) -> bool {
        if let Some(edges) = self.adjacency.get(&a) {
            for edge_id in edges {
                if let Some(edge) = self.edges.get(edge_id)
                    && edge.connects(a, b)
                {
                    return true;
                }
            }
        }
        false
    }

    fn compute_chokepoints(&mut self) {
        let region_ids: Vec<_> = self.regions.keys().copied().collect();

        for region_id in region_ids {
            let neighbor_count = self.neighbors(region_id).len();

            if neighbor_count == 2 {
                let is_on_critical_path = self.critical_path.contains(&region_id);
                let is_on_path_between_tiers = self.check_tier_gateway(region_id);

                if (is_on_critical_path || is_on_path_between_tiers)
                    && let Some(region) = self.regions.get_mut(&region_id)
                {
                    region.tags.insert(RegionTag::Chokepoint);
                }
            }
        }
    }

    fn check_tier_gateway(&self, region_id: RegionId) -> bool {
        let Some(region) = self.regions.get(&region_id) else {
            return false;
        };
        let my_tier = region.tier;

        let neighbors = self.neighbors(region_id);
        if neighbors.len() != 2 {
            return false;
        }

        let tiers: Vec<_> = neighbors
            .iter()
            .filter_map(|n| self.regions.get(n))
            .map(|r| r.tier)
            .collect();

        if tiers.len() == 2 {
            let different_tiers = tiers[0] != tiers[1];
            let connects_lower = tiers.iter().any(|t| *t < my_tier);
            let connects_higher = tiers.iter().any(|t| *t > my_tier);
            return different_tiers || connects_lower || connects_higher;
        }

        false
    }
}

impl Default for RegionGraph {
    fn default() -> Self {
        Self::new(RegionGraphConfig::default())
    }
}

/// Summary of a progression tier.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierSummary {
    /// Number of regions in this tier.
    pub region_count: usize,
    /// Number of critical path regions.
    pub critical_count: usize,
    /// Number of dead ends.
    pub dead_end_count: usize,
    /// Number of branch points.
    pub branch_count: usize,
    /// Total resource quantity.
    pub total_resources: u32,
    /// Number of hazardous regions.
    pub hazard_count: usize,
}

/// Overall graph summary.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSummary {
    /// Total region count.
    pub region_count: usize,
    /// Total edge count.
    pub edge_count: usize,
    /// Number of tiers.
    pub tier_count: u8,
    /// Number of dead ends.
    pub dead_end_count: usize,
    /// Number of branch points.
    pub branch_count: usize,
    /// Number of chokepoints.
    pub chokepoint_count: usize,
    /// Critical path length.
    pub critical_path_length: usize,
    /// Whether spawn exists.
    pub has_spawn: bool,
    /// Whether goal exists.
    pub has_goal: bool,
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(0x5851_F42D_4C95_7F2D);
        self.state = self.state.wrapping_add(0x1405_7B7E_F767_814F);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional truncation to u32"
    )]
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for random floats"
    )]
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    #[expect(
        clippy::cast_possible_wrap,
        reason = "range is small enough to fit in i32"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "max - min is always positive when max > min"
    )]
    fn next_i32_range(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u32;
        min + (self.next_u32() % range) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let graph = RegionGraph::new(RegionGraphConfig::new(42));
        assert_eq!(graph.region_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn generate_small_graph() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        assert!(graph.region_count() >= 2);
        assert!(graph.spawn().is_some());
        assert!(graph.goal().is_some());
        assert!(!graph.critical_path().is_empty());
    }

    #[test]
    fn generate_deterministic() {
        let config1 = RegionGraphConfig::medium(12345);
        let config2 = RegionGraphConfig::medium(12345);

        let graph1 = RegionGraph::generate(config1);
        let graph2 = RegionGraph::generate(config2);

        assert_eq!(graph1.region_count(), graph2.region_count());
        assert_eq!(graph1.edge_count(), graph2.edge_count());
        assert!(graph1.fingerprint().matches(&graph2.fingerprint()));
    }

    #[test]
    fn generate_different_seeds() {
        let graph1 = RegionGraph::generate(RegionGraphConfig::small(1));
        let graph2 = RegionGraph::generate(RegionGraphConfig::small(2));

        assert!(!graph1.fingerprint().matches(&graph2.fingerprint()));
    }

    #[test]
    fn neighbor_queries() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        let spawn = graph.spawn().unwrap();
        let neighbors = graph.neighbors(spawn);

        assert!(!neighbors.is_empty());
    }

    #[test]
    fn reachability() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        let spawn = graph.spawn().unwrap();
        let reachable = graph.reachable_from(spawn, ProgressionTier::MAX);

        assert!(reachable.contains(&spawn));
        if let Some(goal) = graph.goal() {
            assert!(reachable.contains(&goal));
        }
    }

    #[test]
    fn shortest_path_exists() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        let spawn = graph.spawn().unwrap();
        let goal = graph.goal().unwrap();

        let path = graph.shortest_path(spawn, goal, ProgressionTier::MAX);
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path.first(), Some(&spawn));
        assert_eq!(path.last(), Some(&goal));
    }

    #[test]
    fn shortest_path_same_node() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        let spawn = graph.spawn().unwrap();
        let path = graph.shortest_path(spawn, spawn, ProgressionTier::MAX);

        assert_eq!(path, Some(vec![spawn]));
    }

    #[test]
    fn tier_summary() {
        let config = RegionGraphConfig::medium(42);
        let graph = RegionGraph::generate(config);

        let summary = graph.tier_summary();
        assert!(!summary.is_empty());

        let total_regions: usize = summary.values().map(|s| s.region_count).sum();
        assert_eq!(total_regions, graph.region_count());
    }

    #[test]
    fn edge_symmetry() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        for edge in graph.edges() {
            if edge.bidirectional {
                assert!(edge.connects(edge.from, edge.to));
                assert!(edge.connects(edge.to, edge.from));
            } else {
                assert!(edge.connects(edge.from, edge.to));
                assert!(!edge.connects(edge.to, edge.from));
            }
        }
    }

    #[test]
    fn fingerprint_stability() {
        let config = RegionGraphConfig::medium(99999);
        let graph = RegionGraph::generate(config);

        let fp1 = graph.fingerprint();
        let fp2 = graph.fingerprint();

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn checksum_includes_state() {
        let config = RegionGraphConfig::small(42);
        let mut graph = RegionGraph::generate(config);

        let cs1 = graph.checksum();

        if let Some(spawn) = graph.spawn()
            && let Some(region) = graph.region_mut(spawn)
        {
            region.visit();
        }

        let cs2 = graph.checksum();

        assert!(cs1.structure_matches(&cs2));
        assert!(!cs1.matches(&cs2));
    }

    #[test]
    fn graph_summary() {
        let config = RegionGraphConfig::medium(42);
        let graph = RegionGraph::generate(config);

        let summary = graph.summary();
        assert_eq!(summary.region_count, graph.region_count());
        assert_eq!(summary.edge_count, graph.edge_count());
        assert!(summary.has_spawn);
        assert!(summary.has_goal);
    }

    #[test]
    fn serde_roundtrip() {
        let config = RegionGraphConfig::small(42);
        let graph = RegionGraph::generate(config);

        let json = serde_json::to_string(&graph).unwrap();
        let recovered: RegionGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(graph.region_count(), recovered.region_count());
        assert_eq!(graph.edge_count(), recovered.edge_count());
        assert!(graph.fingerprint().matches(&recovered.fingerprint()));
    }

    #[test]
    fn manual_graph_construction() {
        let config = RegionGraphConfig::new(1);
        let mut graph = RegionGraph::new(config);

        let r1 = graph.add_region(
            RegionNode::new(RegionId::default(), RegionKind::Station).with_name("Station A"),
        );
        let r2 = graph.add_region(
            RegionNode::new(RegionId::default(), RegionKind::Station).with_name("Station B"),
        );

        graph.add_edge(RegionEdge::path(EdgeId::default(), r1, r2));

        assert_eq!(graph.region_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.are_connected(r1, r2));
    }

    #[test]
    fn accessibility_with_gates() {
        let config = RegionGraphConfig::new(1);
        let mut graph = RegionGraph::new(config);

        let r1 = graph
            .add_region(RegionNode::new(RegionId::default(), RegionKind::Station).with_tier(0));
        let r2 = graph.add_region(
            RegionNode::new(RegionId::default(), RegionKind::Station)
                .with_tier(2)
                .with_gate(GateRequirement::tier(2)),
        );

        graph.add_edge(RegionEdge::path(EdgeId::default(), r1, r2));

        let reachable_t0 = graph.reachable_from(r1, ProgressionTier::new(0));
        assert!(reachable_t0.contains(&r1));
        assert!(!reachable_t0.contains(&r2));

        let reachable_t2 = graph.reachable_from(r1, ProgressionTier::new(2));
        assert!(reachable_t2.contains(&r1));
        assert!(reachable_t2.contains(&r2));
    }

    #[test]
    fn dead_ends_and_branches() {
        let config = RegionGraphConfig::medium(42);
        let graph = RegionGraph::generate(config);

        let dead_ends = graph.dead_ends();
        let branches = graph.branch_points();

        for id in &dead_ends {
            let region = graph.region(*id).unwrap();
            assert!(region.is_dead_end());
        }

        for id in &branches {
            let region = graph.region(*id).unwrap();
            assert!(region.is_branch());
        }
    }

    #[test]
    fn critical_path_connectivity() {
        let config = RegionGraphConfig::small(42).with_critical_path(5);
        let graph = RegionGraph::generate(config);

        let critical_path = graph.critical_path();
        if critical_path.len() >= 2 {
            for window in critical_path.windows(2) {
                let from = window[0];
                let to = window[1];
                let neighbors = graph.neighbors(from);
                assert!(neighbors.contains(&to), "Critical path should be connected");
            }
        }
    }

    #[test]
    fn rng_determinism() {
        let mut rng1 = SimpleRng::new(42);
        let mut rng2 = SimpleRng::new(42);

        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn config_validation_in_generation() {
        let mut bad_config = RegionGraphConfig::new(1);
        bad_config.region_count = 1;
        let graph = RegionGraph::generate(bad_config);
        assert_eq!(graph.region_count(), 0);
    }
}

//! Behavior graph definition for block types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ChecksumBuilder;
use crate::chunk::BlockId;

use super::node::{BehaviorNode, NodeId};

/// Filter for matching block types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockFilter {
    /// Match a specific block ID.
    Exact(BlockId),
    /// Match any block in a set.
    Any(Vec<BlockId>),
    /// Match all blocks except those in a set.
    Except(Vec<BlockId>),
    /// Match all blocks.
    All,
}

impl BlockFilter {
    /// Check if a block matches this filter.
    #[must_use]
    pub fn matches(&self, block: BlockId) -> bool {
        match self {
            Self::Exact(id) => block == *id,
            Self::Any(ids) => ids.contains(&block),
            Self::Except(ids) => !ids.contains(&block),
            Self::All => true,
        }
    }

    /// Feed filter data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "lengths fit in u32")]
    pub fn feed_checksum(&self, hasher: &mut ChecksumBuilder) {
        match self {
            Self::Exact(id) => {
                hasher.feed_u32(0);
                hasher.feed_u32(u32::from(id.raw()));
            }
            Self::Any(ids) => {
                hasher.feed_u32(1);
                hasher.feed_u32(ids.len() as u32);
                for id in ids {
                    hasher.feed_u32(u32::from(id.raw()));
                }
            }
            Self::Except(ids) => {
                hasher.feed_u32(2);
                hasher.feed_u32(ids.len() as u32);
                for id in ids {
                    hasher.feed_u32(u32::from(id.raw()));
                }
            }
            Self::All => {
                hasher.feed_u32(3);
            }
        }
    }
}

/// Fingerprint for graph identity and change detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphFingerprint {
    value: u64,
}

impl GraphFingerprint {
    /// Create a fingerprint from raw value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self { value }
    }

    /// Get the raw fingerprint value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.value
    }
}

/// Complete behavior definition for a block type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BehaviorGraph {
    /// The block type this graph applies to.
    pub block_id: BlockId,
    /// Human-readable name for debugging.
    pub name: Option<String>,
    /// Graph version for compatibility checking.
    pub version: u32,
    /// Nodes in the graph, keyed by ID.
    nodes: HashMap<NodeId, BehaviorNode>,
    /// Cached sorted node order for deterministic evaluation.
    #[serde(skip)]
    sorted_nodes: Vec<NodeId>,
    /// Whether the sorted cache is valid.
    #[serde(skip)]
    cache_valid: bool,
}

impl BehaviorGraph {
    /// Create a new empty behavior graph for a block type.
    #[must_use]
    pub fn new(block_id: BlockId) -> Self {
        Self {
            block_id,
            name: None,
            version: 1,
            nodes: HashMap::new(),
            sorted_nodes: Vec::new(),
            cache_valid: false,
        }
    }

    /// Set the graph name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the graph version.
    #[must_use]
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: BehaviorNode) {
        self.nodes.insert(node.id, node);
        self.cache_valid = false;
    }

    /// Remove a node from the graph.
    pub fn remove_node(&mut self, id: NodeId) -> Option<BehaviorNode> {
        self.cache_valid = false;
        self.nodes.remove(&id)
    }

    /// Get a node by ID.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<&BehaviorNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable reference to a node.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut BehaviorNode> {
        self.cache_valid = false;
        self.nodes.get_mut(&id)
    }

    /// Get all nodes in the graph.
    pub fn nodes(&self) -> impl Iterator<Item = &BehaviorNode> {
        self.nodes.values()
    }

    /// Get nodes in deterministic evaluation order (priority desc, then ID asc).
    pub fn nodes_ordered(&mut self) -> impl Iterator<Item = &BehaviorNode> {
        self.ensure_sorted();
        self.sorted_nodes.iter().filter_map(|id| self.nodes.get(id))
    }

    /// Get the number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the graph is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn ensure_sorted(&mut self) {
        if self.cache_valid {
            return;
        }

        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort();
        self.sorted_nodes = nodes.into_iter().map(|n| n.id).collect();
        self.cache_valid = true;
    }

    /// Compute a fingerprint for this graph.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "node count fits in u32")]
    pub fn fingerprint(&mut self) -> GraphFingerprint {
        self.ensure_sorted();

        let mut hasher = ChecksumBuilder::new();
        hasher.feed_u32(u32::from(self.block_id.raw()));
        hasher.feed_u32(self.version);
        hasher.feed_u32(self.nodes.len() as u32);

        for id in &self.sorted_nodes {
            if let Some(node) = self.nodes.get(id) {
                node.feed_checksum(&mut hasher);
            }
        }

        let checksum = hasher.build();
        GraphFingerprint::from_raw(u64::from(checksum.value()))
    }

    /// Feed graph data into a checksum builder.
    #[expect(clippy::cast_possible_truncation, reason = "node count fits in u32")]
    pub fn feed_checksum(&mut self, hasher: &mut ChecksumBuilder) {
        self.ensure_sorted();

        hasher.feed_u32(u32::from(self.block_id.raw()));
        hasher.feed_u32(self.version);
        hasher.feed_u32(self.nodes.len() as u32);

        for id in &self.sorted_nodes {
            if let Some(node) = self.nodes.get(id) {
                node.feed_checksum(hasher);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior_graph::{BehaviorAction, BehaviorCondition, BehaviorTrigger};

    #[test]
    fn graph_creation() {
        let graph = BehaviorGraph::new(BlockId(100)).with_name("torch_behavior");

        assert_eq!(graph.block_id, BlockId(100));
        assert_eq!(graph.name, Some("torch_behavior".into()));
        assert!(graph.is_empty());
    }

    #[test]
    fn graph_add_remove_nodes() {
        let mut graph = BehaviorGraph::new(BlockId(1));

        graph.add_node(BehaviorNode::new(1).with_name("node_a"));
        graph.add_node(BehaviorNode::new(2).with_name("node_b"));

        assert_eq!(graph.node_count(), 2);
        assert!(graph.get_node(NodeId::new(1)).is_some());

        graph.remove_node(NodeId::new(1));
        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node(NodeId::new(1)).is_none());
    }

    #[test]
    fn graph_deterministic_order() {
        let mut graph = BehaviorGraph::new(BlockId(1));

        graph.add_node(BehaviorNode::new(3).with_priority(5));
        graph.add_node(BehaviorNode::new(1).with_priority(10));
        graph.add_node(BehaviorNode::new(2).with_priority(5));

        let ordered: Vec<_> = graph.nodes_ordered().map(|n| n.id.raw()).collect();
        assert_eq!(ordered, vec![1, 2, 3]);
    }

    #[test]
    fn graph_fingerprint_deterministic() {
        let mut graph1 = BehaviorGraph::new(BlockId(100));
        graph1.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 }),
        );

        let mut graph2 = BehaviorGraph::new(BlockId(100));
        graph2.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::Use)
                .with_action(BehaviorAction::PlaySound { sound_id: 1 }),
        );

        assert_eq!(graph1.fingerprint(), graph2.fingerprint());
    }

    #[test]
    fn graph_fingerprint_differs_on_change() {
        let mut graph1 = BehaviorGraph::new(BlockId(100));
        graph1.add_node(BehaviorNode::new(1).with_trigger(BehaviorTrigger::Use));

        let mut graph2 = BehaviorGraph::new(BlockId(100));
        graph2.add_node(BehaviorNode::new(1).with_trigger(BehaviorTrigger::Mine));

        assert_ne!(graph1.fingerprint(), graph2.fingerprint());
    }

    #[test]
    fn block_filter_exact() {
        let filter = BlockFilter::Exact(BlockId(5));
        assert!(filter.matches(BlockId(5)));
        assert!(!filter.matches(BlockId(6)));
    }

    #[test]
    fn block_filter_any() {
        let filter = BlockFilter::Any(vec![BlockId(1), BlockId(2), BlockId(3)]);
        assert!(filter.matches(BlockId(2)));
        assert!(!filter.matches(BlockId(5)));
    }

    #[test]
    fn block_filter_except() {
        let filter = BlockFilter::Except(vec![BlockId(1), BlockId(2)]);
        assert!(!filter.matches(BlockId(1)));
        assert!(filter.matches(BlockId(5)));
    }

    #[test]
    fn block_filter_all() {
        let filter = BlockFilter::All;
        assert!(filter.matches(BlockId(0)));
        assert!(filter.matches(BlockId(999)));
    }

    #[test]
    fn serde_round_trip() {
        let mut graph = BehaviorGraph::new(BlockId(42))
            .with_name("test_graph")
            .with_version(2);
        graph.add_node(
            BehaviorNode::new(1)
                .with_trigger(BehaviorTrigger::FluidContact { fluid_kind: 0 })
                .with_condition(BehaviorCondition::Always)
                .with_action(BehaviorAction::DestroyBlock),
        );

        let json = serde_json::to_string(&graph).unwrap();
        let recovered: BehaviorGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.block_id, graph.block_id);
        assert_eq!(recovered.name, graph.name);
        assert_eq!(recovered.version, graph.version);
        assert_eq!(recovered.node_count(), 1);
    }
}

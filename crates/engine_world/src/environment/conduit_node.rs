//! Network node types for conduit endpoints.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Serialize};

use super::ConduitKind;

/// Role of a node in the conduit network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeRole {
    /// Simple junction/passthrough.
    #[default]
    Junction = 0,
    /// Produces/injects resource into network.
    Source = 1,
    /// Consumes/extracts resource from network.
    Sink = 2,
    /// Stores resource with bidirectional flow.
    Storage = 3,
}

impl NodeRole {
    /// Number of node roles.
    pub const COUNT: usize = 4;

    /// All node roles in index order.
    pub const ALL: [NodeRole; Self::COUNT] = [
        NodeRole::Junction,
        NodeRole::Source,
        NodeRole::Sink,
        NodeRole::Storage,
    ];

    /// Convert to array index.
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    /// Create from array index.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(NodeRole::Junction),
            1 => Some(NodeRole::Source),
            2 => Some(NodeRole::Sink),
            3 => Some(NodeRole::Storage),
            _ => None,
        }
    }

    /// Display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            NodeRole::Junction => "Junction",
            NodeRole::Source => "Source",
            NodeRole::Sink => "Sink",
            NodeRole::Storage => "Storage",
        }
    }

    /// Whether this role can provide resource to the network.
    #[must_use]
    pub const fn can_provide(self) -> bool {
        matches!(self, NodeRole::Source | NodeRole::Storage)
    }

    /// Whether this role can accept resource from the network.
    #[must_use]
    pub const fn can_accept(self) -> bool {
        matches!(self, NodeRole::Sink | NodeRole::Storage)
    }
}

/// A node in the conduit network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConduitNode {
    /// Position within chunk.
    pub pos: LocalPos,
    /// Type of conduit.
    pub kind: ConduitKind,
    /// Role in the network.
    pub role: NodeRole,
    /// Production/consumption rate (per second).
    pub rate: f32,
    /// Current stored amount.
    pub stored: f32,
    /// Maximum storage capacity.
    pub capacity: f32,
    /// Priority for distribution (higher = earlier).
    pub priority: i8,
    /// Whether the node is currently enabled.
    pub enabled: bool,
}

impl ConduitNode {
    /// Create a junction node.
    #[must_use]
    pub fn junction(pos: LocalPos, kind: ConduitKind) -> Self {
        Self {
            pos,
            kind,
            role: NodeRole::Junction,
            rate: 0.0,
            stored: 0.0,
            capacity: 0.0,
            priority: 0,
            enabled: true,
        }
    }

    /// Create a source node.
    #[must_use]
    pub fn source(pos: LocalPos, kind: ConduitKind, rate: f32) -> Self {
        Self {
            pos,
            kind,
            role: NodeRole::Source,
            rate: rate.max(0.0),
            stored: 0.0,
            capacity: 0.0,
            priority: 0,
            enabled: true,
        }
    }

    /// Create a sink node.
    #[must_use]
    pub fn sink(pos: LocalPos, kind: ConduitKind, rate: f32) -> Self {
        Self {
            pos,
            kind,
            role: NodeRole::Sink,
            rate: rate.max(0.0),
            stored: 0.0,
            capacity: 0.0,
            priority: 0,
            enabled: true,
        }
    }

    /// Create a storage node.
    #[must_use]
    pub fn storage(pos: LocalPos, kind: ConduitKind, capacity: f32) -> Self {
        Self {
            pos,
            kind,
            role: NodeRole::Storage,
            rate: kind.base_capacity(),
            stored: 0.0,
            capacity: capacity.max(0.0),
            priority: -1,
            enabled: true,
        }
    }

    /// Amount this node wants to provide this tick.
    #[must_use]
    pub fn supply_available(&self, dt: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        match self.role {
            NodeRole::Source => self.rate * dt,
            NodeRole::Storage => self.stored.min(self.rate * dt),
            _ => 0.0,
        }
    }

    /// Amount this node wants to consume this tick.
    #[must_use]
    pub fn demand(&self, dt: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        match self.role {
            NodeRole::Sink => self.rate * dt,
            NodeRole::Storage => (self.capacity - self.stored).min(self.rate * dt),
            _ => 0.0,
        }
    }

    /// Apply production (source produces, removes from storage).
    pub fn produce(&mut self, amount: f32) -> f32 {
        match self.role {
            NodeRole::Source => amount,
            NodeRole::Storage => {
                let produced = amount.min(self.stored);
                self.stored -= produced;
                produced
            }
            _ => 0.0,
        }
    }

    /// Apply consumption (sink consumes, adds to storage).
    pub fn consume(&mut self, amount: f32) -> f32 {
        match self.role {
            NodeRole::Sink => amount,
            NodeRole::Storage => {
                let space = self.capacity - self.stored;
                let consumed = amount.min(space);
                self.stored += consumed;
                consumed
            }
            _ => 0.0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, reason = "tests check exact constructor values")]
mod tests {
    use super::*;

    #[test]
    fn role_round_trip() {
        for role in NodeRole::ALL {
            let index = role.as_index();
            let recovered = NodeRole::from_index(index);
            assert_eq!(recovered, Some(role));
        }
    }

    #[test]
    fn role_from_index_invalid() {
        assert_eq!(NodeRole::from_index(4), None);
        assert_eq!(NodeRole::from_index(100), None);
    }

    #[test]
    fn role_can_provide() {
        assert!(!NodeRole::Junction.can_provide());
        assert!(NodeRole::Source.can_provide());
        assert!(!NodeRole::Sink.can_provide());
        assert!(NodeRole::Storage.can_provide());
    }

    #[test]
    fn role_can_accept() {
        assert!(!NodeRole::Junction.can_accept());
        assert!(!NodeRole::Source.can_accept());
        assert!(NodeRole::Sink.can_accept());
        assert!(NodeRole::Storage.can_accept());
    }

    #[test]
    fn junction_node() {
        let node = ConduitNode::junction(LocalPos::new(5, 5, 5), ConduitKind::Power);
        assert_eq!(node.role, NodeRole::Junction);
        assert_eq!(node.supply_available(1.0), 0.0);
        assert_eq!(node.demand(1.0), 0.0);
    }

    #[test]
    fn source_node() {
        let node = ConduitNode::source(LocalPos::new(1, 1, 1), ConduitKind::Power, 10.0);
        assert_eq!(node.role, NodeRole::Source);
        assert!((node.supply_available(0.5) - 5.0).abs() < 0.001);
        assert_eq!(node.demand(1.0), 0.0);
    }

    #[test]
    fn sink_node() {
        let node = ConduitNode::sink(LocalPos::new(2, 2, 2), ConduitKind::Power, 8.0);
        assert_eq!(node.role, NodeRole::Sink);
        assert_eq!(node.supply_available(1.0), 0.0);
        assert!((node.demand(0.5) - 4.0).abs() < 0.001);
    }

    #[test]
    fn storage_node_supply() {
        let mut node = ConduitNode::storage(LocalPos::new(3, 3, 3), ConduitKind::Power, 100.0);
        node.stored = 50.0;
        let supply = node.supply_available(0.1);
        assert!(supply > 0.0);
        assert!(supply <= 50.0);
    }

    #[test]
    fn storage_node_demand() {
        let mut node = ConduitNode::storage(LocalPos::new(3, 3, 3), ConduitKind::Power, 100.0);
        node.stored = 20.0;
        let demand = node.demand(0.1);
        assert!(demand > 0.0);
    }

    #[test]
    fn source_produce() {
        let mut node = ConduitNode::source(LocalPos::new(0, 0, 0), ConduitKind::Power, 10.0);
        let produced = node.produce(5.0);
        assert!((produced - 5.0).abs() < 0.001);
    }

    #[test]
    fn storage_produce() {
        let mut node = ConduitNode::storage(LocalPos::new(0, 0, 0), ConduitKind::Power, 100.0);
        node.stored = 30.0;
        let produced = node.produce(20.0);
        assert!((produced - 20.0).abs() < 0.001);
        assert!((node.stored - 10.0).abs() < 0.001);
    }

    #[test]
    fn storage_produce_limited() {
        let mut node = ConduitNode::storage(LocalPos::new(0, 0, 0), ConduitKind::Power, 100.0);
        node.stored = 10.0;
        let produced = node.produce(20.0);
        assert!((produced - 10.0).abs() < 0.001);
        assert!((node.stored - 0.0).abs() < 0.001);
    }

    #[test]
    fn sink_consume() {
        let mut node = ConduitNode::sink(LocalPos::new(0, 0, 0), ConduitKind::Power, 10.0);
        let consumed = node.consume(5.0);
        assert!((consumed - 5.0).abs() < 0.001);
    }

    #[test]
    fn storage_consume() {
        let mut node = ConduitNode::storage(LocalPos::new(0, 0, 0), ConduitKind::Power, 100.0);
        node.stored = 20.0;
        let consumed = node.consume(30.0);
        assert!((consumed - 30.0).abs() < 0.001);
        assert!((node.stored - 50.0).abs() < 0.001);
    }

    #[test]
    fn storage_consume_limited() {
        let mut node = ConduitNode::storage(LocalPos::new(0, 0, 0), ConduitKind::Power, 100.0);
        node.stored = 90.0;
        let consumed = node.consume(20.0);
        assert!((consumed - 10.0).abs() < 0.001);
        assert!((node.stored - 100.0).abs() < 0.001);
    }

    #[test]
    fn disabled_node() {
        let mut node = ConduitNode::source(LocalPos::new(0, 0, 0), ConduitKind::Power, 10.0);
        node.enabled = false;
        assert_eq!(node.supply_available(1.0), 0.0);
        assert_eq!(node.demand(1.0), 0.0);
    }

    #[test]
    fn serde_role_round_trip() {
        for role in NodeRole::ALL {
            let json = serde_json::to_string(&role).unwrap();
            let recovered: NodeRole = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, role);
        }
    }

    #[test]
    fn serde_node_round_trip() {
        let mut node = ConduitNode::storage(LocalPos::new(5, 6, 7), ConduitKind::Fluid, 50.0);
        node.stored = 25.0;
        node.priority = 5;
        let json = serde_json::to_string(&node).unwrap();
        let recovered: ConduitNode = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, node);
    }
}

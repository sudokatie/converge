//! Topology node definition.

use serde::{Deserialize, Serialize};

/// Unique identifier for a topology node.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create a new node ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N{}", self.0)
    }
}

impl From<u64> for NodeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Role of a node in the topology.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum NodeRole {
    /// Standard passable node.
    #[default]
    Standard = 0,
    /// Entry point into the topology.
    Entry = 1,
    /// Exit point from the topology.
    Exit = 2,
    /// Junction with multiple connections.
    Junction = 3,
    /// Dead end with single connection.
    DeadEnd = 4,
    /// Chamber or room (larger open space).
    Chamber = 5,
    /// Transition between topology types.
    Transition = 6,
}

impl NodeRole {
    /// Get the name of this role.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Entry => "entry",
            Self::Exit => "exit",
            Self::Junction => "junction",
            Self::DeadEnd => "dead_end",
            Self::Chamber => "chamber",
            Self::Transition => "transition",
        }
    }

    /// Check if this is a terminal role.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Entry | Self::Exit | Self::DeadEnd)
    }

    /// Check if this is a connection role.
    #[must_use]
    pub const fn is_connection(&self) -> bool {
        matches!(self, Self::Junction | Self::Transition)
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Standard),
            1 => Some(Self::Entry),
            2 => Some(Self::Exit),
            3 => Some(Self::Junction),
            4 => Some(Self::DeadEnd),
            5 => Some(Self::Chamber),
            6 => Some(Self::Transition),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// A node in the topology graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyNode {
    /// Unique identifier.
    pub id: NodeId,
    /// Role of this node.
    pub role: NodeRole,
    /// Position in 3D space (center point).
    pub position: [f32; 3],
    /// Size/radius of the node area.
    pub radius: f32,
    /// Height of the node area (for enclosed spaces).
    pub height: f32,
    /// Depth from entry node (0 = entry).
    pub depth: u32,
}

impl TopologyNode {
    /// Create a new topology node.
    #[must_use]
    pub fn new(id: NodeId, position: [f32; 3], radius: f32, height: f32) -> Self {
        Self {
            id,
            role: NodeRole::Standard,
            position,
            radius,
            height,
            depth: 0,
        }
    }

    /// Create a new entry node.
    #[must_use]
    pub fn entry(id: NodeId, position: [f32; 3], radius: f32, height: f32) -> Self {
        Self {
            id,
            role: NodeRole::Entry,
            position,
            radius,
            height,
            depth: 0,
        }
    }

    /// Set the role.
    #[must_use]
    pub fn with_role(mut self, role: NodeRole) -> Self {
        self.role = role;
        self
    }

    /// Set the depth.
    #[must_use]
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Get the X position.
    #[must_use]
    pub fn x(&self) -> f32 {
        self.position[0]
    }

    /// Get the Y position.
    #[must_use]
    pub fn y(&self) -> f32 {
        self.position[1]
    }

    /// Get the Z position.
    #[must_use]
    pub fn z(&self) -> f32 {
        self.position[2]
    }

    /// Calculate squared distance to a point.
    #[must_use]
    pub fn distance_squared(&self, point: [f32; 3]) -> f32 {
        let dx = self.position[0] - point[0];
        let dy = self.position[1] - point[1];
        let dz = self.position[2] - point[2];
        dx * dx + dy * dy + dz * dz
    }

    /// Calculate distance to a point.
    #[must_use]
    pub fn distance(&self, point: [f32; 3]) -> f32 {
        self.distance_squared(point).sqrt()
    }

    /// Check if a point is within the node's bounds.
    #[must_use]
    pub fn contains(&self, point: [f32; 3]) -> bool {
        let dx = self.position[0] - point[0];
        let dz = self.position[2] - point[2];
        let horizontal_dist_sq = dx * dx + dz * dz;

        if horizontal_dist_sq > self.radius * self.radius {
            return false;
        }

        let dy = point[1] - self.position[1];
        dy.abs() <= self.height / 2.0
    }

    /// Get the volume of this node (approximate).
    #[must_use]
    pub fn volume(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_creation() {
        let node = TopologyNode::new(NodeId::new(1), [10.0, 20.0, 30.0], 5.0, 3.0);
        assert_eq!(node.id, NodeId::new(1));
        assert_eq!(node.role, NodeRole::Standard);
        assert!((node.x() - 10.0).abs() < f32::EPSILON);
        assert!((node.y() - 20.0).abs() < f32::EPSILON);
        assert!((node.z() - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn node_entry() {
        let node = TopologyNode::entry(NodeId::new(0), [0.0, 0.0, 0.0], 4.0, 3.0);
        assert_eq!(node.role, NodeRole::Entry);
        assert_eq!(node.depth, 0);
    }

    #[test]
    fn node_contains() {
        let node = TopologyNode::new(NodeId::new(1), [0.0, 0.0, 0.0], 5.0, 4.0);

        assert!(node.contains([0.0, 0.0, 0.0]));
        assert!(node.contains([3.0, 1.0, 3.0]));
        assert!(!node.contains([10.0, 0.0, 0.0]));
        assert!(!node.contains([0.0, 10.0, 0.0]));
    }

    #[test]
    fn node_distance() {
        let node = TopologyNode::new(NodeId::new(1), [0.0, 0.0, 0.0], 5.0, 4.0);
        let dist = node.distance([3.0, 0.0, 4.0]);
        assert!((dist - 5.0).abs() < 0.001);
    }

    #[test]
    fn node_role_properties() {
        assert!(NodeRole::Entry.is_terminal());
        assert!(NodeRole::DeadEnd.is_terminal());
        assert!(NodeRole::Junction.is_connection());
        assert!(!NodeRole::Chamber.is_terminal());
    }

    #[test]
    fn node_role_from_raw() {
        for i in 0..7 {
            let role = NodeRole::from_raw(i);
            assert!(role.is_some());
            assert_eq!(role.unwrap().as_raw(), i);
        }
        assert!(NodeRole::from_raw(99).is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let node = TopologyNode::new(NodeId::new(42), [1.0, 2.0, 3.0], 5.0, 4.0)
            .with_role(NodeRole::Junction)
            .with_depth(3);

        let json = serde_json::to_string(&node).unwrap();
        let recovered: TopologyNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, recovered);
    }
}

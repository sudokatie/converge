//! Edges connecting regions in the graph.

use serde::{Deserialize, Serialize};

use super::edge_id::EdgeId;
use super::edge_kind::EdgeKind;
use super::gate::{GateRequirement, ProgressionTier};
use super::region_id::RegionId;

/// An edge connecting two regions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionEdge {
    /// Unique identifier.
    pub id: EdgeId,
    /// Source region.
    pub from: RegionId,
    /// Target region.
    pub to: RegionId,
    /// Kind of edge.
    pub kind: EdgeKind,
    /// Traversal cost.
    pub cost: u32,
    /// Whether this edge is bidirectional.
    pub bidirectional: bool,
    /// Gate requirement (if any).
    pub gate: Option<GateRequirement>,
    /// Whether this edge is currently passable.
    pub passable: bool,
    /// Whether this edge is visible on the map.
    pub visible: bool,
}

impl RegionEdge {
    /// Create a new edge.
    #[must_use]
    pub fn new(id: EdgeId, from: RegionId, to: RegionId, kind: EdgeKind) -> Self {
        Self {
            id,
            from,
            to,
            kind,
            cost: kind.base_cost(),
            bidirectional: !kind.is_one_way(),
            gate: None,
            passable: true,
            visible: true,
        }
    }

    /// Create a simple bidirectional path.
    #[must_use]
    pub fn path(id: EdgeId, from: RegionId, to: RegionId) -> Self {
        Self::new(id, from, to, EdgeKind::Path)
    }

    /// Set the cost.
    #[must_use]
    pub fn with_cost(mut self, cost: u32) -> Self {
        self.cost = cost;
        self
    }

    /// Mark as one-way.
    #[must_use]
    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    /// Set the gate requirement.
    #[must_use]
    pub fn with_gate(mut self, gate: GateRequirement) -> Self {
        self.gate = Some(gate);
        self
    }

    /// Mark as impassable.
    #[must_use]
    pub fn blocked(mut self) -> Self {
        self.passable = false;
        self
    }

    /// Mark as hidden.
    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Check if this edge connects the given regions (in either direction if bidirectional).
    #[must_use]
    pub fn connects(&self, a: RegionId, b: RegionId) -> bool {
        if self.from == a && self.to == b {
            return true;
        }
        self.bidirectional && self.from == b && self.to == a
    }

    /// Check if this edge can be traversed from the given region.
    #[must_use]
    pub fn can_traverse_from(&self, region: RegionId) -> bool {
        if !self.passable {
            return false;
        }
        if self.from == region {
            return true;
        }
        self.bidirectional && self.to == region
    }

    /// Get the destination when traversing from the given region.
    #[must_use]
    pub fn destination_from(&self, region: RegionId) -> Option<RegionId> {
        if self.from == region {
            return Some(self.to);
        }
        if self.bidirectional && self.to == region {
            return Some(self.from);
        }
        None
    }

    /// Check if this edge is accessible at the given tier.
    #[must_use]
    pub fn is_accessible(&self, player_tier: ProgressionTier) -> bool {
        if !self.passable {
            return false;
        }
        if let Some(gate) = &self.gate {
            return gate.is_accessible(player_tier);
        }
        true
    }

    /// Get the effective cost considering accessibility.
    #[must_use]
    pub fn effective_cost(&self, player_tier: ProgressionTier) -> Option<u32> {
        if !self.is_accessible(player_tier) {
            return None;
        }
        Some(self.cost)
    }

    /// Ordering key for deterministic sorting.
    fn sort_key(&self) -> (u64, u64, u8) {
        (self.from.raw(), self.to.raw(), self.kind.as_raw())
    }
}

impl PartialOrd for RegionEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RegionEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl std::hash::Hash for RegionEdge {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_creation() {
        let edge = RegionEdge::new(
            EdgeId::new(1, 1),
            RegionId::new(1, 1),
            RegionId::new(1, 2),
            EdgeKind::Corridor,
        );

        assert!(edge.bidirectional);
        assert!(edge.passable);
        assert_eq!(edge.cost, EdgeKind::Corridor.base_cost());
    }

    #[test]
    fn edge_traversal() {
        let r1 = RegionId::new(1, 1);
        let r2 = RegionId::new(1, 2);
        let r3 = RegionId::new(1, 3);

        let bidir = RegionEdge::path(EdgeId::new(1, 1), r1, r2);
        assert!(bidir.can_traverse_from(r1));
        assert!(bidir.can_traverse_from(r2));
        assert!(!bidir.can_traverse_from(r3));

        let oneway = RegionEdge::new(EdgeId::new(1, 2), r1, r2, EdgeKind::Drop);
        assert!(oneway.can_traverse_from(r1));
        assert!(!oneway.can_traverse_from(r2));
    }

    #[test]
    fn edge_destination() {
        let r1 = RegionId::new(1, 1);
        let r2 = RegionId::new(1, 2);

        let edge = RegionEdge::path(EdgeId::new(1, 1), r1, r2);
        assert_eq!(edge.destination_from(r1), Some(r2));
        assert_eq!(edge.destination_from(r2), Some(r1));

        let oneway = RegionEdge::path(EdgeId::new(1, 2), r1, r2).one_way();
        assert_eq!(oneway.destination_from(r1), Some(r2));
        assert_eq!(oneway.destination_from(r2), None);
    }

    #[test]
    fn edge_connects() {
        let r1 = RegionId::new(1, 1);
        let r2 = RegionId::new(1, 2);
        let r3 = RegionId::new(1, 3);

        let edge = RegionEdge::path(EdgeId::new(1, 1), r1, r2);
        assert!(edge.connects(r1, r2));
        assert!(edge.connects(r2, r1));
        assert!(!edge.connects(r1, r3));

        let oneway = RegionEdge::path(EdgeId::new(1, 2), r1, r2).one_way();
        assert!(oneway.connects(r1, r2));
        assert!(!oneway.connects(r2, r1));
    }

    #[test]
    fn edge_accessibility() {
        let r1 = RegionId::new(1, 1);
        let r2 = RegionId::new(1, 2);

        let edge = RegionEdge::path(EdgeId::new(1, 1), r1, r2).with_gate(GateRequirement::tier(3));

        assert!(!edge.is_accessible(ProgressionTier::new(2)));
        assert!(edge.is_accessible(ProgressionTier::new(3)));

        let blocked = RegionEdge::path(EdgeId::new(1, 2), r1, r2).blocked();
        assert!(!blocked.is_accessible(ProgressionTier::START));
    }

    #[test]
    fn edge_ordering() {
        let r1 = RegionId::new(1, 1);
        let r2 = RegionId::new(1, 2);
        let r3 = RegionId::new(1, 3);

        let e1 = RegionEdge::path(EdgeId::new(1, 1), r1, r2);
        let e2 = RegionEdge::path(EdgeId::new(1, 2), r1, r3);
        let e3 = RegionEdge::path(EdgeId::new(1, 3), r2, r3);

        assert!(e1 < e2);
        assert!(e2 < e3);
    }

    #[test]
    fn serde_roundtrip() {
        let edge = RegionEdge::new(
            EdgeId::new(42, 1),
            RegionId::new(1, 1),
            RegionId::new(1, 2),
            EdgeKind::Airlock,
        )
        .with_cost(50)
        .with_gate(GateRequirement::key("airlock_key"));

        let json = serde_json::to_string(&edge).unwrap();
        let recovered: RegionEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, recovered);
    }
}

//! Topology segment (connection between nodes).

use serde::{Deserialize, Serialize};

use super::node::NodeId;

/// Unique identifier for a topology segment.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct SegmentId(pub u64);

impl SegmentId {
    /// Create a new segment ID.
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

impl std::fmt::Display for SegmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S{}", self.0)
    }
}

impl From<u64> for SegmentId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Kind of topology segment.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum SegmentKind {
    /// Standard corridor/passage.
    #[default]
    Corridor = 0,
    /// Narrow tunnel.
    Tunnel = 1,
    /// Wide shaft (vertical).
    Shaft = 2,
    /// Stairway or ramp.
    Ramp = 3,
    /// Bridge or crossing.
    Bridge = 4,
    /// Airlock or sealed passage.
    Airlock = 5,
    /// Open connection (no walls).
    Open = 6,
}

impl SegmentKind {
    /// Get the name of this kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Corridor => "corridor",
            Self::Tunnel => "tunnel",
            Self::Shaft => "shaft",
            Self::Ramp => "ramp",
            Self::Bridge => "bridge",
            Self::Airlock => "airlock",
            Self::Open => "open",
        }
    }

    /// Check if this is an enclosed kind.
    #[must_use]
    pub const fn is_enclosed(&self) -> bool {
        matches!(self, Self::Corridor | Self::Tunnel | Self::Airlock)
    }

    /// Check if this is a vertical kind.
    #[must_use]
    pub const fn is_vertical(&self) -> bool {
        matches!(self, Self::Shaft | Self::Ramp)
    }

    /// Get traversal cost multiplier.
    #[must_use]
    pub const fn cost_multiplier(&self) -> f32 {
        match self {
            Self::Corridor | Self::Open => 1.0,
            Self::Tunnel => 1.2,
            Self::Shaft => 1.5,
            Self::Ramp => 1.3,
            Self::Bridge => 1.1,
            Self::Airlock => 2.0,
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Corridor),
            1 => Some(Self::Tunnel),
            2 => Some(Self::Shaft),
            3 => Some(Self::Ramp),
            4 => Some(Self::Bridge),
            5 => Some(Self::Airlock),
            6 => Some(Self::Open),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// A segment connecting two topology nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologySegment {
    /// Unique identifier.
    pub id: SegmentId,
    /// Starting node.
    pub from: NodeId,
    /// Ending node.
    pub to: NodeId,
    /// Kind of segment.
    pub kind: SegmentKind,
    /// Width of the segment.
    pub width: f32,
    /// Height of the segment.
    pub height: f32,
    /// Length of the segment.
    pub length: f32,
    /// Whether the segment is bidirectional.
    pub bidirectional: bool,
    /// Traversal cost (considering length and kind).
    pub cost: u32,
}

impl TopologySegment {
    /// Create a new topology segment.
    #[must_use]
    pub fn new(id: SegmentId, from: NodeId, to: NodeId, length: f32) -> Self {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "cost always fits in u32"
        )]
        let cost = (length * 10.0) as u32;
        Self {
            id,
            from,
            to,
            kind: SegmentKind::Corridor,
            width: 4.0,
            height: 3.0,
            length,
            bidirectional: true,
            cost,
        }
    }

    /// Set the kind.
    #[must_use]
    pub fn with_kind(mut self, kind: SegmentKind) -> Self {
        self.kind = kind;
        self.update_cost();
        self
    }

    /// Set dimensions.
    #[must_use]
    pub fn with_dimensions(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set bidirectional flag.
    #[must_use]
    pub fn with_bidirectional(mut self, bidirectional: bool) -> Self {
        self.bidirectional = bidirectional;
        self
    }

    /// Update cost based on length and kind.
    fn update_cost(&mut self) {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "cost always fits in u32"
        )]
        let cost = (self.length * self.kind.cost_multiplier() * 10.0) as u32;
        self.cost = cost;
    }

    /// Check if this segment connects two specific nodes (in either direction).
    #[must_use]
    pub fn connects(&self, a: NodeId, b: NodeId) -> bool {
        (self.from == a && self.to == b) || (self.bidirectional && self.from == b && self.to == a)
    }

    /// Get the other end of the segment given one end.
    #[must_use]
    pub fn other_end(&self, node: NodeId) -> Option<NodeId> {
        if self.from == node {
            Some(self.to)
        } else if self.to == node && self.bidirectional {
            Some(self.from)
        } else {
            None
        }
    }

    /// Get the cross-sectional area.
    #[must_use]
    pub fn cross_section_area(&self) -> f32 {
        self.width * self.height
    }

    /// Get the volume.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.cross_section_area() * self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_creation() {
        let seg = TopologySegment::new(SegmentId::new(1), NodeId::new(0), NodeId::new(1), 10.0);
        assert_eq!(seg.id, SegmentId::new(1));
        assert_eq!(seg.from, NodeId::new(0));
        assert_eq!(seg.to, NodeId::new(1));
        assert!((seg.length - 10.0).abs() < f32::EPSILON);
        assert!(seg.bidirectional);
    }

    #[test]
    fn segment_connects() {
        let seg = TopologySegment::new(SegmentId::new(1), NodeId::new(0), NodeId::new(1), 10.0);

        assert!(seg.connects(NodeId::new(0), NodeId::new(1)));
        assert!(seg.connects(NodeId::new(1), NodeId::new(0)));
        assert!(!seg.connects(NodeId::new(0), NodeId::new(2)));
    }

    #[test]
    fn segment_one_way() {
        let seg = TopologySegment::new(SegmentId::new(1), NodeId::new(0), NodeId::new(1), 10.0)
            .with_bidirectional(false);

        assert!(seg.connects(NodeId::new(0), NodeId::new(1)));
        assert!(!seg.connects(NodeId::new(1), NodeId::new(0)));
    }

    #[test]
    fn segment_other_end() {
        let seg = TopologySegment::new(SegmentId::new(1), NodeId::new(0), NodeId::new(1), 10.0);

        assert_eq!(seg.other_end(NodeId::new(0)), Some(NodeId::new(1)));
        assert_eq!(seg.other_end(NodeId::new(1)), Some(NodeId::new(0)));
        assert_eq!(seg.other_end(NodeId::new(2)), None);
    }

    #[test]
    fn segment_kind_properties() {
        assert!(SegmentKind::Corridor.is_enclosed());
        assert!(SegmentKind::Shaft.is_vertical());
        assert!(!SegmentKind::Open.is_enclosed());
        assert!(!SegmentKind::Bridge.is_vertical());
    }

    #[test]
    fn segment_kind_from_raw() {
        for i in 0..7 {
            let kind = SegmentKind::from_raw(i);
            assert!(kind.is_some());
            assert_eq!(kind.unwrap().as_raw(), i);
        }
        assert!(SegmentKind::from_raw(99).is_none());
    }

    #[test]
    fn segment_cost_multiplier() {
        let base = TopologySegment::new(SegmentId::new(1), NodeId::new(0), NodeId::new(1), 10.0);
        let base_cost = base.cost;

        let airlock = base.clone().with_kind(SegmentKind::Airlock);
        assert!(airlock.cost > base_cost);
    }

    #[test]
    fn serde_roundtrip() {
        let seg = TopologySegment::new(SegmentId::new(42), NodeId::new(1), NodeId::new(2), 15.0)
            .with_kind(SegmentKind::Tunnel)
            .with_dimensions(3.0, 2.5);

        let json = serde_json::to_string(&seg).unwrap();
        let recovered: TopologySegment = serde_json::from_str(&json).unwrap();
        assert_eq!(seg, recovered);
    }
}

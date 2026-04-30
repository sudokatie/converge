//! Topology kind classification.

use serde::{Deserialize, Serialize};

/// Kind of topology layout.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum TopologyKind {
    /// Deep ocean trench with vertical walls and narrow passages.
    #[default]
    Trench = 0,
    /// Ice tunnel system with branching corridors.
    IceTunnel = 1,
    /// Station deck with rooms and corridors.
    StationDeck = 2,
    /// Hollow sphere interior with curved walls.
    HollowSphere = 3,
}

impl TopologyKind {
    /// Get the name of this kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Trench => "trench",
            Self::IceTunnel => "ice_tunnel",
            Self::StationDeck => "station_deck",
            Self::HollowSphere => "hollow_sphere",
        }
    }

    /// Get the display name of this kind.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Trench => "Trench",
            Self::IceTunnel => "Ice Tunnel",
            Self::StationDeck => "Station Deck",
            Self::HollowSphere => "Hollow Sphere",
        }
    }

    /// Get default node count for this kind.
    #[must_use]
    pub const fn default_node_count(&self) -> u32 {
        match self {
            Self::Trench => 20,
            Self::IceTunnel => 30,
            Self::StationDeck => 25,
            Self::HollowSphere => 15,
        }
    }

    /// Get default segment width range for this kind.
    #[must_use]
    pub const fn default_width_range(&self) -> (f32, f32) {
        match self {
            Self::Trench => (8.0, 20.0),
            Self::IceTunnel => (4.0, 12.0),
            Self::StationDeck => (3.0, 8.0),
            Self::HollowSphere => (10.0, 30.0),
        }
    }

    /// Get default segment height range for this kind.
    #[must_use]
    pub const fn default_height_range(&self) -> (f32, f32) {
        match self {
            Self::Trench => (20.0, 80.0),
            Self::IceTunnel => (3.0, 8.0),
            Self::StationDeck => (3.0, 5.0),
            Self::HollowSphere => (15.0, 50.0),
        }
    }

    /// Check if this kind typically has vertical structure.
    #[must_use]
    pub const fn is_vertical(&self) -> bool {
        matches!(self, Self::Trench | Self::HollowSphere)
    }

    /// Check if this kind typically has enclosed spaces.
    #[must_use]
    pub const fn is_enclosed(&self) -> bool {
        matches!(self, Self::IceTunnel | Self::StationDeck)
    }

    /// Get branching factor (average connections per node).
    #[must_use]
    pub const fn branching_factor(&self) -> f32 {
        match self {
            Self::Trench => 1.5,
            Self::IceTunnel => 2.5,
            Self::StationDeck => 2.0,
            Self::HollowSphere => 3.0,
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Trench),
            1 => Some(Self::IceTunnel),
            2 => Some(Self::StationDeck),
            3 => Some(Self::HollowSphere),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }

    /// Iterate over all topology kinds.
    pub fn all() -> impl Iterator<Item = Self> {
        [
            Self::Trench,
            Self::IceTunnel,
            Self::StationDeck,
            Self::HollowSphere,
        ]
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_properties() {
        assert!(TopologyKind::Trench.is_vertical());
        assert!(TopologyKind::IceTunnel.is_enclosed());
        assert!(!TopologyKind::StationDeck.is_vertical());
        assert!(!TopologyKind::HollowSphere.is_enclosed());
    }

    #[test]
    fn kind_from_raw() {
        assert_eq!(TopologyKind::from_raw(0), Some(TopologyKind::Trench));
        assert_eq!(TopologyKind::from_raw(1), Some(TopologyKind::IceTunnel));
        assert_eq!(TopologyKind::from_raw(2), Some(TopologyKind::StationDeck));
        assert_eq!(TopologyKind::from_raw(3), Some(TopologyKind::HollowSphere));
        assert_eq!(TopologyKind::from_raw(99), None);
    }

    #[test]
    fn kind_roundtrip() {
        for kind in TopologyKind::all() {
            let raw = kind.as_raw();
            let recovered = TopologyKind::from_raw(raw);
            assert_eq!(recovered, Some(kind));
        }
    }

    #[test]
    fn serde_roundtrip() {
        let kind = TopologyKind::IceTunnel;
        let json = serde_json::to_string(&kind).unwrap();
        let recovered: TopologyKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, recovered);
    }

    #[test]
    fn default_ranges_valid() {
        for kind in TopologyKind::all() {
            let (w_min, w_max) = kind.default_width_range();
            assert!(w_min > 0.0 && w_min <= w_max);

            let (h_min, h_max) = kind.default_height_range();
            assert!(h_min > 0.0 && h_min <= h_max);
        }
    }
}

//! Edge kind classifications for region connections.

use serde::{Deserialize, Serialize};

/// Kind of edge connecting regions.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum EdgeKind {
    /// Generic traversable connection.
    #[default]
    Path = 0,
    /// Corridor or hallway.
    Corridor = 1,
    /// Tunnel or passage.
    Tunnel = 2,
    /// Bridge or elevated path.
    Bridge = 3,
    /// Ladder or vertical climb.
    Ladder = 4,
    /// Elevator or lift.
    Elevator = 5,
    /// Teleporter or portal.
    Portal = 6,
    /// Airlock or pressurized door.
    Airlock = 7,
    /// One-way drop.
    Drop = 8,
    /// Swim or dive path.
    Swim = 9,
    /// Hazardous route.
    Hazardous = 10,
    /// Locked or secured path.
    Locked = 11,
    /// Hidden or secret path.
    Secret = 12,
}

impl EdgeKind {
    /// Get the name of this kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Corridor => "corridor",
            Self::Tunnel => "tunnel",
            Self::Bridge => "bridge",
            Self::Ladder => "ladder",
            Self::Elevator => "elevator",
            Self::Portal => "portal",
            Self::Airlock => "airlock",
            Self::Drop => "drop",
            Self::Swim => "swim",
            Self::Hazardous => "hazardous",
            Self::Locked => "locked",
            Self::Secret => "secret",
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Path => "Path",
            Self::Corridor => "Corridor",
            Self::Tunnel => "Tunnel",
            Self::Bridge => "Bridge",
            Self::Ladder => "Ladder",
            Self::Elevator => "Elevator",
            Self::Portal => "Portal",
            Self::Airlock => "Airlock",
            Self::Drop => "Drop",
            Self::Swim => "Swim",
            Self::Hazardous => "Hazardous",
            Self::Locked => "Locked",
            Self::Secret => "Secret",
        }
    }

    /// Check if this edge requires special traversal.
    #[must_use]
    pub const fn requires_special_traversal(&self) -> bool {
        matches!(
            self,
            Self::Ladder | Self::Elevator | Self::Portal | Self::Swim
        )
    }

    /// Check if this edge is dangerous.
    #[must_use]
    pub const fn is_dangerous(&self) -> bool {
        matches!(self, Self::Hazardous | Self::Drop)
    }

    /// Check if this edge is restricted.
    #[must_use]
    pub const fn is_restricted(&self) -> bool {
        matches!(self, Self::Locked | Self::Secret | Self::Airlock)
    }

    /// Check if this edge is one-way.
    #[must_use]
    pub const fn is_one_way(&self) -> bool {
        matches!(self, Self::Drop)
    }

    /// Default traversal cost for this edge kind.
    #[must_use]
    pub const fn base_cost(&self) -> u32 {
        match self {
            Self::Portal => 1,
            Self::Elevator | Self::Drop => 5,
            Self::Path | Self::Corridor | Self::Secret => 10,
            Self::Bridge => 12,
            Self::Tunnel => 15,
            Self::Ladder => 20,
            Self::Airlock => 25,
            Self::Swim => 30,
            Self::Hazardous => 50,
            Self::Locked => 100,
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Path),
            1 => Some(Self::Corridor),
            2 => Some(Self::Tunnel),
            3 => Some(Self::Bridge),
            4 => Some(Self::Ladder),
            5 => Some(Self::Elevator),
            6 => Some(Self::Portal),
            7 => Some(Self::Airlock),
            8 => Some(Self::Drop),
            9 => Some(Self::Swim),
            10 => Some(Self::Hazardous),
            11 => Some(Self::Locked),
            12 => Some(Self::Secret),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kind_properties() {
        assert!(EdgeKind::Ladder.requires_special_traversal());
        assert!(EdgeKind::Hazardous.is_dangerous());
        assert!(EdgeKind::Locked.is_restricted());
        assert!(EdgeKind::Drop.is_one_way());
        assert!(!EdgeKind::Path.is_one_way());
    }

    #[test]
    fn edge_kind_costs() {
        assert_eq!(EdgeKind::Path.base_cost(), 10);
        assert_eq!(EdgeKind::Portal.base_cost(), 1);
        assert_eq!(EdgeKind::Hazardous.base_cost(), 50);
    }

    #[test]
    fn edge_kind_from_raw() {
        assert_eq!(EdgeKind::from_raw(0), Some(EdgeKind::Path));
        assert_eq!(EdgeKind::from_raw(6), Some(EdgeKind::Portal));
        assert_eq!(EdgeKind::from_raw(99), None);
    }

    #[test]
    fn serde_roundtrip() {
        let kind = EdgeKind::Portal;
        let json = serde_json::to_string(&kind).unwrap();
        let recovered: EdgeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, recovered);
    }
}

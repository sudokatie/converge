//! Region kind and tag classifications.

use serde::{Deserialize, Serialize};

/// Kind of region in the graph.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum RegionKind {
    /// Generic region with no special properties.
    #[default]
    Generic = 0,
    /// Station or outpost.
    Station = 1,
    /// Trench or canyon.
    Trench = 2,
    /// Cave or tunnel network.
    Cave = 3,
    /// Hollow sphere or void.
    Sphere = 4,
    /// Colony or settlement.
    Colony = 5,
    /// Transit hub or junction.
    Hub = 6,
    /// Gate or portal location.
    Gate = 7,
    /// Hazard zone.
    Hazard = 8,
    /// Resource deposit.
    Resource = 9,
    /// Mission objective location.
    Objective = 10,
    /// Spawn or start point.
    Spawn = 11,
    /// End or goal point.
    Goal = 12,
}

impl RegionKind {
    /// Get the name of this kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Station => "station",
            Self::Trench => "trench",
            Self::Cave => "cave",
            Self::Sphere => "sphere",
            Self::Colony => "colony",
            Self::Hub => "hub",
            Self::Gate => "gate",
            Self::Hazard => "hazard",
            Self::Resource => "resource",
            Self::Objective => "objective",
            Self::Spawn => "spawn",
            Self::Goal => "goal",
        }
    }

    /// Get the display name of this kind.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Generic => "Generic",
            Self::Station => "Station",
            Self::Trench => "Trench",
            Self::Cave => "Cave",
            Self::Sphere => "Sphere",
            Self::Colony => "Colony",
            Self::Hub => "Hub",
            Self::Gate => "Gate",
            Self::Hazard => "Hazard Zone",
            Self::Resource => "Resource",
            Self::Objective => "Objective",
            Self::Spawn => "Spawn",
            Self::Goal => "Goal",
        }
    }

    /// Check if this is a navigational kind.
    #[must_use]
    pub const fn is_navigation(&self) -> bool {
        matches!(self, Self::Hub | Self::Gate | Self::Spawn | Self::Goal)
    }

    /// Check if this is a hazardous kind.
    #[must_use]
    pub const fn is_hazardous(&self) -> bool {
        matches!(self, Self::Hazard | Self::Trench | Self::Cave)
    }

    /// Check if this is a settlement kind.
    #[must_use]
    pub const fn is_settlement(&self) -> bool {
        matches!(self, Self::Station | Self::Colony)
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Generic),
            1 => Some(Self::Station),
            2 => Some(Self::Trench),
            3 => Some(Self::Cave),
            4 => Some(Self::Sphere),
            5 => Some(Self::Colony),
            6 => Some(Self::Hub),
            7 => Some(Self::Gate),
            8 => Some(Self::Hazard),
            9 => Some(Self::Resource),
            10 => Some(Self::Objective),
            11 => Some(Self::Spawn),
            12 => Some(Self::Goal),
            _ => None,
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// Tags for additional region classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum RegionTag {
    /// Critical path node (must be traversed).
    Critical = 0,
    /// Optional side path.
    Optional = 1,
    /// Dead end (no further connections).
    DeadEnd = 2,
    /// Branch point (multiple paths).
    Branch = 3,
    /// Chokepoint (single required passage).
    Chokepoint = 4,
    /// Safe zone.
    Safe = 5,
    /// Dangerous zone.
    Dangerous = 6,
    /// Hidden or secret.
    Hidden = 7,
    /// Locked or gated.
    Locked = 8,
    /// Contains loot or rewards.
    Loot = 9,
    /// Contains enemies or threats.
    Enemy = 10,
    /// Contains NPCs.
    Npc = 11,
}

impl RegionTag {
    /// Get the name of this tag.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Optional => "optional",
            Self::DeadEnd => "dead_end",
            Self::Branch => "branch",
            Self::Chokepoint => "chokepoint",
            Self::Safe => "safe",
            Self::Dangerous => "dangerous",
            Self::Hidden => "hidden",
            Self::Locked => "locked",
            Self::Loot => "loot",
            Self::Enemy => "enemy",
            Self::Npc => "npc",
        }
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Critical),
            1 => Some(Self::Optional),
            2 => Some(Self::DeadEnd),
            3 => Some(Self::Branch),
            4 => Some(Self::Chokepoint),
            5 => Some(Self::Safe),
            6 => Some(Self::Dangerous),
            7 => Some(Self::Hidden),
            8 => Some(Self::Locked),
            9 => Some(Self::Loot),
            10 => Some(Self::Enemy),
            11 => Some(Self::Npc),
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
    fn region_kind_properties() {
        assert!(RegionKind::Hub.is_navigation());
        assert!(RegionKind::Hazard.is_hazardous());
        assert!(RegionKind::Station.is_settlement());
        assert!(!RegionKind::Generic.is_navigation());
    }

    #[test]
    fn region_kind_from_raw() {
        assert_eq!(RegionKind::from_raw(0), Some(RegionKind::Generic));
        assert_eq!(RegionKind::from_raw(1), Some(RegionKind::Station));
        assert_eq!(RegionKind::from_raw(99), None);
    }

    #[test]
    fn region_tag_from_raw() {
        assert_eq!(RegionTag::from_raw(0), Some(RegionTag::Critical));
        assert_eq!(RegionTag::from_raw(4), Some(RegionTag::Chokepoint));
        assert_eq!(RegionTag::from_raw(99), None);
    }

    #[test]
    fn serde_roundtrip() {
        let kind = RegionKind::Station;
        let json = serde_json::to_string(&kind).unwrap();
        let recovered: RegionKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, recovered);

        let tag = RegionTag::Critical;
        let json = serde_json::to_string(&tag).unwrap();
        let recovered: RegionTag = serde_json::from_str(&json).unwrap();
        assert_eq!(tag, recovered);
    }
}

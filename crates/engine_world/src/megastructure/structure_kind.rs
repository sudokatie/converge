//! Megastructure category types.

use serde::{Deserialize, Serialize};

/// Category of megastructure.
///
/// Determines streaming priority, collision handling, and rendering approach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureKind {
    /// Space station - large orbital or floating structure.
    Station,
    /// Titan - massive mobile structure (capital ship, walking fortress).
    Titan,
    /// Interior space - enclosed area within another structure.
    Interior,
    /// Trench wall - linear barrier or canyon wall.
    TrenchWall,
}

impl StructureKind {
    /// Get all variants for iteration.
    pub const ALL: [Self; 4] = [Self::Station, Self::Titan, Self::Interior, Self::TrenchWall];

    /// Streaming priority (lower = higher priority).
    ///
    /// Interiors load first (players are inside), then stations,
    /// titans, and finally trench walls.
    #[must_use]
    pub const fn streaming_priority(self) -> u8 {
        match self {
            Self::Interior => 0,
            Self::Station => 1,
            Self::Titan => 2,
            Self::TrenchWall => 3,
        }
    }

    /// Whether this structure type can move.
    #[must_use]
    pub const fn is_mobile(self) -> bool {
        matches!(self, Self::Titan)
    }

    /// Whether this structure typically has an interior.
    #[must_use]
    pub const fn has_interior(self) -> bool {
        matches!(self, Self::Station | Self::Titan)
    }

    /// Whether this structure is a boundary/barrier type.
    #[must_use]
    pub const fn is_boundary(self) -> bool {
        matches!(self, Self::TrenchWall)
    }

    /// Short string identifier for the kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Station => "station",
            Self::Titan => "titan",
            Self::Interior => "interior",
            Self::TrenchWall => "trench",
        }
    }
}

impl std::fmt::Display for StructureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Zone within a megastructure.
///
/// Used for categorizing chunks belonging to a structure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureZone {
    /// Outside the structure boundary.
    #[default]
    Exterior,
    /// Inside the structure (habitable space).
    Interior,
    /// Structure hull/shell.
    Hull,
    /// Wall segment (for trench walls).
    Wall,
}

impl StructureZone {
    /// Whether this zone is inside the structure.
    #[must_use]
    pub const fn is_inside(self) -> bool {
        matches!(self, Self::Interior | Self::Hull)
    }

    /// Whether this zone is part of the structure body.
    #[must_use]
    pub const fn is_structure(self) -> bool {
        !matches!(self, Self::Exterior)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_priority_order() {
        assert!(
            StructureKind::Interior.streaming_priority()
                < StructureKind::Station.streaming_priority()
        );
        assert!(
            StructureKind::Station.streaming_priority() < StructureKind::Titan.streaming_priority()
        );
        assert!(
            StructureKind::Titan.streaming_priority()
                < StructureKind::TrenchWall.streaming_priority()
        );
    }

    #[test]
    fn test_is_mobile() {
        assert!(!StructureKind::Station.is_mobile());
        assert!(StructureKind::Titan.is_mobile());
        assert!(!StructureKind::Interior.is_mobile());
        assert!(!StructureKind::TrenchWall.is_mobile());
    }

    #[test]
    fn test_has_interior() {
        assert!(StructureKind::Station.has_interior());
        assert!(StructureKind::Titan.has_interior());
        assert!(!StructureKind::Interior.has_interior());
        assert!(!StructureKind::TrenchWall.has_interior());
    }

    #[test]
    fn test_is_boundary() {
        assert!(!StructureKind::Station.is_boundary());
        assert!(StructureKind::TrenchWall.is_boundary());
    }

    #[test]
    fn test_all_variants() {
        assert_eq!(StructureKind::ALL.len(), 4);
    }

    #[test]
    fn test_zone_is_inside() {
        assert!(!StructureZone::Exterior.is_inside());
        assert!(StructureZone::Interior.is_inside());
        assert!(StructureZone::Hull.is_inside());
        assert!(!StructureZone::Wall.is_inside());
    }

    #[test]
    fn test_zone_is_structure() {
        assert!(!StructureZone::Exterior.is_structure());
        assert!(StructureZone::Interior.is_structure());
        assert!(StructureZone::Hull.is_structure());
        assert!(StructureZone::Wall.is_structure());
    }

    #[test]
    fn test_serde_kind() {
        let kind = StructureKind::Titan;
        let serialized = bincode::serialize(&kind).unwrap();
        let deserialized: StructureKind = bincode::deserialize(&serialized).unwrap();
        assert_eq!(kind, deserialized);
    }

    #[test]
    fn test_serde_zone() {
        let zone = StructureZone::Interior;
        let serialized = bincode::serialize(&zone).unwrap();
        let deserialized: StructureZone = bincode::deserialize(&serialized).unwrap();
        assert_eq!(zone, deserialized);
    }
}

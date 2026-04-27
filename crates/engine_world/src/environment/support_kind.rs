//! Structural support type definitions.

use serde::{Deserialize, Serialize};

/// Types of structural support that cells can provide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SupportKind {
    /// No structural support (air, liquids).
    #[default]
    None = 0,

    /// Solid ground or bedrock - provides absolute support.
    Foundation = 1,

    /// Vertical load-bearing column.
    Column = 2,

    /// Horizontal beam spanning between supports.
    Beam = 3,

    /// Diagonal brace for lateral stability.
    Brace = 4,

    /// General solid block with moderate strength.
    Solid = 5,

    /// Weak material that can support limited load.
    Weak = 6,
}

impl SupportKind {
    /// Total number of support kinds.
    pub const COUNT: usize = 7;

    /// All support kinds in order.
    pub const ALL: [SupportKind; Self::COUNT] = [
        SupportKind::None,
        SupportKind::Foundation,
        SupportKind::Column,
        SupportKind::Beam,
        SupportKind::Brace,
        SupportKind::Solid,
        SupportKind::Weak,
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
            0 => Some(SupportKind::None),
            1 => Some(SupportKind::Foundation),
            2 => Some(SupportKind::Column),
            3 => Some(SupportKind::Beam),
            4 => Some(SupportKind::Brace),
            5 => Some(SupportKind::Solid),
            6 => Some(SupportKind::Weak),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            SupportKind::None => "None",
            SupportKind::Foundation => "Foundation",
            SupportKind::Column => "Column",
            SupportKind::Beam => "Beam",
            SupportKind::Brace => "Brace",
            SupportKind::Solid => "Solid",
            SupportKind::Weak => "Weak",
        }
    }

    /// Whether this kind provides any structural support.
    #[must_use]
    pub const fn provides_support(self) -> bool {
        !matches!(self, SupportKind::None)
    }

    /// Whether this is immovable foundation.
    #[must_use]
    pub const fn is_foundation(self) -> bool {
        matches!(self, SupportKind::Foundation)
    }

    /// Maximum load this support type can bear (0.0 to 1.0 normalized).
    #[must_use]
    pub const fn max_load_factor(self) -> f32 {
        match self {
            SupportKind::None => 0.0,
            SupportKind::Foundation => 1.0,
            SupportKind::Column => 0.9,
            SupportKind::Beam => 0.7,
            SupportKind::Brace => 0.5,
            SupportKind::Solid => 0.6,
            SupportKind::Weak => 0.3,
        }
    }

    /// How far support can propagate from this type (in cells).
    #[must_use]
    pub const fn support_range(self) -> u8 {
        match self {
            SupportKind::None => 0,
            SupportKind::Foundation => 255,
            SupportKind::Column => 16,
            SupportKind::Beam => 8,
            SupportKind::Brace => 6,
            SupportKind::Solid => 4,
            SupportKind::Weak => 2,
        }
    }

    /// Whether this type primarily transfers load vertically.
    #[must_use]
    pub const fn vertical_transfer(self) -> bool {
        matches!(self, SupportKind::Column | SupportKind::Foundation)
    }

    /// Whether this type primarily transfers load horizontally.
    #[must_use]
    pub const fn horizontal_transfer(self) -> bool {
        matches!(self, SupportKind::Beam | SupportKind::Brace)
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::uninlined_format_args,
    clippy::manual_range_contains,
    reason = "tests check exact values; format args and range checks clearer in tests"
)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(SupportKind::ALL.len(), SupportKind::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for kind in SupportKind::ALL {
            let index = kind.as_index();
            let recovered = SupportKind::from_index(index);
            assert_eq!(recovered, Some(kind));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(SupportKind::from_index(7), None);
        assert_eq!(SupportKind::from_index(255), None);
    }

    #[test]
    fn provides_support() {
        assert!(!SupportKind::None.provides_support());
        assert!(SupportKind::Foundation.provides_support());
        assert!(SupportKind::Column.provides_support());
        assert!(SupportKind::Solid.provides_support());
    }

    #[test]
    fn foundation_properties() {
        assert!(SupportKind::Foundation.is_foundation());
        assert!(!SupportKind::Column.is_foundation());
        assert_eq!(SupportKind::Foundation.max_load_factor(), 1.0);
        assert_eq!(SupportKind::Foundation.support_range(), 255);
    }

    #[test]
    fn max_load_factor_in_range() {
        for kind in SupportKind::ALL {
            let factor = kind.max_load_factor();
            assert!(factor >= 0.0 && factor <= 1.0, "{:?}", kind);
        }
    }

    #[test]
    fn transfer_directions() {
        assert!(SupportKind::Column.vertical_transfer());
        assert!(!SupportKind::Column.horizontal_transfer());

        assert!(SupportKind::Beam.horizontal_transfer());
        assert!(!SupportKind::Beam.vertical_transfer());

        assert!(!SupportKind::Solid.vertical_transfer());
        assert!(!SupportKind::Solid.horizontal_transfer());
    }

    #[test]
    fn default_is_none() {
        assert_eq!(SupportKind::default(), SupportKind::None);
    }

    #[test]
    fn serde_round_trip() {
        for kind in SupportKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: SupportKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}

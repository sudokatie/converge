//! Hazard type definitions and metadata.

use serde::{Deserialize, Serialize};

/// Types of environmental hazards that can propagate through the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum HazardKind {
    /// Fire spreads to flammable materials, decays without fuel.
    Fire = 0,

    /// Biological infection spreads through organic matter.
    Infection = 1,

    /// Frost spreads through conductive materials, slows entities.
    Frost = 2,

    /// Vacuum propagates when pressure barriers fail.
    Vacuum = 3,

    /// Flood spreads through open spaces, affected by gravity.
    Flood = 4,

    /// Corruption transforms terrain, spreads persistently.
    Corruption = 5,
}

impl HazardKind {
    /// Total number of hazard kinds.
    pub const COUNT: usize = 6;

    /// All hazard kinds in order.
    pub const ALL: [HazardKind; Self::COUNT] = [
        HazardKind::Fire,
        HazardKind::Infection,
        HazardKind::Frost,
        HazardKind::Vacuum,
        HazardKind::Flood,
        HazardKind::Corruption,
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
            0 => Some(HazardKind::Fire),
            1 => Some(HazardKind::Infection),
            2 => Some(HazardKind::Frost),
            3 => Some(HazardKind::Vacuum),
            4 => Some(HazardKind::Flood),
            5 => Some(HazardKind::Corruption),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            HazardKind::Fire => "Fire",
            HazardKind::Infection => "Infection",
            HazardKind::Frost => "Frost",
            HazardKind::Vacuum => "Vacuum",
            HazardKind::Flood => "Flood",
            HazardKind::Corruption => "Corruption",
        }
    }

    /// Whether this hazard spreads preferentially downward (gravity-affected).
    #[must_use]
    pub const fn gravity_affected(self) -> bool {
        matches!(self, HazardKind::Flood | HazardKind::Fire)
    }

    /// Whether this hazard can exist at zero intensity (vacuum is presence-based).
    #[must_use]
    pub const fn presence_based(self) -> bool {
        matches!(self, HazardKind::Vacuum)
    }

    /// Default intensity when hazard first appears (0.0-1.0).
    #[must_use]
    pub const fn default_intensity(self) -> f32 {
        match self {
            HazardKind::Fire => 0.5,
            HazardKind::Infection => 0.3,
            HazardKind::Frost => 0.4,
            HazardKind::Vacuum => 1.0,
            HazardKind::Flood => 0.8,
            HazardKind::Corruption => 0.2,
        }
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
        assert_eq!(HazardKind::ALL.len(), HazardKind::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for kind in HazardKind::ALL {
            let index = kind.as_index();
            let recovered = HazardKind::from_index(index);
            assert_eq!(recovered, Some(kind));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(HazardKind::from_index(6), None);
        assert_eq!(HazardKind::from_index(255), None);
    }

    #[test]
    fn gravity_affected_kinds() {
        assert!(HazardKind::Flood.gravity_affected());
        assert!(HazardKind::Fire.gravity_affected());
        assert!(!HazardKind::Frost.gravity_affected());
    }

    #[test]
    fn presence_based_vacuum() {
        assert!(HazardKind::Vacuum.presence_based());
        assert!(!HazardKind::Fire.presence_based());
    }

    #[test]
    fn default_intensity_in_range() {
        for kind in HazardKind::ALL {
            let intensity = kind.default_intensity();
            assert!(intensity >= 0.0 && intensity <= 1.0, "{:?}", kind);
        }
    }

    #[test]
    fn serde_round_trip() {
        for kind in HazardKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: HazardKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}

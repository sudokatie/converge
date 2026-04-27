//! Fluid kind enumeration for volume transport.

use serde::{Deserialize, Serialize};

/// Kinds of fluids supported by the transport system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum FluidKind {
    /// Standard liquid water.
    Water = 0,
    /// Gaseous fluid (air, steam, toxic gas).
    Gas = 1,
    /// Dense mixture of liquid and solite (mud, concrete).
    Slurry = 2,
    /// High-temperature molten material.
    Lava = 3,
}

impl FluidKind {
    /// Number of fluid kinds.
    pub const COUNT: usize = 4;

    /// All fluid kinds in order.
    pub const ALL: [FluidKind; Self::COUNT] = [
        FluidKind::Water,
        FluidKind::Gas,
        FluidKind::Slurry,
        FluidKind::Lava,
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
            0 => Some(FluidKind::Water),
            1 => Some(FluidKind::Gas),
            2 => Some(FluidKind::Slurry),
            3 => Some(FluidKind::Lava),
            _ => None,
        }
    }

    /// Base viscosity factor (higher = slower flow).
    #[must_use]
    pub const fn base_viscosity(self) -> f32 {
        match self {
            FluidKind::Water => 1.0,
            FluidKind::Gas => 0.1,
            FluidKind::Slurry => 5.0,
            FluidKind::Lava => 10.0,
        }
    }

    /// Whether this fluid rises (gas) or falls (liquids).
    #[must_use]
    pub const fn rises(self) -> bool {
        matches!(self, FluidKind::Gas)
    }

    /// Default temperature for this fluid kind in Celsius.
    #[must_use]
    pub const fn default_temperature(self) -> f32 {
        match self {
            FluidKind::Water | FluidKind::Gas => 20.0,
            FluidKind::Slurry => 15.0,
            FluidKind::Lava => 1200.0,
        }
    }

    /// Evaporation rate factor (volume lost per second per degree above threshold).
    #[must_use]
    pub const fn evaporation_rate(self) -> f32 {
        match self {
            FluidKind::Water => 0.001,
            FluidKind::Gas | FluidKind::Lava => 0.0,
            FluidKind::Slurry => 0.0005,
        }
    }

    /// Temperature above which evaporation occurs.
    #[must_use]
    pub const fn evaporation_threshold(self) -> f32 {
        match self {
            FluidKind::Water | FluidKind::Slurry => 100.0,
            FluidKind::Gas | FluidKind::Lava => f32::MAX,
        }
    }

    /// Cooling rate (degrees lost per second).
    #[must_use]
    pub const fn cooling_rate(self) -> f32 {
        match self {
            FluidKind::Water => 0.5,
            FluidKind::Gas => 1.0,
            FluidKind::Slurry => 0.3,
            FluidKind::Lava => 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trip() {
        for kind in FluidKind::ALL {
            assert_eq!(FluidKind::from_index(kind.as_index()), Some(kind));
        }
    }

    #[test]
    fn from_index_invalid() {
        assert_eq!(FluidKind::from_index(4), None);
        assert_eq!(FluidKind::from_index(100), None);
    }

    #[test]
    fn count_matches_all() {
        assert_eq!(FluidKind::ALL.len(), FluidKind::COUNT);
    }

    #[test]
    fn viscosity_ordering() {
        assert!(FluidKind::Gas.base_viscosity() < FluidKind::Water.base_viscosity());
        assert!(FluidKind::Water.base_viscosity() < FluidKind::Slurry.base_viscosity());
        assert!(FluidKind::Slurry.base_viscosity() < FluidKind::Lava.base_viscosity());
    }

    #[test]
    fn rises_only_gas() {
        assert!(!FluidKind::Water.rises());
        assert!(FluidKind::Gas.rises());
        assert!(!FluidKind::Slurry.rises());
        assert!(!FluidKind::Lava.rises());
    }

    #[test]
    fn serde_round_trip() {
        for kind in FluidKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: FluidKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}

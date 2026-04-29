//! Conduit type definitions for base/station infrastructure networks.

use serde::{Deserialize, Serialize};

/// Types of conduits for transporting resources through infrastructure.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum ConduitKind {
    /// Electrical power transmission.
    #[default]
    Power = 0,
    /// Thermal energy transfer (heating/cooling).
    Heat = 1,
    /// Liquid/gas fluid transport.
    Fluid = 2,
    /// Data/control signal transmission.
    Signal = 3,
}

impl ConduitKind {
    /// Number of conduit kinds.
    pub const COUNT: usize = 4;

    /// All conduit kinds in index order.
    pub const ALL: [ConduitKind; Self::COUNT] = [
        ConduitKind::Power,
        ConduitKind::Heat,
        ConduitKind::Fluid,
        ConduitKind::Signal,
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
            0 => Some(ConduitKind::Power),
            1 => Some(ConduitKind::Heat),
            2 => Some(ConduitKind::Fluid),
            3 => Some(ConduitKind::Signal),
            _ => None,
        }
    }

    /// Display name for the conduit type.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            ConduitKind::Power => "Power",
            ConduitKind::Heat => "Heat",
            ConduitKind::Fluid => "Fluid",
            ConduitKind::Signal => "Signal",
        }
    }

    /// Base capacity factor (units per second at full throughput).
    #[must_use]
    pub const fn base_capacity(self) -> f32 {
        match self {
            ConduitKind::Power => 100.0,
            ConduitKind::Heat => 50.0,
            ConduitKind::Fluid => 10.0,
            ConduitKind::Signal => 1000.0,
        }
    }

    /// Base resistance factor (loss per unit length).
    #[must_use]
    pub const fn base_resistance(self) -> f32 {
        match self {
            ConduitKind::Power => 0.02,
            ConduitKind::Heat => 0.05,
            ConduitKind::Fluid => 0.01,
            ConduitKind::Signal => 0.001,
        }
    }

    /// Base loss factor (fraction lost per step without active transfer).
    #[must_use]
    pub const fn base_loss(self) -> f32 {
        match self {
            ConduitKind::Heat => 0.01,
            ConduitKind::Power | ConduitKind::Fluid | ConduitKind::Signal => 0.0,
        }
    }

    /// Whether this conduit type can store resources.
    #[must_use]
    pub const fn can_store(self) -> bool {
        !matches!(self, ConduitKind::Signal)
    }

    /// Whether this conduit type uses pressure-based flow.
    #[must_use]
    pub const fn uses_pressure(self) -> bool {
        matches!(self, ConduitKind::Fluid)
    }

    /// Whether this conduit type uses temperature gradients.
    #[must_use]
    pub const fn uses_temperature(self) -> bool {
        matches!(self, ConduitKind::Heat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(ConduitKind::ALL.len(), ConduitKind::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for kind in ConduitKind::ALL {
            let index = kind.as_index();
            let recovered = ConduitKind::from_index(index);
            assert_eq!(recovered, Some(kind));
        }
    }

    #[test]
    fn from_index_invalid() {
        assert_eq!(ConduitKind::from_index(4), None);
        assert_eq!(ConduitKind::from_index(100), None);
    }

    #[test]
    fn all_have_names() {
        for kind in ConduitKind::ALL {
            assert!(!kind.name().is_empty());
        }
    }

    #[test]
    fn capacity_positive() {
        for kind in ConduitKind::ALL {
            assert!(kind.base_capacity() > 0.0);
        }
    }

    #[test]
    fn resistance_non_negative() {
        for kind in ConduitKind::ALL {
            assert!(kind.base_resistance() >= 0.0);
        }
    }

    #[test]
    fn loss_non_negative() {
        for kind in ConduitKind::ALL {
            assert!(kind.base_loss() >= 0.0);
            assert!(kind.base_loss() <= 1.0);
        }
    }

    #[test]
    fn fluid_uses_pressure() {
        assert!(ConduitKind::Fluid.uses_pressure());
        assert!(!ConduitKind::Power.uses_pressure());
        assert!(!ConduitKind::Heat.uses_pressure());
        assert!(!ConduitKind::Signal.uses_pressure());
    }

    #[test]
    fn heat_uses_temperature() {
        assert!(ConduitKind::Heat.uses_temperature());
        assert!(!ConduitKind::Power.uses_temperature());
        assert!(!ConduitKind::Fluid.uses_temperature());
        assert!(!ConduitKind::Signal.uses_temperature());
    }

    #[test]
    fn signal_cannot_store() {
        assert!(!ConduitKind::Signal.can_store());
        assert!(ConduitKind::Power.can_store());
        assert!(ConduitKind::Heat.can_store());
        assert!(ConduitKind::Fluid.can_store());
    }

    #[test]
    fn serde_round_trip() {
        for kind in ConduitKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: ConduitKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}

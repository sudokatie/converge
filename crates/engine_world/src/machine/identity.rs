//! Machine identity types.

use serde::{Deserialize, Serialize};

/// Unique identifier for a machine instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MachineId(pub u64);

impl MachineId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl From<u64> for MachineId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Category of machine determining base behavior.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum MachineCategory {
    /// Manual crafting station (workbench, assembly table).
    #[default]
    Crafting = 0,
    /// Automated processor (refiner, smelter, chemical processor).
    Processor = 1,
    /// Power generator (reactor, generator, fuel cell).
    Reactor = 2,
    /// Growth chamber (incubator, bioreactor, greenhouse).
    Incubator = 3,
    /// Life support system (scrubber, pressurizer, HVAC).
    LifeSupport = 4,
}

impl MachineCategory {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Crafting => "crafting",
            Self::Processor => "processor",
            Self::Reactor => "reactor",
            Self::Incubator => "incubator",
            Self::LifeSupport => "life_support",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [MachineCategory] {
        &[
            Self::Crafting,
            Self::Processor,
            Self::Reactor,
            Self::Incubator,
            Self::LifeSupport,
        ]
    }

    #[must_use]
    pub const fn requires_power(&self) -> bool {
        matches!(self, Self::Processor | Self::Incubator | Self::LifeSupport)
    }

    #[must_use]
    pub const fn produces_power(&self) -> bool {
        matches!(self, Self::Reactor)
    }

    #[must_use]
    pub const fn produces_heat(&self) -> bool {
        matches!(self, Self::Reactor | Self::Processor)
    }

    #[must_use]
    pub const fn affects_atmosphere(&self) -> bool {
        matches!(self, Self::LifeSupport | Self::Reactor)
    }

    #[must_use]
    pub const fn supports_queue(&self) -> bool {
        matches!(self, Self::Crafting | Self::Processor)
    }

    #[must_use]
    pub const fn is_continuous(&self) -> bool {
        matches!(self, Self::Reactor | Self::LifeSupport)
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Crafting),
            1 => Some(Self::Processor),
            2 => Some(Self::Reactor),
            3 => Some(Self::Incubator),
            4 => Some(Self::LifeSupport),
            _ => None,
        }
    }
}

/// Machine quality tier affecting efficiency and capacity.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum MachineTier {
    /// Basic tier - standard efficiency.
    #[default]
    Basic = 0,
    /// Standard tier - improved efficiency.
    Standard = 1,
    /// Advanced tier - high efficiency.
    Advanced = 2,
    /// Elite tier - maximum efficiency.
    Elite = 3,
}

impl MachineTier {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Standard => "standard",
            Self::Advanced => "advanced",
            Self::Elite => "elite",
        }
    }

    #[must_use]
    pub const fn speed_multiplier(&self) -> f32 {
        match self {
            Self::Basic => 1.0,
            Self::Standard => 1.25,
            Self::Advanced => 1.5,
            Self::Elite => 2.0,
        }
    }

    #[must_use]
    pub const fn efficiency_multiplier(&self) -> f32 {
        match self {
            Self::Basic => 1.0,
            Self::Standard => 1.1,
            Self::Advanced => 1.25,
            Self::Elite => 1.5,
        }
    }

    #[must_use]
    pub const fn capacity_multiplier(&self) -> f32 {
        match self {
            Self::Basic => 1.0,
            Self::Standard => 1.5,
            Self::Advanced => 2.0,
            Self::Elite => 3.0,
        }
    }

    #[must_use]
    pub const fn maintenance_interval_multiplier(&self) -> f32 {
        match self {
            Self::Basic => 1.0,
            Self::Standard => 1.25,
            Self::Advanced => 1.5,
            Self::Elite => 2.0,
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Basic),
            1 => Some(Self::Standard),
            2 => Some(Self::Advanced),
            3 => Some(Self::Elite),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_id_ordering() {
        let a = MachineId::new(1);
        let b = MachineId::new(2);
        assert!(a < b);
    }

    #[test]
    fn category_properties() {
        assert!(MachineCategory::Reactor.produces_power());
        assert!(MachineCategory::Reactor.produces_heat());
        assert!(MachineCategory::LifeSupport.affects_atmosphere());
        assert!(MachineCategory::Processor.requires_power());
        assert!(MachineCategory::Crafting.supports_queue());
        assert!(MachineCategory::Reactor.is_continuous());
    }

    #[test]
    fn tier_multipliers() {
        assert!((MachineTier::Basic.speed_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((MachineTier::Elite.speed_multiplier() - 2.0).abs() < f32::EPSILON);
        assert!((MachineTier::Elite.efficiency_multiplier() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn category_all() {
        let all = MachineCategory::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn category_from_raw() {
        assert_eq!(
            MachineCategory::from_raw(0),
            Some(MachineCategory::Crafting)
        );
        assert_eq!(
            MachineCategory::from_raw(4),
            Some(MachineCategory::LifeSupport)
        );
        assert_eq!(MachineCategory::from_raw(5), None);
    }

    #[test]
    fn serde_machine_id() {
        let id = MachineId::new(12345);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: MachineId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn serde_category() {
        let cat = MachineCategory::Reactor;
        let json = serde_json::to_string(&cat).unwrap();
        let recovered: MachineCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(cat, recovered);
    }

    #[test]
    fn serde_tier() {
        let tier = MachineTier::Advanced;
        let json = serde_json::to_string(&tier).unwrap();
        let recovered: MachineTier = serde_json::from_str(&json).unwrap();
        assert_eq!(tier, recovered);
    }
}

//! Unified diagnostic channel types.

use serde::{Deserialize, Serialize};

use crate::environment::{ConduitKind, FieldChannel, FluidKind, HazardKind, VectorFieldChannel};

/// High-level diagnostic category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiagnosticCategory {
    ScalarField = 0,
    VectorField = 1,
    Hazard = 2,
    Fluid = 3,
    Structural = 4,
    Conduit = 5,
    Atmosphere = 6,
    Scheduler = 7,
    Custom = 8,
}

impl DiagnosticCategory {
    pub const COUNT: usize = 9;

    pub const ALL: [DiagnosticCategory; Self::COUNT] = [
        DiagnosticCategory::ScalarField,
        DiagnosticCategory::VectorField,
        DiagnosticCategory::Hazard,
        DiagnosticCategory::Fluid,
        DiagnosticCategory::Structural,
        DiagnosticCategory::Conduit,
        DiagnosticCategory::Atmosphere,
        DiagnosticCategory::Scheduler,
        DiagnosticCategory::Custom,
    ];

    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(DiagnosticCategory::ScalarField),
            1 => Some(DiagnosticCategory::VectorField),
            2 => Some(DiagnosticCategory::Hazard),
            3 => Some(DiagnosticCategory::Fluid),
            4 => Some(DiagnosticCategory::Structural),
            5 => Some(DiagnosticCategory::Conduit),
            6 => Some(DiagnosticCategory::Atmosphere),
            7 => Some(DiagnosticCategory::Scheduler),
            8 => Some(DiagnosticCategory::Custom),
            _ => None,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            DiagnosticCategory::ScalarField => "Scalar Fields",
            DiagnosticCategory::VectorField => "Vector Fields",
            DiagnosticCategory::Hazard => "Hazards",
            DiagnosticCategory::Fluid => "Fluids",
            DiagnosticCategory::Structural => "Structural",
            DiagnosticCategory::Conduit => "Conduits",
            DiagnosticCategory::Atmosphere => "Atmosphere",
            DiagnosticCategory::Scheduler => "Scheduler",
            DiagnosticCategory::Custom => "Custom",
        }
    }
}

/// Unified diagnostic channel combining all field/hazard/fluid/structural/conduit/atmosphere types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticChannel {
    Scalar(FieldChannel),
    Vector(VectorFieldChannel),
    Hazard(HazardKind),
    Fluid(FluidKind),
    Structural(StructuralChannel),
    Conduit(ConduitKind),
    Atmosphere(AtmosphereChannel),
    Scheduler(SchedulerChannel),
    Custom(u8),
}

/// Structural diagnostic sub-channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum StructuralChannel {
    SupportKind = 0,
    Load = 1,
    Stress = 2,
    Integrity = 3,
    SupportDistance = 4,
}

impl StructuralChannel {
    pub const COUNT: usize = 5;

    pub const ALL: [StructuralChannel; Self::COUNT] = [
        StructuralChannel::SupportKind,
        StructuralChannel::Load,
        StructuralChannel::Stress,
        StructuralChannel::Integrity,
        StructuralChannel::SupportDistance,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            StructuralChannel::SupportKind => "Support Kind",
            StructuralChannel::Load => "Load",
            StructuralChannel::Stress => "Stress",
            StructuralChannel::Integrity => "Integrity",
            StructuralChannel::SupportDistance => "Support Distance",
        }
    }
}

/// Atmosphere diagnostic sub-channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum AtmosphereChannel {
    Layer = 0,
    SealQuality = 1,
    Ventilation = 2,
    Contamination = 3,
}

impl AtmosphereChannel {
    pub const COUNT: usize = 4;

    pub const ALL: [AtmosphereChannel; Self::COUNT] = [
        AtmosphereChannel::Layer,
        AtmosphereChannel::SealQuality,
        AtmosphereChannel::Ventilation,
        AtmosphereChannel::Contamination,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            AtmosphereChannel::Layer => "Layer",
            AtmosphereChannel::SealQuality => "Seal Quality",
            AtmosphereChannel::Ventilation => "Ventilation",
            AtmosphereChannel::Contamination => "Contamination",
        }
    }
}

/// Scheduler diagnostic sub-channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum SchedulerChannel {
    Fidelity = 0,
    Interest = 1,
    Distance = 2,
    Priority = 3,
    Accumulated = 4,
}

impl SchedulerChannel {
    pub const COUNT: usize = 5;

    pub const ALL: [SchedulerChannel; Self::COUNT] = [
        SchedulerChannel::Fidelity,
        SchedulerChannel::Interest,
        SchedulerChannel::Distance,
        SchedulerChannel::Priority,
        SchedulerChannel::Accumulated,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            SchedulerChannel::Fidelity => "Fidelity",
            SchedulerChannel::Interest => "Interest",
            SchedulerChannel::Distance => "Distance",
            SchedulerChannel::Priority => "Priority",
            SchedulerChannel::Accumulated => "Accumulated",
        }
    }
}

impl DiagnosticChannel {
    #[must_use]
    pub const fn category(self) -> DiagnosticCategory {
        match self {
            DiagnosticChannel::Scalar(_) => DiagnosticCategory::ScalarField,
            DiagnosticChannel::Vector(_) => DiagnosticCategory::VectorField,
            DiagnosticChannel::Hazard(_) => DiagnosticCategory::Hazard,
            DiagnosticChannel::Fluid(_) => DiagnosticCategory::Fluid,
            DiagnosticChannel::Structural(_) => DiagnosticCategory::Structural,
            DiagnosticChannel::Conduit(_) => DiagnosticCategory::Conduit,
            DiagnosticChannel::Atmosphere(_) => DiagnosticCategory::Atmosphere,
            DiagnosticChannel::Scheduler(_) => DiagnosticCategory::Scheduler,
            DiagnosticChannel::Custom(_) => DiagnosticCategory::Custom,
        }
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            DiagnosticChannel::Scalar(c) => c.name(),
            DiagnosticChannel::Vector(c) => c.name(),
            DiagnosticChannel::Hazard(k) => k.name(),
            DiagnosticChannel::Fluid(k) => k.name(),
            DiagnosticChannel::Structural(c) => c.name(),
            DiagnosticChannel::Conduit(k) => k.name(),
            DiagnosticChannel::Atmosphere(c) => c.name(),
            DiagnosticChannel::Scheduler(c) => c.name(),
            DiagnosticChannel::Custom(_) => "Custom",
        }
    }

    pub fn all_scalar() -> impl Iterator<Item = DiagnosticChannel> {
        FieldChannel::ALL.into_iter().map(DiagnosticChannel::Scalar)
    }

    pub fn all_vector() -> impl Iterator<Item = DiagnosticChannel> {
        VectorFieldChannel::ALL
            .into_iter()
            .map(DiagnosticChannel::Vector)
    }

    pub fn all_hazard() -> impl Iterator<Item = DiagnosticChannel> {
        HazardKind::ALL.into_iter().map(DiagnosticChannel::Hazard)
    }

    pub fn all_fluid() -> impl Iterator<Item = DiagnosticChannel> {
        FluidKind::ALL.into_iter().map(DiagnosticChannel::Fluid)
    }

    pub fn all_structural() -> impl Iterator<Item = DiagnosticChannel> {
        StructuralChannel::ALL
            .into_iter()
            .map(DiagnosticChannel::Structural)
    }

    pub fn all_conduit() -> impl Iterator<Item = DiagnosticChannel> {
        ConduitKind::ALL.into_iter().map(DiagnosticChannel::Conduit)
    }

    pub fn all_atmosphere() -> impl Iterator<Item = DiagnosticChannel> {
        AtmosphereChannel::ALL
            .into_iter()
            .map(DiagnosticChannel::Atmosphere)
    }

    pub fn all_scheduler() -> impl Iterator<Item = DiagnosticChannel> {
        SchedulerChannel::ALL
            .into_iter()
            .map(DiagnosticChannel::Scheduler)
    }
}

impl From<FieldChannel> for DiagnosticChannel {
    fn from(c: FieldChannel) -> Self {
        DiagnosticChannel::Scalar(c)
    }
}

impl From<VectorFieldChannel> for DiagnosticChannel {
    fn from(c: VectorFieldChannel) -> Self {
        DiagnosticChannel::Vector(c)
    }
}

impl From<HazardKind> for DiagnosticChannel {
    fn from(k: HazardKind) -> Self {
        DiagnosticChannel::Hazard(k)
    }
}

impl From<FluidKind> for DiagnosticChannel {
    fn from(k: FluidKind) -> Self {
        DiagnosticChannel::Fluid(k)
    }
}

impl From<ConduitKind> for DiagnosticChannel {
    fn from(k: ConduitKind) -> Self {
        DiagnosticChannel::Conduit(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_round_trip() {
        for cat in DiagnosticCategory::ALL {
            let idx = cat.as_index();
            assert_eq!(DiagnosticCategory::from_index(idx), Some(cat));
        }
    }

    #[test]
    fn test_category_out_of_range() {
        assert_eq!(DiagnosticCategory::from_index(100), None);
    }

    #[test]
    fn test_channel_category() {
        assert_eq!(
            DiagnosticChannel::Scalar(FieldChannel::Temperature).category(),
            DiagnosticCategory::ScalarField
        );
        assert_eq!(
            DiagnosticChannel::Vector(VectorFieldChannel::Wind).category(),
            DiagnosticCategory::VectorField
        );
        assert_eq!(
            DiagnosticChannel::Hazard(HazardKind::Fire).category(),
            DiagnosticCategory::Hazard
        );
        assert_eq!(
            DiagnosticChannel::Custom(42).category(),
            DiagnosticCategory::Custom
        );
    }

    #[test]
    fn test_channel_name() {
        assert_eq!(
            DiagnosticChannel::Scalar(FieldChannel::Radiation).name(),
            "Radiation"
        );
        assert_eq!(
            DiagnosticChannel::Structural(StructuralChannel::Stress).name(),
            "Stress"
        );
    }

    #[test]
    fn test_from_conversions() {
        let ch: DiagnosticChannel = FieldChannel::Oxygen.into();
        assert_eq!(ch, DiagnosticChannel::Scalar(FieldChannel::Oxygen));

        let ch: DiagnosticChannel = HazardKind::Frost.into();
        assert_eq!(ch, DiagnosticChannel::Hazard(HazardKind::Frost));
    }

    #[test]
    fn test_deterministic_ordering() {
        let mut channels: Vec<DiagnosticChannel> = vec![
            DiagnosticChannel::Hazard(HazardKind::Fire),
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            DiagnosticChannel::Vector(VectorFieldChannel::Wind),
            DiagnosticChannel::Custom(1),
        ];
        channels.sort();
        channels.sort();
        let second_sort = channels.clone();
        channels.sort();
        assert_eq!(second_sort, channels);
    }
}

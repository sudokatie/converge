//! Geological identity types.

use serde::{Deserialize, Serialize};

/// Unique identifier for a geological layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LayerId(pub u32);

impl LayerId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

impl From<u32> for LayerId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for LayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "layer:{}", self.0)
    }
}

/// Unique identifier for a geological material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MaterialId(pub u32);

impl MaterialId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

impl From<u32> for MaterialId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for MaterialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mat:{}", self.0)
    }
}

/// Unique identifier for a geological feature (fault, pocket, seam).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FeatureId(pub u64);

impl FeatureId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl From<u64> for FeatureId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for FeatureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "feat:{}", self.0)
    }
}

/// Kind of geological feature.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum FeatureKind {
    /// Fault line for tectonic activity.
    #[default]
    Fault = 0,
    /// Magma pocket.
    MagmaPocket = 1,
    /// Magma flow channel.
    MagmaFlow = 2,
    /// Crystal seam deposit.
    CrystalSeam = 3,
    /// Mineral deposit.
    MineralDeposit = 4,
}

impl FeatureKind {
    pub const ALL: [FeatureKind; 5] = [
        Self::Fault,
        Self::MagmaPocket,
        Self::MagmaFlow,
        Self::CrystalSeam,
        Self::MineralDeposit,
    ];

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Fault => "fault",
            Self::MagmaPocket => "magma_pocket",
            Self::MagmaFlow => "magma_flow",
            Self::CrystalSeam => "crystal_seam",
            Self::MineralDeposit => "mineral_deposit",
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Fault),
            1 => Some(Self::MagmaPocket),
            2 => Some(Self::MagmaFlow),
            3 => Some(Self::CrystalSeam),
            4 => Some(Self::MineralDeposit),
            _ => None,
        }
    }
}

/// Type of rock material.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum RockType {
    /// Igneous rock (volcanic origin).
    #[default]
    Igneous = 0,
    /// Sedimentary rock (layered deposits).
    Sedimentary = 1,
    /// Metamorphic rock (transformed by heat/pressure).
    Metamorphic = 2,
    /// Crystalline rock (high crystal content).
    Crystalline = 3,
    /// Volcanic rock (recent volcanic activity).
    Volcanic = 4,
    /// Molten material.
    Molite = 5,
}

impl RockType {
    pub const ALL: [RockType; 6] = [
        Self::Igneous,
        Self::Sedimentary,
        Self::Metamorphic,
        Self::Crystalline,
        Self::Volcanic,
        Self::Molite,
    ];

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Igneous => "igneous",
            Self::Sedimentary => "sedimentary",
            Self::Metamorphic => "metamorphic",
            Self::Crystalline => "crystalline",
            Self::Volcanic => "volcanic",
            Self::Molite => "molite",
        }
    }

    #[must_use]
    pub const fn base_density(&self) -> f32 {
        match self {
            Self::Igneous => 2.7,
            Self::Sedimentary => 2.3,
            Self::Metamorphic => 2.8,
            Self::Crystalline => 2.6,
            Self::Volcanic => 2.4,
            Self::Molite => 2.9,
        }
    }

    #[must_use]
    pub const fn thermal_conductivity(&self) -> f32 {
        match self {
            Self::Igneous => 2.5,
            Self::Sedimentary => 1.5,
            Self::Metamorphic => 3.0,
            Self::Crystalline => 4.0,
            Self::Volcanic => 1.8,
            Self::Molite => 0.5,
        }
    }

    #[must_use]
    pub const fn compressive_strength(&self) -> f32 {
        match self {
            Self::Igneous => 200.0,
            Self::Sedimentary => 80.0,
            Self::Metamorphic => 250.0,
            Self::Crystalline => 150.0,
            Self::Volcanic => 100.0,
            Self::Molite => 0.0,
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Igneous),
            1 => Some(Self::Sedimentary),
            2 => Some(Self::Metamorphic),
            3 => Some(Self::Crystalline),
            4 => Some(Self::Volcanic),
            5 => Some(Self::Molite),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_id_ordering() {
        let a = LayerId::new(1);
        let b = LayerId::new(2);
        assert!(a < b);
        assert_eq!(a.raw(), 1);
        assert_eq!(format!("{a}"), "layer:1");
    }

    #[test]
    fn material_id_ordering() {
        let a = MaterialId::new(10);
        let b = MaterialId::new(20);
        assert!(a < b);
        assert_eq!(format!("{a}"), "mat:10");
    }

    #[test]
    fn feature_id_display() {
        let id = FeatureId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "feat:42");
    }

    #[test]
    fn feature_kind_properties() {
        assert_eq!(FeatureKind::Fault.name(), "fault");
        assert_eq!(FeatureKind::from_raw(0), Some(FeatureKind::Fault));
        assert_eq!(FeatureKind::from_raw(5), None);
        assert_eq!(FeatureKind::ALL.len(), 5);
    }

    #[test]
    fn rock_type_properties() {
        assert_eq!(RockType::Igneous.name(), "igneous");
        assert!((RockType::Igneous.base_density() - 2.7).abs() < f32::EPSILON);
        assert!(
            RockType::Metamorphic.compressive_strength()
                > RockType::Sedimentary.compressive_strength()
        );
        assert!(
            RockType::Crystalline.thermal_conductivity()
                > RockType::Volcanic.thermal_conductivity()
        );
    }

    #[test]
    fn rock_type_from_raw() {
        for (i, rt) in RockType::ALL.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let idx = i as u8;
            assert_eq!(RockType::from_raw(idx), Some(*rt));
        }
        assert_eq!(RockType::from_raw(10), None);
    }

    #[test]
    fn serde_layer_id() {
        let id = LayerId::new(123);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: LayerId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn serde_material_id() {
        let id = MaterialId::new(456);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: MaterialId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn serde_feature_id() {
        let id = FeatureId::new(789);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: FeatureId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn serde_feature_kind() {
        let kind = FeatureKind::CrystalSeam;
        let json = serde_json::to_string(&kind).unwrap();
        let recovered: FeatureKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, recovered);
    }

    #[test]
    fn serde_rock_type() {
        let rt = RockType::Metamorphic;
        let json = serde_json::to_string(&rt).unwrap();
        let recovered: RockType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, recovered);
    }
}

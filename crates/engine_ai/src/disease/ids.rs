//! Unique identifiers for the disease system.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;

/// Unique identifier for a pathogen type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PathogenId(pub String);

impl PathogenId {
    pub const PLAGUE: &'static str = "plague";
    pub const BLIGHT: &'static str = "blight";
    pub const ROT: &'static str = "rot";
    pub const SPORE_LUNG: &'static str = "spore_lung";
    pub const WASTING: &'static str = "wasting";
    pub const FEVER: &'static str = "fever";

    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn plague() -> Self {
        Self::new(Self::PLAGUE)
    }

    #[must_use]
    pub fn blight() -> Self {
        Self::new(Self::BLIGHT)
    }

    #[must_use]
    pub fn rot() -> Self {
        Self::new(Self::ROT)
    }

    #[must_use]
    pub fn spore_lung() -> Self {
        Self::new(Self::SPORE_LUNG)
    }

    #[must_use]
    pub fn wasting() -> Self {
        Self::new(Self::WASTING)
    }

    #[must_use]
    pub fn fever() -> Self {
        Self::new(Self::FEVER)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for PathogenId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for PathogenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a contamination zone.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContaminationZoneId(pub u64);

impl ContaminationZoneId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ContaminationZoneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zone:{}", self.0)
    }
}

/// Unique identifier for a host entity.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct HostId(pub u64);

impl HostId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for HostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "host:{}", self.0)
    }
}

/// Unique identifier for a disease strain (pathogen + mutation variant).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StrainId {
    pub pathogen: PathogenId,
    pub variant: u32,
}

impl StrainId {
    #[must_use]
    pub fn new(pathogen: PathogenId, variant: u32) -> Self {
        Self { pathogen, variant }
    }

    #[must_use]
    pub fn base(pathogen: PathogenId) -> Self {
        Self::new(pathogen, 0)
    }
}

impl fmt::Display for StrainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.variant == 0 {
            write!(f, "{}", self.pathogen)
        } else {
            write!(f, "{}:v{}", self.pathogen, self.variant)
        }
    }
}

/// Unique identifier for a region (for spread planning).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DiseaseRegionId(pub String);

impl DiseaseRegionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiseaseRegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathogen_id_constants() {
        assert_eq!(PathogenId::plague().as_str(), "plague");
        assert_eq!(PathogenId::blight().as_str(), "blight");
        assert_eq!(PathogenId::rot().as_str(), "rot");
        assert_eq!(PathogenId::spore_lung().as_str(), "spore_lung");
        assert_eq!(PathogenId::wasting().as_str(), "wasting");
        assert_eq!(PathogenId::fever().as_str(), "fever");
    }

    #[test]
    fn test_pathogen_id_from() {
        let id: PathogenId = "custom".into();
        assert_eq!(id.as_str(), "custom");
    }

    #[test]
    fn test_pathogen_id_display() {
        assert_eq!(format!("{}", PathogenId::plague()), "plague");
    }

    #[test]
    fn test_contamination_zone_id() {
        let id = ContaminationZoneId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "zone:42");
    }

    #[test]
    fn test_host_id() {
        let id = HostId::new(123);
        assert_eq!(id.raw(), 123);
        assert_eq!(format!("{id}"), "host:123");
    }

    #[test]
    fn test_strain_id_base() {
        let strain = StrainId::base(PathogenId::plague());
        assert_eq!(strain.variant, 0);
        assert_eq!(format!("{strain}"), "plague");
    }

    #[test]
    fn test_strain_id_variant() {
        let strain = StrainId::new(PathogenId::plague(), 3);
        assert_eq!(format!("{strain}"), "plague:v3");
    }

    #[test]
    fn test_disease_region_id() {
        let id = DiseaseRegionId::new("sector_7g");
        assert_eq!(id.as_str(), "sector_7g");
    }

    #[test]
    fn test_serde_pathogen_id() {
        let id = PathogenId::plague();
        let json = serde_json::to_string(&id).unwrap();
        let restored: PathogenId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn test_serde_strain_id() {
        let strain = StrainId::new(PathogenId::blight(), 5);
        let json = serde_json::to_string(&strain).unwrap();
        let restored: StrainId = serde_json::from_str(&json).unwrap();
        assert_eq!(strain, restored);
    }
}

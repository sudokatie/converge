//! Typed identifiers for portal culling system.

use serde::{Deserialize, Serialize};

/// Unique identifier for a cull region.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct CullRegionId(u64);

impl CullRegionId {
    /// Create a new cull region ID.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CullRegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cull:{:016x}", self.0)
    }
}

impl From<u64> for CullRegionId {
    fn from(value: u64) -> Self {
        Self::from_raw(value)
    }
}

/// Unique identifier for a render pass.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct RenderPassId(u64);

impl RenderPassId {
    /// Create a new render pass ID.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for RenderPassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pass:{:016x}", self.0)
    }
}

impl From<u64> for RenderPassId {
    fn from(value: u64) -> Self {
        Self::from_raw(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cull_region_id_basics() {
        let id = CullRegionId::from_raw(42);
        assert_eq!(id.raw(), 42);
        assert!(format!("{id}").contains("cull:"));
    }

    #[test]
    fn render_pass_id_basics() {
        let id = RenderPassId::from_raw(99);
        assert_eq!(id.raw(), 99);
        assert!(format!("{id}").contains("pass:"));
    }

    #[test]
    fn ordering() {
        let id1 = CullRegionId::from_raw(1);
        let id2 = CullRegionId::from_raw(2);
        assert!(id1 < id2);
    }

    #[test]
    fn serde_roundtrip() {
        let id = CullRegionId::from_raw(12345);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: CullRegionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }
}

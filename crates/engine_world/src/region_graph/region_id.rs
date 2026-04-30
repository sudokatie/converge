//! Unique identifier for regions in the graph.

use serde::{Deserialize, Serialize};

/// Unique identifier for a region node.
///
/// Combines a seed (world/graph identifier) and sequence number for
/// deterministic generation and efficient lookup.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RegionId(u64);

impl RegionId {
    /// Create a region ID from raw value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Create an ID from a seed and sequence number.
    #[must_use]
    pub const fn new(seed: u32, sequence: u32) -> Self {
        Self(((seed as u64) << 32) | (sequence as u64))
    }

    /// Get the raw value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Extract the seed component.
    #[must_use]
    pub const fn seed(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Extract the sequence component.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional extraction of lower 32 bits"
    )]
    pub const fn sequence(self) -> u32 {
        self.0 as u32
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "region:{:08x}:{:08x}", self.seed(), self.sequence())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_extract() {
        let id = RegionId::new(0xDEAD, 0xBEEF);
        assert_eq!(id.seed(), 0xDEAD);
        assert_eq!(id.sequence(), 0xBEEF);
    }

    #[test]
    fn from_raw_roundtrip() {
        let raw = 0x1234_5678_9ABC_DEF0;
        let id = RegionId::from_raw(raw);
        assert_eq!(id.raw(), raw);
    }

    #[test]
    fn display() {
        let id = RegionId::new(0x0000_1234, 0x0000_5678);
        let s = format!("{id}");
        assert_eq!(s, "region:00001234:00005678");
    }

    #[test]
    fn ordering() {
        let id1 = RegionId::new(1, 0);
        let id2 = RegionId::new(1, 1);
        let id3 = RegionId::new(2, 0);

        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    #[test]
    fn serde_roundtrip() {
        let id = RegionId::new(42, 100);
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: RegionId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }
}

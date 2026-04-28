//! Unique identifier for megastructures.

use serde::{Deserialize, Serialize};

/// Unique identifier for a megastructure.
///
/// Uses a 64-bit value combining a type tag and sequence number for
/// deterministic generation and efficient lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MegastructureId(u64);

impl MegastructureId {
    /// Create a new megastructure ID from raw value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Create an ID from a seed and sequence number.
    ///
    /// Combines seed (world/dimension identifier) with sequence for uniqueness.
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

impl std::fmt::Display for MegastructureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mega:{:08x}:{:08x}", self.seed(), self.sequence())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_extract() {
        let id = MegastructureId::new(0xDEAD, 0xBEEF);
        assert_eq!(id.seed(), 0xDEAD);
        assert_eq!(id.sequence(), 0xBEEF);
    }

    #[test]
    fn test_from_raw_roundtrip() {
        let raw = 0x1234_5678_9ABC_DEF0;
        let id = MegastructureId::from_raw(raw);
        assert_eq!(id.raw(), raw);
    }

    #[test]
    fn test_display() {
        let id = MegastructureId::new(0x0000_1234, 0x0000_5678);
        let s = format!("{id}");
        assert_eq!(s, "mega:00001234:00005678");
    }

    #[test]
    fn test_ordering() {
        let id1 = MegastructureId::new(1, 0);
        let id2 = MegastructureId::new(1, 1);
        let id3 = MegastructureId::new(2, 0);

        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    #[test]
    fn test_serde_roundtrip() {
        let id = MegastructureId::new(42, 100);
        let serialized = bincode::serialize(&id).unwrap();
        let deserialized: MegastructureId = bincode::deserialize(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }
}

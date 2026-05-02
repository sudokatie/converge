//! Typed identifiers for portal system.

use serde::{Deserialize, Serialize};

/// Unique identifier for a portal.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct PortalId(u64);

impl PortalId {
    /// Create a new portal ID from raw value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Create a portal ID from seed and sequence.
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

impl std::fmt::Display for PortalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "portal:{:08x}:{:08x}", self.seed(), self.sequence())
    }
}

impl From<u64> for PortalId {
    fn from(value: u64) -> Self {
        Self::from_raw(value)
    }
}

/// Unique identifier for a zone (connected region of space).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct ZoneId(u64);

impl ZoneId {
    /// Create a new zone ID from raw value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Create a zone ID from seed and sequence.
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

impl std::fmt::Display for ZoneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "zone:{:08x}:{:08x}", self.seed(), self.sequence())
    }
}

impl From<u64> for ZoneId {
    fn from(value: u64) -> Self {
        Self::from_raw(value)
    }
}

/// Unique identifier for a traversal path through portal graph.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct TraversalId(u64);

impl TraversalId {
    /// Create a new traversal ID.
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

impl std::fmt::Display for TraversalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trav:{:016x}", self.0)
    }
}

impl From<u64> for TraversalId {
    fn from(value: u64) -> Self {
        Self::from_raw(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_id_new_and_extract() {
        let id = PortalId::new(0xDEAD, 0xBEEF);
        assert_eq!(id.seed(), 0xDEAD);
        assert_eq!(id.sequence(), 0xBEEF);
    }

    #[test]
    fn portal_id_from_raw_roundtrip() {
        let raw = 0x1234_5678_9ABC_DEF0;
        let id = PortalId::from_raw(raw);
        assert_eq!(id.raw(), raw);
    }

    #[test]
    fn portal_id_display() {
        let id = PortalId::new(0x0000_1234, 0x0000_5678);
        assert_eq!(format!("{id}"), "portal:00001234:00005678");
    }

    #[test]
    fn zone_id_new_and_extract() {
        let id = ZoneId::new(0xCAFE, 0xBABE);
        assert_eq!(id.seed(), 0xCAFE);
        assert_eq!(id.sequence(), 0xBABE);
    }

    #[test]
    fn zone_id_display() {
        let id = ZoneId::new(0x0000_ABCD, 0x0000_EF01);
        assert_eq!(format!("{id}"), "zone:0000abcd:0000ef01");
    }

    #[test]
    fn traversal_id_display() {
        let id = TraversalId::from_raw(0x1234_5678_9ABC_DEF0);
        assert_eq!(format!("{id}"), "trav:123456789abcdef0");
    }

    #[test]
    fn id_ordering() {
        let id1 = PortalId::new(1, 0);
        let id2 = PortalId::new(1, 1);
        let id3 = PortalId::new(2, 0);

        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    #[test]
    fn serde_roundtrip_portal_id() {
        let id = PortalId::new(42, 100);
        let serialized = bincode::serialize(&id).unwrap();
        let deserialized: PortalId = bincode::deserialize(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn serde_roundtrip_zone_id() {
        let id = ZoneId::new(99, 200);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: ZoneId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }
}

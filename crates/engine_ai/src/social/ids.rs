//! Typed identifiers for the social simulation system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a social agent (individual creature/settler).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SocialAgentId(pub u64);

impl SocialAgentId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SocialAgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "social_agent:{}", self.0)
    }
}

/// Unique identifier for a social faction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SocialFactionId(pub String);

impl SocialFactionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for SocialFactionId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for SocialFactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "social_faction:{}", self.0)
    }
}

/// Unique identifier for a social group (pack, squad, party).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SocialGroupId(pub u64);

impl SocialGroupId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SocialGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "social_group:{}", self.0)
    }
}

/// Unique identifier for a betrayal incident.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BetrayalId(pub u64);

impl BetrayalId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for BetrayalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "betrayal:{}", self.0)
    }
}

/// Unique identifier for a diplomatic relation/treaty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DiplomacyId(pub u64);

impl DiplomacyId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DiplomacyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "diplomacy:{}", self.0)
    }
}

/// Unique identifier for a panic event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PanicId(pub u64);

impl PanicId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for PanicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panic:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_social_agent_id() {
        let id = SocialAgentId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "social_agent:42");
    }

    #[test]
    fn test_social_faction_id() {
        let id = SocialFactionId::new("empire");
        assert_eq!(id.as_str(), "empire");
        assert_eq!(format!("{id}"), "social_faction:empire");
    }

    #[test]
    fn test_social_group_id() {
        let id = SocialGroupId::new(7);
        assert_eq!(id.raw(), 7);
        assert_eq!(format!("{id}"), "social_group:7");
    }

    #[test]
    fn test_betrayal_id() {
        let id = BetrayalId::new(99);
        assert_eq!(id.raw(), 99);
        assert_eq!(format!("{id}"), "betrayal:99");
    }

    #[test]
    fn test_diplomacy_id() {
        let id = DiplomacyId::new(15);
        assert_eq!(id.raw(), 15);
        assert_eq!(format!("{id}"), "diplomacy:15");
    }

    #[test]
    fn test_panic_id() {
        let id = PanicId::new(33);
        assert_eq!(id.raw(), 33);
        assert_eq!(format!("{id}"), "panic:33");
    }

    #[test]
    fn test_id_serde() {
        let agent = SocialAgentId::new(1);
        let json = serde_json::to_string(&agent).unwrap();
        let restored: SocialAgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, agent);

        let faction = SocialFactionId::new("rebels");
        let json = serde_json::to_string(&faction).unwrap();
        let restored: SocialFactionId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "rebels");
    }

    #[test]
    fn test_id_ordering() {
        let id1 = SocialAgentId::new(1);
        let id2 = SocialAgentId::new(2);
        assert!(id1 < id2);
    }
}

//! Typed identifiers for the emergency response system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for an emergency incident.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EmergencyId(pub u64);

impl EmergencyId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for EmergencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "emergency:{}", self.0)
    }
}

/// Unique identifier for an emergency responder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResponderId(pub u64);

impl ResponderId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ResponderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "responder:{}", self.0)
    }
}

/// Unique identifier for a response plan instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResponsePlanId(pub u64);

impl ResponsePlanId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ResponsePlanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "response-plan:{}", self.0)
    }
}

/// Unique identifier for a response action instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResponseActionId(pub u64);

impl ResponseActionId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ResponseActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "action:{}", self.0)
    }
}

/// Unique identifier for a safe zone / shelter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShelterZoneId(pub u64);

impl ShelterZoneId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ShelterZoneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shelter:{}", self.0)
    }
}

/// Unique identifier for a containment zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContainmentZoneId(pub u64);

impl ContainmentZoneId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ContainmentZoneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "containment:{}", self.0)
    }
}

/// String-based identifier for emergency type definitions.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EmergencyTypeId(pub String);

impl EmergencyTypeId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for EmergencyTypeId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for EmergencyTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "etype:{}", self.0)
    }
}

/// String-based identifier for response protocol definitions.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResponseProtocolId(pub String);

impl ResponseProtocolId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ResponseProtocolId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for ResponseProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol:{}", self.0)
    }
}

/// String-based identifier for responder capability/role.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResponderRoleId(pub String);

impl ResponderRoleId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ResponderRoleId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for ResponderRoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "role:{}", self.0)
    }
}

/// Re-export `RegionId` from settler module for consistency.
pub use crate::settler::RegionId;

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_emergency_id() {
        let id = EmergencyId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "emergency:42");
    }

    #[test]
    fn test_responder_id() {
        let id = ResponderId::new(123);
        assert_eq!(id.raw(), 123);
        assert_eq!(format!("{id}"), "responder:123");
    }

    #[test]
    fn test_response_plan_id() {
        let id = ResponsePlanId::new(7);
        assert_eq!(id.raw(), 7);
        assert_eq!(format!("{id}"), "response-plan:7");
    }

    #[test]
    fn test_response_action_id() {
        let id = ResponseActionId::new(99);
        assert_eq!(id.raw(), 99);
        assert_eq!(format!("{id}"), "action:99");
    }

    #[test]
    fn test_shelter_zone_id() {
        let id = ShelterZoneId::new(5);
        assert_eq!(id.raw(), 5);
        assert_eq!(format!("{id}"), "shelter:5");
    }

    #[test]
    fn test_containment_zone_id() {
        let id = ContainmentZoneId::new(3);
        assert_eq!(id.raw(), 3);
        assert_eq!(format!("{id}"), "containment:3");
    }

    #[test]
    fn test_emergency_type_id() {
        let id = EmergencyTypeId::new("fire");
        assert_eq!(id.as_str(), "fire");
        assert_eq!(format!("{id}"), "etype:fire");
    }

    #[test]
    fn test_response_protocol_id() {
        let id = ResponseProtocolId::new("standard_evacuation");
        assert_eq!(id.as_str(), "standard_evacuation");
        assert_eq!(format!("{id}"), "protocol:standard_evacuation");
    }

    #[test]
    fn test_responder_role_id() {
        let id = ResponderRoleId::new("firefighter");
        assert_eq!(id.as_str(), "firefighter");
        assert_eq!(format!("{id}"), "role:firefighter");
    }

    #[test]
    fn test_id_serde() {
        let emergency = EmergencyId::new(1);
        let json = serde_json::to_string(&emergency).unwrap();
        let restored: EmergencyId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, emergency);

        let protocol = ResponseProtocolId::new("containment");
        let json = serde_json::to_string(&protocol).unwrap();
        let restored: ResponseProtocolId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "containment");

        let role = ResponderRoleId::new("medic");
        let json = serde_json::to_string(&role).unwrap();
        let restored: ResponderRoleId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "medic");
    }

    #[test]
    fn test_id_ordering() {
        let id1 = EmergencyId::new(1);
        let id2 = EmergencyId::new(2);
        let id3 = EmergencyId::new(1);

        assert!(id1 < id2);
        assert_eq!(id1, id3);

        let type1 = EmergencyTypeId::new("alpha");
        let type2 = EmergencyTypeId::new("beta");
        assert!(type1 < type2);
    }
}

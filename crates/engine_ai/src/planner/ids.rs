//! Deterministic typed identifiers for the planner.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActorId(u64);

impl ActorId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor:{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlanId(u64);

impl PlanId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for PlanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "plan:{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionInstanceId(u64);

impl ActionInstanceId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ActionInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "action_inst:{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocationId(u64);

impl LocationId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for LocationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "location:{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionDefId(String);

impl ActionDefId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ActionDefId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ActionDefId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ActionDefId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntentId(String);

impl IntentId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn acquire_resource() -> Self {
        Self::new("acquire_resource")
    }

    #[must_use]
    pub fn flee_threat() -> Self {
        Self::new("flee_threat")
    }

    #[must_use]
    pub fn explore() -> Self {
        Self::new("explore")
    }

    #[must_use]
    pub fn defend() -> Self {
        Self::new("defend")
    }

    #[must_use]
    pub fn rest() -> Self {
        Self::new("rest")
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for IntentId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for IntentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FactId(String);

impl FactId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for FactId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for FactId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FactionScopeId(String);

impl FactionScopeId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FactionScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for FactionScopeId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for FactionScopeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResourceTypeId(String);

impl ResourceTypeId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ResourceTypeId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ResourceTypeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_id() {
        let id = ActorId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "actor:42");
    }

    #[test]
    fn test_plan_id() {
        let id = PlanId::new(1);
        assert_eq!(id.raw(), 1);
        assert_eq!(format!("{id}"), "plan:1");
    }

    #[test]
    fn test_action_instance_id() {
        let id = ActionInstanceId::new(99);
        assert_eq!(id.raw(), 99);
    }

    #[test]
    fn test_location_id() {
        let id = LocationId::new(5);
        assert_eq!(id.raw(), 5);
    }

    #[test]
    fn test_action_def_id() {
        let id = ActionDefId::new("gather");
        assert_eq!(id.as_str(), "gather");

        let id2: ActionDefId = "move".into();
        assert_eq!(id2.as_str(), "move");
    }

    #[test]
    fn test_intent_id() {
        let id = IntentId::new("custom");
        assert_eq!(id.as_str(), "custom");

        assert_eq!(IntentId::acquire_resource().as_str(), "acquire_resource");
        assert_eq!(IntentId::flee_threat().as_str(), "flee_threat");

        let id2: IntentId = "test".into();
        assert_eq!(id2.as_str(), "test");
    }

    #[test]
    fn test_fact_id() {
        let id = FactId::new("has_weapon");
        assert_eq!(id.as_str(), "has_weapon");

        let id2: FactId = "is_hungry".into();
        assert_eq!(id2.as_str(), "is_hungry");
    }

    #[test]
    fn test_faction_scope_id() {
        let id = FactionScopeId::new("guild");
        assert_eq!(id.as_str(), "guild");
    }

    #[test]
    fn test_resource_type_id() {
        let id = ResourceTypeId::new("gold");
        assert_eq!(id.as_str(), "gold");
    }

    #[test]
    fn test_id_ordering() {
        let a = ActorId::new(1);
        let b = ActorId::new(2);
        assert!(a < b);

        let x = ActionDefId::new("aaa");
        let y = ActionDefId::new("bbb");
        assert!(x < y);
    }

    #[test]
    fn test_id_serde() {
        let actor = ActorId::new(42);
        let json = serde_json::to_string(&actor).unwrap();
        let restored: ActorId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, actor);

        let intent = IntentId::new("test");
        let json = serde_json::to_string(&intent).unwrap();
        let restored: IntentId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, intent);
    }

    #[test]
    fn test_id_bincode() {
        let actor = ActorId::new(42);
        let bytes = bincode::serialize(&actor).unwrap();
        let restored: ActorId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, actor);

        let plan = PlanId::new(100);
        let bytes = bincode::serialize(&plan).unwrap();
        let restored: PlanId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, plan);

        let action = ActionInstanceId::new(77);
        let bytes = bincode::serialize(&action).unwrap();
        let restored: ActionInstanceId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, action);

        let location = LocationId::new(5);
        let bytes = bincode::serialize(&location).unwrap();
        let restored: LocationId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, location);

        let action_def = ActionDefId::new("gather");
        let bytes = bincode::serialize(&action_def).unwrap();
        let restored: ActionDefId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, action_def);

        let intent = IntentId::flee_threat();
        let bytes = bincode::serialize(&intent).unwrap();
        let restored: IntentId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, intent);

        let fact = FactId::new("has_weapon");
        let bytes = bincode::serialize(&fact).unwrap();
        let restored: FactId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, fact);

        let faction = FactionScopeId::new("guild");
        let bytes = bincode::serialize(&faction).unwrap();
        let restored: FactionScopeId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, faction);

        let resource = ResourceTypeId::new("gold");
        let bytes = bincode::serialize(&resource).unwrap();
        let restored: ResourceTypeId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored, resource);
    }
}

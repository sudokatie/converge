//! Typed identifiers for the settler task AI system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a settler/worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SettlerId(pub u64);

impl SettlerId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SettlerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "settler:{}", self.0)
    }
}

/// Unique identifier for a task instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskId(pub u64);

impl TaskId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task:{}", self.0)
    }
}

/// Unique identifier for a task definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskDefId(pub String);

impl TaskDefId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for TaskDefId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for TaskDefId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "taskdef:{}", self.0)
    }
}

/// Unique identifier for a capability/skill.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for CapabilityId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cap:{}", self.0)
    }
}

/// Unique identifier for a region/zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionId(pub u64);

impl RegionId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "region:{}", self.0)
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_settler_id() {
        let id = SettlerId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "settler:42");
    }

    #[test]
    fn test_task_id() {
        let id = TaskId::new(123);
        assert_eq!(id.raw(), 123);
        assert_eq!(format!("{id}"), "task:123");
    }

    #[test]
    fn test_task_def_id() {
        let id = TaskDefId::new("mining");
        assert_eq!(id.as_str(), "mining");
        assert_eq!(format!("{id}"), "taskdef:mining");
    }

    #[test]
    fn test_capability_id() {
        let id = CapabilityId::new("carpentry");
        assert_eq!(id.as_str(), "carpentry");
        assert_eq!(format!("{id}"), "cap:carpentry");
    }

    #[test]
    fn test_region_id() {
        let id = RegionId::new(7);
        assert_eq!(id.raw(), 7);
        assert_eq!(format!("{id}"), "region:7");
    }

    #[test]
    fn test_id_serde() {
        let settler = SettlerId::new(1);
        let json = serde_json::to_string(&settler).unwrap();
        let restored: SettlerId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, settler);

        let task_def = TaskDefId::new("hauling");
        let json = serde_json::to_string(&task_def).unwrap();
        let restored: TaskDefId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "hauling");
    }
}

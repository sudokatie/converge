//! Typed identifiers for the colony simulation system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a colony.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColonyId(pub u64);

impl ColonyId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ColonyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "colony:{}", self.0)
    }
}

/// Unique identifier for a job definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobDefId(pub String);

impl JobDefId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for JobDefId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for JobDefId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "jobdef:{}", self.0)
    }
}

/// Unique identifier for a job instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

impl JobId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job:{}", self.0)
    }
}

/// Unique identifier for a resource/item type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub String);

impl ResourceId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ResourceId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resource:{}", self.0)
    }
}

/// Unique identifier for a storage/logistics node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StorageNodeId(pub u64);

impl StorageNodeId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for StorageNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "storage:{}", self.0)
    }
}

/// Unique identifier for a logistics route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RouteId(pub u64);

impl RouteId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RouteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "route:{}", self.0)
    }
}

/// Unique identifier for a shelter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShelterId(pub u64);

impl ShelterId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ShelterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shelter:{}", self.0)
    }
}

/// Unique identifier for a failure event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FailureId(pub u64);

impl FailureId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for FailureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failure:{}", self.0)
    }
}

/// Unique identifier for a worker in the colony.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub u64);

impl WorkerId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "worker:{}", self.0)
    }
}

/// Unique identifier for a skill/capability.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl SkillId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for SkillId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "skill:{}", self.0)
    }
}

/// Unique identifier for a transfer operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransferId(pub u64);

impl TransferId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "transfer:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colony_id() {
        let id = ColonyId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "colony:42");
    }

    #[test]
    fn test_job_def_id() {
        let id = JobDefId::new("mining");
        assert_eq!(id.as_str(), "mining");
        assert_eq!(format!("{id}"), "jobdef:mining");
    }

    #[test]
    fn test_job_id() {
        let id = JobId::new(123);
        assert_eq!(id.raw(), 123);
        assert_eq!(format!("{id}"), "job:123");
    }

    #[test]
    fn test_resource_id() {
        let id = ResourceId::new("iron_ore");
        assert_eq!(id.as_str(), "iron_ore");
        assert_eq!(format!("{id}"), "resource:iron_ore");
    }

    #[test]
    fn test_storage_node_id() {
        let id = StorageNodeId::new(7);
        assert_eq!(id.raw(), 7);
        assert_eq!(format!("{id}"), "storage:7");
    }

    #[test]
    fn test_route_id() {
        let id = RouteId::new(15);
        assert_eq!(id.raw(), 15);
        assert_eq!(format!("{id}"), "route:15");
    }

    #[test]
    fn test_shelter_id() {
        let id = ShelterId::new(3);
        assert_eq!(id.raw(), 3);
        assert_eq!(format!("{id}"), "shelter:3");
    }

    #[test]
    fn test_failure_id() {
        let id = FailureId::new(99);
        assert_eq!(id.raw(), 99);
        assert_eq!(format!("{id}"), "failure:99");
    }

    #[test]
    fn test_worker_id() {
        let id = WorkerId::new(55);
        assert_eq!(id.raw(), 55);
        assert_eq!(format!("{id}"), "worker:55");
    }

    #[test]
    fn test_skill_id() {
        let id = SkillId::new("carpentry");
        assert_eq!(id.as_str(), "carpentry");
        assert_eq!(format!("{id}"), "skill:carpentry");
    }

    #[test]
    fn test_transfer_id() {
        let id = TransferId::new(77);
        assert_eq!(id.raw(), 77);
        assert_eq!(format!("{id}"), "transfer:77");
    }

    #[test]
    fn test_id_ordering() {
        let id1 = JobId::new(1);
        let id2 = JobId::new(2);
        let id3 = JobId::new(1);

        assert!(id1 < id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_string_id_ordering() {
        let id1 = ResourceId::new("apple");
        let id2 = ResourceId::new("banana");
        let id3 = ResourceId::new("apple");

        assert!(id1 < id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_id_serde_json() {
        let colony = ColonyId::new(1);
        let json = serde_json::to_string(&colony).unwrap();
        let restored: ColonyId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, colony);

        let job_def = JobDefId::new("hauling");
        let json = serde_json::to_string(&job_def).unwrap();
        let restored: JobDefId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "hauling");

        let resource = ResourceId::new("wood");
        let json = serde_json::to_string(&resource).unwrap();
        let restored: ResourceId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "wood");
    }

    #[test]
    fn test_id_bincode() {
        let colony = ColonyId::new(12345);
        let bytes = bincode::serialize(&colony).unwrap();
        let restored: ColonyId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 12345);

        let job = JobId::new(67890);
        let bytes = bincode::serialize(&job).unwrap();
        let restored: JobId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 67890);

        let shelter = ShelterId::new(111);
        let bytes = bincode::serialize(&shelter).unwrap();
        let restored: ShelterId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 111);

        let failure = FailureId::new(222);
        let bytes = bincode::serialize(&failure).unwrap();
        let restored: FailureId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 222);

        let resource = ResourceId::new("test_resource");
        let bytes = bincode::serialize(&resource).unwrap();
        let restored: ResourceId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.as_str(), "test_resource");

        let skill = SkillId::new("test_skill");
        let bytes = bincode::serialize(&skill).unwrap();
        let restored: SkillId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.as_str(), "test_skill");
    }

    #[test]
    fn test_from_impl() {
        let job_def: JobDefId = "mining".into();
        assert_eq!(job_def.as_str(), "mining");

        let resource: ResourceId = String::from("stone").into();
        assert_eq!(resource.as_str(), "stone");

        let skill: SkillId = "building".into();
        assert_eq!(skill.as_str(), "building");
    }
}

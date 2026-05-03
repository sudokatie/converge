//! Typed identifiers for the director system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a director instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DirectorId(pub u64);

impl DirectorId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DirectorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "director:{}", self.0)
    }
}

/// Unique identifier for a disaster event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DisasterId(pub u64);

impl DisasterId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DisasterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "disaster:{}", self.0)
    }
}

/// Unique identifier for a director recommendation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RecommendationId(pub u64);

impl RecommendationId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RecommendationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recommendation:{}", self.0)
    }
}

/// Unique identifier for a pacing profile.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PacingProfileId(pub String);

impl PacingProfileId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for PacingProfileId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl Default for PacingProfileId {
    fn default() -> Self {
        Self::new("default")
    }
}

impl fmt::Display for PacingProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pacing:{}", self.0)
    }
}

/// Unique identifier for a competence signal type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CompetenceSignalId(pub String);

impl CompetenceSignalId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for CompetenceSignalId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for CompetenceSignalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "competence:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_id() {
        let id = DirectorId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "director:42");
    }

    #[test]
    fn test_disaster_id() {
        let id = DisasterId::new(99);
        assert_eq!(id.raw(), 99);
        assert_eq!(format!("{id}"), "disaster:99");
    }

    #[test]
    fn test_recommendation_id() {
        let id = RecommendationId::new(7);
        assert_eq!(id.raw(), 7);
        assert_eq!(format!("{id}"), "recommendation:7");
    }

    #[test]
    fn test_pacing_profile_id() {
        let id = PacingProfileId::new("calm");
        assert_eq!(id.as_str(), "calm");
        assert_eq!(format!("{id}"), "pacing:calm");
    }

    #[test]
    fn test_competence_signal_id() {
        let id = CompetenceSignalId::new("task_completion");
        assert_eq!(id.as_str(), "task_completion");
        assert_eq!(format!("{id}"), "competence:task_completion");
    }

    #[test]
    fn test_id_ordering() {
        let id1 = DisasterId::new(1);
        let id2 = DisasterId::new(2);
        let id3 = DisasterId::new(1);

        assert!(id1 < id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_string_id_ordering() {
        let id1 = PacingProfileId::new("alpha");
        let id2 = PacingProfileId::new("beta");
        let id3 = PacingProfileId::new("alpha");

        assert!(id1 < id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_id_serde_json() {
        let director = DirectorId::new(123);
        let json = serde_json::to_string(&director).unwrap();
        let restored: DirectorId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, director);

        let disaster = DisasterId::new(456);
        let json = serde_json::to_string(&disaster).unwrap();
        let restored: DisasterId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.raw(), 456);

        let pacing = PacingProfileId::new("intense");
        let json = serde_json::to_string(&pacing).unwrap();
        let restored: PacingProfileId = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_str(), "intense");
    }

    #[test]
    fn test_id_bincode() {
        let director = DirectorId::new(12345);
        let bytes = bincode::serialize(&director).unwrap();
        let restored: DirectorId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 12345);

        let disaster = DisasterId::new(67890);
        let bytes = bincode::serialize(&disaster).unwrap();
        let restored: DisasterId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 67890);

        let recommendation = RecommendationId::new(111);
        let bytes = bincode::serialize(&recommendation).unwrap();
        let restored: RecommendationId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.raw(), 111);

        let pacing = PacingProfileId::new("test_profile");
        let bytes = bincode::serialize(&pacing).unwrap();
        let restored: PacingProfileId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.as_str(), "test_profile");

        let competence = CompetenceSignalId::new("test_signal");
        let bytes = bincode::serialize(&competence).unwrap();
        let restored: CompetenceSignalId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.as_str(), "test_signal");
    }

    #[test]
    fn test_from_impl() {
        let pacing: PacingProfileId = "relaxed".into();
        assert_eq!(pacing.as_str(), "relaxed");

        let competence: CompetenceSignalId = String::from("efficiency").into();
        assert_eq!(competence.as_str(), "efficiency");
    }
}

//! Stable fingerprints and checksums for mission state.

use serde::{Deserialize, Serialize};

use super::{MissionId, MissionState, ObjectiveId, ObjectiveState};

/// Fingerprint of a mission definition for change detection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionFingerprint(pub u64);

impl MissionFingerprint {
    /// Create a new fingerprint from raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Compute fingerprint from mission definition properties.
    #[must_use]
    pub fn from_definition(
        id: &str,
        objective_count: usize,
        base_duration: Option<u64>,
        enabled: bool,
        repeatable: bool,
    ) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(id.as_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let count = objective_count as u32;
        hasher.update(&count.to_le_bytes());
        hasher.update(&base_duration.unwrap_or(0).to_le_bytes());
        hasher.update(&[u8::from(enabled)]);
        hasher.update(&[u8::from(repeatable)]);
        Self(u64::from(hasher.finalize()))
    }

    /// Get raw fingerprint value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Combine with another fingerprint.
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self {
        Self(self.0.wrapping_mul(31).wrapping_add(other.0))
    }
}

/// Checksum of mission tracker state for synchronization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionChecksum {
    /// Checksum of active missions.
    pub active_missions: u32,

    /// Checksum of objective progress.
    pub objective_progress: u32,

    /// Checksum of completed missions.
    pub completed_missions: u32,

    /// Checksum of event history.
    pub event_history: u32,

    /// Tick when checksum was computed.
    pub tick: u64,
}

impl MissionChecksum {
    /// Create a new checksum.
    #[must_use]
    pub const fn new(tick: u64) -> Self {
        Self {
            active_missions: 0,
            objective_progress: 0,
            completed_missions: 0,
            event_history: 0,
            tick,
        }
    }

    /// Set active missions checksum.
    #[must_use]
    pub const fn with_active_missions(mut self, checksum: u32) -> Self {
        self.active_missions = checksum;
        self
    }

    /// Set objective progress checksum.
    #[must_use]
    pub const fn with_objective_progress(mut self, checksum: u32) -> Self {
        self.objective_progress = checksum;
        self
    }

    /// Set completed missions checksum.
    #[must_use]
    pub const fn with_completed_missions(mut self, checksum: u32) -> Self {
        self.completed_missions = checksum;
        self
    }

    /// Set event history checksum.
    #[must_use]
    pub const fn with_event_history(mut self, checksum: u32) -> Self {
        self.event_history = checksum;
        self
    }

    /// Compute combined checksum.
    #[must_use]
    pub fn combined(&self) -> u64 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.active_missions.to_le_bytes());
        hasher.update(&self.objective_progress.to_le_bytes());
        hasher.update(&self.completed_missions.to_le_bytes());
        hasher.update(&self.event_history.to_le_bytes());
        u64::from(hasher.finalize())
    }

    /// Check if this checksum matches another.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.active_missions == other.active_missions
            && self.objective_progress == other.objective_progress
            && self.completed_missions == other.completed_missions
            && self.event_history == other.event_history
    }

    /// Get mismatched components.
    #[must_use]
    pub fn mismatches(&self, other: &Self) -> Vec<&'static str> {
        let mut result = Vec::new();
        if self.active_missions != other.active_missions {
            result.push("active_missions");
        }
        if self.objective_progress != other.objective_progress {
            result.push("objective_progress");
        }
        if self.completed_missions != other.completed_missions {
            result.push("completed_missions");
        }
        if self.event_history != other.event_history {
            result.push("event_history");
        }
        result
    }
}

/// Builder for computing mission state checksums.
#[derive(Clone, Debug, Default)]
pub struct ChecksumBuilder {
    active: crc32fast::Hasher,
    progress: crc32fast::Hasher,
    completed: crc32fast::Hasher,
}

impl ChecksumBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an active mission to the checksum.
    pub fn add_active_mission(
        &mut self,
        mission_id: MissionId,
        definition_id: &str,
        state: MissionState,
        started_at: Option<u64>,
    ) {
        self.active.update(&mission_id.raw().to_le_bytes());
        self.active.update(definition_id.as_bytes());
        self.active.update(&[state as u8]);
        self.active.update(&started_at.unwrap_or(0).to_le_bytes());
    }

    /// Add objective progress to the checksum.
    pub fn add_objective_progress(
        &mut self,
        mission_id: MissionId,
        objective_id: ObjectiveId,
        state: ObjectiveState,
        current_count: u32,
        elapsed_ticks: u64,
    ) {
        self.progress.update(&mission_id.raw().to_le_bytes());
        self.progress.update(&objective_id.raw().to_le_bytes());
        self.progress.update(&[state as u8]);
        self.progress.update(&current_count.to_le_bytes());
        self.progress.update(&elapsed_ticks.to_le_bytes());
    }

    /// Add a completed mission to the checksum.
    pub fn add_completed_mission(
        &mut self,
        mission_id: MissionId,
        definition_id: &str,
        state: MissionState,
        ended_at: u64,
    ) {
        self.completed.update(&mission_id.raw().to_le_bytes());
        self.completed.update(definition_id.as_bytes());
        self.completed.update(&[state as u8]);
        self.completed.update(&ended_at.to_le_bytes());
    }

    /// Build the checksum.
    #[must_use]
    pub fn build(self, tick: u64, event_checksum: u32) -> MissionChecksum {
        MissionChecksum {
            active_missions: self.active.finalize(),
            objective_progress: self.progress.finalize(),
            completed_missions: self.completed.finalize(),
            event_history: event_checksum,
            tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_deterministic() {
        let f1 = MissionFingerprint::from_definition("test", 3, Some(1000), true, false);
        let f2 = MissionFingerprint::from_definition("test", 3, Some(1000), true, false);
        assert_eq!(f1, f2);
    }

    #[test]
    fn fingerprint_differs_on_change() {
        let f1 = MissionFingerprint::from_definition("test", 3, Some(1000), true, false);
        let f2 = MissionFingerprint::from_definition("test", 4, Some(1000), true, false);
        assert_ne!(f1, f2);

        let f3 = MissionFingerprint::from_definition("test", 3, Some(2000), true, false);
        assert_ne!(f1, f3);
    }

    #[test]
    fn fingerprint_combine() {
        let f1 = MissionFingerprint::new(100);
        let f2 = MissionFingerprint::new(200);
        let combined = f1.combine(&f2);
        assert_ne!(combined, f1);
        assert_ne!(combined, f2);
    }

    #[test]
    fn checksum_matches() {
        let c1 = MissionChecksum::new(100)
            .with_active_missions(1)
            .with_objective_progress(2)
            .with_completed_missions(3)
            .with_event_history(4);

        let c2 = MissionChecksum::new(100)
            .with_active_missions(1)
            .with_objective_progress(2)
            .with_completed_missions(3)
            .with_event_history(4);

        assert!(c1.matches(&c2));
    }

    #[test]
    fn checksum_mismatches() {
        let c1 = MissionChecksum::new(100)
            .with_active_missions(1)
            .with_objective_progress(2);

        let c2 = MissionChecksum::new(100)
            .with_active_missions(1)
            .with_objective_progress(3);

        assert!(!c1.matches(&c2));
        assert_eq!(c1.mismatches(&c2), vec!["objective_progress"]);
    }

    #[test]
    fn checksum_combined_deterministic() {
        let c1 = MissionChecksum::new(100).with_active_missions(1);
        let c2 = MissionChecksum::new(100).with_active_missions(1);
        assert_eq!(c1.combined(), c2.combined());
    }

    #[test]
    fn builder_deterministic() {
        let mut b1 = ChecksumBuilder::new();
        b1.add_active_mission(MissionId::new(1), "test", MissionState::Active, Some(100));
        b1.add_objective_progress(
            MissionId::new(1),
            ObjectiveId::new(0),
            ObjectiveState::InProgress,
            5,
            50,
        );
        let c1 = b1.build(100, 0);

        let mut b2 = ChecksumBuilder::new();
        b2.add_active_mission(MissionId::new(1), "test", MissionState::Active, Some(100));
        b2.add_objective_progress(
            MissionId::new(1),
            ObjectiveId::new(0),
            ObjectiveState::InProgress,
            5,
            50,
        );
        let c2 = b2.build(100, 0);

        assert!(c1.matches(&c2));
    }

    #[test]
    fn serde_round_trip() {
        let fp = MissionFingerprint::new(12345);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: MissionFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, fp);

        let cs = MissionChecksum::new(100)
            .with_active_missions(1)
            .with_objective_progress(2);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: MissionChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, cs);
    }
}

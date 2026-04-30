//! Mission event records for deterministic tracking.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::{MissionId, MissionState, ObjectiveId, ObjectiveState};

/// Kind of mission event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum MissionEventKind {
    /// Mission was accepted.
    Accepted = 0,
    /// Mission was started.
    Started = 1,
    /// Objective progress was recorded.
    Progress = 2,
    /// Objective state changed.
    ObjectiveStateChanged = 3,
    /// Mission state changed.
    MissionStateChanged = 4,
    /// Deadline was extended.
    DeadlineExtended = 5,
    /// Mission was abandoned.
    Abandoned = 6,
    /// Mission was completed.
    Completed = 7,
    /// Mission failed.
    Failed = 8,
    /// Mission expired.
    Expired = 9,
    /// Custom event.
    Custom = 10,
}

impl MissionEventKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Started => "started",
            Self::Progress => "progress",
            Self::ObjectiveStateChanged => "objective_state_changed",
            Self::MissionStateChanged => "mission_state_changed",
            Self::DeadlineExtended => "deadline_extended",
            Self::Abandoned => "abandoned",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Custom => "custom",
        }
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Expired | Self::Abandoned
        )
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Accepted),
            1 => Some(Self::Started),
            2 => Some(Self::Progress),
            3 => Some(Self::ObjectiveStateChanged),
            4 => Some(Self::MissionStateChanged),
            5 => Some(Self::DeadlineExtended),
            6 => Some(Self::Abandoned),
            7 => Some(Self::Completed),
            8 => Some(Self::Failed),
            9 => Some(Self::Expired),
            10 => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Payload for mission events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MissionEventPayload {
    /// No additional data.
    None,
    /// Progress amount.
    Progress {
        objective_id: ObjectiveId,
        amount: u32,
        new_total: u32,
    },
    /// Objective state change.
    ObjectiveState {
        objective_id: ObjectiveId,
        old_state: ObjectiveState,
        new_state: ObjectiveState,
    },
    /// Mission state change.
    MissionState {
        old_state: MissionState,
        new_state: MissionState,
    },
    /// Deadline extension.
    DeadlineExtension {
        old_deadline: u64,
        new_deadline: u64,
        extension_count: u32,
    },
    /// Custom event data.
    Custom { key: String, value: String },
}

impl MissionEventPayload {
    /// Create progress payload.
    #[must_use]
    pub fn progress(objective_id: ObjectiveId, amount: u32, new_total: u32) -> Self {
        Self::Progress {
            objective_id,
            amount,
            new_total,
        }
    }

    /// Create objective state change payload.
    #[must_use]
    pub fn objective_state(
        objective_id: ObjectiveId,
        old_state: ObjectiveState,
        new_state: ObjectiveState,
    ) -> Self {
        Self::ObjectiveState {
            objective_id,
            old_state,
            new_state,
        }
    }

    /// Create mission state change payload.
    #[must_use]
    pub fn mission_state(old_state: MissionState, new_state: MissionState) -> Self {
        Self::MissionState {
            old_state,
            new_state,
        }
    }

    /// Create deadline extension payload.
    #[must_use]
    pub fn deadline_extension(old_deadline: u64, new_deadline: u64, extension_count: u32) -> Self {
        Self::DeadlineExtension {
            old_deadline,
            new_deadline,
            extension_count,
        }
    }

    /// Create custom payload.
    #[must_use]
    pub fn custom(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Custom {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Deterministic record of a mission event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionEvent {
    /// Mission this event belongs to.
    pub mission_id: MissionId,

    /// Event kind.
    pub kind: MissionEventKind,

    /// Tick when event occurred.
    pub tick: u64,

    /// Revision number for ordering events at same tick.
    pub revision: u64,

    /// Event payload.
    pub payload: MissionEventPayload,
}

impl MissionEvent {
    /// Create a new mission event.
    #[must_use]
    pub fn new(mission_id: MissionId, kind: MissionEventKind, tick: u64, revision: u64) -> Self {
        Self {
            mission_id,
            kind,
            tick,
            revision,
            payload: MissionEventPayload::None,
        }
    }

    /// Set payload.
    #[must_use]
    pub fn with_payload(mut self, payload: MissionEventPayload) -> Self {
        self.payload = payload;
        self
    }

    /// Create an accepted event.
    #[must_use]
    pub fn accepted(mission_id: MissionId, tick: u64, revision: u64) -> Self {
        Self::new(mission_id, MissionEventKind::Accepted, tick, revision)
    }

    /// Create a started event.
    #[must_use]
    pub fn started(mission_id: MissionId, tick: u64, revision: u64) -> Self {
        Self::new(mission_id, MissionEventKind::Started, tick, revision)
    }

    /// Create a progress event.
    #[must_use]
    pub fn progress(
        mission_id: MissionId,
        objective_id: ObjectiveId,
        amount: u32,
        new_total: u32,
        tick: u64,
        revision: u64,
    ) -> Self {
        Self::new(mission_id, MissionEventKind::Progress, tick, revision).with_payload(
            MissionEventPayload::progress(objective_id, amount, new_total),
        )
    }

    /// Create an objective state changed event.
    #[must_use]
    pub fn objective_state_changed(
        mission_id: MissionId,
        objective_id: ObjectiveId,
        old_state: ObjectiveState,
        new_state: ObjectiveState,
        tick: u64,
        revision: u64,
    ) -> Self {
        Self::new(
            mission_id,
            MissionEventKind::ObjectiveStateChanged,
            tick,
            revision,
        )
        .with_payload(MissionEventPayload::objective_state(
            objective_id,
            old_state,
            new_state,
        ))
    }

    /// Create a mission state changed event.
    #[must_use]
    pub fn mission_state_changed(
        mission_id: MissionId,
        old_state: MissionState,
        new_state: MissionState,
        tick: u64,
        revision: u64,
    ) -> Self {
        Self::new(
            mission_id,
            MissionEventKind::MissionStateChanged,
            tick,
            revision,
        )
        .with_payload(MissionEventPayload::mission_state(old_state, new_state))
    }

    /// Create a completed event.
    #[must_use]
    pub fn completed(mission_id: MissionId, tick: u64, revision: u64) -> Self {
        Self::new(mission_id, MissionEventKind::Completed, tick, revision)
    }

    /// Create a failed event.
    #[must_use]
    pub fn failed(mission_id: MissionId, tick: u64, revision: u64) -> Self {
        Self::new(mission_id, MissionEventKind::Failed, tick, revision)
    }

    /// Create an expired event.
    #[must_use]
    pub fn expired(mission_id: MissionId, tick: u64, revision: u64) -> Self {
        Self::new(mission_id, MissionEventKind::Expired, tick, revision)
    }

    /// Create an abandoned event.
    #[must_use]
    pub fn abandoned(mission_id: MissionId, tick: u64, revision: u64) -> Self {
        Self::new(mission_id, MissionEventKind::Abandoned, tick, revision)
    }

    /// Ordering key for deterministic sorting.
    fn sort_key(&self) -> (u64, u64, u8, u64) {
        (
            self.tick,
            self.revision,
            self.kind as u8,
            self.mission_id.raw(),
        )
    }
}

impl Eq for MissionEvent {}

impl Ord for MissionEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for MissionEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// History of mission events with deterministic ordering.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MissionEventHistory {
    /// Events in order.
    events: Vec<MissionEvent>,

    /// Current revision counter.
    next_revision: u64,

    /// Maximum events to retain.
    max_events: usize,
}

impl MissionEventHistory {
    /// Create a new event history.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_revision: 0,
            max_events: 10000,
        }
    }

    /// Create with custom max events.
    #[must_use]
    pub fn with_max_events(max: usize) -> Self {
        Self {
            events: Vec::new(),
            next_revision: 0,
            max_events: max,
        }
    }

    /// Record an event and return its revision.
    pub fn record(&mut self, mut event: MissionEvent) -> u64 {
        let revision = self.next_revision;
        self.next_revision += 1;
        event.revision = revision;

        self.events.push(event);
        self.events.sort();

        if self.events.len() > self.max_events {
            self.events.drain(0..self.events.len() - self.max_events);
        }

        revision
    }

    /// Get all events.
    #[must_use]
    pub fn events(&self) -> &[MissionEvent] {
        &self.events
    }

    /// Get events for a specific mission.
    pub fn events_for_mission(&self, mission_id: MissionId) -> impl Iterator<Item = &MissionEvent> {
        self.events
            .iter()
            .filter(move |e| e.mission_id == mission_id)
    }

    /// Get events since a revision.
    pub fn events_since(&self, revision: u64) -> impl Iterator<Item = &MissionEvent> {
        self.events.iter().filter(move |e| e.revision > revision)
    }

    /// Get events in a tick range.
    pub fn events_in_range(
        &self,
        start_tick: u64,
        end_tick: u64,
    ) -> impl Iterator<Item = &MissionEvent> {
        self.events
            .iter()
            .filter(move |e| e.tick >= start_tick && e.tick <= end_tick)
    }

    /// Get the latest revision.
    #[must_use]
    pub fn latest_revision(&self) -> u64 {
        self.next_revision.saturating_sub(1)
    }

    /// Get event count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
        self.next_revision = 0;
    }

    /// Compute checksum of events since a revision.
    #[must_use]
    pub fn checksum_since(&self, revision: u64) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        for event in self.events_since(revision) {
            hasher.update(&event.mission_id.raw().to_le_bytes());
            hasher.update(&event.tick.to_le_bytes());
            hasher.update(&event.revision.to_le_bytes());
            hasher.update(&[event.kind as u8]);
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_properties() {
        assert!(MissionEventKind::Completed.is_terminal());
        assert!(MissionEventKind::Failed.is_terminal());
        assert!(!MissionEventKind::Progress.is_terminal());

        assert_eq!(
            MissionEventKind::from_raw(0),
            Some(MissionEventKind::Accepted)
        );
        assert_eq!(MissionEventKind::from_raw(255), None);
    }

    #[test]
    fn event_creation() {
        let event = MissionEvent::accepted(MissionId::new(1), 100, 0);
        assert_eq!(event.mission_id, MissionId::new(1));
        assert_eq!(event.kind, MissionEventKind::Accepted);
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn event_progress() {
        let event = MissionEvent::progress(MissionId::new(1), ObjectiveId::new(0), 5, 10, 100, 0);

        assert_eq!(event.kind, MissionEventKind::Progress);
        if let MissionEventPayload::Progress {
            objective_id,
            amount,
            new_total,
        } = event.payload
        {
            assert_eq!(objective_id, ObjectiveId::new(0));
            assert_eq!(amount, 5);
            assert_eq!(new_total, 10);
        } else {
            panic!("Expected Progress payload");
        }
    }

    #[test]
    fn event_ordering() {
        let e1 = MissionEvent::new(MissionId::new(1), MissionEventKind::Accepted, 100, 0);
        let e2 = MissionEvent::new(MissionId::new(1), MissionEventKind::Started, 100, 1);
        let e3 = MissionEvent::new(MissionId::new(1), MissionEventKind::Progress, 200, 2);

        assert!(e1 < e2);
        assert!(e2 < e3);
    }

    #[test]
    fn event_ordering_same_tick() {
        let e1 = MissionEvent::new(MissionId::new(1), MissionEventKind::Accepted, 100, 0);
        let e2 = MissionEvent::new(MissionId::new(2), MissionEventKind::Accepted, 100, 1);

        assert!(e1 < e2);
    }

    #[test]
    fn history_record() {
        let mut history = MissionEventHistory::new();

        let rev = history.record(MissionEvent::accepted(MissionId::new(1), 100, 0));
        assert_eq!(rev, 0);

        let rev = history.record(MissionEvent::started(MissionId::new(1), 150, 0));
        assert_eq!(rev, 1);

        assert_eq!(history.len(), 2);
        assert_eq!(history.latest_revision(), 1);
    }

    #[test]
    fn history_ordering() {
        let mut history = MissionEventHistory::new();

        history.record(MissionEvent::new(
            MissionId::new(1),
            MissionEventKind::Progress,
            200,
            0,
        ));
        history.record(MissionEvent::new(
            MissionId::new(1),
            MissionEventKind::Accepted,
            100,
            0,
        ));

        let events: Vec<_> = history.events().iter().collect();
        assert_eq!(events[0].tick, 100);
        assert_eq!(events[1].tick, 200);
    }

    #[test]
    fn history_filter_mission() {
        let mut history = MissionEventHistory::new();

        history.record(MissionEvent::accepted(MissionId::new(1), 100, 0));
        history.record(MissionEvent::accepted(MissionId::new(2), 100, 0));
        history.record(MissionEvent::started(MissionId::new(1), 150, 0));

        let mission1_events: Vec<_> = history.events_for_mission(MissionId::new(1)).collect();
        assert_eq!(mission1_events.len(), 2);
    }

    #[test]
    fn history_filter_since() {
        let mut history = MissionEventHistory::new();

        history.record(MissionEvent::accepted(MissionId::new(1), 100, 0));
        history.record(MissionEvent::started(MissionId::new(1), 150, 0));
        history.record(MissionEvent::completed(MissionId::new(1), 200, 0));

        let since_events: Vec<_> = history.events_since(1).collect();
        assert_eq!(since_events.len(), 1);
    }

    #[test]
    fn history_checksum() {
        let mut history1 = MissionEventHistory::new();
        let mut history2 = MissionEventHistory::new();

        history1.record(MissionEvent::accepted(MissionId::new(1), 100, 0));
        history1.record(MissionEvent::started(MissionId::new(1), 150, 0));

        history2.record(MissionEvent::accepted(MissionId::new(1), 100, 0));
        history2.record(MissionEvent::started(MissionId::new(1), 150, 0));

        assert_eq!(history1.checksum_since(0), history2.checksum_since(0));
    }

    #[test]
    fn history_max_events() {
        let mut history = MissionEventHistory::with_max_events(3);

        for i in 0..5 {
            history.record(MissionEvent::accepted(MissionId::new(i), i * 100, 0));
        }

        assert_eq!(history.len(), 3);
    }

    #[test]
    fn serde_event() {
        let event = MissionEvent::progress(MissionId::new(1), ObjectiveId::new(0), 5, 10, 100, 0);

        let json = serde_json::to_string(&event).unwrap();
        let recovered: MissionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, event);
    }

    #[test]
    fn serde_history() {
        let mut history = MissionEventHistory::new();
        history.record(MissionEvent::accepted(MissionId::new(1), 100, 0));
        history.record(MissionEvent::started(MissionId::new(1), 150, 0));

        let json = serde_json::to_string(&history).unwrap();
        let recovered: MissionEventHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), history.len());
    }
}

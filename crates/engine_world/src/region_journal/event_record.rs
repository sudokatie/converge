//! Event record type for journal entries.

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use super::{EventKind, Severity};

/// Compact payload for event-specific data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EventPayload {
    /// Primary u64 value (event ID, entity ID, etc.).
    pub primary: u64,
    /// Secondary u64 value (target ID, count, etc.).
    pub secondary: u64,
    /// Tertiary i32 values (coordinates, deltas, etc.).
    pub tertiary: [i32; 4],
    /// Optional compact string data (kind name, label, etc.).
    pub label: String,
}

impl EventPayload {
    /// Create an empty payload.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            primary: 0,
            secondary: 0,
            tertiary: [0; 4],
            label: String::new(),
        }
    }

    /// Create a payload with a primary value.
    #[must_use]
    pub fn with_primary(primary: u64) -> Self {
        Self {
            primary,
            ..Self::empty()
        }
    }

    /// Create a payload with primary and secondary values.
    #[must_use]
    pub fn with_values(primary: u64, secondary: u64) -> Self {
        Self {
            primary,
            secondary,
            ..Self::empty()
        }
    }

    /// Builder: set primary value.
    #[must_use]
    pub const fn primary(mut self, value: u64) -> Self {
        self.primary = value;
        self
    }

    /// Builder: set secondary value.
    #[must_use]
    pub const fn secondary(mut self, value: u64) -> Self {
        self.secondary = value;
        self
    }

    /// Builder: set tertiary values.
    #[must_use]
    pub const fn tertiary(mut self, values: [i32; 4]) -> Self {
        self.tertiary = values;
        self
    }

    /// Builder: set label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Check if payload is empty (all zeros, no label).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.primary == 0 && self.secondary == 0 && self.tertiary == [0; 4] && self.label.is_empty()
    }
}

/// A single event record in the region journal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    /// Simulation tick when event occurred.
    tick: u64,
    /// Monotonically increasing sequence number for ordering within tick.
    sequence: u64,
    /// Chunk position where event occurred.
    chunk_pos: ChunkPos,
    /// Severity level.
    severity: Severity,
    /// Event kind (determines category).
    kind: EventKind,
    /// Source system identifier.
    source_id: u32,
    /// Affected local positions within chunk (optional).
    affected_positions: Vec<LocalPos>,
    /// Tags for filtering and categorization.
    tags: Vec<String>,
    /// Event-specific payload.
    payload: EventPayload,
}

impl EventRecord {
    /// Create a new event record.
    #[must_use]
    pub fn new(tick: u64, sequence: u64, chunk_pos: ChunkPos, kind: EventKind) -> Self {
        Self {
            tick,
            sequence,
            chunk_pos,
            severity: Severity::Info,
            kind,
            source_id: 0,
            affected_positions: Vec::new(),
            tags: Vec::new(),
            payload: EventPayload::empty(),
        }
    }

    /// Get the tick.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Get the sequence number.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get the chunk position.
    #[must_use]
    pub const fn chunk_pos(&self) -> ChunkPos {
        self.chunk_pos
    }

    /// Get the severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Get the event kind.
    #[must_use]
    pub const fn kind(&self) -> EventKind {
        self.kind
    }

    /// Get the source system ID.
    #[must_use]
    pub const fn source_id(&self) -> u32 {
        self.source_id
    }

    /// Get affected local positions.
    #[must_use]
    pub fn affected_positions(&self) -> &[LocalPos] {
        &self.affected_positions
    }

    /// Get tags.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Get the payload.
    #[must_use]
    pub const fn payload(&self) -> &EventPayload {
        &self.payload
    }

    /// Builder: set severity.
    #[must_use]
    pub const fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Builder: set source ID.
    #[must_use]
    pub const fn with_source_id(mut self, source_id: u32) -> Self {
        self.source_id = source_id;
        self
    }

    /// Builder: set affected positions.
    #[must_use]
    pub fn with_affected_positions(mut self, positions: Vec<LocalPos>) -> Self {
        self.affected_positions = positions;
        self
    }

    /// Builder: add a single affected position.
    #[must_use]
    pub fn with_affected_position(mut self, pos: LocalPos) -> Self {
        self.affected_positions.push(pos);
        self
    }

    /// Builder: set tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: add a single tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Builder: set payload.
    #[must_use]
    pub fn with_payload(mut self, payload: EventPayload) -> Self {
        self.payload = payload;
        self
    }

    /// Check if record has a specific tag.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Set sequence (used internally by journal).
    pub(crate) fn set_sequence(&mut self, sequence: u64) {
        self.sequence = sequence;
    }
}

impl PartialOrd for EventRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.tick.cmp(&other.tick) {
            std::cmp::Ordering::Equal => self.sequence.cmp(&other.sequence),
            ord => ord,
        }
    }
}

impl Eq for EventRecord {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_empty() {
        let payload = EventPayload::empty();
        assert!(payload.is_empty());
    }

    #[test]
    fn payload_builder() {
        let payload = EventPayload::empty()
            .primary(42)
            .secondary(100)
            .tertiary([1, 2, 3, 4])
            .label("test");

        assert_eq!(payload.primary, 42);
        assert_eq!(payload.secondary, 100);
        assert_eq!(payload.tertiary, [1, 2, 3, 4]);
        assert_eq!(payload.label, "test");
        assert!(!payload.is_empty());
    }

    #[test]
    fn record_new() {
        let record = EventRecord::new(100, 0, ChunkPos::new(1, 2, 3), EventKind::ChunkLoaded);

        assert_eq!(record.tick(), 100);
        assert_eq!(record.sequence(), 0);
        assert_eq!(record.chunk_pos(), ChunkPos::new(1, 2, 3));
        assert_eq!(record.kind(), EventKind::ChunkLoaded);
        assert_eq!(record.severity(), Severity::Info);
    }

    #[test]
    fn record_builder() {
        let record = EventRecord::new(50, 1, ChunkPos::new(0, 0, 0), EventKind::HazardSpawn)
            .with_severity(Severity::Warning)
            .with_source_id(123)
            .with_tag("fire")
            .with_tag("dangerous")
            .with_payload(EventPayload::with_primary(999));

        assert_eq!(record.severity(), Severity::Warning);
        assert_eq!(record.source_id(), 123);
        assert!(record.has_tag("fire"));
        assert!(record.has_tag("dangerous"));
        assert!(!record.has_tag("water"));
        assert_eq!(record.payload().primary, 999);
    }

    #[test]
    fn record_ordering_by_tick() {
        let r1 = EventRecord::new(100, 0, ChunkPos::new(0, 0, 0), EventKind::ChunkLoaded);
        let r2 = EventRecord::new(200, 0, ChunkPos::new(0, 0, 0), EventKind::ChunkLoaded);

        assert!(r1 < r2);
    }

    #[test]
    fn record_ordering_by_sequence() {
        let r1 = EventRecord::new(100, 0, ChunkPos::new(0, 0, 0), EventKind::ChunkLoaded);
        let r2 = EventRecord::new(100, 1, ChunkPos::new(0, 0, 0), EventKind::ChunkLoaded);

        assert!(r1 < r2);
    }

    #[test]
    fn record_affected_positions() {
        let record = EventRecord::new(100, 0, ChunkPos::new(0, 0, 0), EventKind::BlockModified)
            .with_affected_position(LocalPos::new(1, 2, 3))
            .with_affected_position(LocalPos::new(4, 5, 6));

        assert_eq!(record.affected_positions().len(), 2);
        assert_eq!(record.affected_positions()[0], LocalPos::new(1, 2, 3));
    }

    #[test]
    fn serde_round_trip_payload() {
        let payload = EventPayload::empty()
            .primary(12345)
            .secondary(67890)
            .tertiary([1, -2, 3, -4])
            .label("test_label");

        let json = serde_json::to_string(&payload).unwrap();
        let recovered: EventPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn serde_round_trip_record() {
        let record = EventRecord::new(100, 5, ChunkPos::new(10, 20, 30), EventKind::HazardSpawn)
            .with_severity(Severity::Warning)
            .with_source_id(42)
            .with_tag("test")
            .with_affected_position(LocalPos::new(1, 2, 3))
            .with_payload(EventPayload::with_primary(999));

        let json = serde_json::to_string(&record).unwrap();
        let recovered: EventRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, record);
    }
}

//! Recording types for deterministic replay.

use serde::{Deserialize, Serialize};

use super::{ReplayMetadata, StepChecksum};

/// Kind of replay entry for categorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplayEntryKind {
    /// A world event started.
    WorldEventStart,
    /// A world event ended.
    WorldEventEnd,
    /// A scheduler job was dispatched.
    SchedulerJob,
    /// An environment hint was applied.
    EnvironmentHint,
    /// A per-step checksum was recorded.
    StepChecksum,
    /// A season transition occurred.
    SeasonTransition,
    /// A custom user-defined entry.
    Custom,
}

/// A single recorded entry in the replay log.
///
/// Entries are ordered by tick, then by sequence number within tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayEntry {
    /// The tick at which this entry occurred.
    tick: u64,
    /// Sequence number within the tick (for deterministic ordering).
    sequence: u32,
    /// Entry kind for categorization.
    kind: ReplayEntryKind,
    /// Associated identifier (event ID, job position hash, etc.).
    identifier: u64,
    /// Human-readable label for debugging.
    label: String,
    /// Optional checksum for this entry.
    checksum: Option<StepChecksum>,
    /// Optional position (x, y, z) for spatial entries.
    position: Option<(i32, i32, i32)>,
}

impl ReplayEntry {
    /// Create a new replay entry.
    #[must_use]
    pub fn new(tick: u64, sequence: u32, kind: ReplayEntryKind, identifier: u64) -> Self {
        Self {
            tick,
            sequence,
            kind,
            identifier,
            label: String::new(),
            checksum: None,
            position: None,
        }
    }

    /// Get the tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Get the sequence number.
    #[must_use]
    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Get the entry kind.
    #[must_use]
    pub fn kind(&self) -> ReplayEntryKind {
        self.kind
    }

    /// Get the identifier.
    #[must_use]
    pub fn identifier(&self) -> u64 {
        self.identifier
    }

    /// Get the label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get the checksum.
    #[must_use]
    pub fn checksum(&self) -> Option<StepChecksum> {
        self.checksum
    }

    /// Get the position.
    #[must_use]
    pub fn position(&self) -> Option<(i32, i32, i32)> {
        self.position
    }

    /// Set the label.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// Set the checksum.
    pub fn set_checksum(&mut self, checksum: StepChecksum) {
        self.checksum = Some(checksum);
    }

    /// Set the position.
    pub fn set_position(&mut self, x: i32, y: i32, z: i32) {
        self.position = Some((x, y, z));
    }

    /// Builder-style: set label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Builder-style: set checksum.
    #[must_use]
    pub fn with_checksum(mut self, checksum: StepChecksum) -> Self {
        self.checksum = Some(checksum);
        self
    }

    /// Builder-style: set position.
    #[must_use]
    pub fn with_position(mut self, x: i32, y: i32, z: i32) -> Self {
        self.position = Some((x, y, z));
        self
    }
}

impl PartialOrd for ReplayEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReplayEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.tick.cmp(&other.tick) {
            std::cmp::Ordering::Equal => self.sequence.cmp(&other.sequence),
            ord => ord,
        }
    }
}

impl Eq for ReplayEntry {}

/// Records simulation events and checksums for deterministic replay.
///
/// The recorder maintains an ordered log of entries that can be used
/// for playback verification. All recording operations are deterministic
/// and produce the same output given the same inputs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayRecorder {
    /// Session metadata.
    metadata: ReplayMetadata,
    /// Ordered list of recorded entries.
    entries: Vec<ReplayEntry>,
    /// Current tick (for relative recording).
    current_tick: u64,
    /// Maximum tick recorded (for `end_tick` in metadata).
    max_tick: u64,
    /// Sequence counter for entries within current tick.
    tick_sequence: u32,
}

impl ReplayRecorder {
    /// Create a new recorder with the given metadata.
    #[must_use]
    pub fn new(metadata: ReplayMetadata) -> Self {
        let start_tick = metadata.start_tick();
        Self {
            metadata,
            entries: Vec::new(),
            current_tick: start_tick,
            max_tick: start_tick,
            tick_sequence: 0,
        }
    }

    /// Get the metadata.
    #[must_use]
    pub fn metadata(&self) -> &ReplayMetadata {
        &self.metadata
    }

    /// Get mutable access to metadata.
    pub fn metadata_mut(&mut self) -> &mut ReplayMetadata {
        &mut self.metadata
    }

    /// Get all recorded entries (sorted).
    #[must_use]
    pub fn entries(&self) -> &[ReplayEntry] {
        &self.entries
    }

    /// Get the number of recorded entries.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Advance to a new tick.
    ///
    /// Resets the sequence counter and updates the current tick.
    pub fn advance_to_tick(&mut self, tick: u64) {
        if tick != self.current_tick {
            self.current_tick = tick;
            self.tick_sequence = 0;
        }
        self.max_tick = self.max_tick.max(tick);
    }

    /// Record a world event start.
    pub fn record_world_event_start(&mut self, tick: u64, event_id: u64, label: &str) {
        self.advance_to_tick(tick);
        let entry = ReplayEntry::new(
            tick,
            self.tick_sequence,
            ReplayEntryKind::WorldEventStart,
            event_id,
        )
        .with_label(label);
        self.push_entry(entry);
    }

    /// Record a world event end.
    pub fn record_world_event_end(&mut self, tick: u64, event_id: u64, label: &str) {
        self.advance_to_tick(tick);
        let entry = ReplayEntry::new(
            tick,
            self.tick_sequence,
            ReplayEntryKind::WorldEventEnd,
            event_id,
        )
        .with_label(label);
        self.push_entry(entry);
    }

    /// Record a scheduler job dispatch.
    pub fn record_scheduler_job(
        &mut self,
        tick: u64,
        position: (i32, i32, i32),
        priority: i64,
        label: &str,
    ) {
        self.advance_to_tick(tick);
        #[expect(clippy::cast_sign_loss, reason = "priority stored as identifier")]
        let identifier = priority as u64;
        let entry = ReplayEntry::new(
            tick,
            self.tick_sequence,
            ReplayEntryKind::SchedulerJob,
            identifier,
        )
        .with_label(label)
        .with_position(position.0, position.1, position.2);
        self.push_entry(entry);
    }

    /// Record an environment hint application.
    pub fn record_environment_hint(
        &mut self,
        tick: u64,
        position: (i32, i32, i32),
        hint_type: &str,
    ) {
        self.advance_to_tick(tick);
        let entry = ReplayEntry::new(
            tick,
            self.tick_sequence,
            ReplayEntryKind::EnvironmentHint,
            0,
        )
        .with_label(hint_type)
        .with_position(position.0, position.1, position.2);
        self.push_entry(entry);
    }

    /// Record a per-step checksum.
    pub fn record_step_checksum(&mut self, tick: u64, checksum: impl Into<StepChecksum>) {
        self.advance_to_tick(tick);
        let entry = ReplayEntry::new(tick, self.tick_sequence, ReplayEntryKind::StepChecksum, 0)
            .with_checksum(checksum.into());
        self.push_entry(entry);
    }

    /// Record a season transition.
    pub fn record_season_transition(&mut self, tick: u64, from_season: u8, to_season: u8) {
        self.advance_to_tick(tick);
        let identifier = u64::from(from_season) << 8 | u64::from(to_season);
        let label = format!("{from_season} -> {to_season}");
        let entry = ReplayEntry::new(
            tick,
            self.tick_sequence,
            ReplayEntryKind::SeasonTransition,
            identifier,
        )
        .with_label(label);
        self.push_entry(entry);
    }

    /// Record a custom entry.
    pub fn record_custom(
        &mut self,
        tick: u64,
        identifier: u64,
        label: &str,
        checksum: Option<StepChecksum>,
    ) {
        self.advance_to_tick(tick);
        let mut entry = ReplayEntry::new(
            tick,
            self.tick_sequence,
            ReplayEntryKind::Custom,
            identifier,
        )
        .with_label(label);
        if let Some(cs) = checksum {
            entry = entry.with_checksum(cs);
        }
        self.push_entry(entry);
    }

    /// Record a raw entry directly.
    pub fn record_entry(&mut self, mut entry: ReplayEntry) {
        self.advance_to_tick(entry.tick());
        entry.sequence = self.tick_sequence;
        self.push_entry(entry);
    }

    /// Finalize the recording and update metadata end tick.
    pub fn finalize(&mut self) {
        self.metadata.set_end_tick(self.max_tick);
        self.entries.sort();
    }

    /// Get entries for a specific tick.
    pub fn entries_at_tick(&self, tick: u64) -> impl Iterator<Item = &ReplayEntry> {
        self.entries.iter().filter(move |e| e.tick() == tick)
    }

    /// Get entries of a specific kind.
    pub fn entries_of_kind(&self, kind: ReplayEntryKind) -> impl Iterator<Item = &ReplayEntry> {
        self.entries.iter().filter(move |e| e.kind() == kind)
    }

    /// Get all checksum entries.
    pub fn checksum_entries(&self) -> impl Iterator<Item = &ReplayEntry> {
        self.entries_of_kind(ReplayEntryKind::StepChecksum)
    }

    /// Clear all entries (keeps metadata).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.tick_sequence = 0;
        self.current_tick = self.metadata.start_tick();
        self.max_tick = self.metadata.start_tick();
    }

    fn push_entry(&mut self, entry: ReplayEntry) {
        self.entries.push(entry);
        self.tick_sequence = self.tick_sequence.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_recorder() -> ReplayRecorder {
        let metadata = ReplayMetadata::new("test", 0, 1000);
        ReplayRecorder::new(metadata)
    }

    #[test]
    fn entry_ordering() {
        let e1 = ReplayEntry::new(100, 0, ReplayEntryKind::StepChecksum, 0);
        let e2 = ReplayEntry::new(100, 1, ReplayEntryKind::StepChecksum, 0);
        let e3 = ReplayEntry::new(101, 0, ReplayEntryKind::StepChecksum, 0);

        assert!(e1 < e2);
        assert!(e2 < e3);
        assert!(e1 < e3);
    }

    #[test]
    fn entry_builder() {
        let entry = ReplayEntry::new(50, 0, ReplayEntryKind::WorldEventStart, 42)
            .with_label("Eclipse")
            .with_checksum(StepChecksum::from_raw(1234))
            .with_position(1, 2, 3);

        assert_eq!(entry.tick(), 50);
        assert_eq!(entry.identifier(), 42);
        assert_eq!(entry.label(), "Eclipse");
        assert!(entry.checksum().is_some());
        assert_eq!(entry.position(), Some((1, 2, 3)));
    }

    #[test]
    fn record_world_events() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Eclipse");
        recorder.record_world_event_end(200, 1, "Eclipse");

        assert_eq!(recorder.entry_count(), 2);

        let starts: Vec<_> = recorder
            .entries_of_kind(ReplayEntryKind::WorldEventStart)
            .collect();
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].tick(), 100);
        assert_eq!(starts[0].identifier(), 1);
    }

    #[test]
    fn record_scheduler_jobs() {
        let mut recorder = make_recorder();
        recorder.record_scheduler_job(50, (10, 20, 30), 500, "Near");

        let jobs: Vec<_> = recorder
            .entries_of_kind(ReplayEntryKind::SchedulerJob)
            .collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].position(), Some((10, 20, 30)));
        assert_eq!(jobs[0].label(), "Near");
    }

    #[test]
    fn record_checksums() {
        let mut recorder = make_recorder();
        recorder.record_step_checksum(100, 0xDEAD_BEEF_u32);
        recorder.record_step_checksum(101, StepChecksum::from_raw(0xCAFE_BABE));

        let checksums: Vec<_> = recorder.checksum_entries().collect();
        assert_eq!(checksums.len(), 2);
        assert_eq!(checksums[0].checksum().unwrap().value(), 0xDEAD_BEEF);
        assert_eq!(checksums[1].checksum().unwrap().value(), 0xCAFE_BABE);
    }

    #[test]
    fn record_environment_hints() {
        let mut recorder = make_recorder();
        recorder.record_environment_hint(75, (5, 5, 5), "structural");

        let hints: Vec<_> = recorder
            .entries_of_kind(ReplayEntryKind::EnvironmentHint)
            .collect();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label(), "structural");
    }

    #[test]
    fn record_season_transition() {
        let mut recorder = make_recorder();
        recorder.record_season_transition(1000, 0, 1);

        let transitions: Vec<_> = recorder
            .entries_of_kind(ReplayEntryKind::SeasonTransition)
            .collect();
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].identifier(), 0x0001);
    }

    #[test]
    fn record_custom() {
        let mut recorder = make_recorder();
        recorder.record_custom(200, 999, "custom_event", Some(StepChecksum::from_raw(111)));

        let custom: Vec<_> = recorder.entries_of_kind(ReplayEntryKind::Custom).collect();
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].identifier(), 999);
        assert_eq!(custom[0].label(), "custom_event");
    }

    #[test]
    fn sequence_within_tick() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "A");
        recorder.record_world_event_start(100, 2, "B");
        recorder.record_world_event_start(100, 3, "C");

        let at_100: Vec<_> = recorder.entries_at_tick(100).collect();
        assert_eq!(at_100.len(), 3);
        assert_eq!(at_100[0].sequence(), 0);
        assert_eq!(at_100[1].sequence(), 1);
        assert_eq!(at_100[2].sequence(), 2);
    }

    #[test]
    fn sequence_resets_on_tick_change() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "A");
        recorder.record_world_event_start(100, 2, "B");
        recorder.record_world_event_start(200, 3, "C");

        let at_200: Vec<_> = recorder.entries_at_tick(200).collect();
        assert_eq!(at_200[0].sequence(), 0);
    }

    #[test]
    fn finalize_sorts_and_updates_metadata() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(300, 1, "Late");
        recorder.record_world_event_start(100, 2, "Early");
        recorder.finalize();

        assert_eq!(recorder.metadata().end_tick(), 300);
        assert_eq!(recorder.entries()[0].tick(), 100);
        assert_eq!(recorder.entries()[1].tick(), 300);
    }

    #[test]
    fn clear_keeps_metadata() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Test");
        recorder.clear();

        assert_eq!(recorder.entry_count(), 0);
        assert_eq!(recorder.metadata().seed_identifier(), "test");
    }

    #[test]
    fn serde_round_trip_entry() {
        let entry = ReplayEntry::new(100, 5, ReplayEntryKind::SchedulerJob, 42)
            .with_label("test_job")
            .with_checksum(StepChecksum::from_raw(0xABCD))
            .with_position(1, 2, 3);

        let json = serde_json::to_string(&entry).unwrap();
        let recovered: ReplayEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, entry);
    }

    #[test]
    fn serde_round_trip_recorder() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Eclipse");
        recorder.record_step_checksum(100, 0xDEAD_BEEF_u32);
        recorder.record_scheduler_job(150, (5, 5, 5), 100, "Near");
        recorder.finalize();

        let json = serde_json::to_string(&recorder).unwrap();
        let recovered: ReplayRecorder = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.entry_count(), recorder.entry_count());
        assert_eq!(
            recovered.metadata().seed_identifier(),
            recorder.metadata().seed_identifier()
        );
    }

    #[test]
    fn deterministic_recording_order() {
        let record_session = |recorder: &mut ReplayRecorder| {
            recorder.record_world_event_start(100, 1, "A");
            recorder.record_scheduler_job(100, (0, 0, 0), 10, "job1");
            recorder.record_step_checksum(100, 0x1111_u32);
            recorder.record_world_event_start(100, 2, "B");
            recorder.finalize();
        };

        let mut r1 = make_recorder();
        record_session(&mut r1);

        let mut r2 = make_recorder();
        record_session(&mut r2);

        assert_eq!(r1.entries().len(), r2.entries().len());
        for (e1, e2) in r1.entries().iter().zip(r2.entries().iter()) {
            assert_eq!(e1.tick(), e2.tick());
            assert_eq!(e1.sequence(), e2.sequence());
            assert_eq!(e1.kind(), e2.kind());
            assert_eq!(e1.identifier(), e2.identifier());
        }
    }
}

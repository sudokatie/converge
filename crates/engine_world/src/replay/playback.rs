//! Playback and verification for deterministic replay.

use serde::{Deserialize, Serialize};

use super::{ReplayEntry, ReplayEntryKind, ReplayRecorder, StepChecksum};

/// Kind of mismatch detected during replay verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MismatchKind {
    /// A checksum did not match.
    ChecksumMismatch,
    /// An event was expected but not found.
    MissingEvent,
    /// An unexpected event was encountered.
    UnexpectedEvent,
    /// Event identifier did not match.
    IdentifierMismatch,
    /// Event label did not match.
    LabelMismatch,
    /// Position did not match.
    PositionMismatch,
    /// Sequence order did not match.
    SequenceMismatch,
}

/// Details about a detected mismatch during replay verification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mismatch {
    /// The tick where the mismatch occurred.
    tick: u64,
    /// The kind of mismatch.
    kind: MismatchKind,
    /// The entry kind involved (if applicable).
    entry_kind: Option<ReplayEntryKind>,
    /// Expected value (stringified for display).
    expected: String,
    /// Actual value (stringified for display).
    actual: String,
    /// Additional context for debugging.
    context: String,
}

impl Mismatch {
    /// Create a new mismatch.
    #[must_use]
    pub fn new(tick: u64, kind: MismatchKind, expected: String, actual: String) -> Self {
        Self {
            tick,
            kind,
            entry_kind: None,
            expected,
            actual,
            context: String::new(),
        }
    }

    /// Get the tick where the mismatch occurred.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Get the mismatch kind.
    #[must_use]
    pub fn kind(&self) -> &MismatchKind {
        &self.kind
    }

    /// Get the entry kind involved.
    #[must_use]
    pub fn entry_kind(&self) -> Option<ReplayEntryKind> {
        self.entry_kind
    }

    /// Get the expected value.
    #[must_use]
    pub fn expected(&self) -> &str {
        &self.expected
    }

    /// Get the actual value.
    #[must_use]
    pub fn actual(&self) -> &str {
        &self.actual
    }

    /// Get additional context.
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    /// Set the entry kind.
    pub fn set_entry_kind(&mut self, kind: ReplayEntryKind) {
        self.entry_kind = Some(kind);
    }

    /// Set additional context.
    pub fn set_context(&mut self, context: impl Into<String>) {
        self.context = context.into();
    }

    /// Builder-style: set entry kind.
    #[must_use]
    pub fn with_entry_kind(mut self, kind: ReplayEntryKind) -> Self {
        self.entry_kind = Some(kind);
        self
    }

    /// Builder-style: set context.
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    /// Create a checksum mismatch.
    #[must_use]
    pub fn checksum(tick: u64, expected: StepChecksum, actual: StepChecksum) -> Self {
        Self::new(
            tick,
            MismatchKind::ChecksumMismatch,
            format!("0x{:08X}", expected.value()),
            format!("0x{:08X}", actual.value()),
        )
        .with_entry_kind(ReplayEntryKind::StepChecksum)
    }

    /// Create a missing event mismatch.
    #[must_use]
    pub fn missing_event(tick: u64, expected_entry: &ReplayEntry) -> Self {
        Self::new(
            tick,
            MismatchKind::MissingEvent,
            format!(
                "id={} label={}",
                expected_entry.identifier(),
                expected_entry.label()
            ),
            "not found".to_string(),
        )
        .with_entry_kind(expected_entry.kind())
    }

    /// Create an unexpected event mismatch.
    #[must_use]
    pub fn unexpected_event(tick: u64, actual_entry: &ReplayEntry) -> Self {
        Self::new(
            tick,
            MismatchKind::UnexpectedEvent,
            "nothing".to_string(),
            format!(
                "id={} label={}",
                actual_entry.identifier(),
                actual_entry.label()
            ),
        )
        .with_entry_kind(actual_entry.kind())
    }

    /// Create an identifier mismatch.
    #[must_use]
    pub fn identifier(tick: u64, entry_kind: ReplayEntryKind, expected: u64, actual: u64) -> Self {
        Self::new(
            tick,
            MismatchKind::IdentifierMismatch,
            expected.to_string(),
            actual.to_string(),
        )
        .with_entry_kind(entry_kind)
    }

    /// Create a position mismatch.
    #[must_use]
    pub fn position(
        tick: u64,
        entry_kind: ReplayEntryKind,
        expected: (i32, i32, i32),
        actual: (i32, i32, i32),
    ) -> Self {
        Self::new(
            tick,
            MismatchKind::PositionMismatch,
            format!("({}, {}, {})", expected.0, expected.1, expected.2),
            format!("({}, {}, {})", actual.0, actual.1, actual.2),
        )
        .with_entry_kind(entry_kind)
    }
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tick {}: {:?} - expected {}, got {}",
            self.tick, self.kind, self.expected, self.actual
        )?;
        if !self.context.is_empty() {
            write!(f, " ({})", self.context)?;
        }
        Ok(())
    }
}

/// Verifies replay data against a replayed simulation run.
///
/// The verifier compares recorded events, jobs, and checksums against
/// actual values from the replayed simulation, reporting any mismatches
/// found.
#[derive(Clone, Debug)]
pub struct ReplayVerifier {
    /// Expected entries from the recording.
    expected_entries: Vec<ReplayEntry>,
    /// Current position in expected entries.
    cursor: usize,
    /// Detected mismatches.
    mismatches: Vec<Mismatch>,
    /// Current tick being verified.
    current_tick: u64,
    /// Sequence counter for current tick.
    tick_sequence: u32,
    /// Whether to stop on first mismatch.
    stop_on_first: bool,
}

impl ReplayVerifier {
    /// Create a new verifier from recorded entries.
    #[must_use]
    pub fn new(expected_entries: Vec<ReplayEntry>) -> Self {
        Self {
            expected_entries,
            cursor: 0,
            mismatches: Vec::new(),
            current_tick: 0,
            tick_sequence: 0,
            stop_on_first: false,
        }
    }

    /// Create a verifier from a recorder.
    #[must_use]
    pub fn from_recorder(recorder: &ReplayRecorder) -> Self {
        Self::new(recorder.entries().to_vec())
    }

    /// Set whether to stop on the first mismatch.
    pub fn set_stop_on_first(&mut self, stop: bool) {
        self.stop_on_first = stop;
    }

    /// Builder-style: stop on first mismatch.
    #[must_use]
    pub fn with_stop_on_first(mut self, stop: bool) -> Self {
        self.stop_on_first = stop;
        self
    }

    /// Get all detected mismatches.
    #[must_use]
    pub fn mismatches(&self) -> &[Mismatch] {
        &self.mismatches
    }

    /// Check if verification passed (no mismatches).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.mismatches.is_empty()
    }

    /// Get the number of mismatches.
    #[must_use]
    pub fn mismatch_count(&self) -> usize {
        self.mismatches.len()
    }

    /// Get the first mismatch tick (if any).
    #[must_use]
    pub fn first_mismatch_tick(&self) -> Option<u64> {
        self.mismatches.first().map(Mismatch::tick)
    }

    /// Get the current verification cursor position.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Get remaining entries to verify.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.expected_entries.len().saturating_sub(self.cursor)
    }

    /// Advance to a new tick.
    pub fn advance_to_tick(&mut self, tick: u64) {
        if tick != self.current_tick {
            self.current_tick = tick;
            self.tick_sequence = 0;
        }
    }

    /// Verify a world event start.
    ///
    /// Returns true if the event matches the expected entry.
    pub fn verify_world_event_start(&mut self, tick: u64, event_id: u64, label: &str) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        self.verify_entry(
            tick,
            ReplayEntryKind::WorldEventStart,
            event_id,
            label,
            None,
        )
    }

    /// Verify a world event end.
    pub fn verify_world_event_end(&mut self, tick: u64, event_id: u64, label: &str) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        self.verify_entry(tick, ReplayEntryKind::WorldEventEnd, event_id, label, None)
    }

    /// Verify a scheduler job dispatch.
    pub fn verify_scheduler_job(
        &mut self,
        tick: u64,
        position: (i32, i32, i32),
        priority: i64,
        label: &str,
    ) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        #[expect(clippy::cast_sign_loss, reason = "priority stored as identifier")]
        let identifier = priority as u64;
        self.verify_entry(
            tick,
            ReplayEntryKind::SchedulerJob,
            identifier,
            label,
            Some(position),
        )
    }

    /// Verify an environment hint.
    pub fn verify_environment_hint(
        &mut self,
        tick: u64,
        position: (i32, i32, i32),
        hint_type: &str,
    ) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        self.verify_entry(
            tick,
            ReplayEntryKind::EnvironmentHint,
            0,
            hint_type,
            Some(position),
        )
    }

    /// Verify a per-step checksum.
    pub fn verify_step_checksum(&mut self, tick: u64, checksum: impl Into<StepChecksum>) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        let actual_checksum = checksum.into();

        let matches_expected = self
            .expected_entries
            .get(self.cursor)
            .is_some_and(|e| e.tick() == tick && e.kind() == ReplayEntryKind::StepChecksum);

        if matches_expected {
            let entry = self.expected_entries[self.cursor].clone();
            self.cursor += 1;
            self.tick_sequence = self.tick_sequence.saturating_add(1);

            if let Some(expected_cs) = entry.checksum()
                && expected_cs != actual_checksum
            {
                self.mismatches
                    .push(Mismatch::checksum(tick, expected_cs, actual_checksum));
                return false;
            }
            true
        } else {
            let actual_entry =
                ReplayEntry::new(tick, self.tick_sequence, ReplayEntryKind::StepChecksum, 0)
                    .with_checksum(actual_checksum);
            self.mismatches
                .push(Mismatch::unexpected_event(tick, &actual_entry));
            self.tick_sequence = self.tick_sequence.saturating_add(1);
            false
        }
    }

    /// Verify a season transition.
    pub fn verify_season_transition(&mut self, tick: u64, from_season: u8, to_season: u8) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        let identifier = u64::from(from_season) << 8 | u64::from(to_season);
        let label = format!("{from_season} -> {to_season}");
        self.verify_entry(
            tick,
            ReplayEntryKind::SeasonTransition,
            identifier,
            &label,
            None,
        )
    }

    /// Verify a custom entry.
    pub fn verify_custom(&mut self, tick: u64, identifier: u64, label: &str) -> bool {
        if self.should_stop() {
            return false;
        }
        self.advance_to_tick(tick);
        self.verify_entry(tick, ReplayEntryKind::Custom, identifier, label, None)
    }

    /// Check for any remaining unverified entries at the end of replay.
    ///
    /// Call this after all replay events have been verified to detect
    /// missing events in the replayed run.
    pub fn finish_verification(&mut self) {
        while self.cursor < self.expected_entries.len() {
            let entry = &self.expected_entries[self.cursor];
            self.mismatches
                .push(Mismatch::missing_event(entry.tick(), entry));
            self.cursor += 1;
        }
    }

    /// Reset the verifier for a new verification pass.
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.mismatches.clear();
        self.current_tick = 0;
        self.tick_sequence = 0;
    }

    /// Get mismatches of a specific kind.
    pub fn mismatches_of_kind(&self, kind: &MismatchKind) -> impl Iterator<Item = &Mismatch> {
        self.mismatches.iter().filter(move |m| m.kind() == kind)
    }

    /// Get checksum mismatches.
    pub fn checksum_mismatches(&self) -> impl Iterator<Item = &Mismatch> {
        self.mismatches_of_kind(&MismatchKind::ChecksumMismatch)
    }

    fn should_stop(&self) -> bool {
        self.stop_on_first && !self.mismatches.is_empty()
    }

    fn peek_expected(&self, tick: u64, kind: ReplayEntryKind) -> Option<&ReplayEntry> {
        self.expected_entries.get(self.cursor).and_then(|entry| {
            if entry.tick() == tick && entry.kind() == kind {
                Some(entry)
            } else {
                None
            }
        })
    }

    fn verify_entry(
        &mut self,
        tick: u64,
        kind: ReplayEntryKind,
        identifier: u64,
        label: &str,
        position: Option<(i32, i32, i32)>,
    ) -> bool {
        let Some(expected) = self.peek_expected(tick, kind) else {
            let actual_entry =
                ReplayEntry::new(tick, self.tick_sequence, kind, identifier).with_label(label);
            self.mismatches
                .push(Mismatch::unexpected_event(tick, &actual_entry));
            self.tick_sequence = self.tick_sequence.saturating_add(1);
            return false;
        };

        let entry = expected.clone();
        self.cursor += 1;
        self.tick_sequence = self.tick_sequence.saturating_add(1);

        let mut valid = true;

        if entry.identifier() != identifier {
            self.mismatches.push(Mismatch::identifier(
                tick,
                kind,
                entry.identifier(),
                identifier,
            ));
            valid = false;
        }

        if !label.is_empty() && entry.label() != label {
            self.mismatches.push(
                Mismatch::new(
                    tick,
                    MismatchKind::LabelMismatch,
                    entry.label().to_string(),
                    label.to_string(),
                )
                .with_entry_kind(kind),
            );
            valid = false;
        }

        if let (Some(expected_pos), Some(actual_pos)) = (entry.position(), position)
            && expected_pos != actual_pos
        {
            self.mismatches
                .push(Mismatch::position(tick, kind, expected_pos, actual_pos));
            valid = false;
        }

        valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::ReplayMetadata;

    fn make_recorder() -> ReplayRecorder {
        let metadata = ReplayMetadata::new("test", 0, 1000);
        ReplayRecorder::new(metadata)
    }

    #[test]
    fn verify_matching_events() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Eclipse");
        recorder.record_step_checksum(100, 0xDEAD_BEEF_u32);
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(verifier.verify_world_event_start(100, 1, "Eclipse"));
        assert!(verifier.verify_step_checksum(100, 0xDEAD_BEEF_u32));
        verifier.finish_verification();

        assert!(verifier.is_valid());
        assert_eq!(verifier.mismatch_count(), 0);
    }

    #[test]
    fn detect_checksum_mismatch() {
        let mut recorder = make_recorder();
        recorder.record_step_checksum(100, 0xAAAA_AAAA_u32);
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(!verifier.verify_step_checksum(100, 0xBBBB_BBBB_u32));

        assert!(!verifier.is_valid());
        assert_eq!(verifier.mismatch_count(), 1);

        let mismatch = &verifier.mismatches()[0];
        assert_eq!(mismatch.tick(), 100);
        assert_eq!(*mismatch.kind(), MismatchKind::ChecksumMismatch);
        assert_eq!(mismatch.expected(), "0xAAAAAAAA");
        assert_eq!(mismatch.actual(), "0xBBBBBBBB");
    }

    #[test]
    fn detect_missing_event() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Eclipse");
        recorder.record_world_event_start(200, 2, "Collapse");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(verifier.verify_world_event_start(100, 1, "Eclipse"));
        verifier.finish_verification();

        assert!(!verifier.is_valid());
        let missing: Vec<_> = verifier
            .mismatches_of_kind(&MismatchKind::MissingEvent)
            .collect();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].tick(), 200);
    }

    #[test]
    fn detect_unexpected_event() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Eclipse");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(verifier.verify_world_event_start(100, 1, "Eclipse"));
        assert!(!verifier.verify_world_event_start(200, 2, "Extra"));

        assert!(!verifier.is_valid());
        let unexpected: Vec<_> = verifier
            .mismatches_of_kind(&MismatchKind::UnexpectedEvent)
            .collect();
        assert_eq!(unexpected.len(), 1);
    }

    #[test]
    fn detect_identifier_mismatch() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "Eclipse");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(!verifier.verify_world_event_start(100, 999, "Eclipse"));

        let mismatches: Vec<_> = verifier
            .mismatches_of_kind(&MismatchKind::IdentifierMismatch)
            .collect();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].expected(), "1");
        assert_eq!(mismatches[0].actual(), "999");
    }

    #[test]
    fn detect_position_mismatch() {
        let mut recorder = make_recorder();
        recorder.record_scheduler_job(100, (10, 20, 30), 500, "Near");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(!verifier.verify_scheduler_job(100, (99, 99, 99), 500, "Near"));

        let mismatches: Vec<_> = verifier
            .mismatches_of_kind(&MismatchKind::PositionMismatch)
            .collect();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].expected(), "(10, 20, 30)");
        assert_eq!(mismatches[0].actual(), "(99, 99, 99)");
    }

    #[test]
    fn stop_on_first_mismatch() {
        let mut recorder = make_recorder();
        recorder.record_step_checksum(100, 0xAAAA_u32);
        recorder.record_step_checksum(200, 0xBBBB_u32);
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder).with_stop_on_first(true);
        verifier.verify_step_checksum(100, 0xFFFF_u32);
        verifier.verify_step_checksum(200, 0xEEEE_u32);

        assert_eq!(verifier.mismatch_count(), 1);
        assert_eq!(verifier.first_mismatch_tick(), Some(100));
    }

    #[test]
    fn verify_scheduler_jobs() {
        let mut recorder = make_recorder();
        recorder.record_scheduler_job(50, (5, 10, 15), 100, "Immediate");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(verifier.verify_scheduler_job(50, (5, 10, 15), 100, "Immediate"));
        assert!(verifier.is_valid());
    }

    #[test]
    fn verify_environment_hints() {
        let mut recorder = make_recorder();
        recorder.record_environment_hint(75, (1, 2, 3), "structural");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(verifier.verify_environment_hint(75, (1, 2, 3), "structural"));
        assert!(verifier.is_valid());
    }

    #[test]
    fn verify_season_transition() {
        let mut recorder = make_recorder();
        recorder.record_season_transition(1000, 0, 1);
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        assert!(verifier.verify_season_transition(1000, 0, 1));
        assert!(verifier.is_valid());
    }

    #[test]
    fn reset_verifier() {
        let mut recorder = make_recorder();
        recorder.record_step_checksum(100, 0xAAAA_u32);
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);
        verifier.verify_step_checksum(100, 0xFFFF_u32);
        assert!(!verifier.is_valid());

        verifier.reset();
        assert!(verifier.is_valid());
        assert_eq!(verifier.cursor(), 0);
        assert_eq!(verifier.remaining(), 1);
    }

    #[test]
    fn mismatch_display() {
        let mismatch = Mismatch::new(
            100,
            MismatchKind::ChecksumMismatch,
            "0xAAAA".to_string(),
            "0xBBBB".to_string(),
        )
        .with_context("during physics step");

        let display = format!("{mismatch}");
        assert!(display.contains("tick 100"));
        assert!(display.contains("ChecksumMismatch"));
        assert!(display.contains("0xAAAA"));
        assert!(display.contains("0xBBBB"));
        assert!(display.contains("during physics step"));
    }

    #[test]
    fn serde_round_trip_mismatch() {
        let mismatch = Mismatch::checksum(
            100,
            StepChecksum::from_raw(0xAAAA),
            StepChecksum::from_raw(0xBBBB),
        )
        .with_context("test");

        let json = serde_json::to_string(&mismatch).unwrap();
        let recovered: Mismatch = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, mismatch);
    }

    #[test]
    fn deterministic_verification_order() {
        let mut recorder = make_recorder();
        recorder.record_world_event_start(100, 1, "A");
        recorder.record_world_event_start(100, 2, "B");
        recorder.record_step_checksum(100, 0x1234_u32);
        recorder.record_world_event_start(200, 3, "C");
        recorder.finalize();

        let mut verifier = ReplayVerifier::from_recorder(&recorder);

        assert!(verifier.verify_world_event_start(100, 1, "A"));
        assert!(verifier.verify_world_event_start(100, 2, "B"));
        assert!(verifier.verify_step_checksum(100, 0x1234_u32));
        assert!(verifier.verify_world_event_start(200, 3, "C"));

        verifier.finish_verification();
        assert!(verifier.is_valid());
    }
}

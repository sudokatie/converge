//! Region event journal for postmortem debugging and simulation recovery.

use std::collections::HashMap;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::{
    EventCategory, EventKind, EventPayload, EventRecord, RecoverySummary, RegionSummary, Severity,
};
use crate::replay::ChecksumBuilder;

/// Query filter for journal entries.
#[derive(Clone, Debug, Default)]
pub struct JournalQuery {
    /// Filter by tick range (inclusive).
    pub tick_range: Option<(u64, u64)>,
    /// Filter by chunk position.
    pub chunk_pos: Option<ChunkPos>,
    /// Filter by event category.
    pub category: Option<EventCategory>,
    /// Filter by event kind.
    pub kind: Option<EventKind>,
    /// Filter by minimum severity.
    pub min_severity: Option<Severity>,
    /// Filter by tag (any match).
    pub tag: Option<String>,
    /// Filter by source ID.
    pub source_id: Option<u32>,
}

impl JournalQuery {
    /// Create a new empty query (matches all).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tick_range: None,
            chunk_pos: None,
            category: None,
            kind: None,
            min_severity: None,
            tag: None,
            source_id: None,
        }
    }

    /// Filter by tick range.
    #[must_use]
    pub const fn with_tick_range(mut self, start: u64, end: u64) -> Self {
        self.tick_range = Some((start, end));
        self
    }

    /// Filter by chunk position.
    #[must_use]
    pub const fn with_chunk_pos(mut self, chunk_pos: ChunkPos) -> Self {
        self.chunk_pos = Some(chunk_pos);
        self
    }

    /// Filter by category.
    #[must_use]
    pub const fn with_category(mut self, category: EventCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Filter by kind.
    #[must_use]
    pub const fn with_kind(mut self, kind: EventKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Filter by minimum severity.
    #[must_use]
    pub const fn with_min_severity(mut self, severity: Severity) -> Self {
        self.min_severity = Some(severity);
        self
    }

    /// Filter by tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Filter by source ID.
    #[must_use]
    pub const fn with_source_id(mut self, source_id: u32) -> Self {
        self.source_id = Some(source_id);
        self
    }

    /// Check if a record matches this query.
    #[must_use]
    pub fn matches(&self, record: &EventRecord) -> bool {
        if let Some((start, end)) = self.tick_range
            && (record.tick() < start || record.tick() > end)
        {
            return false;
        }
        if let Some(pos) = self.chunk_pos
            && record.chunk_pos() != pos
        {
            return false;
        }
        if let Some(cat) = self.category
            && record.kind().category() != cat
        {
            return false;
        }
        if let Some(kind) = self.kind
            && record.kind() != kind
        {
            return false;
        }
        if let Some(sev) = self.min_severity
            && !record.severity().is_at_least(sev)
        {
            return false;
        }
        if let Some(ref tag) = self.tag
            && !record.has_tag(tag)
        {
            return false;
        }
        if let Some(source) = self.source_id
            && record.source_id() != source
        {
            return false;
        }
        true
    }
}

/// Append-only journal for region events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionJournal {
    /// Journal identifier.
    identifier: String,
    /// All event records, sorted by (tick, sequence).
    records: Vec<EventRecord>,
    /// Next sequence number.
    next_sequence: u64,
    /// Tick range covered.
    tick_range: (u64, u64),
    /// Total events appended (including truncated).
    total_appended: u64,
}

impl RegionJournal {
    /// Create a new empty journal.
    #[must_use]
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            records: Vec::new(),
            next_sequence: 0,
            tick_range: (u64::MAX, 0),
            total_appended: 0,
        }
    }

    /// Get the journal identifier.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Get the number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if journal is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get the tick range covered by current records.
    #[must_use]
    pub fn tick_range(&self) -> Option<(u64, u64)> {
        if self.records.is_empty() {
            None
        } else {
            Some(self.tick_range)
        }
    }

    /// Get total events ever appended (including truncated).
    #[must_use]
    pub fn total_appended(&self) -> u64 {
        self.total_appended
    }

    /// Append a new event record.
    pub fn append(&mut self, mut record: EventRecord) {
        record.set_sequence(self.next_sequence);
        self.next_sequence += 1;
        self.total_appended += 1;

        let tick = record.tick();
        self.tick_range.0 = self.tick_range.0.min(tick);
        self.tick_range.1 = self.tick_range.1.max(tick);

        self.records.push(record);
    }

    /// Append a simple event with minimal data.
    pub fn append_simple(
        &mut self,
        tick: u64,
        chunk_pos: ChunkPos,
        kind: EventKind,
        severity: Severity,
    ) {
        let record = EventRecord::new(tick, 0, chunk_pos, kind).with_severity(severity);
        self.append(record);
    }

    /// Append an event with payload.
    pub fn append_with_payload(
        &mut self,
        tick: u64,
        chunk_pos: ChunkPos,
        kind: EventKind,
        severity: Severity,
        payload: EventPayload,
    ) {
        let record = EventRecord::new(tick, 0, chunk_pos, kind)
            .with_severity(severity)
            .with_payload(payload);
        self.append(record);
    }

    /// Get all records (sorted by tick, sequence).
    #[must_use]
    pub fn records(&self) -> &[EventRecord] {
        &self.records
    }

    /// Iterate over records matching a query.
    pub fn query<'a>(&'a self, query: &'a JournalQuery) -> impl Iterator<Item = &'a EventRecord> {
        self.records.iter().filter(move |r| query.matches(r))
    }

    /// Get records in a tick range.
    pub fn records_in_range(&self, start: u64, end: u64) -> impl Iterator<Item = &EventRecord> {
        self.records
            .iter()
            .filter(move |r| r.tick() >= start && r.tick() <= end)
    }

    /// Get records for a specific chunk.
    pub fn records_for_chunk(&self, chunk_pos: ChunkPos) -> impl Iterator<Item = &EventRecord> {
        self.records
            .iter()
            .filter(move |r| r.chunk_pos() == chunk_pos)
    }

    /// Get records of a specific category.
    pub fn records_by_category(
        &self,
        category: EventCategory,
    ) -> impl Iterator<Item = &EventRecord> {
        self.records
            .iter()
            .filter(move |r| r.kind().category() == category)
    }

    /// Get records at or above a severity threshold.
    pub fn records_by_severity(&self, min: Severity) -> impl Iterator<Item = &EventRecord> {
        self.records
            .iter()
            .filter(move |r| r.severity().is_at_least(min))
    }

    /// Truncate records older than a tick threshold.
    pub fn truncate_before(&mut self, tick: u64) -> usize {
        let original_len = self.records.len();
        self.records.retain(|r| r.tick() >= tick);
        let removed = original_len - self.records.len();

        if self.records.is_empty() {
            self.tick_range = (u64::MAX, 0);
        } else {
            self.tick_range.0 = self.records.first().map_or(u64::MAX, EventRecord::tick);
        }

        removed
    }

    /// Retain only the most recent N records.
    pub fn retain_recent(&mut self, max_records: usize) {
        if self.records.len() > max_records {
            let drain_count = self.records.len() - max_records;
            self.records.drain(0..drain_count);

            if !self.records.is_empty() {
                self.tick_range.0 = self.records.first().map_or(u64::MAX, EventRecord::tick);
            }
        }
    }

    /// Retain only records at or above a severity threshold.
    pub fn retain_severity(&mut self, min: Severity) -> usize {
        let original_len = self.records.len();
        self.records.retain(|r| r.severity().is_at_least(min));
        let removed = original_len - self.records.len();
        self.update_tick_range();
        removed
    }

    /// Compact journal by merging consecutive similar events.
    pub fn compact(&mut self) -> usize {
        if self.records.len() < 2 {
            return 0;
        }

        let mut compacted = Vec::with_capacity(self.records.len());
        let mut prev: Option<EventRecord> = None;
        let mut merged_count = 0u64;
        let mut total_removed = 0usize;

        for record in self.records.drain(..) {
            match prev.take() {
                Some(p) if Self::can_merge(&p, &record) => {
                    merged_count += 1;
                    total_removed += 1;
                    prev = Some(p);
                    continue;
                }
                Some(mut to_push) => {
                    if merged_count > 0 {
                        let mut payload = to_push.payload().clone();
                        payload.secondary = merged_count + 1;
                        to_push = to_push.with_payload(payload);
                    }
                    compacted.push(to_push);
                    merged_count = 0;
                }
                None => {}
            }
            prev = Some(record);
        }

        if let Some(mut last) = prev {
            if merged_count > 0 {
                let mut payload = last.payload().clone();
                payload.secondary = merged_count + 1;
                last = last.with_payload(payload);
            }
            compacted.push(last);
        }

        self.records = compacted;
        self.update_tick_range();
        total_removed
    }

    /// Merge another journal into this one.
    pub fn merge(&mut self, other: &RegionJournal) {
        for record in &other.records {
            let mut cloned = record.clone();
            cloned.set_sequence(self.next_sequence);
            self.next_sequence += 1;
            self.total_appended += 1;

            let tick = cloned.tick();
            self.tick_range.0 = self.tick_range.0.min(tick);
            self.tick_range.1 = self.tick_range.1.max(tick);

            self.records.push(cloned);
        }
        self.records.sort();
    }

    /// Generate a deterministic checksum for a tick range.
    #[must_use]
    pub fn checksum_range(&self, start: u64, end: u64) -> u32 {
        let mut builder = ChecksumBuilder::new();
        builder.feed_str(&self.identifier);
        builder.feed_u64(start);
        builder.feed_u64(end);

        for record in self.records_in_range(start, end) {
            builder.feed_u64(record.tick());
            builder.feed_u64(record.sequence());
            builder.feed_position(
                record.chunk_pos().x(),
                record.chunk_pos().y(),
                record.chunk_pos().z(),
            );
            builder.feed_u32(record.severity() as u32);
            builder.feed_u32(record.kind() as u32);
            builder.feed_u32(record.source_id());
            builder.feed_u64(record.payload().primary);
            builder.feed_u64(record.payload().secondary);
            for &v in &record.payload().tertiary {
                builder.feed_i32(v);
            }
            builder.feed_str(&record.payload().label);
        }

        builder.build().value()
    }

    /// Generate a deterministic checksum for the entire journal.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        if self.records.is_empty() {
            return 0;
        }
        self.checksum_range(self.tick_range.0, self.tick_range.1)
    }

    /// Generate a recovery summary for a tick range.
    #[must_use]
    pub fn recovery_summary(&self, start: u64, end: u64) -> RecoverySummary {
        let mut summary = RecoverySummary::empty((start, end));
        let mut region_map: HashMap<ChunkPos, RegionSummary> = HashMap::new();
        let mut last_checkpoint = None;

        for record in self.records_in_range(start, end) {
            summary.total_events += 1;
            let chunk_pos = record.chunk_pos();

            let region = region_map
                .entry(chunk_pos)
                .or_insert_with(|| RegionSummary::empty(chunk_pos));

            region.total_events += 1;
            region.tick_range.0 = region.tick_range.0.min(record.tick());
            region.tick_range.1 = region.tick_range.1.max(record.tick());

            let cat_idx = record.kind().category().as_index();
            region.by_category[cat_idx].add(record.severity());

            if record.severity() > region.max_severity {
                region.max_severity = record.severity();
            }

            if region.recent_kinds.len() < 8 {
                region.recent_kinds.push(record.kind());
            } else {
                region.recent_kinds.remove(0);
                region.recent_kinds.push(record.kind());
            }

            for tag in record.tags() {
                if let Some(entry) = region.frequent_tags.iter_mut().find(|(t, _)| t == tag) {
                    entry.1 += 1;
                } else {
                    region.frequent_tags.push((tag.clone(), 1));
                }
            }

            match record.kind() {
                EventKind::Checkpoint => {
                    last_checkpoint = Some(record.tick());
                }
                EventKind::Rollback => {
                    summary.rollback_count += 1;
                }
                EventKind::SyncMismatch => {
                    summary.mismatch_count += 1;
                }
                _ => {}
            }

            if record.severity().is_at_least(Severity::Error)
                && !summary.error_regions.contains(&chunk_pos)
            {
                summary.error_regions.push(chunk_pos);
            }
        }

        summary.regions_affected = region_map.len() as u64;
        summary.last_checkpoint = last_checkpoint;

        let mut summaries: Vec<_> = region_map.into_values().collect();
        summaries.sort_by(|a, b| {
            let a_tuple = (a.chunk_pos.x(), a.chunk_pos.y(), a.chunk_pos.z());
            let b_tuple = (b.chunk_pos.x(), b.chunk_pos.y(), b.chunk_pos.z());
            a_tuple.cmp(&b_tuple)
        });

        for s in &mut summaries {
            s.frequent_tags.sort_by(|a, b| b.1.cmp(&a.1));
            s.frequent_tags.truncate(10);
        }

        summary.region_summaries = summaries;
        summary
    }

    /// Clear all records.
    pub fn clear(&mut self) {
        self.records.clear();
        self.tick_range = (u64::MAX, 0);
    }

    fn can_merge(a: &EventRecord, b: &EventRecord) -> bool {
        a.tick() == b.tick()
            && a.chunk_pos() == b.chunk_pos()
            && a.kind() == b.kind()
            && a.severity() == b.severity()
            && a.source_id() == b.source_id()
    }

    fn update_tick_range(&mut self) {
        if self.records.is_empty() {
            self.tick_range = (u64::MAX, 0);
        } else {
            self.tick_range.0 = self.records.first().map_or(u64::MAX, EventRecord::tick);
            self.tick_range.1 = self.records.last().map_or(0, EventRecord::tick);
        }
    }
}

impl Default for RegionJournal {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use engine_core::coords::LocalPos;

    use super::*;

    fn make_record(tick: u64, x: i32, kind: EventKind) -> EventRecord {
        EventRecord::new(tick, 0, ChunkPos::new(x, 0, 0), kind)
    }

    #[test]
    fn journal_new() {
        let journal = RegionJournal::new("test");
        assert_eq!(journal.identifier(), "test");
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
        assert!(journal.tick_range().is_none());
    }

    #[test]
    fn journal_append() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(200, 1, EventKind::ChunkLoaded));

        assert_eq!(journal.len(), 2);
        assert_eq!(journal.tick_range(), Some((100, 200)));
        assert_eq!(journal.records()[0].sequence(), 0);
        assert_eq!(journal.records()[1].sequence(), 1);
    }

    #[test]
    fn journal_append_simple() {
        let mut journal = RegionJournal::new("test");
        journal.append_simple(
            50,
            ChunkPos::new(0, 0, 0),
            EventKind::HazardSpawn,
            Severity::Warning,
        );

        assert_eq!(journal.len(), 1);
        assert_eq!(journal.records()[0].severity(), Severity::Warning);
    }

    #[test]
    fn journal_query_tick_range() {
        let mut journal = RegionJournal::new("test");
        for tick in [50, 100, 150, 200] {
            journal.append(make_record(tick, 0, EventKind::ChunkLoaded));
        }

        let query = JournalQuery::new().with_tick_range(75, 175);
        let results: Vec<_> = journal.query(&query).collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].tick(), 100);
        assert_eq!(results[1].tick(), 150);
    }

    #[test]
    fn journal_query_chunk_pos() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(100, 1, EventKind::ChunkLoaded));
        journal.append(make_record(100, 0, EventKind::ChunkUnloaded));

        let query = JournalQuery::new().with_chunk_pos(ChunkPos::new(0, 0, 0));
        let results: Vec<_> = journal.query(&query).collect();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn journal_query_category() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(100, 0, EventKind::HazardSpawn));
        journal.append(make_record(100, 0, EventKind::JobDispatched));

        let query = JournalQuery::new().with_category(EventCategory::Environment);
        let results: Vec<_> = journal.query(&query).collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind(), EventKind::HazardSpawn);
    }

    #[test]
    fn journal_query_severity() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded).with_severity(Severity::Info));
        journal
            .append(make_record(100, 0, EventKind::HazardSpawn).with_severity(Severity::Warning));
        journal.append(make_record(100, 0, EventKind::SyncMismatch).with_severity(Severity::Error));

        let query = JournalQuery::new().with_min_severity(Severity::Warning);
        let results: Vec<_> = journal.query(&query).collect();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn journal_query_tag() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::HazardSpawn).with_tag("fire"));
        journal.append(make_record(100, 0, EventKind::HazardSpawn).with_tag("flood"));
        journal.append(make_record(100, 0, EventKind::HazardSpawn).with_tag("fire"));

        let query = JournalQuery::new().with_tag("fire");
        let results: Vec<_> = journal.query(&query).collect();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn journal_query_source_id() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded).with_source_id(1));
        journal.append(make_record(100, 0, EventKind::ChunkLoaded).with_source_id(2));

        let query = JournalQuery::new().with_source_id(1);
        let results: Vec<_> = journal.query(&query).collect();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn journal_truncate_before() {
        let mut journal = RegionJournal::new("test");
        for tick in [50, 100, 150, 200] {
            journal.append(make_record(tick, 0, EventKind::ChunkLoaded));
        }

        let removed = journal.truncate_before(125);

        assert_eq!(removed, 2);
        assert_eq!(journal.len(), 2);
        assert_eq!(journal.tick_range(), Some((150, 200)));
    }

    #[test]
    fn journal_retain_recent() {
        let mut journal = RegionJournal::new("test");
        for tick in 0..100 {
            journal.append(make_record(tick, 0, EventKind::ChunkLoaded));
        }

        journal.retain_recent(10);

        assert_eq!(journal.len(), 10);
        assert_eq!(journal.records()[0].tick(), 90);
    }

    #[test]
    fn journal_retain_severity() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded).with_severity(Severity::Trace));
        journal
            .append(make_record(100, 0, EventKind::HazardSpawn).with_severity(Severity::Warning));
        journal.append(make_record(100, 0, EventKind::SyncMismatch).with_severity(Severity::Error));

        let removed = journal.retain_severity(Severity::Warning);

        assert_eq!(removed, 1);
        assert_eq!(journal.len(), 2);
    }

    #[test]
    fn journal_compact() {
        let mut journal = RegionJournal::new("test");
        for _ in 0..5 {
            journal.append(make_record(100, 0, EventKind::BlockModified));
        }
        journal.append(make_record(100, 0, EventKind::HazardSpawn));
        for _ in 0..3 {
            journal.append(make_record(100, 0, EventKind::BlockModified));
        }

        let removed = journal.compact();

        assert_eq!(removed, 6);
        assert_eq!(journal.len(), 3);
        assert_eq!(journal.records()[0].payload().secondary, 5);
        assert_eq!(journal.records()[2].payload().secondary, 3);
    }

    #[test]
    fn journal_merge() {
        let mut journal1 = RegionJournal::new("j1");
        journal1.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal1.append(make_record(300, 0, EventKind::ChunkLoaded));

        let mut journal2 = RegionJournal::new("j2");
        journal2.append(make_record(200, 0, EventKind::ChunkLoaded));

        journal1.merge(&journal2);

        assert_eq!(journal1.len(), 3);
        assert_eq!(journal1.records()[0].tick(), 100);
        assert_eq!(journal1.records()[1].tick(), 200);
        assert_eq!(journal1.records()[2].tick(), 300);
    }

    #[test]
    fn journal_checksum_deterministic() {
        let mut journal1 = RegionJournal::new("test");
        let mut journal2 = RegionJournal::new("test");

        for j in [&mut journal1, &mut journal2] {
            j.append(make_record(100, 0, EventKind::ChunkLoaded));
            j.append(
                make_record(200, 1, EventKind::HazardSpawn)
                    .with_payload(EventPayload::empty().primary(42).label("fire")),
            );
        }

        assert_eq!(journal1.checksum(), journal2.checksum());
        assert_eq!(
            journal1.checksum_range(100, 200),
            journal2.checksum_range(100, 200)
        );
    }

    #[test]
    fn journal_checksum_different() {
        let mut journal1 = RegionJournal::new("test");
        let mut journal2 = RegionJournal::new("test");

        journal1.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal2.append(make_record(100, 1, EventKind::ChunkLoaded));

        assert_ne!(journal1.checksum(), journal2.checksum());
    }

    #[test]
    fn journal_recovery_summary() {
        let mut journal = RegionJournal::new("test");

        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(150, 0, EventKind::Checkpoint));
        journal
            .append(make_record(200, 0, EventKind::HazardSpawn).with_severity(Severity::Warning));
        journal.append(make_record(250, 1, EventKind::SyncMismatch).with_severity(Severity::Error));
        journal.append(make_record(300, 1, EventKind::Rollback));

        let summary = journal.recovery_summary(0, 500);

        assert_eq!(summary.total_events, 5);
        assert_eq!(summary.regions_affected, 2);
        assert_eq!(summary.last_checkpoint, Some(150));
        assert_eq!(summary.mismatch_count, 1);
        assert_eq!(summary.rollback_count, 1);
        assert_eq!(summary.error_regions.len(), 1);
        assert!(summary.needs_recovery());
    }

    #[test]
    fn journal_recovery_summary_region_details() {
        let mut journal = RegionJournal::new("test");

        journal.append(
            make_record(100, 0, EventKind::HazardSpawn)
                .with_severity(Severity::Warning)
                .with_tag("fire"),
        );
        journal.append(
            make_record(100, 0, EventKind::HazardSpread)
                .with_severity(Severity::Warning)
                .with_tag("fire"),
        );

        let summary = journal.recovery_summary(0, 200);
        let region = summary.region_summary(ChunkPos::new(0, 0, 0)).unwrap();

        assert_eq!(region.total_events, 2);
        assert_eq!(region.max_severity, Severity::Warning);
        assert!(region.has_warnings());
        assert!(!region.has_errors());
        assert_eq!(region.recent_kinds.len(), 2);
        assert!(!region.frequent_tags.is_empty());
    }

    #[test]
    fn journal_iteration_stability() {
        let mut journal = RegionJournal::new("test");

        journal.append(make_record(300, 0, EventKind::ChunkLoaded));
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(200, 0, EventKind::ChunkLoaded));

        let ticks: Vec<_> = journal.records().iter().map(EventRecord::tick).collect();
        assert_eq!(ticks, vec![300, 100, 200]);
    }

    #[test]
    fn journal_records_for_chunk() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(100, 5, EventKind::ChunkLoaded));
        journal.append(make_record(100, 0, EventKind::ChunkUnloaded));

        let results: Vec<_> = journal.records_for_chunk(ChunkPos::new(0, 0, 0)).collect();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn journal_records_by_category() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.append(make_record(100, 0, EventKind::HazardSpawn));

        let results: Vec<_> = journal
            .records_by_category(EventCategory::ChunkMutation)
            .collect();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn query_combined_filters() {
        let query = JournalQuery::new()
            .with_tick_range(0, 1000)
            .with_chunk_pos(ChunkPos::new(1, 0, 0))
            .with_min_severity(Severity::Warning);

        let record = make_record(500, 1, EventKind::HazardSpawn).with_severity(Severity::Error);
        assert!(query.matches(&record));

        let record2 = make_record(500, 0, EventKind::HazardSpawn);
        assert!(!query.matches(&record2));
    }

    #[test]
    fn serde_round_trip_journal() {
        let mut journal = RegionJournal::new("test_journal");
        journal.append(
            make_record(100, 0, EventKind::HazardSpawn)
                .with_severity(Severity::Warning)
                .with_tag("fire")
                .with_affected_position(LocalPos::new(1, 2, 3))
                .with_payload(EventPayload::with_primary(999)),
        );
        journal.append(make_record(200, 1, EventKind::ChunkLoaded));

        let json = serde_json::to_string(&journal).unwrap();
        let recovered: RegionJournal = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.identifier(), journal.identifier());
        assert_eq!(recovered.len(), journal.len());
        assert_eq!(recovered.checksum(), journal.checksum());
    }

    #[test]
    fn journal_clear() {
        let mut journal = RegionJournal::new("test");
        journal.append(make_record(100, 0, EventKind::ChunkLoaded));
        journal.clear();

        assert!(journal.is_empty());
        assert!(journal.tick_range().is_none());
    }

    #[test]
    fn journal_total_appended_tracks_truncated() {
        let mut journal = RegionJournal::new("test");
        for tick in 0..10 {
            journal.append(make_record(tick, 0, EventKind::ChunkLoaded));
        }

        assert_eq!(journal.total_appended(), 10);

        journal.truncate_before(5);

        assert_eq!(journal.len(), 5);
        assert_eq!(journal.total_appended(), 10);
    }
}

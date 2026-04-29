//! Chunk mutation journaling for late-join sync and rollback reconciliation.
//!
//! Provides deterministic ordered mutation records for block changes with
//! support for querying, compaction, checksum computation, late-join snapshots,
//! and chunk rollback operations.

use std::collections::HashMap;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::chunk::{BlockId, Chunk};
use crate::persistence::{ChunkDelta, DeltaIndex};

/// Monotonically increasing sequence number within a tick.
pub type Sequence = u64;

/// Source system that originated a mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MutationSource {
    /// Player action (building, mining).
    Player = 0,
    /// Environmental simulation (fluid, structural).
    Environment = 1,
    /// Entity behavior (mob, NPC).
    Entity = 2,
    /// Terrain generation.
    Generation = 3,
    /// Network sync from authoritative source.
    NetworkSync = 4,
    /// Rollback/reconciliation operation.
    Rollback = 5,
    /// Custom/scripted mutation.
    Custom = 6,
}

impl MutationSource {
    /// Get the display name for this source.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Environment => "environment",
            Self::Entity => "entity",
            Self::Generation => "generation",
            Self::NetworkSync => "network_sync",
            Self::Rollback => "rollback",
            Self::Custom => "custom",
        }
    }
}

/// Reason category for a mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MutationReason {
    /// Block placement.
    Place = 0,
    /// Block destruction/mining.
    Destroy = 1,
    /// Block replacement/swap.
    Replace = 2,
    /// Fluid flow.
    FluidFlow = 3,
    /// Structural collapse.
    Collapse = 4,
    /// Environmental decay.
    Decay = 5,
    /// Growth/spread.
    Growth = 6,
    /// Explosion damage.
    Explosion = 7,
    /// Chunk generation.
    Generate = 8,
    /// State correction.
    Correction = 9,
    /// Batch delta application.
    DeltaApply = 10,
}

impl MutationReason {
    /// Get the display name for this reason.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Destroy => "destroy",
            Self::Replace => "replace",
            Self::FluidFlow => "fluid_flow",
            Self::Collapse => "collapse",
            Self::Decay => "decay",
            Self::Growth => "growth",
            Self::Explosion => "explosion",
            Self::Generate => "generate",
            Self::Correction => "correction",
            Self::DeltaApply => "delta_apply",
        }
    }
}

/// A single block mutation record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationRecord {
    /// Simulation tick when mutation occurred.
    pub tick: u64,
    /// Sequence number within the tick for ordering.
    pub sequence: Sequence,
    /// Chunk position.
    pub chunk_pos: ChunkPos,
    /// Local position within chunk (packed as `DeltaIndex`).
    pub local_index: DeltaIndex,
    /// Block ID before mutation.
    pub old_block: BlockId,
    /// Block ID after mutation.
    pub new_block: BlockId,
    /// Source system.
    pub source: MutationSource,
    /// Reason category.
    pub reason: MutationReason,
    /// Optional source entity/player ID.
    pub source_id: Option<u32>,
    /// Optional tags for filtering.
    pub tags: Vec<String>,
}

impl MutationRecord {
    /// Create a new mutation record.
    #[expect(
        clippy::too_many_arguments,
        reason = "mutation record requires all context"
    )]
    #[must_use]
    pub fn new(
        tick: u64,
        sequence: Sequence,
        chunk_pos: ChunkPos,
        local_pos: LocalPos,
        old_block: BlockId,
        new_block: BlockId,
        source: MutationSource,
        reason: MutationReason,
    ) -> Self {
        Self {
            tick,
            sequence,
            chunk_pos,
            local_index: DeltaIndex::from_local_pos(local_pos),
            old_block,
            new_block,
            source,
            reason,
            source_id: None,
            tags: Vec::new(),
        }
    }

    /// Add a source entity/player ID.
    #[must_use]
    pub fn with_source_id(mut self, id: u32) -> Self {
        self.source_id = Some(id);
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Get the local position.
    #[must_use]
    pub fn local_pos(&self) -> LocalPos {
        self.local_index.to_local_pos()
    }

    /// Check if this record has a specific tag.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Check if this is a no-op (old == new).
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.old_block == self.new_block
    }

    /// Create a reversed record for rollback.
    #[must_use]
    pub fn reversed(&self, new_tick: u64, new_sequence: Sequence) -> Self {
        Self {
            tick: new_tick,
            sequence: new_sequence,
            chunk_pos: self.chunk_pos,
            local_index: self.local_index,
            old_block: self.new_block,
            new_block: self.old_block,
            source: MutationSource::Rollback,
            reason: MutationReason::Correction,
            source_id: None,
            tags: vec!["rollback".to_string()],
        }
    }

    /// Ordering key for deterministic sorting (tick, sequence, chunk, index).
    #[must_use]
    fn sort_key(&self) -> (u64, Sequence, i32, i32, i32, u16) {
        (
            self.tick,
            self.sequence,
            self.chunk_pos.x(),
            self.chunk_pos.y(),
            self.chunk_pos.z(),
            self.local_index.raw(),
        )
    }
}

impl PartialOrd for MutationRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MutationRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

/// Query builder for filtering mutation records.
#[derive(Clone, Debug, Default)]
pub struct MutationQuery {
    tick_min: Option<u64>,
    tick_max: Option<u64>,
    chunk_pos: Option<ChunkPos>,
    sources: Option<Vec<MutationSource>>,
    reasons: Option<Vec<MutationReason>>,
    source_id: Option<u32>,
    tag: Option<String>,
    limit: Option<usize>,
}

impl MutationQuery {
    /// Create a new empty query (matches all).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by minimum tick (inclusive).
    #[must_use]
    pub fn tick_min(mut self, tick: u64) -> Self {
        self.tick_min = Some(tick);
        self
    }

    /// Filter by maximum tick (inclusive).
    #[must_use]
    pub fn tick_max(mut self, tick: u64) -> Self {
        self.tick_max = Some(tick);
        self
    }

    /// Filter by tick range (inclusive).
    #[must_use]
    pub fn tick_range(mut self, min: u64, max: u64) -> Self {
        self.tick_min = Some(min);
        self.tick_max = Some(max);
        self
    }

    /// Filter by chunk position.
    #[must_use]
    pub fn chunk(mut self, pos: ChunkPos) -> Self {
        self.chunk_pos = Some(pos);
        self
    }

    /// Filter by mutation sources.
    #[must_use]
    pub fn sources(mut self, sources: Vec<MutationSource>) -> Self {
        self.sources = Some(sources);
        self
    }

    /// Filter by single source.
    #[must_use]
    pub fn source(mut self, source: MutationSource) -> Self {
        self.sources = Some(vec![source]);
        self
    }

    /// Filter by mutation reasons.
    #[must_use]
    pub fn reasons(mut self, reasons: Vec<MutationReason>) -> Self {
        self.reasons = Some(reasons);
        self
    }

    /// Filter by single reason.
    #[must_use]
    pub fn reason(mut self, reason: MutationReason) -> Self {
        self.reasons = Some(vec![reason]);
        self
    }

    /// Filter by source entity/player ID.
    #[must_use]
    pub fn source_id(mut self, id: u32) -> Self {
        self.source_id = Some(id);
        self
    }

    /// Filter by tag.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Limit result count.
    #[must_use]
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Check if a record matches this query.
    #[must_use]
    pub fn matches(&self, record: &MutationRecord) -> bool {
        if self.tick_min.is_some_and(|min| record.tick < min) {
            return false;
        }
        if self.tick_max.is_some_and(|max| record.tick > max) {
            return false;
        }
        if self.chunk_pos.is_some_and(|pos| record.chunk_pos != pos) {
            return false;
        }
        if self
            .sources
            .as_ref()
            .is_some_and(|s| !s.contains(&record.source))
        {
            return false;
        }
        if self
            .reasons
            .as_ref()
            .is_some_and(|r| !r.contains(&record.reason))
        {
            return false;
        }
        if self
            .source_id
            .is_some_and(|id| record.source_id != Some(id))
        {
            return false;
        }
        if self.tag.as_ref().is_some_and(|t| !record.has_tag(t)) {
            return false;
        }
        true
    }
}

/// A snapshot of chunk state for late-join sync.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalSnapshot {
    /// Base tick for the snapshot.
    pub base_tick: u64,
    /// End tick (exclusive) for included mutations.
    pub end_tick: u64,
    /// Chunk deltas representing state at `base_tick`.
    pub chunk_deltas: HashMap<ChunkPos, ChunkDelta>,
    /// Mutations since `base_tick` for replay.
    pub pending_mutations: Vec<MutationRecord>,
}

impl JournalSnapshot {
    /// Create an empty snapshot at a given tick.
    #[must_use]
    pub fn empty(tick: u64) -> Self {
        Self {
            base_tick: tick,
            end_tick: tick,
            chunk_deltas: HashMap::new(),
            pending_mutations: Vec::new(),
        }
    }

    /// Number of chunks in the snapshot.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunk_deltas.len()
    }

    /// Number of pending mutations.
    #[must_use]
    pub fn mutation_count(&self) -> usize {
        self.pending_mutations.len()
    }
}

/// Summary statistics for a mutation journal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalStats {
    /// Total number of records.
    pub record_count: usize,
    /// Number of unique chunks affected.
    pub chunk_count: usize,
    /// Earliest tick in journal.
    pub min_tick: Option<u64>,
    /// Latest tick in journal.
    pub max_tick: Option<u64>,
    /// Count by source type.
    pub by_source: [usize; 7],
    /// Count by reason type.
    pub by_reason: [usize; 11],
}

/// Chunk mutation journal for tracking block changes.
///
/// Provides append-only mutation tracking with support for:
/// - Deterministic ordering by (tick, sequence, chunk, `local_pos`)
/// - Flexible querying by tick range, chunk, source, reason, tag
/// - Compaction and retention policies
/// - Checksum computation for verification
/// - Late-join snapshot generation
/// - Chunk rollback through recorded mutations
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MutationJournal {
    records: Vec<MutationRecord>,
    next_sequence: Sequence,
    current_tick: u64,
}

impl MutationJournal {
    /// Create a new empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a journal starting at a specific tick.
    #[must_use]
    pub fn at_tick(tick: u64) -> Self {
        Self {
            records: Vec::new(),
            next_sequence: 0,
            current_tick: tick,
        }
    }

    /// Advance to a new tick, resetting sequence counter.
    pub fn advance_tick(&mut self, tick: u64) {
        debug_assert!(tick >= self.current_tick, "tick must not go backwards");
        if tick > self.current_tick {
            self.current_tick = tick;
            self.next_sequence = 0;
        }
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the next sequence number (without advancing).
    #[must_use]
    pub fn next_sequence(&self) -> Sequence {
        self.next_sequence
    }

    /// Append a single block change and return a reference to it.
    #[expect(
        clippy::missing_panics_doc,
        reason = "record is always pushed before unwrap"
    )]
    pub fn append(
        &mut self,
        chunk_pos: ChunkPos,
        local_pos: LocalPos,
        old_block: BlockId,
        new_block: BlockId,
        source: MutationSource,
        reason: MutationReason,
    ) -> &MutationRecord {
        let record = MutationRecord::new(
            self.current_tick,
            self.next_sequence,
            chunk_pos,
            local_pos,
            old_block,
            new_block,
            source,
            reason,
        );
        self.next_sequence += 1;
        self.records.push(record);
        self.records.last().unwrap()
    }

    /// Append a pre-built record, updating tick/sequence.
    #[expect(
        clippy::missing_panics_doc,
        reason = "record is always pushed before unwrap"
    )]
    pub fn append_record(&mut self, mut record: MutationRecord) -> &MutationRecord {
        record.tick = self.current_tick;
        record.sequence = self.next_sequence;
        self.next_sequence += 1;
        self.records.push(record);
        self.records.last().unwrap()
    }

    /// Append multiple block changes from a delta.
    pub fn append_delta(
        &mut self,
        chunk_pos: ChunkPos,
        delta: &ChunkDelta,
        base: &Chunk,
        source: MutationSource,
    ) {
        for (local_pos, new_block) in delta.iter() {
            let old_block = base.get(local_pos);
            if old_block != new_block {
                self.append(
                    chunk_pos,
                    local_pos,
                    old_block,
                    new_block,
                    source,
                    MutationReason::DeltaApply,
                );
            }
        }
    }

    /// Get the number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if the journal is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get all records.
    #[must_use]
    pub fn records(&self) -> &[MutationRecord] {
        &self.records
    }

    /// Query records matching criteria.
    pub fn query(&self, q: &MutationQuery) -> Vec<&MutationRecord> {
        let iter = self.records.iter().filter(|r| q.matches(r));
        if let Some(limit) = q.limit {
            iter.take(limit).collect()
        } else {
            iter.collect()
        }
    }

    /// Get records in a tick range (inclusive).
    pub fn records_in_range(&self, min_tick: u64, max_tick: u64) -> Vec<&MutationRecord> {
        self.records
            .iter()
            .filter(|r| r.tick >= min_tick && r.tick <= max_tick)
            .collect()
    }

    /// Get records for a specific chunk.
    pub fn records_for_chunk(&self, chunk_pos: ChunkPos) -> Vec<&MutationRecord> {
        self.records
            .iter()
            .filter(|r| r.chunk_pos == chunk_pos)
            .collect()
    }

    /// Get records by source type.
    pub fn records_by_source(&self, source: MutationSource) -> Vec<&MutationRecord> {
        self.records.iter().filter(|r| r.source == source).collect()
    }

    /// Get records by reason type.
    pub fn records_by_reason(&self, reason: MutationReason) -> Vec<&MutationRecord> {
        self.records.iter().filter(|r| r.reason == reason).collect()
    }

    /// Truncate records before a tick.
    pub fn truncate_before(&mut self, tick: u64) {
        self.records.retain(|r| r.tick >= tick);
    }

    /// Retain only the most recent N ticks.
    pub fn retain_recent(&mut self, tick_count: u64) {
        if let Some(max_tick) = self.records.iter().map(|r| r.tick).max() {
            let min_tick = max_tick.saturating_sub(tick_count.saturating_sub(1));
            self.truncate_before(min_tick);
        }
    }

    /// Compact by removing no-op records.
    pub fn compact_noops(&mut self) {
        self.records.retain(|r| !r.is_noop());
    }

    /// Clear all records.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Sort records into deterministic order.
    pub fn sort(&mut self) {
        self.records.sort();
    }

    /// Check if records are in sorted order.
    #[must_use]
    pub fn is_sorted(&self) -> bool {
        self.records.windows(2).all(|w| w[0] <= w[1])
    }

    /// Compute a deterministic checksum of all records.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        use std::hash::{Hash, Hasher};

        struct ChecksumHasher(u32);

        impl Hasher for ChecksumHasher {
            fn finish(&self) -> u64 {
                u64::from(self.0)
            }

            fn write(&mut self, bytes: &[u8]) {
                for &byte in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(u32::from(byte));
                }
            }
        }

        let mut hasher = ChecksumHasher(0);
        for record in &self.records {
            record.tick.hash(&mut hasher);
            record.sequence.hash(&mut hasher);
            record.chunk_pos.x().hash(&mut hasher);
            record.chunk_pos.y().hash(&mut hasher);
            record.chunk_pos.z().hash(&mut hasher);
            record.local_index.raw().hash(&mut hasher);
            record.old_block.0.hash(&mut hasher);
            record.new_block.0.hash(&mut hasher);
        }
        hasher.0
    }

    /// Compute checksum for a tick range.
    #[must_use]
    pub fn checksum_range(&self, min_tick: u64, max_tick: u64) -> u32 {
        use std::hash::{Hash, Hasher};

        struct ChecksumHasher(u32);

        impl Hasher for ChecksumHasher {
            fn finish(&self) -> u64 {
                u64::from(self.0)
            }

            fn write(&mut self, bytes: &[u8]) {
                for &byte in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(u32::from(byte));
                }
            }
        }

        let mut hasher = ChecksumHasher(0);
        for record in &self.records {
            if record.tick >= min_tick && record.tick <= max_tick {
                record.tick.hash(&mut hasher);
                record.sequence.hash(&mut hasher);
                record.chunk_pos.x().hash(&mut hasher);
                record.chunk_pos.y().hash(&mut hasher);
                record.chunk_pos.z().hash(&mut hasher);
                record.local_index.raw().hash(&mut hasher);
                record.old_block.0.hash(&mut hasher);
                record.new_block.0.hash(&mut hasher);
            }
        }
        hasher.0
    }

    /// Generate a late-join snapshot.
    ///
    /// Produces chunk deltas representing state at `snapshot_tick` and
    /// pending mutations from `snapshot_tick+1` to current tick.
    #[must_use]
    pub fn snapshot(
        &self,
        snapshot_tick: u64,
        base_chunks: &HashMap<ChunkPos, Chunk>,
    ) -> JournalSnapshot {
        let mut chunk_deltas = HashMap::new();

        for (&chunk_pos, base) in base_chunks {
            let mut delta = ChunkDelta::new();

            for record in &self.records {
                if record.chunk_pos == chunk_pos && record.tick <= snapshot_tick {
                    delta.set(record.local_pos(), record.new_block);
                }
            }

            delta.compact(base);
            if !delta.is_empty() {
                chunk_deltas.insert(chunk_pos, delta);
            }
        }

        let pending_mutations: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.tick > snapshot_tick)
            .cloned()
            .collect();

        let end_tick = pending_mutations
            .last()
            .map_or(snapshot_tick, |r| r.tick + 1);

        JournalSnapshot {
            base_tick: snapshot_tick,
            end_tick,
            chunk_deltas,
            pending_mutations,
        }
    }

    /// Generate a snapshot slice for specific chunks.
    #[must_use]
    pub fn snapshot_slice(
        &self,
        snapshot_tick: u64,
        base_chunks: &HashMap<ChunkPos, Chunk>,
        chunk_filter: &[ChunkPos],
    ) -> JournalSnapshot {
        let filtered: HashMap<_, _> = base_chunks
            .iter()
            .filter(|(pos, _)| chunk_filter.contains(pos))
            .map(|(&pos, chunk)| (pos, chunk.clone()))
            .collect();
        self.snapshot(snapshot_tick, &filtered)
    }

    /// Rollback mutations on a chunk to a target tick.
    ///
    /// Applies reverse mutations from current state back to `target_tick`.
    /// Returns the number of blocks rolled back.
    pub fn rollback_chunk(
        &mut self,
        chunk: &mut Chunk,
        chunk_pos: ChunkPos,
        target_tick: u64,
    ) -> usize {
        let mut reversed: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.chunk_pos == chunk_pos && r.tick > target_tick)
            .cloned()
            .collect();

        reversed.sort_by(|a, b| b.cmp(a));

        let count = reversed.len();
        for record in reversed {
            chunk.set(record.local_pos(), record.old_block);
            let reverse = record.reversed(self.current_tick, self.next_sequence);
            self.next_sequence += 1;
            self.records.push(reverse);
        }

        count
    }

    /// Rollback mutations on multiple chunks.
    pub fn rollback_chunks(
        &mut self,
        chunks: &mut HashMap<ChunkPos, Chunk>,
        target_tick: u64,
    ) -> usize {
        let chunk_positions: Vec<_> = chunks.keys().copied().collect();
        let mut total = 0;
        for pos in chunk_positions {
            if let Some(chunk) = chunks.get_mut(&pos) {
                total += self.rollback_chunk(chunk, pos, target_tick);
            }
        }
        total
    }

    /// Compute statistics about the journal.
    #[must_use]
    pub fn stats(&self) -> JournalStats {
        let mut stats = JournalStats {
            record_count: self.records.len(),
            ..Default::default()
        };

        let mut chunks = std::collections::HashSet::new();

        for record in &self.records {
            chunks.insert(record.chunk_pos);

            if stats.min_tick.is_none() || Some(record.tick) < stats.min_tick {
                stats.min_tick = Some(record.tick);
            }
            if stats.max_tick.is_none() || Some(record.tick) > stats.max_tick {
                stats.max_tick = Some(record.tick);
            }

            stats.by_source[record.source as usize] += 1;
            stats.by_reason[record.reason as usize] += 1;
        }

        stats.chunk_count = chunks.len();
        stats
    }

    /// Summarize journal contents as text.
    #[must_use]
    pub fn summarize(&self) -> String {
        let stats = self.stats();
        format!(
            "MutationJournal: {} records, {} chunks, ticks {:?}-{:?}",
            stats.record_count, stats.chunk_count, stats.min_tick, stats.max_tick
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{AIR, STONE};

    fn make_journal() -> MutationJournal {
        let mut journal = MutationJournal::at_tick(100);

        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        journal.advance_tick(101);
        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(6, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        journal.append(
            ChunkPos::new(1, 0, 0),
            LocalPos::new(0, 0, 0),
            STONE,
            AIR,
            MutationSource::Environment,
            MutationReason::Collapse,
        );

        journal.advance_tick(102);
        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(5, 5, 5),
            STONE,
            AIR,
            MutationSource::Player,
            MutationReason::Destroy,
        );

        journal
    }

    #[test]
    fn test_new_journal() {
        let journal = MutationJournal::new();
        assert!(journal.is_empty());
        assert_eq!(journal.current_tick(), 0);
        assert_eq!(journal.next_sequence(), 0);
    }

    #[test]
    fn test_at_tick() {
        let journal = MutationJournal::at_tick(500);
        assert_eq!(journal.current_tick(), 500);
    }

    #[test]
    fn test_append() {
        let mut journal = MutationJournal::at_tick(10);

        journal.append(
            ChunkPos::new(1, 2, 3),
            LocalPos::new(4, 5, 6),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        assert_eq!(journal.len(), 1);
        assert_eq!(journal.next_sequence(), 1);

        let record = &journal.records()[0];
        assert_eq!(record.tick, 10);
        assert_eq!(record.sequence, 0);
        assert_eq!(record.chunk_pos, ChunkPos::new(1, 2, 3));
        assert_eq!(record.old_block, AIR);
        assert_eq!(record.new_block, STONE);
    }

    #[test]
    fn test_advance_tick() {
        let mut journal = MutationJournal::at_tick(10);

        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );
        assert_eq!(journal.next_sequence(), 1);

        journal.advance_tick(11);
        assert_eq!(journal.current_tick(), 11);
        assert_eq!(journal.next_sequence(), 0);
    }

    #[test]
    fn test_records_in_range() {
        let journal = make_journal();

        let in_range = journal.records_in_range(100, 101);
        assert_eq!(in_range.len(), 3);

        let single_tick = journal.records_in_range(102, 102);
        assert_eq!(single_tick.len(), 1);
    }

    #[test]
    fn test_records_for_chunk() {
        let journal = make_journal();

        let chunk0 = journal.records_for_chunk(ChunkPos::new(0, 0, 0));
        assert_eq!(chunk0.len(), 3);

        let chunk1 = journal.records_for_chunk(ChunkPos::new(1, 0, 0));
        assert_eq!(chunk1.len(), 1);
    }

    #[test]
    fn test_records_by_source() {
        let journal = make_journal();

        let player = journal.records_by_source(MutationSource::Player);
        assert_eq!(player.len(), 3);

        let env = journal.records_by_source(MutationSource::Environment);
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn test_query() {
        let journal = make_journal();

        let query = MutationQuery::new()
            .tick_range(100, 101)
            .source(MutationSource::Player);

        let results = journal.query(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_with_limit() {
        let journal = make_journal();

        let query = MutationQuery::new().limit(2);
        let results = journal.query(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_truncate_before() {
        let mut journal = make_journal();
        assert_eq!(journal.len(), 4);

        journal.truncate_before(102);
        assert_eq!(journal.len(), 1);
    }

    #[test]
    fn test_retain_recent() {
        let mut journal = make_journal();
        assert_eq!(journal.len(), 4);

        journal.retain_recent(2);
        let ticks: Vec<_> = journal.records().iter().map(|r| r.tick).collect();
        assert!(ticks.iter().all(|&t| t >= 101));
    }

    #[test]
    fn test_compact_noops() {
        let mut journal = MutationJournal::at_tick(10);

        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(1, 0, 0),
            STONE,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        assert_eq!(journal.len(), 2);
        journal.compact_noops();
        assert_eq!(journal.len(), 1);
    }

    #[test]
    fn test_is_sorted() {
        let journal = make_journal();
        assert!(journal.is_sorted());
    }

    #[test]
    fn test_checksum_deterministic() {
        let journal1 = make_journal();
        let journal2 = make_journal();

        assert_eq!(journal1.checksum(), journal2.checksum());
    }

    #[test]
    fn test_checksum_differs() {
        let journal1 = make_journal();

        let mut journal2 = make_journal();
        journal2.advance_tick(200);
        journal2.append(
            ChunkPos::new(5, 5, 5),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Custom,
            MutationReason::Generate,
        );

        assert_ne!(journal1.checksum(), journal2.checksum());
    }

    #[test]
    fn test_checksum_range() {
        let journal = make_journal();

        let full = journal.checksum();
        let partial = journal.checksum_range(100, 101);

        assert_ne!(full, partial);
    }

    #[test]
    fn test_snapshot() {
        let mut journal = MutationJournal::at_tick(100);
        let chunk_pos = ChunkPos::new(0, 0, 0);

        journal.append(
            chunk_pos,
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        journal.advance_tick(101);
        journal.append(
            chunk_pos,
            LocalPos::new(6, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let base = Chunk::new();
        let mut bases = HashMap::new();
        bases.insert(chunk_pos, base);

        let snapshot = journal.snapshot(100, &bases);

        assert_eq!(snapshot.base_tick, 100);
        assert_eq!(snapshot.chunk_deltas.len(), 1);
        assert_eq!(snapshot.pending_mutations.len(), 1);

        let delta = snapshot.chunk_deltas.get(&chunk_pos).unwrap();
        assert_eq!(delta.get(LocalPos::new(5, 5, 5)), Some(STONE));
        assert_eq!(delta.get(LocalPos::new(6, 5, 5)), None);
    }

    #[test]
    fn test_rollback_chunk() {
        let mut journal = MutationJournal::at_tick(100);
        let chunk_pos = ChunkPos::new(0, 0, 0);
        let pos = LocalPos::new(5, 5, 5);

        let mut chunk = Chunk::new();
        chunk.set(pos, STONE);

        journal.append(
            chunk_pos,
            pos,
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        journal.advance_tick(101);
        chunk.set(pos, BlockId(50));

        journal.append(
            chunk_pos,
            pos,
            STONE,
            BlockId(50),
            MutationSource::Player,
            MutationReason::Replace,
        );

        journal.advance_tick(102);

        let rolled = journal.rollback_chunk(&mut chunk, chunk_pos, 100);
        assert_eq!(rolled, 1);
        assert_eq!(chunk.get(pos), STONE);
    }

    #[test]
    fn test_stats() {
        let journal = make_journal();
        let stats = journal.stats();

        assert_eq!(stats.record_count, 4);
        assert_eq!(stats.chunk_count, 2);
        assert_eq!(stats.min_tick, Some(100));
        assert_eq!(stats.max_tick, Some(102));
        assert_eq!(stats.by_source[MutationSource::Player as usize], 3);
        assert_eq!(stats.by_source[MutationSource::Environment as usize], 1);
    }

    #[test]
    fn test_summarize() {
        let journal = make_journal();
        let summary = journal.summarize();
        assert!(summary.contains("4 records"));
        assert!(summary.contains("2 chunks"));
    }

    #[test]
    fn test_mutation_record_builder() {
        let record = MutationRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        )
        .with_source_id(42)
        .with_tag("test")
        .with_tags(["a", "b"]);

        assert_eq!(record.source_id, Some(42));
        assert!(record.has_tag("test"));
        assert!(record.has_tag("a"));
        assert!(record.has_tag("b"));
    }

    #[test]
    fn test_mutation_record_reversed() {
        let record = MutationRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let reversed = record.reversed(200, 5);

        assert_eq!(reversed.tick, 200);
        assert_eq!(reversed.sequence, 5);
        assert_eq!(reversed.old_block, STONE);
        assert_eq!(reversed.new_block, AIR);
        assert_eq!(reversed.source, MutationSource::Rollback);
        assert!(reversed.has_tag("rollback"));
    }

    #[test]
    fn test_mutation_record_ordering() {
        let r1 = MutationRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let r2 = MutationRecord::new(
            100,
            1,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let r3 = MutationRecord::new(
            101,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        assert!(r1 < r2);
        assert!(r2 < r3);
    }

    #[test]
    fn test_append_delta() {
        let mut journal = MutationJournal::at_tick(100);
        let chunk_pos = ChunkPos::new(0, 0, 0);

        let base = Chunk::new();
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(5, 5, 5), STONE);
        delta.set(LocalPos::new(6, 5, 5), STONE);

        journal.append_delta(chunk_pos, &delta, &base, MutationSource::NetworkSync);

        assert_eq!(journal.len(), 2);
        assert!(
            journal
                .records()
                .iter()
                .all(|r| r.source == MutationSource::NetworkSync)
        );
        assert!(
            journal
                .records()
                .iter()
                .all(|r| r.reason == MutationReason::DeltaApply)
        );
    }

    #[test]
    fn test_serde_roundtrip_record() {
        let record = MutationRecord::new(
            100,
            5,
            ChunkPos::new(1, 2, 3),
            LocalPos::new(4, 5, 6),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        )
        .with_source_id(42)
        .with_tag("test");

        let serialized = bincode::serialize(&record).unwrap();
        let deserialized: MutationRecord = bincode::deserialize(&serialized).unwrap();

        assert_eq!(record, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_journal() {
        let journal = make_journal();

        let serialized = bincode::serialize(&journal).unwrap();
        let deserialized: MutationJournal = bincode::deserialize(&serialized).unwrap();

        assert_eq!(journal.len(), deserialized.len());
        assert_eq!(journal.current_tick(), deserialized.current_tick());
        assert_eq!(journal.checksum(), deserialized.checksum());
    }

    #[test]
    fn test_serde_roundtrip_snapshot() {
        let mut journal = MutationJournal::at_tick(100);
        let chunk_pos = ChunkPos::new(0, 0, 0);

        journal.append(
            chunk_pos,
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let mut bases = HashMap::new();
        bases.insert(chunk_pos, Chunk::new());

        let snapshot = journal.snapshot(99, &bases);

        let serialized = bincode::serialize(&snapshot).unwrap();
        let deserialized: JournalSnapshot = bincode::deserialize(&serialized).unwrap();

        assert_eq!(snapshot.base_tick, deserialized.base_tick);
        assert_eq!(
            snapshot.pending_mutations.len(),
            deserialized.pending_mutations.len()
        );
    }

    #[test]
    fn test_serde_json_record() {
        let record = MutationRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: MutationRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(record, deserialized);
    }

    #[test]
    fn test_deterministic_ordering_after_sort() {
        let mut journal = MutationJournal::at_tick(100);

        journal.records.push(MutationRecord::new(
            102,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        ));

        journal.records.push(MutationRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        ));

        journal.records.push(MutationRecord::new(
            101,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        ));

        assert!(!journal.is_sorted());
        journal.sort();
        assert!(journal.is_sorted());

        let ticks: Vec<_> = journal.records().iter().map(|r| r.tick).collect();
        assert_eq!(ticks, vec![100, 101, 102]);
    }

    #[test]
    fn test_query_by_tag() {
        let mut journal = MutationJournal::at_tick(100);

        let record = MutationRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        )
        .with_tag("important");

        journal.append_record(record);

        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(1, 0, 0),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        let query = MutationQuery::new().tag("important");
        let results = journal.query(&query);

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_source_and_reason_names() {
        assert_eq!(MutationSource::Player.name(), "player");
        assert_eq!(MutationSource::Environment.name(), "environment");
        assert_eq!(MutationReason::Place.name(), "place");
        assert_eq!(MutationReason::Destroy.name(), "destroy");
    }

    #[test]
    fn test_journal_snapshot_empty() {
        let snapshot = JournalSnapshot::empty(100);
        assert_eq!(snapshot.base_tick, 100);
        assert_eq!(snapshot.end_tick, 100);
        assert_eq!(snapshot.chunk_count(), 0);
        assert_eq!(snapshot.mutation_count(), 0);
    }
}

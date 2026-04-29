//! Compact hazard delta representation for network synchronization.
//!
//! Provides deterministic, space-efficient encoding of hazard state changes
//! with support for per-client area filtering and since-tick delta queries.

use std::collections::{BTreeMap, HashMap};

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use super::HazardKind;
use crate::persistence::DeltaIndex;
use crate::replay::{ChecksumBuilder, StepChecksum};

/// Compact single-cell hazard change.
///
/// Encodes a hazard intensity change at a specific position within a chunk.
/// Uses `Option<f32>` where `None` indicates deactivation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HazardCellDelta {
    /// Compact local position index (0-4095).
    pub index: DeltaIndex,
    /// New intensity, or `None` for deactivation.
    pub intensity: Option<f32>,
}

impl HazardCellDelta {
    /// Create a delta setting intensity at a position.
    #[must_use]
    pub fn set(pos: LocalPos, intensity: f32) -> Self {
        Self {
            index: DeltaIndex::from_local_pos(pos),
            intensity: Some(intensity),
        }
    }

    /// Create a delta deactivating a position.
    #[must_use]
    pub fn deactivate(pos: LocalPos) -> Self {
        Self {
            index: DeltaIndex::from_local_pos(pos),
            intensity: None,
        }
    }

    /// Get the local position.
    #[must_use]
    pub fn local_pos(&self) -> LocalPos {
        self.index.to_local_pos()
    }

    /// Check if this delta deactivates the cell.
    #[must_use]
    pub fn is_deactivation(&self) -> bool {
        self.intensity.is_none()
    }
}

/// Per-kind hazard changes within a single chunk.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChunkHazardDelta {
    /// Changes per hazard kind. Only populated kinds are present.
    changes: BTreeMap<HazardKind, Vec<HazardCellDelta>>,
}

impl ChunkHazardDelta {
    /// Create an empty delta.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the delta has no changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty() || self.changes.values().all(Vec::is_empty)
    }

    /// Total number of cell changes across all hazard kinds.
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.changes.values().map(Vec::len).sum()
    }

    /// Add a cell change for a hazard kind.
    pub fn add(&mut self, kind: HazardKind, delta: HazardCellDelta) {
        self.changes.entry(kind).or_default().push(delta);
    }

    /// Add a set operation.
    pub fn add_set(&mut self, kind: HazardKind, pos: LocalPos, intensity: f32) {
        self.add(kind, HazardCellDelta::set(pos, intensity));
    }

    /// Add a deactivation.
    pub fn add_deactivate(&mut self, kind: HazardKind, pos: LocalPos) {
        self.add(kind, HazardCellDelta::deactivate(pos));
    }

    /// Get changes for a specific hazard kind.
    #[must_use]
    pub fn get(&self, kind: HazardKind) -> Option<&[HazardCellDelta]> {
        self.changes.get(&kind).map(Vec::as_slice)
    }

    /// Iterate over all (kind, deltas) pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (HazardKind, &[HazardCellDelta])> {
        self.changes.iter().map(|(&k, v)| (k, v.as_slice()))
    }

    /// Iterate over all kinds that have changes.
    pub fn kinds(&self) -> impl Iterator<Item = HazardKind> + '_ {
        self.changes.keys().copied()
    }

    /// Merge another delta into this one.
    pub fn merge(&mut self, other: Self) {
        for (kind, deltas) in other.changes {
            self.changes.entry(kind).or_default().extend(deltas);
        }
    }

    /// Sort all deltas by position index for deterministic ordering.
    pub fn sort(&mut self) {
        for deltas in self.changes.values_mut() {
            deltas.sort_by_key(|d| d.index.raw());
        }
    }

    /// Compute a deterministic checksum of all changes.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "index and len fit in u32")]
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        for (kind, deltas) in &self.changes {
            builder.feed_u32(kind.as_index() as u32);
            builder.feed_u32(deltas.len() as u32);
            for delta in deltas {
                builder.feed_u32(u32::from(delta.index.raw()));
                match delta.intensity {
                    Some(i) => {
                        builder.feed_u32(1);
                        builder.feed_f32(i);
                    }
                    None => {
                        builder.feed_u32(0);
                    }
                }
            }
        }
        builder.build()
    }
}

/// A timestamped hazard delta record for journaling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HazardDeltaRecord {
    /// Simulation tick when changes occurred.
    pub tick: u64,
    /// Sequence number within the tick.
    pub sequence: u32,
    /// Chunk position.
    pub chunk_pos: ChunkPos,
    /// The hazard changes.
    pub delta: ChunkHazardDelta,
}

impl HazardDeltaRecord {
    /// Create a new record.
    #[must_use]
    pub fn new(tick: u64, sequence: u32, chunk_pos: ChunkPos, delta: ChunkHazardDelta) -> Self {
        Self {
            tick,
            sequence,
            chunk_pos,
            delta,
        }
    }

    /// Ordering key for deterministic sorting.
    fn sort_key(&self) -> (u64, u32, i32, i32, i32) {
        (
            self.tick,
            self.sequence,
            self.chunk_pos.x(),
            self.chunk_pos.y(),
            self.chunk_pos.z(),
        )
    }
}

impl PartialOrd for HazardDeltaRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HazardDeltaRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for HazardDeltaRecord {}

/// Journal of hazard changes for late-join sync and delta queries.
///
/// Provides deterministic ordered storage of hazard deltas with support for:
/// - Tick-based querying (`since_tick`)
/// - Chunk position filtering (area bounds)
/// - Compaction and retention policies
/// - Checksum computation for verification
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HazardDeltaJournal {
    records: Vec<HazardDeltaRecord>,
    next_sequence: u32,
    current_tick: u64,
}

impl HazardDeltaJournal {
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

    /// Append a hazard delta for a chunk.
    pub fn append(&mut self, chunk_pos: ChunkPos, delta: ChunkHazardDelta) {
        if delta.is_empty() {
            return;
        }
        let record =
            HazardDeltaRecord::new(self.current_tick, self.next_sequence, chunk_pos, delta);
        self.next_sequence += 1;
        self.records.push(record);
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
    pub fn records(&self) -> &[HazardDeltaRecord] {
        &self.records
    }

    /// Query records since a given tick (exclusive).
    pub fn since_tick(&self, tick: u64) -> impl Iterator<Item = &HazardDeltaRecord> {
        self.records.iter().filter(move |r| r.tick > tick)
    }

    /// Query records for chunks within a bounding box.
    pub fn in_area(
        &self,
        min: ChunkPos,
        max: ChunkPos,
    ) -> impl Iterator<Item = &HazardDeltaRecord> {
        self.records.iter().filter(move |r| {
            r.chunk_pos.x() >= min.x()
                && r.chunk_pos.x() <= max.x()
                && r.chunk_pos.y() >= min.y()
                && r.chunk_pos.y() <= max.y()
                && r.chunk_pos.z() >= min.z()
                && r.chunk_pos.z() <= max.z()
        })
    }

    /// Query records since a tick within a bounding box.
    pub fn since_tick_in_area(
        &self,
        tick: u64,
        min: ChunkPos,
        max: ChunkPos,
    ) -> impl Iterator<Item = &HazardDeltaRecord> {
        self.records.iter().filter(move |r| {
            r.tick > tick
                && r.chunk_pos.x() >= min.x()
                && r.chunk_pos.x() <= max.x()
                && r.chunk_pos.y() >= min.y()
                && r.chunk_pos.y() <= max.y()
                && r.chunk_pos.z() >= min.z()
                && r.chunk_pos.z() <= max.z()
        })
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
    pub fn checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        for record in &self.records {
            builder.feed_u64(record.tick);
            builder.feed_u32(record.sequence);
            builder.feed_i32(record.chunk_pos.x());
            builder.feed_i32(record.chunk_pos.y());
            builder.feed_i32(record.chunk_pos.z());
            let delta_checksum = record.delta.checksum();
            builder.feed_u32(delta_checksum.value());
        }
        builder.build()
    }

    /// Compute checksum for a tick range.
    #[must_use]
    pub fn checksum_range(&self, min_tick: u64, max_tick: u64) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        for record in &self.records {
            if record.tick >= min_tick && record.tick <= max_tick {
                builder.feed_u64(record.tick);
                builder.feed_u32(record.sequence);
                builder.feed_i32(record.chunk_pos.x());
                builder.feed_i32(record.chunk_pos.y());
                builder.feed_i32(record.chunk_pos.z());
                let delta_checksum = record.delta.checksum();
                builder.feed_u32(delta_checksum.value());
            }
        }
        builder.build()
    }
}

/// Compact snapshot of hazard state for late-join clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HazardSnapshot {
    /// Base tick for the snapshot.
    pub base_tick: u64,
    /// Full hazard state per chunk (only chunks with active hazards).
    pub chunk_states: HashMap<ChunkPos, ChunkHazardSnapshot>,
}

/// Per-chunk hazard snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkHazardSnapshot {
    /// Active hazard cells per kind.
    pub layers: BTreeMap<HazardKind, Vec<(DeltaIndex, f32)>>,
}

impl ChunkHazardSnapshot {
    /// Create an empty snapshot.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
        }
    }

    /// Check if the snapshot has any hazards.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty() || self.layers.values().all(Vec::is_empty)
    }

    /// Add an active cell.
    pub fn add(&mut self, kind: HazardKind, pos: LocalPos, intensity: f32) {
        self.layers
            .entry(kind)
            .or_default()
            .push((DeltaIndex::from_local_pos(pos), intensity));
    }

    /// Get active cells for a kind.
    #[must_use]
    pub fn get(&self, kind: HazardKind) -> Option<&[(DeltaIndex, f32)]> {
        self.layers.get(&kind).map(Vec::as_slice)
    }

    /// Iterate over all (kind, cells) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (HazardKind, &[(DeltaIndex, f32)])> {
        self.layers.iter().map(|(&k, v)| (k, v.as_slice()))
    }

    /// Total active cell count.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.layers.values().map(Vec::len).sum()
    }
}

impl Default for ChunkHazardSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl HazardSnapshot {
    /// Create an empty snapshot at a tick.
    #[must_use]
    pub fn empty(tick: u64) -> Self {
        Self {
            base_tick: tick,
            chunk_states: HashMap::new(),
        }
    }

    /// Number of chunks with hazard state.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunk_states.len()
    }

    /// Total active hazard cells across all chunks.
    #[must_use]
    pub fn total_active(&self) -> usize {
        self.chunk_states
            .values()
            .map(ChunkHazardSnapshot::active_count)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_delta_constructors() {
        let set = HazardCellDelta::set(LocalPos::new(5, 6, 7), 0.75);
        assert_eq!(set.local_pos(), LocalPos::new(5, 6, 7));
        assert_eq!(set.intensity, Some(0.75));
        assert!(!set.is_deactivation());

        let deactivate = HazardCellDelta::deactivate(LocalPos::new(1, 2, 3));
        assert!(deactivate.is_deactivation());
        assert_eq!(deactivate.intensity, None);
    }

    #[test]
    fn chunk_delta_empty() {
        let delta = ChunkHazardDelta::new();
        assert!(delta.is_empty());
        assert_eq!(delta.change_count(), 0);
    }

    #[test]
    fn chunk_delta_add_changes() {
        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        delta.add_set(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        delta.add_deactivate(HazardKind::Frost, LocalPos::new(5, 5, 5));

        assert!(!delta.is_empty());
        assert_eq!(delta.change_count(), 3);
        assert_eq!(delta.get(HazardKind::Fire).map(<[_]>::len), Some(2));
        assert_eq!(delta.get(HazardKind::Frost).map(<[_]>::len), Some(1));
        assert!(delta.get(HazardKind::Infection).is_none());
    }

    #[test]
    fn chunk_delta_deterministic_iteration() {
        let mut delta1 = ChunkHazardDelta::new();
        delta1.add_set(HazardKind::Frost, LocalPos::new(0, 0, 0), 0.5);
        delta1.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);

        let mut delta2 = ChunkHazardDelta::new();
        delta2.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        delta2.add_set(HazardKind::Frost, LocalPos::new(0, 0, 0), 0.5);

        let kinds1: Vec<_> = delta1.kinds().collect();
        let kinds2: Vec<_> = delta2.kinds().collect();
        assert_eq!(kinds1, kinds2);
    }

    #[test]
    fn chunk_delta_merge() {
        let mut delta1 = ChunkHazardDelta::new();
        delta1.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);

        let mut delta2 = ChunkHazardDelta::new();
        delta2.add_set(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        delta2.add_set(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);

        delta1.merge(delta2);
        assert_eq!(delta1.change_count(), 3);
    }

    #[test]
    fn chunk_delta_checksum_deterministic() {
        let mut delta1 = ChunkHazardDelta::new();
        delta1.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        delta1.add_set(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);

        let mut delta2 = ChunkHazardDelta::new();
        delta2.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        delta2.add_set(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);

        assert_eq!(delta1.checksum(), delta2.checksum());
    }

    #[test]
    fn journal_basic_operations() {
        let mut journal = HazardDeltaJournal::at_tick(100);
        assert!(journal.is_empty());
        assert_eq!(journal.current_tick(), 100);

        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);

        journal.append(ChunkPos::new(0, 0, 0), delta);
        assert_eq!(journal.len(), 1);
    }

    #[test]
    fn journal_advance_tick() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        journal.append(ChunkPos::new(0, 0, 0), delta);

        journal.advance_tick(101);
        assert_eq!(journal.current_tick(), 101);

        let mut delta2 = ChunkHazardDelta::new();
        delta2.add_set(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        journal.append(ChunkPos::new(0, 0, 0), delta2);

        assert_eq!(journal.len(), 2);
        assert_eq!(journal.records()[0].tick, 100);
        assert_eq!(journal.records()[1].tick, 101);
    }

    #[test]
    fn journal_since_tick() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        for tick in 100..105 {
            journal.advance_tick(tick);
            let mut delta = ChunkHazardDelta::new();
            delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
            journal.append(ChunkPos::new(0, 0, 0), delta);
        }

        let since_102: Vec<_> = journal.since_tick(102).collect();
        assert_eq!(since_102.len(), 2);
        assert!(since_102.iter().all(|r| r.tick > 102));
    }

    #[test]
    fn journal_in_area() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        for x in -2..3 {
            let mut delta = ChunkHazardDelta::new();
            delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
            journal.append(ChunkPos::new(x, 0, 0), delta);
        }

        let in_area: Vec<_> = journal
            .in_area(ChunkPos::new(-1, 0, 0), ChunkPos::new(1, 0, 0))
            .collect();
        assert_eq!(in_area.len(), 3);
    }

    #[test]
    fn journal_since_tick_in_area() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        for tick in 100..103 {
            journal.advance_tick(tick);
            for x in -1..2 {
                let mut delta = ChunkHazardDelta::new();
                delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
                journal.append(ChunkPos::new(x, 0, 0), delta);
            }
        }

        let filtered: Vec<_> = journal
            .since_tick_in_area(100, ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0))
            .collect();
        assert_eq!(filtered.len(), 4);
    }

    #[test]
    fn journal_truncate_before() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        for tick in 100..110 {
            journal.advance_tick(tick);
            let mut delta = ChunkHazardDelta::new();
            delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
            journal.append(ChunkPos::new(0, 0, 0), delta);
        }

        journal.truncate_before(105);
        assert_eq!(journal.len(), 5);
        assert!(journal.records().iter().all(|r| r.tick >= 105));
    }

    #[test]
    fn journal_retain_recent() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        for tick in 100..110 {
            journal.advance_tick(tick);
            let mut delta = ChunkHazardDelta::new();
            delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
            journal.append(ChunkPos::new(0, 0, 0), delta);
        }

        journal.retain_recent(3);
        assert_eq!(journal.len(), 3);
    }

    #[test]
    fn journal_checksum_deterministic() {
        let mut journal1 = HazardDeltaJournal::at_tick(100);
        let mut journal2 = HazardDeltaJournal::at_tick(100);

        for journal in [&mut journal1, &mut journal2] {
            let mut delta = ChunkHazardDelta::new();
            delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
            journal.append(ChunkPos::new(0, 0, 0), delta);
        }

        assert_eq!(journal1.checksum(), journal2.checksum());
    }

    #[test]
    fn journal_sorting() {
        let mut journal = HazardDeltaJournal::at_tick(100);

        journal.records.push(HazardDeltaRecord::new(
            102,
            0,
            ChunkPos::new(0, 0, 0),
            ChunkHazardDelta::new(),
        ));
        journal.records.push(HazardDeltaRecord::new(
            100,
            0,
            ChunkPos::new(0, 0, 0),
            ChunkHazardDelta::new(),
        ));
        journal.records.push(HazardDeltaRecord::new(
            101,
            0,
            ChunkPos::new(0, 0, 0),
            ChunkHazardDelta::new(),
        ));

        assert!(!journal.is_sorted());
        journal.sort();
        assert!(journal.is_sorted());

        let ticks: Vec<_> = journal.records().iter().map(|r| r.tick).collect();
        assert_eq!(ticks, vec![100, 101, 102]);
    }

    #[test]
    fn snapshot_operations() {
        let mut snapshot = ChunkHazardSnapshot::new();
        assert!(snapshot.is_empty());

        snapshot.add(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        snapshot.add(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        snapshot.add(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);

        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.active_count(), 3);
        assert_eq!(snapshot.get(HazardKind::Fire).map(<[_]>::len), Some(2));
    }

    #[test]
    fn hazard_snapshot_totals() {
        let mut snapshot = HazardSnapshot::empty(100);
        assert_eq!(snapshot.chunk_count(), 0);
        assert_eq!(snapshot.total_active(), 0);

        let mut chunk_state = ChunkHazardSnapshot::new();
        chunk_state.add(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        chunk_state.add(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);
        snapshot
            .chunk_states
            .insert(ChunkPos::new(0, 0, 0), chunk_state);

        assert_eq!(snapshot.chunk_count(), 1);
        assert_eq!(snapshot.total_active(), 2);
    }

    #[test]
    fn serde_roundtrip_cell_delta() {
        let delta = HazardCellDelta::set(LocalPos::new(5, 6, 7), 0.75);
        let serialized = bincode::serialize(&delta).unwrap();
        let deserialized: HazardCellDelta = bincode::deserialize(&serialized).unwrap();
        assert_eq!(delta, deserialized);
    }

    #[test]
    fn serde_roundtrip_chunk_delta() {
        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        delta.add_deactivate(HazardKind::Frost, LocalPos::new(5, 5, 5));

        let serialized = bincode::serialize(&delta).unwrap();
        let deserialized: ChunkHazardDelta = bincode::deserialize(&serialized).unwrap();
        assert_eq!(delta, deserialized);
    }

    #[test]
    fn serde_roundtrip_journal() {
        let mut journal = HazardDeltaJournal::at_tick(100);
        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        journal.append(ChunkPos::new(0, 0, 0), delta);

        let serialized = bincode::serialize(&journal).unwrap();
        let deserialized: HazardDeltaJournal = bincode::deserialize(&serialized).unwrap();

        assert_eq!(journal.len(), deserialized.len());
        assert_eq!(journal.current_tick(), deserialized.current_tick());
        assert_eq!(journal.checksum(), deserialized.checksum());
    }

    #[test]
    fn serde_roundtrip_snapshot() {
        let mut snapshot = HazardSnapshot::empty(100);
        let mut chunk_state = ChunkHazardSnapshot::new();
        chunk_state.add(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        snapshot
            .chunk_states
            .insert(ChunkPos::new(0, 0, 0), chunk_state);

        let serialized = bincode::serialize(&snapshot).unwrap();
        let deserialized: HazardSnapshot = bincode::deserialize(&serialized).unwrap();

        assert_eq!(snapshot.base_tick, deserialized.base_tick);
        assert_eq!(snapshot.chunk_count(), deserialized.chunk_count());
    }

    #[test]
    fn compact_encoding_size() {
        let mut delta = ChunkHazardDelta::new();
        for i in 0..100 {
            delta.add_set(HazardKind::Fire, LocalPos::from_index(i), 0.5);
        }

        let serialized = bincode::serialize(&delta).unwrap();
        assert!(serialized.len() < 1000);
    }
}

//! Regional backup/restore for deterministic partial world rollback.
//!
//! Supports creating point-in-time snapshots of selected chunks (regions)
//! rather than full-world rollback, with checksum verification and
//! incremental restore planning.
//!
//! # Overview
//!
//! - [`BackupId`]: Deterministic identifier for backup snapshots
//! - [`BackupMetadata`]: Context and provenance for a backup
//! - [`ChunkEntry`]: Per-chunk data with position and checksum
//! - [`BackupManifest`]: Summary of backup contents and fingerprint
//! - [`RegionalBackup`]: Complete backup with manifest and chunk data
//! - [`RestorePlan`]: Planned restore operations with issue detection
//! - [`RestoreResult`]: Outcome of applied restore operations
//! - [`BackupIssue`]: Issues encountered during backup/restore

use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use crate::chunk::Chunk;
use crate::persistence::{ChunkChecksum, ChunkDelta, FingerprintBuilder, SnapshotFingerprint};

/// Deterministic identifier for a backup snapshot.
///
/// Generated from seed and sequence number using CRC32 for stability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackupId(u64);

impl BackupId {
    /// Generate a backup ID from seed and sequence.
    #[must_use]
    pub fn generate(seed: u64, sequence: u64) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&seed.to_le_bytes());
        hasher.update(&sequence.to_le_bytes());
        let low = hasher.finalize();

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&sequence.to_le_bytes());
        hasher.update(&seed.to_le_bytes());
        let high = hasher.finalize();

        Self((u64::from(high) << 32) | u64::from(low))
    }

    /// Create from a raw value (for deserialization).
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Category of backup/restore issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BackupIssueKind {
    /// Chunk is missing from the backup.
    MissingChunk = 0,
    /// Chunk data is stale (checksum mismatch with current).
    StaleChunk = 1,
    /// Checksum verification failed.
    ChecksumMismatch = 2,
    /// Chunk position is out of expected region bounds.
    OutOfBounds = 3,
    /// Duplicate chunk entry in backup.
    DuplicateEntry = 4,
    /// Backup manifest is corrupted or invalid.
    CorruptManifest = 5,
    /// Target chunk cannot be modified.
    ImmutableTarget = 6,
}

impl BackupIssueKind {
    /// Get the display name for this category.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MissingChunk => "missing_chunk",
            Self::StaleChunk => "stale_chunk",
            Self::ChecksumMismatch => "checksum_mismatch",
            Self::OutOfBounds => "out_of_bounds",
            Self::DuplicateEntry => "duplicate_entry",
            Self::CorruptManifest => "corrupt_manifest",
            Self::ImmutableTarget => "immutable_target",
        }
    }

    /// Check if this issue is recoverable.
    #[must_use]
    pub const fn is_recoverable(self) -> bool {
        matches!(self, Self::StaleChunk | Self::MissingChunk)
    }
}

/// Severity level for backup/restore issues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BackupIssueSeverity {
    /// Informational, no action required.
    Info = 0,
    /// Warning, restore may produce unexpected results.
    Warning = 1,
    /// Error, restore cannot proceed for this chunk.
    Error = 2,
    /// Critical, backup integrity compromised.
    Critical = 3,
}

/// An issue detected during backup or restore.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupIssue {
    /// Issue category.
    pub kind: BackupIssueKind,
    /// Issue severity.
    pub severity: BackupIssueSeverity,
    /// Affected chunk position.
    pub chunk_pos: ChunkPos,
    /// Human-readable description.
    pub description: String,
    /// Expected value (checksum, etc).
    pub expected: Option<String>,
    /// Actual value encountered.
    pub actual: Option<String>,
}

impl BackupIssue {
    /// Create a new backup issue.
    #[must_use]
    pub fn new(kind: BackupIssueKind, severity: BackupIssueSeverity, chunk_pos: ChunkPos) -> Self {
        Self {
            kind,
            severity,
            chunk_pos,
            description: String::new(),
            expected: None,
            actual: None,
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set expected/actual values.
    #[must_use]
    pub fn with_values(mut self, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }

    /// Check if this issue blocks restore.
    #[must_use]
    pub fn blocks_restore(&self) -> bool {
        matches!(
            self.severity,
            BackupIssueSeverity::Error | BackupIssueSeverity::Critical
        )
    }
}

/// Per-chunk entry in a backup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkEntry {
    /// Chunk position.
    pub pos: ChunkPos,
    /// CRC32 checksum of chunk data.
    pub checksum: ChunkChecksum,
    /// Serialized chunk data.
    pub data: Vec<u8>,
    /// Non-air block count at backup time.
    pub non_air_count: u32,
}

impl ChunkEntry {
    /// Create a new chunk entry from a chunk.
    ///
    /// # Panics
    ///
    /// Panics if bincode serialization fails (should not happen for valid chunks).
    #[must_use]
    pub fn from_chunk(pos: ChunkPos, chunk: &Chunk) -> Self {
        let checksum = ChunkChecksum::compute(chunk);
        let data = bincode::serialize(chunk).expect("chunk serialization should not fail");
        Self {
            pos,
            checksum,
            data,
            non_air_count: chunk.non_air_count(),
        }
    }

    /// Deserialize the chunk data.
    ///
    /// Returns `None` if deserialization fails.
    #[must_use]
    pub fn to_chunk(&self) -> Option<Chunk> {
        bincode::deserialize(&self.data).ok()
    }

    /// Verify the checksum matches the stored data.
    #[must_use]
    pub fn verify_checksum(&self) -> bool {
        self.to_chunk()
            .is_some_and(|c| ChunkChecksum::compute(&c).matches(self.checksum))
    }

    /// Compute entry fingerprint for manifest.
    #[must_use]
    pub fn fingerprint(&self) -> SnapshotFingerprint {
        let mut builder = FingerprintBuilder::new();
        builder.feed_chunk_pos(self.pos);
        builder.feed_u32(self.checksum.value());
        builder.feed_u32(self.non_air_count);
        builder.build()
    }
}

/// Metadata about a backup snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup identifier.
    pub id: BackupId,
    /// Backup creation tick (game time).
    pub tick: u64,
    /// Human-readable label.
    pub label: String,
    /// Optional description/reason.
    pub description: Option<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Source system that created the backup.
    pub source: String,
}

impl BackupMetadata {
    /// Create new backup metadata.
    #[must_use]
    pub fn new(id: BackupId, tick: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            tick,
            label: label.into(),
            description: None,
            tags: Vec::new(),
            source: String::from("regional_backup"),
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the source system.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Compute metadata fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> SnapshotFingerprint {
        let mut builder = FingerprintBuilder::new();
        builder.feed_u64(self.id.as_u64());
        builder.feed_u64(self.tick);
        builder.feed_bytes(self.label.as_bytes());
        if let Some(ref desc) = self.description {
            builder.feed_bytes(desc.as_bytes());
        }
        for tag in &self.tags {
            builder.feed_bytes(tag.as_bytes());
        }
        builder.feed_bytes(self.source.as_bytes());
        builder.build()
    }
}

/// Summary manifest for a backup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Backup metadata.
    pub metadata: BackupMetadata,
    /// Chunk positions included (sorted).
    pub positions: Vec<ChunkPos>,
    /// Per-chunk checksums (sorted by position for determinism).
    pub checksums: Vec<(ChunkPos, ChunkChecksum)>,
    /// Total non-air blocks across all chunks.
    pub total_non_air: u64,
    /// Overall backup fingerprint.
    pub fingerprint: SnapshotFingerprint,
}

impl BackupManifest {
    /// Create a manifest from entries.
    #[must_use]
    pub fn from_entries(metadata: BackupMetadata, entries: &[ChunkEntry]) -> Self {
        let mut positions: Vec<_> = entries.iter().map(|e| e.pos).collect();
        positions.sort_by_key(|p| (p.x(), p.y(), p.z()));

        let mut checksums: Vec<_> = entries.iter().map(|e| (e.pos, e.checksum)).collect();
        checksums.sort_by_key(|(p, _)| (p.x(), p.y(), p.z()));

        let total_non_air: u64 = entries.iter().map(|e| u64::from(e.non_air_count)).sum();

        let fingerprint = Self::compute_fingerprint(&metadata, entries);

        Self {
            metadata,
            positions,
            checksums,
            total_non_air,
            fingerprint,
        }
    }

    /// Compute the fingerprint for a manifest.
    fn compute_fingerprint(
        metadata: &BackupMetadata,
        entries: &[ChunkEntry],
    ) -> SnapshotFingerprint {
        let mut builder = FingerprintBuilder::new();

        let meta_fp = metadata.fingerprint();
        builder.feed_u32(meta_fp.as_u32());

        let mut sorted: Vec<_> = entries.iter().collect();
        sorted.sort_by_key(|e| (e.pos.x(), e.pos.y(), e.pos.z()));

        for entry in sorted {
            let entry_fp = entry.fingerprint();
            builder.feed_u32(entry_fp.as_u32());
        }

        builder.build()
    }

    /// Get chunk count.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.positions.len()
    }

    /// Check if a position is in this backup.
    #[must_use]
    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.checksums.iter().any(|(p, _)| *p == pos)
    }

    /// Get expected checksum for a position.
    #[must_use]
    pub fn get_checksum(&self, pos: ChunkPos) -> Option<ChunkChecksum> {
        self.checksums
            .iter()
            .find(|(p, _)| *p == pos)
            .map(|(_, cs)| *cs)
    }
}

/// A complete regional backup with chunk data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionalBackup {
    /// Backup manifest.
    pub manifest: BackupManifest,
    /// Chunk entries (position -> entry).
    entries: HashMap<ChunkPos, ChunkEntry>,
}

impl RegionalBackup {
    /// Create a backup from chunks.
    #[must_use]
    pub fn create<S: BuildHasher>(
        metadata: BackupMetadata,
        positions: &[ChunkPos],
        chunks: &HashMap<ChunkPos, Chunk, S>,
    ) -> (Self, Vec<BackupIssue>) {
        let mut entries = HashMap::new();
        let mut issues = Vec::new();

        for &pos in positions {
            if let Some(chunk) = chunks.get(&pos) {
                entries.insert(pos, ChunkEntry::from_chunk(pos, chunk));
            } else {
                issues.push(
                    BackupIssue::new(
                        BackupIssueKind::MissingChunk,
                        BackupIssueSeverity::Warning,
                        pos,
                    )
                    .with_description("chunk not found during backup"),
                );
            }
        }

        let entry_vec: Vec<_> = entries.values().cloned().collect();
        let manifest = BackupManifest::from_entries(metadata, &entry_vec);

        (Self { manifest, entries }, issues)
    }

    /// Create a backup from an iterator of (position, chunk) pairs.
    #[must_use]
    pub fn from_chunks<I>(metadata: BackupMetadata, chunks: I) -> Self
    where
        I: IntoIterator<Item = (ChunkPos, Chunk)>,
    {
        let entries: HashMap<_, _> = chunks
            .into_iter()
            .map(|(pos, chunk)| (pos, ChunkEntry::from_chunk(pos, &chunk)))
            .collect();

        let entry_vec: Vec<_> = entries.values().cloned().collect();
        let manifest = BackupManifest::from_entries(metadata, &entry_vec);

        Self { manifest, entries }
    }

    /// Get chunk entry by position.
    #[must_use]
    pub fn get_entry(&self, pos: ChunkPos) -> Option<&ChunkEntry> {
        self.entries.get(&pos)
    }

    /// Get chunk data by position.
    #[must_use]
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<Chunk> {
        self.entries.get(&pos).and_then(ChunkEntry::to_chunk)
    }

    /// Iterate over all entries.
    pub fn entries(&self) -> impl Iterator<Item = &ChunkEntry> {
        self.entries.values()
    }

    /// Get all positions in the backup.
    pub fn positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.entries.keys().copied()
    }

    /// Select a subset of the backup by positions.
    #[must_use]
    pub fn select_subset(&self, positions: &[ChunkPos]) -> (Self, Vec<BackupIssue>) {
        let mut subset_entries = HashMap::new();
        let mut issues = Vec::new();

        for &pos in positions {
            if let Some(entry) = self.entries.get(&pos) {
                subset_entries.insert(pos, entry.clone());
            } else {
                issues.push(
                    BackupIssue::new(
                        BackupIssueKind::MissingChunk,
                        BackupIssueSeverity::Warning,
                        pos,
                    )
                    .with_description("position not found in backup"),
                );
            }
        }

        let subset_meta = BackupMetadata::new(
            self.manifest.metadata.id,
            self.manifest.metadata.tick,
            format!("{}_subset", self.manifest.metadata.label),
        )
        .with_source(self.manifest.metadata.source.clone());

        let entry_vec: Vec<_> = subset_entries.values().cloned().collect();
        let manifest = BackupManifest::from_entries(subset_meta, &entry_vec);

        (
            Self {
                manifest,
                entries: subset_entries,
            },
            issues,
        )
    }

    /// Verify all checksums in the backup.
    #[must_use]
    pub fn verify_checksums(&self) -> Vec<BackupIssue> {
        let mut issues = Vec::new();

        for entry in self.entries.values() {
            if !entry.verify_checksum() {
                issues.push(
                    BackupIssue::new(
                        BackupIssueKind::ChecksumMismatch,
                        BackupIssueSeverity::Error,
                        entry.pos,
                    )
                    .with_description("stored checksum does not match chunk data"),
                );
            }
        }

        issues
    }

    /// Verify the manifest fingerprint.
    #[must_use]
    pub fn verify_manifest(&self) -> bool {
        let entry_vec: Vec<_> = self.entries.values().cloned().collect();
        let expected = BackupManifest::compute_fingerprint(&self.manifest.metadata, &entry_vec);
        expected.matches(self.manifest.fingerprint)
    }

    /// Compute total backup size in bytes.
    #[must_use]
    pub fn total_size(&self) -> usize {
        self.entries.values().map(|e| e.data.len()).sum()
    }
}

/// A single restore operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreOp {
    /// Replace entire chunk with backup data.
    ReplaceChunk {
        pos: ChunkPos,
        backup_checksum: ChunkChecksum,
    },
    /// Apply delta to restore specific blocks.
    ApplyDelta {
        pos: ChunkPos,
        delta: ChunkDelta,
        expected_checksum: ChunkChecksum,
    },
    /// Skip restoration (chunk already matches).
    Skip { pos: ChunkPos, reason: String },
}

impl RestoreOp {
    /// Get the affected chunk position.
    #[must_use]
    pub fn chunk_pos(&self) -> ChunkPos {
        match self {
            Self::ReplaceChunk { pos, .. }
            | Self::ApplyDelta { pos, .. }
            | Self::Skip { pos, .. } => *pos,
        }
    }

    /// Check if this operation modifies the chunk.
    #[must_use]
    pub fn is_modification(&self) -> bool {
        !matches!(self, Self::Skip { .. })
    }
}

/// A restore plan with operations and detected issues.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RestorePlan {
    /// Backup being restored from.
    pub backup_id: Option<BackupId>,
    /// Planned operations (sorted by position).
    pub operations: Vec<RestoreOp>,
    /// Issues detected during planning.
    pub issues: Vec<BackupIssue>,
    /// Maximum chunks to modify (bound).
    pub max_modifications: usize,
    /// Whether the plan is within modification bounds.
    pub within_bounds: bool,
}

impl RestorePlan {
    /// Create a new empty restore plan.
    #[must_use]
    pub fn new(max_modifications: usize) -> Self {
        Self {
            backup_id: None,
            operations: Vec::new(),
            issues: Vec::new(),
            max_modifications,
            within_bounds: true,
        }
    }

    /// Create a restore plan from a backup and current chunks.
    #[must_use]
    pub fn from_backup<S: BuildHasher>(
        backup: &RegionalBackup,
        current_chunks: &HashMap<ChunkPos, Chunk, S>,
        max_modifications: usize,
    ) -> Self {
        let mut plan = Self::new(max_modifications);
        plan.backup_id = Some(backup.manifest.metadata.id);

        let mut modification_count = 0;

        for entry in backup.entries() {
            if let Some(current) = current_chunks.get(&entry.pos) {
                let current_checksum = ChunkChecksum::compute(current);

                if current_checksum.matches(entry.checksum) {
                    plan.operations.push(RestoreOp::Skip {
                        pos: entry.pos,
                        reason: String::from("chunk already matches backup"),
                    });
                } else {
                    if modification_count >= max_modifications {
                        plan.within_bounds = false;
                        plan.issues.push(
                            BackupIssue::new(
                                BackupIssueKind::OutOfBounds,
                                BackupIssueSeverity::Warning,
                                entry.pos,
                            )
                            .with_description("modification limit exceeded"),
                        );
                        continue;
                    }

                    plan.operations.push(RestoreOp::ReplaceChunk {
                        pos: entry.pos,
                        backup_checksum: entry.checksum,
                    });

                    plan.issues.push(
                        BackupIssue::new(
                            BackupIssueKind::StaleChunk,
                            BackupIssueSeverity::Info,
                            entry.pos,
                        )
                        .with_description("chunk modified since backup")
                        .with_values(
                            format!("{:08x}", entry.checksum.value()),
                            format!("{:08x}", current_checksum.value()),
                        ),
                    );

                    modification_count += 1;
                }
            } else {
                if modification_count >= max_modifications {
                    plan.within_bounds = false;
                    continue;
                }

                plan.operations.push(RestoreOp::ReplaceChunk {
                    pos: entry.pos,
                    backup_checksum: entry.checksum,
                });

                plan.issues.push(
                    BackupIssue::new(
                        BackupIssueKind::MissingChunk,
                        BackupIssueSeverity::Info,
                        entry.pos,
                    )
                    .with_description("chunk missing in current world, will be created"),
                );

                modification_count += 1;
            }
        }

        plan.operations
            .sort_by_key(|op| (op.chunk_pos().x(), op.chunk_pos().y(), op.chunk_pos().z()));

        plan
    }

    /// Get count of modifications.
    #[must_use]
    pub fn modification_count(&self) -> usize {
        self.operations
            .iter()
            .filter(|op| op.is_modification())
            .count()
    }

    /// Get count of skipped chunks.
    #[must_use]
    pub fn skip_count(&self) -> usize {
        self.operations
            .iter()
            .filter(|op| !op.is_modification())
            .count()
    }

    /// Check if any operations exist.
    #[must_use]
    pub fn has_operations(&self) -> bool {
        self.operations.iter().any(RestoreOp::is_modification)
    }

    /// Check if the plan has blocking issues.
    #[must_use]
    pub fn has_blocking_issues(&self) -> bool {
        self.issues.iter().any(BackupIssue::blocks_restore)
    }

    /// Get issues by kind.
    #[must_use]
    pub fn issues_by_kind(&self) -> BTreeMap<BackupIssueKind, usize> {
        let mut counts = BTreeMap::new();
        for issue in &self.issues {
            *counts.entry(issue.kind).or_insert(0) += 1;
        }
        counts
    }
}

/// Result of applying a restore plan.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Number of chunks replaced.
    pub chunks_replaced: usize,
    /// Number of deltas applied.
    pub deltas_applied: usize,
    /// Number of chunks skipped.
    pub chunks_skipped: usize,
    /// Operations that failed.
    pub operations_failed: usize,
    /// Errors encountered.
    pub errors: Vec<String>,
    /// Post-restore checksum verification results.
    pub verification_passed: usize,
    /// Post-restore checksum failures.
    pub verification_failed: usize,
}

impl RestoreResult {
    /// Check if restore was fully successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.operations_failed == 0 && self.errors.is_empty() && self.verification_failed == 0
    }

    /// Check if any restores were applied.
    #[must_use]
    pub fn any_applied(&self) -> bool {
        self.chunks_replaced > 0 || self.deltas_applied > 0
    }

    /// Total operations attempted.
    #[must_use]
    pub fn total_operations(&self) -> usize {
        self.chunks_replaced + self.deltas_applied + self.chunks_skipped + self.operations_failed
    }
}

/// Apply a restore plan to mutable chunks.
pub fn apply_restore<S: BuildHasher>(
    backup: &RegionalBackup,
    plan: &RestorePlan,
    chunks: &mut HashMap<ChunkPos, Chunk, S>,
) -> RestoreResult {
    let mut result = RestoreResult::default();

    for op in &plan.operations {
        match op {
            RestoreOp::ReplaceChunk {
                pos,
                backup_checksum,
            } => {
                if let Some(entry) = backup.get_entry(*pos) {
                    if let Some(chunk) = entry.to_chunk() {
                        chunks.insert(*pos, chunk);
                        result.chunks_replaced += 1;

                        if let Some(restored) = chunks.get(pos) {
                            let restored_checksum = ChunkChecksum::compute(restored);
                            if restored_checksum.matches(*backup_checksum) {
                                result.verification_passed += 1;
                            } else {
                                result.verification_failed += 1;
                                result
                                    .errors
                                    .push(format!("checksum mismatch after restore at {pos:?}"));
                            }
                        }
                    } else {
                        result.operations_failed += 1;
                        result
                            .errors
                            .push(format!("failed to deserialize backup chunk at {pos:?}"));
                    }
                } else {
                    result.operations_failed += 1;
                    result
                        .errors
                        .push(format!("chunk not found in backup at {pos:?}"));
                }
            }
            RestoreOp::ApplyDelta {
                pos,
                delta,
                expected_checksum,
            } => {
                if let Some(chunk) = chunks.get_mut(pos) {
                    for (local_pos, block) in delta.iter() {
                        chunk.set(local_pos, block);
                    }
                    result.deltas_applied += 1;

                    let restored_checksum = ChunkChecksum::compute(chunk);
                    if restored_checksum.matches(*expected_checksum) {
                        result.verification_passed += 1;
                    } else {
                        result.verification_failed += 1;
                    }
                } else {
                    result.operations_failed += 1;
                    result
                        .errors
                        .push(format!("chunk not found for delta apply at {pos:?}"));
                }
            }
            RestoreOp::Skip { .. } => {
                result.chunks_skipped += 1;
            }
        }
    }

    result
}

/// Verify current chunks match backup checksums.
#[must_use]
pub fn verify_against_backup<S: BuildHasher>(
    backup: &RegionalBackup,
    chunks: &HashMap<ChunkPos, Chunk, S>,
) -> Vec<BackupIssue> {
    let mut issues = Vec::new();

    for (pos, expected_checksum) in &backup.manifest.checksums {
        if let Some(chunk) = chunks.get(pos) {
            let actual = ChunkChecksum::compute(chunk);
            if !actual.matches(*expected_checksum) {
                issues.push(
                    BackupIssue::new(
                        BackupIssueKind::ChecksumMismatch,
                        BackupIssueSeverity::Info,
                        *pos,
                    )
                    .with_description("chunk differs from backup")
                    .with_values(
                        format!("{:08x}", expected_checksum.value()),
                        format!("{:08x}", actual.value()),
                    ),
                );
            }
        } else {
            issues.push(
                BackupIssue::new(
                    BackupIssueKind::MissingChunk,
                    BackupIssueSeverity::Warning,
                    *pos,
                )
                .with_description("chunk from backup is missing in current world"),
            );
        }
    }

    issues.sort_by_key(|i| (i.chunk_pos.x(), i.chunk_pos.y(), i.chunk_pos.z()));
    issues
}

/// Compute delta required to transform current chunk to backup state.
#[must_use]
pub fn compute_restore_delta(backup_chunk: &Chunk, current_chunk: &Chunk) -> ChunkDelta {
    let mut delta = ChunkDelta::new();

    for (pos, backup_block) in backup_chunk.iter() {
        let current_block = current_chunk.get(pos);
        if backup_block != current_block {
            delta.set(pos, backup_block);
        }
    }

    delta
}

/// Summary statistics for a backup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSummary {
    /// Number of chunks in backup.
    pub chunk_count: usize,
    /// Total non-air blocks.
    pub total_non_air: u64,
    /// Total data size in bytes.
    pub total_bytes: usize,
    /// Backup fingerprint.
    pub fingerprint: SnapshotFingerprint,
    /// Bounding box min (if any chunks).
    pub bounds_min: Option<ChunkPos>,
    /// Bounding box max (if any chunks).
    pub bounds_max: Option<ChunkPos>,
}

impl BackupSummary {
    /// Compute summary from a backup.
    #[must_use]
    pub fn from_backup(backup: &RegionalBackup) -> Self {
        let chunk_count = backup.manifest.chunk_count();
        let total_non_air = backup.manifest.total_non_air;
        let total_bytes = backup.total_size();
        let fingerprint = backup.manifest.fingerprint;

        let (bounds_min, bounds_max) = if backup.manifest.positions.is_empty() {
            (None, None)
        } else {
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut min_z = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            let mut max_z = i32::MIN;

            for pos in &backup.manifest.positions {
                min_x = min_x.min(pos.x());
                min_y = min_y.min(pos.y());
                min_z = min_z.min(pos.z());
                max_x = max_x.max(pos.x());
                max_y = max_y.max(pos.y());
                max_z = max_z.max(pos.z());
            }

            (
                Some(ChunkPos::new(min_x, min_y, min_z)),
                Some(ChunkPos::new(max_x, max_y, max_z)),
            )
        };

        Self {
            chunk_count,
            total_non_air,
            total_bytes,
            fingerprint,
            bounds_min,
            bounds_max,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{AIR, BlockId, STONE};
    use engine_core::coords::LocalPos;

    fn test_chunk() -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(1, 0, 0), STONE);
        chunk.set(LocalPos::new(0, 1, 0), BlockId(100));
        chunk
    }

    fn test_chunk_modified() -> Chunk {
        let mut chunk = test_chunk();
        chunk.set(LocalPos::new(5, 5, 5), STONE);
        chunk
    }

    #[test]
    fn test_backup_id_deterministic() {
        let id1 = BackupId::generate(12345, 1);
        let id2 = BackupId::generate(12345, 1);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_backup_id_differs_by_seed() {
        let id1 = BackupId::generate(12345, 1);
        let id2 = BackupId::generate(54321, 1);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_backup_id_differs_by_sequence() {
        let id1 = BackupId::generate(12345, 1);
        let id2 = BackupId::generate(12345, 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_chunk_entry_roundtrip() {
        let chunk = test_chunk();
        let entry = ChunkEntry::from_chunk(ChunkPos::new(0, 0, 0), &chunk);

        let recovered = entry.to_chunk().expect("deserialization should succeed");
        assert_eq!(chunk.non_air_count(), recovered.non_air_count());
        assert_eq!(
            chunk.get(LocalPos::new(0, 0, 0)),
            recovered.get(LocalPos::new(0, 0, 0))
        );
    }

    #[test]
    fn test_chunk_entry_checksum_verification() {
        let chunk = test_chunk();
        let entry = ChunkEntry::from_chunk(ChunkPos::new(0, 0, 0), &chunk);
        assert!(entry.verify_checksum());
    }

    #[test]
    fn test_chunk_entry_fingerprint_deterministic() {
        let chunk = test_chunk();
        let entry = ChunkEntry::from_chunk(ChunkPos::new(0, 0, 0), &chunk);
        let fp1 = entry.fingerprint();
        let fp2 = entry.fingerprint();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_backup_metadata_fingerprint_stable() {
        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test")
            .with_description("a description")
            .with_tag("tag1");

        let fp1 = meta.fingerprint();
        let fp2 = meta.fingerprint();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_regional_backup_create() {
        let mut chunks = HashMap::new();
        let pos1 = ChunkPos::new(0, 0, 0);
        let pos2 = ChunkPos::new(1, 0, 0);
        chunks.insert(pos1, test_chunk());
        chunks.insert(pos2, Chunk::new());

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let (backup, issues) = RegionalBackup::create(meta, &[pos1, pos2], &chunks);

        assert_eq!(backup.manifest.chunk_count(), 2);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_regional_backup_missing_chunk() {
        let chunks: HashMap<ChunkPos, Chunk> = HashMap::new();
        let pos = ChunkPos::new(0, 0, 0);

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let (backup, issues) = RegionalBackup::create(meta, &[pos], &chunks);

        assert_eq!(backup.manifest.chunk_count(), 0);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, BackupIssueKind::MissingChunk);
    }

    #[test]
    fn test_regional_backup_from_chunks() {
        let chunks = vec![
            (ChunkPos::new(0, 0, 0), test_chunk()),
            (ChunkPos::new(1, 0, 0), Chunk::new()),
        ];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        assert_eq!(backup.manifest.chunk_count(), 2);
    }

    #[test]
    fn test_regional_backup_select_subset() {
        let chunks = vec![
            (ChunkPos::new(0, 0, 0), test_chunk()),
            (ChunkPos::new(1, 0, 0), Chunk::new()),
            (ChunkPos::new(2, 0, 0), test_chunk_modified()),
        ];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "full");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let (subset, issues) =
            backup.select_subset(&[ChunkPos::new(0, 0, 0), ChunkPos::new(2, 0, 0)]);

        assert_eq!(subset.manifest.chunk_count(), 2);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_regional_backup_select_subset_missing() {
        let chunks = vec![(ChunkPos::new(0, 0, 0), test_chunk())];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "full");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let (subset, issues) =
            backup.select_subset(&[ChunkPos::new(0, 0, 0), ChunkPos::new(99, 0, 0)]);

        assert_eq!(subset.manifest.chunk_count(), 1);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, BackupIssueKind::MissingChunk);
    }

    #[test]
    fn test_regional_backup_verify_checksums() {
        let chunks = vec![(ChunkPos::new(0, 0, 0), test_chunk())];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let issues = backup.verify_checksums();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_regional_backup_verify_manifest() {
        let chunks = vec![(ChunkPos::new(0, 0, 0), test_chunk())];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        assert!(backup.verify_manifest());
    }

    #[test]
    fn test_restore_plan_identical_chunks() {
        let chunk = test_chunk();
        let mut chunks = HashMap::new();
        let pos = ChunkPos::new(0, 0, 0);
        chunks.insert(pos, chunk.clone());

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, chunk)]);

        let plan = RestorePlan::from_backup(&backup, &chunks, 100);

        assert_eq!(plan.modification_count(), 0);
        assert_eq!(plan.skip_count(), 1);
    }

    #[test]
    fn test_restore_plan_modified_chunk() {
        let pos = ChunkPos::new(0, 0, 0);

        let backup_chunk = test_chunk();
        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, backup_chunk)]);

        let mut current_chunks = HashMap::new();
        current_chunks.insert(pos, test_chunk_modified());

        let plan = RestorePlan::from_backup(&backup, &current_chunks, 100);

        assert_eq!(plan.modification_count(), 1);
        assert!(plan.has_operations());
    }

    #[test]
    fn test_restore_plan_missing_chunk() {
        let pos = ChunkPos::new(0, 0, 0);

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, test_chunk())]);

        let current_chunks: HashMap<ChunkPos, Chunk> = HashMap::new();

        let plan = RestorePlan::from_backup(&backup, &current_chunks, 100);

        assert_eq!(plan.modification_count(), 1);
        assert!(
            plan.issues
                .iter()
                .any(|i| i.kind == BackupIssueKind::MissingChunk)
        );
    }

    #[test]
    fn test_restore_plan_bounded() {
        let positions: Vec<_> = (0..10).map(|i| ChunkPos::new(i, 0, 0)).collect();
        let chunks: Vec<_> = positions.iter().map(|&p| (p, test_chunk())).collect();

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let current_chunks: HashMap<ChunkPos, Chunk> = HashMap::new();

        let plan = RestorePlan::from_backup(&backup, &current_chunks, 5);

        assert!(!plan.within_bounds);
        assert_eq!(plan.modification_count(), 5);
    }

    #[test]
    fn test_apply_restore() {
        let pos = ChunkPos::new(0, 0, 0);

        let backup_chunk = test_chunk();
        let backup_checksum = ChunkChecksum::compute(&backup_chunk);
        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, backup_chunk)]);

        let mut current_chunks = HashMap::new();
        current_chunks.insert(pos, test_chunk_modified());

        let plan = RestorePlan::from_backup(&backup, &current_chunks, 100);
        let result = apply_restore(&backup, &plan, &mut current_chunks);

        assert!(result.is_success());
        assert_eq!(result.chunks_replaced, 1);
        assert_eq!(result.verification_passed, 1);

        let restored = current_chunks.get(&pos).unwrap();
        assert_eq!(ChunkChecksum::compute(restored), backup_checksum);
    }

    #[test]
    fn test_apply_restore_creates_missing() {
        let pos = ChunkPos::new(0, 0, 0);

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, test_chunk())]);

        let mut current_chunks: HashMap<ChunkPos, Chunk> = HashMap::new();

        let plan = RestorePlan::from_backup(&backup, &current_chunks, 100);
        let result = apply_restore(&backup, &plan, &mut current_chunks);

        assert!(result.is_success());
        assert_eq!(result.chunks_replaced, 1);
        assert!(current_chunks.contains_key(&pos));
    }

    #[test]
    fn test_verify_against_backup() {
        let pos = ChunkPos::new(0, 0, 0);

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, test_chunk())]);

        let mut current_chunks = HashMap::new();
        current_chunks.insert(pos, test_chunk());

        let issues = verify_against_backup(&backup, &current_chunks);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_verify_against_backup_modified() {
        let pos = ChunkPos::new(0, 0, 0);

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, test_chunk())]);

        let mut current_chunks = HashMap::new();
        current_chunks.insert(pos, test_chunk_modified());

        let issues = verify_against_backup(&backup, &current_chunks);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, BackupIssueKind::ChecksumMismatch);
    }

    #[test]
    fn test_verify_against_backup_missing() {
        let pos = ChunkPos::new(0, 0, 0);

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, vec![(pos, test_chunk())]);

        let current_chunks: HashMap<ChunkPos, Chunk> = HashMap::new();

        let issues = verify_against_backup(&backup, &current_chunks);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, BackupIssueKind::MissingChunk);
    }

    #[test]
    fn test_compute_restore_delta() {
        let backup_chunk = test_chunk();
        let current_chunk = test_chunk_modified();

        let delta = compute_restore_delta(&backup_chunk, &current_chunk);

        assert_eq!(delta.len(), 1);
        assert_eq!(delta.get(LocalPos::new(5, 5, 5)), Some(AIR));
    }

    #[test]
    fn test_backup_summary() {
        let chunks = vec![
            (ChunkPos::new(0, 0, 0), test_chunk()),
            (ChunkPos::new(2, 1, 3), Chunk::new()),
        ];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let summary = BackupSummary::from_backup(&backup);

        assert_eq!(summary.chunk_count, 2);
        assert_eq!(summary.bounds_min, Some(ChunkPos::new(0, 0, 0)));
        assert_eq!(summary.bounds_max, Some(ChunkPos::new(2, 1, 3)));
    }

    #[test]
    fn test_backup_issue_blocks_restore() {
        let warning = BackupIssue::new(
            BackupIssueKind::StaleChunk,
            BackupIssueSeverity::Warning,
            ChunkPos::new(0, 0, 0),
        );
        assert!(!warning.blocks_restore());

        let error = BackupIssue::new(
            BackupIssueKind::ChecksumMismatch,
            BackupIssueSeverity::Error,
            ChunkPos::new(0, 0, 0),
        );
        assert!(error.blocks_restore());
    }

    #[test]
    fn test_backup_manifest_fingerprint_stable() {
        let chunks = vec![
            (ChunkPos::new(0, 0, 0), test_chunk()),
            (ChunkPos::new(1, 0, 0), Chunk::new()),
        ];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let fp1 = backup.manifest.fingerprint;
        let fp2 = backup.manifest.fingerprint;
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_deterministic_ordering() {
        let positions: Vec<_> = vec![
            ChunkPos::new(2, 0, 0),
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(1, 0, 0),
        ];

        let chunks: Vec<_> = positions.iter().map(|&p| (p, test_chunk())).collect();

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        assert_eq!(backup.manifest.positions[0], ChunkPos::new(0, 0, 0));
        assert_eq!(backup.manifest.positions[1], ChunkPos::new(1, 0, 0));
        assert_eq!(backup.manifest.positions[2], ChunkPos::new(2, 0, 0));
    }

    #[test]
    fn test_serde_json_roundtrip_backup_id() {
        let id = BackupId::generate(12345, 67);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: BackupId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn test_serde_json_roundtrip_metadata() {
        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test")
            .with_description("desc")
            .with_tag("tag1")
            .with_source("test_source");

        let json = serde_json::to_string(&meta).unwrap();
        let recovered: BackupMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, recovered);
    }

    #[test]
    fn test_serde_json_roundtrip_manifest() {
        let chunks = vec![(ChunkPos::new(0, 0, 0), test_chunk())];
        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let json = serde_json::to_string(&backup.manifest).unwrap();
        let recovered: BackupManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(backup.manifest, recovered);
    }

    #[test]
    fn test_serde_json_roundtrip_issue() {
        let issue = BackupIssue::new(
            BackupIssueKind::ChecksumMismatch,
            BackupIssueSeverity::Error,
            ChunkPos::new(1, 2, 3),
        )
        .with_description("test issue")
        .with_values("expected", "actual");

        let json = serde_json::to_string(&issue).unwrap();
        let recovered: BackupIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(issue, recovered);
    }

    #[test]
    fn test_serde_json_roundtrip_restore_plan() {
        let plan = RestorePlan::new(100);
        let json = serde_json::to_string(&plan).unwrap();
        let recovered: RestorePlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan.max_modifications, recovered.max_modifications);
    }

    #[test]
    fn test_serde_json_roundtrip_restore_result() {
        let result = RestoreResult {
            chunks_replaced: 5,
            errors: vec![String::from("an error")],
            ..Default::default()
        };

        let json = serde_json::to_string(&result).unwrap();
        let recovered: RestoreResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.chunks_replaced, recovered.chunks_replaced);
        assert_eq!(result.errors, recovered.errors);
    }

    #[test]
    fn test_serde_bincode_roundtrip_backup() {
        let chunks = vec![
            (ChunkPos::new(0, 0, 0), test_chunk()),
            (ChunkPos::new(1, 0, 0), Chunk::new()),
        ];

        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "test");
        let backup = RegionalBackup::from_chunks(meta, chunks);

        let bytes = bincode::serialize(&backup).unwrap();
        let recovered: RegionalBackup = bincode::deserialize(&bytes).unwrap();

        assert_eq!(
            backup.manifest.chunk_count(),
            recovered.manifest.chunk_count()
        );
        assert_eq!(backup.manifest.fingerprint, recovered.manifest.fingerprint);
    }

    #[test]
    fn test_serde_bincode_roundtrip_restore_op() {
        let delta = {
            let mut d = ChunkDelta::new();
            d.set(LocalPos::new(0, 0, 0), STONE);
            d
        };

        let op = RestoreOp::ApplyDelta {
            pos: ChunkPos::new(1, 2, 3),
            delta,
            expected_checksum: ChunkChecksum::from_raw(0xDEAD_BEEF),
        };

        let bytes = bincode::serialize(&op).unwrap();
        let recovered: RestoreOp = bincode::deserialize(&bytes).unwrap();

        assert_eq!(op.chunk_pos(), recovered.chunk_pos());
    }

    #[test]
    fn test_backup_summary_empty() {
        let meta = BackupMetadata::new(BackupId::generate(1, 1), 100, "empty");
        let backup = RegionalBackup::from_chunks(meta, vec![]);

        let summary = BackupSummary::from_backup(&backup);

        assert_eq!(summary.chunk_count, 0);
        assert!(summary.bounds_min.is_none());
        assert!(summary.bounds_max.is_none());
    }

    #[test]
    fn test_issue_kind_recoverable() {
        assert!(BackupIssueKind::StaleChunk.is_recoverable());
        assert!(BackupIssueKind::MissingChunk.is_recoverable());
        assert!(!BackupIssueKind::ChecksumMismatch.is_recoverable());
        assert!(!BackupIssueKind::CorruptManifest.is_recoverable());
    }

    #[test]
    fn test_restore_op_is_modification() {
        let replace = RestoreOp::ReplaceChunk {
            pos: ChunkPos::new(0, 0, 0),
            backup_checksum: ChunkChecksum::from_raw(0),
        };
        assert!(replace.is_modification());

        let skip = RestoreOp::Skip {
            pos: ChunkPos::new(0, 0, 0),
            reason: String::from("test"),
        };
        assert!(!skip.is_modification());
    }

    #[test]
    fn test_restore_result_success_states() {
        let success = RestoreResult {
            chunks_replaced: 1,
            verification_passed: 1,
            ..Default::default()
        };
        assert!(success.is_success());
        assert!(success.any_applied());

        let failed = RestoreResult {
            operations_failed: 1,
            errors: vec![String::from("error")],
            ..Default::default()
        };
        assert!(!failed.is_success());
        assert!(!failed.any_applied());

        let verification_failed = RestoreResult {
            chunks_replaced: 1,
            verification_failed: 1,
            ..Default::default()
        };
        assert!(!verification_failed.is_success());
    }
}

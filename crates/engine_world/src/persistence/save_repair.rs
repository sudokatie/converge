//! Savegame diff/repair tooling for snapshot comparison and recovery.
//!
//! Provides deterministic tooling for comparing save/world/chunk snapshots,
//! detecting repairable issues, planning bounded repairs, and applying fixes.
//!
//! # Overview
//!
//! - [`SnapshotFingerprint`]: Stable fingerprint for world state comparison
//! - [`ChunkChecksum`]: CRC32 checksum for individual chunks
//! - [`SnapshotDiff`]: Detected differences between snapshots
//! - [`RepairIssue`]: Categorized repairable issues
//! - [`RepairPlan`]: Bounded repair operations
//! - [`RepairResult`]: Outcome of applied repairs

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::BuildHasher;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::chunk::{AIR, BlockId, CHUNK_VOLUME, Chunk};
use crate::persistence::{ChunkDelta, DeltaIndex, WorldMeta};

/// Stable fingerprint for snapshot comparison.
///
/// Uses CRC32 internally for deterministic, order-sensitive hashing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotFingerprint {
    hash: u32,
}

impl SnapshotFingerprint {
    /// Create an empty fingerprint.
    #[must_use]
    pub const fn new() -> Self {
        Self { hash: 0 }
    }

    /// Create from a raw hash value.
    #[must_use]
    pub const fn from_raw(hash: u32) -> Self {
        Self { hash }
    }

    /// Get the raw hash value.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.hash
    }

    /// Check if two fingerprints match.
    #[must_use]
    pub const fn matches(self, other: Self) -> bool {
        self.hash == other.hash
    }

    /// Combine two fingerprints (order-dependent).
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.hash.to_le_bytes());
        hasher.update(&other.hash.to_le_bytes());
        Self {
            hash: hasher.finalize(),
        }
    }
}

/// Builder for constructing snapshot fingerprints.
#[derive(Debug)]
pub struct FingerprintBuilder {
    hasher: crc32fast::Hasher,
}

impl FingerprintBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hasher: crc32fast::Hasher::new(),
        }
    }

    /// Feed a u64 value.
    pub fn feed_u64(&mut self, value: u64) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a u32 value.
    pub fn feed_u32(&mut self, value: u32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed an i32 value.
    pub fn feed_i32(&mut self, value: i32) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed a u16 value.
    pub fn feed_u16(&mut self, value: u16) -> &mut Self {
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Feed raw bytes.
    pub fn feed_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.hasher.update(bytes);
        self
    }

    /// Feed a chunk position.
    pub fn feed_chunk_pos(&mut self, pos: ChunkPos) -> &mut Self {
        self.feed_i32(pos.x()).feed_i32(pos.y()).feed_i32(pos.z())
    }

    /// Finalize and produce the fingerprint.
    #[must_use]
    pub fn build(self) -> SnapshotFingerprint {
        SnapshotFingerprint {
            hash: self.hasher.finalize(),
        }
    }

    /// Finalize and reset for reuse.
    #[must_use]
    pub fn finish(&mut self) -> SnapshotFingerprint {
        let value = self.hasher.clone().finalize();
        self.hasher = crc32fast::Hasher::new();
        SnapshotFingerprint { hash: value }
    }
}

impl Default for FingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// CRC32 checksum for an individual chunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkChecksum {
    value: u32,
}

impl ChunkChecksum {
    /// Compute checksum for a chunk.
    #[must_use]
    pub fn compute(chunk: &Chunk) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        for block in chunk.blocks() {
            hasher.update(&block.0.to_le_bytes());
        }
        Self {
            value: hasher.finalize(),
        }
    }

    /// Create from a raw value.
    #[must_use]
    pub const fn from_raw(value: u32) -> Self {
        Self { value }
    }

    /// Get the raw checksum value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }

    /// Check if checksums match.
    #[must_use]
    pub const fn matches(self, other: Self) -> bool {
        self.value == other.value
    }
}

/// Category of detected repair issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IssueCategory {
    /// Chunk is missing but expected to exist.
    MissingChunk = 0,
    /// Chunk exists but has corrupted data.
    CorruptChunk = 1,
    /// Block count mismatch (`non_air_count` is wrong).
    BlockCountMismatch = 2,
    /// Unexpected air blocks in solid region.
    UnexpectedAir = 3,
    /// Orphaned blocks (floating without support).
    OrphanedBlocks = 4,
    /// Invalid block ID (out of registry range).
    InvalidBlockId = 5,
    /// Chunk checksum mismatch between regions.
    ChecksumMismatch = 6,
}

impl IssueCategory {
    /// Get the display name for this category.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MissingChunk => "missing_chunk",
            Self::CorruptChunk => "corrupt_chunk",
            Self::BlockCountMismatch => "block_count_mismatch",
            Self::UnexpectedAir => "unexpected_air",
            Self::OrphanedBlocks => "orphaned_blocks",
            Self::InvalidBlockId => "invalid_block_id",
            Self::ChecksumMismatch => "checksum_mismatch",
        }
    }

    /// Check if this issue is safely repairable.
    #[must_use]
    pub const fn is_safe_repair(self) -> bool {
        matches!(
            self,
            Self::BlockCountMismatch | Self::InvalidBlockId | Self::ChecksumMismatch
        )
    }
}

/// Severity level for repair issues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IssueSeverity {
    /// Informational, no action required.
    Info = 0,
    /// Warning, repair recommended.
    Warning = 1,
    /// Error, repair required for save integrity.
    Error = 2,
    /// Critical, data loss may have occurred.
    Critical = 3,
}

/// A detected repairable issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairIssue {
    /// Issue category.
    pub category: IssueCategory,
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Affected chunk position.
    pub chunk_pos: ChunkPos,
    /// Optional local position within chunk.
    pub local_pos: Option<LocalPos>,
    /// Human-readable description.
    pub description: String,
    /// Expected value (if applicable).
    pub expected: Option<String>,
    /// Actual value (if applicable).
    pub actual: Option<String>,
}

impl RepairIssue {
    /// Create a new repair issue.
    #[must_use]
    pub fn new(category: IssueCategory, severity: IssueSeverity, chunk_pos: ChunkPos) -> Self {
        Self {
            category,
            severity,
            chunk_pos,
            local_pos: None,
            description: String::new(),
            expected: None,
            actual: None,
        }
    }

    /// Set the local position.
    #[must_use]
    pub fn with_local_pos(mut self, pos: LocalPos) -> Self {
        self.local_pos = Some(pos);
        self
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

    /// Check if this issue is safely repairable.
    #[must_use]
    pub fn is_safe_repair(&self) -> bool {
        self.category.is_safe_repair()
    }
}

/// A single repair operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairOp {
    /// Recalculate chunk's non-air count.
    RecalculateCount { chunk_pos: ChunkPos },
    /// Replace invalid block with air.
    ReplaceInvalidBlock {
        chunk_pos: ChunkPos,
        local_pos: LocalPos,
        old_block: BlockId,
    },
    /// Apply a chunk delta as repair.
    ApplyDelta {
        chunk_pos: ChunkPos,
        delta: ChunkDelta,
    },
    /// Regenerate chunk from seed.
    RegenerateChunk { chunk_pos: ChunkPos },
}

impl RepairOp {
    /// Get the affected chunk position.
    #[must_use]
    pub fn chunk_pos(&self) -> ChunkPos {
        match self {
            Self::RecalculateCount { chunk_pos }
            | Self::ReplaceInvalidBlock { chunk_pos, .. }
            | Self::ApplyDelta { chunk_pos, .. }
            | Self::RegenerateChunk { chunk_pos } => *chunk_pos,
        }
    }

    /// Estimate the cost of this repair (number of blocks affected).
    #[must_use]
    pub fn estimated_cost(&self) -> usize {
        match self {
            Self::RecalculateCount { .. } => 0,
            Self::ReplaceInvalidBlock { .. } => 1,
            Self::ApplyDelta { delta, .. } => delta.len(),
            Self::RegenerateChunk { .. } => CHUNK_VOLUME,
        }
    }
}

/// A bounded repair plan.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepairPlan {
    /// Issues detected.
    pub issues: Vec<RepairIssue>,
    /// Planned repair operations.
    pub operations: Vec<RepairOp>,
    /// Maximum blocks to modify (bound).
    pub max_modifications: usize,
    /// Whether the plan is within bounds.
    pub within_bounds: bool,
}

impl RepairPlan {
    /// Create a new empty repair plan.
    #[must_use]
    pub fn new(max_modifications: usize) -> Self {
        Self {
            issues: Vec::new(),
            operations: Vec::new(),
            max_modifications,
            within_bounds: true,
        }
    }

    /// Add an issue to the plan.
    pub fn add_issue(&mut self, issue: RepairIssue) {
        self.issues.push(issue);
    }

    /// Add a repair operation if within bounds.
    pub fn add_operation(&mut self, op: RepairOp) -> bool {
        let cost = op.estimated_cost();
        let current_cost: usize = self.operations.iter().map(RepairOp::estimated_cost).sum();

        if current_cost + cost <= self.max_modifications {
            self.operations.push(op);
            true
        } else {
            self.within_bounds = false;
            false
        }
    }

    /// Get total estimated modifications.
    #[must_use]
    pub fn total_modifications(&self) -> usize {
        self.operations.iter().map(RepairOp::estimated_cost).sum()
    }

    /// Get count of safe repairs.
    #[must_use]
    pub fn safe_repair_count(&self) -> usize {
        self.issues.iter().filter(|i| i.is_safe_repair()).count()
    }

    /// Get count of issues by category.
    #[must_use]
    pub fn issues_by_category(&self) -> BTreeMap<IssueCategory, usize> {
        let mut counts = BTreeMap::new();
        for issue in &self.issues {
            *counts.entry(issue.category).or_insert(0) += 1;
        }
        counts
    }

    /// Check if the plan has any operations.
    #[must_use]
    pub fn has_operations(&self) -> bool {
        !self.operations.is_empty()
    }

    /// Check if all issues are addressed.
    #[must_use]
    pub fn all_addressed(&self) -> bool {
        self.within_bounds && self.issues.len() == self.operations.len()
    }
}

/// Result of applying a repair plan.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepairResult {
    /// Number of chunks modified.
    pub chunks_modified: usize,
    /// Number of blocks changed.
    pub blocks_changed: usize,
    /// Operations successfully applied.
    pub operations_applied: usize,
    /// Operations that failed.
    pub operations_failed: usize,
    /// Errors encountered.
    pub errors: Vec<String>,
}

impl RepairResult {
    /// Check if repair was fully successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.operations_failed == 0 && self.errors.is_empty()
    }

    /// Check if any repairs were applied.
    #[must_use]
    pub fn any_applied(&self) -> bool {
        self.operations_applied > 0
    }
}

/// Difference between two chunk states.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDiff {
    /// Position of the chunk.
    pub chunk_pos: ChunkPos,
    /// Block changes (local index -> (old, new)).
    pub changes: BTreeMap<DeltaIndex, (BlockId, BlockId)>,
    /// Checksum of source chunk.
    pub source_checksum: ChunkChecksum,
    /// Checksum of target chunk.
    pub target_checksum: ChunkChecksum,
}

impl ChunkDiff {
    /// Compute diff between two chunks.
    #[must_use]
    pub fn compute(chunk_pos: ChunkPos, source: &Chunk, target: &Chunk) -> Self {
        let mut changes = BTreeMap::new();

        for (pos, source_block) in source.iter() {
            let target_block = target.get(pos);
            if source_block != target_block {
                changes.insert(
                    DeltaIndex::from_local_pos(pos),
                    (source_block, target_block),
                );
            }
        }

        Self {
            chunk_pos,
            changes,
            source_checksum: ChunkChecksum::compute(source),
            target_checksum: ChunkChecksum::compute(target),
        }
    }

    /// Check if there are no differences.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Get the number of changed blocks.
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// Convert to a forward delta (source -> target).
    #[must_use]
    pub fn to_forward_delta(&self) -> ChunkDelta {
        let mut delta = ChunkDelta::new();
        for (&idx, &(_, new)) in &self.changes {
            delta.set(idx.to_local_pos(), new);
        }
        delta
    }

    /// Convert to a reverse delta (target -> source).
    #[must_use]
    pub fn to_reverse_delta(&self) -> ChunkDelta {
        let mut delta = ChunkDelta::new();
        for (&idx, &(old, _)) in &self.changes {
            delta.set(idx.to_local_pos(), old);
        }
        delta
    }
}

/// Difference between two world snapshots.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// Chunks only in source (sorted by position).
    pub source_only: Vec<ChunkPos>,
    /// Chunks only in target (sorted by position).
    pub target_only: Vec<ChunkPos>,
    /// Chunks with differences.
    pub chunk_diffs: Vec<ChunkDiff>,
    /// Fingerprint of source snapshot.
    pub source_fingerprint: SnapshotFingerprint,
    /// Fingerprint of target snapshot.
    pub target_fingerprint: SnapshotFingerprint,
}

impl SnapshotDiff {
    /// Create an empty diff.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Compute diff between two sets of chunks.
    #[must_use]
    pub fn compute<S: BuildHasher>(
        source: &HashMap<ChunkPos, Chunk, S>,
        target: &HashMap<ChunkPos, Chunk, S>,
    ) -> Self {
        use std::collections::HashSet;

        let source_positions: HashSet<_> = source.keys().copied().collect();
        let target_positions: HashSet<_> = target.keys().copied().collect();

        let mut source_only: Vec<_> = source_positions
            .difference(&target_positions)
            .copied()
            .collect();
        source_only.sort_by_key(|p| (p.x(), p.y(), p.z()));

        let mut target_only: Vec<_> = target_positions
            .difference(&source_positions)
            .copied()
            .collect();
        target_only.sort_by_key(|p| (p.x(), p.y(), p.z()));

        let mut chunk_diffs = Vec::new();
        for pos in source_positions.intersection(&target_positions) {
            let (Some(source_chunk), Some(target_chunk)) = (source.get(pos), target.get(pos))
            else {
                continue;
            };

            let diff = ChunkDiff::compute(*pos, source_chunk, target_chunk);
            if !diff.is_empty() {
                chunk_diffs.push(diff);
            }
        }

        chunk_diffs.sort_by_key(|d| (d.chunk_pos.x(), d.chunk_pos.y(), d.chunk_pos.z()));

        let source_fingerprint = compute_chunks_fingerprint(source);
        let target_fingerprint = compute_chunks_fingerprint(target);

        Self {
            source_only,
            target_only,
            chunk_diffs,
            source_fingerprint,
            target_fingerprint,
        }
    }

    /// Check if snapshots are identical.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.source_only.is_empty() && self.target_only.is_empty() && self.chunk_diffs.is_empty()
    }

    /// Get total number of block changes.
    #[must_use]
    pub fn total_changes(&self) -> usize {
        self.chunk_diffs.iter().map(ChunkDiff::change_count).sum()
    }

    /// Get summary statistics.
    #[must_use]
    pub fn summary(&self) -> DiffSummary {
        DiffSummary {
            source_only_chunks: self.source_only.len(),
            target_only_chunks: self.target_only.len(),
            modified_chunks: self.chunk_diffs.len(),
            total_block_changes: self.total_changes(),
            fingerprints_match: self.source_fingerprint.matches(self.target_fingerprint),
        }
    }
}

/// Summary statistics for a snapshot diff.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Chunks present only in source.
    pub source_only_chunks: usize,
    /// Chunks present only in target.
    pub target_only_chunks: usize,
    /// Chunks with block differences.
    pub modified_chunks: usize,
    /// Total block-level changes.
    pub total_block_changes: usize,
    /// Whether fingerprints match.
    pub fingerprints_match: bool,
}

/// Compute a deterministic fingerprint for a set of chunks.
#[must_use]
pub fn compute_chunks_fingerprint<S: BuildHasher>(
    chunks: &HashMap<ChunkPos, Chunk, S>,
) -> SnapshotFingerprint {
    let mut builder = FingerprintBuilder::new();

    let mut positions: Vec<_> = chunks.keys().collect();
    positions.sort_by_key(|p| (p.x(), p.y(), p.z()));

    for pos in positions {
        let Some(chunk) = chunks.get(pos) else {
            continue;
        };
        builder.feed_chunk_pos(*pos);
        let checksum = ChunkChecksum::compute(chunk);
        builder.feed_u32(checksum.value());
    }

    builder.build()
}

/// Compute a deterministic fingerprint for world metadata.
#[must_use]
pub fn compute_meta_fingerprint(meta: &WorldMeta) -> SnapshotFingerprint {
    let mut builder = FingerprintBuilder::new();
    builder.feed_u64(meta.seed);
    builder.feed_i32(meta.spawn.x());
    builder.feed_i32(meta.spawn.y());
    builder.feed_i32(meta.spawn.z());
    builder.build()
}

/// Analyzer for detecting repair issues in chunks.
pub struct RepairAnalyzer {
    max_valid_block_id: u16,
}

impl RepairAnalyzer {
    /// Create a new analyzer.
    #[must_use]
    pub fn new(max_valid_block_id: u16) -> Self {
        Self { max_valid_block_id }
    }

    /// Analyze a single chunk for issues.
    #[must_use]
    pub fn analyze_chunk(&self, pos: ChunkPos, chunk: &Chunk) -> Vec<RepairIssue> {
        let mut issues = Vec::new();

        // Check block count consistency
        let actual_count = chunk.blocks().iter().filter(|b| **b != AIR).count();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "actual_count bounded by CHUNK_VOLUME (4096)"
        )]
        if chunk.non_air_count() != actual_count as u32 {
            issues.push(
                RepairIssue::new(
                    IssueCategory::BlockCountMismatch,
                    IssueSeverity::Warning,
                    pos,
                )
                .with_description("`non_air_count` does not match actual block count")
                .with_values(
                    format!("{}", chunk.non_air_count()),
                    format!("{actual_count}"),
                ),
            );
        }

        // Check for invalid block IDs
        for (local_pos, block) in chunk.iter() {
            if block.0 > self.max_valid_block_id && block != AIR {
                issues.push(
                    RepairIssue::new(IssueCategory::InvalidBlockId, IssueSeverity::Error, pos)
                        .with_local_pos(local_pos)
                        .with_description("block ID exceeds valid range")
                        .with_values(
                            format!("<= {}", self.max_valid_block_id),
                            format!("{}", block.0),
                        ),
                );
            }
        }

        issues
    }

    /// Analyze multiple chunks.
    #[must_use]
    pub fn analyze_chunks<S: BuildHasher>(
        &self,
        chunks: &HashMap<ChunkPos, Chunk, S>,
    ) -> Vec<RepairIssue> {
        let mut all_issues = Vec::new();
        for (&pos, chunk) in chunks {
            all_issues.extend(self.analyze_chunk(pos, chunk));
        }
        all_issues.sort_by_key(|i| (i.chunk_pos.x(), i.chunk_pos.y(), i.chunk_pos.z()));
        all_issues
    }

    /// Create a repair plan from detected issues.
    #[must_use]
    pub fn plan_repairs(&self, issues: Vec<RepairIssue>, max_modifications: usize) -> RepairPlan {
        let mut plan = RepairPlan::new(max_modifications);

        for issue in issues {
            match issue.category {
                IssueCategory::BlockCountMismatch => {
                    plan.add_operation(RepairOp::RecalculateCount {
                        chunk_pos: issue.chunk_pos,
                    });
                }
                IssueCategory::InvalidBlockId => {
                    if let Some((local_pos, block_id)) = issue
                        .local_pos
                        .zip(issue.actual.as_ref().and_then(|a| a.parse::<u16>().ok()))
                    {
                        plan.add_operation(RepairOp::ReplaceInvalidBlock {
                            chunk_pos: issue.chunk_pos,
                            local_pos,
                            old_block: BlockId(block_id),
                        });
                    }
                }
                _ => {}
            }
            plan.add_issue(issue);
        }

        plan
    }
}

impl Default for RepairAnalyzer {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Apply a repair plan to chunks.
pub fn apply_repairs<S: BuildHasher>(
    chunks: &mut HashMap<ChunkPos, Chunk, S>,
    plan: &RepairPlan,
) -> RepairResult {
    let mut result = RepairResult::default();
    let mut modified_chunks = HashSet::new();

    for op in &plan.operations {
        match op {
            RepairOp::RecalculateCount { chunk_pos } => {
                if let Some(chunk) = chunks.get_mut(chunk_pos) {
                    chunk.recalculate_count();
                    modified_chunks.insert(*chunk_pos);
                    result.operations_applied += 1;
                } else {
                    result.operations_failed += 1;
                    result
                        .errors
                        .push(format!("chunk not found: {chunk_pos:?}"));
                }
            }
            RepairOp::ReplaceInvalidBlock {
                chunk_pos,
                local_pos,
                ..
            } => {
                if let Some(chunk) = chunks.get_mut(chunk_pos) {
                    chunk.set(*local_pos, AIR);
                    modified_chunks.insert(*chunk_pos);
                    result.blocks_changed += 1;
                    result.operations_applied += 1;
                } else {
                    result.operations_failed += 1;
                    result
                        .errors
                        .push(format!("chunk not found: {chunk_pos:?}"));
                }
            }
            RepairOp::ApplyDelta { chunk_pos, delta } => {
                if let Some(chunk) = chunks.get_mut(chunk_pos) {
                    for (local_pos, block) in delta.iter() {
                        chunk.set(local_pos, block);
                        result.blocks_changed += 1;
                    }
                    modified_chunks.insert(*chunk_pos);
                    result.operations_applied += 1;
                } else {
                    result.operations_failed += 1;
                    result
                        .errors
                        .push(format!("chunk not found: {chunk_pos:?}"));
                }
            }
            RepairOp::RegenerateChunk { chunk_pos } => {
                result.operations_failed += 1;
                result.errors.push(format!(
                    "regeneration not supported in apply_repairs: {chunk_pos:?}"
                ));
            }
        }
    }

    result.chunks_modified = modified_chunks.len();
    result
}

/// Verify chunk checksums against expected values.
#[must_use]
pub fn verify_checksums<S1: BuildHasher, S2: BuildHasher>(
    chunks: &HashMap<ChunkPos, Chunk, S1>,
    expected: &HashMap<ChunkPos, ChunkChecksum, S2>,
) -> Vec<RepairIssue> {
    let mut issues = Vec::new();

    for (pos, expected_checksum) in expected {
        if let Some(chunk) = chunks.get(pos) {
            let actual = ChunkChecksum::compute(chunk);
            if !actual.matches(*expected_checksum) {
                issues.push(
                    RepairIssue::new(IssueCategory::ChecksumMismatch, IssueSeverity::Error, *pos)
                        .with_description("chunk checksum mismatch")
                        .with_values(
                            format!("{:08x}", expected_checksum.value()),
                            format!("{:08x}", actual.value()),
                        ),
                );
            }
        } else {
            issues.push(
                RepairIssue::new(IssueCategory::MissingChunk, IssueSeverity::Critical, *pos)
                    .with_description("expected chunk is missing"),
            );
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::STONE;

    fn test_chunk() -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(1, 0, 0), STONE);
        chunk.set(LocalPos::new(0, 1, 0), BlockId(100));
        chunk
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let mut b1 = FingerprintBuilder::new();
        b1.feed_u64(100).feed_i32(5);
        let fp1 = b1.build();

        let mut b2 = FingerprintBuilder::new();
        b2.feed_u64(100).feed_i32(5);
        let fp2 = b2.build();

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_order_matters() {
        let mut b1 = FingerprintBuilder::new();
        b1.feed_u64(1).feed_u64(2);
        let fp1 = b1.build();

        let mut b2 = FingerprintBuilder::new();
        b2.feed_u64(2).feed_u64(1);
        let fp2 = b2.build();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_combine() {
        let fp1 = SnapshotFingerprint::from_raw(100);
        let fp2 = SnapshotFingerprint::from_raw(200);
        let combined = fp1.combine(fp2);

        assert_ne!(combined, fp1);
        assert_ne!(combined, fp2);
    }

    #[test]
    fn test_chunk_checksum_deterministic() {
        let chunk = test_chunk();
        let cs1 = ChunkChecksum::compute(&chunk);
        let cs2 = ChunkChecksum::compute(&chunk);
        assert_eq!(cs1, cs2);
    }

    #[test]
    fn test_chunk_checksum_differs() {
        let chunk1 = test_chunk();
        let mut chunk2 = test_chunk();
        chunk2.set(LocalPos::new(5, 5, 5), STONE);

        let cs1 = ChunkChecksum::compute(&chunk1);
        let cs2 = ChunkChecksum::compute(&chunk2);
        assert_ne!(cs1, cs2);
    }

    #[test]
    fn test_chunk_diff_identical() {
        let chunk = test_chunk();
        let diff = ChunkDiff::compute(ChunkPos::new(0, 0, 0), &chunk, &chunk);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_chunk_diff_single_change() {
        let chunk1 = Chunk::new();
        let mut chunk2 = Chunk::new();
        chunk2.set(LocalPos::new(5, 5, 5), STONE);

        let diff = ChunkDiff::compute(ChunkPos::new(0, 0, 0), &chunk1, &chunk2);
        assert_eq!(diff.change_count(), 1);
    }

    #[test]
    fn test_chunk_diff_to_delta() {
        let chunk1 = Chunk::new();
        let mut chunk2 = Chunk::new();
        chunk2.set(LocalPos::new(5, 5, 5), STONE);

        let diff = ChunkDiff::compute(ChunkPos::new(0, 0, 0), &chunk1, &chunk2);
        let forward = diff.to_forward_delta();
        let reverse = diff.to_reverse_delta();

        assert_eq!(forward.get(LocalPos::new(5, 5, 5)), Some(STONE));
        assert_eq!(reverse.get(LocalPos::new(5, 5, 5)), Some(AIR));
    }

    #[test]
    fn test_snapshot_diff_identical() {
        let mut chunks = HashMap::new();
        chunks.insert(ChunkPos::new(0, 0, 0), test_chunk());

        let diff = SnapshotDiff::compute(&chunks, &chunks);
        assert!(diff.is_identical());
    }

    #[test]
    fn test_snapshot_diff_missing_chunk() {
        let mut source = HashMap::new();
        source.insert(ChunkPos::new(0, 0, 0), test_chunk());

        let target = HashMap::new();

        let diff = SnapshotDiff::compute(&source, &target);
        assert!(!diff.is_identical());
        assert_eq!(diff.source_only.len(), 1);
        assert!(diff.source_only.contains(&ChunkPos::new(0, 0, 0)));
    }

    #[test]
    fn test_snapshot_diff_added_chunk() {
        let source = HashMap::new();

        let mut target = HashMap::new();
        target.insert(ChunkPos::new(0, 0, 0), test_chunk());

        let diff = SnapshotDiff::compute(&source, &target);
        assert!(!diff.is_identical());
        assert_eq!(diff.target_only.len(), 1);
    }

    #[test]
    fn test_snapshot_diff_modified_chunk() {
        let mut source = HashMap::new();
        source.insert(ChunkPos::new(0, 0, 0), Chunk::new());

        let mut target = HashMap::new();
        target.insert(ChunkPos::new(0, 0, 0), test_chunk());

        let diff = SnapshotDiff::compute(&source, &target);
        assert!(!diff.is_identical());
        assert_eq!(diff.chunk_diffs.len(), 1);
        assert_eq!(diff.total_changes(), 3);
    }

    #[test]
    fn test_repair_analyzer_block_count() {
        let mut chunk = test_chunk();
        chunk.blocks_mut()[100] = STONE;

        let analyzer = RepairAnalyzer::default();
        let issues = analyzer.analyze_chunk(ChunkPos::new(0, 0, 0), &chunk);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::BlockCountMismatch);
    }

    #[test]
    fn test_repair_analyzer_invalid_block() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(5, 5, 5), BlockId(9999));

        let analyzer = RepairAnalyzer::new(1000);
        let issues = analyzer.analyze_chunk(ChunkPos::new(0, 0, 0), &chunk);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::InvalidBlockId);
    }

    #[test]
    fn test_repair_plan_bounded() {
        let mut chunk = Chunk::new();
        for i in 0..100 {
            chunk.set(LocalPos::from_index(i), BlockId(9999));
        }

        let analyzer = RepairAnalyzer::new(1000);
        let issues = analyzer.analyze_chunk(ChunkPos::new(0, 0, 0), &chunk);
        let plan = analyzer.plan_repairs(issues, 50);

        assert!(!plan.within_bounds);
    }

    #[test]
    fn test_apply_repairs_recalculate() {
        let mut chunk = test_chunk();
        chunk.blocks_mut()[100] = STONE;

        let mut chunks = HashMap::new();
        let pos = ChunkPos::new(0, 0, 0);
        chunks.insert(pos, chunk);

        let analyzer = RepairAnalyzer::default();
        let issues = analyzer.analyze_chunks(&chunks);
        let plan = analyzer.plan_repairs(issues, 1000);

        let result = apply_repairs(&mut chunks, &plan);
        assert!(result.is_success());
        assert_eq!(result.operations_applied, 1);

        let repaired = chunks.get(&pos).unwrap();
        assert_eq!(repaired.non_air_count(), 4);
    }

    #[test]
    fn test_apply_repairs_invalid_block() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(5, 5, 5), BlockId(9999));

        let mut chunks = HashMap::new();
        let pos = ChunkPos::new(0, 0, 0);
        chunks.insert(pos, chunk);

        let analyzer = RepairAnalyzer::new(1000);
        let issues = analyzer.analyze_chunks(&chunks);
        let plan = analyzer.plan_repairs(issues, 1000);

        let result = apply_repairs(&mut chunks, &plan);
        assert!(result.is_success());
        assert_eq!(result.blocks_changed, 1);

        let repaired = chunks.get(&pos).unwrap();
        assert_eq!(repaired.get(LocalPos::new(5, 5, 5)), AIR);
    }

    #[test]
    fn test_verify_checksums() {
        let chunk = test_chunk();
        let pos = ChunkPos::new(0, 0, 0);

        let mut chunks = HashMap::new();
        chunks.insert(pos, chunk.clone());

        let mut expected = HashMap::new();
        expected.insert(pos, ChunkChecksum::compute(&chunk));

        let issues = verify_checksums(&chunks, &expected);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_verify_checksums_mismatch() {
        let chunk = test_chunk();
        let pos = ChunkPos::new(0, 0, 0);

        let mut chunks = HashMap::new();
        chunks.insert(pos, chunk);

        let mut expected = HashMap::new();
        expected.insert(pos, ChunkChecksum::from_raw(0xDEAD_BEEF));

        let issues = verify_checksums(&chunks, &expected);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::ChecksumMismatch);
    }

    #[test]
    fn test_verify_checksums_missing() {
        let chunks = HashMap::new();

        let mut expected = HashMap::new();
        expected.insert(ChunkPos::new(0, 0, 0), ChunkChecksum::from_raw(123));

        let issues = verify_checksums(&chunks, &expected);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, IssueCategory::MissingChunk);
    }

    #[test]
    fn test_serde_roundtrip_fingerprint() {
        let fp = SnapshotFingerprint::from_raw(0xCAFE_BABE);
        let json = serde_json::to_string(&fp).unwrap();
        let recovered: SnapshotFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(fp, recovered);
    }

    #[test]
    fn test_serde_roundtrip_checksum() {
        let cs = ChunkChecksum::from_raw(0x1234_5678);
        let json = serde_json::to_string(&cs).unwrap();
        let recovered: ChunkChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(cs, recovered);
    }

    #[test]
    fn test_serde_roundtrip_issue() {
        let issue = RepairIssue::new(
            IssueCategory::BlockCountMismatch,
            IssueSeverity::Warning,
            ChunkPos::new(1, 2, 3),
        )
        .with_description("test issue")
        .with_values("10", "5");

        let json = serde_json::to_string(&issue).unwrap();
        let recovered: RepairIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(issue, recovered);
    }

    #[test]
    fn test_serde_roundtrip_plan() {
        let plan = RepairPlan::new(1000);
        let json = serde_json::to_string(&plan).unwrap();
        let recovered: RepairPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan.max_modifications, recovered.max_modifications);
    }

    #[test]
    fn test_serde_roundtrip_diff() {
        let chunk1 = Chunk::new();
        let mut chunk2 = Chunk::new();
        chunk2.set(LocalPos::new(5, 5, 5), STONE);

        let diff = ChunkDiff::compute(ChunkPos::new(0, 0, 0), &chunk1, &chunk2);
        let json = serde_json::to_string(&diff).unwrap();
        let recovered: ChunkDiff = serde_json::from_str(&json).unwrap();

        assert_eq!(diff.chunk_pos, recovered.chunk_pos);
        assert_eq!(diff.change_count(), recovered.change_count());
    }

    #[test]
    fn test_serde_bincode_roundtrip() {
        let chunk1 = Chunk::new();
        let mut chunk2 = Chunk::new();
        chunk2.set(LocalPos::new(5, 5, 5), STONE);

        let diff = ChunkDiff::compute(ChunkPos::new(0, 0, 0), &chunk1, &chunk2);
        let bytes = bincode::serialize(&diff).unwrap();
        let recovered: ChunkDiff = bincode::deserialize(&bytes).unwrap();

        assert_eq!(diff.chunk_pos, recovered.chunk_pos);
        assert_eq!(diff.changes, recovered.changes);
    }

    #[test]
    fn test_compute_chunks_fingerprint_deterministic() {
        let mut chunks = HashMap::new();
        chunks.insert(ChunkPos::new(0, 0, 0), test_chunk());
        chunks.insert(ChunkPos::new(1, 0, 0), Chunk::new());

        let fp1 = compute_chunks_fingerprint(&chunks);
        let fp2 = compute_chunks_fingerprint(&chunks);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_compute_chunks_fingerprint_differs() {
        let mut chunks1 = HashMap::new();
        chunks1.insert(ChunkPos::new(0, 0, 0), test_chunk());

        let mut chunks2 = HashMap::new();
        chunks2.insert(ChunkPos::new(0, 0, 0), Chunk::new());

        let fp1 = compute_chunks_fingerprint(&chunks1);
        let fp2 = compute_chunks_fingerprint(&chunks2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_issue_category_safe_repair() {
        assert!(IssueCategory::BlockCountMismatch.is_safe_repair());
        assert!(IssueCategory::InvalidBlockId.is_safe_repair());
        assert!(IssueCategory::ChecksumMismatch.is_safe_repair());
        assert!(!IssueCategory::MissingChunk.is_safe_repair());
        assert!(!IssueCategory::CorruptChunk.is_safe_repair());
    }

    #[test]
    fn test_repair_op_estimated_cost() {
        let recalc = RepairOp::RecalculateCount {
            chunk_pos: ChunkPos::new(0, 0, 0),
        };
        assert_eq!(recalc.estimated_cost(), 0);

        let replace = RepairOp::ReplaceInvalidBlock {
            chunk_pos: ChunkPos::new(0, 0, 0),
            local_pos: LocalPos::new(0, 0, 0),
            old_block: BlockId(9999),
        };
        assert_eq!(replace.estimated_cost(), 1);

        let regen = RepairOp::RegenerateChunk {
            chunk_pos: ChunkPos::new(0, 0, 0),
        };
        assert_eq!(regen.estimated_cost(), CHUNK_VOLUME);
    }

    #[test]
    fn test_diff_summary() {
        let mut source = HashMap::new();
        source.insert(ChunkPos::new(0, 0, 0), Chunk::new());
        source.insert(ChunkPos::new(1, 0, 0), test_chunk());

        let mut target = HashMap::new();
        target.insert(ChunkPos::new(0, 0, 0), test_chunk());
        target.insert(ChunkPos::new(2, 0, 0), Chunk::new());

        let diff = SnapshotDiff::compute(&source, &target);
        let summary = diff.summary();

        assert_eq!(summary.source_only_chunks, 1);
        assert_eq!(summary.target_only_chunks, 1);
        assert_eq!(summary.modified_chunks, 1);
        assert!(!summary.fingerprints_match);
    }
}

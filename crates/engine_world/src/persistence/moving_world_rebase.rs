//! Titan-scale moving-world rebasing for living, streaming voxel maps.
//!
//! Handles deterministic coordinate transformation when the world origin shifts,
//! enabling infinite-world exploration by keeping the player near the coordinate
//! origin while the entire chunk grid translates around them. Designed for
//! massive procedural worlds where millions of chunks may exist across multiple
//! streaming regions.
//!
//! # Overview
//!
//! - [`RebaseMode`]: Full or partial rebasing behavior
//! - [`RebaseId`]: Deterministic identifier for rebase operations
//! - [`RebaseConfig`]: Configuration for rebase behavior, thresholds, and budgets
//! - [`RebaseValidationError`]: Validation errors for configuration
//! - [`RebaseOffset`]: Coordinate offset for rebasing
//! - [`RebaseState`]: State machine tracking rebase lifecycle
//! - [`ChunkMapping`]: Mapping from old to new chunk positions
//! - [`RebasePlan`]: Plan for rebasing chunk data
//! - [`RebaseResult`]: Outcome of applying a rebase plan
//! - [`RebaseSummary`]: Summary statistics for a rebase operation
//! - [`RebaseFingerprint`]: Deterministic fingerprint for rebase plans
//!
//! # Titan-Scale Design
//!
//! The rebasing system supports two modes for handling massive chunk counts:
//!
//! - **Full mode**: All chunks are rebased in a single operation. If the chunk
//!   count exceeds `max_chunks_per_plan`, the operation fails deterministically
//!   rather than producing a partial result.
//!
//! - **Partial mode**: Allows incremental rebasing where only a sorted prefix
//!   of chunks (up to `max_chunks_per_plan`) is processed per plan. Useful for
//!   streaming worlds where rebasing can be amortized across multiple frames.
//!
//! The `rebase_threshold_chunks` config controls when rebasing activates based
//! on offset magnitude, preventing unnecessary work for small movements.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::BuildHasher;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

fn chunk_pos_key(pos: ChunkPos) -> (i32, i32, i32) {
    (pos.x(), pos.y(), pos.z())
}

fn sort_chunk_positions(positions: &mut [ChunkPos]) {
    positions.sort_by_key(|p| chunk_pos_key(*p));
}

/// Rebase mode controlling full vs partial rebasing behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RebaseMode {
    /// Full rebasing: all chunks must be processed in a single plan.
    /// Exceeding `max_chunks_per_plan` produces a validation failure.
    #[default]
    Full = 0,
    /// Partial rebasing: process up to `max_chunks_per_plan` chunks,
    /// leaving remaining chunks for subsequent plans.
    Partial = 1,
}

impl RebaseMode {
    /// Get the display name for this mode.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Partial => "partial",
        }
    }
}

/// Deterministic identifier for a rebase operation.
///
/// Generated from seed and sequence number using CRC32 for stability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RebaseId(u64);

impl RebaseId {
    /// Generate a rebase ID from seed and sequence.
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

    /// Create from a raw value.
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

/// Validation error for rebase configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebaseValidationError {
    /// Threshold is zero, which would trigger rebasing on any offset.
    ThresholdZero,
    /// Max chunks per plan is zero, which would prevent any rebasing.
    MaxChunksZero,
    /// Bounds are invalid (min > max on some axis).
    InvalidBounds { axis: String, min: i32, max: i32 },
    /// Chunk count exceeds budget in full mode.
    BudgetExceeded { chunk_count: usize, budget: usize },
}

impl RebaseValidationError {
    /// Get a human-readable description of this error.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::ThresholdZero => {
                "rebase_threshold_chunks is zero; would trigger on any offset".to_string()
            }
            Self::MaxChunksZero => {
                "max_chunks_per_plan is zero; would prevent any rebasing".to_string()
            }
            Self::InvalidBounds { axis, min, max } => {
                format!("invalid bounds on {axis} axis: min ({min}) > max ({max})")
            }
            Self::BudgetExceeded {
                chunk_count,
                budget,
            } => {
                format!("chunk count ({chunk_count}) exceeds budget ({budget}) in full mode")
            }
        }
    }
}

/// Configuration for rebase behavior.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseConfig {
    /// Whether to allow collisions (overwriting existing chunks).
    pub allow_collisions: bool,
    /// Whether to drop chunks that would go out of bounds.
    pub drop_out_of_bounds: bool,
    /// Optional bounds for valid chunk positions.
    pub bounds: Option<RebaseBounds>,
    /// Minimum offset Manhattan distance (in chunks) before rebasing activates.
    /// Offsets below this threshold produce no-op/unchanged mappings.
    pub rebase_threshold_chunks: u32,
    /// Maximum number of chunk mappings per plan.
    /// In partial mode, excess chunks are deferred; in full mode, exceeding
    /// this budget produces a validation failure.
    pub max_chunks_per_plan: usize,
    /// Rebasing mode (full or partial).
    pub mode: RebaseMode,
}

impl Default for RebaseConfig {
    fn default() -> Self {
        Self {
            allow_collisions: false,
            drop_out_of_bounds: true,
            bounds: None,
            rebase_threshold_chunks: 1,
            max_chunks_per_plan: usize::MAX,
            mode: RebaseMode::Full,
        }
    }
}

impl RebaseConfig {
    /// Validate the configuration, returning errors if invalid.
    #[must_use]
    pub fn validate(&self) -> Vec<RebaseValidationError> {
        let mut errors = Vec::new();

        if self.rebase_threshold_chunks == 0 {
            errors.push(RebaseValidationError::ThresholdZero);
        }

        if self.max_chunks_per_plan == 0 {
            errors.push(RebaseValidationError::MaxChunksZero);
        }

        if let Some(ref bounds) = self.bounds {
            if bounds.min_x > bounds.max_x {
                errors.push(RebaseValidationError::InvalidBounds {
                    axis: "x".to_string(),
                    min: bounds.min_x,
                    max: bounds.max_x,
                });
            }
            if bounds.min_y > bounds.max_y {
                errors.push(RebaseValidationError::InvalidBounds {
                    axis: "y".to_string(),
                    min: bounds.min_y,
                    max: bounds.max_y,
                });
            }
            if bounds.min_z > bounds.max_z {
                errors.push(RebaseValidationError::InvalidBounds {
                    axis: "z".to_string(),
                    min: bounds.min_z,
                    max: bounds.max_z,
                });
            }
        }

        errors
    }

    /// Check if the configuration is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    /// Check if an offset is below the rebase threshold.
    #[must_use]
    pub fn is_below_threshold(&self, offset: RebaseOffset) -> bool {
        offset.manhattan_distance() < self.rebase_threshold_chunks
    }

    /// Check if a chunk count exceeds the budget.
    #[must_use]
    pub fn exceeds_budget(&self, chunk_count: usize) -> bool {
        chunk_count > self.max_chunks_per_plan
    }
}

/// Bounds for valid chunk positions after rebasing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_z: i32,
    pub max_z: i32,
}

impl RebaseBounds {
    /// Create new bounds.
    #[must_use]
    pub const fn new(
        min_x: i32,
        max_x: i32,
        min_y: i32,
        max_y: i32,
        min_z: i32,
        max_z: i32,
    ) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        }
    }

    /// Check if a position is within bounds.
    #[must_use]
    pub fn contains(&self, pos: ChunkPos) -> bool {
        pos.x() >= self.min_x
            && pos.x() <= self.max_x
            && pos.y() >= self.min_y
            && pos.y() <= self.max_y
            && pos.z() >= self.min_z
            && pos.z() <= self.max_z
    }
}

/// Offset for rebasing chunk positions when the world origin shifts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct RebaseOffset {
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
}

impl RebaseOffset {
    #[must_use]
    pub const fn new(dx: i32, dy: i32, dz: i32) -> Self {
        Self { dx, dy, dz }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    #[must_use]
    pub fn apply(self, pos: ChunkPos) -> ChunkPos {
        ChunkPos::new(
            pos.x().saturating_add(self.dx),
            pos.y().saturating_add(self.dy),
            pos.z().saturating_add(self.dz),
        )
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.dx == 0 && self.dy == 0 && self.dz == 0
    }

    /// Compute the inverse offset.
    #[must_use]
    pub const fn inverse(self) -> Self {
        Self::new(-self.dx, -self.dy, -self.dz)
    }

    /// Compute the Manhattan distance of this offset.
    #[must_use]
    pub const fn manhattan_distance(self) -> u32 {
        self.dx.unsigned_abs() + self.dy.unsigned_abs() + self.dz.unsigned_abs()
    }
}

/// State machine for tracking rebase lifecycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RebaseState {
    /// Initial state, no rebase in progress.
    #[default]
    Idle = 0,
    /// Planning phase, computing mappings.
    Planning = 1,
    /// Validation phase, checking for issues.
    Validating = 2,
    /// Executing phase, applying changes.
    Executing = 3,
    /// Completed successfully.
    Completed = 4,
    /// Failed with errors.
    Failed = 5,
}

impl RebaseState {
    /// Get the display name for this state.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Planning => "planning",
            Self::Validating => "validating",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// Check if this is a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// Category of chunk in a rebase operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ChunkCategory {
    /// Chunk will be dropped (out of bounds or explicitly removed).
    Dropped = 0,
    /// Chunk position will be shifted.
    Shifted = 1,
    /// Chunk position unchanged (zero offset or not in shift list).
    Unchanged = 2,
    /// Chunk would collide with another chunk after shifting.
    Colliding = 3,
}

impl ChunkCategory {
    /// Get the display name for this category.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dropped => "dropped",
            Self::Shifted => "shifted",
            Self::Unchanged => "unchanged",
            Self::Colliding => "colliding",
        }
    }
}

/// Mapping entry from old to new chunk position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkMappingEntry {
    pub old_pos: ChunkPos,
    pub new_pos: ChunkPos,
    pub category: ChunkCategory,
}

impl ChunkMappingEntry {
    /// Create a new mapping entry.
    #[must_use]
    pub const fn new(old_pos: ChunkPos, new_pos: ChunkPos, category: ChunkCategory) -> Self {
        Self {
            old_pos,
            new_pos,
            category,
        }
    }

    /// Create a dropped entry.
    #[must_use]
    pub const fn dropped(pos: ChunkPos) -> Self {
        Self::new(pos, pos, ChunkCategory::Dropped)
    }

    /// Create an unchanged entry.
    #[must_use]
    pub const fn unchanged(pos: ChunkPos) -> Self {
        Self::new(pos, pos, ChunkCategory::Unchanged)
    }
}

/// Complete mapping of old to new chunk positions for a rebase.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChunkMapping {
    entries: HashMap<ChunkPos, ChunkMappingEntry>,
    collisions: Vec<(ChunkPos, ChunkPos)>,
}

impl ChunkMapping {
    /// Create a new empty mapping.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mapping entry.
    pub fn insert(&mut self, entry: ChunkMappingEntry) {
        self.entries.insert(entry.old_pos, entry);
    }

    /// Record a collision between two chunks.
    pub fn add_collision(&mut self, pos_a: ChunkPos, pos_b: ChunkPos) {
        let (min, max) = if chunk_pos_key(pos_a) < chunk_pos_key(pos_b) {
            (pos_a, pos_b)
        } else {
            (pos_b, pos_a)
        };
        self.collisions.push((min, max));
    }

    /// Get an entry by old position.
    #[must_use]
    pub fn get(&self, old_pos: ChunkPos) -> Option<&ChunkMappingEntry> {
        self.entries.get(&old_pos)
    }

    /// Get all entries in deterministic order (sorted by `old_pos`).
    #[must_use]
    pub fn entries_sorted(&self) -> Vec<ChunkMappingEntry> {
        let mut entries: Vec<_> = self.entries.values().copied().collect();
        entries.sort_by_key(|e| chunk_pos_key(e.old_pos));
        entries
    }

    /// Get all collisions (sorted for determinism).
    #[must_use]
    pub fn collisions(&self) -> Vec<(ChunkPos, ChunkPos)> {
        let mut collisions = self.collisions.clone();
        collisions.sort_by_key(|(a, _)| chunk_pos_key(*a));
        collisions
    }

    /// Count entries by category.
    #[must_use]
    pub fn count_by_category(&self) -> BTreeMap<ChunkCategory, usize> {
        let mut counts = BTreeMap::new();
        for entry in self.entries.values() {
            *counts.entry(entry.category).or_insert(0) += 1;
        }
        counts
    }

    /// Check if there are any collisions.
    #[must_use]
    pub fn has_collisions(&self) -> bool {
        !self.collisions.is_empty()
    }

    /// Get the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Plan for rebasing chunk data to a new coordinate origin.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RebasePlan {
    pub id: Option<RebaseId>,
    pub offset: RebaseOffset,
    pub config: RebaseConfig,
    pub mapping: ChunkMapping,
    pub state: RebaseState,
    /// Validation errors encountered during planning.
    pub validation_errors: Vec<RebaseValidationError>,
    /// Number of chunks deferred (in partial mode).
    pub deferred_count: usize,
}

impl RebasePlan {
    /// Create a new plan with the given offset.
    #[must_use]
    pub fn new(offset: RebaseOffset) -> Self {
        Self {
            id: None,
            offset,
            config: RebaseConfig::default(),
            mapping: ChunkMapping::new(),
            state: RebaseState::Idle,
            validation_errors: Vec::new(),
            deferred_count: 0,
        }
    }

    /// Create a plan with ID and configuration.
    #[must_use]
    pub fn with_config(id: RebaseId, offset: RebaseOffset, config: RebaseConfig) -> Self {
        Self {
            id: Some(id),
            offset,
            config,
            mapping: ChunkMapping::new(),
            state: RebaseState::Idle,
            validation_errors: Vec::new(),
            deferred_count: 0,
        }
    }

    /// Check if this plan is effectively a no-op.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.offset.is_zero() && self.mapping.is_empty()
    }

    /// Check if this plan has validation errors.
    #[must_use]
    pub fn has_validation_errors(&self) -> bool {
        !self.validation_errors.is_empty()
    }

    /// Check if this plan is partial (has deferred chunks).
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.deferred_count > 0
    }

    /// Get chunks marked for dropping in deterministic order.
    #[must_use]
    pub fn chunks_to_drop(&self) -> Vec<ChunkPos> {
        self.mapping
            .entries_sorted()
            .into_iter()
            .filter(|e| e.category == ChunkCategory::Dropped)
            .map(|e| e.old_pos)
            .collect()
    }

    /// Get chunks marked for shifting in deterministic order.
    #[must_use]
    pub fn chunks_to_shift(&self) -> Vec<ChunkPos> {
        self.mapping
            .entries_sorted()
            .into_iter()
            .filter(|e| e.category == ChunkCategory::Shifted)
            .map(|e| e.old_pos)
            .collect()
    }

    /// Compute the fingerprint for this plan.
    #[must_use]
    pub fn fingerprint(&self) -> RebaseFingerprint {
        let mut builder = RebaseFingerprintBuilder::new();

        if let Some(id) = self.id {
            builder.feed_u64(id.as_u64());
        }
        builder.feed_offset(self.offset);

        for entry in self.mapping.entries_sorted() {
            builder.feed_chunk_pos(entry.old_pos);
            builder.feed_chunk_pos(entry.new_pos);
            builder.feed_u8(entry.category as u8);
        }

        builder.build()
    }
}

/// Result of applying a rebase plan.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseResult {
    pub chunks_dropped: usize,
    pub chunks_shifted: usize,
    pub chunks_unchanged: usize,
    pub chunks_collided: usize,
    pub state: RebaseState,
}

impl RebaseResult {
    /// Total chunks processed.
    #[must_use]
    pub fn total(&self) -> usize {
        self.chunks_dropped + self.chunks_shifted + self.chunks_unchanged + self.chunks_collided
    }

    /// Check if the rebase was successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.state == RebaseState::Completed
    }
}

/// Summary statistics for a rebase operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseSummary {
    pub id: Option<RebaseId>,
    pub offset: RebaseOffset,
    pub total_chunks: usize,
    pub dropped: usize,
    pub shifted: usize,
    pub unchanged: usize,
    pub collisions: usize,
    pub fingerprint: RebaseFingerprint,
}

impl RebaseSummary {
    /// Create a summary from a plan and result.
    #[must_use]
    pub fn from_plan_and_result(plan: &RebasePlan, result: &RebaseResult) -> Self {
        Self {
            id: plan.id,
            offset: plan.offset,
            total_chunks: result.total(),
            dropped: result.chunks_dropped,
            shifted: result.chunks_shifted,
            unchanged: result.chunks_unchanged,
            collisions: result.chunks_collided,
            fingerprint: plan.fingerprint(),
        }
    }
}

/// CRC32-based fingerprint for rebase plans.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RebaseFingerprint {
    hash: u32,
}

impl RebaseFingerprint {
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
}

/// Builder for constructing rebase fingerprints.
#[derive(Debug)]
pub struct RebaseFingerprintBuilder {
    hasher: crc32fast::Hasher,
}

impl RebaseFingerprintBuilder {
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

    /// Feed a u8 value.
    pub fn feed_u8(&mut self, value: u8) -> &mut Self {
        self.hasher.update(&[value]);
        self
    }

    /// Feed a chunk position.
    pub fn feed_chunk_pos(&mut self, pos: ChunkPos) -> &mut Self {
        self.feed_i32(pos.x()).feed_i32(pos.y()).feed_i32(pos.z())
    }

    /// Feed an offset.
    pub fn feed_offset(&mut self, offset: RebaseOffset) -> &mut Self {
        self.feed_i32(offset.dx)
            .feed_i32(offset.dy)
            .feed_i32(offset.dz)
    }

    /// Finalize and produce the fingerprint.
    #[must_use]
    pub fn build(self) -> RebaseFingerprint {
        RebaseFingerprint {
            hash: self.hasher.finalize(),
        }
    }
}

impl Default for RebaseFingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute a rebase plan from a set of chunk positions.
///
/// Analyzes which chunks should be shifted, dropped, or left unchanged,
/// and detects any collisions. Respects threshold and budget configuration:
///
/// - Offsets below `rebase_threshold_chunks` produce unchanged mappings.
/// - In partial mode, only `max_chunks_per_plan` chunks are processed.
/// - In full mode, exceeding the budget produces a validation error.
#[must_use]
pub fn compute_rebase_plan<S>(
    chunks: &HashSet<ChunkPos, S>,
    offset: RebaseOffset,
    config: &RebaseConfig,
) -> RebasePlan
where
    S: BuildHasher,
{
    let mut plan = RebasePlan::new(offset);
    plan.config = config.clone();
    plan.state = RebaseState::Planning;

    // Check config validation
    let config_errors = config.validate();
    if !config_errors.is_empty() {
        plan.validation_errors = config_errors;
        plan.state = RebaseState::Failed;
        return plan;
    }

    // Check threshold - if below, all chunks are unchanged
    if config.is_below_threshold(offset) {
        for &pos in chunks {
            plan.mapping.insert(ChunkMappingEntry::unchanged(pos));
        }
        plan.state = RebaseState::Completed;
        return plan;
    }

    // Check budget
    let chunk_count = chunks.len();
    if config.exceeds_budget(chunk_count) {
        match config.mode {
            RebaseMode::Full => {
                plan.validation_errors
                    .push(RebaseValidationError::BudgetExceeded {
                        chunk_count,
                        budget: config.max_chunks_per_plan,
                    });
                plan.state = RebaseState::Failed;
                return plan;
            }
            RebaseMode::Partial => {
                plan.deferred_count = chunk_count.saturating_sub(config.max_chunks_per_plan);
            }
        }
    }

    if offset.is_zero() {
        for &pos in chunks {
            plan.mapping.insert(ChunkMappingEntry::unchanged(pos));
        }
        plan.state = RebaseState::Completed;
        return plan;
    }

    let mut new_positions: HashMap<ChunkPos, ChunkPos> = HashMap::new();

    let mut sorted_chunks: Vec<_> = chunks.iter().copied().collect();
    sort_chunk_positions(&mut sorted_chunks);

    // In partial mode, limit to budget
    let chunks_to_process = if config.mode == RebaseMode::Partial {
        config.max_chunks_per_plan.min(sorted_chunks.len())
    } else {
        sorted_chunks.len()
    };

    for (idx, old_pos) in sorted_chunks.iter().enumerate() {
        if idx >= chunks_to_process {
            break;
        }

        let new_pos = offset.apply(*old_pos);

        let out_of_bounds = config
            .bounds
            .as_ref()
            .is_some_and(|bounds| !bounds.contains(new_pos));

        if out_of_bounds && config.drop_out_of_bounds {
            plan.mapping.insert(ChunkMappingEntry::dropped(*old_pos));
            continue;
        }

        // Check if two chunks map to the same destination
        if let Some(&existing_old) = new_positions.get(&new_pos) {
            plan.mapping.add_collision(existing_old, *old_pos);
            if !config.allow_collisions {
                plan.mapping.insert(ChunkMappingEntry::new(
                    *old_pos,
                    new_pos,
                    ChunkCategory::Colliding,
                ));
                continue;
            }
        }

        // Check if destination is occupied by another chunk's source position
        if chunks.contains(&new_pos) {
            plan.mapping.add_collision(new_pos, *old_pos);
            if !config.allow_collisions {
                plan.mapping.insert(ChunkMappingEntry::new(
                    *old_pos,
                    new_pos,
                    ChunkCategory::Colliding,
                ));
                continue;
            }
        }

        new_positions.insert(new_pos, *old_pos);
        plan.mapping.insert(ChunkMappingEntry::new(
            *old_pos,
            new_pos,
            ChunkCategory::Shifted,
        ));
    }

    plan.state = if plan.mapping.has_collisions() && !config.allow_collisions {
        RebaseState::Failed
    } else {
        RebaseState::Validating
    };

    plan
}

/// Apply a rebase plan to chunk data, shifting positions by the plan's offset.
///
/// Returns chunks in deterministic order for consistent results.
pub fn apply_rebase_plan<S>(
    chunk_data: &mut HashMap<ChunkPos, Vec<u8>, S>,
    plan: &RebasePlan,
) -> RebaseResult
where
    S: BuildHasher + Default,
{
    let mut result = RebaseResult {
        state: RebaseState::Executing,
        ..Default::default()
    };

    let to_drop: Vec<ChunkPos> = plan.chunks_to_drop();
    for pos in to_drop {
        if chunk_data.remove(&pos).is_some() {
            result.chunks_dropped += 1;
        }
    }

    if plan.offset.is_zero() {
        result.chunks_unchanged = chunk_data.len();
        result.state = RebaseState::Completed;
        return result;
    }

    let to_shift: Vec<ChunkPos> = plan.chunks_to_shift();
    let entries: Vec<_> = to_shift
        .iter()
        .filter_map(|pos| chunk_data.remove(pos).map(|data| (*pos, data)))
        .collect();

    for (old_pos, data) in entries {
        let new_pos = plan.offset.apply(old_pos);
        if chunk_data.contains_key(&new_pos) && !plan.config.allow_collisions {
            result.chunks_collided += 1;
        } else {
            chunk_data.insert(new_pos, data);
            result.chunks_shifted += 1;
        }
    }

    result.chunks_unchanged = chunk_data.len().saturating_sub(result.chunks_shifted);
    result.state = if result.chunks_collided > 0 {
        RebaseState::Failed
    } else {
        RebaseState::Completed
    };

    result
}

/// Collect chunk positions from a slice into a Vec.
#[must_use]
pub fn collect_chunk_positions(chunks: &[ChunkPos]) -> Vec<ChunkPos> {
    chunks.to_vec()
}

/// Collect chunk positions in deterministic (sorted) order.
#[must_use]
pub fn collect_chunk_positions_sorted<S>(chunks: &HashSet<ChunkPos, S>) -> Vec<ChunkPos>
where
    S: BuildHasher,
{
    let mut sorted: Vec<_> = chunks.iter().copied().collect();
    sort_chunk_positions(&mut sorted);
    sorted
}

/// Detect chunks that exist in `actual` but not in `expected`.
pub fn detect_stale_chunks<S1, S2>(
    expected: &HashSet<ChunkPos, S1>,
    actual: &HashSet<ChunkPos, S2>,
) -> Vec<ChunkPos>
where
    S1: BuildHasher,
    S2: BuildHasher,
{
    let mut stale: Vec<_> = actual
        .iter()
        .filter(|pos| !expected.contains(pos))
        .copied()
        .collect();
    sort_chunk_positions(&mut stale);
    stale
}

/// Detect chunks that exist in `expected` but not in `actual`.
pub fn detect_missing_chunks<S1, S2>(
    expected: &HashSet<ChunkPos, S1>,
    actual: &HashSet<ChunkPos, S2>,
) -> Vec<ChunkPos>
where
    S1: BuildHasher,
    S2: BuildHasher,
{
    let mut missing: Vec<_> = expected
        .iter()
        .filter(|pos| !actual.contains(pos))
        .copied()
        .collect();
    sort_chunk_positions(&mut missing);
    missing
}

/// Compute the symmetric difference between two chunk sets.
///
/// Returns (stale, missing) in deterministic sorted order.
pub fn detect_chunk_diff<S1, S2>(
    expected: &HashSet<ChunkPos, S1>,
    actual: &HashSet<ChunkPos, S2>,
) -> (Vec<ChunkPos>, Vec<ChunkPos>)
where
    S1: BuildHasher,
    S2: BuildHasher,
{
    let stale = detect_stale_chunks(expected, actual);
    let missing = detect_missing_chunks(expected, actual);
    (stale, missing)
}

/// Compute checksum for chunk data.
#[must_use]
pub fn compute_chunk_checksum(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Validate a rebase plan for issues before application.
#[must_use]
pub fn validate_rebase_plan(plan: &RebasePlan) -> Vec<RebaseIssue> {
    let mut issues = Vec::new();

    if plan.mapping.has_collisions() && !plan.config.allow_collisions {
        for (a, b) in plan.mapping.collisions() {
            issues.push(RebaseIssue {
                kind: RebaseIssueKind::Collision,
                chunk_pos: a,
                other_pos: Some(b),
                description: format!(
                    "chunks ({},{},{}) and ({},{},{}) collide after shift",
                    a.x(),
                    a.y(),
                    a.z(),
                    b.x(),
                    b.y(),
                    b.z()
                ),
            });
        }
    }

    if let Some(ref bounds) = plan.config.bounds {
        for entry in plan.mapping.entries_sorted() {
            if entry.category == ChunkCategory::Shifted && !bounds.contains(entry.new_pos) {
                issues.push(RebaseIssue {
                    kind: RebaseIssueKind::OutOfBounds,
                    chunk_pos: entry.old_pos,
                    other_pos: Some(entry.new_pos),
                    description: format!(
                        "shifted chunk would be out of bounds at ({},{},{})",
                        entry.new_pos.x(),
                        entry.new_pos.y(),
                        entry.new_pos.z()
                    ),
                });
            }
        }
    }

    issues
}

/// Category of rebase issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RebaseIssueKind {
    /// Two chunks would occupy the same position after shift.
    Collision = 0,
    /// Chunk would be outside valid bounds after shift.
    OutOfBounds = 1,
    /// Chunk data is missing.
    MissingData = 2,
    /// Chunk data is stale or corrupted.
    StaleData = 3,
}

impl RebaseIssueKind {
    /// Get the display name for this kind.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Collision => "collision",
            Self::OutOfBounds => "out_of_bounds",
            Self::MissingData => "missing_data",
            Self::StaleData => "stale_data",
        }
    }
}

/// An issue detected during rebase planning or validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseIssue {
    pub kind: RebaseIssueKind,
    pub chunk_pos: ChunkPos,
    pub other_pos: Option<ChunkPos>,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Moving World Identity and Pose Types
// ---------------------------------------------------------------------------

/// Unique identifier for a moving world instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MovingWorldId(u32);

impl MovingWorldId {
    /// Create a new moving world ID.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Identifier for an anchor frame (reference frame for coordinate transforms).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AnchorFrameId(u32);

impl AnchorFrameId {
    /// The world origin anchor frame.
    pub const WORLD_ORIGIN: Self = Self(0);

    /// Create a new anchor frame ID.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl Default for AnchorFrameId {
    fn default() -> Self {
        Self::WORLD_ORIGIN
    }
}

/// Pose of a moving world relative to an anchor frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MovingWorldPose {
    /// The world this pose belongs to.
    pub world_id: MovingWorldId,
    /// The anchor frame this pose is relative to.
    pub anchor_frame: AnchorFrameId,
    /// The chunk position of the anchor in the world.
    pub anchor_chunk: ChunkPos,
    /// Generation counter for tracking rebase operations.
    pub generation: u64,
}

impl MovingWorldPose {
    /// Create a new pose.
    #[must_use]
    pub const fn new(
        world_id: MovingWorldId,
        anchor_frame: AnchorFrameId,
        anchor_chunk: ChunkPos,
        generation: u64,
    ) -> Self {
        Self {
            world_id,
            anchor_frame,
            anchor_chunk,
            generation,
        }
    }

    /// Create a new pose with a different anchor frame.
    #[must_use]
    pub const fn with_frame(self, anchor_frame: AnchorFrameId) -> Self {
        Self {
            anchor_frame,
            ..self
        }
    }

    /// Advance the pose to a new anchor, returning the new pose and the offset.
    #[must_use]
    pub fn advance_to(self, new_anchor: ChunkPos) -> (Self, RebaseOffset) {
        let offset = RebaseOffset::new(
            self.anchor_chunk.x() - new_anchor.x(),
            self.anchor_chunk.y() - new_anchor.y(),
            self.anchor_chunk.z() - new_anchor.z(),
        );
        let new_pose = Self {
            anchor_chunk: new_anchor,
            generation: self.generation.saturating_add(1),
            ..self
        };
        (new_pose, offset)
    }

    /// Compute the squared distance between two poses (in chunk coordinates).
    #[must_use]
    pub fn distance_squared(&self, other: &Self) -> i64 {
        let dx = i64::from(self.anchor_chunk.x()) - i64::from(other.anchor_chunk.x());
        let dy = i64::from(self.anchor_chunk.y()) - i64::from(other.anchor_chunk.y());
        let dz = i64::from(self.anchor_chunk.z()) - i64::from(other.anchor_chunk.z());
        dx * dx + dy * dy + dz * dz
    }
}

/// Registry of moving worlds and their poses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MovingWorldRegistry {
    worlds: BTreeMap<MovingWorldId, MovingWorldPose>,
}

impl MovingWorldRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new world with the given ID and initial pose.
    pub fn create(&mut self, world_id: MovingWorldId, pose: MovingWorldPose) -> bool {
        if self.worlds.contains_key(&world_id) {
            return false;
        }
        self.worlds.insert(world_id, pose);
        true
    }

    /// Register a world (insert or update).
    pub fn register(&mut self, world_id: MovingWorldId, pose: MovingWorldPose) {
        self.worlds.insert(world_id, pose);
    }

    /// Get a world's pose by ID.
    #[must_use]
    pub fn get(&self, world_id: MovingWorldId) -> Option<&MovingWorldPose> {
        self.worlds.get(&world_id)
    }

    /// Update a world's pose, returning the old pose if it existed.
    pub fn update_pose(
        &mut self,
        world_id: MovingWorldId,
        pose: MovingWorldPose,
    ) -> Option<MovingWorldPose> {
        self.worlds.insert(world_id, pose)
    }

    /// Compute a deterministic checksum of the registry.
    #[must_use]
    pub fn checksum(&self) -> RebaseFingerprint {
        let mut builder = RebaseFingerprintBuilder::new();
        for (id, pose) in &self.worlds {
            builder.feed_u32(id.raw());
            builder.feed_u32(pose.anchor_frame.raw());
            builder.feed_chunk_pos(pose.anchor_chunk);
            builder.feed_u64(pose.generation);
        }
        builder.build()
    }

    /// Get the number of registered worlds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.worlds.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.worlds.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Query Helpers
// ---------------------------------------------------------------------------

/// Convert a world position to local coordinates relative to an anchor.
#[must_use]
pub fn world_to_local(world_pos: ChunkPos, anchor: ChunkPos) -> ChunkPos {
    ChunkPos::new(
        world_pos.x() - anchor.x(),
        world_pos.y() - anchor.y(),
        world_pos.z() - anchor.z(),
    )
}

/// Convert a local position to world coordinates given an anchor.
#[must_use]
pub fn local_to_world(local_pos: ChunkPos, anchor: ChunkPos) -> ChunkPos {
    ChunkPos::new(
        local_pos.x() + anchor.x(),
        local_pos.y() + anchor.y(),
        local_pos.z() + anchor.z(),
    )
}

/// Apply a rebase offset to a position.
#[must_use]
pub fn rebased_position(original: ChunkPos, offset: RebaseOffset) -> ChunkPos {
    offset.apply(original)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::RandomState;

    #[test]
    fn test_rebase_id_generate() {
        let id1 = RebaseId::generate(42, 1);
        let id2 = RebaseId::generate(42, 2);
        let id3 = RebaseId::generate(42, 1);

        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_rebase_offset_apply() {
        let offset = RebaseOffset::new(1, -2, 3);
        let pos = ChunkPos::new(10, 20, 30);
        let result = offset.apply(pos);
        assert_eq!(result.x(), 11);
        assert_eq!(result.y(), 18);
        assert_eq!(result.z(), 33);
    }

    #[test]
    fn test_rebase_offset_zero() {
        let offset = RebaseOffset::zero();
        assert!(offset.is_zero());
        let pos = ChunkPos::new(5, 5, 5);
        assert_eq!(offset.apply(pos), pos);
    }

    #[test]
    fn test_rebase_offset_inverse() {
        let offset = RebaseOffset::new(1, -2, 3);
        let inverse = offset.inverse();
        assert_eq!(inverse, RebaseOffset::new(-1, 2, -3));
    }

    #[test]
    fn test_rebase_offset_manhattan_distance() {
        let offset = RebaseOffset::new(1, -2, 3);
        assert_eq!(offset.manhattan_distance(), 6);
    }

    #[test]
    fn test_rebase_bounds_contains() {
        let bounds = RebaseBounds::new(-10, 10, -10, 10, -10, 10);
        assert!(bounds.contains(ChunkPos::new(0, 0, 0)));
        assert!(bounds.contains(ChunkPos::new(10, 10, 10)));
        assert!(!bounds.contains(ChunkPos::new(11, 0, 0)));
    }

    #[test]
    fn test_chunk_mapping_deterministic_order() {
        let mut mapping = ChunkMapping::new();
        mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(2, 0, 0),
            ChunkPos::new(12, 0, 0),
            ChunkCategory::Shifted,
        ));
        mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(10, 0, 0),
            ChunkCategory::Shifted,
        ));
        mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(1, 0, 0),
            ChunkPos::new(11, 0, 0),
            ChunkCategory::Shifted,
        ));

        let positions: Vec<_> = mapping.entries_sorted().iter().map(|e| e.old_pos).collect();
        assert_eq!(
            positions,
            vec![
                ChunkPos::new(0, 0, 0),
                ChunkPos::new(1, 0, 0),
                ChunkPos::new(2, 0, 0),
            ]
        );
    }

    #[test]
    fn test_compute_rebase_plan_with_collisions() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(0, 0, 0));
        chunks.insert(ChunkPos::new(1, 0, 0));

        let offset = RebaseOffset::new(-1, 0, 0);
        let config = RebaseConfig {
            allow_collisions: false,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert!(plan.mapping.has_collisions());
        assert_eq!(plan.state, RebaseState::Failed);
    }

    #[test]
    fn test_compute_rebase_plan_with_bounds() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(0, 0, 0));
        chunks.insert(ChunkPos::new(5, 0, 0));

        let offset = RebaseOffset::new(10, 0, 0);
        let config = RebaseConfig {
            bounds: Some(RebaseBounds::new(0, 12, -100, 100, -100, 100)),
            drop_out_of_bounds: true,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        let dropped = plan.chunks_to_drop();
        let shifted = plan.chunks_to_shift();

        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0], ChunkPos::new(5, 0, 0));
        assert_eq!(shifted.len(), 1);
        assert_eq!(shifted[0], ChunkPos::new(0, 0, 0));
    }

    #[test]
    fn test_apply_rebase_plan_drop() {
        let mut data: HashMap<ChunkPos, Vec<u8>> = HashMap::new();
        data.insert(ChunkPos::new(0, 0, 0), vec![1, 2, 3]);
        data.insert(ChunkPos::new(1, 0, 0), vec![4, 5, 6]);

        let mut plan = RebasePlan::new(RebaseOffset::zero());
        plan.mapping
            .insert(ChunkMappingEntry::dropped(ChunkPos::new(0, 0, 0)));

        let result = apply_rebase_plan(&mut data, &plan);
        assert_eq!(result.chunks_dropped, 1);
        assert!(!data.contains_key(&ChunkPos::new(0, 0, 0)));
        assert!(data.contains_key(&ChunkPos::new(1, 0, 0)));
    }

    #[test]
    fn test_apply_rebase_plan_shift() {
        let mut data: HashMap<ChunkPos, Vec<u8>> = HashMap::new();
        data.insert(ChunkPos::new(0, 0, 0), vec![1, 2, 3]);
        data.insert(ChunkPos::new(1, 0, 0), vec![4, 5, 6]);

        let offset = RebaseOffset::new(10, 0, 0);
        let mut plan = RebasePlan::new(offset);
        plan.mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(10, 0, 0),
            ChunkCategory::Shifted,
        ));
        plan.mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(1, 0, 0),
            ChunkPos::new(11, 0, 0),
            ChunkCategory::Shifted,
        ));

        let result = apply_rebase_plan(&mut data, &plan);
        assert_eq!(result.chunks_shifted, 2);
        assert!(data.contains_key(&ChunkPos::new(10, 0, 0)));
        assert!(data.contains_key(&ChunkPos::new(11, 0, 0)));
    }

    #[test]
    fn test_collect_chunk_positions() {
        let chunks = vec![ChunkPos::new(0, 0, 0), ChunkPos::new(1, 1, 1)];
        let collected = collect_chunk_positions(&chunks);
        assert_eq!(chunks, collected);
    }

    #[test]
    fn test_collect_chunk_positions_sorted() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(2, 0, 0));
        chunks.insert(ChunkPos::new(0, 0, 0));
        chunks.insert(ChunkPos::new(1, 0, 0));

        let sorted = collect_chunk_positions_sorted(&chunks);
        assert_eq!(
            sorted,
            vec![
                ChunkPos::new(0, 0, 0),
                ChunkPos::new(1, 0, 0),
                ChunkPos::new(2, 0, 0),
            ]
        );
    }

    #[test]
    fn test_detect_stale_chunks_same_hasher() {
        let expected: HashSet<ChunkPos> = [ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)]
            .into_iter()
            .collect();
        let actual: HashSet<ChunkPos> = [
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(1, 0, 0),
            ChunkPos::new(2, 0, 0),
        ]
        .into_iter()
        .collect();

        let stale = detect_stale_chunks(&expected, &actual);
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0], ChunkPos::new(2, 0, 0));
    }

    #[test]
    fn test_detect_stale_chunks_different_hashers() {
        let expected: HashSet<ChunkPos, RandomState> =
            [ChunkPos::new(0, 0, 0)].into_iter().collect();
        let actual: HashSet<ChunkPos, RandomState> =
            [ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)]
                .into_iter()
                .collect();

        let stale = detect_stale_chunks(&expected, &actual);
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn test_detect_missing_chunks() {
        let expected: HashSet<ChunkPos> = [ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)]
            .into_iter()
            .collect();
        let actual: HashSet<ChunkPos> = [ChunkPos::new(0, 0, 0)].into_iter().collect();

        let missing = detect_missing_chunks(&expected, &actual);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], ChunkPos::new(1, 0, 0));
    }

    #[test]
    fn test_detect_chunk_diff() {
        let expected: HashSet<ChunkPos> = [ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)]
            .into_iter()
            .collect();
        let actual: HashSet<ChunkPos> = [ChunkPos::new(0, 0, 0), ChunkPos::new(2, 0, 0)]
            .into_iter()
            .collect();

        let (stale, missing) = detect_chunk_diff(&expected, &actual);
        assert_eq!(stale.len(), 1);
        assert_eq!(missing.len(), 1);
        assert_eq!(stale[0], ChunkPos::new(2, 0, 0));
        assert_eq!(missing[0], ChunkPos::new(1, 0, 0));
    }

    #[test]
    fn test_rebase_plan_empty() {
        let plan = RebasePlan::default();
        assert!(plan.is_empty());

        let plan_with_offset = RebasePlan::new(RebaseOffset::new(1, 0, 0));
        assert!(!plan_with_offset.is_empty());
    }

    #[test]
    fn test_rebase_fingerprint_determinism() {
        let mut plan1 = RebasePlan::new(RebaseOffset::new(1, 2, 3));
        plan1.mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(1, 2, 3),
            ChunkCategory::Shifted,
        ));

        let mut plan2 = RebasePlan::new(RebaseOffset::new(1, 2, 3));
        plan2.mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(1, 2, 3),
            ChunkCategory::Shifted,
        ));

        assert!(plan1.fingerprint().matches(plan2.fingerprint()));
    }

    #[test]
    fn test_validate_rebase_plan_collision() {
        let mut plan = RebasePlan::new(RebaseOffset::new(-1, 0, 0));
        plan.mapping
            .add_collision(ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0));

        let issues = validate_rebase_plan(&plan);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, RebaseIssueKind::Collision);
    }

    #[test]
    fn test_compute_chunk_checksum() {
        let data1 = vec![1, 2, 3, 4];
        let data2 = vec![1, 2, 3, 4];
        let data3 = vec![4, 3, 2, 1];

        assert_eq!(
            compute_chunk_checksum(&data1),
            compute_chunk_checksum(&data2)
        );
        assert_ne!(
            compute_chunk_checksum(&data1),
            compute_chunk_checksum(&data3)
        );
    }

    #[test]
    fn test_rebase_summary_from_plan_and_result() {
        let mut plan = RebasePlan::new(RebaseOffset::new(10, 0, 0));
        plan.id = Some(RebaseId::generate(42, 1));
        plan.mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(10, 0, 0),
            ChunkCategory::Shifted,
        ));

        let result = RebaseResult {
            chunks_dropped: 0,
            chunks_shifted: 1,
            chunks_unchanged: 0,
            chunks_collided: 0,
            state: RebaseState::Completed,
        };

        let summary = RebaseSummary::from_plan_and_result(&plan, &result);
        assert_eq!(summary.shifted, 1);
        assert_eq!(summary.total_chunks, 1);
        assert!(summary.id.is_some());
    }

    #[test]
    fn test_serde_bincode_roundtrip() {
        let offset = RebaseOffset::new(1, -2, 3);
        let bytes = bincode::serialize(&offset).unwrap();
        let recovered: RebaseOffset = bincode::deserialize(&bytes).unwrap();
        assert_eq!(offset, recovered);

        let id = RebaseId::generate(42, 1);
        let bytes = bincode::serialize(&id).unwrap();
        let recovered: RebaseId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(id, recovered);

        let fingerprint = RebaseFingerprint::from_raw(0xDEAD_BEEF);
        let bytes = bincode::serialize(&fingerprint).unwrap();
        let recovered: RebaseFingerprint = bincode::deserialize(&bytes).unwrap();
        assert_eq!(fingerprint, recovered);
    }

    #[test]
    fn test_serde_bincode_plan_roundtrip() {
        let mut plan = RebasePlan::new(RebaseOffset::new(5, -3, 7));
        plan.id = Some(RebaseId::generate(100, 5));
        plan.mapping.insert(ChunkMappingEntry::new(
            ChunkPos::new(0, 0, 0),
            ChunkPos::new(5, -3, 7),
            ChunkCategory::Shifted,
        ));
        plan.mapping
            .insert(ChunkMappingEntry::dropped(ChunkPos::new(10, 10, 10)));

        let bytes = bincode::serialize(&plan).unwrap();
        let recovered: RebasePlan = bincode::deserialize(&bytes).unwrap();

        assert_eq!(recovered.offset, plan.offset);
        assert_eq!(recovered.id, plan.id);
        assert_eq!(recovered.mapping.len(), plan.mapping.len());
    }

    #[test]
    fn test_serde_bincode_issue_roundtrip() {
        let issue = RebaseIssue {
            kind: RebaseIssueKind::Collision,
            chunk_pos: ChunkPos::new(1, 2, 3),
            other_pos: Some(ChunkPos::new(4, 5, 6)),
            description: "test collision".to_string(),
        };

        let bytes = bincode::serialize(&issue).unwrap();
        let recovered: RebaseIssue = bincode::deserialize(&bytes).unwrap();
        assert_eq!(issue, recovered);
    }

    #[test]
    fn test_rebase_state_properties() {
        assert!(!RebaseState::Idle.is_terminal());
        assert!(!RebaseState::Planning.is_terminal());
        assert!(RebaseState::Completed.is_terminal());
        assert!(RebaseState::Failed.is_terminal());
    }

    #[test]
    fn test_chunk_category_names() {
        assert_eq!(ChunkCategory::Dropped.name(), "dropped");
        assert_eq!(ChunkCategory::Shifted.name(), "shifted");
        assert_eq!(ChunkCategory::Unchanged.name(), "unchanged");
        assert_eq!(ChunkCategory::Colliding.name(), "colliding");
    }

    #[test]
    fn test_moving_world_id() {
        let id = MovingWorldId::new(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn test_anchor_frame_id() {
        let frame = AnchorFrameId::new(5);
        assert_eq!(frame.raw(), 5);
        assert_eq!(AnchorFrameId::WORLD_ORIGIN.raw(), 0);
        assert_eq!(AnchorFrameId::default(), AnchorFrameId::WORLD_ORIGIN);
    }

    #[test]
    fn test_moving_world_pose_advance_offset_roundtrip() {
        let world_id = MovingWorldId::new(1);
        let pose = MovingWorldPose::new(
            world_id,
            AnchorFrameId::WORLD_ORIGIN,
            ChunkPos::new(100, 50, 200),
            0,
        );

        let new_anchor = ChunkPos::new(110, 55, 210);
        let (new_pose, offset) = pose.advance_to(new_anchor);

        assert_eq!(new_pose.anchor_chunk, new_anchor);
        assert_eq!(new_pose.generation, 1);
        assert_eq!(offset, RebaseOffset::new(-10, -5, -10));

        let original_pos = ChunkPos::new(105, 52, 205);
        let rebased = rebased_position(original_pos, offset);
        assert_eq!(rebased, ChunkPos::new(95, 47, 195));

        let back = rebased_position(rebased, offset.inverse());
        assert_eq!(back, original_pos);
    }

    #[test]
    fn test_moving_world_pose_distance_squared() {
        let world_id = MovingWorldId::new(1);
        let pose1 = MovingWorldPose::new(
            world_id,
            AnchorFrameId::WORLD_ORIGIN,
            ChunkPos::new(0, 0, 0),
            0,
        );
        let pose2 = MovingWorldPose::new(
            world_id,
            AnchorFrameId::WORLD_ORIGIN,
            ChunkPos::new(3, 4, 0),
            0,
        );
        assert_eq!(pose1.distance_squared(&pose2), 25);
    }

    #[test]
    fn test_moving_world_registry_checksum_determinism() {
        let mut reg1 = MovingWorldRegistry::new();
        let mut reg2 = MovingWorldRegistry::new();

        let world1 = MovingWorldId::new(1);
        let world2 = MovingWorldId::new(2);
        let pose1 = MovingWorldPose::new(
            world1,
            AnchorFrameId::WORLD_ORIGIN,
            ChunkPos::new(0, 0, 0),
            0,
        );
        let pose2 =
            MovingWorldPose::new(world2, AnchorFrameId::new(1), ChunkPos::new(10, 20, 30), 5);

        reg1.register(world1, pose1);
        reg1.register(world2, pose2);

        reg2.register(world1, pose1);
        reg2.register(world2, pose2);

        assert!(reg1.checksum().matches(reg2.checksum()));

        let mut reg3 = MovingWorldRegistry::new();
        reg3.register(world2, pose2);
        reg3.register(world1, pose1);
        assert!(reg1.checksum().matches(reg3.checksum()));
    }

    #[test]
    fn test_moving_world_registry_operations() {
        let mut registry = MovingWorldRegistry::new();
        assert!(registry.is_empty());

        let world_id = MovingWorldId::new(1);
        let pose = MovingWorldPose::new(
            world_id,
            AnchorFrameId::WORLD_ORIGIN,
            ChunkPos::new(0, 0, 0),
            0,
        );

        assert!(registry.create(world_id, pose));
        assert!(!registry.create(world_id, pose));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        assert_eq!(registry.get(world_id), Some(&pose));
        assert_eq!(registry.get(MovingWorldId::new(999)), None);

        let new_pose = MovingWorldPose::new(
            world_id,
            AnchorFrameId::WORLD_ORIGIN,
            ChunkPos::new(10, 10, 10),
            1,
        );
        let old = registry.update_pose(world_id, new_pose);
        assert_eq!(old, Some(pose));
        assert_eq!(registry.get(world_id), Some(&new_pose));
    }

    #[test]
    fn test_query_helpers_world_to_local() {
        let world_pos = ChunkPos::new(150, 75, 300);
        let anchor = ChunkPos::new(100, 50, 200);
        let local = world_to_local(world_pos, anchor);
        assert_eq!(local, ChunkPos::new(50, 25, 100));
    }

    #[test]
    fn test_query_helpers_local_to_world() {
        let local_pos = ChunkPos::new(50, 25, 100);
        let anchor = ChunkPos::new(100, 50, 200);
        let world = local_to_world(local_pos, anchor);
        assert_eq!(world, ChunkPos::new(150, 75, 300));
    }

    #[test]
    fn test_query_helpers_roundtrip() {
        let anchor = ChunkPos::new(100, 50, 200);
        let world_pos = ChunkPos::new(150, 75, 300);

        let local = world_to_local(world_pos, anchor);
        let back = local_to_world(local, anchor);
        assert_eq!(back, world_pos);
    }

    #[test]
    fn test_rebased_position() {
        let original = ChunkPos::new(10, 20, 30);
        let offset = RebaseOffset::new(5, -10, 15);
        let result = rebased_position(original, offset);
        assert_eq!(result, ChunkPos::new(15, 10, 45));
    }

    // ---------------------------------------------------------------------------
    // New tests for RebaseMode, threshold, budget, and validation
    // ---------------------------------------------------------------------------

    #[test]
    fn test_rebase_mode_names() {
        assert_eq!(RebaseMode::Full.name(), "full");
        assert_eq!(RebaseMode::Partial.name(), "partial");
    }

    #[test]
    fn test_rebase_mode_default() {
        assert_eq!(RebaseMode::default(), RebaseMode::Full);
    }

    #[test]
    fn test_rebase_config_default_values() {
        let config = RebaseConfig::default();
        assert!(!config.allow_collisions);
        assert!(config.drop_out_of_bounds);
        assert!(config.bounds.is_none());
        assert_eq!(config.rebase_threshold_chunks, 1);
        assert_eq!(config.max_chunks_per_plan, usize::MAX);
        assert_eq!(config.mode, RebaseMode::Full);
    }

    #[test]
    fn test_rebase_config_validate_valid() {
        let config = RebaseConfig::default();
        let errors = config.validate();
        assert!(errors.is_empty());
        assert!(config.is_valid());
    }

    #[test]
    fn test_rebase_config_validate_threshold_zero() {
        let config = RebaseConfig {
            rebase_threshold_chunks: 0,
            ..Default::default()
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], RebaseValidationError::ThresholdZero);
        assert!(!config.is_valid());
    }

    #[test]
    fn test_rebase_config_validate_max_chunks_zero() {
        let config = RebaseConfig {
            max_chunks_per_plan: 0,
            ..Default::default()
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], RebaseValidationError::MaxChunksZero);
    }

    #[test]
    fn test_rebase_config_validate_invalid_bounds() {
        let config = RebaseConfig {
            bounds: Some(RebaseBounds::new(10, 5, 0, 10, 0, 10)),
            ..Default::default()
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            RebaseValidationError::InvalidBounds { axis, .. } => assert_eq!(axis, "x"),
            _ => panic!("expected InvalidBounds error"),
        }
    }

    #[test]
    fn test_rebase_config_validate_multiple_errors() {
        let config = RebaseConfig {
            rebase_threshold_chunks: 0,
            max_chunks_per_plan: 0,
            bounds: Some(RebaseBounds::new(10, 5, 20, 10, 30, 20)),
            ..Default::default()
        };
        let errors = config.validate();
        assert_eq!(errors.len(), 5);
    }

    #[test]
    fn test_rebase_config_is_below_threshold() {
        let config = RebaseConfig {
            rebase_threshold_chunks: 5,
            ..Default::default()
        };
        assert!(config.is_below_threshold(RebaseOffset::new(1, 1, 1)));
        assert!(config.is_below_threshold(RebaseOffset::new(2, 1, 1)));
        assert!(!config.is_below_threshold(RebaseOffset::new(3, 1, 1)));
        assert!(!config.is_below_threshold(RebaseOffset::new(5, 0, 0)));
    }

    #[test]
    fn test_rebase_config_exceeds_budget() {
        let config = RebaseConfig {
            max_chunks_per_plan: 100,
            ..Default::default()
        };
        assert!(!config.exceeds_budget(50));
        assert!(!config.exceeds_budget(100));
        assert!(config.exceeds_budget(101));
    }

    #[test]
    fn test_compute_rebase_plan_below_threshold_unchanged() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(0, 0, 0));
        chunks.insert(ChunkPos::new(1, 0, 0));
        chunks.insert(ChunkPos::new(2, 0, 0));

        let offset = RebaseOffset::new(1, 0, 0);
        let config = RebaseConfig {
            rebase_threshold_chunks: 5,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.state, RebaseState::Completed);
        assert_eq!(plan.mapping.len(), 3);
        for entry in plan.mapping.entries_sorted() {
            assert_eq!(entry.category, ChunkCategory::Unchanged);
        }
    }

    #[test]
    fn test_compute_rebase_plan_above_threshold_shifted() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(0, 0, 0));
        chunks.insert(ChunkPos::new(100, 0, 0));

        let offset = RebaseOffset::new(5, 0, 0);
        let config = RebaseConfig {
            rebase_threshold_chunks: 3,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.state, RebaseState::Validating);
        let shifted: Vec<_> = plan
            .mapping
            .entries_sorted()
            .into_iter()
            .filter(|e| e.category == ChunkCategory::Shifted)
            .collect();
        assert_eq!(shifted.len(), 2);
    }

    #[test]
    fn test_compute_rebase_plan_full_mode_budget_exceeded() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        for i in 0..10 {
            chunks.insert(ChunkPos::new(i * 10, 0, 0));
        }

        let offset = RebaseOffset::new(5, 0, 0);
        let config = RebaseConfig {
            max_chunks_per_plan: 5,
            mode: RebaseMode::Full,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.state, RebaseState::Failed);
        assert!(plan.has_validation_errors());
        assert!(matches!(
            plan.validation_errors[0],
            RebaseValidationError::BudgetExceeded {
                chunk_count: 10,
                budget: 5
            }
        ));
    }

    #[test]
    fn test_compute_rebase_plan_partial_mode_budget_respected() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        for i in 0..10 {
            chunks.insert(ChunkPos::new(i * 10, 0, 0));
        }

        let offset = RebaseOffset::new(5, 0, 0);
        let config = RebaseConfig {
            max_chunks_per_plan: 5,
            mode: RebaseMode::Partial,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.state, RebaseState::Validating);
        assert!(!plan.has_validation_errors());
        assert_eq!(plan.mapping.len(), 5);
        assert_eq!(plan.deferred_count, 5);
        assert!(plan.is_partial());
    }

    #[test]
    fn test_compute_rebase_plan_partial_mode_sorted_prefix() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(20, 0, 0));
        chunks.insert(ChunkPos::new(10, 0, 0));
        chunks.insert(ChunkPos::new(30, 0, 0));
        chunks.insert(ChunkPos::new(0, 0, 0));

        let offset = RebaseOffset::new(1, 0, 0);
        let config = RebaseConfig {
            max_chunks_per_plan: 2,
            mode: RebaseMode::Partial,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        let entries = plan.mapping.entries_sorted();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].old_pos, ChunkPos::new(0, 0, 0));
        assert_eq!(entries[1].old_pos, ChunkPos::new(10, 0, 0));
    }

    #[test]
    fn test_compute_rebase_plan_invalid_config_fails() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        chunks.insert(ChunkPos::new(0, 0, 0));

        let offset = RebaseOffset::new(1, 0, 0);
        let config = RebaseConfig {
            rebase_threshold_chunks: 0,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.state, RebaseState::Failed);
        assert!(plan.has_validation_errors());
    }

    #[test]
    fn test_rebase_validation_error_description() {
        let err = RebaseValidationError::ThresholdZero;
        assert!(err.description().contains("threshold"));

        let err = RebaseValidationError::MaxChunksZero;
        assert!(err.description().contains("max_chunks"));

        let err = RebaseValidationError::InvalidBounds {
            axis: "x".to_string(),
            min: 10,
            max: 5,
        };
        assert!(err.description().contains("x axis"));

        let err = RebaseValidationError::BudgetExceeded {
            chunk_count: 100,
            budget: 50,
        };
        assert!(err.description().contains("100"));
        assert!(err.description().contains("50"));
    }

    #[test]
    fn test_rebase_validation_error_serde() {
        let err = RebaseValidationError::BudgetExceeded {
            chunk_count: 100,
            budget: 50,
        };
        let bytes = bincode::serialize(&err).unwrap();
        let recovered: RebaseValidationError = bincode::deserialize(&bytes).unwrap();
        assert_eq!(err, recovered);
    }

    #[test]
    fn test_rebase_mode_serde() {
        let mode = RebaseMode::Partial;
        let bytes = bincode::serialize(&mode).unwrap();
        let recovered: RebaseMode = bincode::deserialize(&bytes).unwrap();
        assert_eq!(mode, recovered);
    }

    #[test]
    fn test_rebase_plan_with_validation_errors_serde() {
        let mut plan = RebasePlan::new(RebaseOffset::new(1, 0, 0));
        plan.validation_errors
            .push(RebaseValidationError::ThresholdZero);
        plan.deferred_count = 5;

        let bytes = bincode::serialize(&plan).unwrap();
        let recovered: RebasePlan = bincode::deserialize(&bytes).unwrap();
        assert_eq!(recovered.validation_errors.len(), 1);
        assert_eq!(recovered.deferred_count, 5);
    }

    #[test]
    fn test_threshold_exact_boundary() {
        let config = RebaseConfig {
            rebase_threshold_chunks: 5,
            ..Default::default()
        };

        assert!(config.is_below_threshold(RebaseOffset::new(4, 0, 0)));
        assert!(!config.is_below_threshold(RebaseOffset::new(5, 0, 0)));
        assert!(!config.is_below_threshold(RebaseOffset::new(6, 0, 0)));
    }

    #[test]
    fn test_partial_mode_exactly_at_budget() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        for i in 0..5 {
            chunks.insert(ChunkPos::new(i * 10, 0, 0));
        }

        let offset = RebaseOffset::new(1, 0, 0);
        let config = RebaseConfig {
            max_chunks_per_plan: 5,
            mode: RebaseMode::Partial,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.mapping.len(), 5);
        assert_eq!(plan.deferred_count, 0);
        assert!(!plan.is_partial());
    }

    #[test]
    fn test_full_mode_exactly_at_budget() {
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        for i in 0..5 {
            chunks.insert(ChunkPos::new(i * 10, 0, 0));
        }

        let offset = RebaseOffset::new(1, 0, 0);
        let config = RebaseConfig {
            max_chunks_per_plan: 5,
            mode: RebaseMode::Full,
            ..Default::default()
        };

        let plan = compute_rebase_plan(&chunks, offset, &config);

        assert_eq!(plan.state, RebaseState::Validating);
        assert!(!plan.has_validation_errors());
        assert_eq!(plan.mapping.len(), 5);
    }
}

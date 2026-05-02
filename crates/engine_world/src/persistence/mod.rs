//! World persistence system.
//!
//! Provides chunk serialization, region-based storage, and world saves.
//!
//! # Multi-State Persistence
//!
//! For alternate dimensions, time-loop snapshots, and phased realities,
//! use the multi-state persistence types:
//!
//! - [`StateId`]: Unique identifier for reality states
//! - [`StateKind`]: Semantic classification of state types
//! - [`MultiStateChunk`]: Container for multiple chunk states
//! - [`MultiStateRegion`]: Region file format for multi-state chunks
//!
//! # Delta/Overlay Persistence
//!
//! For memory-efficient storage of chunk variants:
//!
//! - [`ChunkDelta`]: Compact overlay storing only changed blocks
//! - [`DeltaIndex`]: Compact position index for delta storage
//! - [`DeltaStats`]: Statistics about delta contents
//!
//! # Mutation Journaling
//!
//! For late-join sync and rollback reconciliation:
//!
//! - [`MutationJournal`]: Append-only journal tracking block changes
//! - [`MutationRecord`]: Individual block mutation with old/new state
//! - [`MutationSource`]: Origin system for mutations
//! - [`MutationReason`]: Semantic reason for mutations
//! - [`MutationQuery`]: Query builder for filtering records
//! - [`JournalSnapshot`]: Late-join state snapshot with pending mutations
//!
//! # Save Diff/Repair
//!
//! For savegame comparison and recovery:
//!
//! - [`SnapshotFingerprint`]: Stable fingerprint for world state
//! - [`ChunkChecksum`]: CRC32 checksum for individual chunks
//! - [`SnapshotDiff`]: Detected differences between snapshots
//! - [`RepairIssue`]: Categorized repairable issues
//! - [`RepairPlan`]: Bounded repair operations
//! - [`RepairAnalyzer`]: Issue detection and repair planning
//!
//! # Regional Backup/Restore
//!
//! For deterministic partial world rollback:
//!
//! - [`BackupId`]: Deterministic identifier for backup snapshots
//! - [`BackupMetadata`]: Context and provenance for a backup
//! - [`ChunkEntry`]: Per-chunk data with position and checksum
//! - [`BackupManifest`]: Summary of backup contents and fingerprint
//! - [`RegionalBackup`]: Complete backup with manifest and chunk data
//! - [`RestorePlan`]: Planned restore operations with issue detection
//! - [`RestoreResult`]: Outcome of applied restore operations
//! - [`BackupIssue`]: Issues encountered during backup/restore
//!
//! # Schema Migration
//!
//! For long-lived world saves and version upgrades:
//!
//! - [`SchemaVersion`]: Semantic version for persistence format
//! - [`MigrationStep`]: Single migration transformation
//! - [`MigrationPlan`]: Ordered sequence of migration steps
//! - [`MigrationFixture`]: Test fixture for versioned world state
//! - [`InvariantCheck`]: Post-migration invariant validation
//! - [`MigrationExecutor`]: Migration planning and execution
//!
//! # Admin Tools
//!
//! For replayable world repair and moderation:
//!
//! - [`AdminOpId`]: Deterministic operation identifier
//! - [`AdminMetadata`]: Authorization and context for operations
//! - [`AdminOp`]: Admin operation variants (repair, fill, quarantine, moderation)
//! - [`AdminRecord`]: Logged operation with outcome
//! - [`AdminLog`]: Append-only deterministic operation log
//! - [`AdminQuery`]: Query builder for filtering records
//! - [`DryRunResult`]: Planning result before execution
//!
//! # Parallel Reality Diff/Merge
//!
//! For Fracture-style reality swaps and multi-timeline management:
//!
//! - [`RealityId`]: Unique identifier for a parallel reality
//! - [`RealityBranch`]: Metadata about a reality branch
//! - [`RealityRegistry`]: Registry of all realities and relationships
//! - [`RealityDiff`]: Differences between two realities
//! - [`MergeStrategy`]: Conflict resolution strategies
//! - [`FracturePoint`]: Marked location where reality swap can occur
//! - [`FractureSwap`]: Atomic swap operation between realities

mod admin_tools;
mod chunk_delta;
mod multi_state_chunk;
mod multi_state_region;
mod mutation_journal;
mod parallel_reality;
mod region;
mod regional_backup;
mod save_repair;
mod schema_migration;
mod state_id;
mod time_loop;
mod world_meta;

pub use admin_tools::{
    AdminLog, AdminLogStats, AdminMetadata, AdminOp, AdminOpId, AdminQuery, AdminRecord, AuthLevel,
    BlockFillSpec, BlockReplaceSpec, DryRunResult, MAX_BLOCK_REGION_SIZE, MAX_REGION_BOUND_CHUNKS,
    MarkerCategory, ModerationAction, OpCategory, OpOutcome, PlayerModerationRecord,
    QuarantineSeverity, QuarantineStatus, RegionMarker, ReplayResult, TeleportDestination,
    ValidationResult, WorldBounds,
};
pub use chunk_delta::{ChunkDelta, DeltaIndex, DeltaStats};
pub use multi_state_chunk::{MultiStateChunk, StateFallback};
pub use multi_state_region::{
    MultiStateRegion, MultiStateRegionError, RegionStats, multi_state_region_filename,
};
pub use mutation_journal::{
    JournalSnapshot, JournalStats, MutationJournal, MutationQuery, MutationReason, MutationRecord,
    MutationSource, Sequence,
};
pub use parallel_reality::{
    ChunkConflict, ChunkDiffEntry, ConflictResolution, FractureId, FracturePoint, FractureRegistry,
    FractureRegistrySummary, FractureSwap, MergeResult, MergeStrategy, RealityBranch,
    RealityChecksum, RealityChecksumBuilder, RealityDiff, RealityDiffSummary, RealityError,
    RealityId, RealityRegistry, RealityRegistrySummary, RealityTag, ResolvedConflict, SwapSnapshot,
    merge_diff,
};
pub use region::{
    REGION_SIZE, Region, RegionError, chunk_to_local, chunk_to_region, region_filename,
};
pub use regional_backup::{
    BackupId, BackupIssue, BackupIssueKind, BackupIssueSeverity, BackupManifest, BackupMetadata,
    BackupSummary, ChunkEntry, RegionalBackup, RestoreOp, RestorePlan, RestoreResult,
    apply_restore, compute_restore_delta, verify_against_backup,
};
pub use save_repair::{
    ChunkChecksum, ChunkDiff, DiffSummary, FingerprintBuilder, IssueCategory, IssueSeverity,
    RepairAnalyzer, RepairIssue, RepairOp, RepairPlan, RepairResult, SnapshotDiff,
    SnapshotFingerprint, apply_repairs, compute_chunks_fingerprint, compute_meta_fingerprint,
    verify_checksums,
};
pub use schema_migration::{
    ChunkFixture, CompatibilityReport, InvariantCheck, InvariantKind, InvariantResult, MetaFixture,
    MigrationError, MigrationExecutor, MigrationFixture, MigrationKind, MigrationPlan,
    MigrationResult, MigrationStep, MigrationStepResult, MultiStateFixture, SchemaVersion,
    apply_block_remap, apply_block_remap_delta, compute_plan_fingerprint,
};
pub use state_id::{StateId, StateKind};
pub use time_loop::{
    CommitAction, CommitPlan, ConflictResolutionAction, ExitEvaluation, ExitReason, LoopExitRule,
    LoopIterationId, LoopIterationState, LoopLifecycleState, LoopPlanAction, LoopResetMode,
    LoopResetPlan, LoopRules, LoopWindow, ParadoxConflict, ParadoxDetector, ParadoxGuardPolicy,
    ParadoxKind, ParadoxReport, ParadoxResolutionResult, PersistentLoopDeltas, ProtectedViolation,
    ResolvedParadox, TimeLoopDefinition, TimeLoopError, TimeLoopFingerprint, TimeLoopId,
    TimeLoopRuntime, TimeLoopSnapshot, TimeLoopSummary, TimeLoopValidationError, TimelineId,
};
pub use world_meta::{WorldError, WorldMeta, WorldPersistence};

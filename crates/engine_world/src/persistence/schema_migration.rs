//! Schema migration test harness for long-lived world saves.
//!
//! Provides deterministic, fixture-based testing for schema version migrations
//! without requiring real old binaries. Supports dry-run execution, invariant
//! checking, and compatibility reporting.
//!
//! # Overview
//!
//! - [`SchemaVersion`]: Semantic version identifier for persistence format
//! - [`MigrationStep`]: Single migration transformation (version A -> B)
//! - [`MigrationPlan`]: Ordered sequence of migration steps
//! - [`MigrationFixture`]: Test fixture representing versioned world state
//! - [`InvariantCheck`]: Post-migration invariant validation
//! - [`MigrationResult`]: Outcome of dry-run or apply execution
//! - [`MigrationExecutor`]: Orchestrates migration execution
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::persistence::{MigrationExecutor, MigrationFixture, SchemaVersion};
//!
//! let executor = MigrationExecutor::new();
//! let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0));
//! let plan = executor.plan(fixture.version(), SchemaVersion::current());
//! let result = executor.dry_run(&fixture, &plan);
//! ```

use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::chunk::{BlockId, Chunk};
use crate::persistence::{
    ChunkChecksum, ChunkDelta, MultiStateChunk, SnapshotFingerprint, StateId, WorldMeta,
};

/// Semantic version for persistence schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl SchemaVersion {
    /// Current schema version.
    pub const CURRENT: Self = Self::new(2, 0, 0);

    /// Minimum supported schema version.
    pub const MIN_SUPPORTED: Self = Self::new(1, 0, 0);

    /// Create a new schema version.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Get the current schema version.
    #[must_use]
    pub const fn current() -> Self {
        Self::CURRENT
    }

    /// Get the major version.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// Get the minor version.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }

    /// Get the patch version.
    #[must_use]
    pub const fn patch(self) -> u16 {
        self.patch
    }

    /// Check if this version is compatible with another (same major).
    #[must_use]
    pub const fn is_compatible_with(self, other: Self) -> bool {
        self.major == other.major
    }

    /// Check if this version requires migration to reach target.
    #[must_use]
    pub const fn requires_migration_to(self, target: Self) -> bool {
        self.major < target.major
            || (self.major == target.major && self.minor < target.minor)
            || (self.major == target.major
                && self.minor == target.minor
                && self.patch < target.patch)
    }

    /// Check if this version is supported for migration.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        self.major >= Self::MIN_SUPPORTED.major
    }

    /// Compute a fingerprint for this version.
    #[must_use]
    pub fn fingerprint(self) -> SnapshotFingerprint {
        let mut hasher = crate::persistence::FingerprintBuilder::new();
        hasher.feed_u16(self.major);
        hasher.feed_u16(self.minor);
        hasher.feed_u16(self.patch);
        hasher.build()
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let major = self.major;
        let minor = self.minor;
        let patch = self.patch;
        write!(f, "{major}.{minor}.{patch}")
    }
}

/// Error type for migration operations.
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("unsupported schema version: {0}")]
    UnsupportedVersion(SchemaVersion),

    #[error("no migration path from {from} to {to}")]
    NoMigrationPath {
        from: SchemaVersion,
        to: SchemaVersion,
    },

    #[error("migration step failed: {step} - {reason}")]
    StepFailed { step: String, reason: String },

    #[error("invariant check failed: {name} - {details}")]
    InvariantFailed { name: String, details: String },

    #[error("fixture validation failed: {0}")]
    FixtureInvalid(String),

    #[error("checksum mismatch at {pos:?}: expected {expected:08x}, got {actual:08x}")]
    ChecksumMismatch {
        pos: ChunkPos,
        expected: u32,
        actual: u32,
    },

    #[error("version mismatch: expected {expected}, got {actual}")]
    VersionMismatch {
        expected: SchemaVersion,
        actual: SchemaVersion,
    },
}

/// Type of migration transformation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MigrationKind {
    /// Block ID remapping.
    BlockRemap,
    /// Chunk format change.
    ChunkFormat,
    /// World metadata update.
    MetaUpdate,
    /// State ID migration.
    StateRemap,
    /// Region format change.
    RegionFormat,
    /// Add new fields with defaults.
    AddFields,
    /// Remove deprecated fields.
    RemoveFields,
}

impl MigrationKind {
    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BlockRemap => "block_remap",
            Self::ChunkFormat => "chunk_format",
            Self::MetaUpdate => "meta_update",
            Self::StateRemap => "state_remap",
            Self::RegionFormat => "region_format",
            Self::AddFields => "add_fields",
            Self::RemoveFields => "remove_fields",
        }
    }

    /// Check if this migration type is reversible.
    #[must_use]
    pub const fn is_reversible(self) -> bool {
        matches!(
            self,
            Self::BlockRemap | Self::StateRemap | Self::MetaUpdate | Self::AddFields
        )
    }
}

/// A single migration step from one version to another.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationStep {
    /// Source version.
    pub from: SchemaVersion,
    /// Target version.
    pub to: SchemaVersion,
    /// Kind of migration.
    pub kind: MigrationKind,
    /// Human-readable description.
    pub description: String,
    /// Block ID remapping (if applicable).
    pub block_remap: Option<BTreeMap<u16, u16>>,
    /// State ID remapping (if applicable).
    pub state_remap: Option<BTreeMap<u16, u16>>,
    /// Metadata field changes (if applicable).
    pub meta_changes: Option<BTreeMap<String, String>>,
}

impl MigrationStep {
    /// Create a new migration step.
    #[must_use]
    pub fn new(from: SchemaVersion, to: SchemaVersion, kind: MigrationKind) -> Self {
        Self {
            from,
            to,
            kind,
            description: String::new(),
            block_remap: None,
            state_remap: None,
            meta_changes: None,
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set block remapping.
    #[must_use]
    pub fn with_block_remap(mut self, remap: BTreeMap<u16, u16>) -> Self {
        self.block_remap = Some(remap);
        self
    }

    /// Set state remapping.
    #[must_use]
    pub fn with_state_remap(mut self, remap: BTreeMap<u16, u16>) -> Self {
        self.state_remap = Some(remap);
        self
    }

    /// Set metadata changes.
    #[must_use]
    pub fn with_meta_changes(mut self, changes: BTreeMap<String, String>) -> Self {
        self.meta_changes = Some(changes);
        self
    }

    /// Get a unique identifier for this step.
    #[must_use]
    pub fn id(&self) -> String {
        let from = self.from;
        let to = self.to;
        let kind = self.kind.name();
        format!("{from}_to_{to}_{kind}")
    }

    /// Check if this step is valid (from < to).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.from < self.to
    }

    /// Estimate the cost of this migration (affected blocks).
    #[must_use]
    pub fn estimated_cost(&self) -> usize {
        match self.kind {
            MigrationKind::BlockRemap => self.block_remap.as_ref().map_or(0, BTreeMap::len) * 100,
            MigrationKind::ChunkFormat | MigrationKind::RegionFormat => 1000,
            MigrationKind::MetaUpdate | MigrationKind::AddFields | MigrationKind::RemoveFields => 1,
            MigrationKind::StateRemap => self.state_remap.as_ref().map_or(0, BTreeMap::len) * 50,
        }
    }
}

/// An ordered sequence of migration steps.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Source version.
    pub from: SchemaVersion,
    /// Target version.
    pub to: SchemaVersion,
    /// Ordered steps.
    pub steps: Vec<MigrationStep>,
    /// Whether all steps are reversible.
    pub reversible: bool,
}

impl MigrationPlan {
    /// Create a new empty migration plan.
    #[must_use]
    pub fn new(from: SchemaVersion, to: SchemaVersion) -> Self {
        Self {
            from,
            to,
            steps: Vec::new(),
            reversible: true,
        }
    }

    /// Add a step to the plan.
    pub fn add_step(&mut self, step: MigrationStep) {
        if !step.kind.is_reversible() {
            self.reversible = false;
        }
        self.steps.push(step);
    }

    /// Check if the plan has any steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get the number of steps.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Get total estimated cost.
    #[must_use]
    pub fn total_cost(&self) -> usize {
        self.steps.iter().map(MigrationStep::estimated_cost).sum()
    }

    /// Validate the plan (steps are contiguous and ordered).
    ///
    /// # Errors
    ///
    /// Returns an error if steps are not contiguous or don't match plan bounds.
    pub fn validate(&self) -> Result<(), MigrationError> {
        if self.steps.is_empty() {
            return Ok(());
        }

        if self.steps[0].from != self.from {
            return Err(MigrationError::VersionMismatch {
                expected: self.from,
                actual: self.steps[0].from,
            });
        }

        for window in self.steps.windows(2) {
            if window[0].to != window[1].from {
                return Err(MigrationError::NoMigrationPath {
                    from: window[0].to,
                    to: window[1].from,
                });
            }
        }

        if let Some(last) = self.steps.last()
            && last.to != self.to
        {
            return Err(MigrationError::VersionMismatch {
                expected: self.to,
                actual: last.to,
            });
        }

        Ok(())
    }

    /// Get a summary of migration kinds in the plan.
    #[must_use]
    pub fn kinds_summary(&self) -> BTreeMap<MigrationKind, usize> {
        let mut counts = BTreeMap::new();
        for step in &self.steps {
            *counts.entry(step.kind).or_insert(0) += 1;
        }
        counts
    }
}

/// Versioned world metadata fixture for testing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaFixture {
    /// Schema version of this fixture.
    pub version: SchemaVersion,
    /// World metadata.
    pub meta: WorldMeta,
    /// Expected fingerprint.
    pub fingerprint: SnapshotFingerprint,
}

impl MetaFixture {
    /// Create a new metadata fixture.
    #[must_use]
    pub fn new(version: SchemaVersion, meta: WorldMeta) -> Self {
        let fingerprint = crate::persistence::compute_meta_fingerprint(&meta);
        Self {
            version,
            meta,
            fingerprint,
        }
    }

    /// Validate the fixture fingerprint.
    #[must_use]
    pub fn validate(&self) -> bool {
        let computed = crate::persistence::compute_meta_fingerprint(&self.meta);
        computed.matches(self.fingerprint)
    }
}

/// Versioned chunk fixture for testing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkFixture {
    /// Schema version of this fixture.
    pub version: SchemaVersion,
    /// Chunk position.
    pub pos: ChunkPos,
    /// Chunk data.
    pub chunk: Chunk,
    /// Expected checksum.
    pub checksum: ChunkChecksum,
}

impl ChunkFixture {
    /// Create a new chunk fixture.
    #[must_use]
    pub fn new(version: SchemaVersion, pos: ChunkPos, chunk: Chunk) -> Self {
        let checksum = ChunkChecksum::compute(&chunk);
        Self {
            version,
            pos,
            chunk,
            checksum,
        }
    }

    /// Validate the fixture checksum.
    #[must_use]
    pub fn validate(&self) -> bool {
        let computed = ChunkChecksum::compute(&self.chunk);
        computed.matches(self.checksum)
    }
}

/// Versioned multi-state chunk fixture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiStateFixture {
    /// Schema version of this fixture.
    pub version: SchemaVersion,
    /// Chunk position.
    pub pos: ChunkPos,
    /// Multi-state chunk data.
    pub chunk: MultiStateChunk,
    /// Expected checksums per state.
    pub checksums: BTreeMap<StateId, ChunkChecksum>,
}

impl MultiStateFixture {
    /// Create a new multi-state fixture.
    #[must_use]
    pub fn new(version: SchemaVersion, pos: ChunkPos, chunk: MultiStateChunk) -> Self {
        let mut checksums = BTreeMap::new();
        for (state_id, state_chunk) in chunk.iter() {
            checksums.insert(state_id, ChunkChecksum::compute(state_chunk));
        }
        Self {
            version,
            pos,
            chunk,
            checksums,
        }
    }

    /// Validate all state checksums.
    #[must_use]
    pub fn validate(&self) -> bool {
        for (state_id, expected) in &self.checksums {
            if let Some(chunk) = self.chunk.get(*state_id) {
                let computed = ChunkChecksum::compute(chunk);
                if !computed.matches(*expected) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

/// Complete migration test fixture.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationFixture {
    /// Schema version.
    pub version: SchemaVersion,
    /// Metadata fixture.
    pub meta: Option<MetaFixture>,
    /// Chunk fixtures.
    pub chunks: Vec<ChunkFixture>,
    /// Multi-state fixtures.
    pub multi_state_chunks: Vec<MultiStateFixture>,
    /// Overall fingerprint.
    pub fingerprint: SnapshotFingerprint,
}

impl MigrationFixture {
    /// Create a new empty fixture.
    #[must_use]
    pub fn new(version: SchemaVersion) -> Self {
        Self {
            version,
            ..Default::default()
        }
    }

    /// Set metadata fixture.
    #[must_use]
    pub fn with_meta(mut self, meta: WorldMeta) -> Self {
        self.meta = Some(MetaFixture::new(self.version, meta));
        self.recompute_fingerprint();
        self
    }

    /// Add a chunk fixture.
    #[must_use]
    pub fn with_chunk(mut self, pos: ChunkPos, chunk: Chunk) -> Self {
        self.chunks
            .push(ChunkFixture::new(self.version, pos, chunk));
        self.recompute_fingerprint();
        self
    }

    /// Add a multi-state chunk fixture.
    #[must_use]
    pub fn with_multi_state(mut self, pos: ChunkPos, chunk: MultiStateChunk) -> Self {
        self.multi_state_chunks
            .push(MultiStateFixture::new(self.version, pos, chunk));
        self.recompute_fingerprint();
        self
    }

    /// Get the fixture version.
    #[must_use]
    pub fn version(&self) -> SchemaVersion {
        self.version
    }

    /// Validate all sub-fixtures.
    ///
    /// # Errors
    ///
    /// Returns an error if any fixture checksum or fingerprint is invalid.
    pub fn validate(&self) -> Result<(), MigrationError> {
        if let Some(ref meta) = self.meta
            && !meta.validate()
        {
            return Err(MigrationError::FixtureInvalid(
                "metadata fingerprint mismatch".into(),
            ));
        }

        for chunk_fix in &self.chunks {
            if !chunk_fix.validate() {
                return Err(MigrationError::ChecksumMismatch {
                    pos: chunk_fix.pos,
                    expected: chunk_fix.checksum.value(),
                    actual: ChunkChecksum::compute(&chunk_fix.chunk).value(),
                });
            }
        }

        for ms_fix in &self.multi_state_chunks {
            if !ms_fix.validate() {
                let pos = ms_fix.pos;
                return Err(MigrationError::FixtureInvalid(format!(
                    "multi-state checksum mismatch at {pos:?}"
                )));
            }
        }

        Ok(())
    }

    /// Recompute the overall fingerprint.
    fn recompute_fingerprint(&mut self) {
        let mut builder = crate::persistence::FingerprintBuilder::new();

        builder.feed_u16(self.version.major);
        builder.feed_u16(self.version.minor);
        builder.feed_u16(self.version.patch);

        if let Some(ref meta) = self.meta {
            builder.feed_u32(meta.fingerprint.as_u32());
        }

        for chunk in &self.chunks {
            builder.feed_chunk_pos(chunk.pos);
            builder.feed_u32(chunk.checksum.value());
        }

        for ms in &self.multi_state_chunks {
            builder.feed_chunk_pos(ms.pos);
            for (state_id, checksum) in &ms.checksums {
                builder.feed_u16(state_id.id());
                builder.feed_u32(checksum.value());
            }
        }

        self.fingerprint = builder.build();
    }

    /// Check if fingerprints match.
    #[must_use]
    pub fn fingerprint_matches(&self, other: &Self) -> bool {
        self.fingerprint.matches(other.fingerprint)
    }
}

/// Type of invariant check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantKind {
    /// Block IDs are within valid range.
    BlockIdRange,
    /// Chunk non-air count is accurate.
    BlockCount,
    /// State IDs are valid.
    StateIdValid,
    /// Checksums match.
    ChecksumMatch,
    /// Version is supported.
    VersionSupported,
    /// No data loss.
    NoDataLoss,
    /// Custom invariant.
    Custom,
}

impl InvariantKind {
    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BlockIdRange => "block_id_range",
            Self::BlockCount => "block_count",
            Self::StateIdValid => "state_id_valid",
            Self::ChecksumMatch => "checksum_match",
            Self::VersionSupported => "version_supported",
            Self::NoDataLoss => "no_data_loss",
            Self::Custom => "custom",
        }
    }
}

/// An invariant check definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantCheck {
    /// Kind of invariant.
    pub kind: InvariantKind,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Whether this check is critical.
    pub critical: bool,
}

impl InvariantCheck {
    /// Create a new invariant check.
    #[must_use]
    pub fn new(kind: InvariantKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            description: String::new(),
            critical: false,
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as critical.
    #[must_use]
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }
}

/// Result of a single invariant check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantResult {
    /// The check that was run.
    pub check: InvariantCheck,
    /// Whether it passed.
    pub passed: bool,
    /// Failure details if not passed.
    pub failure_details: Option<String>,
    /// Affected positions (if any).
    pub affected_positions: Vec<ChunkPos>,
}

impl InvariantResult {
    /// Create a passing result.
    #[must_use]
    pub fn pass(check: InvariantCheck) -> Self {
        Self {
            check,
            passed: true,
            failure_details: None,
            affected_positions: Vec::new(),
        }
    }

    /// Create a failing result.
    #[must_use]
    pub fn fail(check: InvariantCheck, details: impl Into<String>) -> Self {
        Self {
            check,
            passed: false,
            failure_details: Some(details.into()),
            affected_positions: Vec::new(),
        }
    }

    /// Add affected positions.
    #[must_use]
    pub fn with_positions(mut self, positions: Vec<ChunkPos>) -> Self {
        self.affected_positions = positions;
        self
    }
}

/// Result of applying a migration step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationStepResult {
    /// Step that was applied.
    pub step_id: String,
    /// Whether it succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Blocks modified.
    pub blocks_modified: usize,
    /// Chunks modified.
    pub chunks_modified: usize,
}

impl MigrationStepResult {
    /// Create a successful result.
    #[must_use]
    pub fn success(step_id: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            ..Default::default()
        }
    }

    /// Create a failed result.
    #[must_use]
    pub fn fail(step_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            success: false,
            error: Some(error.into()),
            ..Default::default()
        }
    }

    /// Set modification counts.
    #[must_use]
    pub fn with_counts(mut self, blocks: usize, chunks: usize) -> Self {
        self.blocks_modified = blocks;
        self.chunks_modified = chunks;
        self
    }
}

/// Complete result of a migration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationResult {
    /// Source version.
    pub from: SchemaVersion,
    /// Target version.
    pub to: SchemaVersion,
    /// Whether it was a dry run.
    pub dry_run: bool,
    /// Whether the migration succeeded.
    pub success: bool,
    /// Results of each step.
    pub step_results: Vec<MigrationStepResult>,
    /// Invariant check results.
    pub invariant_results: Vec<InvariantResult>,
    /// Total blocks modified.
    pub total_blocks_modified: usize,
    /// Total chunks modified.
    pub total_chunks_modified: usize,
    /// Fingerprint before migration.
    pub fingerprint_before: SnapshotFingerprint,
    /// Fingerprint after migration.
    pub fingerprint_after: SnapshotFingerprint,
}

impl MigrationResult {
    /// Create a new result.
    #[must_use]
    pub fn new(from: SchemaVersion, to: SchemaVersion, dry_run: bool) -> Self {
        Self {
            from,
            to,
            dry_run,
            ..Default::default()
        }
    }

    /// Check if all steps succeeded.
    #[must_use]
    pub fn all_steps_succeeded(&self) -> bool {
        self.step_results.iter().all(|r| r.success)
    }

    /// Check if all invariants passed.
    #[must_use]
    pub fn all_invariants_passed(&self) -> bool {
        self.invariant_results.iter().all(|r| r.passed)
    }

    /// Check if any critical invariants failed.
    #[must_use]
    pub fn has_critical_failures(&self) -> bool {
        self.invariant_results
            .iter()
            .any(|r| !r.passed && r.check.critical)
    }

    /// Get count of failed steps.
    #[must_use]
    pub fn failed_step_count(&self) -> usize {
        self.step_results.iter().filter(|r| !r.success).count()
    }

    /// Get count of failed invariants.
    #[must_use]
    pub fn failed_invariant_count(&self) -> usize {
        self.invariant_results.iter().filter(|r| !r.passed).count()
    }
}

/// Compatibility report for a migration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompatibilityReport {
    /// Source version.
    pub source: SchemaVersion,
    /// Target version.
    pub target: SchemaVersion,
    /// Whether migration is possible.
    pub can_migrate: bool,
    /// Required migration steps.
    pub required_steps: usize,
    /// Estimated cost.
    pub estimated_cost: usize,
    /// Whether the migration is reversible.
    pub reversible: bool,
    /// Warnings.
    pub warnings: Vec<String>,
    /// Blockers.
    pub blockers: Vec<String>,
}

impl CompatibilityReport {
    /// Create a new report.
    #[must_use]
    pub fn new(source: SchemaVersion, target: SchemaVersion) -> Self {
        Self {
            source,
            target,
            ..Default::default()
        }
    }

    /// Mark as compatible.
    #[must_use]
    pub fn compatible(mut self, plan: &MigrationPlan) -> Self {
        self.can_migrate = true;
        self.required_steps = plan.step_count();
        self.estimated_cost = plan.total_cost();
        self.reversible = plan.reversible;
        self
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add a blocker.
    pub fn add_blocker(&mut self, blocker: impl Into<String>) {
        self.blockers.push(blocker.into());
        self.can_migrate = false;
    }
}

/// Migration executor for planning and running migrations.
#[derive(Debug, Default)]
pub struct MigrationExecutor {
    /// Registered migration steps.
    steps: Vec<MigrationStep>,
    /// Registered invariant checks.
    invariants: Vec<InvariantCheck>,
    /// Maximum valid block ID.
    max_block_id: u16,
}

impl MigrationExecutor {
    /// Create a new executor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            invariants: Vec::new(),
            max_block_id: 1000,
        }
    }

    /// Create with standard migration steps and invariants.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut executor = Self::new();
        executor.register_default_invariants();
        executor
    }

    /// Set the maximum valid block ID.
    pub fn set_max_block_id(&mut self, max: u16) {
        self.max_block_id = max;
    }

    /// Register a migration step.
    pub fn register_step(&mut self, step: MigrationStep) {
        self.steps.push(step);
    }

    /// Register an invariant check.
    pub fn register_invariant(&mut self, check: InvariantCheck) {
        self.invariants.push(check);
    }

    /// Register default invariants.
    pub fn register_default_invariants(&mut self) {
        self.invariants.push(
            InvariantCheck::new(InvariantKind::BlockIdRange, "block_id_range")
                .with_description("All block IDs are within valid range")
                .critical(),
        );

        self.invariants.push(
            InvariantCheck::new(InvariantKind::BlockCount, "block_count")
                .with_description("Chunk non-air counts are accurate"),
        );

        self.invariants.push(
            InvariantCheck::new(InvariantKind::ChecksumMatch, "checksum_match")
                .with_description("Chunk checksums match after migration")
                .critical(),
        );

        self.invariants.push(
            InvariantCheck::new(InvariantKind::VersionSupported, "version_supported")
                .with_description("Target version is supported")
                .critical(),
        );
    }

    /// Get all registered steps.
    #[must_use]
    pub fn steps(&self) -> &[MigrationStep] {
        &self.steps
    }

    /// Get all registered invariants.
    #[must_use]
    pub fn invariants(&self) -> &[InvariantCheck] {
        &self.invariants
    }

    /// Plan a migration from source to target version.
    ///
    /// # Errors
    ///
    /// Returns an error if the source version is unsupported, if attempting
    /// a downgrade, or if no migration path exists between versions.
    pub fn plan(
        &self,
        from: SchemaVersion,
        to: SchemaVersion,
    ) -> Result<MigrationPlan, MigrationError> {
        if !from.is_supported() {
            return Err(MigrationError::UnsupportedVersion(from));
        }

        let mut plan = MigrationPlan::new(from, to);

        if from == to {
            return Ok(plan);
        }

        if from > to {
            return Err(MigrationError::NoMigrationPath { from, to });
        }

        let mut current = from;
        while current < to {
            let next_step = self
                .steps
                .iter()
                .filter(|s| s.from == current && s.to <= to)
                .max_by_key(|s| s.to);

            if let Some(step) = next_step {
                plan.add_step(step.clone());
                current = step.to;
            } else {
                return Err(MigrationError::NoMigrationPath { from: current, to });
            }
        }

        plan.validate()?;
        Ok(plan)
    }

    /// Generate a compatibility report.
    #[must_use]
    pub fn check_compatibility(
        &self,
        source: SchemaVersion,
        target: SchemaVersion,
    ) -> CompatibilityReport {
        let mut report = CompatibilityReport::new(source, target);

        if !source.is_supported() {
            report.add_blocker(format!("Source version {source} is not supported"));
            return report;
        }

        if source > target {
            report.add_blocker("Downgrade migrations are not supported".to_string());
            return report;
        }

        match self.plan(source, target) {
            Ok(plan) => {
                report = report.compatible(&plan);

                if !plan.reversible {
                    report.add_warning("Migration contains irreversible steps".to_string());
                }

                if plan.total_cost() > 10000 {
                    let cost = plan.total_cost();
                    report.add_warning(format!("High migration cost: {cost} estimated operations"));
                }
            }
            Err(e) => {
                report.add_blocker(format!("Cannot create migration plan: {e}"));
            }
        }

        report
    }

    /// Execute a dry-run migration.
    #[must_use]
    pub fn dry_run(&self, fixture: &MigrationFixture, plan: &MigrationPlan) -> MigrationResult {
        let mut result = MigrationResult::new(plan.from, plan.to, true);
        result.fingerprint_before = fixture.fingerprint;

        for step in &plan.steps {
            let step_result = self.simulate_step(fixture, step);
            result.total_blocks_modified += step_result.blocks_modified;
            result.total_chunks_modified += step_result.chunks_modified;
            result.step_results.push(step_result);
        }

        result.invariant_results = self.check_invariants(fixture);
        result.success = result.all_steps_succeeded() && !result.has_critical_failures();
        result.fingerprint_after = fixture.fingerprint;

        result
    }

    /// Apply a migration to chunks.
    pub fn apply<S: BuildHasher>(
        &self,
        chunks: &mut HashMap<ChunkPos, Chunk, S>,
        plan: &MigrationPlan,
    ) -> MigrationResult {
        let mut result = MigrationResult::new(plan.from, plan.to, false);
        result.fingerprint_before = crate::persistence::compute_chunks_fingerprint(chunks);

        for step in &plan.steps {
            let step_result = self.apply_step(chunks, step);
            if !step_result.success {
                result.step_results.push(step_result);
                result.success = false;
                return result;
            }
            result.total_blocks_modified += step_result.blocks_modified;
            result.total_chunks_modified += step_result.chunks_modified;
            result.step_results.push(step_result);
        }

        result.invariant_results = self.check_chunk_invariants(chunks);
        result.success = result.all_steps_succeeded() && !result.has_critical_failures();
        result.fingerprint_after = crate::persistence::compute_chunks_fingerprint(chunks);

        result
    }

    /// Simulate a single step without modifying data.
    fn simulate_step(
        &self,
        fixture: &MigrationFixture,
        step: &MigrationStep,
    ) -> MigrationStepResult {
        let _ = self;
        let mut result = MigrationStepResult::success(step.id());

        match step.kind {
            MigrationKind::BlockRemap => {
                if let Some(ref remap) = step.block_remap {
                    for chunk_fix in &fixture.chunks {
                        for (_, block) in chunk_fix.chunk.iter() {
                            if remap.contains_key(&block.0) {
                                result.blocks_modified += 1;
                            }
                        }
                        if result.blocks_modified > 0 {
                            result.chunks_modified += 1;
                        }
                    }
                }
            }
            MigrationKind::MetaUpdate => {
                if fixture.meta.is_some() {
                    result.chunks_modified = 1;
                }
            }
            _ => {
                result.chunks_modified = fixture.chunks.len();
            }
        }

        result
    }

    /// Apply a single step to chunks.
    fn apply_step<S: BuildHasher>(
        &self,
        chunks: &mut HashMap<ChunkPos, Chunk, S>,
        step: &MigrationStep,
    ) -> MigrationStepResult {
        let _ = self;
        let mut result = MigrationStepResult::success(step.id());

        match step.kind {
            MigrationKind::BlockRemap => {
                if let Some(ref remap) = step.block_remap {
                    for chunk in chunks.values_mut() {
                        let mut modified = false;
                        let blocks = chunk.blocks_mut();
                        for block in blocks.iter_mut() {
                            if let Some(&new_id) = remap.get(&block.0) {
                                *block = BlockId(new_id);
                                result.blocks_modified += 1;
                                modified = true;
                            }
                        }
                        if modified {
                            chunk.recalculate_count();
                            result.chunks_modified += 1;
                        }
                    }
                }
            }
            MigrationKind::ChunkFormat | MigrationKind::RegionFormat => {
                result.chunks_modified = chunks.len();
            }
            _ => {}
        }

        result
    }

    /// Check invariants on a fixture.
    #[must_use]
    pub fn check_invariants(&self, fixture: &MigrationFixture) -> Vec<InvariantResult> {
        let mut results = Vec::new();

        for check in &self.invariants {
            let result = match check.kind {
                InvariantKind::BlockIdRange => {
                    self.check_block_id_range_fixture(fixture, check.clone())
                }
                InvariantKind::BlockCount => self.check_block_count_fixture(fixture, check.clone()),
                InvariantKind::ChecksumMatch => self.check_checksum_fixture(fixture, check.clone()),
                InvariantKind::VersionSupported => {
                    self.check_version_supported(fixture.version, check.clone())
                }
                _ => InvariantResult::pass(check.clone()),
            };
            results.push(result);
        }

        results
    }

    /// Check invariants on chunks.
    fn check_chunk_invariants<S: BuildHasher>(
        &self,
        chunks: &HashMap<ChunkPos, Chunk, S>,
    ) -> Vec<InvariantResult> {
        let mut results = Vec::new();

        for check in &self.invariants {
            let result = match check.kind {
                InvariantKind::BlockIdRange => {
                    self.check_block_id_range_chunks(chunks, check.clone())
                }
                InvariantKind::BlockCount => self.check_block_count_chunks(chunks, check.clone()),
                _ => InvariantResult::pass(check.clone()),
            };
            results.push(result);
        }

        results
    }

    fn check_block_id_range_fixture(
        &self,
        fixture: &MigrationFixture,
        check: InvariantCheck,
    ) -> InvariantResult {
        let mut invalid_positions = Vec::new();

        for chunk_fix in &fixture.chunks {
            for (_, block) in chunk_fix.chunk.iter() {
                if block.0 > self.max_block_id && block.0 != 0 {
                    invalid_positions.push(chunk_fix.pos);
                    break;
                }
            }
        }

        if invalid_positions.is_empty() {
            InvariantResult::pass(check)
        } else {
            let count = invalid_positions.len();
            InvariantResult::fail(
                check,
                format!("Found {count} chunks with invalid block IDs"),
            )
            .with_positions(invalid_positions)
        }
    }

    fn check_block_id_range_chunks<S: BuildHasher>(
        &self,
        chunks: &HashMap<ChunkPos, Chunk, S>,
        check: InvariantCheck,
    ) -> InvariantResult {
        let mut invalid_positions = Vec::new();

        for (&pos, chunk) in chunks {
            for (_, block) in chunk.iter() {
                if block.0 > self.max_block_id && block.0 != 0 {
                    invalid_positions.push(pos);
                    break;
                }
            }
        }

        if invalid_positions.is_empty() {
            InvariantResult::pass(check)
        } else {
            let count = invalid_positions.len();
            InvariantResult::fail(
                check,
                format!("Found {count} chunks with invalid block IDs"),
            )
            .with_positions(invalid_positions)
        }
    }

    fn check_block_count_fixture(
        &self,
        fixture: &MigrationFixture,
        check: InvariantCheck,
    ) -> InvariantResult {
        let _ = self;
        let mut invalid_positions = Vec::new();

        for chunk_fix in &fixture.chunks {
            let count = chunk_fix.chunk.blocks().iter().filter(|b| b.0 != 0).count();
            let actual = u32::try_from(count).unwrap_or(u32::MAX);
            if chunk_fix.chunk.non_air_count() != actual {
                invalid_positions.push(chunk_fix.pos);
            }
        }

        if invalid_positions.is_empty() {
            InvariantResult::pass(check)
        } else {
            let count = invalid_positions.len();
            InvariantResult::fail(
                check,
                format!("Found {count} chunks with incorrect block counts"),
            )
            .with_positions(invalid_positions)
        }
    }

    fn check_block_count_chunks<S: BuildHasher>(
        &self,
        chunks: &HashMap<ChunkPos, Chunk, S>,
        check: InvariantCheck,
    ) -> InvariantResult {
        let _ = self;
        let mut invalid_positions = Vec::new();

        for (&pos, chunk) in chunks {
            let count = chunk.blocks().iter().filter(|b| b.0 != 0).count();
            let actual = u32::try_from(count).unwrap_or(u32::MAX);
            if chunk.non_air_count() != actual {
                invalid_positions.push(pos);
            }
        }

        if invalid_positions.is_empty() {
            InvariantResult::pass(check)
        } else {
            let count = invalid_positions.len();
            InvariantResult::fail(
                check,
                format!("Found {count} chunks with incorrect block counts"),
            )
            .with_positions(invalid_positions)
        }
    }

    fn check_checksum_fixture(
        &self,
        fixture: &MigrationFixture,
        check: InvariantCheck,
    ) -> InvariantResult {
        let _ = self;
        let mut mismatched = Vec::new();

        for chunk_fix in &fixture.chunks {
            if !chunk_fix.validate() {
                mismatched.push(chunk_fix.pos);
            }
        }

        if mismatched.is_empty() {
            InvariantResult::pass(check)
        } else {
            let count = mismatched.len();
            InvariantResult::fail(
                check,
                format!("Found {count} chunks with checksum mismatches"),
            )
            .with_positions(mismatched)
        }
    }

    fn check_version_supported(
        &self,
        version: SchemaVersion,
        check: InvariantCheck,
    ) -> InvariantResult {
        let _ = self;
        if version.is_supported() {
            InvariantResult::pass(check)
        } else {
            InvariantResult::fail(check, format!("Version {version} is not supported"))
        }
    }
}

/// Apply a block ID remap to a chunk.
pub fn apply_block_remap(chunk: &mut Chunk, remap: &BTreeMap<u16, u16>) -> usize {
    let mut modified = 0;
    let blocks = chunk.blocks_mut();

    for block in blocks.iter_mut() {
        if let Some(&new_id) = remap.get(&block.0) {
            *block = BlockId(new_id);
            modified += 1;
        }
    }

    if modified > 0 {
        chunk.recalculate_count();
    }

    modified
}

/// Apply a block ID remap to a chunk delta.
pub fn apply_block_remap_delta(delta: &mut ChunkDelta, remap: &BTreeMap<u16, u16>) -> usize {
    let mut modified = 0;
    let mut updates = Vec::new();

    for (pos, block) in delta.iter() {
        if let Some(&new_id) = remap.get(&block.0) {
            updates.push((pos, BlockId(new_id)));
            modified += 1;
        }
    }

    for (pos, block) in updates {
        delta.set(pos, block);
    }

    modified
}

/// Compute a stable fingerprint for a migration plan.
#[must_use]
pub fn compute_plan_fingerprint(plan: &MigrationPlan) -> SnapshotFingerprint {
    let mut builder = crate::persistence::FingerprintBuilder::new();

    builder.feed_u16(plan.from.major);
    builder.feed_u16(plan.from.minor);
    builder.feed_u16(plan.from.patch);
    builder.feed_u16(plan.to.major);
    builder.feed_u16(plan.to.minor);
    builder.feed_u16(plan.to.patch);

    for step in &plan.steps {
        builder.feed_bytes(step.id().as_bytes());
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::STONE;
    use engine_core::coords::LocalPos;

    fn test_chunk() -> Chunk {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(1, 0, 0), STONE);
        chunk.set(LocalPos::new(0, 1, 0), BlockId(100));
        chunk
    }

    fn test_meta() -> WorldMeta {
        WorldMeta::new(12345, "Test World")
    }

    // SchemaVersion tests

    #[test]
    fn test_schema_version_new() {
        let v = SchemaVersion::new(1, 2, 3);
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 2);
        assert_eq!(v.patch(), 3);
    }

    #[test]
    fn test_schema_version_current() {
        let v = SchemaVersion::current();
        assert_eq!(v, SchemaVersion::CURRENT);
    }

    #[test]
    fn test_schema_version_ordering() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 1, 0);
        let v3 = SchemaVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_schema_version_compatible() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 5, 0);
        let v3 = SchemaVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(v2));
        assert!(!v1.is_compatible_with(v3));
    }

    #[test]
    fn test_schema_version_requires_migration() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 1, 0);
        let v3 = SchemaVersion::new(1, 1, 1);

        assert!(v1.requires_migration_to(v2));
        assert!(v2.requires_migration_to(v3));
        assert!(!v3.requires_migration_to(v1));
    }

    #[test]
    fn test_schema_version_supported() {
        assert!(SchemaVersion::new(1, 0, 0).is_supported());
        assert!(SchemaVersion::new(2, 0, 0).is_supported());
        assert!(!SchemaVersion::new(0, 9, 0).is_supported());
    }

    #[test]
    fn test_schema_version_fingerprint_deterministic() {
        let v = SchemaVersion::new(1, 2, 3);
        let fp1 = v.fingerprint();
        let fp2 = v.fingerprint();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_schema_version_display() {
        let v = SchemaVersion::new(1, 2, 3);
        assert_eq!(format!("{v}"), "1.2.3");
    }

    #[test]
    fn test_schema_version_serde() {
        let v = SchemaVersion::new(1, 2, 3);
        let json = serde_json::to_string(&v).unwrap();
        let recovered: SchemaVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(v, recovered);
    }

    // MigrationStep tests

    #[test]
    fn test_migration_step_new() {
        let step = MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        );
        assert!(step.is_valid());
    }

    #[test]
    fn test_migration_step_invalid() {
        let step = MigrationStep::new(
            SchemaVersion::new(2, 0, 0),
            SchemaVersion::new(1, 0, 0),
            MigrationKind::BlockRemap,
        );
        assert!(!step.is_valid());
    }

    #[test]
    fn test_migration_step_with_block_remap() {
        let mut remap = BTreeMap::new();
        remap.insert(1, 2);
        remap.insert(3, 4);

        let step = MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        )
        .with_block_remap(remap);

        assert!(step.block_remap.is_some());
        assert_eq!(step.block_remap.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_migration_step_id() {
        let step = MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        );
        assert_eq!(step.id(), "1.0.0_to_1.1.0_block_remap");
    }

    #[test]
    fn test_migration_step_estimated_cost() {
        let mut remap = BTreeMap::new();
        remap.insert(1, 2);
        remap.insert(3, 4);

        let step = MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        )
        .with_block_remap(remap);

        assert_eq!(step.estimated_cost(), 200);
    }

    #[test]
    fn test_migration_kind_reversible() {
        assert!(MigrationKind::BlockRemap.is_reversible());
        assert!(MigrationKind::StateRemap.is_reversible());
        assert!(!MigrationKind::ChunkFormat.is_reversible());
        assert!(!MigrationKind::RegionFormat.is_reversible());
    }

    // MigrationPlan tests

    #[test]
    fn test_migration_plan_empty() {
        let plan = MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 0, 0));
        assert!(plan.is_empty());
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_migration_plan_add_step() {
        let mut plan = MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 2, 0));

        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        ));

        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 1, 0),
            SchemaVersion::new(1, 2, 0),
            MigrationKind::MetaUpdate,
        ));

        assert_eq!(plan.step_count(), 2);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_migration_plan_validate_gap() {
        let mut plan = MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 3, 0));

        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        ));

        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 2, 0),
            SchemaVersion::new(1, 3, 0),
            MigrationKind::MetaUpdate,
        ));

        assert!(plan.validate().is_err());
    }

    #[test]
    fn test_migration_plan_reversible() {
        let mut plan = MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 1, 0));

        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        ));

        assert!(plan.reversible);

        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 1, 0),
            SchemaVersion::new(1, 2, 0),
            MigrationKind::ChunkFormat,
        ));

        assert!(!plan.reversible);
    }

    // MigrationFixture tests

    #[test]
    fn test_fixture_new() {
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0));
        assert_eq!(fixture.version(), SchemaVersion::new(1, 0, 0));
        assert!(fixture.validate().is_ok());
    }

    #[test]
    fn test_fixture_with_meta() {
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0)).with_meta(test_meta());
        assert!(fixture.meta.is_some());
        assert!(fixture.validate().is_ok());
    }

    #[test]
    fn test_fixture_with_chunk() {
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());
        assert_eq!(fixture.chunks.len(), 1);
        assert!(fixture.validate().is_ok());
    }

    #[test]
    fn test_fixture_fingerprint_deterministic() {
        let fixture1 = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_meta(test_meta())
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let fixture2 = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_meta(test_meta())
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        assert!(fixture1.fingerprint_matches(&fixture2));
    }

    #[test]
    fn test_fixture_fingerprint_differs() {
        let fixture1 = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let fixture2 = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(1, 0, 0), test_chunk());

        assert!(!fixture1.fingerprint_matches(&fixture2));
    }

    // InvariantCheck tests

    #[test]
    fn test_invariant_check_new() {
        let check = InvariantCheck::new(InvariantKind::BlockIdRange, "test_check");
        assert_eq!(check.kind, InvariantKind::BlockIdRange);
        assert_eq!(check.name, "test_check");
        assert!(!check.critical);
    }

    #[test]
    fn test_invariant_check_critical() {
        let check = InvariantCheck::new(InvariantKind::BlockIdRange, "test_check").critical();
        assert!(check.critical);
    }

    #[test]
    fn test_invariant_result_pass() {
        let check = InvariantCheck::new(InvariantKind::BlockCount, "test");
        let result = InvariantResult::pass(check);
        assert!(result.passed);
        assert!(result.failure_details.is_none());
    }

    #[test]
    fn test_invariant_result_fail() {
        let check = InvariantCheck::new(InvariantKind::BlockCount, "test");
        let result = InvariantResult::fail(check, "something went wrong");
        assert!(!result.passed);
        assert!(result.failure_details.is_some());
    }

    // MigrationExecutor tests

    #[test]
    fn test_executor_new() {
        let executor = MigrationExecutor::new();
        assert!(executor.steps().is_empty());
        assert!(executor.invariants().is_empty());
    }

    #[test]
    fn test_executor_with_defaults() {
        let executor = MigrationExecutor::with_defaults();
        assert!(!executor.invariants().is_empty());
    }

    #[test]
    fn test_executor_register_step() {
        let mut executor = MigrationExecutor::new();
        executor.register_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        ));
        assert_eq!(executor.steps().len(), 1);
    }

    #[test]
    fn test_executor_plan_same_version() {
        let executor = MigrationExecutor::new();
        let plan = executor
            .plan(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 0, 0))
            .unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn test_executor_plan_with_steps() {
        let mut executor = MigrationExecutor::new();
        executor.register_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        ));
        executor.register_step(MigrationStep::new(
            SchemaVersion::new(1, 1, 0),
            SchemaVersion::new(2, 0, 0),
            MigrationKind::ChunkFormat,
        ));

        let plan = executor
            .plan(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0))
            .unwrap();

        assert_eq!(plan.step_count(), 2);
    }

    #[test]
    fn test_executor_plan_no_path() {
        let executor = MigrationExecutor::new();
        let result = executor.plan(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_executor_plan_unsupported() {
        let executor = MigrationExecutor::new();
        let result = executor.plan(SchemaVersion::new(0, 5, 0), SchemaVersion::new(1, 0, 0));
        assert!(matches!(result, Err(MigrationError::UnsupportedVersion(_))));
    }

    #[test]
    fn test_executor_dry_run() {
        let mut executor = MigrationExecutor::with_defaults();
        executor.register_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::MetaUpdate,
        ));

        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_meta(test_meta())
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let plan = executor
            .plan(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 1, 0))
            .unwrap();

        let result = executor.dry_run(&fixture, &plan);
        assert!(result.dry_run);
        assert!(result.success);
    }

    #[test]
    fn test_executor_apply_block_remap() {
        let mut executor = MigrationExecutor::with_defaults();

        let mut remap = BTreeMap::new();
        remap.insert(STONE.0, 200);

        executor.register_step(
            MigrationStep::new(
                SchemaVersion::new(1, 0, 0),
                SchemaVersion::new(1, 1, 0),
                MigrationKind::BlockRemap,
            )
            .with_block_remap(remap),
        );

        let mut chunks = HashMap::new();
        chunks.insert(ChunkPos::new(0, 0, 0), test_chunk());

        let plan = executor
            .plan(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 1, 0))
            .unwrap();

        let result = executor.apply(&mut chunks, &plan);
        assert!(result.success);
        assert!(result.total_blocks_modified > 0);

        let chunk = chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(chunk.get(LocalPos::new(0, 0, 0)), BlockId(200));
    }

    #[test]
    fn test_executor_check_compatibility() {
        let mut executor = MigrationExecutor::new();
        executor.register_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            MigrationKind::ChunkFormat,
        ));

        let report =
            executor.check_compatibility(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));

        assert!(report.can_migrate);
        assert_eq!(report.required_steps, 1);
        assert!(!report.reversible);
    }

    #[test]
    fn test_executor_check_compatibility_no_path() {
        let executor = MigrationExecutor::new();
        let report =
            executor.check_compatibility(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));

        assert!(!report.can_migrate);
        assert!(!report.blockers.is_empty());
    }

    // apply_block_remap tests

    #[test]
    fn test_apply_block_remap_function() {
        let mut chunk = test_chunk();
        let mut remap = BTreeMap::new();
        remap.insert(STONE.0, 999);

        let modified = apply_block_remap(&mut chunk, &remap);
        assert_eq!(modified, 2);
        assert_eq!(chunk.get(LocalPos::new(0, 0, 0)), BlockId(999));
    }

    #[test]
    fn test_apply_block_remap_delta() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(1, 0, 0), BlockId(100));

        let mut remap = BTreeMap::new();
        remap.insert(STONE.0, 500);

        let modified = apply_block_remap_delta(&mut delta, &remap);
        assert_eq!(modified, 1);
        assert_eq!(delta.get(LocalPos::new(0, 0, 0)), Some(BlockId(500)));
        assert_eq!(delta.get(LocalPos::new(1, 0, 0)), Some(BlockId(100)));
    }

    // MigrationResult tests

    #[test]
    fn test_migration_result_new() {
        let result = MigrationResult::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            false,
        );
        assert!(!result.dry_run);
        assert!(!result.success);
    }

    #[test]
    fn test_migration_result_all_steps_succeeded() {
        let mut result = MigrationResult::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            false,
        );
        result
            .step_results
            .push(MigrationStepResult::success("step1"));
        result
            .step_results
            .push(MigrationStepResult::success("step2"));

        assert!(result.all_steps_succeeded());
    }

    #[test]
    fn test_migration_result_failed_steps() {
        let mut result = MigrationResult::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            false,
        );
        result
            .step_results
            .push(MigrationStepResult::success("step1"));
        result
            .step_results
            .push(MigrationStepResult::fail("step2", "error"));

        assert!(!result.all_steps_succeeded());
        assert_eq!(result.failed_step_count(), 1);
    }

    #[test]
    fn test_migration_result_critical_failures() {
        let mut result = MigrationResult::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            false,
        );

        let critical_check =
            InvariantCheck::new(InvariantKind::BlockIdRange, "critical").critical();
        result
            .invariant_results
            .push(InvariantResult::fail(critical_check, "failed"));

        assert!(result.has_critical_failures());
    }

    // Serde roundtrip tests

    #[test]
    fn test_serde_migration_step() {
        let step = MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        )
        .with_description("test step");

        let json = serde_json::to_string(&step).unwrap();
        let recovered: MigrationStep = serde_json::from_str(&json).unwrap();

        assert_eq!(step.from, recovered.from);
        assert_eq!(step.to, recovered.to);
        assert_eq!(step.kind, recovered.kind);
    }

    #[test]
    fn test_serde_migration_plan() {
        let mut plan = MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 1, 0));
        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::MetaUpdate,
        ));

        let json = serde_json::to_string(&plan).unwrap();
        let recovered: MigrationPlan = serde_json::from_str(&json).unwrap();

        assert_eq!(plan.from, recovered.from);
        assert_eq!(plan.step_count(), recovered.step_count());
    }

    #[test]
    fn test_serde_fixture() {
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_meta(test_meta())
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let json = serde_json::to_string(&fixture).unwrap();
        let recovered: MigrationFixture = serde_json::from_str(&json).unwrap();

        assert_eq!(fixture.version, recovered.version);
        assert_eq!(fixture.chunks.len(), recovered.chunks.len());
    }

    #[test]
    fn test_bincode_migration_plan() {
        let mut plan = MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(1, 1, 0));
        plan.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(1, 1, 0),
            MigrationKind::BlockRemap,
        ));

        let bytes = bincode::serialize(&plan).unwrap();
        let recovered: MigrationPlan = bincode::deserialize(&bytes).unwrap();

        assert_eq!(plan.from, recovered.from);
        assert_eq!(plan.to, recovered.to);
    }

    #[test]
    fn test_bincode_fixture() {
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let bytes = bincode::serialize(&fixture).unwrap();
        let recovered: MigrationFixture = bincode::deserialize(&bytes).unwrap();

        assert!(recovered.validate().is_ok());
    }

    // Plan fingerprint test

    #[test]
    fn test_plan_fingerprint_deterministic() {
        let mut plan1 =
            MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));
        plan1.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            MigrationKind::ChunkFormat,
        ));

        let mut plan2 =
            MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));
        plan2.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            MigrationKind::ChunkFormat,
        ));

        let fp1 = compute_plan_fingerprint(&plan1);
        let fp2 = compute_plan_fingerprint(&plan2);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_plan_fingerprint_differs() {
        let mut plan1 =
            MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));
        plan1.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            MigrationKind::ChunkFormat,
        ));

        let mut plan2 =
            MigrationPlan::new(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));
        plan2.add_step(MigrationStep::new(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            MigrationKind::BlockRemap,
        ));

        let fp1 = compute_plan_fingerprint(&plan1);
        let fp2 = compute_plan_fingerprint(&plan2);

        assert_ne!(fp1, fp2);
    }

    // Multi-state fixture tests

    #[test]
    fn test_multi_state_fixture() {
        let mut msc = MultiStateChunk::new(test_chunk());
        msc.insert(StateId::new(1), Chunk::new());

        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_multi_state(ChunkPos::new(0, 0, 0), msc);

        assert_eq!(fixture.multi_state_chunks.len(), 1);
        assert!(fixture.validate().is_ok());
    }

    #[test]
    fn test_multi_state_fixture_validate() {
        let mut msc = MultiStateChunk::new(test_chunk());
        msc.insert(StateId::new(1), Chunk::new());

        let ms_fixture =
            MultiStateFixture::new(SchemaVersion::new(1, 0, 0), ChunkPos::new(0, 0, 0), msc);

        assert!(ms_fixture.validate());
        assert_eq!(ms_fixture.checksums.len(), 2);
    }

    // Invariant check detail tests

    #[test]
    fn test_check_block_id_range_pass() {
        let executor = MigrationExecutor::with_defaults();
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let results = executor.check_invariants(&fixture);
        let block_range = results
            .iter()
            .find(|r| r.check.kind == InvariantKind::BlockIdRange)
            .unwrap();

        assert!(block_range.passed);
    }

    #[test]
    fn test_check_block_id_range_fail() {
        let mut executor = MigrationExecutor::with_defaults();
        executor.set_max_block_id(50);

        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), BlockId(100));

        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(0, 0, 0), chunk);

        let results = executor.check_invariants(&fixture);
        let block_range = results
            .iter()
            .find(|r| r.check.kind == InvariantKind::BlockIdRange)
            .unwrap();

        assert!(!block_range.passed);
        assert!(!block_range.affected_positions.is_empty());
    }

    #[test]
    fn test_check_block_count_pass() {
        let executor = MigrationExecutor::with_defaults();
        let fixture = MigrationFixture::new(SchemaVersion::new(1, 0, 0))
            .with_chunk(ChunkPos::new(0, 0, 0), test_chunk());

        let results = executor.check_invariants(&fixture);
        let block_count = results
            .iter()
            .find(|r| r.check.kind == InvariantKind::BlockCount)
            .unwrap();

        assert!(block_count.passed);
    }

    // CompatibilityReport tests

    #[test]
    fn test_compatibility_report_downgrade() {
        let executor = MigrationExecutor::new();
        let report =
            executor.check_compatibility(SchemaVersion::new(2, 0, 0), SchemaVersion::new(1, 0, 0));

        assert!(!report.can_migrate);
        assert!(report.blockers.iter().any(|b| b.contains("Downgrade")));
    }

    #[test]
    fn test_compatibility_report_high_cost_warning() {
        let mut executor = MigrationExecutor::new();

        let mut remap = BTreeMap::new();
        for i in 0..200 {
            remap.insert(i, i + 1000);
        }

        executor.register_step(
            MigrationStep::new(
                SchemaVersion::new(1, 0, 0),
                SchemaVersion::new(2, 0, 0),
                MigrationKind::BlockRemap,
            )
            .with_block_remap(remap),
        );

        let report =
            executor.check_compatibility(SchemaVersion::new(1, 0, 0), SchemaVersion::new(2, 0, 0));

        assert!(report.can_migrate);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("High migration cost"))
        );
    }
}

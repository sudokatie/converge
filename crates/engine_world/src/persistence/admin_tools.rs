//! Replayable admin tools for world repair and moderation.
//!
//! Provides deterministic, data-only admin operations that can be logged,
//! replayed, and audited. All operations are designed for replay safety
//! with bounded validation and dry-run support.
//!
//! # Overview
//!
//! - [`AdminOpId`]: Deterministic operation identifier
//! - [`AdminMetadata`]: Authorization and context for operations
//! - [`AdminOp`]: Admin operation variants (repair, block fill, quarantine, moderation)
//! - [`AdminRecord`]: Logged operation with metadata and outcome
//! - [`AdminLog`]: Append-only deterministic log of operations
//! - [`DryRunResult`]: Planning result before execution
//! - [`AdminQuery`]: Query builder for filtering records

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::BuildHasher;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::chunk::{BlockId, Chunk};
use crate::persistence::{ChunkDelta, RepairPlan};

/// Maximum blocks modifiable in a single block operation.
pub const MAX_BLOCK_REGION_SIZE: usize = 65536;

/// Maximum region bounds in chunks.
pub const MAX_REGION_BOUND_CHUNKS: i32 = 64;

/// Deterministic operation identifier.
///
/// Generated from log index and timestamp hash for uniqueness and replay stability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AdminOpId(u64);

impl AdminOpId {
    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get raw value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Generate from log index and seed.
    #[must_use]
    pub fn generate(log_index: u64, seed: u64) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&log_index.to_le_bytes());
        hasher.update(&seed.to_le_bytes());
        Self(u64::from(hasher.finalize()) | (log_index << 32))
    }
}

/// Authorization level for admin operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum AuthLevel {
    /// Operator-level access.
    Operator = 0,
    /// Moderator-level access.
    Moderator = 1,
    /// Administrator-level access.
    Admin = 2,
    /// System-level automated operation.
    System = 3,
}

impl AuthLevel {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Moderator => "moderator",
            Self::Admin => "admin",
            Self::System => "system",
        }
    }

    /// Check if this level can perform moderation actions.
    #[must_use]
    pub const fn can_moderate(self) -> bool {
        matches!(self, Self::Moderator | Self::Admin | Self::System)
    }

    /// Check if this level can perform repair actions.
    #[must_use]
    pub const fn can_repair(self) -> bool {
        matches!(self, Self::Admin | Self::System)
    }
}

/// Authorization and context metadata for admin operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminMetadata {
    /// Admin identifier (opaque, system-defined).
    pub admin_id: u64,
    /// Authorization level.
    pub auth_level: AuthLevel,
    /// Operation timestamp (simulation tick).
    pub tick: u64,
    /// Human-readable reason for the operation.
    pub reason: String,
    /// Optional ticket/issue reference.
    pub ticket_ref: Option<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
}

impl AdminMetadata {
    /// Create new metadata.
    #[must_use]
    pub fn new(admin_id: u64, auth_level: AuthLevel, tick: u64) -> Self {
        Self {
            admin_id,
            auth_level,
            tick,
            reason: String::new(),
            ticket_ref: None,
            tags: Vec::new(),
        }
    }

    /// Create system-level metadata.
    #[must_use]
    pub fn system(tick: u64) -> Self {
        Self::new(0, AuthLevel::System, tick)
    }

    /// Set reason.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Set ticket reference.
    #[must_use]
    pub fn with_ticket(mut self, ticket: impl Into<String>) -> Self {
        self.ticket_ref = Some(ticket.into());
        self
    }

    /// Add tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if metadata has required authorization for operation.
    #[must_use]
    pub fn is_authorized_for(&self, op: &AdminOp) -> bool {
        match op {
            AdminOp::ApplyRepairPlan { .. } | AdminOp::RegenerateChunks { .. } => {
                self.auth_level.can_repair()
            }
            AdminOp::FillBlocks { .. }
            | AdminOp::ReplaceBlocks { .. }
            | AdminOp::MarkRegion { .. }
            | AdminOp::UnmarkRegion { .. } => self.auth_level >= AuthLevel::Operator,
            AdminOp::QuarantineRegion { .. }
            | AdminOp::UnquarantineRegion { .. }
            | AdminOp::PlayerKick { .. }
            | AdminOp::PlayerBan { .. }
            | AdminOp::PlayerUnban { .. }
            | AdminOp::PlayerTeleport { .. } => self.auth_level.can_moderate(),
        }
    }
}

/// Axis-aligned bounding box for world regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldBounds {
    /// Minimum corner (chunk coordinates).
    pub min_chunk: ChunkPos,
    /// Maximum corner (chunk coordinates, inclusive).
    pub max_chunk: ChunkPos,
}

impl WorldBounds {
    /// Create new bounds from corners.
    #[must_use]
    pub fn new(min: ChunkPos, max: ChunkPos) -> Self {
        Self {
            min_chunk: ChunkPos::new(
                min.x().min(max.x()),
                min.y().min(max.y()),
                min.z().min(max.z()),
            ),
            max_chunk: ChunkPos::new(
                max.x().max(min.x()),
                max.y().max(min.y()),
                max.z().max(min.z()),
            ),
        }
    }

    /// Create bounds for a single chunk.
    #[must_use]
    pub fn single(pos: ChunkPos) -> Self {
        Self {
            min_chunk: pos,
            max_chunk: pos,
        }
    }

    /// Create bounds centered on a chunk with radius.
    #[must_use]
    pub fn centered(center: ChunkPos, radius: i32) -> Self {
        Self::new(
            ChunkPos::new(
                center.x() - radius,
                center.y() - radius,
                center.z() - radius,
            ),
            ChunkPos::new(
                center.x() + radius,
                center.y() + radius,
                center.z() + radius,
            ),
        )
    }

    /// Check if chunk is within bounds.
    #[must_use]
    pub fn contains(&self, pos: ChunkPos) -> bool {
        pos.x() >= self.min_chunk.x()
            && pos.x() <= self.max_chunk.x()
            && pos.y() >= self.min_chunk.y()
            && pos.y() <= self.max_chunk.y()
            && pos.z() >= self.min_chunk.z()
            && pos.z() <= self.max_chunk.z()
    }

    /// Get the number of chunks in the bounds.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        let (dx, dy, dz) = self.extents();
        // extents are always >= 1 due to min/max normalization in constructor
        (dx.cast_unsigned() as usize)
            * (dy.cast_unsigned() as usize)
            * (dz.cast_unsigned() as usize)
    }

    /// Get the extent on each axis.
    #[must_use]
    pub fn extents(&self) -> (i32, i32, i32) {
        (
            self.max_chunk.x() - self.min_chunk.x() + 1,
            self.max_chunk.y() - self.min_chunk.y() + 1,
            self.max_chunk.z() - self.min_chunk.z() + 1,
        )
    }

    /// Check if bounds are within maximum allowed size.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let (dx, dy, dz) = self.extents();
        dx > 0
            && dy > 0
            && dz > 0
            && dx <= MAX_REGION_BOUND_CHUNKS
            && dy <= MAX_REGION_BOUND_CHUNKS
            && dz <= MAX_REGION_BOUND_CHUNKS
    }

    /// Iterate over all chunk positions in bounds.
    pub fn iter(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        let min = self.min_chunk;
        let max = self.max_chunk;
        (min.z()..=max.z()).flat_map(move |z| {
            (min.y()..=max.y())
                .flat_map(move |y| (min.x()..=max.x()).map(move |x| ChunkPos::new(x, y, z)))
        })
    }

    /// Compute deterministic fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.min_chunk.x().to_le_bytes());
        hasher.update(&self.min_chunk.y().to_le_bytes());
        hasher.update(&self.min_chunk.z().to_le_bytes());
        hasher.update(&self.max_chunk.x().to_le_bytes());
        hasher.update(&self.max_chunk.y().to_le_bytes());
        hasher.update(&self.max_chunk.z().to_le_bytes());
        hasher.finalize()
    }

    fn sort_key(&self) -> (i32, i32, i32, i32, i32, i32) {
        (
            self.min_chunk.x(),
            self.min_chunk.y(),
            self.min_chunk.z(),
            self.max_chunk.x(),
            self.max_chunk.y(),
            self.max_chunk.z(),
        )
    }
}

impl PartialOrd for WorldBounds {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WorldBounds {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

/// Block fill specification for a bounded region within a chunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockFillSpec {
    /// Chunk position.
    pub chunk_pos: ChunkPos,
    /// Minimum local position (inclusive).
    pub min_local: LocalPos,
    /// Maximum local position (inclusive).
    pub max_local: LocalPos,
    /// Block to fill with.
    pub block: BlockId,
}

impl BlockFillSpec {
    /// Create a new fill specification.
    #[must_use]
    pub fn new(
        chunk_pos: ChunkPos,
        min_local: LocalPos,
        max_local: LocalPos,
        block: BlockId,
    ) -> Self {
        Self {
            chunk_pos,
            min_local: LocalPos::new(
                min_local.x().min(max_local.x()),
                min_local.y().min(max_local.y()),
                min_local.z().min(max_local.z()),
            ),
            max_local: LocalPos::new(
                max_local.x().max(min_local.x()),
                max_local.y().max(min_local.y()),
                max_local.z().max(min_local.z()),
            ),
            block,
        }
    }

    /// Create spec to fill entire chunk.
    #[must_use]
    pub fn entire_chunk(chunk_pos: ChunkPos, block: BlockId) -> Self {
        Self {
            chunk_pos,
            min_local: LocalPos::new(0, 0, 0),
            max_local: LocalPos::new(15, 15, 15),
            block,
        }
    }

    /// Count of blocks affected.
    #[must_use]
    pub fn block_count(&self) -> usize {
        let dx = (self.max_local.x() - self.min_local.x() + 1) as usize;
        let dy = (self.max_local.y() - self.min_local.y() + 1) as usize;
        let dz = (self.max_local.z() - self.min_local.z() + 1) as usize;
        dx * dy * dz
    }

    /// Check if specification is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.block_count() > 0 && self.block_count() <= MAX_BLOCK_REGION_SIZE
    }

    /// Iterate over all local positions in the fill region.
    pub fn iter_positions(&self) -> impl Iterator<Item = LocalPos> + '_ {
        let min = self.min_local;
        let max = self.max_local;
        (min.z()..=max.z()).flat_map(move |z| {
            (min.y()..=max.y())
                .flat_map(move |y| (min.x()..=max.x()).map(move |x| LocalPos::new(x, y, z)))
        })
    }

    /// Convert to chunk delta.
    #[must_use]
    pub fn to_delta(&self) -> ChunkDelta {
        let mut delta = ChunkDelta::new();
        for pos in self.iter_positions() {
            delta.set(pos, self.block);
        }
        delta
    }
}

/// Block replace specification for targeted block replacement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockReplaceSpec {
    /// Chunk position.
    pub chunk_pos: ChunkPos,
    /// Minimum local position (inclusive).
    pub min_local: LocalPos,
    /// Maximum local position (inclusive).
    pub max_local: LocalPos,
    /// Block to replace.
    pub from_block: BlockId,
    /// Block to replace with.
    pub to_block: BlockId,
}

impl BlockReplaceSpec {
    /// Create a new replace specification.
    #[must_use]
    pub fn new(
        chunk_pos: ChunkPos,
        min_local: LocalPos,
        max_local: LocalPos,
        from_block: BlockId,
        to_block: BlockId,
    ) -> Self {
        Self {
            chunk_pos,
            min_local: LocalPos::new(
                min_local.x().min(max_local.x()),
                min_local.y().min(max_local.y()),
                min_local.z().min(max_local.z()),
            ),
            max_local: LocalPos::new(
                max_local.x().max(min_local.x()),
                max_local.y().max(min_local.y()),
                max_local.z().max(min_local.z()),
            ),
            from_block,
            to_block,
        }
    }

    /// Count blocks that would be replaced in a chunk.
    #[must_use]
    pub fn count_matches(&self, chunk: &Chunk) -> usize {
        self.iter_positions()
            .filter(|&pos| chunk.get(pos) == self.from_block)
            .count()
    }

    /// Check if specification is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let dx = (self.max_local.x() - self.min_local.x() + 1) as usize;
        let dy = (self.max_local.y() - self.min_local.y() + 1) as usize;
        let dz = (self.max_local.z() - self.min_local.z() + 1) as usize;
        let count = dx * dy * dz;
        count > 0 && count <= MAX_BLOCK_REGION_SIZE
    }

    /// Iterate over all local positions in the region.
    pub fn iter_positions(&self) -> impl Iterator<Item = LocalPos> + '_ {
        let min = self.min_local;
        let max = self.max_local;
        (min.z()..=max.z()).flat_map(move |z| {
            (min.y()..=max.y())
                .flat_map(move |y| (min.x()..=max.x()).map(move |x| LocalPos::new(x, y, z)))
        })
    }

    /// Apply to chunk and return delta of changes.
    #[must_use]
    pub fn apply_to_delta(&self, chunk: &Chunk) -> ChunkDelta {
        let mut delta = ChunkDelta::new();
        for pos in self.iter_positions() {
            if chunk.get(pos) == self.from_block {
                delta.set(pos, self.to_block);
            }
        }
        delta
    }
}

/// Region quarantine status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineStatus {
    /// Quarantine timestamp (tick when quarantined).
    pub quarantined_at: u64,
    /// Admin who quarantined.
    pub admin_id: u64,
    /// Reason for quarantine.
    pub reason: String,
    /// Severity level.
    pub severity: QuarantineSeverity,
}

/// Severity level for quarantine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum QuarantineSeverity {
    /// Low severity, monitoring.
    Low = 0,
    /// Medium severity, restricted access.
    Medium = 1,
    /// High severity, no access.
    High = 2,
    /// Critical, pending deletion.
    Critical = 3,
}

impl QuarantineSeverity {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Region marker annotation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionMarker {
    /// Marker label.
    pub label: String,
    /// Marker category.
    pub category: MarkerCategory,
    /// Timestamp when marked.
    pub marked_at: u64,
    /// Admin who marked.
    pub admin_id: u64,
    /// Optional notes.
    pub notes: Option<String>,
}

/// Marker category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MarkerCategory {
    /// Informational marker.
    Info = 0,
    /// Warning marker.
    Warning = 1,
    /// Protected region.
    Protected = 2,
    /// Investigation pending.
    Investigation = 3,
    /// Historical significance.
    Historical = 4,
}

impl MarkerCategory {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Protected => "protected",
            Self::Investigation => "investigation",
            Self::Historical => "historical",
        }
    }
}

/// Player moderation action type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ModerationAction {
    /// Player kicked (temporary disconnect).
    Kick = 0,
    /// Player banned (permanent or timed).
    Ban = 1,
    /// Player unbanned.
    Unban = 2,
    /// Player teleported.
    Teleport = 3,
}

impl ModerationAction {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Kick => "kick",
            Self::Ban => "ban",
            Self::Unban => "unban",
            Self::Teleport => "teleport",
        }
    }
}

/// Player moderation record (data-only, no networking side effects).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerModerationRecord {
    /// Target player ID.
    pub player_id: u64,
    /// Action taken.
    pub action: ModerationAction,
    /// Timestamp (tick).
    pub tick: u64,
    /// Admin who performed action.
    pub admin_id: u64,
    /// Reason for action.
    pub reason: String,
    /// Duration in ticks (for bans, None = permanent).
    pub duration_ticks: Option<u64>,
    /// Expiration tick (computed from tick + duration).
    pub expires_at: Option<u64>,
    /// Additional context.
    pub context: Option<String>,
}

impl PlayerModerationRecord {
    /// Create a kick record.
    #[must_use]
    pub fn kick(player_id: u64, admin_id: u64, tick: u64, reason: impl Into<String>) -> Self {
        Self {
            player_id,
            action: ModerationAction::Kick,
            tick,
            admin_id,
            reason: reason.into(),
            duration_ticks: None,
            expires_at: None,
            context: None,
        }
    }

    /// Create a ban record.
    #[must_use]
    pub fn ban(
        player_id: u64,
        admin_id: u64,
        tick: u64,
        reason: impl Into<String>,
        duration_ticks: Option<u64>,
    ) -> Self {
        Self {
            player_id,
            action: ModerationAction::Ban,
            tick,
            admin_id,
            reason: reason.into(),
            duration_ticks,
            expires_at: duration_ticks.map(|d| tick + d),
            context: None,
        }
    }

    /// Create an unban record.
    #[must_use]
    pub fn unban(player_id: u64, admin_id: u64, tick: u64, reason: impl Into<String>) -> Self {
        Self {
            player_id,
            action: ModerationAction::Unban,
            tick,
            admin_id,
            reason: reason.into(),
            duration_ticks: None,
            expires_at: None,
            context: None,
        }
    }

    /// Create a teleport record.
    #[must_use]
    pub fn teleport(player_id: u64, admin_id: u64, tick: u64, reason: impl Into<String>) -> Self {
        Self {
            player_id,
            action: ModerationAction::Teleport,
            tick,
            admin_id,
            reason: reason.into(),
            duration_ticks: None,
            expires_at: None,
            context: None,
        }
    }

    /// Add context.
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Check if ban is still active at given tick.
    #[must_use]
    pub fn is_active_at(&self, tick: u64) -> bool {
        match self.action {
            ModerationAction::Ban => self.expires_at.is_none_or(|exp| tick < exp),
            _ => false,
        }
    }

    /// Check if this record is expired at given tick.
    #[must_use]
    pub fn is_expired_at(&self, tick: u64) -> bool {
        self.expires_at.is_some_and(|exp| tick >= exp)
    }
}

/// Teleport destination specification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeleportDestination {
    /// Target chunk.
    pub chunk_pos: ChunkPos,
    /// Local position within chunk.
    pub local_pos: LocalPos,
}

impl TeleportDestination {
    /// Create new destination.
    #[must_use]
    pub fn new(chunk_pos: ChunkPos, local_pos: LocalPos) -> Self {
        Self {
            chunk_pos,
            local_pos,
        }
    }
}

/// Admin operation variants.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminOp {
    /// Apply an existing repair plan.
    ApplyRepairPlan {
        /// The repair plan to apply.
        plan: RepairPlan,
    },

    /// Regenerate chunks from seed.
    RegenerateChunks {
        /// Chunks to regenerate.
        chunks: Vec<ChunkPos>,
    },

    /// Fill blocks in a bounded region.
    FillBlocks {
        /// Fill specifications.
        specs: Vec<BlockFillSpec>,
    },

    /// Replace blocks in a bounded region.
    ReplaceBlocks {
        /// Replace specifications.
        specs: Vec<BlockReplaceSpec>,
    },

    /// Quarantine a region.
    QuarantineRegion {
        /// Region bounds.
        bounds: WorldBounds,
        /// Quarantine status.
        status: QuarantineStatus,
    },

    /// Remove quarantine from a region.
    UnquarantineRegion {
        /// Region bounds.
        bounds: WorldBounds,
    },

    /// Mark a region with annotation.
    MarkRegion {
        /// Region bounds.
        bounds: WorldBounds,
        /// Marker to apply.
        marker: RegionMarker,
    },

    /// Remove marker from a region.
    UnmarkRegion {
        /// Region bounds.
        bounds: WorldBounds,
        /// Marker label to remove.
        label: String,
    },

    /// Record a player kick (data-only).
    PlayerKick {
        /// Moderation record.
        record: PlayerModerationRecord,
    },

    /// Record a player ban (data-only).
    PlayerBan {
        /// Moderation record.
        record: PlayerModerationRecord,
    },

    /// Record a player unban (data-only).
    PlayerUnban {
        /// Moderation record.
        record: PlayerModerationRecord,
    },

    /// Record a player teleport (data-only).
    PlayerTeleport {
        /// Moderation record.
        record: PlayerModerationRecord,
        /// Teleport destination.
        destination: TeleportDestination,
    },
}

impl AdminOp {
    /// Get operation category name.
    #[must_use]
    pub fn category(&self) -> OpCategory {
        match self {
            Self::ApplyRepairPlan { .. } | Self::RegenerateChunks { .. } => OpCategory::Repair,
            Self::FillBlocks { .. } | Self::ReplaceBlocks { .. } => OpCategory::BlockEdit,
            Self::QuarantineRegion { .. } | Self::UnquarantineRegion { .. } => {
                OpCategory::Quarantine
            }
            Self::MarkRegion { .. } | Self::UnmarkRegion { .. } => OpCategory::Marker,
            Self::PlayerKick { .. }
            | Self::PlayerBan { .. }
            | Self::PlayerUnban { .. }
            | Self::PlayerTeleport { .. } => OpCategory::Moderation,
        }
    }

    /// Estimate the number of blocks affected.
    #[must_use]
    pub fn estimated_block_cost(&self) -> usize {
        match self {
            Self::ApplyRepairPlan { plan } => plan.total_modifications(),
            Self::RegenerateChunks { chunks } => chunks.len() * 4096,
            Self::FillBlocks { specs } => specs.iter().map(BlockFillSpec::block_count).sum(),
            Self::ReplaceBlocks { specs } => specs
                .iter()
                .map(|s| {
                    let dx = (s.max_local.x() - s.min_local.x() + 1) as usize;
                    let dy = (s.max_local.y() - s.min_local.y() + 1) as usize;
                    let dz = (s.max_local.z() - s.min_local.z() + 1) as usize;
                    dx * dy * dz
                })
                .sum(),
            Self::QuarantineRegion { .. }
            | Self::UnquarantineRegion { .. }
            | Self::MarkRegion { .. }
            | Self::UnmarkRegion { .. }
            | Self::PlayerKick { .. }
            | Self::PlayerBan { .. }
            | Self::PlayerUnban { .. }
            | Self::PlayerTeleport { .. } => 0,
        }
    }

    /// Validate operation bounds.
    #[must_use]
    pub fn validate(&self) -> ValidationResult {
        match self {
            Self::ApplyRepairPlan { plan } => {
                if !plan.within_bounds {
                    return ValidationResult::invalid("repair plan exceeds bounds");
                }
                ValidationResult::valid()
            }
            Self::RegenerateChunks { chunks } => {
                if chunks.len() > MAX_REGION_BOUND_CHUNKS as usize {
                    return ValidationResult::invalid("too many chunks to regenerate");
                }
                ValidationResult::valid()
            }
            Self::FillBlocks { specs } => {
                let total: usize = specs.iter().map(BlockFillSpec::block_count).sum();
                if total > MAX_BLOCK_REGION_SIZE {
                    return ValidationResult::invalid("total fill size exceeds limit");
                }
                for spec in specs {
                    if !spec.is_valid() {
                        return ValidationResult::invalid("invalid fill specification");
                    }
                }
                ValidationResult::valid()
            }
            Self::ReplaceBlocks { specs } => {
                for spec in specs {
                    if !spec.is_valid() {
                        return ValidationResult::invalid("invalid replace specification");
                    }
                }
                ValidationResult::valid()
            }
            Self::QuarantineRegion { bounds, .. }
            | Self::UnquarantineRegion { bounds }
            | Self::MarkRegion { bounds, .. }
            | Self::UnmarkRegion { bounds, .. } => {
                if !bounds.is_valid() {
                    return ValidationResult::invalid("region bounds exceed limit");
                }
                ValidationResult::valid()
            }
            Self::PlayerKick { .. }
            | Self::PlayerBan { .. }
            | Self::PlayerUnban { .. }
            | Self::PlayerTeleport { .. } => ValidationResult::valid(),
        }
    }

    /// Compute deterministic fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&[self.category() as u8]);

        match self {
            Self::ApplyRepairPlan { plan } => {
                hasher.update(&(plan.operations.len() as u64).to_le_bytes());
                hasher.update(&(plan.max_modifications as u64).to_le_bytes());
            }
            Self::RegenerateChunks { chunks } => {
                hasher.update(&(chunks.len() as u64).to_le_bytes());
                for chunk in chunks {
                    hasher.update(&chunk.x().to_le_bytes());
                    hasher.update(&chunk.y().to_le_bytes());
                    hasher.update(&chunk.z().to_le_bytes());
                }
            }
            Self::FillBlocks { specs } => {
                hasher.update(&(specs.len() as u64).to_le_bytes());
                for spec in specs {
                    hasher.update(&spec.chunk_pos.x().to_le_bytes());
                    hasher.update(&spec.block.0.to_le_bytes());
                }
            }
            Self::ReplaceBlocks { specs } => {
                hasher.update(&(specs.len() as u64).to_le_bytes());
                for spec in specs {
                    hasher.update(&spec.chunk_pos.x().to_le_bytes());
                    hasher.update(&spec.from_block.0.to_le_bytes());
                    hasher.update(&spec.to_block.0.to_le_bytes());
                }
            }
            Self::QuarantineRegion { bounds, status } => {
                hasher.update(&bounds.fingerprint().to_le_bytes());
                hasher.update(&[status.severity as u8]);
            }
            Self::UnquarantineRegion { bounds } => {
                hasher.update(&bounds.fingerprint().to_le_bytes());
            }
            Self::MarkRegion { bounds, marker } => {
                hasher.update(&bounds.fingerprint().to_le_bytes());
                hasher.update(&[marker.category as u8]);
                hasher.update(marker.label.as_bytes());
            }
            Self::UnmarkRegion { bounds, label } => {
                hasher.update(&bounds.fingerprint().to_le_bytes());
                hasher.update(label.as_bytes());
            }
            Self::PlayerKick { record }
            | Self::PlayerBan { record }
            | Self::PlayerUnban { record } => {
                hasher.update(&record.player_id.to_le_bytes());
                hasher.update(&[record.action as u8]);
            }
            Self::PlayerTeleport {
                record,
                destination,
            } => {
                hasher.update(&record.player_id.to_le_bytes());
                hasher.update(&destination.chunk_pos.x().to_le_bytes());
                hasher.update(&destination.chunk_pos.y().to_le_bytes());
                hasher.update(&destination.chunk_pos.z().to_le_bytes());
            }
        }

        hasher.finalize()
    }
}

/// Operation category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OpCategory {
    /// Repair operations.
    Repair = 0,
    /// Block editing operations.
    BlockEdit = 1,
    /// Quarantine operations.
    Quarantine = 2,
    /// Marker operations.
    Marker = 3,
    /// Player moderation operations.
    Moderation = 4,
}

impl OpCategory {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::BlockEdit => "block_edit",
            Self::Quarantine => "quarantine",
            Self::Marker => "marker",
            Self::Moderation => "moderation",
        }
    }
}

/// Validation result for operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// Error message if invalid.
    pub error: Option<String>,
}

impl ValidationResult {
    /// Create a valid result.
    #[must_use]
    pub fn valid() -> Self {
        Self {
            valid: true,
            error: None,
        }
    }

    /// Create an invalid result.
    #[must_use]
    pub fn invalid(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            error: Some(error.into()),
        }
    }
}

/// Outcome status of an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OpOutcome {
    /// Operation succeeded.
    Success = 0,
    /// Operation partially succeeded.
    Partial = 1,
    /// Operation failed.
    Failed = 2,
    /// Operation was a dry-run (not applied).
    DryRun = 3,
    /// Operation was skipped.
    Skipped = 4,
}

impl OpOutcome {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::DryRun => "dry_run",
            Self::Skipped => "skipped",
        }
    }

    /// Check if outcome indicates success.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::DryRun)
    }
}

/// Logged admin operation record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminRecord {
    /// Operation identifier.
    pub id: AdminOpId,
    /// Log index (0-based).
    pub log_index: u64,
    /// Operation metadata.
    pub metadata: AdminMetadata,
    /// The operation.
    pub operation: AdminOp,
    /// Outcome of the operation.
    pub outcome: OpOutcome,
    /// Blocks actually modified.
    pub blocks_modified: usize,
    /// Chunks affected.
    pub chunks_affected: usize,
    /// Error message if failed.
    pub error: Option<String>,
    /// Operation fingerprint.
    pub fingerprint: u32,
}

impl AdminRecord {
    /// Create a new record.
    #[must_use]
    fn new(
        id: AdminOpId,
        log_index: u64,
        metadata: AdminMetadata,
        operation: AdminOp,
        outcome: OpOutcome,
    ) -> Self {
        let fingerprint = operation.fingerprint();
        Self {
            id,
            log_index,
            metadata,
            operation,
            outcome,
            blocks_modified: 0,
            chunks_affected: 0,
            error: None,
            fingerprint,
        }
    }

    /// Set blocks modified.
    #[must_use]
    pub fn with_blocks_modified(mut self, count: usize) -> Self {
        self.blocks_modified = count;
        self
    }

    /// Set chunks affected.
    #[must_use]
    pub fn with_chunks_affected(mut self, count: usize) -> Self {
        self.chunks_affected = count;
        self
    }

    /// Set error.
    #[must_use]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Check if operation was successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Ordering key for deterministic sorting.
    fn sort_key(&self) -> (u64, u64) {
        (self.metadata.tick, self.log_index)
    }
}

impl PartialEq for AdminRecord {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AdminRecord {}

impl PartialOrd for AdminRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AdminRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

/// Result of a dry-run planning phase.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DryRunResult {
    /// Whether the operation is valid.
    pub valid: bool,
    /// Validation errors.
    pub validation_errors: Vec<String>,
    /// Estimated blocks that would be modified.
    pub estimated_blocks: usize,
    /// Estimated chunks that would be affected.
    pub estimated_chunks: usize,
    /// Chunk positions that would be affected.
    pub affected_chunks: Vec<ChunkPos>,
    /// Warnings (non-fatal issues).
    pub warnings: Vec<String>,
    /// Operation fingerprint for verification.
    pub fingerprint: u32,
}

impl DryRunResult {
    /// Create a successful dry-run result.
    #[must_use]
    pub fn success(
        estimated_blocks: usize,
        affected_chunks: Vec<ChunkPos>,
        fingerprint: u32,
    ) -> Self {
        Self {
            valid: true,
            validation_errors: Vec::new(),
            estimated_blocks,
            estimated_chunks: affected_chunks.len(),
            affected_chunks,
            warnings: Vec::new(),
            fingerprint,
        }
    }

    /// Create a failed dry-run result.
    #[must_use]
    pub fn failed(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            validation_errors: errors,
            estimated_blocks: 0,
            estimated_chunks: 0,
            affected_chunks: Vec::new(),
            warnings: Vec::new(),
            fingerprint: 0,
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Query builder for filtering admin records.
#[derive(Clone, Debug, Default)]
pub struct AdminQuery {
    tick_min: Option<u64>,
    tick_max: Option<u64>,
    admin_id: Option<u64>,
    auth_level_min: Option<AuthLevel>,
    category: Option<OpCategory>,
    outcome: Option<OpOutcome>,
    player_id: Option<u64>,
    chunk_pos: Option<ChunkPos>,
    tag: Option<String>,
    limit: Option<usize>,
}

impl AdminQuery {
    /// Create a new empty query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by minimum tick.
    #[must_use]
    pub fn tick_min(mut self, tick: u64) -> Self {
        self.tick_min = Some(tick);
        self
    }

    /// Filter by maximum tick.
    #[must_use]
    pub fn tick_max(mut self, tick: u64) -> Self {
        self.tick_max = Some(tick);
        self
    }

    /// Filter by tick range.
    #[must_use]
    pub fn tick_range(mut self, min: u64, max: u64) -> Self {
        self.tick_min = Some(min);
        self.tick_max = Some(max);
        self
    }

    /// Filter by admin ID.
    #[must_use]
    pub fn admin_id(mut self, id: u64) -> Self {
        self.admin_id = Some(id);
        self
    }

    /// Filter by minimum auth level.
    #[must_use]
    pub fn auth_level_min(mut self, level: AuthLevel) -> Self {
        self.auth_level_min = Some(level);
        self
    }

    /// Filter by operation category.
    #[must_use]
    pub fn category(mut self, category: OpCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Filter by outcome.
    #[must_use]
    pub fn outcome(mut self, outcome: OpOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Filter by affected player ID.
    #[must_use]
    pub fn player_id(mut self, id: u64) -> Self {
        self.player_id = Some(id);
        self
    }

    /// Filter by affected chunk.
    #[must_use]
    pub fn chunk_pos(mut self, pos: ChunkPos) -> Self {
        self.chunk_pos = Some(pos);
        self
    }

    /// Filter by tag.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Limit results.
    #[must_use]
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Check if a record matches this query.
    #[must_use]
    pub fn matches(&self, record: &AdminRecord) -> bool {
        if self.tick_min.is_some_and(|min| record.metadata.tick < min) {
            return false;
        }
        if self.tick_max.is_some_and(|max| record.metadata.tick > max) {
            return false;
        }
        if self
            .admin_id
            .is_some_and(|id| record.metadata.admin_id != id)
        {
            return false;
        }
        if self
            .auth_level_min
            .is_some_and(|level| record.metadata.auth_level < level)
        {
            return false;
        }
        if self
            .category
            .is_some_and(|cat| record.operation.category() != cat)
        {
            return false;
        }
        if self.outcome.is_some_and(|out| record.outcome != out) {
            return false;
        }
        if self
            .tag
            .as_ref()
            .is_some_and(|t| !record.metadata.tags.contains(t))
        {
            return false;
        }
        if let Some(player_id) = self.player_id {
            let matches_player = match &record.operation {
                AdminOp::PlayerKick { record: r }
                | AdminOp::PlayerBan { record: r }
                | AdminOp::PlayerUnban { record: r }
                | AdminOp::PlayerTeleport { record: r, .. } => r.player_id == player_id,
                _ => false,
            };
            if !matches_player {
                return false;
            }
        }
        if let Some(chunk) = self.chunk_pos {
            let matches_chunk = match &record.operation {
                AdminOp::FillBlocks { specs } => specs.iter().any(|s| s.chunk_pos == chunk),
                AdminOp::ReplaceBlocks { specs } => specs.iter().any(|s| s.chunk_pos == chunk),
                AdminOp::RegenerateChunks { chunks } => chunks.contains(&chunk),
                AdminOp::QuarantineRegion { bounds, .. }
                | AdminOp::UnquarantineRegion { bounds }
                | AdminOp::MarkRegion { bounds, .. }
                | AdminOp::UnmarkRegion { bounds, .. } => bounds.contains(chunk),
                AdminOp::ApplyRepairPlan { plan } => {
                    plan.operations.iter().any(|op| op.chunk_pos() == chunk)
                }
                _ => false,
            };
            if !matches_chunk {
                return false;
            }
        }
        true
    }
}

/// Summary statistics for admin log.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminLogStats {
    /// Total number of records.
    pub record_count: usize,
    /// Count by category.
    pub by_category: BTreeMap<OpCategory, usize>,
    /// Count by outcome.
    pub by_outcome: BTreeMap<OpOutcome, usize>,
    /// Total blocks modified.
    pub total_blocks_modified: usize,
    /// Total chunks affected.
    pub total_chunks_affected: usize,
    /// Earliest tick.
    pub min_tick: Option<u64>,
    /// Latest tick.
    pub max_tick: Option<u64>,
    /// Unique admins.
    pub unique_admins: usize,
}

/// Append-only deterministic admin log.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AdminLog {
    records: Vec<AdminRecord>,
    next_index: u64,
    seed: u64,
    quarantines: BTreeMap<WorldBounds, QuarantineStatus>,
    markers: BTreeMap<WorldBounds, Vec<RegionMarker>>,
    player_bans: BTreeMap<u64, PlayerModerationRecord>,
}

impl AdminLog {
    /// Create a new empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a log with a specific seed.
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Get the number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get all records.
    #[must_use]
    pub fn records(&self) -> &[AdminRecord] {
        &self.records
    }

    /// Plan a dry-run for an operation.
    #[must_use]
    pub fn dry_run(&self, metadata: &AdminMetadata, operation: &AdminOp) -> DryRunResult {
        let validation = operation.validate();
        if !validation.valid {
            return DryRunResult::failed(vec![validation.error.unwrap_or_default()]);
        }

        if !metadata.is_authorized_for(operation) {
            return DryRunResult::failed(vec!["insufficient authorization".to_string()]);
        }

        let estimated_blocks = operation.estimated_block_cost();
        let affected_chunks = Self::collect_affected_chunks(operation);
        let fingerprint = operation.fingerprint();

        let mut result = DryRunResult::success(estimated_blocks, affected_chunks, fingerprint);

        if estimated_blocks > MAX_BLOCK_REGION_SIZE {
            result.add_warning(format!(
                "large operation: {estimated_blocks} blocks exceed recommended limit"
            ));
        }

        result
    }

    /// Collect chunks affected by an operation.
    fn collect_affected_chunks(operation: &AdminOp) -> Vec<ChunkPos> {
        let mut chunks = HashSet::new();

        match operation {
            AdminOp::ApplyRepairPlan { plan } => {
                for op in &plan.operations {
                    chunks.insert(op.chunk_pos());
                }
            }
            AdminOp::RegenerateChunks { chunks: c } => {
                chunks.extend(c.iter().copied());
            }
            AdminOp::FillBlocks { specs } => {
                chunks.extend(specs.iter().map(|s| s.chunk_pos));
            }
            AdminOp::ReplaceBlocks { specs } => {
                chunks.extend(specs.iter().map(|s| s.chunk_pos));
            }
            AdminOp::QuarantineRegion { bounds, .. }
            | AdminOp::UnquarantineRegion { bounds }
            | AdminOp::MarkRegion { bounds, .. }
            | AdminOp::UnmarkRegion { bounds, .. } => {
                chunks.extend(bounds.iter());
            }
            AdminOp::PlayerKick { .. }
            | AdminOp::PlayerBan { .. }
            | AdminOp::PlayerUnban { .. }
            | AdminOp::PlayerTeleport { .. } => {}
        }

        chunks.into_iter().collect()
    }

    /// Append an operation and return its record.
    pub fn append(
        &mut self,
        metadata: AdminMetadata,
        operation: AdminOp,
        outcome: OpOutcome,
    ) -> &AdminRecord {
        let id = AdminOpId::generate(self.next_index, self.seed);
        let record = AdminRecord::new(id, self.next_index, metadata, operation, outcome);
        self.next_index += 1;

        self.update_state_tracking(&record);
        let idx = self.records.len();
        self.records.push(record);
        &self.records[idx]
    }

    /// Append a successful operation with modification counts.
    pub fn append_success(
        &mut self,
        metadata: AdminMetadata,
        operation: AdminOp,
        blocks_modified: usize,
        chunks_affected: usize,
    ) -> &AdminRecord {
        let id = AdminOpId::generate(self.next_index, self.seed);
        let record = AdminRecord::new(id, self.next_index, metadata, operation, OpOutcome::Success)
            .with_blocks_modified(blocks_modified)
            .with_chunks_affected(chunks_affected);
        self.next_index += 1;

        self.update_state_tracking(&record);
        let idx = self.records.len();
        self.records.push(record);
        &self.records[idx]
    }

    /// Append a failed operation with error.
    pub fn append_failed(
        &mut self,
        metadata: AdminMetadata,
        operation: AdminOp,
        error: impl Into<String>,
    ) -> &AdminRecord {
        let id = AdminOpId::generate(self.next_index, self.seed);
        let record = AdminRecord::new(id, self.next_index, metadata, operation, OpOutcome::Failed)
            .with_error(error);
        self.next_index += 1;
        let idx = self.records.len();
        self.records.push(record);
        &self.records[idx]
    }

    /// Append a dry-run record.
    pub fn append_dry_run(&mut self, metadata: AdminMetadata, operation: AdminOp) -> &AdminRecord {
        let id = AdminOpId::generate(self.next_index, self.seed);
        let estimated_blocks = operation.estimated_block_cost();
        let estimated_chunks = Self::collect_affected_chunks(&operation).len();
        let record = AdminRecord::new(id, self.next_index, metadata, operation, OpOutcome::DryRun)
            .with_blocks_modified(estimated_blocks)
            .with_chunks_affected(estimated_chunks);
        self.next_index += 1;
        let idx = self.records.len();
        self.records.push(record);
        &self.records[idx]
    }

    /// Update internal state tracking based on operation.
    fn update_state_tracking(&mut self, record: &AdminRecord) {
        if record.outcome != OpOutcome::Success {
            return;
        }

        match &record.operation {
            AdminOp::QuarantineRegion { bounds, status } => {
                self.quarantines.insert(*bounds, status.clone());
            }
            AdminOp::UnquarantineRegion { bounds } => {
                self.quarantines.remove(bounds);
            }
            AdminOp::MarkRegion { bounds, marker } => {
                self.markers
                    .entry(*bounds)
                    .or_default()
                    .push(marker.clone());
            }
            AdminOp::UnmarkRegion { bounds, label } => {
                if let Some(markers) = self.markers.get_mut(bounds) {
                    markers.retain(|m| m.label != *label);
                    if markers.is_empty() {
                        self.markers.remove(bounds);
                    }
                }
            }
            AdminOp::PlayerBan { record: mod_record } => {
                self.player_bans
                    .insert(mod_record.player_id, mod_record.clone());
            }
            AdminOp::PlayerUnban { record: mod_record } => {
                self.player_bans.remove(&mod_record.player_id);
            }
            _ => {}
        }
    }

    /// Query records matching criteria.
    pub fn query(&self, q: &AdminQuery) -> Vec<&AdminRecord> {
        let iter = self.records.iter().filter(|r| q.matches(r));
        if let Some(limit) = q.limit {
            iter.take(limit).collect()
        } else {
            iter.collect()
        }
    }

    /// Get records in a tick range.
    pub fn records_in_range(&self, min_tick: u64, max_tick: u64) -> Vec<&AdminRecord> {
        self.records
            .iter()
            .filter(|r| r.metadata.tick >= min_tick && r.metadata.tick <= max_tick)
            .collect()
    }

    /// Get records by category.
    pub fn records_by_category(&self, category: OpCategory) -> Vec<&AdminRecord> {
        self.records
            .iter()
            .filter(|r| r.operation.category() == category)
            .collect()
    }

    /// Get records by admin ID.
    pub fn records_by_admin(&self, admin_id: u64) -> Vec<&AdminRecord> {
        self.records
            .iter()
            .filter(|r| r.metadata.admin_id == admin_id)
            .collect()
    }

    /// Check if a region is quarantined.
    #[must_use]
    pub fn is_quarantined(&self, bounds: &WorldBounds) -> bool {
        self.quarantines.contains_key(bounds)
    }

    /// Get quarantine status for a region.
    #[must_use]
    pub fn quarantine_status(&self, bounds: &WorldBounds) -> Option<&QuarantineStatus> {
        self.quarantines.get(bounds)
    }

    /// Get all active quarantines.
    #[must_use]
    pub fn active_quarantines(&self) -> &BTreeMap<WorldBounds, QuarantineStatus> {
        &self.quarantines
    }

    /// Get markers for a region.
    #[must_use]
    pub fn region_markers(&self, bounds: &WorldBounds) -> Option<&Vec<RegionMarker>> {
        self.markers.get(bounds)
    }

    /// Get all region markers.
    #[must_use]
    pub fn all_markers(&self) -> &BTreeMap<WorldBounds, Vec<RegionMarker>> {
        &self.markers
    }

    /// Check if a player is banned.
    #[must_use]
    pub fn is_banned(&self, player_id: u64, current_tick: u64) -> bool {
        self.player_bans
            .get(&player_id)
            .is_some_and(|r| r.is_active_at(current_tick))
    }

    /// Get ban record for a player.
    #[must_use]
    pub fn ban_record(&self, player_id: u64) -> Option<&PlayerModerationRecord> {
        self.player_bans.get(&player_id)
    }

    /// Get all active bans at a tick.
    pub fn active_bans(&self, current_tick: u64) -> Vec<&PlayerModerationRecord> {
        self.player_bans
            .values()
            .filter(|r| r.is_active_at(current_tick))
            .collect()
    }

    /// Compute deterministic checksum of the log.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.seed.to_le_bytes());
        hasher.update(&self.next_index.to_le_bytes());

        for record in &self.records {
            hasher.update(&record.id.raw().to_le_bytes());
            hasher.update(&record.fingerprint.to_le_bytes());
            hasher.update(&[record.outcome as u8]);
        }

        hasher.finalize()
    }

    /// Compute checksum for a tick range.
    #[must_use]
    pub fn checksum_range(&self, min_tick: u64, max_tick: u64) -> u32 {
        let mut hasher = crc32fast::Hasher::new();

        for record in &self.records {
            if record.metadata.tick >= min_tick && record.metadata.tick <= max_tick {
                hasher.update(&record.id.raw().to_le_bytes());
                hasher.update(&record.fingerprint.to_le_bytes());
                hasher.update(&[record.outcome as u8]);
            }
        }

        hasher.finalize()
    }

    /// Compute statistics.
    #[must_use]
    pub fn stats(&self) -> AdminLogStats {
        let mut stats = AdminLogStats {
            record_count: self.records.len(),
            ..Default::default()
        };

        let mut admins = BTreeSet::new();

        for record in &self.records {
            admins.insert(record.metadata.admin_id);

            *stats
                .by_category
                .entry(record.operation.category())
                .or_insert(0) += 1;
            *stats.by_outcome.entry(record.outcome).or_insert(0) += 1;

            stats.total_blocks_modified += record.blocks_modified;
            stats.total_chunks_affected += record.chunks_affected;

            let tick = record.metadata.tick;
            stats.min_tick = Some(stats.min_tick.map_or(tick, |m| m.min(tick)));
            stats.max_tick = Some(stats.max_tick.map_or(tick, |m| m.max(tick)));
        }

        stats.unique_admins = admins.len();
        stats
    }

    /// Generate summary text.
    #[must_use]
    pub fn summarize(&self) -> String {
        let stats = self.stats();
        format!(
            "AdminLog: {} records, {} blocks modified, {} admins, ticks {:?}-{:?}",
            stats.record_count,
            stats.total_blocks_modified,
            stats.unique_admins,
            stats.min_tick,
            stats.max_tick
        )
    }

    /// Replay block operations onto chunks (in-memory).
    pub fn replay_block_ops<S: BuildHasher>(
        &self,
        chunks: &mut HashMap<ChunkPos, Chunk, S>,
        min_tick: u64,
        max_tick: u64,
    ) -> ReplayResult {
        let mut result = ReplayResult::default();

        for record in &self.records {
            if record.metadata.tick < min_tick || record.metadata.tick > max_tick {
                continue;
            }
            if record.outcome != OpOutcome::Success {
                continue;
            }

            match &record.operation {
                AdminOp::FillBlocks { specs } => {
                    for spec in specs {
                        if let Some(chunk) = chunks.get_mut(&spec.chunk_pos) {
                            for pos in spec.iter_positions() {
                                chunk.set(pos, spec.block);
                                result.blocks_modified += 1;
                            }
                            result.chunks_modified.insert(spec.chunk_pos);
                        }
                    }
                    result.operations_replayed += 1;
                }
                AdminOp::ReplaceBlocks { specs } => {
                    for spec in specs {
                        if let Some(chunk) = chunks.get_mut(&spec.chunk_pos) {
                            for pos in spec.iter_positions() {
                                if chunk.get(pos) == spec.from_block {
                                    chunk.set(pos, spec.to_block);
                                    result.blocks_modified += 1;
                                }
                            }
                            result.chunks_modified.insert(spec.chunk_pos);
                        }
                    }
                    result.operations_replayed += 1;
                }
                _ => {}
            }
        }

        result
    }

    /// Clear all records (for testing).
    pub fn clear(&mut self) {
        self.records.clear();
        self.next_index = 0;
        self.quarantines.clear();
        self.markers.clear();
        self.player_bans.clear();
    }

    /// Truncate records before a tick.
    pub fn truncate_before(&mut self, tick: u64) {
        self.records.retain(|r| r.metadata.tick >= tick);
    }

    /// Check if records are sorted.
    #[must_use]
    pub fn is_sorted(&self) -> bool {
        self.records.windows(2).all(|w| w[0] <= w[1])
    }

    /// Sort records into deterministic order.
    pub fn sort(&mut self) {
        self.records.sort();
    }
}

/// Result of replaying operations.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Number of operations replayed.
    pub operations_replayed: usize,
    /// Number of blocks modified.
    pub blocks_modified: usize,
    /// Chunks that were modified.
    pub chunks_modified: HashSet<ChunkPos>,
}

impl ReplayResult {
    /// Check if any changes were made.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.blocks_modified > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{AIR, STONE};

    fn test_metadata() -> AdminMetadata {
        AdminMetadata::new(1, AuthLevel::Admin, 100).with_reason("test operation")
    }

    fn test_bounds() -> WorldBounds {
        WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(2, 2, 2))
    }

    #[test]
    fn test_admin_op_id_generation() {
        let id1 = AdminOpId::generate(0, 12345);
        let id2 = AdminOpId::generate(0, 12345);
        let id3 = AdminOpId::generate(1, 12345);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_world_bounds_single() {
        let bounds = WorldBounds::single(ChunkPos::new(5, 5, 5));
        assert_eq!(bounds.chunk_count(), 1);
        assert!(bounds.contains(ChunkPos::new(5, 5, 5)));
        assert!(!bounds.contains(ChunkPos::new(4, 5, 5)));
    }

    #[test]
    fn test_world_bounds_centered() {
        let bounds = WorldBounds::centered(ChunkPos::new(0, 0, 0), 1);
        assert_eq!(bounds.chunk_count(), 27);
        assert!(bounds.contains(ChunkPos::new(0, 0, 0)));
        assert!(bounds.contains(ChunkPos::new(1, 1, 1)));
        assert!(bounds.contains(ChunkPos::new(-1, -1, -1)));
    }

    #[test]
    fn test_world_bounds_iter() {
        let bounds = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(1, 1, 1));
        let chunks: Vec<_> = bounds.iter().collect();
        assert_eq!(chunks.len(), 8);
    }

    #[test]
    fn test_world_bounds_validation() {
        let valid = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(10, 10, 10));
        assert!(valid.is_valid());

        let too_big = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(100, 100, 100));
        assert!(!too_big.is_valid());
    }

    #[test]
    fn test_block_fill_spec() {
        let spec = BlockFillSpec::new(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            LocalPos::new(3, 3, 3),
            STONE,
        );
        assert_eq!(spec.block_count(), 64);
        assert!(spec.is_valid());

        let delta = spec.to_delta();
        assert_eq!(delta.len(), 64);
    }

    #[test]
    fn test_block_fill_spec_entire_chunk() {
        let spec = BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE);
        assert_eq!(spec.block_count(), 4096);
        assert!(spec.is_valid());
    }

    #[test]
    fn test_block_replace_spec() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(1, 0, 0), STONE);
        chunk.set(LocalPos::new(2, 0, 0), AIR);

        let spec = BlockReplaceSpec::new(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(0, 0, 0),
            LocalPos::new(2, 0, 0),
            STONE,
            AIR,
        );

        assert_eq!(spec.count_matches(&chunk), 2);

        let delta = spec.apply_to_delta(&chunk);
        assert_eq!(delta.len(), 2);
    }

    #[test]
    fn test_player_moderation_record_kick() {
        let record = PlayerModerationRecord::kick(42, 1, 100, "test kick");
        assert_eq!(record.action, ModerationAction::Kick);
        assert_eq!(record.player_id, 42);
        assert!(!record.is_active_at(100));
    }

    #[test]
    fn test_player_moderation_record_ban() {
        let record = PlayerModerationRecord::ban(42, 1, 100, "test ban", Some(1000));
        assert!(record.is_active_at(100));
        assert!(record.is_active_at(1099));
        assert!(!record.is_active_at(1100));
        assert!(record.is_expired_at(1100));
    }

    #[test]
    fn test_player_moderation_record_permanent_ban() {
        let record = PlayerModerationRecord::ban(42, 1, 100, "permanent ban", None);
        assert!(record.is_active_at(100));
        assert!(record.is_active_at(u64::MAX - 1));
    }

    #[test]
    fn test_admin_metadata_authorization() {
        let operator = AdminMetadata::new(1, AuthLevel::Operator, 100);
        let moderator = AdminMetadata::new(1, AuthLevel::Moderator, 100);
        let admin = AdminMetadata::new(1, AuthLevel::Admin, 100);

        let fill_op = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };
        let kick_op = AdminOp::PlayerKick {
            record: PlayerModerationRecord::kick(42, 1, 100, "test"),
        };
        let repair_op = AdminOp::ApplyRepairPlan {
            plan: RepairPlan::new(1000),
        };

        assert!(operator.is_authorized_for(&fill_op));
        assert!(!operator.is_authorized_for(&kick_op));
        assert!(!operator.is_authorized_for(&repair_op));

        assert!(moderator.is_authorized_for(&fill_op));
        assert!(moderator.is_authorized_for(&kick_op));
        assert!(!moderator.is_authorized_for(&repair_op));

        assert!(admin.is_authorized_for(&fill_op));
        assert!(admin.is_authorized_for(&kick_op));
        assert!(admin.is_authorized_for(&repair_op));
    }

    #[test]
    fn test_admin_op_validation() {
        let valid_fill = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };
        assert!(valid_fill.validate().valid);

        let valid_bounds = AdminOp::QuarantineRegion {
            bounds: test_bounds(),
            status: QuarantineStatus {
                quarantined_at: 100,
                admin_id: 1,
                reason: "test".to_string(),
                severity: QuarantineSeverity::Medium,
            },
        };
        assert!(valid_bounds.validate().valid);
    }

    #[test]
    fn test_admin_op_fingerprint_deterministic() {
        let op1 = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };
        let op2 = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };
        assert_eq!(op1.fingerprint(), op2.fingerprint());
    }

    #[test]
    fn test_admin_op_fingerprint_differs() {
        let op1 = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };
        let op2 = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(1, 0, 0), STONE)],
        };
        assert_ne!(op1.fingerprint(), op2.fingerprint());
    }

    #[test]
    fn test_admin_log_new() {
        let log = AdminLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_admin_log_append() {
        let mut log = AdminLog::with_seed(12345);

        let op = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };

        log.append(test_metadata(), op, OpOutcome::Success);

        assert_eq!(log.len(), 1);
        assert_eq!(log.records()[0].log_index, 0);
    }

    #[test]
    fn test_admin_log_dry_run() {
        let log = AdminLog::new();

        let op = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };

        let result = log.dry_run(&test_metadata(), &op);
        assert!(result.valid);
        assert_eq!(result.estimated_blocks, 4096);
        assert_eq!(result.affected_chunks.len(), 1);
    }

    #[test]
    fn test_admin_log_dry_run_unauthorized() {
        let log = AdminLog::new();
        let operator = AdminMetadata::new(1, AuthLevel::Operator, 100);

        let op = AdminOp::ApplyRepairPlan {
            plan: RepairPlan::new(1000),
        };

        let result = log.dry_run(&operator, &op);
        assert!(!result.valid);
    }

    #[test]
    fn test_admin_log_quarantine_tracking() {
        let mut log = AdminLog::new();

        let bounds = test_bounds();
        let status = QuarantineStatus {
            quarantined_at: 100,
            admin_id: 1,
            reason: "test".to_string(),
            severity: QuarantineSeverity::High,
        };

        log.append(
            test_metadata(),
            AdminOp::QuarantineRegion {
                bounds,
                status: status.clone(),
            },
            OpOutcome::Success,
        );

        assert!(log.is_quarantined(&bounds));
        assert_eq!(
            log.quarantine_status(&bounds).unwrap().severity,
            QuarantineSeverity::High
        );

        log.append(
            test_metadata(),
            AdminOp::UnquarantineRegion { bounds },
            OpOutcome::Success,
        );

        assert!(!log.is_quarantined(&bounds));
    }

    #[test]
    fn test_admin_log_ban_tracking() {
        let mut log = AdminLog::new();

        let ban = PlayerModerationRecord::ban(42, 1, 100, "test", Some(1000));
        log.append(
            test_metadata(),
            AdminOp::PlayerBan { record: ban },
            OpOutcome::Success,
        );

        assert!(log.is_banned(42, 100));
        assert!(log.is_banned(42, 1099));
        assert!(!log.is_banned(42, 1100));

        let unban = PlayerModerationRecord::unban(42, 1, 1000, "unbanned");
        log.append(
            AdminMetadata::new(1, AuthLevel::Moderator, 1000),
            AdminOp::PlayerUnban { record: unban },
            OpOutcome::Success,
        );

        assert!(!log.is_banned(42, 1050));
    }

    #[test]
    fn test_admin_log_query() {
        let mut log = AdminLog::with_seed(12345);

        for i in 0..10 {
            let op = AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(i, 0, 0), STONE)],
            };
            log.append(
                AdminMetadata::new(1, AuthLevel::Admin, 100 + u64::from(i.cast_unsigned())),
                op,
                OpOutcome::Success,
            );
        }

        let query = AdminQuery::new().tick_range(102, 105);
        let results = log.query(&query);
        assert_eq!(results.len(), 4);

        let query = AdminQuery::new().category(OpCategory::BlockEdit);
        let results = log.query(&query);
        assert_eq!(results.len(), 10);

        let query = AdminQuery::new().limit(3);
        let results = log.query(&query);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_admin_log_query_by_chunk() {
        let mut log = AdminLog::new();

        let op = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(5, 5, 5), STONE)],
        };
        log.append(test_metadata(), op, OpOutcome::Success);

        let query = AdminQuery::new().chunk_pos(ChunkPos::new(5, 5, 5));
        let results = log.query(&query);
        assert_eq!(results.len(), 1);

        let query = AdminQuery::new().chunk_pos(ChunkPos::new(0, 0, 0));
        let results = log.query(&query);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_admin_log_checksum_deterministic() {
        let mut log1 = AdminLog::with_seed(12345);
        let mut log2 = AdminLog::with_seed(12345);

        let op = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };

        log1.append(test_metadata(), op.clone(), OpOutcome::Success);
        log2.append(test_metadata(), op, OpOutcome::Success);

        assert_eq!(log1.checksum(), log2.checksum());
    }

    #[test]
    fn test_admin_log_checksum_differs() {
        let mut log1 = AdminLog::with_seed(12345);
        let mut log2 = AdminLog::with_seed(12345);

        let op1 = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };
        let op2 = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(1, 0, 0), STONE)],
        };

        log1.append(test_metadata(), op1, OpOutcome::Success);
        log2.append(test_metadata(), op2, OpOutcome::Success);

        assert_ne!(log1.checksum(), log2.checksum());
    }

    #[test]
    fn test_admin_log_stats() {
        let mut log = AdminLog::new();

        log.append_success(
            test_metadata(),
            AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
            },
            4096,
            1,
        );

        log.append_success(
            AdminMetadata::new(2, AuthLevel::Moderator, 200),
            AdminOp::PlayerKick {
                record: PlayerModerationRecord::kick(42, 2, 200, "test"),
            },
            0,
            0,
        );

        let stats = log.stats();
        assert_eq!(stats.record_count, 2);
        assert_eq!(stats.unique_admins, 2);
        assert_eq!(stats.total_blocks_modified, 4096);
        assert_eq!(*stats.by_category.get(&OpCategory::BlockEdit).unwrap(), 1);
        assert_eq!(*stats.by_category.get(&OpCategory::Moderation).unwrap(), 1);
    }

    #[test]
    fn test_admin_log_replay() {
        let mut log = AdminLog::new();

        log.append_success(
            test_metadata(),
            AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::new(
                    ChunkPos::new(0, 0, 0),
                    LocalPos::new(0, 0, 0),
                    LocalPos::new(3, 3, 3),
                    STONE,
                )],
            },
            64,
            1,
        );

        let mut chunks = HashMap::new();
        chunks.insert(ChunkPos::new(0, 0, 0), Chunk::new());

        let result = log.replay_block_ops(&mut chunks, 0, 200);

        assert_eq!(result.operations_replayed, 1);
        assert_eq!(result.blocks_modified, 64);
        assert!(result.chunks_modified.contains(&ChunkPos::new(0, 0, 0)));

        let chunk = chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(chunk.get(LocalPos::new(0, 0, 0)), STONE);
        assert_eq!(chunk.get(LocalPos::new(3, 3, 3)), STONE);
        assert_eq!(chunk.get(LocalPos::new(4, 4, 4)), AIR);
    }

    #[test]
    fn test_admin_record_ordering() {
        let mut log = AdminLog::new();

        log.append(
            AdminMetadata::new(1, AuthLevel::Admin, 200),
            AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
            },
            OpOutcome::Success,
        );

        log.append(
            AdminMetadata::new(1, AuthLevel::Admin, 100),
            AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(1, 0, 0), STONE)],
            },
            OpOutcome::Success,
        );

        assert!(!log.is_sorted());
        log.sort();
        assert!(log.is_sorted());

        assert_eq!(log.records()[0].metadata.tick, 100);
        assert_eq!(log.records()[1].metadata.tick, 200);
    }

    #[test]
    fn test_serde_roundtrip_admin_op() {
        let op = AdminOp::FillBlocks {
            specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
        };

        let json = serde_json::to_string(&op).unwrap();
        let recovered: AdminOp = serde_json::from_str(&json).unwrap();

        assert_eq!(op.fingerprint(), recovered.fingerprint());
    }

    #[test]
    fn test_serde_roundtrip_admin_log() {
        let mut log = AdminLog::with_seed(12345);

        log.append_success(
            test_metadata(),
            AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
            },
            4096,
            1,
        );

        let bytes = bincode::serialize(&log).unwrap();
        let recovered: AdminLog = bincode::deserialize(&bytes).unwrap();

        assert_eq!(log.len(), recovered.len());
        assert_eq!(log.checksum(), recovered.checksum());
    }

    #[test]
    fn test_serde_bincode_roundtrip_metadata() {
        let metadata = AdminMetadata::new(42, AuthLevel::Admin, 12345)
            .with_reason("test reason")
            .with_ticket("TICKET-123")
            .with_tag("important");

        let bytes = bincode::serialize(&metadata).unwrap();
        let recovered: AdminMetadata = bincode::deserialize(&bytes).unwrap();

        assert_eq!(metadata.admin_id, recovered.admin_id);
        assert_eq!(metadata.auth_level, recovered.auth_level);
        assert_eq!(metadata.reason, recovered.reason);
        assert_eq!(metadata.ticket_ref, recovered.ticket_ref);
        assert_eq!(metadata.tags, recovered.tags);
    }

    #[test]
    fn test_serde_bincode_roundtrip_moderation_record() {
        let record = PlayerModerationRecord::ban(42, 1, 100, "test ban", Some(1000))
            .with_context("additional context");

        let bytes = bincode::serialize(&record).unwrap();
        let recovered: PlayerModerationRecord = bincode::deserialize(&bytes).unwrap();

        assert_eq!(record.player_id, recovered.player_id);
        assert_eq!(record.action, recovered.action);
        assert_eq!(record.duration_ticks, recovered.duration_ticks);
        assert_eq!(record.expires_at, recovered.expires_at);
        assert_eq!(record.context, recovered.context);
    }

    #[test]
    fn test_marker_operations() {
        let mut log = AdminLog::new();

        let bounds = test_bounds();
        let marker = RegionMarker {
            label: "protected_area".to_string(),
            category: MarkerCategory::Protected,
            marked_at: 100,
            admin_id: 1,
            notes: Some("Important area".to_string()),
        };

        log.append(
            test_metadata(),
            AdminOp::MarkRegion {
                bounds,
                marker: marker.clone(),
            },
            OpOutcome::Success,
        );

        let markers = log.region_markers(&bounds).unwrap();
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].label, "protected_area");

        log.append(
            test_metadata(),
            AdminOp::UnmarkRegion {
                bounds,
                label: "protected_area".to_string(),
            },
            OpOutcome::Success,
        );

        assert!(log.region_markers(&bounds).is_none());
    }

    #[test]
    fn test_dry_run_result() {
        let result = DryRunResult::success(
            1000,
            vec![ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)],
            0xDEAD_BEEF,
        );

        assert!(result.valid);
        assert_eq!(result.estimated_blocks, 1000);
        assert_eq!(result.estimated_chunks, 2);
        assert_eq!(result.fingerprint, 0xDEAD_BEEF);

        let failed = DryRunResult::failed(vec!["error1".to_string(), "error2".to_string()]);
        assert!(!failed.valid);
        assert_eq!(failed.validation_errors.len(), 2);
    }

    #[test]
    fn test_auth_level_ordering() {
        assert!(AuthLevel::Operator < AuthLevel::Moderator);
        assert!(AuthLevel::Moderator < AuthLevel::Admin);
        assert!(AuthLevel::Admin < AuthLevel::System);
    }

    #[test]
    fn test_quarantine_severity_ordering() {
        assert!(QuarantineSeverity::Low < QuarantineSeverity::Medium);
        assert!(QuarantineSeverity::Medium < QuarantineSeverity::High);
        assert!(QuarantineSeverity::High < QuarantineSeverity::Critical);
    }

    #[test]
    fn test_world_bounds_fingerprint_deterministic() {
        let bounds1 = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(5, 5, 5));
        let bounds2 = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(5, 5, 5));

        assert_eq!(bounds1.fingerprint(), bounds2.fingerprint());
    }

    #[test]
    fn test_world_bounds_fingerprint_differs() {
        let bounds1 = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(5, 5, 5));
        let bounds2 = WorldBounds::new(ChunkPos::new(0, 0, 0), ChunkPos::new(6, 6, 6));

        assert_ne!(bounds1.fingerprint(), bounds2.fingerprint());
    }

    #[test]
    fn test_teleport_destination() {
        let dest = TeleportDestination::new(ChunkPos::new(10, 20, 30), LocalPos::new(8, 8, 8));

        assert_eq!(dest.chunk_pos, ChunkPos::new(10, 20, 30));
        assert_eq!(dest.local_pos.x(), 8);
    }

    #[test]
    fn test_op_category_coverage() {
        let ops = [
            AdminOp::ApplyRepairPlan {
                plan: RepairPlan::new(100),
            },
            AdminOp::RegenerateChunks {
                chunks: vec![ChunkPos::new(0, 0, 0)],
            },
            AdminOp::FillBlocks {
                specs: vec![BlockFillSpec::entire_chunk(ChunkPos::new(0, 0, 0), STONE)],
            },
            AdminOp::ReplaceBlocks {
                specs: vec![BlockReplaceSpec::new(
                    ChunkPos::new(0, 0, 0),
                    LocalPos::new(0, 0, 0),
                    LocalPos::new(15, 15, 15),
                    STONE,
                    AIR,
                )],
            },
            AdminOp::QuarantineRegion {
                bounds: test_bounds(),
                status: QuarantineStatus {
                    quarantined_at: 0,
                    admin_id: 0,
                    reason: String::new(),
                    severity: QuarantineSeverity::Low,
                },
            },
            AdminOp::UnquarantineRegion {
                bounds: test_bounds(),
            },
            AdminOp::MarkRegion {
                bounds: test_bounds(),
                marker: RegionMarker {
                    label: String::new(),
                    category: MarkerCategory::Info,
                    marked_at: 0,
                    admin_id: 0,
                    notes: None,
                },
            },
            AdminOp::UnmarkRegion {
                bounds: test_bounds(),
                label: String::new(),
            },
            AdminOp::PlayerKick {
                record: PlayerModerationRecord::kick(0, 0, 0, ""),
            },
            AdminOp::PlayerBan {
                record: PlayerModerationRecord::ban(0, 0, 0, "", None),
            },
            AdminOp::PlayerUnban {
                record: PlayerModerationRecord::unban(0, 0, 0, ""),
            },
            AdminOp::PlayerTeleport {
                record: PlayerModerationRecord::teleport(0, 0, 0, ""),
                destination: TeleportDestination::new(
                    ChunkPos::new(0, 0, 0),
                    LocalPos::new(0, 0, 0),
                ),
            },
        ];

        for op in ops {
            let _ = op.category();
            let _ = op.estimated_block_cost();
            let _ = op.validate();
            let _ = op.fingerprint();
        }
    }
}

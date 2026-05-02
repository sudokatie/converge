//! Parallel-reality diff/merge model for Fracture-style swaps.
//!
//! Provides high-level operations for managing parallel realities:
//! branching, diffing, merging, and atomic swaps.
//!
//! # Core Concepts
//!
//! - [`RealityId`]: Unique identifier for a parallel reality
//! - [`RealityBranch`]: Metadata about a reality branch
//! - [`RealityRegistry`]: Registry of all realities and their relationships
//! - [`RealityDiff`]: Differences between two realities
//! - [`MergeStrategy`]: Conflict resolution strategies
//! - [`FractureSwap`]: Atomic swap operations
//!
//! # Example
//!
//! ```ignore
//! let mut registry = RealityRegistry::new();
//! let branch_a = registry.fork(RealityId::ROOT, "timeline-a", tick)?;
//! let branch_b = registry.fork(RealityId::ROOT, "timeline-b", tick)?;
//!
//! // Compute differences
//! let diff = RealityDiff::compute(&registry, branch_a, branch_b);
//!
//! // Merge with strategy
//! let result = diff.merge(MergeStrategy::TargetWins)?;
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::chunk_delta::ChunkDelta;
use super::state_id::StateId;

/// Unique identifier for a parallel reality.
///
/// Reality 0 is always the root/canonical reality from which all others branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RealityId(pub u32);

impl RealityId {
    /// The root reality (always exists, canonical world state).
    pub const ROOT: Self = Self(0);

    /// Create a new reality ID.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.0
    }

    /// Check if this is the root reality.
    #[must_use]
    pub const fn is_root(self) -> bool {
        self.0 == 0
    }

    /// Convert to a [`StateId`] for per-chunk storage.
    ///
    /// Maps reality IDs to the alternate-dimension range of [`StateId`].
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn to_state_id(self) -> StateId {
        if self.0 == 0 {
            StateId::PRIMARY
        } else {
            StateId::new(self.0 as u16)
        }
    }
}

impl Default for RealityId {
    fn default() -> Self {
        Self::ROOT
    }
}

impl std::fmt::Display for RealityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_root() {
            write!(f, "reality:root")
        } else {
            write!(f, "reality:{}", self.0)
        }
    }
}

/// Metadata about a reality branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealityBranch {
    /// Unique identifier for this reality.
    pub id: RealityId,

    /// Parent reality this was forked from (None for ROOT).
    pub parent: Option<RealityId>,

    /// Human-readable name/description.
    pub name: String,

    /// Game tick when this reality was created/forked.
    pub divergence_tick: u64,

    /// Optional tag for categorization.
    pub tag: Option<RealityTag>,

    /// Whether this reality is sealed (no further modifications allowed).
    pub sealed: bool,

    /// Number of chunks modified from parent.
    pub modified_chunk_count: u32,
}

impl RealityBranch {
    /// Create a new reality branch.
    #[must_use]
    pub fn new(id: RealityId, parent: Option<RealityId>, name: String, tick: u64) -> Self {
        Self {
            id,
            parent,
            name,
            divergence_tick: tick,
            tag: None,
            sealed: false,
            modified_chunk_count: 0,
        }
    }

    /// Create the root reality branch.
    #[must_use]
    pub fn root() -> Self {
        Self::new(RealityId::ROOT, None, "root".to_string(), 0)
    }

    /// Check if this is the root reality.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.id.is_root()
    }

    /// Set a tag for this branch.
    #[must_use]
    pub fn with_tag(mut self, tag: RealityTag) -> Self {
        self.tag = Some(tag);
        self
    }

    /// Compute the fingerprint for this branch metadata.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.parent.hash(&mut hasher);
        self.name.hash(&mut hasher);
        self.divergence_tick.hash(&mut hasher);
        self.tag.hash(&mut hasher);
        self.sealed.hash(&mut hasher);
        self.modified_chunk_count.hash(&mut hasher);
        hasher.finish()
    }
}

/// Tags for categorizing realities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RealityTag {
    /// Primary timeline.
    Primary,
    /// Alternative timeline created by player choice.
    Alternative,
    /// Temporary branch for testing/preview.
    Temporary,
    /// Time-loop iteration.
    TimeLoop { iteration: u32 },
    /// Phased/ghost dimension.
    Phased { phase: u8 },
    /// Player-created snapshot.
    Snapshot,
    /// Failed/abandoned timeline.
    Abandoned,
}

/// Error types for reality operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealityError {
    /// Reality not found in registry.
    NotFound(RealityId),
    /// Cannot fork from a sealed reality.
    SealedReality(RealityId),
    /// Reality already exists.
    AlreadyExists(RealityId),
    /// Cannot modify root reality.
    CannotModifyRoot,
    /// Merge conflict detected.
    MergeConflict(Vec<ChunkConflict>),
    /// Invalid parent reference.
    InvalidParent(RealityId),
    /// Circular dependency detected.
    CircularDependency,
    /// ID overflow.
    IdOverflow,
}

impl std::fmt::Display for RealityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "reality not found: {id}"),
            Self::SealedReality(id) => write!(f, "reality is sealed: {id}"),
            Self::AlreadyExists(id) => write!(f, "reality already exists: {id}"),
            Self::CannotModifyRoot => write!(f, "cannot modify root reality"),
            Self::MergeConflict(conflicts) => {
                write!(f, "merge conflict: {} chunks", conflicts.len())
            }
            Self::InvalidParent(id) => write!(f, "invalid parent: {id}"),
            Self::CircularDependency => write!(f, "circular dependency detected"),
            Self::IdOverflow => write!(f, "reality ID overflow"),
        }
    }
}

impl std::error::Error for RealityError {}

/// Registry of all realities and their relationships.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealityRegistry {
    /// All registered reality branches.
    branches: BTreeMap<RealityId, RealityBranch>,

    /// Children of each reality (for tree traversal).
    children: BTreeMap<RealityId, BTreeSet<RealityId>>,

    /// Next available reality ID.
    next_id: u32,

    /// Currently active reality for gameplay.
    active: RealityId,
}

impl RealityRegistry {
    /// Create a new registry with only the root reality.
    #[must_use]
    pub fn new() -> Self {
        let mut branches = BTreeMap::new();
        branches.insert(RealityId::ROOT, RealityBranch::root());

        let mut children = BTreeMap::new();
        children.insert(RealityId::ROOT, BTreeSet::new());

        Self {
            branches,
            children,
            next_id: 1,
            active: RealityId::ROOT,
        }
    }

    /// Get the currently active reality.
    #[must_use]
    pub fn active(&self) -> RealityId {
        self.active
    }

    /// Set the active reality.
    ///
    /// # Errors
    ///
    /// Returns error if reality doesn't exist.
    pub fn set_active(&mut self, id: RealityId) -> Result<(), RealityError> {
        if !self.branches.contains_key(&id) {
            return Err(RealityError::NotFound(id));
        }
        self.active = id;
        Ok(())
    }

    /// Get a reality branch by ID.
    #[must_use]
    pub fn get(&self, id: RealityId) -> Option<&RealityBranch> {
        self.branches.get(&id)
    }

    /// Get a mutable reference to a reality branch.
    pub fn get_mut(&mut self, id: RealityId) -> Option<&mut RealityBranch> {
        self.branches.get_mut(&id)
    }

    /// Check if a reality exists.
    #[must_use]
    pub fn contains(&self, id: RealityId) -> bool {
        self.branches.contains_key(&id)
    }

    /// Get the number of registered realities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// Check if the registry is empty (only root).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.branches.len() <= 1
    }

    /// Fork a new reality from an existing one.
    ///
    /// Creates a new reality branching from `parent` at the given tick.
    ///
    /// # Errors
    ///
    /// Returns error if parent doesn't exist or is sealed.
    pub fn fork(
        &mut self,
        parent: RealityId,
        name: impl Into<String>,
        tick: u64,
    ) -> Result<RealityId, RealityError> {
        let parent_branch = self
            .branches
            .get(&parent)
            .ok_or(RealityError::NotFound(parent))?;

        if parent_branch.sealed {
            return Err(RealityError::SealedReality(parent));
        }

        let id = RealityId::new(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(RealityError::IdOverflow)?;

        let branch = RealityBranch::new(id, Some(parent), name.into(), tick);
        self.branches.insert(id, branch);
        self.children.entry(parent).or_default().insert(id);
        self.children.insert(id, BTreeSet::new());

        Ok(id)
    }

    /// Seal a reality, preventing further modifications.
    ///
    /// # Errors
    ///
    /// Returns error if reality doesn't exist or is root.
    pub fn seal(&mut self, id: RealityId) -> Result<(), RealityError> {
        if id.is_root() {
            return Err(RealityError::CannotModifyRoot);
        }
        let branch = self
            .branches
            .get_mut(&id)
            .ok_or(RealityError::NotFound(id))?;
        branch.sealed = true;
        Ok(())
    }

    /// Remove a reality and all its descendants.
    ///
    /// Returns the number of realities removed.
    ///
    /// # Errors
    ///
    /// Returns error if reality doesn't exist or is root.
    pub fn prune(&mut self, id: RealityId) -> Result<usize, RealityError> {
        if id.is_root() {
            return Err(RealityError::CannotModifyRoot);
        }
        if !self.branches.contains_key(&id) {
            return Err(RealityError::NotFound(id));
        }

        let mut to_remove = vec![id];
        let mut removed = 0;

        while let Some(current) = to_remove.pop() {
            if let Some(children) = self.children.remove(&current) {
                to_remove.extend(children);
            }
            if let Some(branch) = self.branches.remove(&current) {
                if let Some(parent) = branch.parent
                    && let Some(siblings) = self.children.get_mut(&parent)
                {
                    siblings.remove(&current);
                }
                removed += 1;
            }
        }

        if self.active == id || !self.branches.contains_key(&self.active) {
            self.active = RealityId::ROOT;
        }

        Ok(removed)
    }

    /// Get the children of a reality.
    #[must_use]
    pub fn children_of(&self, id: RealityId) -> Option<&BTreeSet<RealityId>> {
        self.children.get(&id)
    }

    /// Get ancestors of a reality (path to root).
    #[must_use]
    pub fn ancestors(&self, id: RealityId) -> Vec<RealityId> {
        let mut path = Vec::new();
        let mut current = id;

        while let Some(branch) = self.branches.get(&current) {
            if let Some(parent) = branch.parent {
                path.push(parent);
                current = parent;
            } else {
                break;
            }
        }

        path
    }

    /// Find the common ancestor of two realities.
    #[must_use]
    pub fn common_ancestor(&self, a: RealityId, b: RealityId) -> Option<RealityId> {
        let ancestors_a: BTreeSet<_> = std::iter::once(a).chain(self.ancestors(a)).collect();

        let mut current = b;
        loop {
            if ancestors_a.contains(&current) {
                return Some(current);
            }
            match self.branches.get(&current).and_then(|b| b.parent) {
                Some(parent) => current = parent,
                None => break,
            }
        }

        None
    }

    /// Iterate over all realities.
    pub fn iter(&self) -> impl Iterator<Item = (&RealityId, &RealityBranch)> {
        self.branches.iter()
    }

    /// Iterate over reality IDs.
    pub fn ids(&self) -> impl Iterator<Item = RealityId> + '_ {
        self.branches.keys().copied()
    }

    /// Get depth of a reality in the tree (root = 0).
    #[must_use]
    pub fn depth(&self, id: RealityId) -> usize {
        self.ancestors(id).len()
    }

    /// Compute a stable fingerprint for the registry.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        for (id, branch) in &self.branches {
            id.hash(&mut hasher);
            branch.fingerprint().hash(&mut hasher);
        }
        self.next_id.hash(&mut hasher);
        self.active.hash(&mut hasher);

        hasher.finish()
    }

    /// Get a summary of the registry state.
    #[must_use]
    pub fn summary(&self) -> RealityRegistrySummary {
        let mut max_depth = 0;
        let mut sealed_count = 0;
        let mut total_modified_chunks = 0;

        for branch in self.branches.values() {
            let depth = self.depth(branch.id);
            max_depth = max_depth.max(depth);
            if branch.sealed {
                sealed_count += 1;
            }
            total_modified_chunks += branch.modified_chunk_count;
        }

        RealityRegistrySummary {
            reality_count: self.branches.len(),
            max_depth,
            sealed_count,
            total_modified_chunks,
            active: self.active,
        }
    }
}

impl Default for RealityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of registry state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealityRegistrySummary {
    /// Total number of realities.
    pub reality_count: usize,
    /// Maximum tree depth.
    pub max_depth: usize,
    /// Number of sealed realities.
    pub sealed_count: usize,
    /// Total modified chunks across all realities.
    pub total_modified_chunks: u32,
    /// Currently active reality.
    pub active: RealityId,
}

/// Entry in a reality diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDiffEntry {
    /// Position of the chunk.
    pub pos: ChunkPos,

    /// Changes in source reality relative to common ancestor.
    pub source_delta: ChunkDelta,

    /// Changes in target reality relative to common ancestor.
    pub target_delta: ChunkDelta,

    /// Whether this entry has conflicting changes.
    pub has_conflict: bool,
}

impl ChunkDiffEntry {
    /// Create a new diff entry.
    #[must_use]
    pub fn new(pos: ChunkPos, source_delta: ChunkDelta, target_delta: ChunkDelta) -> Self {
        let has_conflict = Self::check_conflict(&source_delta, &target_delta);
        Self {
            pos,
            source_delta,
            target_delta,
            has_conflict,
        }
    }

    /// Check if two deltas conflict.
    fn check_conflict(source: &ChunkDelta, target: &ChunkDelta) -> bool {
        for (pos, source_block) in source.iter() {
            if let Some(target_block) = target.get(pos)
                && source_block != target_block
            {
                return true;
            }
        }
        false
    }

    /// Compute fingerprint for this entry.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.pos.hash(&mut hasher);

        for (pos, block) in self.source_delta.iter() {
            pos.hash(&mut hasher);
            block.hash(&mut hasher);
        }
        for (pos, block) in self.target_delta.iter() {
            pos.hash(&mut hasher);
            block.hash(&mut hasher);
        }
        self.has_conflict.hash(&mut hasher);

        hasher.finish()
    }
}

/// Conflict at a specific chunk position.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkConflict {
    /// Position of the conflicting chunk.
    pub pos: ChunkPos,

    /// Positions within the chunk that conflict.
    pub conflicting_blocks: Vec<engine_core::coords::LocalPos>,

    /// Source block values at conflict positions.
    pub source_values: Vec<crate::chunk::BlockId>,

    /// Target block values at conflict positions.
    pub target_values: Vec<crate::chunk::BlockId>,
}

impl ChunkConflict {
    /// Create a new chunk conflict.
    #[must_use]
    pub fn new(pos: ChunkPos) -> Self {
        Self {
            pos,
            conflicting_blocks: Vec::new(),
            source_values: Vec::new(),
            target_values: Vec::new(),
        }
    }

    /// Add a conflicting block position.
    pub fn add_block(
        &mut self,
        local: engine_core::coords::LocalPos,
        source: crate::chunk::BlockId,
        target: crate::chunk::BlockId,
    ) {
        self.conflicting_blocks.push(local);
        self.source_values.push(source);
        self.target_values.push(target);
    }

    /// Get the number of conflicting blocks.
    #[must_use]
    pub fn conflict_count(&self) -> usize {
        self.conflicting_blocks.len()
    }

    /// Check if this conflict is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.conflicting_blocks.is_empty()
    }
}

/// Diff between two parallel realities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealityDiff {
    /// Source reality.
    pub source: RealityId,

    /// Target reality.
    pub target: RealityId,

    /// Common ancestor reality.
    pub common_ancestor: RealityId,

    /// Per-chunk differences.
    pub chunks: HashMap<ChunkPos, ChunkDiffEntry>,

    /// Conflicts detected.
    pub conflicts: Vec<ChunkConflict>,
}

impl RealityDiff {
    /// Create a new empty diff.
    #[must_use]
    pub fn new(source: RealityId, target: RealityId, common_ancestor: RealityId) -> Self {
        Self {
            source,
            target,
            common_ancestor,
            chunks: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Add a chunk diff entry.
    pub fn add_chunk(&mut self, entry: ChunkDiffEntry) {
        if entry.has_conflict {
            let mut conflict = ChunkConflict::new(entry.pos);
            for (pos, source_block) in entry.source_delta.iter() {
                if let Some(target_block) = entry.target_delta.get(pos)
                    && source_block != target_block
                {
                    conflict.add_block(pos, source_block, target_block);
                }
            }
            if !conflict.is_empty() {
                self.conflicts.push(conflict);
            }
        }
        self.chunks.insert(entry.pos, entry);
    }

    /// Check if there are any conflicts.
    #[must_use]
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Get the number of modified chunks.
    #[must_use]
    pub fn modified_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get the total number of conflicting blocks.
    #[must_use]
    pub fn total_conflict_blocks(&self) -> usize {
        self.conflicts
            .iter()
            .map(ChunkConflict::conflict_count)
            .sum()
    }

    /// Get chunks that only changed in source.
    pub fn source_only(&self) -> impl Iterator<Item = &ChunkDiffEntry> {
        self.chunks
            .values()
            .filter(|e| !e.source_delta.is_empty() && e.target_delta.is_empty())
    }

    /// Get chunks that only changed in target.
    pub fn target_only(&self) -> impl Iterator<Item = &ChunkDiffEntry> {
        self.chunks
            .values()
            .filter(|e| e.source_delta.is_empty() && !e.target_delta.is_empty())
    }

    /// Get chunks that changed in both.
    pub fn both_changed(&self) -> impl Iterator<Item = &ChunkDiffEntry> {
        self.chunks
            .values()
            .filter(|e| !e.source_delta.is_empty() && !e.target_delta.is_empty())
    }

    /// Compute a stable fingerprint for this diff.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        self.source.hash(&mut hasher);
        self.target.hash(&mut hasher);
        self.common_ancestor.hash(&mut hasher);

        // Sort keys for deterministic iteration
        let mut keys: Vec<_> = self.chunks.keys().collect();
        keys.sort_by_key(|a| (a.x(), a.y(), a.z()));
        for pos in keys {
            pos.hash(&mut hasher);
            self.chunks[pos].fingerprint().hash(&mut hasher);
        }

        self.conflicts.len().hash(&mut hasher);
        for conflict in &self.conflicts {
            conflict.pos.hash(&mut hasher);
            conflict.conflict_count().hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Get a summary of this diff.
    #[must_use]
    pub fn summary(&self) -> RealityDiffSummary {
        RealityDiffSummary {
            source: self.source,
            target: self.target,
            common_ancestor: self.common_ancestor,
            modified_chunks: self.chunks.len(),
            source_only_chunks: self.source_only().count(),
            target_only_chunks: self.target_only().count(),
            both_changed_chunks: self.both_changed().count(),
            conflict_count: self.conflicts.len(),
            conflict_blocks: self.total_conflict_blocks(),
        }
    }
}

/// Summary of a reality diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealityDiffSummary {
    /// Source reality.
    pub source: RealityId,
    /// Target reality.
    pub target: RealityId,
    /// Common ancestor.
    pub common_ancestor: RealityId,
    /// Total modified chunks.
    pub modified_chunks: usize,
    /// Chunks modified only in source.
    pub source_only_chunks: usize,
    /// Chunks modified only in target.
    pub target_only_chunks: usize,
    /// Chunks modified in both.
    pub both_changed_chunks: usize,
    /// Number of chunk conflicts.
    pub conflict_count: usize,
    /// Total conflicting blocks.
    pub conflict_blocks: usize,
}

/// Strategy for resolving merge conflicts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Source reality wins all conflicts.
    SourceWins,
    /// Target reality wins all conflicts.
    #[default]
    TargetWins,
    /// Fail if any conflicts exist.
    FailOnConflict,
    /// Use the older change (by tick).
    OlderWins,
    /// Use the newer change (by tick).
    NewerWins,
    /// Keep both changes on different layers.
    LayeredMerge,
}

/// Result of a merge operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeResult {
    /// Resulting merged deltas per chunk.
    pub merged_chunks: HashMap<ChunkPos, ChunkDelta>,

    /// Conflicts that were auto-resolved.
    pub auto_resolved: Vec<ResolvedConflict>,

    /// Strategy used for resolution.
    pub strategy: MergeStrategy,

    /// Whether merge succeeded.
    pub success: bool,

    /// Error message if failed.
    pub error: Option<String>,
}

impl MergeResult {
    /// Create a successful merge result.
    #[must_use]
    pub fn success(strategy: MergeStrategy) -> Self {
        Self {
            merged_chunks: HashMap::new(),
            auto_resolved: Vec::new(),
            strategy,
            success: true,
            error: None,
        }
    }

    /// Create a failed merge result.
    #[must_use]
    pub fn failure(strategy: MergeStrategy, error: impl Into<String>) -> Self {
        Self {
            merged_chunks: HashMap::new(),
            auto_resolved: Vec::new(),
            strategy,
            success: false,
            error: Some(error.into()),
        }
    }

    /// Add a merged chunk.
    pub fn add_chunk(&mut self, pos: ChunkPos, delta: ChunkDelta) {
        self.merged_chunks.insert(pos, delta);
    }

    /// Add an auto-resolved conflict.
    pub fn add_resolved(&mut self, resolved: ResolvedConflict) {
        self.auto_resolved.push(resolved);
    }

    /// Get fingerprint for this result.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        // Sort keys for deterministic iteration
        let mut keys: Vec<_> = self.merged_chunks.keys().collect();
        keys.sort_by_key(|a| (a.x(), a.y(), a.z()));
        for pos in keys {
            pos.hash(&mut hasher);
            for (local, block) in self.merged_chunks[pos].iter() {
                local.hash(&mut hasher);
                block.hash(&mut hasher);
            }
        }
        self.auto_resolved.len().hash(&mut hasher);
        self.strategy.hash(&mut hasher);
        self.success.hash(&mut hasher);

        hasher.finish()
    }
}

/// A conflict that was automatically resolved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedConflict {
    /// Chunk position.
    pub pos: ChunkPos,
    /// Block position within chunk.
    pub local: engine_core::coords::LocalPos,
    /// Original source value.
    pub source_value: crate::chunk::BlockId,
    /// Original target value.
    pub target_value: crate::chunk::BlockId,
    /// Chosen resolution value.
    pub resolved_value: crate::chunk::BlockId,
    /// Which side won.
    pub resolution: ConflictResolution,
}

/// How a conflict was resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Source value was chosen.
    SourceChosen,
    /// Target value was chosen.
    TargetChosen,
    /// Values were combined/layered.
    Combined,
}

/// Merge diff according to strategy.
///
/// # Errors
///
/// Returns error if strategy is `FailOnConflict` and conflicts exist.
pub fn merge_diff(
    diff: &RealityDiff,
    strategy: MergeStrategy,
) -> Result<MergeResult, RealityError> {
    if strategy == MergeStrategy::FailOnConflict && diff.has_conflicts() {
        return Err(RealityError::MergeConflict(diff.conflicts.clone()));
    }

    let mut result = MergeResult::success(strategy);

    for (pos, entry) in &diff.chunks {
        let mut merged = ChunkDelta::new();

        // Apply source changes
        for (local, block) in entry.source_delta.iter() {
            merged.set(local, block);
        }

        // Apply target changes (potentially overwriting source)
        for (local, block) in entry.target_delta.iter() {
            if let Some(source_block) = entry.source_delta.get(local) {
                if source_block == block {
                    merged.set(local, block);
                } else {
                    // Conflict - resolve according to strategy
                    let (chosen, resolution) = match strategy {
                        MergeStrategy::SourceWins => {
                            (source_block, ConflictResolution::SourceChosen)
                        }
                        MergeStrategy::TargetWins => (block, ConflictResolution::TargetChosen),
                        MergeStrategy::OlderWins | MergeStrategy::NewerWins => {
                            // For now, treat older/newer as source/target
                            (block, ConflictResolution::TargetChosen)
                        }
                        MergeStrategy::LayeredMerge => {
                            // For layered merge, keep target but record both
                            (block, ConflictResolution::Combined)
                        }
                        MergeStrategy::FailOnConflict => {
                            unreachable!("already checked for conflicts above")
                        }
                    };

                    merged.set(local, chosen);
                    result.add_resolved(ResolvedConflict {
                        pos: *pos,
                        local,
                        source_value: source_block,
                        target_value: block,
                        resolved_value: chosen,
                        resolution,
                    });
                }
            } else {
                merged.set(local, block);
            }
        }

        if !merged.is_empty() {
            result.add_chunk(*pos, merged);
        }
    }

    Ok(result)
}

/// A marked fracture point where reality swap can occur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FracturePoint {
    /// Unique identifier for this fracture.
    pub id: FractureId,

    /// Position of the fracture center.
    pub position: engine_core::coords::WorldPos,

    /// Radius of effect (in blocks).
    pub radius: u32,

    /// Source reality.
    pub source: RealityId,

    /// Target reality.
    pub target: RealityId,

    /// Whether the fracture is currently active.
    pub active: bool,

    /// Tick when fracture was created.
    pub created_tick: u64,

    /// Optional duration before fracture closes.
    pub duration_ticks: Option<u64>,
}

impl FracturePoint {
    /// Create a new fracture point.
    #[must_use]
    pub fn new(
        id: FractureId,
        position: engine_core::coords::WorldPos,
        radius: u32,
        source: RealityId,
        target: RealityId,
        tick: u64,
    ) -> Self {
        Self {
            id,
            position,
            radius,
            source,
            target,
            active: true,
            created_tick: tick,
            duration_ticks: None,
        }
    }

    /// Set duration for this fracture.
    #[must_use]
    pub fn with_duration(mut self, ticks: u64) -> Self {
        self.duration_ticks = Some(ticks);
        self
    }

    /// Check if fracture has expired.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        if let Some(duration) = self.duration_ticks {
            current_tick >= self.created_tick.saturating_add(duration)
        } else {
            false
        }
    }

    /// Get remaining duration if applicable.
    #[must_use]
    pub fn remaining_ticks(&self, current_tick: u64) -> Option<u64> {
        self.duration_ticks.map(|duration| {
            let end_tick = self.created_tick.saturating_add(duration);
            end_tick.saturating_sub(current_tick)
        })
    }

    /// Compute fingerprint for this fracture.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.position.hash(&mut hasher);
        self.radius.hash(&mut hasher);
        self.source.hash(&mut hasher);
        self.target.hash(&mut hasher);
        self.active.hash(&mut hasher);
        self.created_tick.hash(&mut hasher);
        self.duration_ticks.hash(&mut hasher);
        hasher.finish()
    }
}

/// Unique identifier for a fracture point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FractureId(pub u32);

impl FractureId {
    /// Create a new fracture ID.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for FractureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fracture:{}", self.0)
    }
}

/// Atomic swap operation between realities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FractureSwap {
    /// Fracture point triggering the swap.
    pub fracture: FractureId,

    /// Chunks affected by the swap.
    pub affected_chunks: Vec<ChunkPos>,

    /// Tick when swap occurred.
    pub swap_tick: u64,

    /// Whether swap is reversible.
    pub reversible: bool,

    /// State before swap (for rollback).
    pub before_state: Option<SwapSnapshot>,
}

impl FractureSwap {
    /// Create a new swap operation.
    #[must_use]
    pub fn new(fracture: FractureId, tick: u64) -> Self {
        Self {
            fracture,
            affected_chunks: Vec::new(),
            swap_tick: tick,
            reversible: true,
            before_state: None,
        }
    }

    /// Add an affected chunk.
    pub fn add_chunk(&mut self, pos: ChunkPos) {
        self.affected_chunks.push(pos);
    }

    /// Set reversibility.
    #[must_use]
    pub fn with_reversible(mut self, reversible: bool) -> Self {
        self.reversible = reversible;
        self
    }

    /// Store state before swap for rollback.
    pub fn store_before_state(&mut self, snapshot: SwapSnapshot) {
        self.before_state = Some(snapshot);
    }

    /// Get the number of affected chunks.
    #[must_use]
    pub fn affected_count(&self) -> usize {
        self.affected_chunks.len()
    }

    /// Compute fingerprint for this swap.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.fracture.hash(&mut hasher);
        self.swap_tick.hash(&mut hasher);
        self.reversible.hash(&mut hasher);
        for pos in &self.affected_chunks {
            pos.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Snapshot of state before a swap (for rollback).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapSnapshot {
    /// Per-chunk deltas representing the original state.
    pub chunk_states: HashMap<ChunkPos, ChunkDelta>,

    /// Active reality before swap.
    pub previous_active: RealityId,

    /// Tick of snapshot.
    pub snapshot_tick: u64,
}

impl SwapSnapshot {
    /// Create a new swap snapshot.
    #[must_use]
    pub fn new(previous_active: RealityId, tick: u64) -> Self {
        Self {
            chunk_states: HashMap::new(),
            previous_active,
            snapshot_tick: tick,
        }
    }

    /// Add a chunk state.
    pub fn add_chunk(&mut self, pos: ChunkPos, delta: ChunkDelta) {
        self.chunk_states.insert(pos, delta);
    }

    /// Get the number of stored chunks.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunk_states.len()
    }

    /// Compute fingerprint for this snapshot.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.previous_active.hash(&mut hasher);
        self.snapshot_tick.hash(&mut hasher);

        // Sort keys for deterministic iteration
        let mut keys: Vec<_> = self.chunk_states.keys().collect();
        keys.sort_by_key(|a| (a.x(), a.y(), a.z()));
        for pos in keys {
            pos.hash(&mut hasher);
            for (local, block) in self.chunk_states[pos].iter() {
                local.hash(&mut hasher);
                block.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

/// Registry for managing fracture points.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FractureRegistry {
    /// All registered fracture points.
    fractures: BTreeMap<FractureId, FracturePoint>,

    /// Next available fracture ID.
    next_id: u32,

    /// Spatial index: chunk -> fractures affecting it.
    chunk_index: HashMap<ChunkPos, BTreeSet<FractureId>>,
}

impl FractureRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new fracture point.
    ///
    /// Returns the assigned fracture ID.
    pub fn register(
        &mut self,
        position: engine_core::coords::WorldPos,
        radius: u32,
        source: RealityId,
        target: RealityId,
        tick: u64,
    ) -> FractureId {
        let id = FractureId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);

        let fracture = FracturePoint::new(id, position, radius, source, target, tick);
        self.fractures.insert(id, fracture);

        id
    }

    /// Get a fracture by ID.
    #[must_use]
    pub fn get(&self, id: FractureId) -> Option<&FracturePoint> {
        self.fractures.get(&id)
    }

    /// Get a mutable reference to a fracture.
    pub fn get_mut(&mut self, id: FractureId) -> Option<&mut FracturePoint> {
        self.fractures.get_mut(&id)
    }

    /// Remove a fracture by ID.
    ///
    /// Returns the removed fracture if it existed.
    pub fn remove(&mut self, id: FractureId) -> Option<FracturePoint> {
        if let Some(fracture) = self.fractures.remove(&id) {
            for chunks in self.chunk_index.values_mut() {
                chunks.remove(&id);
            }
            Some(fracture)
        } else {
            None
        }
    }

    /// Check if a fracture exists.
    #[must_use]
    pub fn contains(&self, id: FractureId) -> bool {
        self.fractures.contains_key(&id)
    }

    /// Get the number of fractures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fractures.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fractures.is_empty()
    }

    /// Iterate over all fractures.
    pub fn iter(&self) -> impl Iterator<Item = (&FractureId, &FracturePoint)> {
        self.fractures.iter()
    }

    /// Get all active fractures.
    pub fn active(&self) -> impl Iterator<Item = &FracturePoint> {
        self.fractures.values().filter(|f| f.active)
    }

    /// Get all expired fractures at a given tick.
    pub fn expired(&self, tick: u64) -> impl Iterator<Item = &FracturePoint> {
        self.fractures.values().filter(move |f| f.is_expired(tick))
    }

    /// Prune all expired fractures.
    ///
    /// Returns the number of fractures removed.
    pub fn prune_expired(&mut self, tick: u64) -> usize {
        let to_remove: Vec<_> = self
            .fractures
            .iter()
            .filter(|(_, f)| f.is_expired(tick))
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.remove(id);
        }
        count
    }

    /// Deactivate a fracture.
    pub fn deactivate(&mut self, id: FractureId) -> bool {
        if let Some(fracture) = self.fractures.get_mut(&id) {
            fracture.active = false;
            true
        } else {
            false
        }
    }

    /// Index a chunk as affected by a fracture.
    pub fn index_chunk(&mut self, chunk: ChunkPos, fracture_id: FractureId) {
        self.chunk_index
            .entry(chunk)
            .or_default()
            .insert(fracture_id);
    }

    /// Get fractures affecting a chunk.
    #[must_use]
    pub fn fractures_at_chunk(&self, chunk: ChunkPos) -> Option<&BTreeSet<FractureId>> {
        self.chunk_index.get(&chunk)
    }

    /// Get fractures connecting two realities.
    pub fn fractures_between(
        &self,
        source: RealityId,
        target: RealityId,
    ) -> impl Iterator<Item = &FracturePoint> {
        self.fractures.values().filter(move |f| {
            (f.source == source && f.target == target) || (f.source == target && f.target == source)
        })
    }

    /// Compute a stable fingerprint for the registry.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        for (id, fracture) in &self.fractures {
            id.hash(&mut hasher);
            fracture.fingerprint().hash(&mut hasher);
        }
        self.next_id.hash(&mut hasher);

        hasher.finish()
    }

    /// Get a summary of the registry.
    #[must_use]
    pub fn summary(&self) -> FractureRegistrySummary {
        let active_count = self.fractures.values().filter(|f| f.active).count();
        let indexed_chunks = self.chunk_index.len();

        FractureRegistrySummary {
            total_fractures: self.fractures.len(),
            active_fractures: active_count,
            indexed_chunks,
        }
    }
}

/// Summary of fracture registry state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FractureRegistrySummary {
    /// Total number of fractures.
    pub total_fractures: usize,
    /// Number of active fractures.
    pub active_fractures: usize,
    /// Number of indexed chunks.
    pub indexed_chunks: usize,
}

/// Checksum builder for parallel reality state.
#[derive(Clone, Debug, Default)]
pub struct RealityChecksumBuilder {
    hasher_state: u64,
}

impl RealityChecksumBuilder {
    /// Create a new checksum builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a reality ID to the checksum.
    pub fn add_reality(&mut self, id: RealityId) {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hasher_state.hash(&mut hasher);
        id.hash(&mut hasher);
        self.hasher_state = hasher.finish();
    }

    /// Add a branch to the checksum.
    pub fn add_branch(&mut self, branch: &RealityBranch) {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hasher_state.hash(&mut hasher);
        branch.fingerprint().hash(&mut hasher);
        self.hasher_state = hasher.finish();
    }

    /// Add a diff to the checksum.
    pub fn add_diff(&mut self, diff: &RealityDiff) {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hasher_state.hash(&mut hasher);
        diff.fingerprint().hash(&mut hasher);
        self.hasher_state = hasher.finish();
    }

    /// Finalize and return the checksum.
    #[must_use]
    pub fn finish(self) -> RealityChecksum {
        RealityChecksum(self.hasher_state)
    }
}

/// Stable checksum for parallel reality state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RealityChecksum(pub u64);

impl RealityChecksum {
    /// Get the raw checksum value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for RealityChecksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{BlockId, STONE};
    use engine_core::coords::LocalPos;

    #[test]
    fn test_reality_id_root() {
        assert!(RealityId::ROOT.is_root());
        assert_eq!(RealityId::ROOT.id(), 0);
        assert!(!RealityId::new(1).is_root());
    }

    #[test]
    fn test_reality_id_display() {
        assert_eq!(format!("{}", RealityId::ROOT), "reality:root");
        assert_eq!(format!("{}", RealityId::new(42)), "reality:42");
    }

    #[test]
    fn test_reality_id_to_state_id() {
        assert_eq!(RealityId::ROOT.to_state_id(), StateId::PRIMARY);
        assert_eq!(RealityId::new(5).to_state_id(), StateId::new(5));
    }

    #[test]
    fn test_reality_branch_new() {
        let branch =
            RealityBranch::new(RealityId::new(1), Some(RealityId::ROOT), "test".into(), 100);
        assert_eq!(branch.id, RealityId::new(1));
        assert_eq!(branch.parent, Some(RealityId::ROOT));
        assert_eq!(branch.name, "test");
        assert_eq!(branch.divergence_tick, 100);
        assert!(!branch.sealed);
    }

    #[test]
    fn test_reality_branch_root() {
        let root = RealityBranch::root();
        assert!(root.is_root());
        assert!(root.parent.is_none());
    }

    #[test]
    fn test_reality_branch_with_tag() {
        let branch = RealityBranch::new(RealityId::new(1), None, "test".into(), 0)
            .with_tag(RealityTag::Snapshot);
        assert_eq!(branch.tag, Some(RealityTag::Snapshot));
    }

    #[test]
    fn test_registry_new() {
        let registry = RealityRegistry::new();
        assert_eq!(registry.len(), 1);
        assert!(registry.contains(RealityId::ROOT));
        assert_eq!(registry.active(), RealityId::ROOT);
    }

    #[test]
    fn test_registry_fork() {
        let mut registry = RealityRegistry::new();
        let id = registry.fork(RealityId::ROOT, "branch-a", 100).unwrap();

        assert_eq!(id, RealityId::new(1));
        assert_eq!(registry.len(), 2);
        assert!(registry.contains(id));

        let branch = registry.get(id).unwrap();
        assert_eq!(branch.parent, Some(RealityId::ROOT));
        assert_eq!(branch.divergence_tick, 100);
    }

    #[test]
    fn test_registry_fork_from_missing() {
        let mut registry = RealityRegistry::new();
        let result = registry.fork(RealityId::new(999), "test", 0);
        assert!(matches!(result, Err(RealityError::NotFound(_))));
    }

    #[test]
    fn test_registry_fork_from_sealed() {
        let mut registry = RealityRegistry::new();
        let id = registry.fork(RealityId::ROOT, "sealed", 0).unwrap();
        registry.seal(id).unwrap();

        let result = registry.fork(id, "child", 100);
        assert!(matches!(result, Err(RealityError::SealedReality(_))));
    }

    #[test]
    fn test_registry_seal() {
        let mut registry = RealityRegistry::new();
        let id = registry.fork(RealityId::ROOT, "test", 0).unwrap();

        registry.seal(id).unwrap();
        assert!(registry.get(id).unwrap().sealed);
    }

    #[test]
    fn test_registry_seal_root() {
        let mut registry = RealityRegistry::new();
        let result = registry.seal(RealityId::ROOT);
        assert!(matches!(result, Err(RealityError::CannotModifyRoot)));
    }

    #[test]
    fn test_registry_prune() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        let _b = registry.fork(a, "b", 100).unwrap();
        let _c = registry.fork(a, "c", 200).unwrap();

        let removed = registry.prune(a).unwrap();
        assert_eq!(removed, 3);
        assert_eq!(registry.len(), 1);
        assert!(!registry.contains(a));
    }

    #[test]
    fn test_registry_prune_root() {
        let mut registry = RealityRegistry::new();
        let result = registry.prune(RealityId::ROOT);
        assert!(matches!(result, Err(RealityError::CannotModifyRoot)));
    }

    #[test]
    fn test_registry_ancestors() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        let b = registry.fork(a, "b", 100).unwrap();
        let c = registry.fork(b, "c", 200).unwrap();

        let ancestors = registry.ancestors(c);
        assert_eq!(ancestors, vec![b, a, RealityId::ROOT]);
    }

    #[test]
    fn test_registry_common_ancestor() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        let b = registry.fork(a, "b", 100).unwrap();
        let c = registry.fork(a, "c", 200).unwrap();

        assert_eq!(registry.common_ancestor(b, c), Some(a));
        assert_eq!(registry.common_ancestor(a, c), Some(a));
        assert_eq!(registry.common_ancestor(b, b), Some(b));
    }

    #[test]
    fn test_registry_depth() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        let b = registry.fork(a, "b", 100).unwrap();

        assert_eq!(registry.depth(RealityId::ROOT), 0);
        assert_eq!(registry.depth(a), 1);
        assert_eq!(registry.depth(b), 2);
    }

    #[test]
    fn test_registry_set_active() {
        let mut registry = RealityRegistry::new();
        let id = registry.fork(RealityId::ROOT, "test", 0).unwrap();

        registry.set_active(id).unwrap();
        assert_eq!(registry.active(), id);
    }

    #[test]
    fn test_registry_set_active_missing() {
        let mut registry = RealityRegistry::new();
        let result = registry.set_active(RealityId::new(999));
        assert!(matches!(result, Err(RealityError::NotFound(_))));
    }

    #[test]
    fn test_registry_fingerprint_deterministic() {
        let mut r1 = RealityRegistry::new();
        r1.fork(RealityId::ROOT, "test", 100).unwrap();

        let mut r2 = RealityRegistry::new();
        r2.fork(RealityId::ROOT, "test", 100).unwrap();

        assert_eq!(r1.fingerprint(), r2.fingerprint());
    }

    #[test]
    fn test_registry_serde_roundtrip() {
        let mut registry = RealityRegistry::new();
        registry.fork(RealityId::ROOT, "branch-a", 100).unwrap();
        registry.fork(RealityId::ROOT, "branch-b", 200).unwrap();

        let serialized = bincode::serialize(&registry).unwrap();
        let deserialized: RealityRegistry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(registry.len(), deserialized.len());
        assert_eq!(registry.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_chunk_diff_entry_no_conflict() {
        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let mut target_delta = ChunkDelta::new();
        target_delta.set(LocalPos::new(1, 0, 0), BlockId(100));

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        assert!(!entry.has_conflict);
    }

    #[test]
    fn test_chunk_diff_entry_with_conflict() {
        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let mut target_delta = ChunkDelta::new();
        target_delta.set(LocalPos::new(0, 0, 0), BlockId(100));

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        assert!(entry.has_conflict);
    }

    #[test]
    fn test_reality_diff_new() {
        let diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);
        assert_eq!(diff.source, RealityId::new(1));
        assert_eq!(diff.target, RealityId::new(2));
        assert_eq!(diff.common_ancestor, RealityId::ROOT);
        assert!(!diff.has_conflicts());
    }

    #[test]
    fn test_reality_diff_add_chunk() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let target_delta = ChunkDelta::new();

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        diff.add_chunk(entry);

        assert_eq!(diff.modified_chunk_count(), 1);
        assert_eq!(diff.source_only().count(), 1);
    }

    #[test]
    fn test_reality_diff_serde_roundtrip() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(5, 5, 5), STONE);

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, ChunkDelta::new());
        diff.add_chunk(entry);

        let serialized = bincode::serialize(&diff).unwrap();
        let deserialized: RealityDiff = bincode::deserialize(&serialized).unwrap();

        assert_eq!(diff.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_merge_diff_target_wins() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let mut target_delta = ChunkDelta::new();
        target_delta.set(LocalPos::new(0, 0, 0), BlockId(100));

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        diff.add_chunk(entry);

        let result = merge_diff(&diff, MergeStrategy::TargetWins).unwrap();
        assert!(result.success);
        assert_eq!(result.auto_resolved.len(), 1);

        let merged = result.merged_chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(merged.get(LocalPos::new(0, 0, 0)), Some(BlockId(100)));
    }

    #[test]
    fn test_merge_diff_source_wins() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let mut target_delta = ChunkDelta::new();
        target_delta.set(LocalPos::new(0, 0, 0), BlockId(100));

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        diff.add_chunk(entry);

        let result = merge_diff(&diff, MergeStrategy::SourceWins).unwrap();
        assert!(result.success);

        let merged = result.merged_chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(merged.get(LocalPos::new(0, 0, 0)), Some(STONE));
    }

    #[test]
    fn test_merge_diff_fail_on_conflict() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let mut target_delta = ChunkDelta::new();
        target_delta.set(LocalPos::new(0, 0, 0), BlockId(100));

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        diff.add_chunk(entry);

        let result = merge_diff(&diff, MergeStrategy::FailOnConflict);
        assert!(matches!(result, Err(RealityError::MergeConflict(_))));
    }

    #[test]
    fn test_merge_diff_no_conflict() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let mut target_delta = ChunkDelta::new();
        target_delta.set(LocalPos::new(1, 0, 0), BlockId(100));

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, target_delta);
        diff.add_chunk(entry);

        let result = merge_diff(&diff, MergeStrategy::FailOnConflict).unwrap();
        assert!(result.success);
        assert!(result.auto_resolved.is_empty());

        let merged = result.merged_chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert_eq!(merged.get(LocalPos::new(0, 0, 0)), Some(STONE));
        assert_eq!(merged.get(LocalPos::new(1, 0, 0)), Some(BlockId(100)));
    }

    #[test]
    fn test_fracture_point_new() {
        let fracture = FracturePoint::new(
            FractureId::new(1),
            engine_core::coords::WorldPos::new(100, 50, 100),
            16,
            RealityId::ROOT,
            RealityId::new(1),
            1000,
        );

        assert!(fracture.active);
        assert!(!fracture.is_expired(1000));
        assert!(fracture.remaining_ticks(1000).is_none());
    }

    #[test]
    fn test_fracture_point_with_duration() {
        let fracture = FracturePoint::new(
            FractureId::new(1),
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            1000,
        )
        .with_duration(500);

        assert!(!fracture.is_expired(1000));
        assert!(!fracture.is_expired(1499));
        assert!(fracture.is_expired(1500));
        assert_eq!(fracture.remaining_ticks(1000), Some(500));
        assert_eq!(fracture.remaining_ticks(1250), Some(250));
        assert_eq!(fracture.remaining_ticks(1600), Some(0));
    }

    #[test]
    fn test_fracture_swap_new() {
        let swap = FractureSwap::new(FractureId::new(1), 1000);
        assert!(swap.reversible);
        assert!(swap.affected_chunks.is_empty());
        assert!(swap.before_state.is_none());
    }

    #[test]
    fn test_fracture_swap_add_chunk() {
        let mut swap = FractureSwap::new(FractureId::new(1), 1000);
        swap.add_chunk(ChunkPos::new(0, 0, 0));
        swap.add_chunk(ChunkPos::new(1, 0, 0));

        assert_eq!(swap.affected_count(), 2);
    }

    #[test]
    fn test_swap_snapshot() {
        let mut snapshot = SwapSnapshot::new(RealityId::ROOT, 1000);

        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        snapshot.add_chunk(ChunkPos::new(0, 0, 0), delta);

        assert_eq!(snapshot.chunk_count(), 1);
        assert_eq!(snapshot.previous_active, RealityId::ROOT);
    }

    #[test]
    fn test_reality_checksum_builder() {
        let mut builder = RealityChecksumBuilder::new();
        builder.add_reality(RealityId::ROOT);
        builder.add_branch(&RealityBranch::root());

        let checksum = builder.finish();
        assert_ne!(checksum.value(), 0);
    }

    #[test]
    fn test_reality_checksum_deterministic() {
        let build = || {
            let mut builder = RealityChecksumBuilder::new();
            builder.add_reality(RealityId::new(1));
            builder.add_reality(RealityId::new(2));
            builder.finish()
        };

        assert_eq!(build(), build());
    }

    #[test]
    fn test_registry_summary() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        let _b = registry.fork(a, "b", 100).unwrap();
        registry.seal(a).unwrap();

        let summary = registry.summary();
        assert_eq!(summary.reality_count, 3);
        assert_eq!(summary.max_depth, 2);
        assert_eq!(summary.sealed_count, 1);
    }

    #[test]
    fn test_diff_summary() {
        let mut diff = RealityDiff::new(RealityId::new(1), RealityId::new(2), RealityId::ROOT);

        let mut source_delta = ChunkDelta::new();
        source_delta.set(LocalPos::new(0, 0, 0), STONE);

        let entry = ChunkDiffEntry::new(ChunkPos::new(0, 0, 0), source_delta, ChunkDelta::new());
        diff.add_chunk(entry);

        let summary = diff.summary();
        assert_eq!(summary.modified_chunks, 1);
        assert_eq!(summary.source_only_chunks, 1);
        assert_eq!(summary.target_only_chunks, 0);
        assert_eq!(summary.conflict_count, 0);
    }

    #[test]
    fn test_fracture_registry_new() {
        let registry = FractureRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_fracture_registry_register() {
        let mut registry = FractureRegistry::new();
        let id = registry.register(
            engine_core::coords::WorldPos::new(100, 50, 100),
            16,
            RealityId::ROOT,
            RealityId::new(1),
            1000,
        );

        assert_eq!(id, FractureId::new(0));
        assert!(registry.contains(id));
        assert_eq!(registry.len(), 1);

        let fracture = registry.get(id).unwrap();
        assert!(fracture.active);
        assert_eq!(fracture.radius, 16);
    }

    #[test]
    fn test_fracture_registry_remove() {
        let mut registry = FractureRegistry::new();
        let id = registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            0,
        );

        let removed = registry.remove(id);
        assert!(removed.is_some());
        assert!(!registry.contains(id));
        assert!(registry.is_empty());
    }

    #[test]
    fn test_fracture_registry_deactivate() {
        let mut registry = FractureRegistry::new();
        let id = registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            0,
        );

        assert!(registry.deactivate(id));
        assert!(!registry.get(id).unwrap().active);
    }

    #[test]
    fn test_fracture_registry_prune_expired() {
        let mut registry = FractureRegistry::new();
        let id = registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            1000,
        );

        if let Some(f) = registry.get_mut(id) {
            f.duration_ticks = Some(100);
        }

        assert_eq!(registry.prune_expired(1050), 0);
        assert_eq!(registry.prune_expired(1100), 1);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_fracture_registry_chunk_index() {
        let mut registry = FractureRegistry::new();
        let id = registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            0,
        );

        let chunk = ChunkPos::new(0, 0, 0);
        registry.index_chunk(chunk, id);

        let fractures = registry.fractures_at_chunk(chunk).unwrap();
        assert!(fractures.contains(&id));
    }

    #[test]
    fn test_fracture_registry_fractures_between() {
        let mut registry = FractureRegistry::new();
        registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            0,
        );
        registry.register(
            engine_core::coords::WorldPos::new(100, 0, 0),
            8,
            RealityId::new(1),
            RealityId::new(2),
            0,
        );

        let between_root_and_1: Vec<_> = registry
            .fractures_between(RealityId::ROOT, RealityId::new(1))
            .collect();
        assert_eq!(between_root_and_1.len(), 1);

        let between_1_and_2: Vec<_> = registry
            .fractures_between(RealityId::new(1), RealityId::new(2))
            .collect();
        assert_eq!(between_1_and_2.len(), 1);
    }

    #[test]
    fn test_fracture_registry_summary() {
        let mut registry = FractureRegistry::new();
        let id1 = registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            0,
        );
        registry.register(
            engine_core::coords::WorldPos::new(100, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(2),
            0,
        );
        registry.deactivate(id1);

        let summary = registry.summary();
        assert_eq!(summary.total_fractures, 2);
        assert_eq!(summary.active_fractures, 1);
    }

    #[test]
    fn test_fracture_point_serde_roundtrip() {
        let fracture = FracturePoint::new(
            FractureId::new(42),
            engine_core::coords::WorldPos::new(100, 50, 100),
            16,
            RealityId::ROOT,
            RealityId::new(1),
            1000,
        )
        .with_duration(500);

        let serialized = bincode::serialize(&fracture).unwrap();
        let deserialized: FracturePoint = bincode::deserialize(&serialized).unwrap();

        assert_eq!(fracture.id, deserialized.id);
        assert_eq!(fracture.position, deserialized.position);
        assert_eq!(fracture.radius, deserialized.radius);
        assert_eq!(fracture.duration_ticks, deserialized.duration_ticks);
        assert_eq!(fracture.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_fracture_swap_serde_roundtrip() {
        let mut swap = FractureSwap::new(FractureId::new(1), 1000);
        swap.add_chunk(ChunkPos::new(0, 0, 0));
        swap.add_chunk(ChunkPos::new(1, 0, 0));

        let serialized = bincode::serialize(&swap).unwrap();
        let deserialized: FractureSwap = bincode::deserialize(&serialized).unwrap();

        assert_eq!(swap.fracture, deserialized.fracture);
        assert_eq!(swap.affected_chunks, deserialized.affected_chunks);
        assert_eq!(swap.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_swap_snapshot_serde_roundtrip() {
        let mut snapshot = SwapSnapshot::new(RealityId::new(1), 1000);
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(5, 5, 5), STONE);
        snapshot.add_chunk(ChunkPos::new(0, 0, 0), delta);

        let serialized = bincode::serialize(&snapshot).unwrap();
        let deserialized: SwapSnapshot = bincode::deserialize(&serialized).unwrap();

        assert_eq!(snapshot.previous_active, deserialized.previous_active);
        assert_eq!(snapshot.snapshot_tick, deserialized.snapshot_tick);
        assert_eq!(snapshot.chunk_count(), deserialized.chunk_count());
        assert_eq!(snapshot.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_merge_result_serde_roundtrip() {
        let mut result = MergeResult::success(MergeStrategy::TargetWins);
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        result.add_chunk(ChunkPos::new(0, 0, 0), delta);

        let serialized = bincode::serialize(&result).unwrap();
        let deserialized: MergeResult = bincode::deserialize(&serialized).unwrap();

        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.strategy, deserialized.strategy);
        assert_eq!(result.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_fracture_registry_serde_roundtrip() {
        let mut registry = FractureRegistry::new();
        registry.register(
            engine_core::coords::WorldPos::new(0, 0, 0),
            8,
            RealityId::ROOT,
            RealityId::new(1),
            0,
        );
        registry.register(
            engine_core::coords::WorldPos::new(100, 0, 0),
            16,
            RealityId::new(1),
            RealityId::new(2),
            500,
        );

        let serialized = bincode::serialize(&registry).unwrap();
        let deserialized: FractureRegistry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(registry.len(), deserialized.len());
        assert_eq!(registry.fingerprint(), deserialized.fingerprint());
    }

    #[test]
    fn test_reality_tag_serde_roundtrip() {
        let tags = [
            RealityTag::Primary,
            RealityTag::Alternative,
            RealityTag::Temporary,
            RealityTag::TimeLoop { iteration: 42 },
            RealityTag::Phased { phase: 128 },
            RealityTag::Snapshot,
            RealityTag::Abandoned,
        ];

        for tag in tags {
            let serialized = bincode::serialize(&tag).unwrap();
            let deserialized: RealityTag = bincode::deserialize(&serialized).unwrap();
            assert_eq!(tag, deserialized);
        }
    }

    #[test]
    fn test_merge_strategy_serde_roundtrip() {
        let strategies = [
            MergeStrategy::SourceWins,
            MergeStrategy::TargetWins,
            MergeStrategy::FailOnConflict,
            MergeStrategy::OlderWins,
            MergeStrategy::NewerWins,
            MergeStrategy::LayeredMerge,
        ];

        for strategy in strategies {
            let serialized = bincode::serialize(&strategy).unwrap();
            let deserialized: MergeStrategy = bincode::deserialize(&serialized).unwrap();
            assert_eq!(strategy, deserialized);
        }
    }

    #[test]
    fn test_reality_checksum_serde_roundtrip() {
        let checksum = RealityChecksum(0xDEAD_BEEF_CAFE_BABE);
        let serialized = bincode::serialize(&checksum).unwrap();
        let deserialized: RealityChecksum = bincode::deserialize(&serialized).unwrap();
        assert_eq!(checksum, deserialized);
    }

    #[test]
    fn test_reality_checksum_display() {
        let checksum = RealityChecksum(0x0000_0000_0000_1234);
        assert_eq!(format!("{checksum}"), "0000000000001234");
    }

    #[test]
    fn test_chunk_conflict_serde_roundtrip() {
        let mut conflict = ChunkConflict::new(ChunkPos::new(5, 10, 15));
        conflict.add_block(LocalPos::new(0, 0, 0), STONE, BlockId(100));
        conflict.add_block(LocalPos::new(1, 1, 1), BlockId(50), BlockId(60));

        let serialized = bincode::serialize(&conflict).unwrap();
        let deserialized: ChunkConflict = bincode::deserialize(&serialized).unwrap();

        assert_eq!(conflict.pos, deserialized.pos);
        assert_eq!(conflict.conflict_count(), deserialized.conflict_count());
    }

    #[test]
    fn test_resolved_conflict_serde_roundtrip() {
        let resolved = ResolvedConflict {
            pos: ChunkPos::new(1, 2, 3),
            local: LocalPos::new(5, 5, 5),
            source_value: STONE,
            target_value: BlockId(100),
            resolved_value: STONE,
            resolution: ConflictResolution::SourceChosen,
        };

        let serialized = bincode::serialize(&resolved).unwrap();
        let deserialized: ResolvedConflict = bincode::deserialize(&serialized).unwrap();

        assert_eq!(resolved, deserialized);
    }

    #[test]
    fn test_fracture_registry_fingerprint_deterministic() {
        let build = || {
            let mut registry = FractureRegistry::new();
            registry.register(
                engine_core::coords::WorldPos::new(0, 0, 0),
                8,
                RealityId::ROOT,
                RealityId::new(1),
                0,
            );
            registry.fingerprint()
        };

        assert_eq!(build(), build());
    }

    #[test]
    fn test_registry_children_of() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        let _b = registry.fork(RealityId::ROOT, "b", 100).unwrap();
        let _c = registry.fork(a, "c", 200).unwrap();

        let root_children = registry.children_of(RealityId::ROOT).unwrap();
        assert_eq!(root_children.len(), 2);

        let a_children = registry.children_of(a).unwrap();
        assert_eq!(a_children.len(), 1);
    }

    #[test]
    fn test_prune_resets_active_to_root() {
        let mut registry = RealityRegistry::new();
        let a = registry.fork(RealityId::ROOT, "a", 0).unwrap();
        registry.set_active(a).unwrap();

        registry.prune(a).unwrap();
        assert_eq!(registry.active(), RealityId::ROOT);
    }
}

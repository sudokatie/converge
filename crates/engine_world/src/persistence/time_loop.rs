//! Time-loop authoring framework with persistent deltas and paradox guards.
//!
//! This module models loop definitions, per-iteration persistent chunk deltas,
//! reset/apply/commit planning, and deterministic paradox detection without
//! coupling to a concrete game.
//!
//! # Budget-Aware Recording
//!
//! Persistent changes are bounded by `max_persistent_changes` in [`LoopRules`].
//! Use [`TimeLoopRuntime::try_record_persistent_change`] for budget-checked
//! recording that returns errors before exceeding limits.
//!
//! # Protected Chunk Guards
//!
//! Both persistent and transient changes to protected chunks are detected.
//! Use [`TimeLoopRuntime::check_protected_violation`] to test individual
//! positions or [`ParadoxDetector::detect`] for full analysis.
//!
//! # Paradox Resolution
//!
//! When [`ParadoxGuardPolicy`] is `SourceWins` or `TargetWins`, use
//! [`TimeLoopRuntime::resolve_paradoxes`] to apply automatic conflict
//! resolution before commit.
//!
//! # Exit Rule Evaluation
//!
//! Use [`TimeLoopRuntime::evaluate_exit_rule`] to check whether the loop
//! should terminate based on iteration count, flags, or paradox thresholds.

use std::collections::{HashMap, HashSet};

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use crate::chunk::BlockId;

use super::chunk_delta::ChunkDelta;
use super::state_id::StateId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimeLoopId(pub u32);

impl TimeLoopId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimelineId(pub u32);

impl TimelineId {
    pub const PRIMARY: Self = Self(0);
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
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

impl Default for TimelineId {
    fn default() -> Self {
        Self::PRIMARY
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LoopIterationId(pub u32);

impl LoopIterationId {
    pub const FIRST: Self = Self(0);
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl Default for LoopIterationId {
    fn default() -> Self {
        Self::FIRST
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopResetMode {
    FullReset,
    KeepPersistentDeltas,
    MergeIntoPrimary,
    ForkTimeline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopExitRule {
    Manual,
    MaxIterations(u32),
    RequiredFlags(u32),
    ParadoxThreshold(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParadoxGuardPolicy {
    ReportOnly,
    BlockCommit,
    SourceWins,
    TargetWins,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopWindow {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl LoopWindow {
    #[must_use]
    pub const fn new(start_tick: u64, end_tick: u64) -> Self {
        Self {
            start_tick,
            end_tick,
        }
    }
    #[must_use]
    pub const fn duration(self) -> u64 {
        self.end_tick.saturating_sub(self.start_tick)
    }
    #[must_use]
    pub const fn contains(self, tick: u64) -> bool {
        tick >= self.start_tick && tick <= self.end_tick
    }
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopRules {
    pub reset_mode: LoopResetMode,
    pub exit_rule: LoopExitRule,
    pub paradox_policy: ParadoxGuardPolicy,
    pub max_persistent_changes: u32,
    pub protected_chunks: HashSet<ChunkPos>,
}

impl Default for LoopRules {
    fn default() -> Self {
        Self {
            reset_mode: LoopResetMode::KeepPersistentDeltas,
            exit_rule: LoopExitRule::Manual,
            paradox_policy: ParadoxGuardPolicy::BlockCommit,
            max_persistent_changes: 4096,
            protected_chunks: HashSet::new(),
        }
    }
}

impl LoopRules {
    #[must_use]
    pub fn is_chunk_protected(&self, chunk: ChunkPos) -> bool {
        self.protected_chunks.contains(&chunk)
    }

    #[must_use]
    pub fn within_budget(&self, current_count: usize) -> bool {
        current_count <= self.max_persistent_changes as usize
    }

    #[must_use]
    pub fn budget_remaining(&self, current_count: usize) -> u32 {
        (self.max_persistent_changes as usize)
            .saturating_sub(current_count)
            .try_into()
            .unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeLoopDefinition {
    pub id: TimeLoopId,
    pub name: String,
    pub source_timeline: TimelineId,
    pub loop_timeline: TimelineId,
    pub window: LoopWindow,
    pub rules: LoopRules,
}

impl TimeLoopDefinition {
    #[must_use]
    pub fn new(id: TimeLoopId, name: impl Into<String>, window: LoopWindow) -> Self {
        Self {
            id,
            name: name.into(),
            source_timeline: TimelineId::PRIMARY,
            loop_timeline: TimelineId::new(id.raw().saturating_add(1)),
            window,
            rules: LoopRules::default(),
        }
    }

    #[must_use]
    pub fn validate(&self) -> Vec<TimeLoopValidationError> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push(TimeLoopValidationError::EmptyName);
        }
        if !self.window.is_valid() {
            errors.push(TimeLoopValidationError::InvalidWindow);
        }
        if self.source_timeline == self.loop_timeline {
            errors.push(TimeLoopValidationError::TimelineAlias);
        }
        if self.rules.max_persistent_changes == 0 {
            errors.push(TimeLoopValidationError::ZeroPersistentChangeBudget);
        }
        errors
    }

    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut fp = TimeLoopFingerprint::new();
        fp.write_u32(self.id.raw());
        fp.write_str(&self.name);
        fp.write_u32(self.source_timeline.raw());
        fp.write_u32(self.loop_timeline.raw());
        fp.write_u64(self.window.start_tick);
        fp.write_u64(self.window.end_tick);
        fp.write_u8(match self.rules.reset_mode {
            LoopResetMode::FullReset => 0,
            LoopResetMode::KeepPersistentDeltas => 1,
            LoopResetMode::MergeIntoPrimary => 2,
            LoopResetMode::ForkTimeline => 3,
        });
        fp.write_u8(match self.rules.paradox_policy {
            ParadoxGuardPolicy::ReportOnly => 0,
            ParadoxGuardPolicy::BlockCommit => 1,
            ParadoxGuardPolicy::SourceWins => 2,
            ParadoxGuardPolicy::TargetWins => 3,
        });
        fp.write_u32(self.rules.max_persistent_changes);
        let mut sorted_chunks: Vec<_> = self.rules.protected_chunks.iter().copied().collect();
        sorted_chunks.sort_by_key(|c| (c.0.x, c.0.y, c.0.z));
        for chunk in sorted_chunks {
            fp.write_chunk(chunk);
        }
        fp.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeLoopValidationError {
    EmptyName,
    InvalidWindow,
    TimelineAlias,
    ZeroPersistentChangeBudget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeLoopError {
    BudgetExceeded { current: u32, max: u32 },
    ProtectedChunkViolation { chunk: ChunkPos },
    ParadoxBlocksCommit,
}

impl std::fmt::Display for TimeLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded { current, max } => {
                write!(f, "persistent change budget exceeded: {current}/{max}")
            }
            Self::ProtectedChunkViolation { chunk } => {
                write!(
                    f,
                    "protected chunk violation at ({}, {}, {})",
                    chunk.0.x, chunk.0.y, chunk.0.z
                )
            }
            Self::ParadoxBlocksCommit => write!(f, "paradox policy blocks commit"),
        }
    }
}

impl std::error::Error for TimeLoopError {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentLoopDeltas {
    chunks: HashMap<ChunkPos, ChunkDelta>,
}

impl PersistentLoopDeltas {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.chunks.values().map(ChunkDelta::len).sum()
    }
    pub fn set(&mut self, chunk: ChunkPos, local: LocalPos, block: BlockId) -> Option<BlockId> {
        self.chunks.entry(chunk).or_default().set(local, block)
    }
    #[must_use]
    pub fn get(&self, chunk: ChunkPos, local: LocalPos) -> Option<BlockId> {
        self.chunks.get(&chunk).and_then(|delta| delta.get(local))
    }
    pub fn remove(&mut self, chunk: ChunkPos, local: LocalPos) -> Option<BlockId> {
        let removed = self
            .chunks
            .get_mut(&chunk)
            .and_then(|delta| delta.remove(local));
        if self.chunks.get(&chunk).is_some_and(ChunkDelta::is_empty) {
            self.chunks.remove(&chunk);
        }
        removed
    }
    pub fn merge_chunk_delta(&mut self, chunk: ChunkPos, delta: &ChunkDelta) {
        self.chunks.entry(chunk).or_default().merge(delta);
        if self.chunks.get(&chunk).is_some_and(ChunkDelta::is_empty) {
            self.chunks.remove(&chunk);
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (ChunkPos, LocalPos, BlockId)> + '_ {
        let mut sorted_chunks: Vec<_> = self.chunks.keys().copied().collect();
        sorted_chunks.sort_by_key(|c| (c.0.x, c.0.y, c.0.z));
        sorted_chunks.into_iter().flat_map(move |chunk| {
            self.chunks.get(&chunk).into_iter().flat_map(move |delta| {
                delta
                    .iter()
                    .map(move |(local, block)| (chunk, local, block))
            })
        })
    }
    #[must_use]
    pub fn checksum(&self) -> u64 {
        let mut fp = TimeLoopFingerprint::new();
        for (chunk, local, block) in self.iter() {
            fp.write_chunk(chunk);
            fp.write_local(local);
            fp.write_u16(block.raw());
        }
        fp.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopLifecycleState {
    Authoring,
    Running,
    ResetPending,
    Committed,
    Abandoned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopIterationState {
    pub iteration: LoopIterationId,
    pub state: LoopLifecycleState,
    pub started_tick: u64,
    pub current_tick: u64,
    pub completed_tick: Option<u64>,
    pub paradox_count: u32,
}

impl LoopIterationState {
    #[must_use]
    pub fn start(iteration: LoopIterationId, tick: u64) -> Self {
        Self {
            iteration,
            state: LoopLifecycleState::Running,
            started_tick: tick,
            current_tick: tick,
            completed_tick: None,
            paradox_count: 0,
        }
    }
    pub fn advance_to(&mut self, tick: u64, window: LoopWindow) {
        self.current_tick = tick;
        if tick >= window.end_tick {
            self.state = LoopLifecycleState::ResetPending;
            self.completed_tick = Some(tick);
        }
    }
    #[must_use]
    pub fn start_next(&self, tick: u64) -> Self {
        Self::start(self.iteration.next(), tick)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeLoopRuntime {
    pub definition: TimeLoopDefinition,
    pub iteration: LoopIterationState,
    pub persistent_deltas: PersistentLoopDeltas,
    pub transient_deltas: PersistentLoopDeltas,
    #[serde(default)]
    pub completed_flags: u32,
}

impl TimeLoopRuntime {
    #[must_use]
    pub fn new(definition: TimeLoopDefinition) -> Self {
        let tick = definition.window.start_tick;
        Self {
            definition,
            iteration: LoopIterationState::start(LoopIterationId::FIRST, tick),
            persistent_deltas: PersistentLoopDeltas::new(),
            transient_deltas: PersistentLoopDeltas::new(),
            completed_flags: 0,
        }
    }

    pub fn record_transient_change(&mut self, chunk: ChunkPos, local: LocalPos, block: BlockId) {
        self.transient_deltas.set(chunk, local, block);
    }

    pub fn record_persistent_change(&mut self, chunk: ChunkPos, local: LocalPos, block: BlockId) {
        self.persistent_deltas.set(chunk, local, block);
    }

    /// Record a persistent change with budget and protection checks.
    ///
    /// # Errors
    ///
    /// Returns `BudgetExceeded` if the change would exceed `max_persistent_changes`.
    /// Returns `ProtectedChunkViolation` if the chunk is protected.
    pub fn try_record_persistent_change(
        &mut self,
        chunk: ChunkPos,
        local: LocalPos,
        block: BlockId,
    ) -> Result<Option<BlockId>, TimeLoopError> {
        if self.definition.rules.is_chunk_protected(chunk) {
            return Err(TimeLoopError::ProtectedChunkViolation { chunk });
        }
        let is_new = self.persistent_deltas.get(chunk, local).is_none();
        if is_new {
            let current = self.persistent_deltas.change_count();
            let max = self.definition.rules.max_persistent_changes as usize;
            if current >= max {
                #[expect(clippy::cast_possible_truncation, reason = "bounded by u32 max")]
                return Err(TimeLoopError::BudgetExceeded {
                    current: current as u32,
                    max: self.definition.rules.max_persistent_changes,
                });
            }
        }
        Ok(self.persistent_deltas.set(chunk, local, block))
    }

    /// Record a transient change with protection checks.
    ///
    /// # Errors
    ///
    /// Returns `ProtectedChunkViolation` if the chunk is protected.
    pub fn try_record_transient_change(
        &mut self,
        chunk: ChunkPos,
        local: LocalPos,
        block: BlockId,
    ) -> Result<Option<BlockId>, TimeLoopError> {
        if self.definition.rules.is_chunk_protected(chunk) {
            return Err(TimeLoopError::ProtectedChunkViolation { chunk });
        }
        Ok(self.transient_deltas.set(chunk, local, block))
    }

    /// Record a persistent change with budget and protection checks under `BlockCommit` policy.
    ///
    /// When the paradox policy is `BlockCommit`, this method refuses protected chunks
    /// and budget overflow by returning the corresponding `ParadoxConflict`.
    ///
    /// # Errors
    ///
    /// Returns `ParadoxConflict` with `ProtectedChunkChanged` if the chunk is protected
    /// and policy is `BlockCommit`.
    /// Returns `ParadoxConflict` with `ChangeBudgetExceeded` if the change would exceed
    /// `max_persistent_changes` and policy is `BlockCommit`.
    pub fn try_record_persistent_change_checked(
        &mut self,
        chunk: ChunkPos,
        local: LocalPos,
        block: BlockId,
    ) -> Result<Option<BlockId>, ParadoxConflict> {
        let policy = self.definition.rules.paradox_policy;
        if policy == ParadoxGuardPolicy::BlockCommit {
            if self.definition.rules.is_chunk_protected(chunk) {
                return Err(ParadoxConflict {
                    kind: ParadoxKind::ProtectedChunkChanged,
                    chunk: Some(chunk),
                    local: Some(local),
                    persistent: Some(block),
                    transient: None,
                });
            }
            let is_new = self.persistent_deltas.get(chunk, local).is_none();
            if is_new {
                let current = self.persistent_deltas.change_count();
                let max = self.definition.rules.max_persistent_changes as usize;
                if current >= max {
                    return Err(ParadoxConflict {
                        kind: ParadoxKind::ChangeBudgetExceeded,
                        chunk: Some(chunk),
                        local: Some(local),
                        persistent: Some(block),
                        transient: None,
                    });
                }
            }
        }
        Ok(self.persistent_deltas.set(chunk, local, block))
    }

    /// Set a flag bit in `completed_flags`.
    pub fn set_flag(&mut self, flag: u32) {
        self.completed_flags |= flag;
    }

    /// Clear a flag bit from `completed_flags`.
    pub fn clear_flag(&mut self, flag: u32) {
        self.completed_flags &= !flag;
    }

    /// Set all `completed_flags` to the given value.
    pub fn set_completed_flags(&mut self, flags: u32) {
        self.completed_flags = flags;
    }

    /// Clear all `completed_flags`.
    pub fn clear_all_flags(&mut self) {
        self.completed_flags = 0;
    }

    /// Evaluate whether the loop should exit based on the exit rule and internal state.
    ///
    /// Uses the runtime's [`completed_flags`](Self::completed_flags) for `RequiredFlags` evaluation.
    #[must_use]
    pub fn should_exit(&self) -> bool {
        self.evaluate_exit_rule(self.completed_flags).should_exit
    }

    /// Get the current exit status with full evaluation details.
    ///
    /// Uses the runtime's [`completed_flags`](Self::completed_flags) for `RequiredFlags` evaluation.
    #[must_use]
    pub fn exit_status(&self) -> ExitEvaluation {
        self.evaluate_exit_rule(self.completed_flags)
    }

    #[must_use]
    pub fn check_protected_violation(
        &self,
        chunk: ChunkPos,
        check_persistent: bool,
        check_transient: bool,
    ) -> Option<ProtectedViolation> {
        if !self.definition.rules.is_chunk_protected(chunk) {
            return None;
        }
        let persistent_count = if check_persistent {
            self.persistent_deltas
                .chunks
                .get(&chunk)
                .map_or(0, ChunkDelta::len)
        } else {
            0
        };
        let transient_count = if check_transient {
            self.transient_deltas
                .chunks
                .get(&chunk)
                .map_or(0, ChunkDelta::len)
        } else {
            0
        };
        if persistent_count > 0 || transient_count > 0 {
            #[expect(clippy::cast_possible_truncation, reason = "bounded by chunk volume")]
            Some(ProtectedViolation {
                chunk,
                persistent_changes: persistent_count as u32,
                transient_changes: transient_count as u32,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn budget_remaining(&self) -> u32 {
        self.definition
            .rules
            .budget_remaining(self.persistent_deltas.change_count())
    }

    #[must_use]
    pub fn is_within_budget(&self) -> bool {
        self.definition
            .rules
            .within_budget(self.persistent_deltas.change_count())
    }
    #[must_use]
    pub fn plan_reset(&self) -> LoopResetPlan {
        let mut actions = Vec::new();
        actions.push(LoopPlanAction::ClearTransientDeltas);
        match self.definition.rules.reset_mode {
            LoopResetMode::FullReset => actions.push(LoopPlanAction::ClearPersistentDeltas),
            LoopResetMode::KeepPersistentDeltas => {
                actions.push(LoopPlanAction::ApplyPersistentDeltas);
            }
            LoopResetMode::MergeIntoPrimary => {
                actions.push(LoopPlanAction::CommitPersistentDeltas {
                    target: self.definition.source_timeline,
                });
            }
            LoopResetMode::ForkTimeline => {
                actions.push(LoopPlanAction::ForkTimeline {
                    source: self.definition.loop_timeline,
                });
            }
        }
        LoopResetPlan {
            loop_id: self.definition.id,
            next_iteration: self.iteration.iteration.next(),
            actions,
            persistent_checksum: self.persistent_deltas.checksum(),
            transient_checksum: self.transient_deltas.checksum(),
        }
    }
    pub fn apply_reset(&mut self, tick: u64) -> LoopResetPlan {
        let plan = self.plan_reset();
        self.transient_deltas = PersistentLoopDeltas::new();
        if self.definition.rules.reset_mode == LoopResetMode::FullReset {
            self.persistent_deltas = PersistentLoopDeltas::new();
        }
        self.iteration = self.iteration.start_next(tick);
        plan
    }
    #[must_use]
    pub fn detect_paradoxes(&self) -> ParadoxReport {
        ParadoxDetector::detect(
            &self.definition,
            &self.persistent_deltas,
            &self.transient_deltas,
        )
    }
    #[must_use]
    pub fn summary(&self) -> TimeLoopSummary {
        let paradoxes = self.detect_paradoxes();
        TimeLoopSummary {
            loop_id: self.definition.id,
            iteration: self.iteration.iteration,
            state: self.iteration.state,
            persistent_chunks: u32::try_from(self.persistent_deltas.chunk_count())
                .unwrap_or(u32::MAX),
            persistent_changes: u32::try_from(self.persistent_deltas.change_count())
                .unwrap_or(u32::MAX),
            transient_changes: u32::try_from(self.transient_deltas.change_count())
                .unwrap_or(u32::MAX),
            paradox_count: u32::try_from(paradoxes.conflicts.len()).unwrap_or(u32::MAX),
            fingerprint: self.fingerprint(),
        }
    }
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut fp = TimeLoopFingerprint::new();
        fp.write_u64(self.definition.fingerprint());
        fp.write_u32(self.iteration.iteration.raw());
        fp.write_u64(self.iteration.current_tick);
        fp.write_u64(self.persistent_deltas.checksum());
        fp.write_u64(self.transient_deltas.checksum());
        fp.finish()
    }

    #[must_use]
    pub fn evaluate_exit_rule(&self, active_flags: u32) -> ExitEvaluation {
        let paradoxes = self.detect_paradoxes();
        let iteration = self.iteration.iteration.raw();
        let paradox_count = u32::try_from(paradoxes.conflicts.len()).unwrap_or(u32::MAX);

        let (should_exit, reason) = match self.definition.rules.exit_rule {
            LoopExitRule::Manual => (false, ExitReason::Manual),
            LoopExitRule::MaxIterations(max) => {
                if iteration >= max {
                    (true, ExitReason::MaxIterationsReached { iteration, max })
                } else {
                    (
                        false,
                        ExitReason::IterationsRemaining {
                            current: iteration,
                            max,
                        },
                    )
                }
            }
            LoopExitRule::RequiredFlags(required) => {
                let met = (active_flags & required) == required;
                if met {
                    (
                        true,
                        ExitReason::FlagsCompleted {
                            required,
                            active: active_flags,
                        },
                    )
                } else {
                    let missing = required & !active_flags;
                    (
                        false,
                        ExitReason::FlagsPending {
                            required,
                            active: active_flags,
                            missing,
                        },
                    )
                }
            }
            LoopExitRule::ParadoxThreshold(threshold) => {
                if paradox_count >= threshold {
                    (
                        true,
                        ExitReason::ParadoxThresholdReached {
                            count: paradox_count,
                            threshold,
                        },
                    )
                } else {
                    (
                        false,
                        ExitReason::ParadoxesBelow {
                            count: paradox_count,
                            threshold,
                        },
                    )
                }
            }
        };

        ExitEvaluation {
            should_exit,
            reason,
            iteration,
            paradox_count,
            fingerprint: self.fingerprint(),
        }
    }

    pub fn resolve_paradoxes(&mut self) -> ParadoxResolutionResult {
        let report = self.detect_paradoxes();
        if report.is_clean() {
            return ParadoxResolutionResult {
                policy: report.policy,
                resolved: Vec::new(),
                unresolved: Vec::new(),
                changes_applied: 0,
            };
        }

        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();
        let mut changes_applied = 0_u32;

        for conflict in report.conflicts {
            match conflict.kind {
                ParadoxKind::PersistentTransientConflict => {
                    if let (Some(chunk), Some(local)) = (conflict.chunk, conflict.local) {
                        match report.policy {
                            ParadoxGuardPolicy::SourceWins => {
                                self.transient_deltas.remove(chunk, local);
                                resolved.push(ResolvedParadox {
                                    conflict: conflict.clone(),
                                    resolution: ConflictResolutionAction::SourceWins,
                                });
                                changes_applied += 1;
                            }
                            ParadoxGuardPolicy::TargetWins => {
                                if let Some(transient_block) = conflict.transient {
                                    self.persistent_deltas.set(chunk, local, transient_block);
                                }
                                resolved.push(ResolvedParadox {
                                    conflict: conflict.clone(),
                                    resolution: ConflictResolutionAction::TargetWins,
                                });
                                changes_applied += 1;
                            }
                            ParadoxGuardPolicy::ReportOnly | ParadoxGuardPolicy::BlockCommit => {
                                unresolved.push(conflict);
                            }
                        }
                    } else {
                        unresolved.push(conflict);
                    }
                }
                ParadoxKind::ProtectedChunkChanged
                | ParadoxKind::ProtectedChunkTransient
                | ParadoxKind::ChangeBudgetExceeded => {
                    unresolved.push(conflict);
                }
            }
        }

        ParadoxResolutionResult {
            policy: report.policy,
            resolved,
            unresolved,
            changes_applied,
        }
    }

    #[must_use]
    pub fn plan_commit(&self) -> CommitPlan {
        let report = self.detect_paradoxes();
        let can_commit = !report.blocks_commit;
        let persistent_count =
            u32::try_from(self.persistent_deltas.change_count()).unwrap_or(u32::MAX);
        let transient_count =
            u32::try_from(self.transient_deltas.change_count()).unwrap_or(u32::MAX);

        let mut actions = Vec::new();
        if can_commit {
            actions.push(CommitAction::MergePersistentToSource {
                target: self.definition.source_timeline,
                change_count: persistent_count,
            });
            actions.push(CommitAction::DiscardTransient {
                change_count: transient_count,
            });
            actions.push(CommitAction::MarkCommitted);
        }

        CommitPlan {
            loop_id: self.definition.id,
            can_commit,
            blocking_paradoxes: if report.blocks_commit {
                report.conflicts.clone()
            } else {
                Vec::new()
            },
            actions,
            persistent_checksum: self.persistent_deltas.checksum(),
            fingerprint: self.fingerprint(),
        }
    }

    /// Apply the commit, marking the loop as committed and clearing transients.
    ///
    /// # Errors
    ///
    /// Returns `ParadoxBlocksCommit` if there are unresolved paradoxes blocking commit.
    pub fn apply_commit(&mut self) -> Result<CommitPlan, TimeLoopError> {
        let plan = self.plan_commit();
        if !plan.can_commit {
            return Err(TimeLoopError::ParadoxBlocksCommit);
        }
        self.iteration.state = LoopLifecycleState::Committed;
        self.transient_deltas = PersistentLoopDeltas::new();
        Ok(plan)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    Manual,
    MaxIterationsReached {
        iteration: u32,
        max: u32,
    },
    IterationsRemaining {
        current: u32,
        max: u32,
    },
    FlagsCompleted {
        required: u32,
        active: u32,
    },
    FlagsPending {
        required: u32,
        active: u32,
        missing: u32,
    },
    ParadoxThresholdReached {
        count: u32,
        threshold: u32,
    },
    ParadoxesBelow {
        count: u32,
        threshold: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitEvaluation {
    pub should_exit: bool,
    pub reason: ExitReason,
    pub iteration: u32,
    pub paradox_count: u32,
    pub fingerprint: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolutionAction {
    SourceWins,
    TargetWins,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedParadox {
    pub conflict: ParadoxConflict,
    pub resolution: ConflictResolutionAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParadoxResolutionResult {
    pub policy: ParadoxGuardPolicy,
    pub resolved: Vec<ResolvedParadox>,
    pub unresolved: Vec<ParadoxConflict>,
    pub changes_applied: u32,
}

impl ParadoxResolutionResult {
    #[must_use]
    pub fn is_fully_resolved(&self) -> bool {
        self.unresolved.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitAction {
    MergePersistentToSource {
        target: TimelineId,
        change_count: u32,
    },
    DiscardTransient {
        change_count: u32,
    },
    MarkCommitted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPlan {
    pub loop_id: TimeLoopId,
    pub can_commit: bool,
    pub blocking_paradoxes: Vec<ParadoxConflict>,
    pub actions: Vec<CommitAction>,
    pub persistent_checksum: u64,
    pub fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopResetPlan {
    pub loop_id: TimeLoopId,
    pub next_iteration: LoopIterationId,
    pub actions: Vec<LoopPlanAction>,
    pub persistent_checksum: u64,
    pub transient_checksum: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopPlanAction {
    ClearTransientDeltas,
    ClearPersistentDeltas,
    ApplyPersistentDeltas,
    CommitPersistentDeltas { target: TimelineId },
    ForkTimeline { source: TimelineId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParadoxKind {
    ProtectedChunkChanged,
    ProtectedChunkTransient,
    PersistentTransientConflict,
    ChangeBudgetExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedViolation {
    pub chunk: ChunkPos,
    pub persistent_changes: u32,
    pub transient_changes: u32,
}

impl ProtectedViolation {
    #[must_use]
    pub fn total_changes(self) -> u32 {
        self.persistent_changes
            .saturating_add(self.transient_changes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParadoxConflict {
    pub kind: ParadoxKind,
    pub chunk: Option<ChunkPos>,
    pub local: Option<LocalPos>,
    pub persistent: Option<BlockId>,
    pub transient: Option<BlockId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParadoxReport {
    pub policy: ParadoxGuardPolicy,
    pub conflicts: Vec<ParadoxConflict>,
    pub blocks_commit: bool,
    pub checksum: u64,
}

impl ParadoxReport {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Filter conflicts by kind.
    #[must_use]
    pub fn conflicts_by_kind(&self, kind: ParadoxKind) -> Vec<&ParadoxConflict> {
        self.conflicts.iter().filter(|c| c.kind == kind).collect()
    }

    /// Check if this report would block commit under `BlockCommit` policy.
    ///
    /// Returns true if there are any conflicts AND the policy is `BlockCommit`.
    #[must_use]
    pub fn would_block_commit(&self) -> bool {
        self.blocks_commit
    }
}

pub struct ParadoxDetector;

impl ParadoxDetector {
    #[must_use]
    pub fn detect(
        definition: &TimeLoopDefinition,
        persistent: &PersistentLoopDeltas,
        transient: &PersistentLoopDeltas,
    ) -> ParadoxReport {
        let mut conflicts = Vec::new();
        let mut sorted_protected: Vec<_> =
            definition.rules.protected_chunks.iter().copied().collect();
        sorted_protected.sort_by_key(|c| (c.0.x, c.0.y, c.0.z));
        for chunk in &sorted_protected {
            if let Some(delta) = persistent.chunks.get(chunk) {
                for (local, block) in delta.iter() {
                    conflicts.push(ParadoxConflict {
                        kind: ParadoxKind::ProtectedChunkChanged,
                        chunk: Some(*chunk),
                        local: Some(local),
                        persistent: Some(block),
                        transient: None,
                    });
                }
            }
        }
        for chunk in &sorted_protected {
            if let Some(delta) = transient.chunks.get(chunk) {
                for (local, block) in delta.iter() {
                    conflicts.push(ParadoxConflict {
                        kind: ParadoxKind::ProtectedChunkTransient,
                        chunk: Some(*chunk),
                        local: Some(local),
                        persistent: None,
                        transient: Some(block),
                    });
                }
            }
        }
        for (chunk, local, persistent_block) in persistent.iter() {
            if let Some(transient_block) = transient.get(chunk, local)
                && transient_block != persistent_block
            {
                conflicts.push(ParadoxConflict {
                    kind: ParadoxKind::PersistentTransientConflict,
                    chunk: Some(chunk),
                    local: Some(local),
                    persistent: Some(persistent_block),
                    transient: Some(transient_block),
                });
            }
        }
        if persistent.change_count() > definition.rules.max_persistent_changes as usize {
            conflicts.push(ParadoxConflict {
                kind: ParadoxKind::ChangeBudgetExceeded,
                chunk: None,
                local: None,
                persistent: None,
                transient: None,
            });
        }
        conflicts.sort_by_key(|conflict| {
            (
                conflict.chunk.map(|c| (c.0.x, c.0.y, c.0.z)),
                conflict.local.map(|l| l.to_index()),
                match conflict.kind {
                    ParadoxKind::ProtectedChunkChanged => 0_u8,
                    ParadoxKind::ProtectedChunkTransient => 1,
                    ParadoxKind::PersistentTransientConflict => 2,
                    ParadoxKind::ChangeBudgetExceeded => 3,
                },
            )
        });
        let mut fp = TimeLoopFingerprint::new();
        for conflict in &conflicts {
            fp.write_u8(match conflict.kind {
                ParadoxKind::ProtectedChunkChanged => 0,
                ParadoxKind::ProtectedChunkTransient => 1,
                ParadoxKind::PersistentTransientConflict => 2,
                ParadoxKind::ChangeBudgetExceeded => 3,
            });
            if let Some(chunk) = conflict.chunk {
                fp.write_chunk(chunk);
            }
            if let Some(local) = conflict.local {
                fp.write_local(local);
            }
            if let Some(block) = conflict.persistent {
                fp.write_u16(block.raw());
            }
            if let Some(block) = conflict.transient {
                fp.write_u16(block.raw());
            }
        }
        let blocks_commit = !conflicts.is_empty()
            && definition.rules.paradox_policy == ParadoxGuardPolicy::BlockCommit;
        ParadoxReport {
            policy: definition.rules.paradox_policy,
            conflicts,
            blocks_commit,
            checksum: fp.finish(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeLoopSummary {
    pub loop_id: TimeLoopId,
    pub iteration: LoopIterationId,
    pub state: LoopLifecycleState,
    pub persistent_chunks: u32,
    pub persistent_changes: u32,
    pub transient_changes: u32,
    pub paradox_count: u32,
    pub fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeLoopSnapshot {
    pub definition: TimeLoopDefinition,
    pub iteration: LoopIterationState,
    pub persistent_checksum: u64,
    pub transient_checksum: u64,
    pub summary: TimeLoopSummary,
}

impl From<&TimeLoopRuntime> for TimeLoopSnapshot {
    fn from(runtime: &TimeLoopRuntime) -> Self {
        Self {
            definition: runtime.definition.clone(),
            iteration: runtime.iteration.clone(),
            persistent_checksum: runtime.persistent_deltas.checksum(),
            transient_checksum: runtime.transient_deltas.checksum(),
            summary: runtime.summary(),
        }
    }
}

#[derive(Default)]
pub struct TimeLoopFingerprint {
    state: u64,
}

impl TimeLoopFingerprint {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: 0xcbf2_9ce4_8422_2325,
        }
    }
    fn write_byte(&mut self, byte: u8) {
        self.state ^= u64::from(byte);
        self.state = self.state.wrapping_mul(0x0000_0100_0000_01b3);
    }
    pub fn write_u8(&mut self, value: u8) {
        self.write_byte(value);
    }
    pub fn write_u16(&mut self, value: u16) {
        for byte in value.to_le_bytes() {
            self.write_byte(byte);
        }
    }
    pub fn write_u32(&mut self, value: u32) {
        for byte in value.to_le_bytes() {
            self.write_byte(byte);
        }
    }
    pub fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.write_byte(byte);
        }
    }
    pub fn write_str(&mut self, value: &str) {
        self.write_u64(value.len() as u64);
        for byte in value.as_bytes() {
            self.write_byte(*byte);
        }
    }
    pub fn write_chunk(&mut self, chunk: ChunkPos) {
        self.write_i32(chunk.0.x);
        self.write_i32(chunk.0.y);
        self.write_i32(chunk.0.z);
    }
    pub fn write_local(&mut self, local: LocalPos) {
        self.write_u32(u32::try_from(local.to_index()).unwrap_or(u32::MAX));
    }
    fn write_i32(&mut self, value: i32) {
        for byte in value.to_le_bytes() {
            self.write_byte(byte);
        }
    }
    #[must_use]
    pub const fn finish(self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{DIRT, GRASS, STONE, WATER};
    use glam::{IVec3, UVec3};

    fn chunk(x: i32, y: i32, z: i32) -> ChunkPos {
        ChunkPos(IVec3::new(x, y, z))
    }
    fn local(x: u32, y: u32, z: u32) -> LocalPos {
        LocalPos(UVec3::new(x, y, z))
    }

    #[test]
    fn time_loop_definition_validation_catches_bad_rules() {
        let mut def = TimeLoopDefinition::new(TimeLoopId::new(7), "", LoopWindow::new(20, 10));
        def.loop_timeline = def.source_timeline;
        def.rules.max_persistent_changes = 0;
        let errors = def.validate();
        assert!(errors.contains(&TimeLoopValidationError::EmptyName));
        assert!(errors.contains(&TimeLoopValidationError::InvalidWindow));
        assert!(errors.contains(&TimeLoopValidationError::TimelineAlias));
        assert!(errors.contains(&TimeLoopValidationError::ZeroPersistentChangeBudget));
    }

    #[test]
    fn time_loop_persistent_deltas_are_deterministic() {
        let mut a = PersistentLoopDeltas::new();
        a.set(chunk(1, 0, 0), local(1, 2, 3), STONE);
        a.set(chunk(0, 0, 0), local(2, 0, 0), DIRT);
        let mut b = PersistentLoopDeltas::new();
        b.set(chunk(0, 0, 0), local(2, 0, 0), DIRT);
        b.set(chunk(1, 0, 0), local(1, 2, 3), STONE);
        assert_eq!(a.change_count(), 2);
        assert_eq!(a.checksum(), b.checksum());
        assert_eq!(a.get(chunk(1, 0, 0), local(1, 2, 3)), Some(STONE));
    }

    #[test]
    fn time_loop_reset_keeps_or_clears_persistent_deltas() {
        let mut runtime = TimeLoopRuntime::new(TimeLoopDefinition::new(
            TimeLoopId::new(1),
            "loop",
            LoopWindow::new(10, 20),
        ));
        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        runtime.record_transient_change(chunk(0, 0, 0), local(0, 0, 1), DIRT);
        let plan = runtime.apply_reset(20);
        assert!(
            plan.actions
                .contains(&LoopPlanAction::ApplyPersistentDeltas)
        );
        assert_eq!(runtime.iteration.iteration, LoopIterationId::new(1));
        assert_eq!(runtime.persistent_deltas.change_count(), 1);
        assert!(runtime.transient_deltas.is_empty());

        runtime.definition.rules.reset_mode = LoopResetMode::FullReset;
        runtime.apply_reset(30);
        assert!(runtime.persistent_deltas.is_empty());
    }

    #[test]
    fn time_loop_paradox_guards_find_protected_and_conflicting_changes() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(2), "paradox", LoopWindow::new(0, 100));
        def.rules.protected_chunks.insert(chunk(9, 0, 0));
        def.rules.max_persistent_changes = 1;
        let mut runtime = TimeLoopRuntime::new(def);
        runtime.record_persistent_change(chunk(9, 0, 0), local(0, 0, 0), STONE);
        runtime.record_persistent_change(chunk(1, 0, 0), local(1, 0, 0), DIRT);
        runtime.record_transient_change(chunk(1, 0, 0), local(1, 0, 0), WATER);
        let report = runtime.detect_paradoxes();
        assert!(report.blocks_commit);
        assert_eq!(report.conflicts.len(), 3);
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ParadoxKind::ProtectedChunkChanged)
        );
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ParadoxKind::PersistentTransientConflict)
        );
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ParadoxKind::ChangeBudgetExceeded)
        );
    }

    #[test]
    fn time_loop_summary_and_snapshot_are_stable() {
        let mut runtime = TimeLoopRuntime::new(TimeLoopDefinition::new(
            TimeLoopId::new(3),
            "stable",
            LoopWindow::new(5, 15),
        ));
        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 1, 0), GRASS);
        let summary_a = runtime.summary();
        let snapshot: TimeLoopSnapshot = (&runtime).into();
        let summary_b = runtime.summary();
        assert_eq!(summary_a, summary_b);
        assert_eq!(snapshot.summary.fingerprint, summary_a.fingerprint);
    }

    #[test]
    fn time_loop_serde_json_and_bincode_round_trip() {
        let mut runtime = TimeLoopRuntime::new(TimeLoopDefinition::new(
            TimeLoopId::new(4),
            "serde",
            LoopWindow::new(1, 8),
        ));
        runtime.record_persistent_change(chunk(-1, 0, 2), local(3, 3, 3), STONE);
        runtime.record_transient_change(chunk(-1, 0, 2), local(4, 3, 3), WATER);

        // JSON: round-trip TimeLoopSnapshot (no HashMap keys that require string serialization)
        let snapshot: TimeLoopSnapshot = (&runtime).into();
        let json = serde_json::to_string(&snapshot).expect("serialize json");
        let from_json: TimeLoopSnapshot = serde_json::from_str(&json).expect("deserialize json");
        assert_eq!(snapshot, from_json);

        // Bincode: round-trip full TimeLoopRuntime including deltas
        let bin = bincode::serialize(&runtime).expect("serialize bincode");
        let from_bin: TimeLoopRuntime = bincode::deserialize(&bin).expect("deserialize bincode");
        assert_eq!(runtime, from_bin);
    }

    #[test]
    fn time_loop_try_record_respects_budget() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(10), "budget", LoopWindow::new(0, 50));
        def.rules.max_persistent_changes = 2;
        let mut runtime = TimeLoopRuntime::new(def);

        assert_eq!(runtime.budget_remaining(), 2);
        assert!(runtime.is_within_budget());

        let r1 = runtime.try_record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        assert!(r1.is_ok());
        assert_eq!(runtime.budget_remaining(), 1);

        let r2 = runtime.try_record_persistent_change(chunk(0, 0, 0), local(1, 0, 0), DIRT);
        assert!(r2.is_ok());
        assert_eq!(runtime.budget_remaining(), 0);

        let r3 = runtime.try_record_persistent_change(chunk(0, 0, 0), local(2, 0, 0), WATER);
        assert!(matches!(
            r3,
            Err(TimeLoopError::BudgetExceeded { current: 2, max: 2 })
        ));

        let update = runtime.try_record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), GRASS);
        assert!(update.is_ok());
        assert_eq!(update.unwrap(), Some(STONE));
    }

    #[test]
    fn time_loop_try_record_rejects_protected_chunks() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(11), "protected", LoopWindow::new(0, 50));
        def.rules.protected_chunks.insert(chunk(5, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        let r1 = runtime.try_record_persistent_change(chunk(5, 0, 0), local(0, 0, 0), STONE);
        assert!(matches!(
            r1,
            Err(TimeLoopError::ProtectedChunkViolation { chunk: c }) if c == chunk(5, 0, 0)
        ));

        let r2 = runtime.try_record_transient_change(chunk(5, 0, 0), local(0, 0, 0), STONE);
        assert!(matches!(
            r2,
            Err(TimeLoopError::ProtectedChunkViolation { chunk: c }) if c == chunk(5, 0, 0)
        ));

        let r3 = runtime.try_record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        assert!(r3.is_ok());
    }

    #[test]
    fn time_loop_check_protected_violation_returns_counts() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(12), "violation", LoopWindow::new(0, 50));
        def.rules.protected_chunks.insert(chunk(7, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(7, 0, 0), local(0, 0, 0), STONE);
        runtime.record_persistent_change(chunk(7, 0, 0), local(1, 0, 0), DIRT);
        runtime.record_transient_change(chunk(7, 0, 0), local(2, 0, 0), WATER);

        let violation = runtime.check_protected_violation(chunk(7, 0, 0), true, true);
        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.persistent_changes, 2);
        assert_eq!(v.transient_changes, 1);
        assert_eq!(v.total_changes(), 3);

        let no_violation = runtime.check_protected_violation(chunk(0, 0, 0), true, true);
        assert!(no_violation.is_none());
    }

    #[test]
    fn time_loop_detects_transient_protected_violations() {
        let mut def = TimeLoopDefinition::new(
            TimeLoopId::new(13),
            "transient_protected",
            LoopWindow::new(0, 100),
        );
        def.rules.protected_chunks.insert(chunk(3, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_transient_change(chunk(3, 0, 0), local(0, 0, 0), STONE);
        runtime.record_transient_change(chunk(3, 0, 0), local(1, 0, 0), DIRT);

        let report = runtime.detect_paradoxes();
        assert!(report.blocks_commit);
        assert_eq!(
            report
                .conflicts
                .iter()
                .filter(|c| c.kind == ParadoxKind::ProtectedChunkTransient)
                .count(),
            2
        );
    }

    #[test]
    fn time_loop_exit_rule_max_iterations() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(14), "max_iter", LoopWindow::new(0, 10));
        def.rules.exit_rule = LoopExitRule::MaxIterations(3);
        let mut runtime = TimeLoopRuntime::new(def);

        let eval0 = runtime.evaluate_exit_rule(0);
        assert!(!eval0.should_exit);
        assert!(matches!(
            eval0.reason,
            ExitReason::IterationsRemaining { current: 0, max: 3 }
        ));

        runtime.apply_reset(10);
        runtime.apply_reset(20);
        runtime.apply_reset(30);

        let eval3 = runtime.evaluate_exit_rule(0);
        assert!(eval3.should_exit);
        assert!(matches!(
            eval3.reason,
            ExitReason::MaxIterationsReached {
                iteration: 3,
                max: 3
            }
        ));
    }

    #[test]
    fn time_loop_exit_rule_required_flags() {
        let mut def = TimeLoopDefinition::new(TimeLoopId::new(15), "flags", LoopWindow::new(0, 10));
        def.rules.exit_rule = LoopExitRule::RequiredFlags(0b0101);
        let runtime = TimeLoopRuntime::new(def);

        let eval_partial = runtime.evaluate_exit_rule(0b0001);
        assert!(!eval_partial.should_exit);
        assert!(matches!(
            eval_partial.reason,
            ExitReason::FlagsPending {
                required: 5,
                active: 1,
                missing: 4
            }
        ));

        let eval_complete = runtime.evaluate_exit_rule(0b0111);
        assert!(eval_complete.should_exit);
        assert!(matches!(
            eval_complete.reason,
            ExitReason::FlagsCompleted {
                required: 5,
                active: 7
            }
        ));
    }

    #[test]
    fn time_loop_exit_rule_paradox_threshold() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(16), "threshold", LoopWindow::new(0, 100));
        def.rules.exit_rule = LoopExitRule::ParadoxThreshold(2);
        def.rules.paradox_policy = ParadoxGuardPolicy::ReportOnly;
        def.rules.protected_chunks.insert(chunk(0, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        let eval0 = runtime.evaluate_exit_rule(0);
        assert!(!eval0.should_exit);

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        runtime.record_persistent_change(chunk(0, 0, 0), local(1, 0, 0), DIRT);

        let eval2 = runtime.evaluate_exit_rule(0);
        assert!(eval2.should_exit);
        assert!(matches!(
            eval2.reason,
            ExitReason::ParadoxThresholdReached {
                count: 2,
                threshold: 2
            }
        ));
    }

    #[test]
    fn time_loop_resolve_paradoxes_source_wins() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(17), "source_wins", LoopWindow::new(0, 100));
        def.rules.paradox_policy = ParadoxGuardPolicy::SourceWins;
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        runtime.record_transient_change(chunk(0, 0, 0), local(0, 0, 0), WATER);

        let result = runtime.resolve_paradoxes();
        assert!(result.is_fully_resolved());
        assert_eq!(result.resolved.len(), 1);
        assert_eq!(result.changes_applied, 1);
        assert!(matches!(
            result.resolved[0].resolution,
            ConflictResolutionAction::SourceWins
        ));

        assert!(
            runtime
                .transient_deltas
                .get(chunk(0, 0, 0), local(0, 0, 0))
                .is_none()
        );
        assert_eq!(
            runtime
                .persistent_deltas
                .get(chunk(0, 0, 0), local(0, 0, 0)),
            Some(STONE)
        );
    }

    #[test]
    fn time_loop_resolve_paradoxes_target_wins() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(18), "target_wins", LoopWindow::new(0, 100));
        def.rules.paradox_policy = ParadoxGuardPolicy::TargetWins;
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        runtime.record_transient_change(chunk(0, 0, 0), local(0, 0, 0), WATER);

        let result = runtime.resolve_paradoxes();
        assert!(result.is_fully_resolved());
        assert_eq!(result.resolved.len(), 1);
        assert_eq!(
            runtime
                .persistent_deltas
                .get(chunk(0, 0, 0), local(0, 0, 0)),
            Some(WATER)
        );
    }

    #[test]
    fn time_loop_resolve_paradoxes_leaves_unresolvable() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(19), "unresolvable", LoopWindow::new(0, 100));
        def.rules.paradox_policy = ParadoxGuardPolicy::SourceWins;
        def.rules.protected_chunks.insert(chunk(5, 0, 0));
        def.rules.max_persistent_changes = 1;
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(5, 0, 0), local(0, 0, 0), STONE);
        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), DIRT);

        let result = runtime.resolve_paradoxes();
        assert!(!result.is_fully_resolved());
        assert_eq!(result.unresolved.len(), 2);
    }

    #[test]
    fn time_loop_commit_plan_blocks_on_paradoxes() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(20), "commit_block", LoopWindow::new(0, 100));
        def.rules.protected_chunks.insert(chunk(0, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);

        let plan = runtime.plan_commit();
        assert!(!plan.can_commit);
        assert!(!plan.blocking_paradoxes.is_empty());
        assert!(plan.actions.is_empty());

        let commit_result = runtime.apply_commit();
        assert!(matches!(
            commit_result,
            Err(TimeLoopError::ParadoxBlocksCommit)
        ));
    }

    #[test]
    fn time_loop_commit_plan_succeeds_when_clean() {
        let def =
            TimeLoopDefinition::new(TimeLoopId::new(21), "commit_clean", LoopWindow::new(0, 100));
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        runtime.record_transient_change(chunk(1, 0, 0), local(0, 0, 0), WATER);

        let plan = runtime.plan_commit();
        assert!(plan.can_commit);
        assert!(plan.blocking_paradoxes.is_empty());
        assert!(!plan.actions.is_empty());

        let commit_result = runtime.apply_commit();
        assert!(commit_result.is_ok());
        assert_eq!(runtime.iteration.state, LoopLifecycleState::Committed);
        assert!(runtime.transient_deltas.is_empty());
    }

    #[test]
    fn time_loop_new_types_serde_roundtrip() {
        let exit_eval = ExitEvaluation {
            should_exit: true,
            reason: ExitReason::MaxIterationsReached {
                iteration: 5,
                max: 5,
            },
            iteration: 5,
            paradox_count: 0,
            fingerprint: 12345,
        };
        let json = serde_json::to_string(&exit_eval).expect("serialize");
        let parsed: ExitEvaluation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(exit_eval, parsed);

        let commit_plan = CommitPlan {
            loop_id: TimeLoopId::new(1),
            can_commit: true,
            blocking_paradoxes: Vec::new(),
            actions: vec![
                CommitAction::MergePersistentToSource {
                    target: TimelineId::PRIMARY,
                    change_count: 10,
                },
                CommitAction::DiscardTransient { change_count: 5 },
                CommitAction::MarkCommitted,
            ],
            persistent_checksum: 999,
            fingerprint: 888,
        };
        let bin = bincode::serialize(&commit_plan).expect("serialize");
        let parsed: CommitPlan = bincode::deserialize(&bin).expect("deserialize");
        assert_eq!(commit_plan, parsed);

        let resolution_result = ParadoxResolutionResult {
            policy: ParadoxGuardPolicy::SourceWins,
            resolved: vec![ResolvedParadox {
                conflict: ParadoxConflict {
                    kind: ParadoxKind::PersistentTransientConflict,
                    chunk: Some(ChunkPos(glam::IVec3::new(1, 2, 3))),
                    local: Some(LocalPos(glam::UVec3::new(4, 5, 6))),
                    persistent: Some(STONE),
                    transient: Some(WATER),
                },
                resolution: ConflictResolutionAction::SourceWins,
            }],
            unresolved: Vec::new(),
            changes_applied: 1,
        };
        let json = serde_json::to_string(&resolution_result).expect("serialize");
        let parsed: ParadoxResolutionResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(resolution_result, parsed);
    }

    #[test]
    fn time_loop_fingerprints_are_deterministic() {
        let def1 = TimeLoopDefinition::new(TimeLoopId::new(30), "fp_test", LoopWindow::new(0, 50));
        let def2 = TimeLoopDefinition::new(TimeLoopId::new(30), "fp_test", LoopWindow::new(0, 50));
        assert_eq!(def1.fingerprint(), def2.fingerprint());

        let mut runtime1 = TimeLoopRuntime::new(def1);
        let mut runtime2 = TimeLoopRuntime::new(def2);

        runtime1.record_persistent_change(chunk(0, 0, 0), local(1, 2, 3), STONE);
        runtime1.record_persistent_change(chunk(1, 0, 0), local(0, 0, 0), DIRT);
        runtime2.record_persistent_change(chunk(1, 0, 0), local(0, 0, 0), DIRT);
        runtime2.record_persistent_change(chunk(0, 0, 0), local(1, 2, 3), STONE);

        assert_eq!(runtime1.fingerprint(), runtime2.fingerprint());
        assert_eq!(
            runtime1.persistent_deltas.checksum(),
            runtime2.persistent_deltas.checksum()
        );
    }

    #[test]
    fn time_loop_try_record_checked_rejects_under_block_commit() {
        let mut def = TimeLoopDefinition::new(
            TimeLoopId::new(40),
            "checked_block_commit",
            LoopWindow::new(0, 50),
        );
        def.rules.paradox_policy = ParadoxGuardPolicy::BlockCommit;
        def.rules.protected_chunks.insert(chunk(5, 0, 0));
        def.rules.max_persistent_changes = 2;
        let mut runtime = TimeLoopRuntime::new(def);

        let r1 =
            runtime.try_record_persistent_change_checked(chunk(5, 0, 0), local(0, 0, 0), STONE);
        assert!(r1.is_err());
        let conflict = r1.unwrap_err();
        assert_eq!(conflict.kind, ParadoxKind::ProtectedChunkChanged);
        assert_eq!(conflict.chunk, Some(chunk(5, 0, 0)));

        let r2 =
            runtime.try_record_persistent_change_checked(chunk(0, 0, 0), local(0, 0, 0), STONE);
        assert!(r2.is_ok());
        let r3 = runtime.try_record_persistent_change_checked(chunk(0, 0, 0), local(1, 0, 0), DIRT);
        assert!(r3.is_ok());

        let r4 =
            runtime.try_record_persistent_change_checked(chunk(0, 0, 0), local(2, 0, 0), WATER);
        assert!(r4.is_err());
        let conflict = r4.unwrap_err();
        assert_eq!(conflict.kind, ParadoxKind::ChangeBudgetExceeded);
    }

    #[test]
    fn time_loop_try_record_checked_allows_under_report_only() {
        let mut def = TimeLoopDefinition::new(
            TimeLoopId::new(41),
            "checked_report_only",
            LoopWindow::new(0, 50),
        );
        def.rules.paradox_policy = ParadoxGuardPolicy::ReportOnly;
        def.rules.protected_chunks.insert(chunk(5, 0, 0));
        def.rules.max_persistent_changes = 1;
        let mut runtime = TimeLoopRuntime::new(def);

        let r1 =
            runtime.try_record_persistent_change_checked(chunk(5, 0, 0), local(0, 0, 0), STONE);
        assert!(r1.is_ok());

        let r2 = runtime.try_record_persistent_change_checked(chunk(0, 0, 0), local(0, 0, 0), DIRT);
        assert!(r2.is_ok());

        let r3 =
            runtime.try_record_persistent_change_checked(chunk(0, 0, 0), local(1, 0, 0), WATER);
        assert!(r3.is_ok());
    }

    #[test]
    fn time_loop_completed_flags_setters_and_clearers() {
        let def =
            TimeLoopDefinition::new(TimeLoopId::new(42), "flags_test", LoopWindow::new(0, 50));
        let mut runtime = TimeLoopRuntime::new(def);

        assert_eq!(runtime.completed_flags, 0);

        runtime.set_flag(0b0001);
        assert_eq!(runtime.completed_flags, 0b0001);

        runtime.set_flag(0b0100);
        assert_eq!(runtime.completed_flags, 0b0101);

        runtime.clear_flag(0b0001);
        assert_eq!(runtime.completed_flags, 0b0100);

        runtime.set_completed_flags(0b1111);
        assert_eq!(runtime.completed_flags, 0b1111);

        runtime.clear_all_flags();
        assert_eq!(runtime.completed_flags, 0);
    }

    #[test]
    fn time_loop_should_exit_uses_completed_flags() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(43), "should_exit", LoopWindow::new(0, 50));
        def.rules.exit_rule = LoopExitRule::RequiredFlags(0b0101);
        let mut runtime = TimeLoopRuntime::new(def);

        assert!(!runtime.should_exit());
        let status = runtime.exit_status();
        assert!(!status.should_exit);
        assert!(matches!(
            status.reason,
            ExitReason::FlagsPending {
                required: 5,
                active: 0,
                missing: 5
            }
        ));

        runtime.set_flag(0b0001);
        assert!(!runtime.should_exit());

        runtime.set_flag(0b0100);
        assert!(runtime.should_exit());
        let status = runtime.exit_status();
        assert!(status.should_exit);
        assert!(matches!(
            status.reason,
            ExitReason::FlagsCompleted {
                required: 5,
                active: 5
            }
        ));
    }

    #[test]
    fn time_loop_should_exit_manual_never_exits() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(44), "manual_exit", LoopWindow::new(0, 50));
        def.rules.exit_rule = LoopExitRule::Manual;
        let runtime = TimeLoopRuntime::new(def);

        assert!(!runtime.should_exit());
        assert!(matches!(runtime.exit_status().reason, ExitReason::Manual));
    }

    #[test]
    fn time_loop_paradox_report_conflicts_by_kind() {
        let mut def = TimeLoopDefinition::new(
            TimeLoopId::new(45),
            "conflicts_by_kind",
            LoopWindow::new(0, 100),
        );
        def.rules.protected_chunks.insert(chunk(9, 0, 0));
        def.rules.max_persistent_changes = 1;
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(9, 0, 0), local(0, 0, 0), STONE);
        runtime.record_persistent_change(chunk(1, 0, 0), local(1, 0, 0), DIRT);
        runtime.record_transient_change(chunk(1, 0, 0), local(1, 0, 0), WATER);

        let report = runtime.detect_paradoxes();
        assert_eq!(report.conflicts.len(), 3);

        let protected = report.conflicts_by_kind(ParadoxKind::ProtectedChunkChanged);
        assert_eq!(protected.len(), 1);
        assert_eq!(protected[0].chunk, Some(chunk(9, 0, 0)));

        let budget = report.conflicts_by_kind(ParadoxKind::ChangeBudgetExceeded);
        assert_eq!(budget.len(), 1);

        let conflicts = report.conflicts_by_kind(ParadoxKind::PersistentTransientConflict);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].chunk, Some(chunk(1, 0, 0)));

        let transient = report.conflicts_by_kind(ParadoxKind::ProtectedChunkTransient);
        assert!(transient.is_empty());
    }

    #[test]
    fn time_loop_paradox_report_would_block_commit() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(46), "would_block", LoopWindow::new(0, 100));
        def.rules.paradox_policy = ParadoxGuardPolicy::BlockCommit;
        def.rules.protected_chunks.insert(chunk(0, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        let clean_report = runtime.detect_paradoxes();
        assert!(!clean_report.would_block_commit());

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        let blocking_report = runtime.detect_paradoxes();
        assert!(blocking_report.would_block_commit());
    }

    #[test]
    fn time_loop_paradox_report_would_block_commit_report_only() {
        let mut def = TimeLoopDefinition::new(
            TimeLoopId::new(47),
            "report_only_block",
            LoopWindow::new(0, 100),
        );
        def.rules.paradox_policy = ParadoxGuardPolicy::ReportOnly;
        def.rules.protected_chunks.insert(chunk(0, 0, 0));
        let mut runtime = TimeLoopRuntime::new(def);

        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);
        let report = runtime.detect_paradoxes();
        assert!(!report.would_block_commit());
        assert!(!report.is_clean());
    }

    #[test]
    fn time_loop_completed_flags_serde_roundtrip() {
        let mut def =
            TimeLoopDefinition::new(TimeLoopId::new(48), "flags_serde", LoopWindow::new(0, 50));
        def.rules.exit_rule = LoopExitRule::RequiredFlags(0b1111);
        let mut runtime = TimeLoopRuntime::new(def);
        runtime.set_completed_flags(0b1010);
        runtime.record_persistent_change(chunk(0, 0, 0), local(0, 0, 0), STONE);

        let bin = bincode::serialize(&runtime).expect("serialize bincode");
        let from_bin: TimeLoopRuntime = bincode::deserialize(&bin).expect("deserialize bincode");
        assert_eq!(runtime.completed_flags, from_bin.completed_flags);
        assert_eq!(runtime, from_bin);

        let snapshot: TimeLoopSnapshot = (&runtime).into();
        let json = serde_json::to_string(&snapshot).expect("serialize json");
        let from_json: TimeLoopSnapshot = serde_json::from_str(&json).expect("deserialize json");
        assert_eq!(snapshot, from_json);
    }
}

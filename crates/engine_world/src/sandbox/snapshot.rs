//! Snapshot types for sandbox state inspection and persistence.

use std::collections::HashMap;

use bitflags::bitflags;
use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use crate::environment::{HazardKind, HazardSnapshot};
use crate::replay::StepChecksum;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct ChunkDataFlags: u8 {
        const SCALAR_FIELDS = 0b0001;
        const VECTOR_FIELDS = 0b0010;
        const FLUIDS        = 0b0100;
        const STRUCTURAL    = 0b1000;
    }
}

/// Summary of a single chunk's state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChunkSummary {
    /// Total active hazard cells.
    pub active_hazards: u32,
    /// Active hazards per kind.
    pub hazards_by_kind: [u32; HazardKind::COUNT],
    /// Flags for allocated data types.
    pub data_flags: ChunkDataFlags,
}

impl ChunkSummary {
    /// Check if the chunk has any allocated data.
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.active_hazards > 0 || !self.data_flags.is_empty()
    }

    /// Whether scalar fields are allocated.
    #[must_use]
    pub fn has_scalar_fields(&self) -> bool {
        self.data_flags.contains(ChunkDataFlags::SCALAR_FIELDS)
    }

    /// Whether vector fields are allocated.
    #[must_use]
    pub fn has_vector_fields(&self) -> bool {
        self.data_flags.contains(ChunkDataFlags::VECTOR_FIELDS)
    }

    /// Whether fluid layers are allocated.
    #[must_use]
    pub fn has_fluids(&self) -> bool {
        self.data_flags.contains(ChunkDataFlags::FLUIDS)
    }

    /// Whether structural data is allocated.
    #[must_use]
    pub fn has_structural(&self) -> bool {
        self.data_flags.contains(ChunkDataFlags::STRUCTURAL)
    }

    /// Set scalar fields flag.
    pub fn set_scalar_fields(&mut self, value: bool) {
        self.data_flags.set(ChunkDataFlags::SCALAR_FIELDS, value);
    }

    /// Set vector fields flag.
    pub fn set_vector_fields(&mut self, value: bool) {
        self.data_flags.set(ChunkDataFlags::VECTOR_FIELDS, value);
    }

    /// Set fluids flag.
    pub fn set_fluids(&mut self, value: bool) {
        self.data_flags.set(ChunkDataFlags::FLUIDS, value);
    }

    /// Set structural flag.
    pub fn set_structural(&mut self, value: bool) {
        self.data_flags.set(ChunkDataFlags::STRUCTURAL, value);
    }
}

/// Overall sandbox state for inspection.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SandboxState {
    /// Current simulation tick.
    pub tick: u64,
    /// Total loaded chunks.
    pub chunk_count: usize,
    /// Total active hazard cells across all chunks.
    pub total_active_hazards: u32,
    /// Active hazards by kind across all chunks.
    pub hazards_by_kind: [u32; HazardKind::COUNT],
    /// Number of commands executed.
    pub commands_executed: u64,
    /// Number of simulation steps run.
    pub steps_run: u64,
}

impl SandboxState {
    /// Check if the sandbox has any active simulation state.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.total_active_hazards > 0 || self.chunk_count > 0
    }

    /// Get active hazard count for a specific kind.
    #[must_use]
    pub fn hazard_count(&self, kind: HazardKind) -> u32 {
        self.hazards_by_kind[kind.as_index()]
    }
}

/// Complete sandbox snapshot for persistence and replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SandboxSnapshot {
    /// Sandbox state summary.
    pub state: SandboxState,
    /// Per-chunk summaries.
    pub chunk_summaries: HashMap<ChunkPos, ChunkSummary>,
    /// Full hazard snapshot.
    pub hazard_snapshot: HazardSnapshot,
    /// Checksum for verification.
    pub checksum: StepChecksum,
    /// Seed used for this sandbox.
    pub seed: u64,
}

impl SandboxSnapshot {
    /// Create an empty snapshot.
    #[must_use]
    pub fn empty(seed: u64, tick: u64) -> Self {
        Self {
            state: SandboxState {
                tick,
                ..Default::default()
            },
            chunk_summaries: HashMap::new(),
            hazard_snapshot: HazardSnapshot::empty(tick),
            checksum: StepChecksum::default(),
            seed,
        }
    }

    /// Number of chunks in snapshot.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunk_summaries.len()
    }

    /// Total active hazards.
    #[must_use]
    pub fn total_hazards(&self) -> u32 {
        self.state.total_active_hazards
    }

    /// Get summary for a specific chunk.
    #[must_use]
    pub fn chunk_summary(&self, pos: ChunkPos) -> Option<&ChunkSummary> {
        self.chunk_summaries.get(&pos)
    }

    /// Iterate over all chunk positions.
    pub fn chunk_positions(&self) -> impl Iterator<Item = &ChunkPos> {
        self.chunk_summaries.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_summary_has_data() {
        let mut summary = ChunkSummary::default();
        assert!(!summary.has_data());

        summary.active_hazards = 1;
        assert!(summary.has_data());

        summary.active_hazards = 0;
        summary.set_fluids(true);
        assert!(summary.has_data());
    }

    #[test]
    fn sandbox_state_accessors() {
        let mut state = SandboxState::default();
        assert!(!state.is_active());

        state.total_active_hazards = 10;
        state.hazards_by_kind[HazardKind::Fire.as_index()] = 5;
        state.hazards_by_kind[HazardKind::Frost.as_index()] = 5;

        assert!(state.is_active());
        assert_eq!(state.hazard_count(HazardKind::Fire), 5);
        assert_eq!(state.hazard_count(HazardKind::Frost), 5);
        assert_eq!(state.hazard_count(HazardKind::Infection), 0);
    }

    #[test]
    fn snapshot_empty() {
        let snapshot = SandboxSnapshot::empty(42, 100);
        assert_eq!(snapshot.seed, 42);
        assert_eq!(snapshot.state.tick, 100);
        assert_eq!(snapshot.chunk_count(), 0);
        assert_eq!(snapshot.total_hazards(), 0);
    }

    #[test]
    fn snapshot_serde_round_trip() {
        let mut snapshot = SandboxSnapshot::empty(123, 50);
        snapshot.chunk_summaries.insert(
            ChunkPos::new(0, 0, 0),
            ChunkSummary {
                active_hazards: 5,
                ..Default::default()
            },
        );
        snapshot.state.total_active_hazards = 5;

        let bytes = bincode::serialize(&snapshot).unwrap();
        let recovered: SandboxSnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(recovered.seed, snapshot.seed);
        assert_eq!(recovered.state.tick, snapshot.state.tick);
        assert_eq!(recovered.chunk_count(), 1);
    }
}

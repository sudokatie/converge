//! Deterministic server-side hazard simulation with tick-based I/O.
//!
//! Provides a simulation coordinator that processes hazard propagation across
//! multiple chunks with deterministic ordering, producing compact deltas for
//! network synchronization and journal recording.

use std::collections::HashMap;

use engine_core::coords::{ChunkPos, LocalPos};
use serde::{Deserialize, Serialize};

use super::hazard_delta::{
    ChunkHazardDelta, ChunkHazardSnapshot, HazardDeltaJournal, HazardDeltaRecord, HazardSnapshot,
};
use super::{ChunkHazards, HazardKind, PropagationConfig, ResistanceMap, propagation_step};
use crate::replay::{ChecksumBuilder, StepChecksum};

/// Input for a single-chunk hazard simulation tick.
pub struct ChunkTickInput<'a, R: ResistanceMap> {
    /// Chunk position.
    pub pos: ChunkPos,
    /// Current hazard state.
    pub hazards: &'a ChunkHazards,
    /// Propagation configs per hazard kind.
    pub configs: &'a [PropagationConfig; HazardKind::COUNT],
    /// Resistance map for spread calculations.
    pub resistance: &'a R,
    /// Delta time in seconds.
    pub dt: f32,
}

/// Output from a single-chunk hazard simulation tick.
#[derive(Clone, Debug, Default)]
pub struct ChunkTickOutput {
    /// Delta to apply to hazard state.
    pub delta: ChunkHazardDelta,
    /// Boundary spreads to propagate to neighbor chunks.
    pub boundary_spreads: Vec<BoundarySpread>,
    /// Statistics for this tick.
    pub stats: TickStats,
    /// Checksum of the simulation output.
    pub checksum: StepChecksum,
}

/// A hazard spread request crossing chunk boundaries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundarySpread {
    /// Source chunk position.
    pub source_chunk: ChunkPos,
    /// Source local position within chunk.
    pub source_pos: LocalPos,
    /// Direction to neighbor chunk (-1, 0, or 1 per axis).
    pub direction: (i32, i32, i32),
    /// Hazard kind.
    pub kind: HazardKind,
    /// Intensity to transfer.
    pub intensity: f32,
}

impl BoundarySpread {
    /// Compute the target chunk position.
    #[must_use]
    pub fn target_chunk(&self) -> ChunkPos {
        ChunkPos::new(
            self.source_chunk.x() + self.direction.0,
            self.source_chunk.y() + self.direction.1,
            self.source_chunk.z() + self.direction.2,
        )
    }

    /// Compute the target local position (wrapping to opposite edge).
    #[must_use]
    pub fn target_local_pos(&self) -> LocalPos {
        let wrap = |v: u32, d: i32| -> u32 {
            match d {
                -1 => 15,
                1 => 0,
                _ => v,
            }
        };

        LocalPos::new(
            wrap(self.source_pos.x(), self.direction.0),
            wrap(self.source_pos.y(), self.direction.1),
            wrap(self.source_pos.z(), self.direction.2),
        )
    }
}

/// Statistics from a simulation tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickStats {
    /// Cells that spread to neighbors.
    pub spread_count: u32,
    /// Cells that decayed.
    pub decay_count: u32,
    /// Cells that were extinguished.
    pub extinguished_count: u32,
    /// Boundary spreads generated.
    pub boundary_count: u32,
}

impl TickStats {
    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.spread_count > 0
            || self.decay_count > 0
            || self.extinguished_count > 0
            || self.boundary_count > 0
    }

    /// Merge another stats into this one.
    pub fn merge(&mut self, other: Self) {
        self.spread_count += other.spread_count;
        self.decay_count += other.decay_count;
        self.extinguished_count += other.extinguished_count;
        self.boundary_count += other.boundary_count;
    }
}

/// Execute deterministic hazard simulation for a single chunk.
///
/// Processes all hazard kinds in deterministic order, producing a compact
/// delta and boundary spread requests.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "index and len fit in u32")]
pub fn simulate_chunk_tick<R: ResistanceMap>(input: &ChunkTickInput<'_, R>) -> ChunkTickOutput {
    let mut output = ChunkTickOutput::default();
    let mut checksum_builder = ChecksumBuilder::new();

    checksum_builder.feed_i32(input.pos.x());
    checksum_builder.feed_i32(input.pos.y());
    checksum_builder.feed_i32(input.pos.z());
    checksum_builder.feed_f32(input.dt);

    for kind in HazardKind::ALL {
        let config = &input.configs[kind.as_index()];
        if !config.spread.is_active() && !config.decay.is_active() {
            continue;
        }

        let result = propagation_step(input.hazards, kind, config, input.dt, input.resistance);

        for delta in &result.deltas {
            if let Some(intensity) = delta.intensity {
                output.delta.add_set(kind, delta.pos, intensity);
                checksum_builder.feed_u32(kind.as_index() as u32);
                checksum_builder.feed_u32(delta.pos.to_index() as u32);
                checksum_builder.feed_f32(intensity);
            } else {
                output.delta.add_deactivate(kind, delta.pos);
                checksum_builder.feed_u32(kind.as_index() as u32);
                checksum_builder.feed_u32(delta.pos.to_index() as u32);
                checksum_builder.feed_u32(0);
            }
        }

        for (source_pos, direction, intensity) in &result.boundary_spreads {
            output.boundary_spreads.push(BoundarySpread {
                source_chunk: input.pos,
                source_pos: *source_pos,
                direction: *direction,
                kind,
                intensity: *intensity,
            });
            checksum_builder.feed_u32(kind.as_index() as u32);
            checksum_builder.feed_i32(direction.0);
            checksum_builder.feed_i32(direction.1);
            checksum_builder.feed_i32(direction.2);
            checksum_builder.feed_f32(*intensity);
        }

        output.stats.spread_count += result.spread_count;
        output.stats.decay_count += result.decayed_count;
        output.stats.extinguished_count += result.extinguished_count;
        output.stats.boundary_count += result.boundary_spreads.len() as u32;
    }

    output.checksum = checksum_builder.build();
    output
}

/// Multi-chunk hazard simulation coordinator.
///
/// Coordinates deterministic simulation across multiple chunks, handling
/// boundary propagation, delta journaling, and snapshot generation.
#[derive(Clone, Debug)]
pub struct HazardSimulator {
    /// Current simulation tick.
    tick: u64,
    /// Propagation configs per hazard kind.
    configs: [PropagationConfig; HazardKind::COUNT],
    /// Pending boundary spreads to apply next tick.
    pending_boundaries: Vec<BoundarySpread>,
    /// Delta journal for network sync.
    journal: HazardDeltaJournal,
    /// Maximum journal retention in ticks.
    max_journal_ticks: u64,
}

impl Default for HazardSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl HazardSimulator {
    /// Create a new simulator with default configs.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tick: 0,
            configs: std::array::from_fn(|i| PropagationConfig::new(HazardKind::ALL[i])),
            pending_boundaries: Vec::new(),
            journal: HazardDeltaJournal::new(),
            max_journal_ticks: 600,
        }
    }

    /// Create with specific starting tick.
    #[must_use]
    pub fn at_tick(tick: u64) -> Self {
        Self {
            tick,
            journal: HazardDeltaJournal::at_tick(tick),
            ..Self::new()
        }
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Get the propagation config for a hazard kind.
    #[must_use]
    pub fn config(&self, kind: HazardKind) -> &PropagationConfig {
        &self.configs[kind.as_index()]
    }

    /// Set the propagation config for a hazard kind.
    pub fn set_config(&mut self, kind: HazardKind, config: PropagationConfig) {
        self.configs[kind.as_index()] = config;
    }

    /// Set maximum journal retention in ticks.
    pub fn set_max_journal_ticks(&mut self, ticks: u64) {
        self.max_journal_ticks = ticks;
    }

    /// Get the delta journal.
    #[must_use]
    pub fn journal(&self) -> &HazardDeltaJournal {
        &self.journal
    }

    /// Execute a simulation tick across multiple chunks.
    ///
    /// Processes chunks in deterministic order (by position), applies deltas,
    /// collects boundary spreads, and records to journal.
    pub fn simulate_tick<R: ResistanceMap>(
        &mut self,
        chunks: &mut HashMap<ChunkPos, ChunkHazards>,
        resistance: &R,
        dt: f32,
    ) -> SimulationTickResult {
        self.tick += 1;
        self.journal.advance_tick(self.tick);

        let mut result = SimulationTickResult {
            tick: self.tick,
            ..Default::default()
        };

        let positions: Vec<_> = chunks
            .iter()
            .filter(|(_, h)| h.total_active() > 0)
            .map(|(&pos, _)| pos)
            .collect();

        self.apply_pending_boundaries(chunks);

        for pos in positions {
            let Some(hazards) = chunks.get(&pos) else {
                continue;
            };

            let input = ChunkTickInput {
                pos,
                hazards,
                configs: &self.configs,
                resistance,
                dt,
            };

            let output = simulate_chunk_tick(&input);

            if !output.delta.is_empty() {
                if let Some(h) = chunks.get_mut(&pos) {
                    apply_chunk_delta(h, &output.delta);
                }
                self.journal.append(pos, output.delta.clone());
                result.chunk_deltas.insert(pos, output.delta);
            }

            self.pending_boundaries.extend(output.boundary_spreads);
            result.stats.merge(output.stats);
            result.checksums.insert(pos, output.checksum);
        }

        if self.max_journal_ticks > 0 {
            self.journal.retain_recent(self.max_journal_ticks);
        }

        result.overall_checksum = Self::compute_tick_checksum(&result);
        result
    }

    /// Apply pending boundary spreads from previous tick.
    fn apply_pending_boundaries(&mut self, chunks: &mut HashMap<ChunkPos, ChunkHazards>) {
        let boundaries = std::mem::take(&mut self.pending_boundaries);

        for spread in boundaries {
            let target_chunk = spread.target_chunk();

            if let Some(hazards) = chunks.get_mut(&target_chunk) {
                let target_pos = spread.target_local_pos();
                let current = hazards.get(spread.kind, target_pos);

                if current.intensity() < spread.intensity {
                    let new_intensity = (current.intensity() + spread.intensity).min(1.0);
                    hazards.activate(spread.kind, target_pos, new_intensity);
                }
            }
        }
    }

    /// Compute a deterministic checksum for the tick result.
    fn compute_tick_checksum(result: &SimulationTickResult) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u64(result.tick);

        for (pos, checksum) in &result.checksums {
            builder.feed_i32(pos.x());
            builder.feed_i32(pos.y());
            builder.feed_i32(pos.z());
            builder.feed_u32(checksum.value());
        }

        builder.build()
    }

    /// Get deltas since a specific tick for a client.
    ///
    /// Filters by tick and optional area bounds for interest management.
    pub fn deltas_for_client(
        &self,
        since_tick: u64,
        area_min: Option<ChunkPos>,
        area_max: Option<ChunkPos>,
    ) -> Vec<&HazardDeltaRecord> {
        match (area_min, area_max) {
            (Some(min), Some(max)) => self
                .journal
                .since_tick_in_area(since_tick, min, max)
                .collect(),
            _ => self.journal.since_tick(since_tick).collect(),
        }
    }

    /// Generate a snapshot of current hazard state for late-join.
    #[must_use]
    pub fn snapshot(&self, chunks: &HashMap<ChunkPos, ChunkHazards>) -> HazardSnapshot {
        let mut snapshot = HazardSnapshot::empty(self.tick);

        for (&pos, hazards) in chunks {
            if hazards.is_empty() {
                continue;
            }

            let mut chunk_snapshot = ChunkHazardSnapshot::new();

            for (kind, local_pos, cell) in hazards.iter_all_active() {
                chunk_snapshot.add(kind, local_pos, cell.intensity());
            }

            if !chunk_snapshot.is_empty() {
                snapshot.chunk_states.insert(pos, chunk_snapshot);
            }
        }

        snapshot
    }

    /// Generate a snapshot for specific chunks within an area.
    #[must_use]
    pub fn snapshot_area(
        &self,
        chunks: &HashMap<ChunkPos, ChunkHazards>,
        min: ChunkPos,
        max: ChunkPos,
    ) -> HazardSnapshot {
        let mut snapshot = HazardSnapshot::empty(self.tick);

        for (&pos, hazards) in chunks {
            if pos.x() < min.x()
                || pos.x() > max.x()
                || pos.y() < min.y()
                || pos.y() > max.y()
                || pos.z() < min.z()
                || pos.z() > max.z()
            {
                continue;
            }

            if hazards.is_empty() {
                continue;
            }

            let mut chunk_snapshot = ChunkHazardSnapshot::new();

            for (kind, local_pos, cell) in hazards.iter_all_active() {
                chunk_snapshot.add(kind, local_pos, cell.intensity());
            }

            if !chunk_snapshot.is_empty() {
                snapshot.chunk_states.insert(pos, chunk_snapshot);
            }
        }

        snapshot
    }
}

/// Result of a multi-chunk simulation tick.
#[derive(Clone, Debug, Default)]
pub struct SimulationTickResult {
    /// Tick number.
    pub tick: u64,
    /// Deltas per chunk.
    pub chunk_deltas: HashMap<ChunkPos, ChunkHazardDelta>,
    /// Per-chunk checksums.
    pub checksums: HashMap<ChunkPos, StepChecksum>,
    /// Aggregated statistics.
    pub stats: TickStats,
    /// Overall tick checksum.
    pub overall_checksum: StepChecksum,
}

impl SimulationTickResult {
    /// Check if any changes occurred.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.chunk_deltas.is_empty() || self.stats.has_changes()
    }

    /// Number of chunks with changes.
    #[must_use]
    pub fn affected_chunk_count(&self) -> usize {
        self.chunk_deltas.len()
    }
}

/// Apply a chunk hazard delta to hazard storage.
pub fn apply_chunk_delta(hazards: &mut ChunkHazards, delta: &ChunkHazardDelta) {
    for (kind, cell_deltas) in delta.iter() {
        for cell_delta in cell_deltas {
            match cell_delta.intensity {
                Some(intensity) => {
                    hazards.activate(kind, cell_delta.local_pos(), intensity);
                }
                None => {
                    hazards.deactivate(kind, cell_delta.local_pos());
                }
            }
        }
    }
}

/// Apply a hazard snapshot to chunk storage.
#[expect(clippy::implicit_hasher, reason = "public API uses standard HashMap")]
pub fn apply_snapshot(chunks: &mut HashMap<ChunkPos, ChunkHazards>, snapshot: &HazardSnapshot) {
    for (&pos, chunk_snapshot) in &snapshot.chunk_states {
        let hazards = chunks.entry(pos).or_default();

        for (kind, cells) in chunk_snapshot.iter() {
            for &(index, intensity) in cells {
                hazards.activate(kind, index.to_local_pos(), intensity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::SpreadConfig;
    use super::*;

    fn default_configs() -> [PropagationConfig; HazardKind::COUNT] {
        std::array::from_fn(|i| PropagationConfig::new(HazardKind::ALL[i]))
    }

    #[test]
    fn boundary_spread_target_positions() {
        let spread = BoundarySpread {
            source_chunk: ChunkPos::new(0, 0, 0),
            source_pos: LocalPos::new(0, 8, 8),
            direction: (-1, 0, 0),
            kind: HazardKind::Fire,
            intensity: 0.5,
        };

        assert_eq!(spread.target_chunk(), ChunkPos::new(-1, 0, 0));
        assert_eq!(spread.target_local_pos(), LocalPos::new(15, 8, 8));
    }

    #[test]
    fn tick_stats_merge() {
        let mut stats1 = TickStats {
            spread_count: 5,
            decay_count: 3,
            extinguished_count: 1,
            boundary_count: 2,
        };

        let stats2 = TickStats {
            spread_count: 3,
            decay_count: 2,
            extinguished_count: 1,
            boundary_count: 1,
        };

        stats1.merge(stats2);

        assert_eq!(stats1.spread_count, 8);
        assert_eq!(stats1.decay_count, 5);
        assert_eq!(stats1.extinguished_count, 2);
        assert_eq!(stats1.boundary_count, 3);
    }

    #[test]
    fn simulate_chunk_tick_empty() {
        let hazards = ChunkHazards::new();
        let configs = default_configs();

        let input = ChunkTickInput {
            pos: ChunkPos::new(0, 0, 0),
            hazards: &hazards,
            configs: &configs,
            resistance: &(),
            dt: 0.1,
        };

        let output = simulate_chunk_tick(&input);
        assert!(output.delta.is_empty());
        assert!(output.boundary_spreads.is_empty());
        assert!(!output.stats.has_changes());
    }

    #[test]
    fn simulate_chunk_tick_with_fire() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);

        let configs = default_configs();

        let input = ChunkTickInput {
            pos: ChunkPos::new(0, 0, 0),
            hazards: &hazards,
            configs: &configs,
            resistance: &(),
            dt: 1.0,
        };

        let output = simulate_chunk_tick(&input);
        assert!(!output.delta.is_empty());
        assert!(output.stats.spread_count > 0 || output.stats.decay_count > 0);
    }

    #[test]
    fn simulate_chunk_tick_boundary_spread() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(0, 8, 8), 1.0);

        let configs = default_configs();

        let input = ChunkTickInput {
            pos: ChunkPos::new(0, 0, 0),
            hazards: &hazards,
            configs: &configs,
            resistance: &(),
            dt: 1.0,
        };

        let output = simulate_chunk_tick(&input);
        assert!(!output.boundary_spreads.is_empty());

        let negative_x = output.boundary_spreads.iter().any(|s| s.direction.0 == -1);
        assert!(negative_x);
    }

    #[test]
    fn simulate_chunk_tick_deterministic() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 0.8);
        hazards.activate(HazardKind::Frost, LocalPos::new(4, 4, 4), 0.6);

        let configs = default_configs();

        let input = ChunkTickInput {
            pos: ChunkPos::new(0, 0, 0),
            hazards: &hazards,
            configs: &configs,
            resistance: &(),
            dt: 0.5,
        };

        let output1 = simulate_chunk_tick(&input);
        let output2 = simulate_chunk_tick(&input);

        assert_eq!(output1.checksum, output2.checksum);
        assert_eq!(output1.delta.checksum(), output2.delta.checksum());
    }

    #[test]
    fn simulator_basic_tick() {
        let mut simulator = HazardSimulator::at_tick(100);
        let mut chunks = HashMap::new();

        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
        chunks.insert(ChunkPos::new(0, 0, 0), hazards);

        let result = simulator.simulate_tick(&mut chunks, &(), 0.5);

        assert_eq!(result.tick, 101);
        assert!(result.has_changes());
    }

    #[test]
    fn simulator_boundary_propagation() {
        let mut simulator = HazardSimulator::at_tick(0);
        let mut chunks = HashMap::new();

        let mut hazards0 = ChunkHazards::new();
        hazards0.activate(HazardKind::Fire, LocalPos::new(15, 8, 8), 1.0);
        chunks.insert(ChunkPos::new(0, 0, 0), hazards0);
        chunks.insert(ChunkPos::new(1, 0, 0), ChunkHazards::new());

        let result = simulator.simulate_tick(&mut chunks, &(), 1.0);
        assert!(
            result.stats.boundary_count > 0,
            "Should generate boundary spreads for edge fire"
        );

        simulator.simulate_tick(&mut chunks, &(), 1.0);

        let neighbor = chunks.get(&ChunkPos::new(1, 0, 0)).unwrap();
        let target_pos = LocalPos::new(0, 8, 8);
        let received = neighbor.get(HazardKind::Fire, target_pos);
        assert!(
            received.intensity() > 0.0,
            "Fire should propagate from (15,8,8) to (0,8,8) in neighbor chunk, got intensity={}",
            received.intensity()
        );
    }

    #[test]
    fn simulator_journal_recording() {
        let mut simulator = HazardSimulator::at_tick(100);
        let mut chunks = HashMap::new();

        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
        chunks.insert(ChunkPos::new(0, 0, 0), hazards);

        for _ in 0..5 {
            simulator.simulate_tick(&mut chunks, &(), 0.5);
        }

        assert!(!simulator.journal().is_empty());
        let records: Vec<_> = simulator.journal().since_tick(102).collect();
        assert!(records.len() >= 2);
    }

    #[test]
    fn simulator_deltas_for_client() {
        let mut simulator = HazardSimulator::at_tick(100);
        let mut chunks = HashMap::new();

        for x in -2..3 {
            let mut hazards = ChunkHazards::new();
            hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
            chunks.insert(ChunkPos::new(x, 0, 0), hazards);
        }

        for _ in 0..3 {
            simulator.simulate_tick(&mut chunks, &(), 0.5);
        }

        let all_deltas = simulator.deltas_for_client(100, None, None);
        let filtered_deltas = simulator.deltas_for_client(
            100,
            Some(ChunkPos::new(-1, 0, 0)),
            Some(ChunkPos::new(1, 0, 0)),
        );

        assert!(filtered_deltas.len() <= all_deltas.len());
    }

    #[test]
    fn simulator_snapshot() {
        let simulator = HazardSimulator::at_tick(100);
        let mut chunks = HashMap::new();

        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        hazards.activate(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        hazards.activate(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);
        chunks.insert(ChunkPos::new(0, 0, 0), hazards);

        let snapshot = simulator.snapshot(&chunks);

        assert_eq!(snapshot.base_tick, 100);
        assert_eq!(snapshot.chunk_count(), 1);
        assert_eq!(snapshot.total_active(), 3);
    }

    #[test]
    fn simulator_snapshot_area() {
        let simulator = HazardSimulator::at_tick(100);
        let mut chunks = HashMap::new();

        for x in -5..6 {
            let mut hazards = ChunkHazards::new();
            hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
            chunks.insert(ChunkPos::new(x, 0, 0), hazards);
        }

        let snapshot =
            simulator.snapshot_area(&chunks, ChunkPos::new(-1, 0, 0), ChunkPos::new(1, 0, 0));

        assert_eq!(snapshot.chunk_count(), 3);
    }

    #[test]
    fn apply_chunk_delta_sets_and_deactivates() {
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);

        let mut delta = ChunkHazardDelta::new();
        delta.add_set(HazardKind::Fire, LocalPos::new(0, 0, 0), 0.5);
        delta.add_set(HazardKind::Fire, LocalPos::new(1, 0, 0), 0.8);
        delta.add_deactivate(HazardKind::Frost, LocalPos::new(5, 5, 5));

        apply_chunk_delta(&mut hazards, &delta);

        assert!(
            (hazards
                .get(HazardKind::Fire, LocalPos::new(0, 0, 0))
                .intensity()
                - 0.5)
                .abs()
                < 0.001
        );
        assert!(
            (hazards
                .get(HazardKind::Fire, LocalPos::new(1, 0, 0))
                .intensity()
                - 0.8)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn apply_snapshot_populates_chunks() {
        let mut chunks = HashMap::new();

        let mut snapshot = HazardSnapshot::empty(100);
        let mut chunk_state = ChunkHazardSnapshot::new();
        chunk_state.add(HazardKind::Fire, LocalPos::new(0, 0, 0), 1.0);
        chunk_state.add(HazardKind::Frost, LocalPos::new(5, 5, 5), 0.5);
        snapshot
            .chunk_states
            .insert(ChunkPos::new(0, 0, 0), chunk_state);

        apply_snapshot(&mut chunks, &snapshot);

        let hazards = chunks.get(&ChunkPos::new(0, 0, 0)).unwrap();
        assert!(
            hazards
                .get(HazardKind::Fire, LocalPos::new(0, 0, 0))
                .is_active()
        );
        assert!(
            hazards
                .get(HazardKind::Frost, LocalPos::new(5, 5, 5))
                .is_active()
        );
    }

    #[test]
    fn simulation_tick_result_accessors() {
        let mut result = SimulationTickResult {
            tick: 100,
            ..Default::default()
        };

        assert!(!result.has_changes());
        assert_eq!(result.affected_chunk_count(), 0);

        result
            .chunk_deltas
            .insert(ChunkPos::new(0, 0, 0), ChunkHazardDelta::new());
        result.stats.spread_count = 5;

        assert!(result.has_changes());
        assert_eq!(result.affected_chunk_count(), 1);
    }

    #[test]
    fn simulator_config_access() {
        let mut simulator = HazardSimulator::new();

        let fire_config = simulator.config(HazardKind::Fire).clone();
        assert!(fire_config.spread.is_active());

        let mut new_config = fire_config.clone();
        new_config.spread = SpreadConfig::NONE;
        simulator.set_config(HazardKind::Fire, new_config);

        assert!(!simulator.config(HazardKind::Fire).spread.is_active());
    }

    #[test]
    fn simulator_journal_retention() {
        let mut simulator = HazardSimulator::at_tick(0);
        simulator.set_max_journal_ticks(5);

        let mut chunks = HashMap::new();
        let mut hazards = ChunkHazards::new();
        hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
        chunks.insert(ChunkPos::new(0, 0, 0), hazards);

        for _ in 0..20 {
            simulator.simulate_tick(&mut chunks, &(), 0.1);
            let hazards = chunks.get_mut(&ChunkPos::new(0, 0, 0)).unwrap();
            hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
        }

        if !simulator.journal().is_empty() {
            let oldest_tick = simulator.journal().records().iter().map(|r| r.tick).min();
            let newest_tick = simulator.journal().records().iter().map(|r| r.tick).max();
            if let (Some(oldest), Some(newest)) = (oldest_tick, newest_tick) {
                let range = newest - oldest + 1;
                assert!(
                    range <= 5,
                    "Journal should retain at most 5 ticks of records"
                );
            }
        }
    }

    #[test]
    fn serde_roundtrip_boundary_spread() {
        let spread = BoundarySpread {
            source_chunk: ChunkPos::new(1, 2, 3),
            source_pos: LocalPos::new(15, 8, 8),
            direction: (1, 0, 0),
            kind: HazardKind::Fire,
            intensity: 0.75,
        };

        let serialized = bincode::serialize(&spread).unwrap();
        let deserialized: BoundarySpread = bincode::deserialize(&serialized).unwrap();

        assert_eq!(spread, deserialized);
    }

    #[test]
    fn serde_roundtrip_tick_stats() {
        let stats = TickStats {
            spread_count: 10,
            decay_count: 5,
            extinguished_count: 2,
            boundary_count: 3,
        };

        let serialized = bincode::serialize(&stats).unwrap();
        let deserialized: TickStats = bincode::deserialize(&serialized).unwrap();

        assert_eq!(stats, deserialized);
    }
}

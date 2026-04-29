//! Main scenario sandbox implementation.

use std::collections::HashMap;

use engine_core::coords::ChunkPos;
use serde::{Deserialize, Serialize};

use super::command::{CommandResult, SpawnCommand, SpawnKind};
use super::config::SandboxConfig;
use super::snapshot::{ChunkSummary, SandboxSnapshot, SandboxState};
use crate::environment::{
    ChunkFields, ChunkFluids, ChunkHazards, ChunkStructural, ChunkVectorFields, FluidCell,
    HazardKind, HazardSimulator, HazardSnapshot, SimulationTickResult, TickStats,
};
use crate::replay::{ChecksumBuilder, StepChecksum};

/// Result of a simulation step.
#[derive(Clone, Debug, Default)]
pub struct StepResult {
    /// Tick number after step.
    pub tick: u64,
    /// Statistics from hazard simulation.
    pub stats: TickStats,
    /// Per-chunk checksums.
    pub checksums: HashMap<ChunkPos, StepChecksum>,
    /// Overall step checksum.
    pub overall_checksum: StepChecksum,
    /// Whether any changes occurred.
    pub had_changes: bool,
}

impl StepResult {
    /// Number of chunks that changed.
    #[must_use]
    pub fn changed_chunk_count(&self) -> usize {
        self.checksums.len()
    }
}

/// History entry for replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Tick when entry was recorded.
    pub tick: u64,
    /// Commands executed this tick.
    pub commands: Vec<SpawnCommand>,
    /// Delta time for this step.
    pub dt: f32,
    /// Checksum after step.
    pub checksum: StepChecksum,
}

/// Scenario sandbox for deterministic simulation testing.
///
/// Provides a self-contained environment for spawning hazards, environmental
/// effects, and stepping simulation forward with deterministic ordering and
/// checksum verification.
pub struct ScenarioSandbox {
    config: SandboxConfig,
    tick: u64,
    hazards: HashMap<ChunkPos, ChunkHazards>,
    scalar_fields: HashMap<ChunkPos, ChunkFields>,
    vector_fields: HashMap<ChunkPos, ChunkVectorFields>,
    fluids: HashMap<ChunkPos, ChunkFluids>,
    structural: HashMap<ChunkPos, ChunkStructural>,
    simulator: HazardSimulator,
    history: Vec<HistoryEntry>,
    pending_commands: Vec<SpawnCommand>,
    commands_executed: u64,
    steps_run: u64,
}

impl ScenarioSandbox {
    /// Create a new sandbox with default config.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::with_config(SandboxConfig::new(seed))
    }

    /// Create a sandbox with specific config.
    #[must_use]
    pub fn with_config(config: SandboxConfig) -> Self {
        let mut simulator = HazardSimulator::new();
        for kind in HazardKind::ALL {
            simulator.set_config(kind, config.hazard_configs[kind.as_index()].clone());
        }

        Self {
            config,
            tick: 0,
            hazards: HashMap::new(),
            scalar_fields: HashMap::new(),
            vector_fields: HashMap::new(),
            fluids: HashMap::new(),
            structural: HashMap::new(),
            simulator,
            history: Vec::new(),
            pending_commands: Vec::new(),
            commands_executed: 0,
            steps_run: 0,
        }
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Get the seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.config.seed
    }

    /// Get the config.
    #[must_use]
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Queue a spawn command to execute on next step.
    pub fn queue(&mut self, command: SpawnCommand) {
        self.pending_commands.push(command);
    }

    /// Execute a spawn command immediately.
    #[allow(clippy::needless_pass_by_value)]
    pub fn execute(&mut self, command: SpawnCommand) -> CommandResult {
        self.execute_command_internal(&command)
    }

    /// Execute multiple commands.
    pub fn execute_batch(&mut self, commands: impl IntoIterator<Item = SpawnCommand>) {
        for cmd in commands {
            self.execute_command_internal(&cmd);
        }
    }

    fn execute_command_internal(&mut self, command: &SpawnCommand) -> CommandResult {
        let chunk_pos = command.pos.to_chunk_pos();
        let local_pos = command.pos.to_local_pos();

        match &command.kind {
            SpawnKind::Hazard { kind, intensity } => {
                self.ensure_hazard_chunk(chunk_pos);
                if let Some(hazards) = self.hazards.get_mut(&chunk_pos) {
                    hazards.activate(*kind, local_pos, *intensity);
                    self.commands_executed += 1;
                    return CommandResult::ok(self.tick);
                }
                CommandResult::err(self.tick, "failed to activate hazard")
            }

            SpawnKind::ScalarField { channel, value } => {
                self.ensure_scalar_fields_chunk(chunk_pos);
                if let Some(fields) = self.scalar_fields.get_mut(&chunk_pos) {
                    fields.set(*channel, local_pos, *value);
                    self.commands_executed += 1;
                    return CommandResult::ok(self.tick);
                }
                CommandResult::err(self.tick, "failed to set scalar field")
            }

            SpawnKind::VectorField { channel, value } => {
                self.ensure_vector_fields_chunk(chunk_pos);
                if let Some(fields) = self.vector_fields.get_mut(&chunk_pos) {
                    fields.set(*channel, local_pos, *value);
                    self.commands_executed += 1;
                    return CommandResult::ok(self.tick);
                }
                CommandResult::err(self.tick, "failed to set vector field")
            }

            SpawnKind::Fluid {
                kind,
                volume,
                pressure,
                temperature,
            } => {
                self.ensure_fluids_chunk(chunk_pos);
                if let Some(fluids) = self.fluids.get_mut(&chunk_pos) {
                    let cell = FluidCell::with_state(*kind, *volume, *pressure, *temperature);
                    fluids.set(*kind, local_pos, cell);
                    self.commands_executed += 1;
                    return CommandResult::ok(self.tick);
                }
                CommandResult::err(self.tick, "failed to set fluid")
            }

            SpawnKind::StructuralLoad { load } => {
                self.ensure_structural_chunk(chunk_pos);
                if let Some(structural) = self.structural.get_mut(&chunk_pos) {
                    let cell = structural.get_mut(local_pos);
                    cell.add_load(*load);
                    self.commands_executed += 1;
                    return CommandResult::ok(self.tick);
                }
                CommandResult::err(self.tick, "failed to add structural load")
            }
        }
    }

    fn ensure_hazard_chunk(&mut self, pos: ChunkPos) {
        if self.config.auto_create_chunks && !self.hazards.contains_key(&pos) {
            self.hazards.insert(pos, ChunkHazards::new());
        }
    }

    fn ensure_scalar_fields_chunk(&mut self, pos: ChunkPos) {
        if self.config.auto_create_chunks && !self.scalar_fields.contains_key(&pos) {
            self.scalar_fields.insert(pos, ChunkFields::new());
        }
    }

    fn ensure_vector_fields_chunk(&mut self, pos: ChunkPos) {
        if self.config.auto_create_chunks && !self.vector_fields.contains_key(&pos) {
            self.vector_fields.insert(pos, ChunkVectorFields::new());
        }
    }

    fn ensure_fluids_chunk(&mut self, pos: ChunkPos) {
        if self.config.auto_create_chunks && !self.fluids.contains_key(&pos) {
            self.fluids.insert(pos, ChunkFluids::new());
        }
    }

    fn ensure_structural_chunk(&mut self, pos: ChunkPos) {
        if self.config.auto_create_chunks && !self.structural.contains_key(&pos) {
            self.structural.insert(pos, ChunkStructural::new());
        }
    }

    /// Step simulation forward by dt seconds.
    pub fn step(&mut self, dt: f32) -> StepResult {
        self.step_with_dt(dt)
    }

    /// Step simulation forward using default dt.
    pub fn step_default(&mut self) -> StepResult {
        self.step_with_dt(self.config.default_dt)
    }

    /// Step multiple times.
    pub fn step_n(&mut self, n: usize, dt: f32) -> Vec<StepResult> {
        (0..n).map(|_| self.step_with_dt(dt)).collect()
    }

    fn step_with_dt(&mut self, dt: f32) -> StepResult {
        let pending = std::mem::take(&mut self.pending_commands);
        for cmd in &pending {
            self.execute_command_internal(cmd);
        }

        let sim_result: SimulationTickResult =
            self.simulator.simulate_tick(&mut self.hazards, &(), dt);

        self.tick = sim_result.tick;
        self.steps_run += 1;

        let had_changes = sim_result.has_changes();
        let result = StepResult {
            tick: self.tick,
            stats: sim_result.stats,
            checksums: sim_result.checksums,
            overall_checksum: sim_result.overall_checksum,
            had_changes,
        };

        if self.config.record_history {
            let entry = HistoryEntry {
                tick: self.tick,
                commands: pending,
                dt,
                checksum: result.overall_checksum,
            };
            self.history.push(entry);

            if self.config.max_history > 0 && self.history.len() > self.config.max_history {
                let drain_count = self.history.len() - self.config.max_history;
                self.history.drain(0..drain_count);
            }
        }

        result
    }

    /// Get hazard cell at world position.
    #[must_use]
    pub fn get_hazard(&self, kind: HazardKind, pos: engine_core::coords::WorldPos) -> f32 {
        let chunk_pos = pos.to_chunk_pos();
        let local_pos = pos.to_local_pos();

        self.hazards
            .get(&chunk_pos)
            .map_or(0.0, |h| h.get(kind, local_pos).intensity())
    }

    /// Get hazard chunk if loaded.
    #[must_use]
    pub fn hazard_chunk(&self, pos: ChunkPos) -> Option<&ChunkHazards> {
        self.hazards.get(&pos)
    }

    /// Iterate over all loaded hazard chunks.
    pub fn hazard_chunks(&self) -> impl Iterator<Item = (&ChunkPos, &ChunkHazards)> {
        self.hazards.iter()
    }

    /// Clear all hazards of a specific kind.
    pub fn clear_hazard_kind(&mut self, kind: HazardKind) {
        for hazards in self.hazards.values_mut() {
            hazards.clear_layer(kind);
        }
    }

    /// Clear all simulation state.
    pub fn clear(&mut self) {
        self.hazards.clear();
        self.scalar_fields.clear();
        self.vector_fields.clear();
        self.fluids.clear();
        self.structural.clear();
        self.history.clear();
        self.pending_commands.clear();
    }

    /// Reset to initial state (keeps config).
    pub fn reset(&mut self) {
        self.clear();
        self.tick = 0;
        self.commands_executed = 0;
        self.steps_run = 0;
        self.simulator = HazardSimulator::new();
        for kind in HazardKind::ALL {
            self.simulator
                .set_config(kind, self.config.hazard_configs[kind.as_index()].clone());
        }
    }

    /// Get current state summary.
    #[must_use]
    pub fn state(&self) -> SandboxState {
        let mut state = SandboxState {
            tick: self.tick,
            chunk_count: self.hazards.len(),
            total_active_hazards: 0,
            hazards_by_kind: [0; HazardKind::COUNT],
            commands_executed: self.commands_executed,
            steps_run: self.steps_run,
        };

        for hazards in self.hazards.values() {
            for kind in HazardKind::ALL {
                if let Some(layer) = hazards.layer(kind) {
                    let count = layer.active_count();
                    state.hazards_by_kind[kind.as_index()] += count;
                    state.total_active_hazards += count;
                }
            }
        }

        state
    }

    /// Get chunk summary.
    #[must_use]
    pub fn chunk_summary(&self, pos: ChunkPos) -> ChunkSummary {
        let mut summary = ChunkSummary::default();

        if let Some(hazards) = self.hazards.get(&pos) {
            for kind in HazardKind::ALL {
                if let Some(layer) = hazards.layer(kind) {
                    let count = layer.active_count();
                    summary.hazards_by_kind[kind.as_index()] = count;
                    summary.active_hazards += count;
                }
            }
        }

        summary.set_scalar_fields(self.scalar_fields.contains_key(&pos));
        summary.set_vector_fields(self.vector_fields.contains_key(&pos));
        summary.set_fluids(self.fluids.contains_key(&pos));
        summary.set_structural(self.structural.contains_key(&pos));

        summary
    }

    /// Generate a full snapshot.
    #[must_use]
    pub fn snapshot(&self) -> SandboxSnapshot {
        let state = self.state();
        let mut chunk_summaries = HashMap::new();

        for &pos in self.hazards.keys() {
            chunk_summaries.insert(pos, self.chunk_summary(pos));
        }

        let hazard_snapshot = self.simulator.snapshot(&self.hazards);

        let checksum = self.compute_snapshot_checksum();

        SandboxSnapshot {
            state,
            chunk_summaries,
            hazard_snapshot,
            checksum,
            seed: self.config.seed,
        }
    }

    /// Get hazard snapshot only.
    #[must_use]
    pub fn hazard_snapshot(&self) -> HazardSnapshot {
        self.simulator.snapshot(&self.hazards)
    }

    fn compute_snapshot_checksum(&self) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        builder.feed_u64(self.tick);
        builder.feed_u64(self.config.seed);

        let state = self.state();
        builder.feed_u32(state.total_active_hazards);

        for &count in &state.hazards_by_kind {
            builder.feed_u32(count);
        }

        builder.build()
    }

    /// Get simulation history.
    #[must_use]
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Replay history from another sandbox.
    pub fn replay(&mut self, entries: &[HistoryEntry]) -> Vec<(u64, bool)> {
        let mut results = Vec::with_capacity(entries.len());

        for entry in entries {
            for cmd in &entry.commands {
                self.execute_command_internal(cmd);
            }

            let result = self.step_with_dt(entry.dt);
            let checksum_matches = result.overall_checksum == entry.checksum;
            results.push((entry.tick, checksum_matches));
        }

        results
    }

    /// Spawn a line of hazards for testing.
    pub fn spawn_hazard_line(
        &mut self,
        kind: HazardKind,
        start: engine_core::coords::WorldPos,
        end: engine_core::coords::WorldPos,
        intensity: f32,
    ) {
        let dx = (end.0.x - start.0.x).signum();
        let dy = (end.0.y - start.0.y).signum();
        let dz = (end.0.z - start.0.z).signum();

        let steps = (end.0.x - start.0.x)
            .abs()
            .max((end.0.y - start.0.y).abs())
            .max((end.0.z - start.0.z).abs());

        let mut current = start;
        for _ in 0..=steps {
            self.execute(SpawnCommand::hazard(current, kind, intensity));

            if current.0.x != end.0.x {
                current.0.x += dx;
            }
            if current.0.y != end.0.y {
                current.0.y += dy;
            }
            if current.0.z != end.0.z {
                current.0.z += dz;
            }
        }
    }

    /// Spawn a filled box of hazards.
    pub fn spawn_hazard_box(
        &mut self,
        kind: HazardKind,
        min: engine_core::coords::WorldPos,
        max: engine_core::coords::WorldPos,
        intensity: f32,
    ) {
        for x in min.x()..=max.x() {
            for y in min.y()..=max.y() {
                for z in min.z()..=max.z() {
                    let pos = engine_core::coords::WorldPos::new(x, y, z);
                    self.execute(SpawnCommand::hazard(pos, kind, intensity));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use engine_core::coords::WorldPos;

    use super::*;

    #[test]
    fn new_sandbox() {
        let sandbox = ScenarioSandbox::new(42);
        assert_eq!(sandbox.seed(), 42);
        assert_eq!(sandbox.current_tick(), 0);
    }

    #[test]
    fn execute_hazard_command() {
        let mut sandbox = ScenarioSandbox::new(0);
        let pos = WorldPos::new(8, 8, 8);

        let result = sandbox.execute(SpawnCommand::hazard(pos, HazardKind::Fire, 0.8));
        assert!(result.success);

        let intensity = sandbox.get_hazard(HazardKind::Fire, pos);
        assert!((intensity - 0.8).abs() < 0.001);
    }

    #[test]
    fn step_simulation() {
        let mut sandbox = ScenarioSandbox::new(0);
        let pos = WorldPos::new(8, 8, 8);

        sandbox.execute(SpawnCommand::hazard(pos, HazardKind::Fire, 1.0));

        let result = sandbox.step(0.1);
        assert_eq!(result.tick, 1);
    }

    #[test]
    fn step_n_times() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));

        let results = sandbox.step_n(5, 0.1);
        assert_eq!(results.len(), 5);
        assert_eq!(sandbox.current_tick(), 5);
    }

    #[test]
    fn state_summary() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(1, 0, 0),
            HazardKind::Fire,
            0.5,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(2, 0, 0),
            HazardKind::Frost,
            0.8,
        ));

        let state = sandbox.state();
        assert_eq!(state.total_active_hazards, 3);
        assert_eq!(state.hazard_count(HazardKind::Fire), 2);
        assert_eq!(state.hazard_count(HazardKind::Frost), 1);
    }

    #[test]
    fn snapshot_generation() {
        let mut sandbox = ScenarioSandbox::new(42);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.step(0.1);

        let snapshot = sandbox.snapshot();
        assert_eq!(snapshot.seed, 42);
        assert_eq!(snapshot.state.tick, 1);
        assert!(snapshot.chunk_count() > 0);
    }

    #[test]
    fn clear_and_reset() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.step(0.1);

        sandbox.clear();
        assert_eq!(sandbox.state().chunk_count, 0);
        assert_eq!(sandbox.current_tick(), 1);

        sandbox.reset();
        assert_eq!(sandbox.current_tick(), 0);
    }

    #[test]
    fn history_recording() {
        let config = SandboxConfig {
            record_history: true,
            ..SandboxConfig::new(0)
        };
        let mut sandbox = ScenarioSandbox::with_config(config);

        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.step(0.1);
        sandbox.step(0.1);

        assert_eq!(sandbox.history().len(), 2);
    }

    #[test]
    fn history_disabled() {
        let config = SandboxConfig {
            record_history: false,
            ..SandboxConfig::new(0)
        };
        let mut sandbox = ScenarioSandbox::with_config(config);

        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.step(0.1);
        sandbox.step(0.1);

        assert!(sandbox.history().is_empty());
    }

    #[test]
    fn spawn_hazard_line() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.spawn_hazard_line(
            HazardKind::Fire,
            WorldPos::new(0, 0, 0),
            WorldPos::new(4, 0, 0),
            1.0,
        );

        let state = sandbox.state();
        assert_eq!(state.hazard_count(HazardKind::Fire), 5);
    }

    #[test]
    fn spawn_hazard_box() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.spawn_hazard_box(
            HazardKind::Frost,
            WorldPos::new(0, 0, 0),
            WorldPos::new(2, 2, 2),
            0.5,
        );

        let state = sandbox.state();
        assert_eq!(state.hazard_count(HazardKind::Frost), 27);
    }

    #[test]
    fn deterministic_simulation() {
        let run_scenario = |seed| {
            let mut sandbox = ScenarioSandbox::new(seed);
            sandbox.execute(SpawnCommand::hazard(
                WorldPos::new(8, 8, 8),
                HazardKind::Fire,
                1.0,
            ));

            for _ in 0..10 {
                sandbox.step(0.1);
            }

            sandbox.snapshot().checksum
        };

        let checksum1 = run_scenario(42);
        let checksum2 = run_scenario(42);
        let checksum3 = run_scenario(99);

        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn queued_commands() {
        let mut sandbox = ScenarioSandbox::new(0);

        sandbox.queue(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));

        assert_eq!(sandbox.state().total_active_hazards, 0);

        sandbox.step(0.1);

        assert!(sandbox.state().total_active_hazards > 0);
    }

    #[test]
    fn replay_matches() {
        let mut sandbox1 = ScenarioSandbox::new(42);
        sandbox1.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));

        for _ in 0..5 {
            sandbox1.step(0.1);
        }

        let history = sandbox1.history().to_vec();

        let mut sandbox2 = ScenarioSandbox::new(42);
        sandbox2.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));

        let results = sandbox2.replay(&history);

        for (tick, matched) in results {
            assert!(matched, "checksum mismatch at tick {tick}");
        }
    }

    #[test]
    fn clear_hazard_kind() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(1, 0, 0),
            HazardKind::Frost,
            1.0,
        ));

        sandbox.clear_hazard_kind(HazardKind::Fire);

        let state = sandbox.state();
        assert_eq!(state.hazard_count(HazardKind::Fire), 0);
        assert_eq!(state.hazard_count(HazardKind::Frost), 1);
    }

    #[test]
    fn chunk_summary() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(0, 0, 0),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(1, 0, 0),
            HazardKind::Fire,
            0.5,
        ));

        let summary = sandbox.chunk_summary(ChunkPos::new(0, 0, 0));
        assert_eq!(summary.active_hazards, 2);
        assert_eq!(summary.hazards_by_kind[HazardKind::Fire.as_index()], 2);
    }

    #[test]
    fn step_result_accessors() {
        let mut sandbox = ScenarioSandbox::new(0);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));

        let result = sandbox.step(1.0);
        assert!(result.had_changes);
        assert!(result.changed_chunk_count() > 0);
    }
}

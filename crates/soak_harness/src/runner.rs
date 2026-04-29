//! Soak test runner implementation.

use std::time::{Duration, Instant};

use engine_core::coords::WorldPos;
use engine_world::{HazardKind, SandboxConfig, ScenarioSandbox, SpawnCommand};

use crate::config::SoakConfig;
use crate::invariant::{Invariant, InvariantViolation};
use crate::report::{CheckpointReport, FinalReport, OutputFormat, ReportBuilder};

/// Current state of the soak runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoakRunnerState {
    /// Not started.
    Idle,
    /// Running simulation.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed due to violations or timeout.
    Failed,
    /// Aborted by user or external signal.
    Aborted,
}

/// Headless soak test runner.
///
/// Executes deterministic simulation for a configurable number of ticks,
/// performing periodic checksum verification and invariant checking.
pub struct SoakRunner {
    config: SoakConfig,
    sandbox: ScenarioSandbox,
    invariant: Invariant,
    report_builder: ReportBuilder,
    state: SoakRunnerState,
    violations: Vec<InvariantViolation>,
    last_checkpoint_tick: u64,
    output_format: OutputFormat,
}

impl SoakRunner {
    /// Create a new runner with the given config.
    #[must_use]
    pub fn new(config: SoakConfig) -> Self {
        let sandbox_config = SandboxConfig {
            seed: config.seed,
            auto_create_chunks: true,
            record_history: false,
            default_dt: config.dt,
            ..SandboxConfig::default()
        };

        let sandbox = ScenarioSandbox::with_config(sandbox_config);
        let invariant = Invariant::new();
        let report_builder = ReportBuilder::new(config.seed, config.tick_count);

        Self {
            config,
            sandbox,
            invariant,
            report_builder,
            state: SoakRunnerState::Idle,
            violations: Vec::new(),
            last_checkpoint_tick: 0,
            output_format: OutputFormat::Text,
        }
    }

    /// Set output format for reports.
    pub fn set_output_format(&mut self, format: OutputFormat) {
        self.output_format = format;
    }

    /// Get the current runner state.
    #[must_use]
    pub fn state(&self) -> SoakRunnerState {
        self.state
    }

    /// Get the current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.sandbox.current_tick()
    }

    /// Get violations collected so far.
    #[must_use]
    pub fn violations(&self) -> &[InvariantViolation] {
        &self.violations
    }

    /// Initialize the simulation with regions and hazards.
    pub fn initialize(&mut self) {
        let regions = &self.config.regions;
        let half_x = regions.grid_size[0] / 2;
        let half_y = regions.grid_size[1] / 2;
        let half_z = regions.grid_size[2] / 2;

        let mut rng_state = self.config.seed;

        for dx in 0..regions.grid_size[0] {
            for dy in 0..regions.grid_size[1] {
                for dz in 0..regions.grid_size[2] {
                    let cx = regions.center[0] + dx - half_x;
                    let cy = regions.center[1] + dy - half_y;
                    let cz = regions.center[2] + dz - half_z;

                    if regions.spawn_hazards {
                        #[allow(clippy::cast_possible_truncation)]
                        let local_x = (rng_state & 0xF) as i32;
                        #[allow(clippy::cast_possible_truncation)]
                        let local_y = ((rng_state >> 4) & 0xF) as i32;
                        #[allow(clippy::cast_possible_truncation)]
                        let local_z = ((rng_state >> 8) & 0xF) as i32;
                        rng_state = rng_state
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1);

                        let world_x = cx * 16 + local_x;
                        let world_y = cy * 16 + local_y;
                        let world_z = cz * 16 + local_z;
                        let pos = WorldPos::new(world_x, world_y, world_z);

                        #[allow(clippy::cast_possible_truncation)]
                        let kind_idx = (rng_state % HazardKind::COUNT as u64) as usize;
                        rng_state = rng_state
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1);
                        let kind = HazardKind::ALL[kind_idx];

                        self.sandbox.execute(SpawnCommand::hazard(
                            pos,
                            kind,
                            regions.hazard_intensity,
                        ));
                    }
                }
            }
        }
    }

    /// Run the complete soak test.
    pub fn run(&mut self) -> FinalReport {
        self.state = SoakRunnerState::Running;
        self.initialize();

        let start_time = Instant::now();
        let max_duration = if self.config.max_duration_secs > 0 {
            Some(Duration::from_secs(self.config.max_duration_secs))
        } else {
            None
        };

        for tick in 1..=self.config.tick_count {
            if let Some(max) = max_duration
                && start_time.elapsed() > max
            {
                self.report_builder
                    .set_termination_reason("max duration exceeded");
                self.state = SoakRunnerState::Failed;
                break;
            }

            let result = self.sandbox.step(self.config.dt);
            let sandbox_state = self.sandbox.state();

            self.report_builder
                .tick_completed(tick, result.overall_checksum);
            self.report_builder.update_peaks(
                sandbox_state.total_active_hazards,
                sandbox_state.chunk_count,
            );

            if self.config.check_invariants {
                let tick_violations = self
                    .invariant
                    .check(&sandbox_state, result.overall_checksum);

                if !tick_violations.is_empty() {
                    let has_critical = tick_violations.iter().any(InvariantViolation::is_critical);
                    self.report_builder.add_violations(tick_violations.clone());
                    self.violations.extend(tick_violations);

                    if self.config.fail_fast && has_critical {
                        self.report_builder
                            .set_termination_reason("critical invariant violation (fail_fast)");
                        self.state = SoakRunnerState::Failed;
                        break;
                    }

                    if self.violations.len() >= self.config.max_violations {
                        self.report_builder
                            .set_termination_reason("max violations exceeded");
                        self.state = SoakRunnerState::Failed;
                        break;
                    }
                }
            }

            if self.config.checkpoint_interval > 0 && tick % self.config.checkpoint_interval == 0 {
                let checkpoint = self.create_checkpoint(tick, &start_time);
                self.report_builder.add_checkpoint(checkpoint.checksum);

                if self.config.verbose {
                    let output = match self.output_format {
                        OutputFormat::Text => checkpoint.to_text(),
                        OutputFormat::Json => checkpoint
                            .to_json()
                            .unwrap_or_else(|e| format!("error: {e}")),
                    };
                    println!("{output}");
                }

                self.last_checkpoint_tick = tick;
            }
        }

        if self.state == SoakRunnerState::Running {
            self.state = if self.violations.iter().any(InvariantViolation::is_critical) {
                SoakRunnerState::Failed
            } else {
                SoakRunnerState::Completed
            };
        }

        std::mem::replace(
            &mut self.report_builder,
            ReportBuilder::new(self.config.seed, self.config.tick_count),
        )
        .build()
    }

    /// Run a determinism check by executing twice and comparing checksums.
    pub fn run_determinism_check(&mut self) -> (FinalReport, Option<InvariantViolation>) {
        self.initialize();

        let mut checksums_a = Vec::new();
        for _ in 1..=self.config.tick_count {
            let result = self.sandbox.step(self.config.dt);
            checksums_a.push(result.overall_checksum);
        }

        self.sandbox.reset();
        self.initialize();

        let mut determinism_violation = None;
        for (tick, expected) in checksums_a.iter().enumerate() {
            let result = self.sandbox.step(self.config.dt);
            let sandbox_state = self.sandbox.state();
            self.report_builder.update_peaks(
                sandbox_state.total_active_hazards,
                sandbox_state.chunk_count,
            );
            if let Some(v) =
                Invariant::check_determinism(tick as u64 + 1, *expected, result.overall_checksum)
            {
                determinism_violation = Some(v);
                self.report_builder
                    .set_termination_reason("determinism check failed");
                break;
            }
            self.report_builder
                .tick_completed(tick as u64 + 1, result.overall_checksum);
        }

        self.state = if determinism_violation.is_some() {
            SoakRunnerState::Failed
        } else {
            SoakRunnerState::Completed
        };

        let report = std::mem::replace(
            &mut self.report_builder,
            ReportBuilder::new(self.config.seed, self.config.tick_count),
        )
        .build();

        (report, determinism_violation)
    }

    #[allow(clippy::cast_precision_loss)]
    fn create_checkpoint(&self, tick: u64, start_time: &Instant) -> CheckpointReport {
        let elapsed = start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        let ticks_per_sec = if elapsed_secs > 0.0 {
            tick as f64 / elapsed_secs
        } else {
            0.0
        };

        let sandbox_state = self.sandbox.state();
        let snapshot = self.sandbox.snapshot();

        let violations_in_interval = self
            .violations
            .iter()
            .filter(|v| v.tick > self.last_checkpoint_tick && v.tick <= tick)
            .count();

        CheckpointReport {
            tick,
            elapsed_secs,
            ticks_per_sec,
            checksum: snapshot.checksum.value(),
            active_hazards: sandbox_state.total_active_hazards,
            chunk_count: sandbox_state.chunk_count,
            violations_in_interval,
            total_violations: self.violations.len(),
        }
    }
}

/// Run a soak test with the given config and return the report.
pub fn run_soak(config: SoakConfig) -> FinalReport {
    let mut runner = SoakRunner::new(config);
    runner.run()
}

/// Run a determinism check with the given config.
pub fn run_determinism_check(config: SoakConfig) -> (FinalReport, Option<InvariantViolation>) {
    let mut runner = SoakRunner::new(config);
    runner.run_determinism_check()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_new() {
        let config = SoakConfig::smoke();
        let runner = SoakRunner::new(config);
        assert_eq!(runner.state(), SoakRunnerState::Idle);
        assert_eq!(runner.current_tick(), 0);
    }

    #[test]
    fn runner_smoke_test() {
        let config = SoakConfig::smoke();
        let mut runner = SoakRunner::new(config);
        let report = runner.run();

        assert_eq!(runner.state(), SoakRunnerState::Completed);
        assert_eq!(report.ticks_completed, 100);
        assert!(report.passed || report.total_violations > 0);
    }

    #[test]
    fn runner_determinism() {
        let config = SoakConfig {
            tick_count: 50,
            ..SoakConfig::smoke()
        };
        let mut runner = SoakRunner::new(config);
        let (report, violation) = runner.run_determinism_check();

        assert!(violation.is_none(), "determinism check should pass");
        assert!(report.ticks_completed > 0);
    }

    #[test]
    fn runner_with_regions() {
        let config = SoakConfig {
            tick_count: 50,
            regions: crate::config::RegionSetup::small(),
            ..SoakConfig::smoke()
        };
        let mut runner = SoakRunner::new(config);
        let report = runner.run();

        assert!(report.peak_chunks > 0);
        assert!(report.peak_hazards > 0);
    }

    #[test]
    fn runner_checkpoint_interval() {
        let config = SoakConfig {
            tick_count: 100,
            checkpoint_interval: 25,
            verbose: false,
            ..SoakConfig::smoke()
        };
        let mut runner = SoakRunner::new(config);
        let report = runner.run();

        assert_eq!(report.checkpoints.len(), 4);
    }

    #[test]
    fn runner_fail_fast() {
        let config = SoakConfig {
            tick_count: 1000,
            check_invariants: true,
            fail_fast: true,
            max_violations: 1,
            ..SoakConfig::smoke()
        };

        let report = run_soak(config);
        assert!(report.ticks_completed <= 1000);
    }

    #[test]
    fn run_soak_convenience() {
        let config = SoakConfig::smoke();
        let report = run_soak(config);
        assert!(report.ticks_completed > 0);
    }

    #[test]
    fn output_format_json() {
        let config = SoakConfig {
            tick_count: 10,
            checkpoint_interval: 5,
            verbose: false,
            ..SoakConfig::smoke()
        };
        let mut runner = SoakRunner::new(config);
        runner.set_output_format(OutputFormat::Json);
        let report = runner.run();

        let json = report.to_json().unwrap();
        assert!(json.contains("\"passed\""));
    }
}

//! Headless simulation harness for overnight soak tests.
//!
//! Provides deterministic soak testing of the simulation pipeline without
//! GPU or windowing dependencies. Configurable seed, tick count, and region
//! setup with periodic checksum verification and invariant detection.

mod config;
mod invariant;
mod report;
mod runner;

pub use config::{RegionSetup, SoakConfig};
pub use invariant::{Invariant, InvariantKind, InvariantViolation};
pub use report::{CheckpointReport, FinalReport, OutputFormat, TickSummary};
pub use runner::{SoakRunner, SoakRunnerState, run_determinism_check, run_soak};

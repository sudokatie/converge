//! Region-scope simulation scheduler with distance-based fidelity scaling.
//!
//! This module provides a scheduler for chunk/region simulation that adjusts
//! simulation fidelity based on distance from observers. Closer regions receive
//! higher-fidelity simulation (more frequent updates), while distant regions
//! run at reduced rates or become dormant.
//!
//! # Architecture
//!
//! - [`Fidelity`]: Discrete simulation quality levels (Immediate, Near, Distant, Dormant)
//! - [`FidelityConfig`]: Per-level tick intervals and distance thresholds
//! - [`RegionState`]: Tracks accumulated time and metadata for each region
//! - [`SimulationJob`]: A batch of work ready to execute
//! - [`SimulationScheduler`]: Central coordinator managing all regions
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::scheduler::{SimulationScheduler, SchedulerConfig, Fidelity};
//! use engine_core::coords::ChunkPos;
//!
//! let config = SchedulerConfig::default();
//! let mut scheduler = SimulationScheduler::new(config);
//!
//! // Register chunks for simulation
//! scheduler.add_region(ChunkPos::new(0, 0, 0));
//! scheduler.add_region(ChunkPos::new(1, 0, 0));
//!
//! // Update observer position
//! scheduler.set_observer(ChunkPos::new(0, 0, 0));
//!
//! // Accumulate time and get jobs ready to run
//! let jobs = scheduler.tick(0.016); // 16ms frame
//!
//! for job in jobs {
//!     // Process simulation at job.fidelity for job.position
//! }
//! ```

mod config;
mod fidelity;
mod job;
mod scheduler;
mod state;

pub use config::{FidelityThresholds, SchedulerConfig, TickIntervals};
pub use fidelity::Fidelity;
pub use job::{EnvironmentHint, SimulationJob};
pub use scheduler::SimulationScheduler;
pub use state::RegionState;

//! Scenario sandbox for spawning hazards and stepping simulation deterministically.
//!
//! Provides a self-contained simulation environment for testing hazard propagation,
//! environmental effects, and world events without requiring a full game world.
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::sandbox::{ScenarioSandbox, SpawnCommand};
//! use engine_world::HazardKind;
//! use engine_core::coords::{ChunkPos, LocalPos, WorldPos};
//!
//! let mut sandbox = ScenarioSandbox::new(42);
//!
//! // Spawn a fire hazard
//! sandbox.execute(SpawnCommand::hazard(WorldPos::new(8, 8, 8), HazardKind::Fire, 1.0));
//!
//! // Step simulation forward
//! for _ in 0..10 {
//!     let result = sandbox.step(0.1);
//!     println!("tick {}: {} changes", result.tick, result.stats.spread_count);
//! }
//!
//! // Query state
//! let snapshot = sandbox.hazard_snapshot();
//! ```

mod command;
mod config;
mod runtime;
mod snapshot;

pub use command::{CommandResult, SpawnCommand, SpawnKind};
pub use config::SandboxConfig;
pub use runtime::{ScenarioSandbox, StepResult};
pub use snapshot::{ChunkSummary, SandboxSnapshot, SandboxState};

//! Voxel world system for the Lattice game engine.
//!
//! Provides chunk management, terrain generation, and world persistence.

pub mod chunk;
pub mod environment;
pub mod generation;
pub mod manager;
pub mod persistence;
pub mod scheduler;

// Re-export environment API at crate root for convenience
pub use environment::{
    AdvectionConfig, ChannelData, ChunkFields, ChunkVectorFields, DiffusionConfig, DiffusionStep,
    FieldChannel, FieldSimConfig, SimStepResult, VectorAdvectionConfig, VectorChannelData,
    VectorDecayConfig, VectorFieldChannel, VectorFieldSimConfig, VectorSmoothingConfig,
};

// Re-export scheduler API at crate root for convenience
pub use scheduler::{
    EnvironmentHint, Fidelity, FidelityThresholds, RegionState, SchedulerConfig, SimulationJob,
    SimulationScheduler, TickIntervals,
};

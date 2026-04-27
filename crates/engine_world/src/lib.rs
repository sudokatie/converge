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
    AdvectionConfig, AtmosphereCell, AtmosphereConfig, AtmosphereEffects, AtmosphereLayer,
    AtmosphereSample, BoundaryOutflow, ChannelData, ChunkAtmosphere, ChunkFields, ChunkFluids,
    ChunkHazards, ChunkVectorFields, DecayConfig, DiffusionConfig, DiffusionStep, FieldChannel,
    FieldSimConfig, FluidCell, FluidDelta, FluidKind, FluidLayer, FluidResistanceMap, FluidSample,
    FluidTransportConfig, FluidTransportResult, HazardCell, HazardKind, HazardLayer,
    LayerNeighborCounts, MaterialCategory, MaterialId, MaterialProperties, MaterialRegistry,
    MaterialRegistryError, PropagationConfig, PropagationResult, Resistance, SimStepResult,
    SpreadConfig, TransitionRules, VectorAdvectionConfig, VectorChannelData, VectorDecayConfig,
    VectorFieldChannel, VectorFieldSimConfig, VectorSmoothingConfig, apply_fluid_deltas,
    cell_from_material, hazard_integration, layer_from_material, transport_step,
};

// Re-export scheduler API at crate root for convenience
pub use scheduler::{
    EnvironmentHint, Fidelity, FidelityThresholds, RegionState, SchedulerConfig, SimulationJob,
    SimulationScheduler, TickIntervals,
};

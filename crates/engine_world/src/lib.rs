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
    ChunkHazards, ChunkStructural, ChunkVectorFields, DecayConfig, DiffusionConfig, DiffusionStep,
    FieldChannel, FieldSimConfig, FluidCell, FluidDelta, FluidKind, FluidLayer, FluidResistanceMap,
    FluidSample, FluidTransportConfig, FluidTransportResult, HazardCell, HazardKind, HazardLayer,
    LayerNeighborCounts, LoadConfig, MaterialCategory, MaterialId, MaterialProperties,
    MaterialRegistry, MaterialRegistryError, PressureMap, PropagationConfig, PropagationResult,
    Resistance, SimStepResult, SpreadConfig, StabilityConfig, StrengthMap, StructuralBoundary,
    StructuralCell, StructuralConfig, StructuralDelta, StructuralEvent, StructuralEventKind,
    StructuralResult, SupportKind, SupportPropagationConfig, TransitionRules,
    VectorAdvectionConfig, VectorChannelData, VectorDecayConfig, VectorFieldChannel,
    VectorFieldSimConfig, VectorSmoothingConfig, apply_fluid_deltas, apply_structural_deltas,
    cell_from_material, check_decompression, detect_cavein, distribute_load, hazard_integration,
    layer_from_material, propagate_support, structural_step, transport_step,
};

// Re-export scheduler API at crate root for convenience
pub use scheduler::{
    EnvironmentHint, Fidelity, FidelityThresholds, RegionState, SchedulerConfig, SimulationJob,
    SimulationScheduler, TickIntervals,
};

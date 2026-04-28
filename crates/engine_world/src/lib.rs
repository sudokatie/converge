//! Voxel world system for the Lattice game engine.
//!
//! Provides chunk management, terrain generation, and world persistence.

pub mod chunk;
pub mod environment;
pub mod generation;
pub mod manager;
pub mod persistence;
pub mod region_journal;
pub mod replay;
pub mod scheduler;
pub mod world_state;

// Re-export environment API at crate root for convenience
pub use environment::{
    AdvectionConfig, AtmosphereCell, AtmosphereConfig, AtmosphereEffects, AtmosphereLayer,
    AtmosphereSample, BoundaryOutflow, ChannelData, ChunkAtmosphere, ChunkConduits, ChunkFields,
    ChunkFluids, ChunkHazards, ChunkStructural, ChunkVectorFields, ConduitBoundary, ConduitCell,
    ConduitDelta, ConduitKind, ConduitLayer, ConduitNetworkConfig, ConduitNetworkResult,
    ConduitNode, ConduitResistanceMap, ConnectedNetwork, DecayConfig, DiffusionConfig,
    DiffusionStep, FieldChannel, FieldSimConfig, FlowConfig, FluidCell, FluidDelta, FluidKind,
    FluidLayer, FluidResistanceMap, FluidSample, FluidTransportConfig, FluidTransportResult,
    GravityModel, GravityProfile, HazardCell, HazardKind, HazardLayer, HeatTransferConfig,
    LayerNeighborCounts, LoadConfig, MAX_GRAVITY_MAGNITUDE, MIN_GRAVITY_MAGNITUDE,
    MaterialCategory, MaterialId, MaterialProperties, MaterialRegistry, MaterialRegistryError,
    NodeRole, PressureConfig, PressureMap, ProfileError, ProfileId, ProfileRegistry,
    PropagationConfig, PropagationResult, Resistance, RuleBundle, RuleOverrides, STANDARD_GRAVITY,
    SimStepResult, SpreadConfig, StabilityConfig, StrengthMap, StructuralBoundary, StructuralCell,
    StructuralConfig, StructuralDelta, StructuralEvent, StructuralEventKind, StructuralResult,
    SupportKind, SupportPropagationConfig, TransitionRules, VectorAdvectionConfig,
    VectorChannelData, VectorDecayConfig, VectorFieldChannel, VectorFieldSimConfig,
    VectorSmoothingConfig, WorldRuleProfile, apply_conduit_deltas, apply_fluid_deltas,
    apply_structural_deltas, cell_from_material, check_decompression, detect_cavein,
    distribute_load, find_boundary_cells, find_networks, hazard_integration, layer_from_material,
    network_step, propagate_support, structural_step, transport_step,
};

// Re-export scheduler API at crate root for convenience
pub use scheduler::{
    EnvironmentHint, Fidelity, FidelityThresholds, RegionState, SchedulerConfig, SimulationJob,
    SimulationScheduler, TickIntervals,
};

// Re-export world_state API at crate root for convenience
pub use world_state::{
    ActiveEffect, ActiveEffects, EntityHint, HazardHint, LightingHint, Season, StructuralHint,
    TemperatureHint, TimelineConfig, WorldEvent, WorldEventKind, WorldStateHints, WorldTimeline,
};

// Re-export replay API at crate root for convenience
pub use replay::{
    ChecksumBuilder, Mismatch, MismatchKind, ReplayEntry, ReplayEntryKind, ReplayMetadata,
    ReplayRecorder, ReplayVerifier, StepChecksum,
};

// Re-export region_journal API at crate root for convenience
pub use region_journal::{
    CategoryStats, EventCategory, EventKind, EventPayload, EventRecord, JournalQuery,
    RecoverySummary, RegionJournal, RegionSummary, Severity,
};

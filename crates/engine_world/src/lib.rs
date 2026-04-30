//! Voxel world system for the Lattice game engine.
//!
//! Provides chunk management, terrain generation, and world persistence.

pub mod automation;
pub mod behavior_graph;
pub mod chunk;
pub mod diagnostics;
pub mod environment;
pub mod generation;
pub mod machine;
pub mod manager;
pub mod megastructure;
pub mod persistence;
pub mod region_journal;
pub mod replay;
pub mod sandbox;
pub mod scheduler;
pub mod world_state;

// Re-export environment API at crate root for convenience
pub use environment::{
    AdvectionConfig, AtmosphereCell, AtmosphereConfig, AtmosphereEffects, AtmosphereLayer,
    AtmosphereSample, BoundaryOutflow, BoundarySpread, ChannelData, ChunkAtmosphere, ChunkConduits,
    ChunkFields, ChunkFluids, ChunkHazardDelta, ChunkHazardSnapshot, ChunkHazards, ChunkStructural,
    ChunkTickInput, ChunkTickOutput, ChunkVectorFields, ConduitBoundary, ConduitCell, ConduitDelta,
    ConduitKind, ConduitLayer, ConduitNetworkConfig, ConduitNetworkResult, ConduitNode,
    ConduitResistanceMap, ConnectedNetwork, DecayConfig, DiffusionConfig, DiffusionStep,
    FieldChannel, FieldSimConfig, FlowConfig, FluidCell, FluidDelta, FluidKind, FluidLayer,
    FluidResistanceMap, FluidSample, FluidTransportConfig, FluidTransportResult, GravityModel,
    GravityProfile, HazardCell, HazardCellDelta, HazardDeltaJournal, HazardDeltaRecord, HazardKind,
    HazardLayer, HazardSimulator, HazardSnapshot, HeatTransferConfig, LayerNeighborCounts,
    LoadConfig, MAX_GRAVITY_MAGNITUDE, MIN_GRAVITY_MAGNITUDE, MaterialCategory, MaterialId,
    MaterialProperties, MaterialRegistry, MaterialRegistryError, NodeRole, PressureConfig,
    PressureMap, ProfileError, ProfileId, ProfileRegistry, PropagationConfig, PropagationResult,
    Resistance, RuleBundle, RuleOverrides, STANDARD_GRAVITY, SimStepResult, SimulationTickResult,
    SpreadConfig, StabilityConfig, StrengthMap, StructuralBoundary, StructuralCell,
    StructuralConfig, StructuralDelta, StructuralEvent, StructuralEventKind, StructuralResult,
    SupportKind, SupportPropagationConfig, TickStats, TransitionRules, VectorAdvectionConfig,
    VectorChannelData, VectorDecayConfig, VectorFieldChannel, VectorFieldSimConfig,
    VectorSmoothingConfig, WorldRuleProfile, apply_chunk_delta, apply_conduit_deltas,
    apply_fluid_deltas, apply_snapshot, apply_structural_deltas, cell_from_material,
    check_decompression, detect_cavein, distribute_load, find_boundary_cells, find_networks,
    hazard_integration, layer_from_material, network_step, propagate_support, simulate_chunk_tick,
    structural_step, transport_step,
};

// Re-export scheduler API at crate root for convenience
pub use scheduler::{
    EnvironmentHint, Fidelity, FidelityThresholds, InterestCategory, InterestConfig,
    InterestCounts, InterestEntry, InterestSummary, RegionInterest, RegionState, SchedulerConfig,
    SimulationJob, SimulationScheduler, TickIntervals,
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

// Re-export megastructure API at crate root for convenience
pub use megastructure::{
    AnchorMetadata, ChunkBounds, ChunkMask, ChunkSlice, IdGenerator, ManifestEntry, Megastructure,
    MegastructureId, MegastructureRegistry, SliceMap, SliceState, StreamingManifest,
    StreamingQuery, StreamingTier, StructureAnchor, StructureKind, StructureZone,
};

// Re-export mutation journal API at crate root for convenience
pub use persistence::{
    JournalSnapshot, JournalStats, MutationJournal, MutationQuery, MutationReason, MutationRecord,
    MutationSource, Sequence,
};

// Re-export automation API at crate root for convenience
pub use automation::{
    AutomationConfig, AutomationDeltaBatch, AutomationDevice, AutomationLink, AutomationNetwork,
    AutomationSnapshot, ChangeKind, ChangePayload, DeviceChangePayload, DeviceConfig, DeviceDelta,
    DeviceId, DeviceKind, GateConfig, GateOp, LinkChangePayload, LinkId, MAX_PORTS, PendingSignal,
    PortId, PortState, PrimitiveConfig, PrimitiveState, PumpConfig, PumpMode, PumpState,
    RelayConfig, RelayMode, RelayState, Revision, RevisionTracker, SensorConfig, SensorState,
    SensorType, SignalValue, SpatialFilter, StateChange, TickResult, TimerConfig, TimerMode,
    TimerState, ValveConfig, ValveMode, ValveState,
};

// Re-export diagnostics API at crate root for convenience
pub use diagnostics::{
    CategoryCounts, ChannelPalette, ChannelStats, DiagnosticCategory, DiagnosticChannel,
    DiagnosticColor, DiagnosticFilter, DiagnosticFingerprint, DiagnosticLegend, DiagnosticSummary,
    FilterMode, LegendEntry, MarkerKind, OverlayMarker, OverlaySpec, SampleCell, SampleGrid,
    ScalarValue, VectorValue,
};

// Re-export sandbox API at crate root for convenience
pub use sandbox::{
    ChunkSummary, CommandResult, SandboxConfig, SandboxSnapshot, SandboxState, ScenarioSandbox,
    SpawnCommand, SpawnKind, StepResult,
};

// Re-export behavior_graph API at crate root for convenience
pub use behavior_graph::{
    BehaviorAction, BehaviorCondition, BehaviorEffect, BehaviorGraph, BehaviorNode,
    BehaviorTrigger, BlockFilter, CompareOp, EffectKind, EvalResult, EvaluatorConfig,
    EvaluatorStats, GraphEvaluator, GraphFingerprint, NodeId, TriggerContext, TriggerEvent,
};

// Re-export entity module at crate root for convenience
pub mod entity;
pub use entity::{
    EquipmentFingerprint, EquipmentLoadout, EquipmentModule, FilterType, GrappleType, LoadoutError,
    LoadoutTickResult, MAX_MODULES, ModuleCategory, ModuleConfig, ModuleEffect, ModuleId,
    ModuleStatus, ModuleTickResult, ModuleTier, ResourceState, StatusEffect, StatusEffectManager,
    StatusEffectType, TankContent,
};

// Re-export machine API at crate root for convenience
pub use machine::{
    AtmosphereEffect, FaultKind, FluidPort, HeatConfig, MachineCategory, MachineConfig,
    MachineEvent, MachineEventKind, MachineFingerprint, MachineId, MachineRegistry, MachineState,
    MachineTickResult, MachineTickStats, MachineTier, MaintenanceState, PortDirection, PowerConfig,
    ProcessDefinition, ProcessId, ProcessQueue, ProcessState, QueuedProcess, RegistryError,
    RegistryQuery, RegistrySummary, ResourceRequirement, ResourceYield,
};

//! Voxel world system for the Lattice game engine.
//!
//! Provides chunk management, terrain generation, and world persistence.

pub mod automation;
pub mod behavior_graph;
pub mod chunk;
pub mod diagnostics;
pub mod environment;
pub mod generation;
pub mod geology;
pub mod machine;
pub mod manager;
pub mod megastructure;
pub mod mission;
pub mod narrative;
pub mod persistence;
pub mod region_graph;
pub mod region_journal;
pub mod replay;
pub mod sandbox;
pub mod scheduler;
pub mod world_state;

// Re-export environment API at crate root for convenience
pub use environment::{
    AdvectionConfig,
    AtmosphereCell,
    AtmosphereConfig,
    AtmosphereEffects,
    AtmosphereLayer,
    AtmosphereSample,
    // Cellular automata
    AutomataCell,
    AutomataConfig,
    AutomataDelta,
    AutomataEntry,
    AutomataFingerprint,
    AutomataKind,
    AutomataPlan,
    AutomataPos,
    AutomataRegion,
    AutomataResistance,
    AutomataResult,
    AutomataRule,
    AutomataSummary,
    AutomataValidation,
    // Regional climate
    BiomeType,
    BoundaryOutflow,
    BoundarySpread,
    CLIMATE_MAX_PRESSURE,
    CLIMATE_MAX_TEMP,
    CLIMATE_MIN_PRESSURE,
    CLIMATE_MIN_TEMP,
    ChannelData,
    ChunkAtmosphere,
    ChunkConduits,
    ChunkFields,
    ChunkFluids,
    ChunkHazardDelta,
    ChunkHazardSnapshot,
    ChunkHazards,
    ChunkStructural,
    ChunkTickInput,
    ChunkTickOutput,
    ChunkVectorFields,
    ClimateCell,
    ClimateProjection,
    ClimateRegion,
    ClimateRegionId,
    ConduitBoundary,
    ConduitCell,
    ConduitDelta,
    ConduitKind,
    ConduitLayer,
    ConduitNetworkConfig,
    ConduitNetworkResult,
    ConduitNode,
    ConduitResistanceMap,
    ConnectedNetwork,
    // Thermal radiation
    DEFAULT_AMBIENT,
    DEFAULT_BASE_HUMIDITY,
    DEFAULT_BASE_MOISTURE,
    DEFAULT_BASE_PRESSURE,
    DEFAULT_BASE_TEMPERATURE,
    // Deformable terrain
    DEFAULT_DUCTILITY,
    DEFAULT_FRACTURE_THRESHOLD,
    DEFAULT_HARDNESS,
    DEFAULT_THERMAL_MASS,
    DecayConfig,
    DeformableTerrainConfig,
    DeformableTerrainFingerprint,
    DeformableTerrainRegion,
    DeformableTerrainResult,
    DeformableTerrainSummary,
    DeformableTerrainValidation,
    DeltaKind,
    DiffusionConfig,
    DiffusionStep,
    FieldChannel,
    FieldSimConfig,
    FlowConfig,
    FlowLink,
    FluidCell,
    FluidDelta,
    FluidKind,
    FluidLayer,
    FluidResistanceMap,
    FluidSample,
    FluidTransportConfig,
    FluidTransportResult,
    FractureEvent,
    FractureLink,
    GravityModel,
    GravityProfile,
    HazardCell,
    HazardCellDelta,
    HazardDeltaJournal,
    HazardDeltaRecord,
    HazardKind,
    HazardLayer,
    HazardSimulator,
    HazardSnapshot,
    HeatTransferConfig,
    KELVIN_OFFSET,
    LayerNeighborCounts,
    LoadConfig,
    MAX_DAMAGE,
    MAX_DEFORMATION,
    MAX_DUCTILITY,
    MAX_EMISSIVITY,
    MAX_FRACTURE_THRESHOLD,
    MAX_GRAVITY_MAGNITUDE,
    MAX_HARDNESS,
    MAX_HUMIDITY,
    MAX_MOISTURE,
    MAX_STRAIN,
    MIN_DUCTILITY,
    MIN_EMISSIVITY,
    MIN_FRACTURE_THRESHOLD,
    MIN_GRAVITY_MAGNITUDE,
    MIN_HARDNESS,
    MIN_HUMIDITY,
    MIN_MOISTURE,
    MaterialCategory,
    MaterialId,
    MaterialProperties,
    MaterialRegistry,
    MaterialRegistryError,
    MoistureTransport,
    Neighborhood,
    NodeRole,
    PrecipitationEvent,
    PressureConfig,
    PressureEqualizationPlan,
    PressureEqualizationStep,
    PressureMap,
    ProfileError,
    ProfileId,
    ProfileRegistry,
    PropagationConfig,
    PropagationResult,
    RadiationExchange,
    RegionNeighbors,
    RegionalClimateConfig,
    RegionalClimateFingerprint,
    RegionalClimateResult,
    RegionalClimateSnapshot,
    RegionalClimateSummary,
    RegionalClimateValidation,
    Resistance,
    RuleBundle,
    RuleOverrides,
    STANDARD_GRAVITY,
    STEFAN_BOLTZMANN,
    SeasonalCycle,
    SimStepResult,
    SimulationTickResult,
    SparseFluidConfig,
    SparseFluidEntry,
    SparseFluidFingerprint,
    SparseFluidPos,
    SparseFluidRegion,
    SparseFluidResult,
    SparseFluidSummary,
    SparseFluidValidation,
    SpreadConfig,
    StabilityConfig,
    StrengthMap,
    StressPropagation,
    StructuralBoundary,
    StructuralCell,
    StructuralConfig,
    StructuralDelta,
    StructuralEvent,
    StructuralEventKind,
    StructuralResult,
    SupportKind,
    SupportPropagationConfig,
    THERMAL_MAX_TEMP,
    THERMAL_MIN_TEMP,
    TerrainCell,
    TerrainEntry,
    TerrainPos,
    ThermalCell,
    ThermalEntry,
    ThermalPos,
    ThermalRadiationConfig,
    ThermalRadiationFingerprint,
    ThermalRadiationRegion,
    ThermalRadiationResult,
    ThermalRadiationSummary,
    ThermalRadiationValidation,
    TickStats,
    TransitionRules,
    VectorAdvectionConfig,
    VectorChannelData,
    VectorDecayConfig,
    VectorFieldChannel,
    VectorFieldSimConfig,
    VectorSmoothingConfig,
    WindVector,
    WorldRuleProfile,
    apply_ambient_exchange,
    apply_automata_plan,
    apply_chunk_delta,
    apply_conduit_deltas,
    // Deformable terrain functions
    apply_deformation_from_damage,
    apply_equalization,
    apply_evaporation,
    apply_flows,
    apply_fluid_deltas,
    apply_moisture_transports,
    apply_precipitation,
    apply_radiation_exchanges,
    apply_seasonal_temperature,
    apply_snapshot,
    apply_stress_propagation,
    apply_stress_relaxation,
    apply_structural_deltas,
    automata_step,
    cell_from_material,
    check_decompression,
    check_fractures,
    compute_flow_links,
    compute_precipitation,
    compute_radiation_transfer,
    deformable_terrain_step,
    detect_cavein,
    distribute_load,
    find_boundary_cells,
    find_networks,
    hazard_integration,
    layer_from_material,
    network_step,
    plan_automata_step,
    plan_moisture_transports,
    plan_pressure_equalization,
    plan_radiation_exchanges,
    plan_stress_propagation,
    propagate_fractures,
    propagate_support,
    regional_climate_step,
    simulate_chunk_tick,
    sparse_fluid_step,
    structural_step,
    thermal_radiation_step,
    transport_step,
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

// Re-export schema migration API at crate root for convenience
pub use persistence::{
    ChunkFixture, CompatibilityReport, InvariantCheck, InvariantKind, InvariantResult, MetaFixture,
    MigrationError, MigrationExecutor, MigrationFixture, MigrationKind, MigrationPlan,
    MigrationResult, MigrationStep, MigrationStepResult, MultiStateFixture, SchemaVersion,
    apply_block_remap, apply_block_remap_delta, compute_plan_fingerprint,
};

// Re-export admin tools API at crate root for convenience
pub use persistence::{
    AdminLog, AdminLogStats, AdminMetadata, AdminOp, AdminOpId, AdminQuery, AdminRecord, AuthLevel,
    BlockFillSpec, BlockReplaceSpec, DryRunResult, MAX_BLOCK_REGION_SIZE, MAX_REGION_BOUND_CHUNKS,
    MarkerCategory, ModerationAction, OpCategory, OpOutcome, PlayerModerationRecord,
    QuarantineSeverity, QuarantineStatus, RegionMarker, ReplayResult as AdminReplayResult,
    TeleportDestination, ValidationResult as AdminValidationResult, WorldBounds,
};

// Re-export regional backup API at crate root for convenience
pub use persistence::{
    BackupId, BackupIssue, BackupIssueKind, BackupIssueSeverity, BackupManifest, BackupMetadata,
    BackupSummary, ChunkEntry, RegionalBackup, RestoreOp, RestorePlan, RestoreResult,
    apply_restore, compute_restore_delta, verify_against_backup,
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

// Re-export narrative API at crate root for convenience
pub use narrative::{
    ActiveEvent, AnomalyPreset, CooldownConfig, CooldownState, DisasterPreset, EventDefinition,
    EventFingerprint, EventId, EventRegistry, NarrativeContext, NarrativeEventKind,
    NarrativeOutput, NarrativeState, NarrativeTrigger, ObjectivePreset, ObjectiveStatus,
    OutputKind, OutputPriority, OutputQueue, Preset, RadioPreset, RepeatMode, StateChecksum,
    TickResult as NarrativeTickResult, TimedObjective, TriggerKind, TriggerPredicate,
    TriggerResult,
};

// Re-export mission API at crate root for convenience
pub use mission::{
    ChecksumBuilder as MissionChecksumBuilder, ContractId, DeadlineConfig, ExpeditionContract,
    FactionSource, MissionChecksum, MissionDefinition, MissionEvent, MissionEventHistory,
    MissionEventKind, MissionEventPayload, MissionFingerprint, MissionId, MissionPreset,
    MissionProgress, MissionQuery, MissionState, MissionTracker, ObjectiveId, ObjectiveKind,
    ObjectiveProgress, ObjectiveSpec, ObjectiveState, PenaltyDefinition, ProjectionSummary,
    RegistryError as MissionRegistryError, RepeatConfig, RewardDefinition, RiskLevel, ScopeConfig,
    TrackerSummary, register_presets,
};

// Re-export region_graph API at crate root for convenience
pub use region_graph::{
    ConfigError as RegionGraphConfigError, EdgeId, EdgeKind,
    FingerprintBuilder as RegionFingerprintBuilder, GateKind, GateRequirement,
    GraphChecksum as RegionGraphChecksum, GraphFingerprint as RegionGraphFingerprint, GraphSummary,
    HazardAnnotation, KindWeights, MissionAnnotation, MissionRole, ProgressionTier, RegionEdge,
    RegionGraph, RegionGraphConfig, RegionId, RegionKind, RegionNode, RegionTag,
    ResourceAnnotation, TierSummary,
};

// Re-export topology API at crate root for convenience
pub use generation::{
    CellQuery, CellState, HazardType, MissionHook, NodeId as TopologyNodeId,
    NodeRole as TopologyNodeRole, PathQuery, PlannerSummary, QueryResult, ResourceType, SegmentId,
    SegmentKind, TopologyAnnotation, TopologyAnnotations, TopologyCell, TopologyChecksum,
    TopologyConfig, TopologyConfigError, TopologyFingerprint, TopologyFingerprintBuilder,
    TopologyKind, TopologyNode, TopologyPlanner, TopologySegment,
};

// Re-export structure_grammar API at crate root for convenience
pub use generation::structure_grammar::{
    Anchor, BlockPalette, BlockType, Bounds, ChildSymbol, ConnectorSummary, Direction,
    GeneratedLayout, GenerationConfig, GenerationContext, GenerationResult, GrammarBuilder,
    GrammarRule, LayoutChecksum, LayoutFingerprint, LayoutFingerprintBuilder, LayoutQuery,
    LayoutQueryResult, LayoutSummary, Placement, PlacementId, PlacementRules, RuleExpansion,
    RuleId, Socket, StructureGenerator, StructureGrammar, StructureTemplate, SymbolId, TemplateId,
    TemplateKind, ValidationError, ValidationErrors, WeightedChoice, generate, generate_with_seed,
};

// Re-export geology API at crate root for convenience
pub use geology::{
    CrystalGrowthConfig, CrystalSeam, CrystalType, FaultConfig, FaultLine, FaultType, FeatureId,
    FeatureKind, GeologicalLayer, GeologyChecksum, GeologyConfig, GeologyEvent, GeologyEventKind,
    GeologyFields, GeologyFingerprint, GeologySimulator, GeologySummary, GeologyTickResult,
    GeologyTickStats, LayerBoundary, LayerId, MagmaConfig, MagmaFlow, MagmaPocket, MagmaState,
    MaterialId as GeoMaterialId, MineralDeposit, MineralType, PressureField,
    ProjectionResult as GeologyProjectionResult, QuakeEvent, RockType, SlipState, StabilityField,
    Stratum, StressAccumulator, TemperatureField, ThermalConfig, VolcanicEvent, VolcanicEventKind,
};

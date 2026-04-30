//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, decision making, needs simulation,
//! sensory perception, faction/reputation/territory systems, goal-based
//! utility AI for survival prioritization, population director for spawn
//! pressure, pacing, migration, and regional threat scaling, offline
//! simulation for unloaded chunks, group AI primitives for packs,
//! swarms, schools, patrols, and evacuation, multi-domain navigation
//! for voxel walking, swimming, climbing, flying, zero-G, and dynamic worlds,
//! ecological simulation for food chains, resource zones, migration
//! paths, and infestation fronts, and creature memory for danger zones,
//! food sources, and player traces.

pub mod behavior;
pub mod creatures;
pub mod ecology;
pub mod faction;
pub mod goal;
pub mod group;
pub mod memory;
pub mod navigation;
pub mod needs;
pub mod offline;
pub mod pathfinding;
pub mod population;
pub mod sensor;

pub use behavior::{BehaviorNode, BehaviorTree, Blackboard, NodeStatus};
pub use creatures::{PassiveAI, PassiveState};
pub use ecology::{
    CarryingCapacityConfig, DepletionBehavior, DepletionProjection, EcologyConfig, EcologyEvent,
    EcologyEventKind, EcologyFingerprint, EcologySimulator, EcologySnapshot, EcologySummary,
    EcologyTickResult, FoodChain, HarvestPressure, InfestationFront, InfestationFrontId,
    InfestationPhase, InfestationType, MigrationPath, MigrationPathId, RecoveryProjection,
    RegenerationMode, ResourceKind, ResourceZone, ResourceZoneId, SustainabilityEvent,
    SustainabilityEventKind, SustainabilityFingerprint, SustainabilityPolicy, SustainabilityRating,
    SustainabilitySummary, SustainabilityTickResult, SustainabilityTracker, TrophicLevelId,
    TrophicLink, TrophicRelation,
};
pub use faction::{
    Claim, ClaimKind, ClaimStrength, DiplomacyTable, Faction, FactionId, FactionMembership,
    FactionRegistry, FactionSnapshot, FactionSummary, FactionTag, Influence, MembershipKind,
    OwnershipStatus, Region, RegionId, ReputationConfig, ReputationDelta, ReputationEvent,
    ReputationHistory, ReputationSet, ReputationTier, Stance, StanceTable, Standing, TerritoryMap,
    TerritorySnapshot,
};
pub use goal::{
    Consideration, ConsiderationId, ConsiderationScore, ContextFact, CooldownConfig, CurveKind,
    GoalContext, GoalContextBuilder, GoalDef, GoalId, GoalScore, GoalSelection, GoalSelector,
    GoalSnapshot, GoalSummary, GoalTag, HysteresisConfig, InertiaConfig, InputBinding,
    ScoringBreakdown, SelectionReason, UtilityCurve,
};
pub use group::{
    EvacuationConfig, EvacuationContext, EvacuationState, EvacuationTrigger, FlockingResult,
    FormationConfig, Group, GroupDecision, GroupEvent, GroupEventKind, GroupId, GroupMember,
    GroupPreset, GroupRegistry, GroupRole, GroupSnapshot, GroupSummary, MemberId, PatrolRoute,
    PatrolRouteId, PatrolState, SafeZone, SerializableVec3, Waypoint, calculate_flocking,
};
pub use memory::{
    CreatureMemory, DangerCategory, DangerZoneMemory, DecayConfig, FoodCategory, FoodSourceMemory,
    MemoryCategory, MemoryFingerprint, MemoryId, MemoryQuery, MemoryQueryBuilder, MemoryRecord,
    MemorySnapshot, MemorySource, MemoryStoreConfig, MemorySummary, MemoryTag, PlayerTraceKind,
    PlayerTraceMemory, QueryResult, RegionMemorySummary, RegionScope,
};
pub use navigation::{
    AgentCapabilities, AgentCapabilityId, CostModifier, DomainCost, DomainTransition, DynamicFrame,
    EdgeAnnotation, EdgeRequirements, FrameCrossing, FrameId, FrameVelocity, MovementDomain,
    MultiDomainWorld, NavPosition, NavRegionId, NavRegionSummary, Navigator, NodeAnnotation,
    RegionConnection, ReplanReason, RouteFailure, RouteLimitExceeded, RouteLimitType, RoutePlan,
    RouteRequest, RouteRequestId, RouteResult, RouteWaypoint, SteeringDirection, SteeringHint,
    SurfaceType, capability_presets,
};
pub use needs::{
    ActiveAffliction, AfflictionCategory, AfflictionDef, AfflictionId, AfflictionRegistry,
    AfflictionSet, AfflictionTickResult, ApplyResult, ColonySnapshot, ColonySummary, DecayMode,
    EffectCategory, EnvironmentSnapshot, EnvironmentalTrigger, ExposureSnapshot, ExposureTrigger,
    ImmunitySet, ManagedStatusSet, ModifierDef, Need, NeedConfig, NeedEvent, NeedHistogram, NeedId,
    NeedProfile, NeedSet, NeedState, ProfileId, RecoveryMode, ResistanceSet, Severity,
    SeverityChange, SeverityEffect, SeverityModifier, SeverityThresholds, StackingBehavior,
    StatusEffect, StatusEffectDef, StatusEffectId, StatusEffectRegistry, StatusModifier, StatusSet,
    Threshold, ThresholdKind, affliction_presets, evaluate_exposure, evaluate_trigger,
    find_triggered_effects, presets, should_recover,
};
pub use offline::{
    AttentionLevel, LoadHandoff, OfflineConfig, OfflineEvent, OfflineEventKind, OfflineRegionId,
    OfflineRegionState, OfflineSimulator, OfflineSummary, RegionBudget, RegionSnapshot,
    StalenessInfo, StepResult, UnloadHandoff, UnloadReason,
};
pub use pathfinding::{AStar, AStarConfig, NavMesh, NavMeshConfig, PathResult};
pub use population::{
    DespawnBudget, DespawnReason, GroupCap, GroupCapId, MigrationConfig, MigrationPhase,
    MigrationRoute, MigrationRouteId, MigrationStatus, MigrationWave, MigrationWaveId,
    PacingIntensity, PacingProfile, PopulationConfig, PopulationDirector, PopulationEvent,
    PopulationEventKind, PopulationSnapshot, PopulationSummary, RegionDensity, RegionalPopulation,
    SpawnBudget, SpawnEvent, SpawnPressure, SpawnPriority, SpeciesCap, SpeciesCapId,
    SpeciesRegistry, ThreatConfig, ThreatLevel, ThreatModifier, ThreatSource, TickResult, ZoneBias,
};
pub use sensor::{
    AttenuationCurve, DetectionStrength, MemoryConfig, Observation, ObservationId,
    ObservationMemory, ObservationPriority, ObservationSet, OcclusionModel, SensorConfig,
    SensorKind, SensorProfile, SensorProfileId, SensorSnapshot, SensorSpec, SensorSuite,
    SensorSummary, Stimulus, StimulusEmitter, StimulusId, StimulusSource, StimulusSummary,
};

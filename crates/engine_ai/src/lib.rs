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
pub mod colony;
pub mod creatures;
pub mod curved_world;
pub mod disease;
pub mod ecology;
pub mod emergency;
pub mod faction;
pub mod goal;
pub mod group;
pub mod lifecycle;
pub mod memory;
pub mod navigation;
pub mod needs;
pub mod offline;
pub mod pathfinding;
pub mod planner;
pub mod population;
pub mod sensor;
pub mod settler;
pub mod social;
pub mod territory_pressure;

pub use behavior::{BehaviorNode, BehaviorTree, Blackboard, NodeStatus};
pub use colony::{
    CascadeConfig, ColonyConfig, ColonyFingerprint, ColonyId, ColonyManager, ColonyProjection,
    ColonyStateSnapshot, ColonyStateSummary, ColonyTickResult, Failure, FailureEvent,
    FailureEventKind, FailureEventLog, FailureFingerprint, FailureId, FailureProjection,
    FailureRegistry, FailureSeverity, FailureStatus, FailureSummary, FailureTrigger, Job,
    JobCategory, JobDef, JobDefId, JobDefRegistry, JobEvent, JobEventKind, JobFailureReason, JobId,
    JobPriority, JobRegistry, JobStatus, LogisticsEvent, LogisticsEventKind, LogisticsFingerprint,
    LogisticsProjection, LogisticsSummary, MitigationAction, Rating, RatingCategory,
    ResourceAmount, ResourceBalance, ResourceId, Route, RouteId, RouteRegistry, Shelter,
    ShelterCoverage, ShelterFingerprint, ShelterId, ShelterRatings, ShelterRecommendation,
    ShelterRegistry, ShelterWeights, SkillId, StorageNode, StorageNodeId, StorageRegistry,
    Transfer, TransferId, TransferRegistry, TransferStatus, Worker, WorkerCapability, WorkerId,
    WorkerRegistry, WorkerSkillSet, generate_recommendations, job_presets, suggest_mitigations,
};
pub use creatures::{PassiveAI, PassiveState};
pub use curved_world::{
    ConnectivityChangeType, CurvedConnectivityChange, CurvedGridCell, CurvedNodeAnnotation,
    CurvedPassabilityChange, CurvedPath, CurvedPathFailure, CurvedPathLimitExceeded,
    CurvedPathLimitType, CurvedPathResult, CurvedPathfinder, CurvedPathfindingConfig,
    CurvedPosition, CurvedSurfaceConfig, CurvedSurfaceId, CurvedSurfaceSummary, CurvedWaypoint,
    CurvedWorldFingerprint, CurvedWorldProjection, CurvedWorldSnapshot, SurfaceGeometry,
    TangentBasis,
};
pub use disease::{
    ActiveInfection, ContaminationRegistry, ContaminationSource, ContaminationZone,
    ContaminationZoneId, CreateZoneRequest, CrossRegionSpread, DiseaseConfig, DiseaseEvent,
    DiseaseEventKind, DiseaseFingerprint, DiseaseProjection, DiseaseRegionId, DiseaseSnapshot,
    DiseaseSummary, DiseaseTickEvents, DiseaseTickResult, DiseaseTracker, EvolutionEvent,
    ExposureEvent, ExposureSource, HostId, HostInfectionState, HostSpreadInfo, HostTickResult,
    ImmunityRecord, InfectionSpreadInfo, InfectionStage, MutationConfig, MutationContext,
    MutationResult, MutationTracker, PathogenCategory, PathogenDef, PathogenId, PathogenRegistry,
    PathogenReservoir, PathogenTraits, RegionPopulation, ResistanceProfile, SpreadConfig,
    SpreadPlan, SpreadPlanSummary, SpreadPlanner, SpreadRoute, StageTransition, StrainId,
    TraitBounds, TraitChanges, pathogen_presets,
};
pub use ecology::{
    CarryingCapacityConfig, CompetitorRelation, DepletionBehavior, DepletionProjection,
    EcologyConfig, EcologyEvent, EcologyEventKind, EcologyFingerprint, EcologySimulator,
    EcologySnapshot, EcologySummary, EcologyTickResult, EcosystemConfig, EcosystemEvent,
    EcosystemEventKind, EcosystemFingerprint, EcosystemProjection, EcosystemRegion,
    EcosystemRegionId, EcosystemSimulator, EcosystemSummary, EcosystemTickResult, FoodChain,
    HarvestPressure, InfestationFront, InfestationFrontId, InfestationPhase, InfestationType,
    MigrationCorridor, MigrationPath, MigrationPathId, Population, PopulationKey,
    PredatorPreyRelation, RecoveryProjection, RegenerationMode, ResourceKind, ResourceZone,
    ResourceZoneId, Species, SpeciesId, SustainabilityEvent, SustainabilityEventKind,
    SustainabilityFingerprint, SustainabilityPolicy, SustainabilityRating, SustainabilitySummary,
    SustainabilityTickResult, SustainabilityTracker, TrophicLevelId, TrophicLink, TrophicRelation,
    TrophicRole,
};
pub use emergency::{
    ActionStatus, Assignment, ContainmentZone, ContainmentZoneId, Emergency, EmergencyEvent,
    EmergencyEventKind, EmergencyFingerprint, EmergencyId, EmergencyKind, EmergencyProjection,
    EmergencySeverity, EmergencySnapshot, EmergencyStatus, EmergencySummary, EmergencyTypeId,
    PlanStatus, Responder, ResponderId, ResponderRoleId, ResponderStatus, ResponseAction,
    ResponseActionId, ResponseActionKind, ResponsePlan, ResponsePlanId, ResponseProtocolId,
    ShelterZone, ShelterZoneId, create_standard_plan,
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
pub use lifecycle::{
    AgingConfig, CorpseState, DecayConfig as LifecycleDecayConfig, EggState, GrowthConfig,
    GrowthPhase, HatchingConfig, IncubationConfig, LifecycleConfig, LifecycleEvent,
    LifecycleEventKind, LifecycleFingerprint, LifecycleId, LifecycleProjection, LifecycleSnapshot,
    LifecycleStage, LifecycleSummary, LifecycleTickResult, LifecycleTracker, LifecycleTrend,
    LivingState, MetamorphosisConfig, MetamorphosisState, MetamorphosisTrigger,
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
pub use planner::{
    ActionCost, ActionDef, ActionDefId, ActionInstanceId, ActionRegistry, ActionUtility,
    ActiveIntent, ActorId, ActorPlanAssignment, BeliefFingerprint, BeliefState, ExecutionConfig,
    ExecutionFailure, FactId, FactModification, FactRequirement, FactValue, FactionScopeId, Intent,
    IntentId, IntentParams, IntentPriority, IntentSet, IntentTag, LocationId, PartialReason, Plan,
    PlanEvent, PlanFailure, PlanFingerprint, PlanId, PlanResult, PlanSelectionMode, PlanState,
    PlanStatus as PlannerPlanStatus, PlanTracker, PlanTrackerFingerprint, PlannedAction, Planner,
    PlannerConfig, PlannerLimit, PlannerSnapshot, PlannerStats, PlannerSummary, ResourceTypeId,
    RiskLevel, StepState, StepStatus,
};
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
pub use settler::{
    AssignmentCandidate, AssignmentConfig, AssignmentEngine, AssignmentResult, CapabilityCategory,
    CapabilityDef, CapabilityId, FailureReason, PriorityConfig, PriorityMode, PriorityScore,
    RegionPriority, ReservationTable, Settler, SettlerEvent, SettlerEventKind, SettlerFingerprint,
    SettlerId, SettlerManager, SettlerManagerConfig, SettlerProjection, SettlerRegistry,
    SettlerSnapshot, SettlerStatus, SettlerSummary, SettlerTickResult, Skill, SkillLevel, SkillSet,
    Task, TaskCategory, TaskDef, TaskDefId, TaskDefRegistry, TaskId, TaskPosition, TaskRegistry,
    TaskStatus, WorkPriorities, capability_presets as settler_capability_presets, task_def_presets,
};
pub use social::{
    AgentMorale, AgentPanic, BetrayalEvent, BetrayalEventKind, BetrayalFactors,
    BetrayalFingerprint, BetrayalId, BetrayalIncident, BetrayalKind, BetrayalProfile,
    BetrayalResolution, BetrayalRisk, BetrayalSeverity, BetrayalStatus, BetrayalTracker,
    DiplomacyEvent, DiplomacyEventKind, DiplomacyFingerprint, DiplomacyId, DiplomacyTracker,
    DiplomaticRelation, DiplomaticStance, FactionSocialSummary, GrievanceLevel, GroupMorale,
    LoyaltyLevel, MoraleEvent, MoraleEventKind, MoraleFactors, MoraleFingerprint, MoraleLevel,
    MoraleProjection, MoraleTracker, PanicCascade, PanicEvent, PanicEventStatus, PanicFingerprint,
    PanicId, PanicLevel, PanicProjection, PanicSource, PanicTracker, PanicTrackingEvent,
    PanicTrackingEventKind, SocialAgentId, SocialFactionId, SocialFingerprint, SocialGroupId,
    SocialProjection, SocialSnapshot, SocialSummary, SocialTrend, StanceCounts, SuspicionLevel,
    Treaty, TreatyId, TreatyKind, TreatyStatus, TrustLevel,
};
pub use territory_pressure::{
    CollapseReason, ContestedFront, ContestedFrontId, ExpansionCandidate, ExpansionFailureReason,
    FrontState, NestEvent, NestEventKind, NestExpansionCandidate, NestExpansionState,
    NestExpansionSummary, NestExpansionTracker, NestFingerprint, NestId, NestKind, NestProjection,
    NestSite, NestSnapshot, NestStage, NestStageTransition, NestTickResult, PressureConfig,
    PressureEvent, PressureEventKind, PressureFingerprint, PressureKind, PressureProjection,
    PressureSource, PressureSourceId, PressureSummary, PressureTickResult, PressureTrend,
    RegionPressureSnapshot, TerritoryPressureTracker,
};

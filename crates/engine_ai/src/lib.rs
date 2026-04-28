//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, decision making, needs simulation,
//! sensory perception, faction/reputation/territory systems, goal-based
//! utility AI for survival prioritization, and population director for spawn
//! pressure, pacing, migration, and regional threat scaling.

pub mod behavior;
pub mod creatures;
pub mod faction;
pub mod goal;
pub mod needs;
pub mod pathfinding;
pub mod population;
pub mod sensor;

pub use behavior::{BehaviorNode, BehaviorTree, Blackboard, NodeStatus};
pub use creatures::{PassiveAI, PassiveState};
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
pub use needs::{
    ColonySnapshot, ColonySummary, Need, NeedConfig, NeedEvent, NeedHistogram, NeedId, NeedProfile,
    NeedSet, NeedState, ProfileId, StatusEffect, StatusEffectId, StatusModifier, StatusSet,
    Threshold, ThresholdKind,
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

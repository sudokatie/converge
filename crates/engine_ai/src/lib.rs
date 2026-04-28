//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, decision making, needs simulation,
//! sensory perception, and faction/reputation/territory systems.

pub mod behavior;
pub mod creatures;
pub mod faction;
pub mod needs;
pub mod pathfinding;
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
pub use needs::{
    ColonySnapshot, ColonySummary, Need, NeedConfig, NeedEvent, NeedHistogram, NeedId, NeedProfile,
    NeedSet, NeedState, ProfileId, StatusEffect, StatusEffectId, StatusModifier, StatusSet,
    Threshold, ThresholdKind,
};
pub use pathfinding::{AStar, AStarConfig, NavMesh, NavMeshConfig, PathResult};
pub use sensor::{
    AttenuationCurve, DetectionStrength, MemoryConfig, Observation, ObservationId,
    ObservationMemory, ObservationPriority, ObservationSet, OcclusionModel, SensorConfig,
    SensorKind, SensorProfile, SensorProfileId, SensorSnapshot, SensorSpec, SensorSuite,
    SensorSummary, Stimulus, StimulusEmitter, StimulusId, StimulusSource, StimulusSummary,
};

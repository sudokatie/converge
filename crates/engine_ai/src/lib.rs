//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, decision making, needs simulation,
//! and sensory perception.

pub mod behavior;
pub mod creatures;
pub mod needs;
pub mod pathfinding;
pub mod sensor;

pub use behavior::{BehaviorNode, BehaviorTree, Blackboard, NodeStatus};
pub use creatures::{PassiveAI, PassiveState};
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

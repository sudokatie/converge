//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, decision making, and needs simulation.

pub mod behavior;
pub mod creatures;
pub mod needs;
pub mod pathfinding;

pub use behavior::{BehaviorNode, BehaviorTree, Blackboard, NodeStatus};
pub use creatures::{PassiveAI, PassiveState};
pub use needs::{
    ColonySnapshot, ColonySummary, Need, NeedConfig, NeedEvent, NeedHistogram, NeedId, NeedProfile,
    NeedSet, NeedState, ProfileId, StatusEffect, StatusEffectId, StatusModifier, StatusSet,
    Threshold, ThresholdKind,
};
pub use pathfinding::{AStar, AStarConfig, NavMesh, NavMeshConfig, PathResult};

//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, and decision making.

pub mod behavior;
pub mod creatures;
pub mod pathfinding;

pub use behavior::{BehaviorNode, BehaviorTree, Blackboard, NodeStatus};
pub use creatures::{PassiveAI, PassiveState};
pub use pathfinding::{AStar, AStarConfig, NavMesh, NavMeshConfig, PathResult};

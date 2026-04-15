//! AI system for the Lattice game engine.
//!
//! Provides creature behavior, pathfinding, and decision making.

pub mod behavior;
pub mod creatures;
pub mod pathfinding;

pub use behavior::{BehaviorTree, BehaviorNode, NodeStatus, Blackboard};
pub use creatures::{PassiveAI, PassiveState};
pub use pathfinding::{AStar, AStarConfig, PathResult, NavMesh, NavMeshConfig};

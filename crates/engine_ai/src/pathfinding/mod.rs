//! Pathfinding module
//!
//! Implements spec 8.2 - A* and `NavMesh` pathfinding for voxel worlds.

pub mod astar;
pub mod cache;
pub mod navmesh;

pub use astar::{AStar, AStarConfig, PathResult};
pub use navmesh::{NavEdge, NavMesh, NavMeshConfig, NavNode};

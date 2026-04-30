//! Region graph generation for macro-scale structure and progression planning.
//!
//! This module provides a deterministic, CPU-side graph planning primitive for
//! generating and querying region-scale layouts. It supports stations, trenches,
//! cave networks, hollow spheres, colonies, routes, gates, hazards, resources,
//! and mission progression.
//!
//! # Overview
//!
//! - [`RegionId`], [`EdgeId`] - Unique identifiers for nodes and edges
//! - [`RegionKind`], [`RegionTag`] - Region classification
//! - [`EdgeKind`] - Edge/connection classification
//! - [`RegionNode`] - A region in the graph
//! - [`RegionEdge`] - A connection between regions
//! - [`RegionGraph`] - The complete graph structure with generation and queries
//! - [`RegionGraphConfig`] - Configuration for graph generation
//! - [`GateRequirement`], [`ProgressionTier`] - Progression gates and tiers
//! - [`ResourceAnnotation`], [`HazardAnnotation`], [`MissionAnnotation`] - Region metadata
//!
//! # Determinism
//!
//! All graph generation and operations are deterministic with stable ordering:
//!
//! - Same seed produces same graph structure
//! - Regions and edges ordered by ID
//! - Queries return sorted results
//! - Fingerprints are stable across runs
//!
//! # Example
//!
//! ```
//! use engine_world::region_graph::{
//!     RegionGraph, RegionGraphConfig, ProgressionTier,
//! };
//!
//! // Generate a small graph
//! let config = RegionGraphConfig::small(42);
//! let graph = RegionGraph::generate(config);
//!
//! // Query the graph
//! let spawn = graph.spawn().unwrap();
//! let goal = graph.goal().unwrap();
//!
//! // Find path from spawn to goal
//! let path = graph.shortest_path(spawn, goal, ProgressionTier::MAX);
//! assert!(path.is_some());
//!
//! // Get reachable regions at tier 0
//! let reachable = graph.reachable_from(spawn, ProgressionTier::new(0));
//! assert!(reachable.contains(&spawn));
//!
//! // Check graph summary
//! let summary = graph.summary();
//! assert!(summary.has_spawn);
//! assert!(summary.has_goal);
//! ```
//!
//! # Generation
//!
//! The generator creates connected graphs with:
//!
//! - A spawn and goal region
//! - A critical path connecting spawn to goal
//! - Optional side branches and dead ends
//! - Loop edges for non-linear progression
//! - Progression tiers with gated access
//! - Resource deposits and hazard zones
//!
//! ```
//! use engine_world::region_graph::{RegionGraph, RegionGraphConfig, KindWeights};
//!
//! // Custom configuration
//! let config = RegionGraphConfig::new(12345)
//!     .with_region_count(40)
//!     .with_tiers(5, 8)
//!     .with_loop_probability(0.3)
//!     .with_hazards(true, 0.15)
//!     .with_resources(true, 0.25)
//!     .with_kind_weights(KindWeights::natural())
//!     .with_critical_path(10);
//!
//! let graph = RegionGraph::generate(config);
//! ```
//!
//! # Queries
//!
//! The graph supports various queries:
//!
//! - Neighbor queries: `neighbors()`, `edges_from()`, `accessible_neighbors()`
//! - Path queries: `shortest_path()`, `reachable_from()`
//! - Structure queries: `dead_ends()`, `branch_points()`, `chokepoints()`
//! - Tier queries: `regions_by_tier()`, `tier_summary()`
//! - Fingerprints: `fingerprint()`, `checksum()`

mod annotation;
mod config;
mod edge;
mod edge_id;
mod edge_kind;
mod fingerprint;
mod gate;
mod graph;
mod region;
mod region_id;
mod region_kind;

pub use annotation::{HazardAnnotation, MissionAnnotation, MissionRole, ResourceAnnotation};
pub use config::{ConfigError, KindWeights, RegionGraphConfig};
pub use edge::RegionEdge;
pub use edge_id::EdgeId;
pub use edge_kind::EdgeKind;
pub use fingerprint::{FingerprintBuilder, GraphChecksum, GraphFingerprint};
pub use gate::{GateKind, GateRequirement, ProgressionTier};
pub use graph::{GraphSummary, RegionGraph, TierSummary};
pub use region::RegionNode;
pub use region_id::RegionId;
pub use region_kind::{RegionKind, RegionTag};

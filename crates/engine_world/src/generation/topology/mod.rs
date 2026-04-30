//! Topology and cave layout generation for deterministic world planning.
//!
//! This module provides CPU-side generation primitives for planning topology
//! layouts like trenches, ice tunnels, station decks, and hollow-sphere interiors.
//!
//! # Overview
//!
//! - [`TopologyKind`] - Classification of topology types
//! - [`TopologyConfig`] - Configuration for topology generation
//! - [`TopologyNode`] - A node in the topology graph
//! - [`TopologySegment`] - A connection between nodes
//! - [`TopologyCell`] - Spatial sampling cell
//! - [`TopologyPlanner`] - Main generator/planner
//! - [`TopologyAnnotation`] - Hazard/resource/mission metadata
//!
//! # Determinism
//!
//! All generation is fully deterministic:
//!
//! - Same seed produces identical topology
//! - Nodes and segments ordered by ID
//! - Fingerprints stable across runs
//!
//! # Example
//!
//! ```
//! use engine_world::generation::topology::{
//!     TopologyConfig, TopologyKind, TopologyPlanner,
//! };
//!
//! let config = TopologyConfig::new(42, TopologyKind::Trench);
//! let planner = TopologyPlanner::generate(&config).unwrap();
//!
//! let entry = planner.entry_node().unwrap();
//! let neighbors = planner.neighbors(entry);
//!
//! let fingerprint = planner.fingerprint();
//! println!("Topology fingerprint: {fingerprint}");
//! ```

mod annotation;
mod cell;
mod config;
mod fingerprint;
mod kind;
mod node;
mod planner;
mod query;
mod segment;

pub use annotation::{
    HazardType, MissionHook, ResourceType, TopologyAnnotation, TopologyAnnotations,
};
pub use cell::{CellQuery, CellState, TopologyCell};
pub use config::{ConfigError, TopologyConfig};
pub use fingerprint::{FingerprintBuilder, TopologyChecksum, TopologyFingerprint};
pub use kind::TopologyKind;
pub use node::{NodeId, NodeRole, TopologyNode};
pub use planner::{PlannerSummary, TopologyPlanner};
pub use query::{PathQuery, QueryResult};
pub use segment::{SegmentId, SegmentKind, TopologySegment};

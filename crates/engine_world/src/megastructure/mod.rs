//! Streaming-friendly megastructure support for large multi-chunk structures.
//!
//! This module provides the infrastructure for managing structures that span
//! many chunks, such as space stations, titans (capital ships), interior spaces,
//! and trench walls.
//!
//! # Architecture
//!
//! - [`MegastructureId`] - Unique identifier for each structure
//! - [`StructureKind`] - Category (station, titan, interior, trench wall)
//! - [`StructureZone`] - Per-chunk classification (exterior, interior, hull, wall)
//! - [`StructureAnchor`] - Origin point and orientation
//! - [`ChunkBounds`] - Axis-aligned bounding box in chunk space
//! - [`ChunkMask`] - Sparse set of owned chunk positions
//! - [`ChunkSlice`] - Per-chunk state tracking (load state, dirty flag)
//! - [`SliceMap`] - Collection of slices with deterministic iteration
//! - [`StreamingManifest`] - Per-structure streaming metadata
//! - [`MegastructureRegistry`] - Central registry with spatial indexing
//!
//! # Streaming Support
//!
//! The system is designed for streaming-friendly operation:
//!
//! - Deterministic chunk ordering via `BTreeMap`/`BTreeSet`
//! - Per-chunk load state tracking (unloaded, loading, loaded, pending unload)
//! - Streaming priority tiers (critical, high, normal, low, background)
//! - Dependency tracking for load ordering
//! - Dirty/generation tracking for cache invalidation
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::megastructure::{
//!     MegastructureRegistry, AnchorMetadata, StructureKind, StructureZone,
//! };
//! use glam::IVec3;
//!
//! let mut registry = MegastructureRegistry::new(world_seed);
//!
//! // Create a station
//! let id = registry.create_station(
//!     IVec3::new(0, 100, 0),     // anchor position
//!     IVec3::new(5, 3, 5),       // size in chunks
//!     AnchorMetadata::named("Alpha Station"),
//! );
//!
//! // Query structures near a player
//! let query = StreamingQuery::from_observer(player_chunk, 10);
//! let nearby = registry.query(&query);
//!
//! // Build streaming manifest for a structure
//! let structure = registry.get(id).unwrap();
//! let manifest = structure.build_manifest();
//!
//! // Load chunks by priority
//! for entry in manifest.by_priority() {
//!     // Load chunk at entry.offset
//! }
//! ```

mod anchor;
mod bounds;
mod manifest;
mod registry;
mod slice;
mod structure_id;
mod structure_kind;

pub use anchor::{AnchorMetadata, StructureAnchor};
pub use bounds::{ChunkBounds, ChunkMask};
pub use manifest::{ManifestEntry, StreamingManifest, StreamingQuery, StreamingTier};
pub use registry::{IdGenerator, Megastructure, MegastructureRegistry};
pub use slice::{ChunkSlice, SliceMap, SliceState};
pub use structure_id::MegastructureId;
pub use structure_kind::{StructureKind, StructureZone};

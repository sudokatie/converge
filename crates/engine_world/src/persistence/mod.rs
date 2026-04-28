//! World persistence system.
//!
//! Provides chunk serialization, region-based storage, and world saves.
//!
//! # Multi-State Persistence
//!
//! For alternate dimensions, time-loop snapshots, and phased realities,
//! use the multi-state persistence types:
//!
//! - [`StateId`]: Unique identifier for reality states
//! - [`StateKind`]: Semantic classification of state types
//! - [`MultiStateChunk`]: Container for multiple chunk states
//! - [`MultiStateRegion`]: Region file format for multi-state chunks
//!
//! # Delta/Overlay Persistence
//!
//! For memory-efficient storage of chunk variants:
//!
//! - [`ChunkDelta`]: Compact overlay storing only changed blocks
//! - [`DeltaIndex`]: Compact position index for delta storage
//! - [`DeltaStats`]: Statistics about delta contents

mod chunk_delta;
mod multi_state_chunk;
mod multi_state_region;
mod region;
mod state_id;
mod world_meta;

pub use chunk_delta::{ChunkDelta, DeltaIndex, DeltaStats};
pub use multi_state_chunk::{MultiStateChunk, StateFallback};
pub use multi_state_region::{
    MultiStateRegion, MultiStateRegionError, RegionStats, multi_state_region_filename,
};
pub use region::{
    REGION_SIZE, Region, RegionError, chunk_to_local, chunk_to_region, region_filename,
};
pub use state_id::{StateId, StateKind};
pub use world_meta::{WorldError, WorldMeta, WorldPersistence};

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

mod multi_state_chunk;
mod multi_state_region;
mod region;
mod state_id;
mod world_meta;

pub use multi_state_chunk::{MultiStateChunk, StateFallback};
pub use multi_state_region::{
    MultiStateRegion, MultiStateRegionError, RegionStats, multi_state_region_filename,
};
pub use region::{
    REGION_SIZE, Region, RegionError, chunk_to_local, chunk_to_region, region_filename,
};
pub use state_id::{StateId, StateKind};
pub use world_meta::{WorldError, WorldMeta, WorldPersistence};

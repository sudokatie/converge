//! Coordinate types for voxel world positioning.
//!
//! Three coordinate systems:
//! - WorldPos: Global voxel position (can be negative)
//! - ChunkPos: Chunk index in world (can be negative)
//! - LocalPos: Position within a chunk (0..CHUNK_SIZE)

mod chunk_pos;
mod conversions;
mod local_pos;
mod world_pos;

pub use chunk_pos::ChunkPos;
pub use local_pos::LocalPos;
pub use world_pos::WorldPos;

/// Size of a chunk in voxels (16x16x16).
pub const CHUNK_SIZE: i32 = 16;

/// Size of a chunk as unsigned for LocalPos bounds.
pub const CHUNK_SIZE_U: u32 = CHUNK_SIZE as u32;

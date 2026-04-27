//! Voxel chunk storage and management.

mod block;
#[expect(
    clippy::module_inception,
    reason = "chunk.rs contains the main Chunk struct"
)]
mod chunk;
mod chunk_state;

pub use block::{AIR, BlockId, BlockProperties, BlockRegistry, DIRT, GRASS, SAND, STONE, WATER};
pub use chunk::{CHUNK_VOLUME, Chunk};
pub use chunk_state::ChunkState;

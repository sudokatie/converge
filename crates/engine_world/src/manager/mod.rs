//! Chunk management system.
//!
//! Handles chunk loading, unloading, generation, and meshing coordination.

mod chunk_manager;
mod loading_queue;
mod neighbor_tracker;

pub use chunk_manager::{ChunkEntry, ChunkManager};
pub use loading_queue::LoadingQueue;
pub use neighbor_tracker::NeighborTracker;

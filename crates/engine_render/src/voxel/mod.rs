//! Voxel rendering systems.
//!
//! Provides chunk meshing, texture atlas, and voxel-specific rendering.

mod ambient_occlusion;
mod chunk_mesh;
mod dirty_tracker;
mod greedy_mesh;
mod mesh_builder;
mod texture_atlas;
mod voxel_pipeline;
mod voxel_renderer;

pub use dirty_tracker::ChunkDirtyTracker;

pub use ambient_occlusion::calculate_ao;
pub use chunk_mesh::{ChunkMesh, ChunkMeshCache};
pub use greedy_mesh::{ChunkNeighbors, greedy_mesh};
pub use mesh_builder::{MeshBuilder, Vertex};
pub use texture_atlas::{TEXTURE_SIZE, TextureAtlas, TextureAtlasError};
pub use voxel_pipeline::{CameraUniform, ChunkUniform, VoxelPipeline};
pub use voxel_renderer::{DepthTexture, RenderStats, VoxelRenderer};

//! Voxel rendering systems.
//!
//! Provides chunk meshing, texture atlas, and voxel-specific rendering.

mod ambient_occlusion;
mod chunk_mesh;
mod greedy_mesh;
mod mesh_builder;

pub use ambient_occlusion::calculate_ao;
pub use chunk_mesh::{ChunkMesh, ChunkMeshCache};
pub use greedy_mesh::{greedy_mesh, ChunkNeighbors};
pub use mesh_builder::{MeshBuilder, Vertex};

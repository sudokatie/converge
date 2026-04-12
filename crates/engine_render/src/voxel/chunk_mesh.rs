//! GPU mesh representation for voxel chunks.

use bytemuck::cast_slice;
use glam::Vec3;
use wgpu::util::DeviceExt;

use super::mesh_builder::MeshBuilder;

/// GPU-resident mesh for a chunk.
#[derive(Debug)]
pub struct ChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    /// Chunk world position (for model matrix).
    position: Vec3,
}

impl ChunkMesh {
    /// Create a new chunk mesh from a mesh builder.
    ///
    /// # Arguments
    /// * `device` - wgpu device for buffer creation.
    /// * `builder` - Mesh data to upload.
    /// * `position` - World position of the chunk origin.
    #[must_use]
    pub fn new(device: &wgpu::Device, builder: &MeshBuilder, position: Vec3) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Vertex Buffer"),
            contents: cast_slice(builder.vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Index Buffer"),
            contents: cast_slice(builder.indices()),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: builder.index_count() as u32,
            position,
        }
    }

    /// Record draw commands for this mesh.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.index_count == 0 {
            return;
        }

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    /// Get the chunk's world position.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get the number of indices (triangles * 3).
    #[must_use]
    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    /// Check if the mesh is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index_count == 0
    }

    /// Get vertex buffer for binding.
    #[must_use]
    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    /// Get index buffer for binding.
    #[must_use]
    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }
}

/// Cache of chunk meshes keyed by position.
#[derive(Debug, Default)]
pub struct ChunkMeshCache {
    meshes: std::collections::HashMap<[i32; 3], ChunkMesh>,
}

impl ChunkMeshCache {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meshes: std::collections::HashMap::new(),
        }
    }

    /// Insert or update a mesh.
    pub fn insert(&mut self, chunk_pos: [i32; 3], mesh: ChunkMesh) {
        self.meshes.insert(chunk_pos, mesh);
    }

    /// Remove a mesh.
    pub fn remove(&mut self, chunk_pos: &[i32; 3]) -> Option<ChunkMesh> {
        self.meshes.remove(chunk_pos)
    }

    /// Get a mesh.
    #[must_use]
    pub fn get(&self, chunk_pos: &[i32; 3]) -> Option<&ChunkMesh> {
        self.meshes.get(chunk_pos)
    }

    /// Iterate over all meshes.
    pub fn iter(&self) -> impl Iterator<Item = (&[i32; 3], &ChunkMesh)> {
        self.meshes.iter()
    }

    /// Number of cached meshes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    /// Check if cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// Clear all meshes.
    pub fn clear(&mut self) {
        self.meshes.clear();
    }
}

// Note: Tests requiring wgpu device are integration tests, not unit tests.
// The draw() method is validated by actually rendering.

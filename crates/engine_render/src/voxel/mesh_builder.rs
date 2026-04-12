//! Mesh construction for voxel data.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Vertex format for voxel meshes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vertex {
    /// World position.
    pub position: [f32; 3],
    /// Packed normal (0-5 for -X,+X,-Y,+Y,-Z,+Z).
    pub normal: u32,
    /// Texture coordinates.
    pub uv: [f32; 2],
    /// Ambient occlusion (0-3).
    pub ao: u8,
    /// Padding for alignment.
    pub _pad: [u8; 3],
}

impl Vertex {
    /// Create a new vertex.
    #[must_use]
    pub fn new(position: Vec3, normal: u32, uv: [f32; 2], ao: u8) -> Self {
        Self {
            position: position.to_array(),
            normal,
            uv,
            ao,
            _pad: [0; 3],
        }
    }

    /// Vertex buffer layout descriptor.
    #[must_use]
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

/// Normal direction indices.
pub mod normals {
    pub const NEG_X: u32 = 0;
    pub const POS_X: u32 = 1;
    pub const NEG_Y: u32 = 2;
    pub const POS_Y: u32 = 3;
    pub const NEG_Z: u32 = 4;
    pub const POS_Z: u32 = 5;
}

/// Builder for constructing mesh data.
#[derive(Debug, Default)]
pub struct MeshBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    /// Create a new empty mesh builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Create with estimated capacity.
    #[must_use]
    pub fn with_capacity(vertex_count: usize, index_count: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_count),
            indices: Vec::with_capacity(index_count),
        }
    }

    /// Add a quad (4 vertices, 6 indices).
    ///
    /// Vertices should be in counter-clockwise order when viewed from outside.
    pub fn add_quad(&mut self, v0: Vertex, v1: Vertex, v2: Vertex, v3: Vertex) {
        let base = self.vertices.len() as u32;

        self.vertices.push(v0);
        self.vertices.push(v1);
        self.vertices.push(v2);
        self.vertices.push(v3);

        // Two triangles: 0-1-2, 0-2-3
        // Flip winding based on AO to avoid visual artifacts
        let ao_sum_02 = v0.ao as u32 + v2.ao as u32;
        let ao_sum_13 = v1.ao as u32 + v3.ao as u32;

        if ao_sum_02 > ao_sum_13 {
            // Standard winding
            self.indices.push(base);
            self.indices.push(base + 1);
            self.indices.push(base + 2);
            self.indices.push(base);
            self.indices.push(base + 2);
            self.indices.push(base + 3);
        } else {
            // Flipped winding to reduce AO artifacts
            self.indices.push(base + 1);
            self.indices.push(base + 2);
            self.indices.push(base + 3);
            self.indices.push(base + 1);
            self.indices.push(base + 3);
            self.indices.push(base);
        }
    }

    /// Get the vertex data.
    #[must_use]
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Get the index data.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of indices.
    #[must_use]
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Clear all data.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_size() {
        assert_eq!(std::mem::size_of::<Vertex>(), 28);
    }

    #[test]
    fn test_add_quad() {
        let mut builder = MeshBuilder::new();
        let v = Vertex::new(Vec3::ZERO, 0, [0.0, 0.0], 0);
        builder.add_quad(v, v, v, v);

        assert_eq!(builder.vertex_count(), 4);
        assert_eq!(builder.index_count(), 6);
    }
}

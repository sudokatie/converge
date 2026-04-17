//! Ghost block placement preview.
//!
//! Renders a semi-transparent block at the position where a block would be placed,
//! showing validity (green = valid, red = invalid). Implements spec 6.4.1.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::{
    include_wgsl, util::DeviceExt, vertex_attr_array, Buffer, BufferUsages, ColorTargetState,
    FragmentState, LoadOp, Operations, PrimitiveState, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, VertexBufferLayout, VertexStepMode,
};

use crate::backend::RenderDevice;
use engine_core::coords::WorldPos;

/// Vertex for the ghost block wireframe/surface.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GhostVertex {
    pub position: [f32; 3],
}

impl GhostVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x3];

    /// Get the vertex buffer layout.
    #[must_use]
    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<GhostVertex>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Uniform data for the ghost block shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GhostUniform {
    /// View-projection matrix.
    view_proj: [[f32; 4]; 4],
    /// Model matrix (translation to block position).
    model: [[f32; 4]; 4],
    /// Tint color (RGBA).
    color: [f32; 4],
}

/// Ghost block placement preview renderer.
///
/// Shows a semi-transparent block where the player is about to place a block.
/// Green tint = valid placement, red tint = invalid.
pub struct GhostBlockRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: Buffer,
    /// Current preview position (None = not showing).
    preview_pos: Option<WorldPos>,
    /// Whether current placement is valid.
    is_valid: bool,
    /// Whether the preview is visible.
    visible: bool,
}

impl GhostBlockRenderer {
    /// Number of vertices for a unit cube (6 faces * 2 triangles * 3 verts).
    const CUBE_VERT_COUNT: u32 = 36;

    /// Create a new ghost block renderer.
    #[must_use]
    pub fn new(device: &RenderDevice) -> Self {
        let shader = device
            .device()
            .create_shader_module(include_wgsl!("../shaders/ghost_block.wgsl"));

        let bind_group_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Ghost Block Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline_layout =
            device
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Ghost Block Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline =
            device
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Ghost Block Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[GhostVertex::layout()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(ColorTargetState {
                            format: device.surface_format(),
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24PlusStencil8,
                        depth_write_enabled: false, // Don't write depth for transparent
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Create unit cube vertices
        let vertices = Self::cube_vertices();
        let vertex_buffer =
            device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ghost Block Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: BufferUsages::VERTEX,
                });

        // Create uniform buffer
        let uniform = GhostUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
            color: [0.0, 1.0, 0.0, 0.4], // Green, semi-transparent
        };
        let uniform_buffer =
            device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ghost Block Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniform]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ghost Block Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer,
            preview_pos: None,
            is_valid: true,
            visible: true,
        }
    }

    /// Generate unit cube vertices (0,0,0) to (1,1,1).
    fn cube_vertices() -> Vec<GhostVertex> {
        // 6 faces, 2 triangles each, 3 vertices per triangle
        let v = [
            // Positions of a unit cube
            [0.0, 0.0, 0.0], // 0
            [1.0, 0.0, 0.0], // 1
            [1.0, 1.0, 0.0], // 2
            [0.0, 1.0, 0.0], // 3
            [0.0, 0.0, 1.0], // 4
            [1.0, 0.0, 1.0], // 5
            [1.0, 1.0, 1.0], // 6
            [0.0, 1.0, 1.0], // 7
        ];

        let faces: [[usize; 4]; 6] = [
            [0, 1, 2, 3], // Front (z=0)
            [5, 4, 7, 6], // Back (z=1)
            [4, 0, 3, 7], // Left (x=0)
            [1, 5, 6, 2], // Right (x=1)
            [3, 2, 6, 7], // Top (y=1)
            [4, 5, 1, 0], // Bottom (y=0)
        ];

        let mut vertices = Vec::with_capacity(36);
        for &[a, b, c, d] in &faces {
            vertices.push(GhostVertex { position: v[a] });
            vertices.push(GhostVertex { position: v[b] });
            vertices.push(GhostVertex { position: v[c] });
            vertices.push(GhostVertex { position: v[a] });
            vertices.push(GhostVertex { position: v[c] });
            vertices.push(GhostVertex { position: v[d] });
        }
        vertices
    }

    /// Update the ghost block preview position and validity.
    pub fn update(&mut self, preview_pos: Option<WorldPos>, is_valid: bool) {
        self.preview_pos = preview_pos;
        self.is_valid = is_valid;
    }

    /// Set visibility.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Render the ghost block.
    pub fn render(
        &self,
        device: &RenderDevice,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        view_proj: Mat4,
    ) {
        if !self.visible || self.preview_pos.is_none() {
            return;
        }

        let pos = self.preview_pos.unwrap();
        let model = Mat4::from_translation(Vec3::new(
            pos.x() as f32,
            pos.y() as f32,
            pos.z() as f32,
        ));

        // Color: green for valid, red for invalid
        let color = if self.is_valid {
            [0.0, 1.0, 0.0, 0.35]
        } else {
            [1.0, 0.0, 0.0, 0.35]
        };

        let uniform = GhostUniform {
            view_proj: view_proj.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            color,
        };

        device
            .queue()
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        let mut encoder = device.create_encoder("Ghost Block Render");

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Ghost Block Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load, // Render over existing scene
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..Self::CUBE_VERT_COUNT, 0..1);
        }

        device.submit([encoder.finish()]);
    }

    /// Get current preview position.
    #[must_use]
    pub fn preview_pos(&self) -> Option<WorldPos> {
        self.preview_pos
    }

    /// Get whether current placement is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::coords::WorldPos;

    #[test]
    fn test_cube_vertices_count() {
        let vertices = GhostBlockRenderer::cube_vertices();
        assert_eq!(vertices.len(), 36, "Cube should have 36 vertices");
    }

    #[test]
    fn test_cube_vertices_in_bounds() {
        let vertices = GhostBlockRenderer::cube_vertices();
        for v in &vertices {
            assert!(
                v.position[0] >= 0.0 && v.position[0] <= 1.0,
                "X out of bounds: {}",
                v.position[0]
            );
            assert!(
                v.position[1] >= 0.0 && v.position[1] <= 1.0,
                "Y out of bounds: {}",
                v.position[1]
            );
            assert!(
                v.position[2] >= 0.0 && v.position[2] <= 1.0,
                "Z out of bounds: {}",
                v.position[2]
            );
        }
    }

    #[test]
    fn test_ghost_vertex_layout() {
        let layout = GhostVertex::layout();
        assert_eq!(
            layout.array_stride as usize,
            std::mem::size_of::<GhostVertex>(),
            "Stride should match vertex size"
        );
        assert_eq!(layout.attributes.len(), 1, "Should have 1 attribute");
    }

    #[test]
    fn test_ghost_uniform_size() {
        // 4x4 view_proj + 4x4 model + 4 color = 36 floats = 144 bytes
        assert_eq!(
            std::mem::size_of::<GhostUniform>(),
            144,
            "GhostUniform should be 144 bytes"
        );
    }

    /// Test-only state tracker (avoids needing GPU device).
    struct GhostState {
        preview_pos: Option<WorldPos>,
        is_valid: bool,
        visible: bool,
    }

    impl GhostState {
        fn new() -> Self {
            Self {
                preview_pos: None,
                is_valid: true,
                visible: true,
            }
        }

        fn update(&mut self, preview_pos: Option<WorldPos>, is_valid: bool) {
            self.preview_pos = preview_pos;
            self.is_valid = is_valid;
        }

        fn set_visible(&mut self, visible: bool) {
            self.visible = visible;
        }

        fn preview_pos(&self) -> Option<WorldPos> {
            self.preview_pos
        }

        fn is_valid(&self) -> bool {
            self.is_valid
        }
    }

    #[test]
    fn test_ghost_update_sets_position() {
        let mut state = GhostState::new();
        let pos = WorldPos::new(5, 10, 15);

        state.update(Some(pos), true);
        assert_eq!(state.preview_pos(), Some(pos));
        assert!(state.is_valid());

        state.update(Some(pos), false);
        assert!(!state.is_valid());
    }

    #[test]
    fn test_ghost_visibility() {
        let mut state = GhostState::new();
        assert!(state.visible);
        state.set_visible(false);
        assert!(!state.visible);
    }

    #[test]
    fn test_ghost_no_preview() {
        let mut state = GhostState::new();
        assert_eq!(state.preview_pos(), None);
        state.update(None, false);
        assert_eq!(state.preview_pos(), None);
    }

    #[test]
    fn test_cube_has_all_corners() {
        let vertices = GhostBlockRenderer::cube_vertices();
        let positions: Vec<[f32; 3]> = vertices.iter().map(|v| v.position).collect();

        // Check all 8 corners appear at least once
        let corners = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];

        for corner in &corners {
            assert!(
                positions.iter().any(|p| {
                    (p[0] - corner[0]).abs() < 0.001
                        && (p[1] - corner[1]).abs() < 0.001
                        && (p[2] - corner[2]).abs() < 0.001
                }),
                "Corner {:?} should appear in cube vertices",
                corner
            );
        }
    }
}

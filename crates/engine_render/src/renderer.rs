//! Basic renderer for testing GPU setup.

use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, util::DeviceExt, vertex_attr_array, Buffer, BufferUsages, Color,
    LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, StoreOp, VertexBufferLayout, VertexStepMode,
};

use crate::backend::RenderDevice;

/// Vertex with position and color.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    /// Get the vertex buffer layout.
    #[must_use]
    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Basic triangle renderer for testing.
pub struct TriangleRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    num_vertices: u32,
    clear_color: Color,
}

impl TriangleRenderer {
    /// Create a new triangle renderer.
    #[must_use]
    pub fn new(device: &RenderDevice) -> Self {
        let shader = device
            .device()
            .create_shader_module(include_wgsl!("shaders/triangle.wgsl"));

        let pipeline_layout =
            device
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Triangle Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let pipeline =
            device
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Triangle Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[Vertex::layout()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: device.surface_format(),
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Default triangle vertices
        let vertices = [
            Vertex {
                position: [0.0, 0.5, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                color: [0.0, 0.0, 1.0],
            },
        ];

        let vertex_buffer =
            device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Triangle Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: BufferUsages::VERTEX,
                });

        Self {
            pipeline,
            vertex_buffer,
            num_vertices: vertices.len() as u32,
            clear_color: Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },
        }
    }

    /// Set the clear color.
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = Color { r, g, b, a: 1.0 };
    }

    /// Render a frame.
    pub fn render(&self, device: &RenderDevice, view: &wgpu::TextureView) {
        let mut encoder = device.create_encoder("Triangle Render");

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Triangle Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        device.submit([encoder.finish()]);
    }
}

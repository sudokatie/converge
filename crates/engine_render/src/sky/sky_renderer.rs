//! Sky renderer with procedural atmospheric scattering.

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::{
    include_wgsl, util::DeviceExt, Buffer, BufferUsages, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    StoreOp,
};

use crate::backend::RenderDevice;

/// Uniform data for sky shader.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SkyUniform {
    /// Inverse view-projection matrix for ray direction calculation.
    inv_view_proj: [[f32; 4]; 4],
    /// Camera position in world space.
    camera_position: [f32; 3],
    /// Time of day (0.0-1.0).
    time_of_day: f32,
}

/// Procedural sky renderer.
///
/// Renders a full-screen quad with atmospheric scattering before the voxel pass.
pub struct SkyRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: wgpu::BindGroup,
    time_of_day: f32,
}

impl SkyRenderer {
    /// Create a new sky renderer.
    #[must_use]
    pub fn new(device: &RenderDevice, _width: u32, _height: u32) -> Self {
        let shader = device
            .device()
            .create_shader_module(include_wgsl!("../shaders/sky.wgsl"));

        // Create uniform buffer
        let uniform = SkyUniform {
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_position: [0.0, 0.0, 0.0],
            time_of_day: 0.5, // Default to noon
        };

        let uniform_buffer =
            device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Sky Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniform]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        // Create bind group layout
        let bind_group_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sky Bind Group Layout"),
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

        let bind_group = device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sky Bind Group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

        // Create pipeline layout
        let pipeline_layout =
            device
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sky Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Create render pipeline
        let pipeline =
            device
                .device()
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Sky Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[], // No vertex buffer - positions computed from vertex_index
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: device.surface_format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None, // No culling for full-screen triangle
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None, // Sky doesn't write depth
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            time_of_day: 0.5,
        }
    }

    /// Resize the renderer (e.g., window resize).
    pub fn resize(&mut self, _device: &RenderDevice, _width: u32, _height: u32) {
        // Sky renderer doesn't need size-dependent resources
    }

    /// Advance time of day.
    ///
    /// # Arguments
    /// * `dt` - Delta time in seconds
    /// * `time_scale` - Multiplier for time progression (e.g., 1.0 = real time)
    pub fn update(&mut self, dt: f32, time_scale: f32) {
        self.time_of_day += dt * time_scale;
        // Wrap at 1.0
        self.time_of_day = self.time_of_day.rem_euclid(1.0);
    }

    /// Set the time of day directly.
    ///
    /// # Arguments
    /// * `t` - Time of day (0.0=midnight, 0.25=sunrise, 0.5=noon, 0.75=sunset)
    pub fn set_time_of_day(&mut self, t: f32) {
        self.time_of_day = t.rem_euclid(1.0);
    }

    /// Get the current time of day.
    #[must_use]
    pub fn time_of_day(&self) -> f32 {
        self.time_of_day
    }

    /// Render the sky.
    ///
    /// This should be called BEFORE the voxel pass. It clears the color attachment.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        camera_vp: &Mat4,
        camera_position: glam::Vec3,
    ) {
        // Update uniform buffer
        let inv_view_proj = camera_vp.inverse();
        let uniform = SkyUniform {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            camera_position: camera_position.to_array(),
            time_of_day: self.time_of_day,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        // Begin render pass with clear
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Sky Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        // Draw full-screen triangle (3 vertices, no vertex buffer)
        render_pass.draw(0..3, 0..1);
    }
}

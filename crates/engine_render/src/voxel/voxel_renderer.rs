//! Voxel chunk renderer with frustum culling.

use engine_core::coords::ChunkPos;
use engine_core::math::Frustum;
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::backend::RenderDevice;
use crate::camera::Camera;

use super::chunk_mesh::ChunkMesh;
use super::voxel_pipeline::{ChunkUniform, VoxelPipeline};

/// Statistics from a render frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    /// Total chunks considered.
    pub chunks_total: u32,
    /// Chunks that passed frustum culling.
    pub chunks_visible: u32,
    /// Chunks actually rendered (non-empty).
    pub chunks_rendered: u32,
    /// Total triangles rendered.
    pub triangles: u32,
}

/// Depth texture for the voxel renderer.
pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl DepthTexture {
    /// Create a new depth texture.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    /// Resize the depth texture.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, width, height);
    }
}

/// Renderer for voxel chunks.
pub struct VoxelRenderer {
    pipeline: VoxelPipeline,
    depth_texture: DepthTexture,
    clear_color: wgpu::Color,
    width: u32,
    height: u32,
}

impl VoxelRenderer {
    /// Create a new voxel renderer.
    pub fn new(device: &RenderDevice, width: u32, height: u32) -> Self {
        let pipeline = VoxelPipeline::new(device.device(), device.surface_format());
        let depth_texture = DepthTexture::new(device.device(), width, height);

        Self {
            pipeline,
            depth_texture,
            clear_color: wgpu::Color {
                r: 0.5,
                g: 0.7,
                b: 1.0,
                a: 1.0,
            },
            width,
            height,
        }
    }

    /// Resize the renderer (e.g., window resize).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.depth_texture.resize(device, width, height);
        }
    }

    /// Set the sky/clear color.
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 };
    }

    /// Render chunks visible from the camera.
    ///
    /// Returns statistics about the render.
    pub fn render_chunks<'a>(
        &self,
        device: &RenderDevice,
        view: &wgpu::TextureView,
        camera: &Camera,
        chunks: impl Iterator<Item = (ChunkPos, &'a ChunkMesh)>,
    ) -> RenderStats {
        let mut stats = RenderStats::default();

        // Update camera uniform
        let aspect = self.width as f32 / self.height as f32;
        let view_proj = camera.view_projection(aspect);
        self.pipeline
            .update_camera(device.queue(), view_proj, camera.position);

        // Get frustum for culling
        let frustum = camera.frustum(aspect);

        // Collect visible chunks
        let visible_chunks: Vec<_> = chunks
            .filter_map(|(pos, mesh)| {
                stats.chunks_total += 1;

                // Skip empty meshes
                if mesh.is_empty() {
                    return None;
                }

                // Frustum culling using chunk AABB
                let chunk_min = chunk_world_pos(pos);
                let chunk_max = chunk_min + Vec3::splat(16.0);

                if !frustum_contains_aabb(&frustum, chunk_min, chunk_max) {
                    return None;
                }

                stats.chunks_visible += 1;
                Some((pos, mesh))
            })
            .collect();

        // Create encoder
        let mut encoder = device.create_encoder("Voxel Render");

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Voxel Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(self.pipeline.pipeline());
            render_pass.set_bind_group(0, self.pipeline.camera_bind_group(), &[]);

            // Render each visible chunk
            for (pos, mesh) in visible_chunks {
                // Create chunk uniform buffer
                let chunk_world = chunk_world_pos(pos);
                let chunk_uniform = ChunkUniform::from_position(chunk_world);
                let chunk_buffer =
                    device
                        .device()
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Chunk Uniform"),
                            contents: bytemuck::cast_slice(&[chunk_uniform]),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                let chunk_bind_group =
                    self.pipeline
                        .create_chunk_bind_group(device.device(), &chunk_buffer);

                render_pass.set_bind_group(1, &chunk_bind_group, &[]);
                mesh.draw(&mut render_pass);

                stats.chunks_rendered += 1;
                stats.triangles += mesh.index_count() / 3;
            }
        }

        device.submit([encoder.finish()]);
        stats
    }

    /// Get the pipeline for external use.
    #[must_use]
    pub fn pipeline(&self) -> &VoxelPipeline {
        &self.pipeline
    }
}

/// Convert chunk position to world coordinates.
fn chunk_world_pos(pos: ChunkPos) -> Vec3 {
    Vec3::new(
        pos.0.x as f32 * 16.0,
        pos.0.y as f32 * 16.0,
        pos.0.z as f32 * 16.0,
    )
}

/// Check if frustum intersects an AABB.
fn frustum_contains_aabb(frustum: &Frustum, min: Vec3, max: Vec3) -> bool {
    use engine_core::math::Aabb;

    let aabb = Aabb::new(min, max);
    frustum.intersects_aabb(&aabb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_world_pos() {
        let pos = ChunkPos(glam::IVec3::new(1, 2, 3));
        let world = chunk_world_pos(pos);
        assert_eq!(world, Vec3::new(16.0, 32.0, 48.0));
    }

    #[test]
    fn test_chunk_world_pos_negative() {
        let pos = ChunkPos(glam::IVec3::new(-1, 0, 0));
        let world = chunk_world_pos(pos);
        assert_eq!(world, Vec3::new(-16.0, 0.0, 0.0));
    }
}

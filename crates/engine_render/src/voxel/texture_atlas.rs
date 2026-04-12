//! Block texture atlas using texture arrays.

use std::path::Path;

use image::{GenericImageView, ImageReader};
use thiserror::Error;
use tracing::info;

/// Texture size for blocks (16x16).
pub const TEXTURE_SIZE: u32 = 16;

/// Maximum number of textures in the atlas.
pub const MAX_TEXTURES: u32 = 256;

/// Error loading texture atlas.
#[derive(Debug, Error)]
pub enum TextureAtlasError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Invalid texture size: expected {TEXTURE_SIZE}x{TEXTURE_SIZE}, got {0}x{1}")]
    InvalidSize(u32, u32),
    #[error("Too many textures: max {MAX_TEXTURES}")]
    TooManyTextures,
}

/// Texture atlas for block textures.
///
/// Uses a 2D texture array where each layer is one block texture.
pub struct TextureAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    layer_count: u32,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TextureAtlas {
    /// Load textures from a directory.
    ///
    /// Each PNG file in the directory becomes a layer in the texture array.
    /// Files are loaded in alphabetical order.
    pub fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self, TextureAtlasError> {
        // Collect all PNG files
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            })
            .collect();

        entries.sort_by_key(|e| e.path());

        if entries.len() > MAX_TEXTURES as usize {
            return Err(TextureAtlasError::TooManyTextures);
        }

        let layer_count = entries.len().max(1) as u32;

        // Create texture array
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Block Texture Atlas"),
            size: wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: layer_count,
            },
            mip_level_count: 4, // 16, 8, 4, 2
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Load each texture into a layer
        for (i, entry) in entries.iter().enumerate() {
            let img = ImageReader::open(entry.path())?.decode()?;
            let (width, height) = img.dimensions();

            if width != TEXTURE_SIZE || height != TEXTURE_SIZE {
                return Err(TextureAtlasError::InvalidSize(width, height));
            }

            let rgba = img.to_rgba8();

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * TEXTURE_SIZE),
                    rows_per_image: Some(TEXTURE_SIZE),
                },
                wgpu::Extent3d {
                    width: TEXTURE_SIZE,
                    height: TEXTURE_SIZE,
                    depth_or_array_layers: 1,
                },
            );

            info!("Loaded texture layer {}: {:?}", i, entry.path());
        }

        // Generate mipmaps (simplified - just for first mip for now)
        // Full mipmap generation would require compute shaders or CPU downscale

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Block Texture Atlas View"),
            format: None,
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            usage: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Block Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixelated look
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Atlas Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Atlas Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            texture,
            view,
            sampler,
            layer_count,
            bind_group_layout,
            bind_group,
        })
    }

    /// Create a placeholder atlas with a single colored texture.
    pub fn placeholder(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Placeholder Texture Atlas"),
            size: wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create a simple checkerboard pattern
        let mut data = vec![0u8; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize];
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let idx = ((y * TEXTURE_SIZE + x) * 4) as usize;
                let checker = ((x / 4) + (y / 4)) % 2 == 0;
                let (r, g, b) = if checker {
                    (200, 200, 200)
                } else {
                    (100, 100, 100)
                };
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255;
            }
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * TEXTURE_SIZE),
                rows_per_image: Some(TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Placeholder Atlas View"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            format: None,
            usage: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Atlas Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Atlas Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            layer_count: 1,
            bind_group_layout,
            bind_group,
        }
    }

    /// Get the bind group for this atlas.
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get the bind group layout.
    #[must_use]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get the number of texture layers.
    #[must_use]
    pub fn layer_count(&self) -> u32 {
        self.layer_count
    }

    /// Get the texture view.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Get the sampler.
    #[must_use]
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

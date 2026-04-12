//! GPU device and surface management.

use std::sync::Arc;

use thiserror::Error;
use wgpu::{
    Adapter, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    SurfaceError, SurfaceTexture, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};
use winit::window::Window;

/// Error type for render device operations.
#[derive(Debug, Error)]
pub enum RenderDeviceError {
    /// No suitable GPU adapter found.
    #[error("No suitable GPU adapter found")]
    NoAdapter,
    /// Failed to request device.
    #[error("Failed to request device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    /// Surface error during frame acquisition.
    #[error("Surface error: {0}")]
    Surface(#[from] SurfaceError),
    /// Surface configuration error.
    #[error("Failed to get surface capabilities")]
    SurfaceCapabilities,
}

/// Context for a single frame of rendering.
pub struct FrameContext {
    /// The surface texture to render to.
    pub texture: SurfaceTexture,
    /// View into the surface texture.
    pub view: TextureView,
}

/// GPU device wrapper for wgpu.
pub struct RenderDevice {
    #[allow(dead_code)]
    instance: Instance,
    #[allow(dead_code)]
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
}

impl RenderDevice {
    /// Create a new render device for the given window.
    ///
    /// # Errors
    /// Returns an error if no suitable GPU is found or device creation fails.
    pub fn new(window: Arc<Window>) -> Result<Self, RenderDeviceError> {
        pollster::block_on(Self::new_async(window))
    }

    /// Async version of device creation.
    async fn new_async(window: Arc<Window>) -> Result<Self, RenderDeviceError> {
        // Create instance
        let instance = Instance::new(&InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone()).map_err(|_| {
            RenderDeviceError::NoAdapter
        })?;

        // Request adapter
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderDeviceError::NoAdapter)?;

        tracing::info!("Using GPU: {}", adapter.get_info().name);

        // Request device
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Lattice Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        // Configure surface
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);

        // Prefer sRGB format
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
        })
    }

    /// Resize the surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Get the GPU device.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get the command queue.
    #[must_use]
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Get the surface texture format.
    #[must_use]
    pub fn surface_format(&self) -> TextureFormat {
        self.config.format
    }

    /// Get the current surface size.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Begin a new frame.
    ///
    /// # Errors
    /// Returns an error if the surface texture cannot be acquired.
    pub fn begin_frame(&mut self) -> Result<FrameContext, RenderDeviceError> {
        let texture = match self.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                // Reconfigure and retry
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture()?
            }
            Err(e) => return Err(e.into()),
        };

        let view = texture.texture.create_view(&TextureViewDescriptor::default());

        Ok(FrameContext { texture, view })
    }

    /// End the frame and present.
    pub fn end_frame(&mut self, frame: FrameContext) {
        frame.texture.present();
    }

    /// Submit command buffers to the queue.
    pub fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(&self, buffers: I) {
        self.queue.submit(buffers);
    }

    /// Create a command encoder.
    #[must_use]
    pub fn create_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }
}

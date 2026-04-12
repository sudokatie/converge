//! Window management using winit.

use std::sync::Arc;

use thiserror::Error;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Fullscreen, Window as WinitWindow, WindowAttributes, WindowId},
};

/// Window creation error.
#[derive(Debug, Error)]
pub enum WindowError {
    /// Failed to create event loop.
    #[error("Failed to create event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    /// Failed to create window.
    #[error("Failed to create window: {0}")]
    CreateWindow(#[from] winit::error::OsError),
}

/// Window configuration.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Initial width in logical pixels.
    pub width: u32,
    /// Initial height in logical pixels.
    pub height: u32,
    /// Start in fullscreen mode.
    pub fullscreen: bool,
    /// Enable VSync (hint only, actual VSync is set in renderer).
    pub vsync: bool,
    /// Allow window resizing.
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("Lattice"),
            width: 1280,
            height: 720,
            fullscreen: false,
            vsync: true,
            resizable: true,
        }
    }
}

/// High-level window wrapper.
pub struct Window {
    inner: Arc<WinitWindow>,
    current_size: PhysicalSize<u32>,
    scale_factor: f64,
}

impl Window {
    /// Get the current window size in physical pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.current_size.width, self.current_size.height)
    }

    /// Get the current DPI scale factor.
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Set fullscreen mode.
    pub fn set_fullscreen(&self, fullscreen: bool) {
        if fullscreen {
            self.inner
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            self.inner.set_fullscreen(None);
        }
    }

    /// Request a redraw.
    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    /// Get the raw winit window handle.
    #[must_use]
    pub fn raw_handle(&self) -> &WinitWindow {
        &self.inner
    }

    /// Get an Arc to the inner window (for wgpu surface creation).
    #[must_use]
    pub fn inner_arc(&self) -> Arc<WinitWindow> {
        Arc::clone(&self.inner)
    }
}

/// Events from the window.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window was resized.
    Resized { width: u32, height: u32 },
    /// Window close requested.
    CloseRequested,
    /// Window gained focus.
    Focused,
    /// Window lost focus.
    Unfocused,
    /// Redraw requested.
    RedrawRequested,
    /// Keyboard input.
    KeyboardInput {
        key: winit::keyboard::KeyCode,
        pressed: bool,
    },
    /// Mouse moved.
    MouseMoved { x: f64, y: f64 },
    /// Mouse button.
    MouseButton {
        button: winit::event::MouseButton,
        pressed: bool,
    },
    /// Mouse scroll.
    MouseScroll { delta_x: f64, delta_y: f64 },
}

/// Application handler for the event loop.
struct AppHandler<F> {
    config: WindowConfig,
    window: Option<Window>,
    callback: F,
}

impl<F> ApplicationHandler for AppHandler<F>
where
    F: FnMut(&mut Window, WindowEvent),
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title(&self.config.title)
                .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
                .with_resizable(self.config.resizable);

            match event_loop.create_window(attrs) {
                Ok(win) => {
                    let size = win.inner_size();
                    let scale = win.scale_factor();

                    if self.config.fullscreen {
                        win.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }

                    self.window = Some(Window {
                        inner: Arc::new(win),
                        current_size: size,
                        scale_factor: scale,
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to create window: {e}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(window) = &mut self.window else {
            return;
        };

        match event {
            winit::event::WindowEvent::Resized(size) => {
                window.current_size = size;
                (self.callback)(
                    window,
                    WindowEvent::Resized {
                        width: size.width,
                        height: size.height,
                    },
                );
            }
            winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                window.scale_factor = scale_factor;
            }
            winit::event::WindowEvent::CloseRequested => {
                (self.callback)(window, WindowEvent::CloseRequested);
                event_loop.exit();
            }
            winit::event::WindowEvent::Focused(focused) => {
                let ev = if focused {
                    WindowEvent::Focused
                } else {
                    WindowEvent::Unfocused
                };
                (self.callback)(window, ev);
            }
            winit::event::WindowEvent::RedrawRequested => {
                (self.callback)(window, WindowEvent::RedrawRequested);
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    (self.callback)(
                        window,
                        WindowEvent::KeyboardInput {
                            key: code,
                            pressed: event.state.is_pressed(),
                        },
                    );
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                (self.callback)(
                    window,
                    WindowEvent::MouseMoved {
                        x: position.x,
                        y: position.y,
                    },
                );
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                (self.callback)(
                    window,
                    WindowEvent::MouseButton {
                        button,
                        pressed: state.is_pressed(),
                    },
                );
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
                };
                (self.callback)(
                    window,
                    WindowEvent::MouseScroll {
                        delta_x: dx,
                        delta_y: dy,
                    },
                );
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Run the event loop with a callback.
///
/// This function does not return until the window is closed.
///
/// # Errors
/// Returns an error if the event loop fails to start.
pub fn run<F>(config: WindowConfig, callback: F) -> Result<(), WindowError>
where
    F: FnMut(&mut Window, WindowEvent),
{
    let event_loop = EventLoop::new()?;

    let mut handler = AppHandler {
        config,
        window: None,
        callback,
    };

    event_loop.run_app(&mut handler)?;
    Ok(())
}

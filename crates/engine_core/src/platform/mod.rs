//! Platform abstraction layer.
//!
//! Provides windowing, input handling, and platform-specific utilities.

mod window;

pub use window::{Window, WindowConfig, WindowEvent};

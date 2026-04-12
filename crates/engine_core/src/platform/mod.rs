//! Platform abstraction layer.
//!
//! Provides windowing, input handling, and platform-specific utilities.

mod input;
mod window;

pub use input::InputState;
pub use window::{Window, WindowConfig, WindowEvent};

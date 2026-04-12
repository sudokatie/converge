//! Platform abstraction layer.
//!
//! Provides windowing, input handling, and platform-specific utilities.

mod action_map;
mod input;
mod window;

pub use action_map::{Action, ActionMap, ActionMapError, KeyBinding};
pub use input::InputState;
pub use window::{Window, WindowConfig, WindowEvent};

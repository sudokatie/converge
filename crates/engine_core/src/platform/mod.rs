//! Platform abstraction layer.
//!
//! Provides windowing, input handling, and platform-specific utilities.

mod action_context;
mod action_map;
mod input;
mod timing;
mod window;

pub use action_context::{ActionContext, BindingConflict, ContextualActionMap};
pub use action_map::{Action, ActionMap, ActionMapError, KeyBinding};
pub use input::InputState;
pub use timing::{Clock, FixedTimestep};
pub use window::{Window, WindowConfig, WindowEvent};

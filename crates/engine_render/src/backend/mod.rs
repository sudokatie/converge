//! GPU backend abstraction.
//!
//! Provides low-level GPU device and surface management.

mod device;

pub use device::{FrameContext, RenderDevice, RenderDeviceError};

//! Camera system for 3D rendering.

#[expect(
    clippy::module_inception,
    reason = "camera module contains Camera struct, matching standard naming convention"
)]
mod camera;
mod controller;

pub use camera::Camera;
pub use controller::FirstPersonController;

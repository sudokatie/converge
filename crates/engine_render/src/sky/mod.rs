//! Procedural sky rendering system.
//!
//! Provides atmospheric scattering, sun/moon positioning, and day/night cycle.

mod sky_renderer;

#[cfg(test)]
mod sky_renderer_test;

pub use sky_renderer::SkyRenderer;

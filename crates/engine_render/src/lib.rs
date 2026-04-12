//! Rendering system for the Lattice game engine.
//!
//! Provides GPU abstraction, voxel rendering, and visual effects.

pub mod backend;
pub mod camera;
mod renderer;
pub mod voxel;

pub use renderer::{TriangleRenderer, Vertex};

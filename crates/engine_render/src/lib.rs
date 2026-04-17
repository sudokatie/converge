//! Rendering system for the Lattice game engine.
//!
//! Provides GPU abstraction, voxel rendering, and visual effects.

pub mod backend;
pub mod camera;
pub mod fog;
pub mod ghost_block;
pub mod lighting;
mod renderer;
pub mod sky;
pub mod voxel;

pub use renderer::{TriangleRenderer, Vertex};

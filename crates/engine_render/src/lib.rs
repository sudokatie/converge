//! Rendering system for the Lattice game engine.
//!
//! Provides GPU abstraction, voxel rendering, and visual effects.

pub mod backend;
mod renderer;

pub use renderer::{TriangleRenderer, Vertex};

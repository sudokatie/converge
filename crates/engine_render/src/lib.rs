//! Rendering system for the Lattice game engine.
//!
//! Provides GPU abstraction, voxel rendering, and visual effects.

pub mod backend;
pub mod camera;
pub mod distortion;
pub mod fog;
pub mod ghost_block;
pub mod lighting;
mod renderer;
pub mod sky;
pub mod volumetric;
pub mod voxel;

pub use distortion::{
    BlendMode, DistortionBatch, DistortionEffect, DistortionInstanceUniform, DistortionKind,
    DistortionPreset, DistortionQuality, DistortionRegion, DistortionRegionUniform,
    DistortionSampler, DistortionShape, FalloffCurve, FlowDirection, ScreenDistortion,
    ScreenDistortionUniform, compute_fingerprint, convert, create_from_preset, create_layered,
    exponential_falloff, linear_falloff, position_hash, radial_wave, sine_wave, smooth_falloff,
    sort_by_priority, spiral_wave,
};
pub use renderer::{TriangleRenderer, Vertex};
pub use volumetric::{
    LightShaft, LightShaftConfig, VolumeRegion, VolumeShape, VolumetricEffect, VolumetricEffectKind,
};

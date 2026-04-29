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
pub mod visibility;
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
pub use visibility::{
    ScreenVisibility, ScreenVisibilityUniform, VisibilityBatch, VisibilityBlendMode,
    VisibilityEffect, VisibilityEffectUniform, VisibilityFalloff, VisibilityInstanceUniform,
    VisibilityKind, VisibilityPreset, VisibilityQuality, VisibilityRegion, VisibilityRegionUniform,
    VisibilitySampler, VisibilityShape, bioluminescent_factor, bioluminescent_pulse,
    compute_fingerprint as compute_visibility_fingerprint,
    create_from_preset as create_visibility_from_preset,
    create_layered as create_visibility_layered, depth_visibility, position_hash_3d,
    sort_by_priority as sort_visibility_by_priority, visibility_from_distance,
    visibility_squared_exp,
};
pub use volumetric::{
    LightShaft, LightShaftConfig, VolumeRegion, VolumeShape, VolumetricEffect, VolumetricEffectKind,
};

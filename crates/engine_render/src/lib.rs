//! Rendering system for the Lattice game engine.
//!
//! Provides GPU abstraction, voxel rendering, and visual effects.

pub mod backend;
pub mod camera;
pub mod curvature;
pub mod distortion;
pub mod fog;
pub mod ghost_block;
pub mod lighting;
mod renderer;
pub mod sky;
pub mod visibility;
pub mod volumetric;
pub mod voxel;

pub use curvature::{
    CurvatureBatch, CurvatureBody, CurvatureBodyKind, CurvatureBodyUniform, CurvatureClipConfig,
    CurvatureClipUniform, CurvatureFadeConfig, CurvatureFogConfig, CurvatureFogUniform,
    CurvatureInstanceUniform, CurvatureRenderConfig, CurvatureSampler, HorizonConfig,
    HorizonConfigUniform, HorizonModel, HorizonModelUniform, HorizonQuality, angular_separation,
    atmospheric_fade, compute_fingerprint as compute_curvature_fingerprint, compute_tangent_frame,
    curvature_distance_correction, filter_active, find_dominant_body, flat_to_curved_direction,
    flat_to_curved_position, great_circle_distance, horizon_clip_distance, horizon_fog_density,
    horizon_visibility, line_of_sight, position_hash as curvature_position_hash, sort_by_distance,
    sort_by_radius, surface_forward, surface_normal,
};
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

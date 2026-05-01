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
pub mod post_process;
mod renderer;
pub mod sky;
pub mod visibility;
pub mod volumetric;
pub mod voxel;
pub mod weather;

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
pub use post_process::{
    BlendWeights, Environment, PostBatch, PostBlendMode, PostCameraState, PostEffect,
    PostEffectKind, PostEffectUniform, PostGlobalUniform, PostInstanceUniform, PostLayer,
    PostLayerStack, PostRegion, PostRegionShape, PostRegionUniform, PostSampler, RegionWeight,
    compute_fingerprint as compute_post_fingerprint,
    compute_layer_fingerprint as compute_post_layer_fingerprint,
    compute_stack_fingerprint as compute_post_stack_fingerprint, convert as convert_post,
    filter_active as filter_post_active, frame_jitter, group_by_environment, group_by_kind,
    halton_jitter, position_hash as post_position_hash, priorities as post_priorities,
    sort_by_execution_order, sort_by_priority as sort_post_by_priority,
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
pub use weather::{
    ColorOverTime, CurvePreset, CurvePreview, DistributionPreview, EmissionMode, EmitterConfig,
    EmitterConfigUniform, Keyframe, OverTimeCurve, ParticleBatch, ParticleInstance,
    ParticleSampler, SampledParticle, SimulationSpace, SpawnPlan, SpawnShape, SpawnShapeKind,
    SpawnShapeUniform, ValidationResult, ValueRange, VelocityMode, WeatherEffect,
    WeatherEffectUniform, WeatherKind, WeatherPreset, WeatherSummary, compute_effect_fingerprint,
    compute_emitter_fingerprint, compute_fingerprint as compute_weather_fingerprint,
    create_from_preset as create_weather_from_preset, create_layered as create_weather_layered,
    deserialize_config as deserialize_weather_config, filter_active as filter_weather_active,
    plan_spawns, position_hash as weather_position_hash, sample_turbulence,
    serialize_config as serialize_weather_config, sort_by_kind as sort_weather_by_kind,
    sort_by_spawn_rate,
};

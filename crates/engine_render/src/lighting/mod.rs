//! Lighting systems for the Lattice renderer.
//!
//! Provides directional lights (sun), point lights (torches), block light
//! propagation, and GPU uniform management.

mod block_light;
mod directional;
mod light_uniform;
mod light_update;
mod point_light;

pub use block_light::{BlockLightMap, LIGHT_MAX, LightValue};
pub use directional::DirectionalLight;
pub use light_uniform::{LightUniform, LightUniformBuffer};
pub use light_update::{
    LIGHT_INVALIDATION_RADIUS, LightUpdate, LightUpdateManager, affected_positions, is_affected,
};
pub use point_light::{MAX_POINT_LIGHTS, PointLight, PointLightManager};

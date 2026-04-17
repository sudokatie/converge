//! Lighting systems for the Lattice renderer.
//!
//! Provides directional lights (sun), point lights (torches), block light
//! propagation, and GPU uniform management.

mod block_light;
mod directional;
mod light_uniform;
mod point_light;

pub use block_light::{BlockLightMap, LightValue, LIGHT_MAX};
pub use directional::DirectionalLight;
pub use light_uniform::{LightUniform, LightUniformBuffer};
pub use point_light::{PointLight, PointLightManager, MAX_POINT_LIGHTS};

//! Volumetric fog and light shaft configuration.
//!
//! Provides CPU-side data primitives for rendering volumetric effects:
//! underwater caustics, blizzards, spore clouds, dust motes, and vacuum leaks.
//! These types configure the GPU volumetric rendering pass.

mod effect;
mod light_shaft;
mod volume_region;

pub use effect::{VolumetricEffect, VolumetricEffectKind};
pub use light_shaft::{LightShaft, LightShaftConfig};
pub use volume_region::{VolumeRegion, VolumeShape};

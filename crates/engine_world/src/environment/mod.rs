//! Unified environmental field system for scalar and vector fields per chunk.
//!
//! This module provides storage and simulation hooks for environmental fields
//! such as temperature, oxygen, pressure, radiation, toxicity, humidity,
//! corruption, and spore density (scalar), as well as wind, water current,
//! pressure gradient, gravity override, and hazard spread (vector).
//!
//! # Architecture
//!
//! ## Scalar Fields
//! - [`FieldChannel`]: Enum defining the 8 supported scalar field types
//! - [`ChunkFields`]: Per-chunk storage for all scalar field channels
//! - [`ChannelData`]: Storage for a single scalar channel within a chunk
//! - [`DiffusionConfig`]/[`AdvectionConfig`]: Scalar simulation configuration
//!
//! ## Vector Fields
//! - [`VectorFieldChannel`]: Enum defining the 5 supported vector field types
//! - [`ChunkVectorFields`]: Per-chunk storage for all vector field channels
//! - [`VectorChannelData`]: Storage for a single vector channel within a chunk
//! - [`VectorAdvectionConfig`]/[`VectorDecayConfig`]/[`VectorSmoothingConfig`]: Vector simulation configuration
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::environment::{ChunkFields, FieldChannel, ChunkVectorFields, VectorFieldChannel};
//! use engine_core::coords::LocalPos;
//! use glam::Vec3;
//!
//! // Scalar fields
//! let mut fields = ChunkFields::new();
//! assert_eq!(fields.get(FieldChannel::Temperature, LocalPos::new(0, 0, 0)), 20.0);
//! fields.set(FieldChannel::Radiation, LocalPos::new(5, 5, 5), 0.8);
//! let temp = fields.sample(FieldChannel::Temperature, 8.5, 8.5, 8.5);
//!
//! // Vector fields
//! let mut vec_fields = ChunkVectorFields::new();
//! assert_eq!(vec_fields.get(VectorFieldChannel::Wind, LocalPos::new(0, 0, 0)), Vec3::ZERO);
//! vec_fields.set(VectorFieldChannel::Wind, LocalPos::new(5, 5, 5), Vec3::new(1.0, 0.0, 0.5));
//! let wind = vec_fields.sample(VectorFieldChannel::Wind, 8.5, 8.5, 8.5);
//! ```

mod channel;
mod chunk_fields;
mod chunk_hazards;
mod chunk_vector_fields;
mod diffusion;
mod hazard_cell;
mod hazard_kind;
mod propagation;
mod propagation_config;
mod vector_channel;
mod vector_diffusion;

pub use channel::FieldChannel;
pub use chunk_fields::{ChannelData, ChunkFields};
pub use chunk_hazards::{ChunkHazards, HazardLayer};
pub use chunk_vector_fields::{ChunkVectorFields, VectorChannelData};
pub use diffusion::{
    AdvectionConfig, DiffusionConfig, DiffusionStep, FieldSimConfig, SimStepResult,
};
pub use hazard_cell::HazardCell;
pub use hazard_kind::HazardKind;
pub use propagation::{
    CellDelta, PropagationResult, ResistanceMap, apply_deltas, decay_step, propagation_step,
};
pub use propagation_config::{DecayConfig, PropagationConfig, Resistance, SpreadConfig};
pub use vector_channel::VectorFieldChannel;
pub use vector_diffusion::{
    VectorAdvectionConfig, VectorDecayConfig, VectorFieldSimConfig, VectorSmoothingConfig,
};

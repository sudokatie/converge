//! Unified environmental field system for scalar fields per chunk.
//!
//! This module provides storage and simulation hooks for environmental fields
//! such as temperature, oxygen, pressure, radiation, toxicity, humidity,
//! corruption, and spore density.
//!
//! # Architecture
//!
//! - [`FieldChannel`]: Enum defining the 8 supported field types
//! - [`ChunkFields`]: Per-chunk storage for all field channels
//! - [`ChannelData`]: Storage for a single channel within a chunk
//! - [`DiffusionConfig`]/[`AdvectionConfig`]: Simulation configuration
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::environment::{ChunkFields, FieldChannel};
//! use engine_core::coords::LocalPos;
//!
//! let mut fields = ChunkFields::new();
//!
//! // Fields return defaults when unallocated
//! assert_eq!(fields.get(FieldChannel::Temperature, LocalPos::new(0, 0, 0)), 20.0);
//!
//! // Setting a value allocates the channel
//! fields.set(FieldChannel::Radiation, LocalPos::new(5, 5, 5), 0.8);
//!
//! // Sample with interpolation
//! let temp = fields.sample(FieldChannel::Temperature, 8.5, 8.5, 8.5);
//! ```

mod channel;
mod chunk_fields;
mod diffusion;

pub use channel::FieldChannel;
pub use chunk_fields::{ChannelData, ChunkFields};
pub use diffusion::{
    AdvectionConfig, DiffusionConfig, DiffusionStep, FieldSimConfig, SimStepResult,
};

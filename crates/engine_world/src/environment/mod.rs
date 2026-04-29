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

mod atmosphere_cell;
mod atmosphere_config;
mod atmosphere_layer;
mod channel;
mod chunk_atmosphere;
mod chunk_fields;
mod chunk_hazards;
mod chunk_vector_fields;
mod diffusion;
mod hazard_cell;
mod hazard_kind;
mod materials;
mod propagation;
mod propagation_config;
mod vector_channel;
mod vector_diffusion;

mod chunk_fluids;
mod fluid_cell;
mod fluid_kind;
mod fluid_transport;

mod chunk_structural;
mod structural_cell;
mod structural_config;
mod structural_event;
mod structural_propagation;
mod support_kind;

mod chunk_conduits;
mod conduit_cell;
mod conduit_config;
mod conduit_kind;
mod conduit_network;
mod conduit_node;

mod gravity;
mod hazard_delta;
mod hazard_simulation;
mod rule_profile;

pub use atmosphere_cell::AtmosphereCell;
pub use atmosphere_config::{
    AtmosphereConfig, AtmosphereEffects, LayerNeighborCounts, TransitionRules, cell_from_material,
    layer_from_material,
};
pub use atmosphere_layer::AtmosphereLayer;
pub use channel::FieldChannel;
pub use chunk_atmosphere::{AtmosphereSample, ChunkAtmosphere};
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

pub use materials::{
    MaterialCategory, MaterialId, MaterialProperties, MaterialRegistry, MaterialRegistryError,
    hazard_integration,
};

pub use chunk_fluids::{ChunkFluids, FluidLayer, FluidSample};
pub use fluid_cell::{
    FluidCell, MAX_PRESSURE, MAX_TEMPERATURE, MAX_VOLUME, MIN_PRESSURE, MIN_TEMPERATURE, MIN_VOLUME,
};
pub use fluid_kind::FluidKind;
pub use fluid_transport::{
    BoundaryOutflow, FluidDelta, FluidResistanceMap, FluidTransportConfig, FluidTransportResult,
    apply_fluid_deltas, transport_step,
};

pub use chunk_structural::ChunkStructural;
pub use structural_cell::{
    FAILURE_THRESHOLD, MAX_LOAD, MAX_STRESS, OVERSTRESS_THRESHOLD, StructuralCell,
};
pub use structural_config::{
    LoadConfig, StabilityConfig, StructuralConfig, SupportPropagationConfig,
};
pub use structural_event::{StructuralBoundary, StructuralEvent, StructuralEventKind};
pub use structural_propagation::{
    PressureMap, StrengthMap, StructuralDelta, StructuralResult, apply_structural_deltas,
    check_decompression, detect_cavein, distribute_load, propagate_support, structural_step,
};
pub use support_kind::SupportKind;

pub use chunk_conduits::{ChunkConduits, ConduitLayer};
pub use conduit_cell::{
    ConduitCell, MAX_PRESSURE as CONDUIT_MAX_PRESSURE, MAX_STORED,
    MAX_TEMPERATURE as CONDUIT_MAX_TEMPERATURE, MIN_PRESSURE as CONDUIT_MIN_PRESSURE, MIN_STORED,
    MIN_TEMPERATURE as CONDUIT_MIN_TEMPERATURE,
};
pub use conduit_config::{ConduitNetworkConfig, FlowConfig, HeatTransferConfig, PressureConfig};
pub use conduit_kind::ConduitKind;
pub use conduit_network::{
    ConduitBoundary, ConduitDelta, ConduitNetworkResult, ConduitResistanceMap, ConnectedNetwork,
    apply_conduit_deltas, find_boundary_cells, find_networks, network_step,
};
pub use conduit_node::{ConduitNode, NodeRole};

pub use gravity::{
    GravityModel, GravityProfile, MAX_GRAVITY_MAGNITUDE, MIN_GRAVITY_MAGNITUDE, STANDARD_GRAVITY,
};
pub use hazard_delta::{
    ChunkHazardDelta, ChunkHazardSnapshot, HazardCellDelta, HazardDeltaJournal, HazardDeltaRecord,
    HazardSnapshot,
};
pub use hazard_simulation::{
    BoundarySpread, ChunkTickInput, ChunkTickOutput, HazardSimulator, SimulationTickResult,
    TickStats, apply_chunk_delta, apply_snapshot, simulate_chunk_tick,
};
pub use rule_profile::{
    ProfileError, ProfileId, ProfileRegistry, RuleBundle, RuleOverrides, WorldRuleProfile,
};

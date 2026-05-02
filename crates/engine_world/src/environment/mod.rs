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
mod cellular_automata;
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
mod sparse_fluid;

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

mod deformable_terrain;
mod gravity;
mod hazard_delta;
mod hazard_simulation;
mod regional_climate;
mod rule_profile;
mod thermal_radiation;

pub use atmosphere_cell::AtmosphereCell;
pub use atmosphere_config::{
    AtmosphereConfig, AtmosphereEffects, LayerNeighborCounts, TransitionRules, cell_from_material,
    layer_from_material,
};
pub use atmosphere_layer::AtmosphereLayer;
pub use cellular_automata::{
    AutomataCell, AutomataConfig, AutomataDelta, AutomataEntry, AutomataFingerprint, AutomataKind,
    AutomataPlan, AutomataPos, AutomataRegion, AutomataResistance, AutomataResult, AutomataRule,
    AutomataSummary, AutomataValidation, DeltaKind, Neighborhood, apply_automata_plan,
    automata_step, plan_automata_step,
};
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
pub use sparse_fluid::{
    FlowLink, PressureEqualizationPlan, PressureEqualizationStep, SparseFluidConfig,
    SparseFluidEntry, SparseFluidFingerprint, SparseFluidPos, SparseFluidRegion, SparseFluidResult,
    SparseFluidSummary, SparseFluidValidation, apply_equalization, apply_flows, compute_flow_links,
    plan_pressure_equalization, sparse_fluid_step,
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
pub use thermal_radiation::{
    DEFAULT_AMBIENT, DEFAULT_THERMAL_MASS, KELVIN_OFFSET, MAX_EMISSIVITY,
    MAX_TEMPERATURE as THERMAL_MAX_TEMP, MIN_EMISSIVITY, MIN_TEMPERATURE as THERMAL_MIN_TEMP,
    RadiationExchange, STEFAN_BOLTZMANN, ThermalCell, ThermalEntry, ThermalPos,
    ThermalRadiationConfig, ThermalRadiationFingerprint, ThermalRadiationRegion,
    ThermalRadiationResult, ThermalRadiationSummary, ThermalRadiationValidation,
    apply_ambient_exchange, apply_radiation_exchanges, compute_radiation_transfer,
    plan_radiation_exchanges, thermal_radiation_step,
};

pub use deformable_terrain::{
    DEFAULT_DUCTILITY, DEFAULT_FRACTURE_THRESHOLD, DEFAULT_HARDNESS, DeformableTerrainConfig,
    DeformableTerrainFingerprint, DeformableTerrainRegion, DeformableTerrainResult,
    DeformableTerrainSummary, DeformableTerrainValidation, FractureEvent, FractureLink, MAX_DAMAGE,
    MAX_DEFORMATION, MAX_DUCTILITY, MAX_FRACTURE_THRESHOLD, MAX_HARDNESS, MAX_STRAIN,
    MAX_STRESS as TERRAIN_MAX_STRESS, MIN_DUCTILITY, MIN_FRACTURE_THRESHOLD, MIN_HARDNESS,
    StressPropagation, TerrainCell, TerrainEntry, TerrainPos, apply_deformation_from_damage,
    apply_stress_propagation, apply_stress_relaxation, check_fractures, deformable_terrain_step,
    plan_stress_propagation, propagate_fractures,
};

pub use regional_climate::{
    BiomeType, ClimateCell, ClimateProjection, ClimateRegion, ClimateRegionId,
    DEFAULT_BASE_HUMIDITY, DEFAULT_BASE_MOISTURE, DEFAULT_BASE_PRESSURE, DEFAULT_BASE_TEMPERATURE,
    MAX_HUMIDITY, MAX_MOISTURE, MAX_PRESSURE as CLIMATE_MAX_PRESSURE,
    MAX_TEMPERATURE as CLIMATE_MAX_TEMP, MIN_HUMIDITY, MIN_MOISTURE,
    MIN_PRESSURE as CLIMATE_MIN_PRESSURE, MIN_TEMPERATURE as CLIMATE_MIN_TEMP, MoistureTransport,
    PrecipitationEvent, RegionNeighbors, RegionalClimateConfig, RegionalClimateFingerprint,
    RegionalClimateResult, RegionalClimateSnapshot, RegionalClimateSummary,
    RegionalClimateValidation, SeasonalCycle, WindVector, apply_evaporation,
    apply_moisture_transports, apply_precipitation, apply_seasonal_temperature,
    compute_precipitation, plan_moisture_transports, regional_climate_step,
};

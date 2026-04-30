//! In-world machine framework for crafting stations, processors, reactors,
//! incubators, and life support systems.
//!
//! Provides deterministic, serde-covered foundation for data-driven machines:
//!
//! - [`MachineId`] - Unique machine identifier
//! - [`MachineCategory`] - Machine type classification
//! - [`MachineConfig`] - Data-driven machine configuration
//! - [`MachineState`] - Runtime state with progress, faults, maintenance
//! - [`ProcessDefinition`] - Recipe/process with inputs, outputs, duration
//! - [`MachineRegistry`] - Central machine management
//!
//! # Determinism
//!
//! All operations are deterministic with stable ordering:
//! - Machines ordered by position and ID
//! - Processes ordered by definition order
//! - Events ordered by tick, machine, and event kind
//! - Fingerprints computed over ordered state
//!
//! # Categories
//!
//! - Crafting: manual assembly stations, workbenches
//! - Processor: automated refiners, smelters, chemical processors
//! - Reactor: power generators, fusion reactors
//! - Incubator: growth chambers, bioreactors
//! - Life support: scrubbers, pressurizers, HVAC systems

mod config;
mod identity;
mod registry;
mod state;
mod tick;

pub use config::{
    AtmosphereEffect, FluidPort, HeatConfig, MachineConfig, PortDirection, PowerConfig,
    ProcessDefinition, ProcessId, ResourceRequirement, ResourceYield,
};
pub use identity::{MachineCategory, MachineId, MachineTier};
pub use registry::{MachineRegistry, RegistryError, RegistryQuery, RegistrySummary};
pub use state::{
    FaultKind, MachineState, MaintenanceState, ProcessQueue, ProcessState, QueuedProcess,
};
pub use tick::{
    MachineEvent, MachineEventKind, MachineFingerprint, MachineTickResult, MachineTickStats,
};

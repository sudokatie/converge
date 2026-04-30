//! Geological layer simulation for pressure, magma, fault lines, and crystal seams.
//!
//! Provides deterministic, serde-covered simulation of subsurface geology:
//!
//! - [`LayerId`]/[`MaterialId`]/[`FeatureId`] - Typed identifiers
//! - [`GeologicalLayer`] - Strata definition with material composition
//! - [`GeologyState`] - Pressure, temperature, and stability fields
//! - [`MagmaPocket`]/[`MagmaFlow`] - Volcanic activity simulation
//! - [`FaultLine`] - Tectonic stress, slip, and earthquake generation
//! - [`CrystalSeam`]/[`MineralDeposit`] - Resource deposits
//! - [`GeologySimulator`] - Deterministic tick-based simulation
//!
//! # Determinism
//!
//! All operations are deterministic with stable ordering:
//! - Layers ordered by depth, then ID
//! - Features ordered by ID
//! - Events ordered by tick, then feature ID
//! - Fingerprints computed over ordered state
//!
//! # Simulation Model
//!
//! Each tick:
//! 1. Accumulate tectonic stress on fault lines
//! 2. Propagate heat from magma to surrounding layers
//! 3. Update magma flow based on pressure differentials
//! 4. Check fault slip thresholds and trigger earthquakes
//! 5. Update crystal growth based on temperature/pressure

mod config;
mod crystal;
mod fault;
mod fingerprint;
mod identity;
mod layer;
mod magma;
mod state;
mod tick;

pub use config::{CrystalGrowthConfig, FaultConfig, GeologyConfig, MagmaConfig, ThermalConfig};
pub use crystal::{CrystalSeam, CrystalType, MineralDeposit, MineralType};
pub use fault::{FaultLine, FaultType, QuakeEvent, SlipState, StressAccumulator};
pub use fingerprint::{GeologyChecksum, GeologyFingerprint};
pub use identity::{FeatureId, FeatureKind, LayerId, MaterialId, RockType};
pub use layer::{GeologicalLayer, LayerBoundary, Stratum};
pub use magma::{MagmaFlow, MagmaPocket, MagmaState, VolcanicEvent, VolcanicEventKind};
pub use state::{GeologyFields, PressureField, StabilityField, TemperatureField};
pub use tick::{
    GeologyEvent, GeologyEventKind, GeologySimulator, GeologySummary, GeologyTickResult,
    GeologyTickStats, ProjectionResult,
};

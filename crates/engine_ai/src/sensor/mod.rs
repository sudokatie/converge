//! Sensor framework for AI perception of stimuli.
//!
//! Provides a generic system for creatures to sense their environment through
//! multiple channels: sight, sound, vibration, smell, heat, pressure, and
//! electrical fields. Supports configurable attenuation, occlusion, thresholds,
//! memory decay, and deterministic ordering for replay/network sync.

mod config;
mod kind;
mod observation;
mod stimulus;
mod suite;
mod summary;

pub use config::{AttenuationCurve, DetectionStrength, OcclusionModel, SensorConfig, SensorSpec};
pub use kind::SensorKind;
pub use observation::{
    MemoryConfig, Observation, ObservationId, ObservationMemory, ObservationPriority,
    ObservationSet,
};
pub use stimulus::{Stimulus, StimulusEmitter, StimulusId, StimulusSource};
pub use suite::{SensorProfile, SensorProfileId, SensorSuite};
pub use summary::{SensorSnapshot, SensorSummary, StimulusSummary};

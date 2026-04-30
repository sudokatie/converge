//! Narrative event hook system for world storytelling.
//!
//! This module provides a deterministic system for triggering narrative events
//! such as disasters, radio chatter, timed objectives, and anomaly sightings.
//!
//! # Architecture
//!
//! ## Event Types
//! - [`NarrativeEventKind`]: Categories of narrative events (disaster, radio, objective, anomaly)
//! - [`NarrativeEvent`]: A scheduled or triggered narrative event instance
//!
//! ## Triggers
//! - [`NarrativeTrigger`]: Conditions that activate narrative events
//! - [`TriggerPredicate`]: Composable predicates for complex trigger logic
//!
//! ## Registration & Definitions
//! - [`EventDefinition`]: Static template for narrative event types
//! - [`EventRegistry`]: Central registry of all event definitions
//!
//! ## Runtime
//! - [`NarrativeState`]: Active runtime state for the narrative system
//! - [`NarrativeOutput`]: Emitted narrative content for UI/audio systems
//! - [`OutputQueue`]: Queued narrative outputs awaiting consumption
//!
//! ## Timing
//! - [`CooldownConfig`]: Cooldown and repeat timing configuration
//! - [`TimedObjective`]: Objective with deadline tracking
//!
//! ## Presets
//! - [`DisasterPreset`], [`RadioPreset`], [`ObjectivePreset`], [`AnomalyPreset`]: Built-in event templates
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::narrative::{
//!     EventRegistry, EventDefinition, NarrativeEventKind, NarrativeTrigger,
//!     NarrativeState, CooldownConfig,
//! };
//!
//! // Create registry with definitions
//! let mut registry = EventRegistry::new();
//! let def = EventDefinition::new("meteor_strike", NarrativeEventKind::Disaster)
//!     .with_trigger(NarrativeTrigger::time_elapsed(10_000))
//!     .with_cooldown(CooldownConfig::once());
//! registry.register(def);
//!
//! // Create runtime state
//! let mut state = NarrativeState::new(&registry);
//!
//! // Tick and collect outputs
//! let outputs = state.tick(current_tick, &context);
//! for output in outputs {
//!     // Handle narrative output (display text, play audio, etc.)
//! }
//! ```

mod cooldown;
mod definition;
mod event_kind;
mod fingerprint;
mod output;
mod preset;
mod runtime;
mod trigger;

pub use cooldown::{CooldownConfig, CooldownState, ObjectiveStatus, RepeatMode, TimedObjective};
pub use definition::{EventDefinition, EventRegistry, RegistryError};
pub use event_kind::NarrativeEventKind;
pub use fingerprint::{ChecksumBuilder, EventFingerprint, StateChecksum};
pub use output::{NarrativeOutput, OutputKind, OutputPriority, OutputQueue};
pub use preset::{AnomalyPreset, DisasterPreset, ObjectivePreset, Preset, RadioPreset};
pub use runtime::{ActiveEvent, EventId, NarrativeContext, NarrativeState, TickResult};
pub use trigger::{NarrativeTrigger, TriggerKind, TriggerPredicate, TriggerResult};

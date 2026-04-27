//! Global world-state controller for world-level events and scheduling.
//!
//! This module provides a deterministic timeline for global events that affect
//! world simulation, including eclipses, collapses, season shifts, biome corruption,
//! and migration waves.
//!
//! # Architecture
//!
//! ## Event Types
//! - [`WorldEventKind`]: Enum defining the 5 supported world event types
//! - [`Season`]: The four seasons with environmental modifiers
//!
//! ## Scheduling
//! - [`WorldEvent`]: A scheduled event with timing and optional region
//! - [`WorldTimeline`]: Deterministic timeline managing all events
//! - [`TimelineConfig`]: Configuration for timeline behavior
//!
//! ## Active Effects
//! - [`ActiveEffect`]: A currently-active effect at a position
//! - [`ActiveEffects`]: Collection of effects for querying
//!
//! ## System Hints
//! - [`WorldStateHints`]: Combined hints for all systems
//! - [`LightingHint`], [`TemperatureHint`], [`StructuralHint`], etc.
//!
//! # Usage
//!
//! ```ignore
//! use engine_world::world_state::{WorldTimeline, TimelineConfig, WorldEventKind, WorldStateHints};
//! use engine_core::coords::ChunkPos;
//!
//! // Create timeline with default config
//! let mut timeline = WorldTimeline::default();
//!
//! // Schedule a global eclipse event
//! let eclipse_id = timeline.schedule_global(WorldEventKind::Eclipse, 1000, 500);
//!
//! // Schedule a regional collapse
//! let center = ChunkPos::new(100, 0, 100);
//! let collapse_id = timeline.schedule_regional(
//!     WorldEventKind::Collapse, 500, 200, center, 10
//! );
//!
//! // Advance time and query effects
//! timeline.advance(1200);
//! let effects = timeline.query_effects(ChunkPos::new(0, 0, 0));
//!
//! // Derive hints for other systems
//! let hints = WorldStateHints::from_effects(&effects, timeline.current_season());
//! ```

mod active_effect;
mod event_kind;
mod season;
mod state_hints;
mod timeline;
mod world_event;

pub use active_effect::{ActiveEffect, ActiveEffects};
pub use event_kind::WorldEventKind;
pub use season::Season;
pub use state_hints::{
    EntityHint, HazardHint, LightingHint, StructuralHint, TemperatureHint, WorldStateHints,
};
pub use timeline::{TimelineConfig, WorldTimeline};
pub use world_event::WorldEvent;

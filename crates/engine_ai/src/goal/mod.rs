//! Goal utility system for AI decision-making.
//!
//! Provides a data-driven framework for survival-oriented goal selection using
//! utility curves, weighted considerations, and deterministic scoring. Integrates
//! with needs, sensors, and faction systems through lightweight context adapters.
//!
//! # Architecture
//!
//! The system consists of several components:
//!
//! - [`GoalId`]: Stable identifiers for goal types
//! - [`GoalDef`]: Definitions with base priority, considerations, and metadata
//! - [`UtilityCurve`]: Response curves mapping input values to utility scores
//! - [`Consideration`]: Individual scoring factors with input bindings
//! - [`GoalContext`]: Snapshot of needs, sensors, and faction state for evaluation
//! - [`GoalSelector`]: Stateful selector with cooldown, inertia, and hysteresis
//! - [`GoalSummary`]: Cheap aggregate for unloaded chunks
//!
//! # Example
//!
//! ```ignore
//! use engine_ai::goal::{GoalDef, GoalSelector, GoalContext, preset};
//!
//! // Create a selector with preset survival goals
//! let mut selector = GoalSelector::new();
//! for goal in preset::survival_goals() {
//!     selector.register(goal);
//! }
//!
//! // Build context from creature state
//! let context = GoalContext::builder()
//!     .with_needs(&creature.needs)
//!     .with_sensor_summary(&sensor_summary)
//!     .build();
//!
//! // Evaluate and select best goal
//! let result = selector.evaluate(&context);
//! println!("Selected: {:?} (score: {})", result.selected.id, result.selected.score);
//! ```

mod consideration;
mod context;
mod curve;
mod definition;
mod preset;
mod scoring;
mod selector;
mod summary;

pub use consideration::{Consideration, ConsiderationId, InputBinding};
pub use context::{ContextFact, GoalContext, GoalContextBuilder};
pub use curve::{CurveKind, UtilityCurve};
pub use definition::{GoalDef, GoalId, GoalTag};
pub use preset::{
    preset_cool_down, preset_defend_territory, preset_flee_danger, preset_follow_leader,
    preset_idle, preset_investigate_stimulus, preset_patrol, preset_rest, preset_satisfy_hunger,
    preset_seek_allies, preset_seek_oxygen, preset_seek_water, preset_warm_up, survival_goals,
};
pub use scoring::{ConsiderationScore, GoalScore, ScoringBreakdown};
pub use selector::{
    CooldownConfig, GoalSelection, GoalSelector, HysteresisConfig, InertiaConfig, SelectionReason,
};
pub use summary::{GoalSnapshot, GoalSummary};

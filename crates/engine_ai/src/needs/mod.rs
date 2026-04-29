//! Needs and status simulation for creatures and NPC colonies.
//!
//! Provides a generic framework for simulating creature needs (hunger, thirst,
//! oxygen, warmth, rest, morale) and status effects. Supports per-creature
//! profiles, deterministic updates, and cheap colony-level aggregation.
//!
//! The `framework` submodule provides a data-driven status effect system with
//! configurable stacking behaviors, decay modes, immunity tracking, and
//! environmental triggers for automatic effect application.

mod effect;
mod framework;
mod need;
mod profile;
mod summary;

pub use effect::{StatusEffect, StatusEffectId, StatusModifier, StatusSet};
pub use framework::{
    ApplyResult, DecayMode, EffectCategory, EnvironmentSnapshot, EnvironmentalTrigger, ImmunitySet,
    ManagedStatusSet, ModifierDef, StackingBehavior, StatusEffectDef, StatusEffectRegistry,
    evaluate_trigger, find_triggered_effects, presets,
};
pub use need::{Need, NeedEvent, NeedId, NeedSet, NeedState, Threshold, ThresholdKind};
pub use profile::{NeedConfig, NeedProfile, ProfileId};
pub use summary::{ColonySnapshot, ColonySummary, NeedHistogram};

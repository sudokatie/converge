//! Needs and status simulation for creatures and NPC colonies.
//!
//! Provides a generic framework for simulating creature needs (hunger, thirst,
//! oxygen, warmth, rest, morale) and status effects. Supports per-creature
//! profiles, deterministic updates, and cheap colony-level aggregation.

mod effect;
mod need;
mod profile;
mod summary;

pub use effect::{StatusEffect, StatusEffectId, StatusModifier, StatusSet};
pub use need::{Need, NeedEvent, NeedId, NeedSet, NeedState, Threshold, ThresholdKind};
pub use profile::{NeedConfig, NeedProfile, ProfileId};
pub use summary::{ColonySnapshot, ColonySummary, NeedHistogram};

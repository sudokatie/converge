//! Faction, reputation, and territory system for cross-game state.
//!
//! Provides a framework for faction identity, inter-faction diplomacy,
//! actor-specific reputation tracking, and territorial claims with
//! influence/contestation. Supports deterministic ordering for replay/sync.

mod core;
mod diplomacy;
mod reputation;
mod summary;
mod territory;

pub use core::{Faction, FactionId, FactionRegistry, FactionTag};
pub use diplomacy::{DiplomacyTable, FactionMembership, MembershipKind, Stance, StanceTable};
pub use reputation::{
    ReputationConfig, ReputationDelta, ReputationEvent, ReputationHistory, ReputationSet,
    ReputationTier, Standing,
};
pub use summary::{FactionSnapshot, FactionSummary, TerritorySnapshot};
pub use territory::{
    Claim, ClaimKind, ClaimStrength, Influence, OwnershipStatus, Region, RegionId, TerritoryMap,
};

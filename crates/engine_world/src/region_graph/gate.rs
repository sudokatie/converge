//! Progression gates and tier definitions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Progression tier for gated content.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct ProgressionTier(pub u8);

impl ProgressionTier {
    /// Starting tier (always accessible).
    pub const START: Self = Self(0);

    /// Maximum supported tier.
    pub const MAX: Self = Self(15);

    /// Create a new tier.
    #[must_use]
    pub const fn new(tier: u8) -> Self {
        Self(tier)
    }

    /// Get the tier value.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Check if this tier is accessible at the given player tier.
    #[must_use]
    pub const fn is_accessible_at(self, player_tier: Self) -> bool {
        self.0 <= player_tier.0
    }

    /// Get the next tier.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        if self.0 < Self::MAX.0 {
            Some(Self(self.0 + 1))
        } else {
            None
        }
    }
}

impl std::fmt::Display for ProgressionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

/// Type of gate requirement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateKind {
    /// Requires minimum progression tier.
    Tier(ProgressionTier),
    /// Requires a specific key item.
    Key(String),
    /// Requires completing a mission.
    Mission(String),
    /// Requires defeating an enemy or boss.
    Defeat(String),
    /// Requires a minimum skill level.
    Skill { skill: String, level: u8 },
    /// Requires a specific resource amount.
    Resource { resource: String, amount: u32 },
    /// Requires multiple conditions (all must be met).
    All(Vec<GateKind>),
    /// Requires any of the conditions.
    Any(Vec<GateKind>),
}

impl GateKind {
    /// Create a tier requirement.
    #[must_use]
    pub const fn tier(tier: u8) -> Self {
        Self::Tier(ProgressionTier::new(tier))
    }

    /// Create a key requirement.
    #[must_use]
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key(key.into())
    }

    /// Create a mission requirement.
    #[must_use]
    pub fn mission(mission: impl Into<String>) -> Self {
        Self::Mission(mission.into())
    }

    /// Create a defeat requirement.
    #[must_use]
    pub fn defeat(enemy: impl Into<String>) -> Self {
        Self::Defeat(enemy.into())
    }

    /// Create a skill requirement.
    #[must_use]
    pub fn skill(skill: impl Into<String>, level: u8) -> Self {
        Self::Skill {
            skill: skill.into(),
            level,
        }
    }

    /// Create a resource requirement.
    #[must_use]
    pub fn resource(resource: impl Into<String>, amount: u32) -> Self {
        Self::Resource {
            resource: resource.into(),
            amount,
        }
    }

    /// Combine with another requirement (all must be met).
    #[must_use]
    pub fn and(self, other: Self) -> Self {
        match self {
            Self::All(mut conditions) => {
                conditions.push(other);
                Self::All(conditions)
            }
            _ => Self::All(vec![self, other]),
        }
    }

    /// Combine with another requirement (any may be met).
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::Any(mut conditions) => {
                conditions.push(other);
                Self::Any(conditions)
            }
            _ => Self::Any(vec![self, other]),
        }
    }

    /// Get the minimum tier required by this gate.
    #[must_use]
    pub fn min_tier(&self) -> ProgressionTier {
        match self {
            Self::Tier(tier) => *tier,
            Self::All(conditions) => conditions
                .iter()
                .map(Self::min_tier)
                .max()
                .unwrap_or(ProgressionTier::START),
            Self::Any(conditions) => conditions
                .iter()
                .map(Self::min_tier)
                .min()
                .unwrap_or(ProgressionTier::START),
            _ => ProgressionTier::START,
        }
    }

    /// Collect all keys required by this gate.
    pub fn required_keys(&self) -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        self.collect_keys(&mut keys);
        keys
    }

    fn collect_keys(&self, keys: &mut BTreeSet<String>) {
        match self {
            Self::Key(key) => {
                keys.insert(key.clone());
            }
            Self::All(conditions) | Self::Any(conditions) => {
                for cond in conditions {
                    cond.collect_keys(keys);
                }
            }
            _ => {}
        }
    }
}

impl Default for GateKind {
    fn default() -> Self {
        Self::Tier(ProgressionTier::START)
    }
}

/// A gate that restricts access to a region or edge.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRequirement {
    /// The requirement to pass this gate.
    pub kind: GateKind,
    /// Whether the gate is currently unlocked.
    pub unlocked: bool,
    /// Display name for the gate.
    pub name: String,
    /// Description of what's needed.
    pub description: String,
}

impl GateRequirement {
    /// Create a new gate requirement.
    #[must_use]
    pub fn new(kind: GateKind) -> Self {
        Self {
            kind,
            unlocked: false,
            name: String::new(),
            description: String::new(),
        }
    }

    /// Create a tier-gated requirement.
    #[must_use]
    pub fn tier(tier: u8) -> Self {
        Self::new(GateKind::tier(tier))
    }

    /// Create a key-gated requirement.
    #[must_use]
    pub fn key(key: impl Into<String>) -> Self {
        Self::new(GateKind::key(key))
    }

    /// Set the name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as unlocked.
    #[must_use]
    pub fn unlocked(mut self) -> Self {
        self.unlocked = true;
        self
    }

    /// Check if accessible (either unlocked or has no tier requirement).
    #[must_use]
    pub fn is_accessible(&self, player_tier: ProgressionTier) -> bool {
        self.unlocked || self.kind.min_tier().is_accessible_at(player_tier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progression_tier_accessibility() {
        let t0 = ProgressionTier::new(0);
        let t1 = ProgressionTier::new(1);
        let t2 = ProgressionTier::new(2);

        assert!(t0.is_accessible_at(t0));
        assert!(t0.is_accessible_at(t1));
        assert!(!t2.is_accessible_at(t1));
    }

    #[test]
    fn progression_tier_next() {
        let t0 = ProgressionTier::START;
        assert_eq!(t0.next(), Some(ProgressionTier::new(1)));
        assert_eq!(ProgressionTier::MAX.next(), None);
    }

    #[test]
    fn gate_kind_min_tier() {
        let gate = GateKind::tier(3);
        assert_eq!(gate.min_tier(), ProgressionTier::new(3));

        let combined = GateKind::tier(2).and(GateKind::tier(5));
        assert_eq!(combined.min_tier(), ProgressionTier::new(5));

        let any = GateKind::tier(2).or(GateKind::tier(5));
        assert_eq!(any.min_tier(), ProgressionTier::new(2));
    }

    #[test]
    fn gate_kind_keys() {
        let gate = GateKind::key("red").and(GateKind::key("blue"));
        let keys = gate.required_keys();
        assert!(keys.contains("red"));
        assert!(keys.contains("blue"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn gate_requirement_accessibility() {
        let gate = GateRequirement::tier(3);
        assert!(!gate.is_accessible(ProgressionTier::new(2)));
        assert!(gate.is_accessible(ProgressionTier::new(3)));
        assert!(gate.is_accessible(ProgressionTier::new(4)));

        let unlocked = GateRequirement::tier(10).unlocked();
        assert!(unlocked.is_accessible(ProgressionTier::new(0)));
    }

    #[test]
    fn serde_roundtrip() {
        let gate = GateRequirement::new(GateKind::tier(2).and(GateKind::key("red")))
            .with_name("Red Door")
            .with_description("Requires tier 2 and red key");

        let json = serde_json::to_string(&gate).unwrap();
        let recovered: GateRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(gate, recovered);
    }
}

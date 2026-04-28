//! Status effects and modifiers for needs.

use super::NeedId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identifier for a status effect.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StatusEffectId(pub String);

impl StatusEffectId {
    pub const WELL_FED: &'static str = "well_fed";
    pub const STARVING: &'static str = "starving";
    pub const DEHYDRATED: &'static str = "dehydrated";
    pub const HYPOTHERMIA: &'static str = "hypothermia";
    pub const HYPERTHERMIA: &'static str = "hyperthermia";
    pub const EXHAUSTED: &'static str = "exhausted";
    pub const RESTED: &'static str = "rested";
    pub const POISONED: &'static str = "poisoned";
    pub const INSPIRED: &'static str = "inspired";
    pub const PANICKED: &'static str = "panicked";

    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for StatusEffectId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// A modifier that a status effect applies to a need.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusModifier {
    /// Which need this modifier affects.
    pub need_id: NeedId,
    /// Multiplier for decay rate (1.0 = no change, 2.0 = double decay).
    pub decay_multiplier: f32,
    /// Multiplier for recovery rate.
    pub recovery_multiplier: f32,
    /// Flat delta applied each tick (can be positive or negative).
    pub tick_delta: f32,
}

impl StatusModifier {
    /// Create a modifier that increases decay rate.
    #[must_use]
    pub fn increased_decay(need_id: NeedId, multiplier: f32) -> Self {
        Self {
            need_id,
            decay_multiplier: multiplier,
            recovery_multiplier: 1.0,
            tick_delta: 0.0,
        }
    }

    /// Create a modifier that decreases decay rate.
    #[must_use]
    pub fn decreased_decay(need_id: NeedId, multiplier: f32) -> Self {
        Self {
            need_id,
            decay_multiplier: multiplier,
            recovery_multiplier: 1.0,
            tick_delta: 0.0,
        }
    }

    /// Create a modifier that increases recovery rate.
    #[must_use]
    pub fn increased_recovery(need_id: NeedId, multiplier: f32) -> Self {
        Self {
            need_id,
            decay_multiplier: 1.0,
            recovery_multiplier: multiplier,
            tick_delta: 0.0,
        }
    }

    /// Create a modifier with a flat tick delta (e.g., poison damage).
    #[must_use]
    pub fn tick_damage(need_id: NeedId, delta: f32) -> Self {
        Self {
            need_id,
            decay_multiplier: 1.0,
            recovery_multiplier: 1.0,
            tick_delta: delta,
        }
    }
}

/// A status effect with duration and modifiers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusEffect {
    /// Unique identifier for this effect type.
    pub id: StatusEffectId,
    /// Remaining duration in ticks (None = permanent until removed).
    pub remaining_ticks: Option<u64>,
    /// Stack count (for stackable effects).
    pub stacks: u32,
    /// Maximum stacks allowed.
    pub max_stacks: u32,
    /// Modifiers this effect applies.
    pub modifiers: Vec<StatusModifier>,
    /// Whether this effect is currently active.
    pub active: bool,
}

impl StatusEffect {
    /// Create a new status effect.
    #[must_use]
    pub fn new(id: StatusEffectId, duration_ticks: Option<u64>) -> Self {
        Self {
            id,
            remaining_ticks: duration_ticks,
            stacks: 1,
            max_stacks: 1,
            modifiers: Vec::new(),
            active: true,
        }
    }

    /// Create a stackable status effect.
    #[must_use]
    pub fn stackable(id: StatusEffectId, duration_ticks: Option<u64>, max_stacks: u32) -> Self {
        Self {
            id,
            remaining_ticks: duration_ticks,
            stacks: 1,
            max_stacks,
            modifiers: Vec::new(),
            active: true,
        }
    }

    /// Add a modifier to this effect.
    #[must_use]
    pub fn with_modifier(mut self, modifier: StatusModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    /// Check if the effect has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.remaining_ticks == Some(0)
    }

    /// Check if the effect is permanent.
    #[must_use]
    pub fn is_permanent(&self) -> bool {
        self.remaining_ticks.is_none()
    }

    /// Add stacks (up to max).
    pub fn add_stacks(&mut self, count: u32) {
        self.stacks = (self.stacks + count).min(self.max_stacks);
    }

    /// Remove stacks (returns true if still has stacks).
    pub fn remove_stacks(&mut self, count: u32) -> bool {
        self.stacks = self.stacks.saturating_sub(count);
        self.stacks > 0
    }

    /// Tick the effect, reducing duration. Returns true if still active.
    pub fn tick(&mut self) -> bool {
        if let Some(ref mut remaining) = self.remaining_ticks {
            *remaining = remaining.saturating_sub(1);
            *remaining > 0
        } else {
            true
        }
    }

    /// Get the effective decay multiplier for a need.
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "stacks bounded by max_stacks, safe for powi"
    )]
    pub fn decay_multiplier_for(&self, need_id: &NeedId) -> f32 {
        self.modifiers
            .iter()
            .filter(|m| &m.need_id == need_id)
            .map(|m| m.decay_multiplier.powi(self.stacks as i32))
            .product()
    }

    /// Get the effective recovery multiplier for a need.
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "stacks bounded by max_stacks, safe for powi"
    )]
    pub fn recovery_multiplier_for(&self, need_id: &NeedId) -> f32 {
        self.modifiers
            .iter()
            .filter(|m| &m.need_id == need_id)
            .map(|m| m.recovery_multiplier.powi(self.stacks as i32))
            .product()
    }

    /// Get the total tick delta for a need.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "stacks count precision loss acceptable"
    )]
    pub fn tick_delta_for(&self, need_id: &NeedId) -> f32 {
        self.modifiers
            .iter()
            .filter(|m| &m.need_id == need_id)
            .map(|m| m.tick_delta * self.stacks as f32)
            .sum()
    }
}

/// A collection of active status effects on a creature.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatusSet {
    effects: BTreeMap<StatusEffectId, StatusEffect>,
}

impl StatusSet {
    /// Create a new empty status set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or refresh an effect. Returns the resulting stack count.
    pub fn apply(&mut self, effect: StatusEffect) -> u32 {
        if let Some(existing) = self.effects.get_mut(&effect.id) {
            existing.add_stacks(effect.stacks);
            if let (Some(new_dur), Some(exist_dur)) =
                (effect.remaining_ticks, &mut existing.remaining_ticks)
            {
                *exist_dur = (*exist_dur).max(new_dur);
            }
            existing.active = true;
            existing.stacks
        } else {
            let stacks = effect.stacks;
            self.effects.insert(effect.id.clone(), effect);
            stacks
        }
    }

    /// Remove an effect entirely.
    pub fn remove(&mut self, id: &StatusEffectId) -> Option<StatusEffect> {
        self.effects.remove(id)
    }

    /// Remove stacks from an effect. Returns true if effect still exists.
    pub fn remove_stacks(&mut self, id: &StatusEffectId, count: u32) -> bool {
        if let Some(effect) = self.effects.get_mut(id) {
            if !effect.remove_stacks(count) {
                self.effects.remove(id);
                return false;
            }
            true
        } else {
            false
        }
    }

    /// Get an effect by ID.
    #[must_use]
    pub fn get(&self, id: &StatusEffectId) -> Option<&StatusEffect> {
        self.effects.get(id)
    }

    /// Get a mutable effect by ID.
    pub fn get_mut(&mut self, id: &StatusEffectId) -> Option<&mut StatusEffect> {
        self.effects.get_mut(id)
    }

    /// Check if an effect is active.
    #[must_use]
    pub fn has(&self, id: &StatusEffectId) -> bool {
        self.effects.get(id).is_some_and(|e| e.active)
    }

    /// Get the number of active effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Iterate over all effects.
    pub fn iter(&self) -> impl Iterator<Item = &StatusEffect> {
        self.effects.values()
    }

    /// Tick all effects, removing expired ones. Returns IDs of removed effects.
    pub fn tick(&mut self) -> Vec<StatusEffectId> {
        let mut removed = Vec::new();

        self.effects.retain(|id, effect| {
            if effect.tick() {
                true
            } else {
                removed.push(id.clone());
                false
            }
        });

        removed
    }

    /// Get combined decay multiplier for a need from all effects.
    #[must_use]
    pub fn combined_decay_multiplier(&self, need_id: &NeedId) -> f32 {
        self.effects
            .values()
            .filter(|e| e.active)
            .map(|e| e.decay_multiplier_for(need_id))
            .product()
    }

    /// Get combined recovery multiplier for a need from all effects.
    #[must_use]
    pub fn combined_recovery_multiplier(&self, need_id: &NeedId) -> f32 {
        self.effects
            .values()
            .filter(|e| e.active)
            .map(|e| e.recovery_multiplier_for(need_id))
            .product()
    }

    /// Get combined tick delta for a need from all effects.
    #[must_use]
    pub fn combined_tick_delta(&self, need_id: &NeedId) -> f32 {
        self.effects
            .values()
            .filter(|e| e.active)
            .map(|e| e.tick_delta_for(need_id))
            .sum()
    }

    /// Get all effect IDs.
    pub fn effect_ids(&self) -> impl Iterator<Item = &StatusEffectId> {
        self.effects.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_effect_id() {
        let id = StatusEffectId::new("test_effect");
        assert_eq!(id.as_str(), "test_effect");
    }

    #[test]
    fn test_status_modifier_increased_decay() {
        let modifier = StatusModifier::increased_decay(NeedId::hunger(), 2.0);

        assert_eq!(modifier.need_id, NeedId::hunger());
        assert!((modifier.decay_multiplier - 2.0).abs() < f32::EPSILON);
        assert!((modifier.recovery_multiplier - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_status_effect_new() {
        let effect = StatusEffect::new(StatusEffectId::new("test"), Some(100));

        assert_eq!(effect.stacks, 1);
        assert_eq!(effect.remaining_ticks, Some(100));
        assert!(effect.active);
        assert!(!effect.is_expired());
    }

    #[test]
    fn test_status_effect_tick() {
        let mut effect = StatusEffect::new(StatusEffectId::new("test"), Some(2));

        assert!(effect.tick());
        assert_eq!(effect.remaining_ticks, Some(1));

        assert!(!effect.tick());
        assert_eq!(effect.remaining_ticks, Some(0));
        assert!(effect.is_expired());
    }

    #[test]
    fn test_status_effect_permanent() {
        let mut effect = StatusEffect::new(StatusEffectId::new("test"), None);

        assert!(effect.is_permanent());
        assert!(effect.tick());
        assert!(effect.tick());
        assert!(!effect.is_expired());
    }

    #[test]
    fn test_status_effect_stacking() {
        let mut effect = StatusEffect::stackable(StatusEffectId::new("test"), Some(100), 5);

        effect.add_stacks(2);
        assert_eq!(effect.stacks, 3);

        effect.add_stacks(10);
        assert_eq!(effect.stacks, 5);

        assert!(effect.remove_stacks(2));
        assert_eq!(effect.stacks, 3);

        assert!(!effect.remove_stacks(5));
        assert_eq!(effect.stacks, 0);
    }

    #[test]
    fn test_status_effect_multipliers() {
        let effect = StatusEffect::stackable(StatusEffectId::new("poison"), Some(100), 3)
            .with_modifier(StatusModifier::increased_decay(NeedId::hunger(), 1.5));

        let mult = effect.decay_multiplier_for(&NeedId::hunger());
        assert!((mult - 1.5).abs() < f32::EPSILON);

        let mut stacked = effect.clone();
        stacked.stacks = 2;
        let mult2 = stacked.decay_multiplier_for(&NeedId::hunger());
        assert!((mult2 - 2.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_status_set_apply() {
        let mut set = StatusSet::new();

        let effect = StatusEffect::new(StatusEffectId::new("test"), Some(100));
        let stacks = set.apply(effect);

        assert_eq!(stacks, 1);
        assert!(set.has(&StatusEffectId::new("test")));
    }

    #[test]
    fn test_status_set_apply_stacking() {
        let mut set = StatusSet::new();

        let effect1 = StatusEffect::stackable(StatusEffectId::new("test"), Some(100), 5);
        set.apply(effect1);

        let effect2 = StatusEffect::stackable(StatusEffectId::new("test"), Some(100), 5);
        let stacks = set.apply(effect2);

        assert_eq!(stacks, 2);
    }

    #[test]
    fn test_status_set_tick_removes_expired() {
        let mut set = StatusSet::new();

        set.apply(StatusEffect::new(StatusEffectId::new("short"), Some(1)));
        set.apply(StatusEffect::new(StatusEffectId::new("long"), Some(100)));

        let removed = set.tick();

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], StatusEffectId::new("short"));
        assert!(!set.has(&StatusEffectId::new("short")));
        assert!(set.has(&StatusEffectId::new("long")));
    }

    #[test]
    fn test_status_set_combined_multipliers() {
        let mut set = StatusSet::new();

        let effect1 = StatusEffect::new(StatusEffectId::new("e1"), Some(100))
            .with_modifier(StatusModifier::increased_decay(NeedId::hunger(), 1.5));

        let effect2 = StatusEffect::new(StatusEffectId::new("e2"), Some(100))
            .with_modifier(StatusModifier::increased_decay(NeedId::hunger(), 2.0));

        set.apply(effect1);
        set.apply(effect2);

        let combined = set.combined_decay_multiplier(&NeedId::hunger());
        assert!((combined - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_status_set_combined_tick_delta() {
        let mut set = StatusSet::new();

        let effect = StatusEffect::new(StatusEffectId::new("poison"), Some(100))
            .with_modifier(StatusModifier::tick_damage(NeedId::hunger(), -5.0));

        set.apply(effect);

        let delta = set.combined_tick_delta(&NeedId::hunger());
        assert!((delta - -5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_status_set_remove() {
        let mut set = StatusSet::new();

        set.apply(StatusEffect::new(StatusEffectId::new("test"), Some(100)));
        assert!(set.has(&StatusEffectId::new("test")));

        set.remove(&StatusEffectId::new("test"));
        assert!(!set.has(&StatusEffectId::new("test")));
    }

    #[test]
    fn test_serde_round_trip() {
        let mut set = StatusSet::new();
        set.apply(
            StatusEffect::stackable(StatusEffectId::new("test"), Some(50), 3)
                .with_modifier(StatusModifier::increased_decay(NeedId::hunger(), 1.5)),
        );

        let json = serde_json::to_string(&set).unwrap();
        let restored: StatusSet = serde_json::from_str(&json).unwrap();

        assert!(restored.has(&StatusEffectId::new("test")));
        let effect = restored.get(&StatusEffectId::new("test")).unwrap();
        assert_eq!(effect.remaining_ticks, Some(50));
    }
}

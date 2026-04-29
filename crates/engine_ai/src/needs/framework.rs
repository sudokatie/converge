//! Data-driven status effect framework with stacking, decay, immunity, and environmental triggers.
//!
//! Extends the base `StatusEffect` system with:
//! - RON-loadable effect definitions
//! - Multiple stacking behaviors (stack intensity, refresh, combine, replace)
//! - Configurable decay modes (linear, exponential, threshold-gated)
//! - Per-entity immunity tracking
//! - Environmental trigger conditions for automatic effect application

use super::{NeedId, StatusEffect, StatusEffectId, StatusModifier, StatusSet};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// How an effect stacks when reapplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StackingBehavior {
    /// Adds to stack count up to max, applies intensity multiplier.
    #[default]
    Intensity,
    /// Refreshes duration only, no stacking.
    Refresh,
    /// Combines: adds stacks AND refreshes duration.
    Combine,
    /// Replaces existing effect entirely.
    Replace,
    /// No stacking - second application is ignored.
    Ignore,
}

/// How an effect decays over time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum DecayMode {
    /// Linear tick countdown (default behavior).
    #[default]
    Linear,
    /// Exponential decay: intensity reduces by factor each tick.
    Exponential {
        /// Decay factor per tick (0.9 = 10% reduction per tick).
        factor: f32,
        /// Minimum intensity before removal.
        min_intensity: f32,
    },
    /// Threshold-gated: only decays when a condition is met.
    Threshold {
        /// Which need must be above/below threshold for decay.
        need_id: NeedId,
        /// Threshold value.
        threshold: f32,
        /// If true, decay when need > threshold; if false, decay when need < threshold.
        decay_above: bool,
    },
    /// No decay - must be explicitly removed.
    Permanent,
}

/// Category of effect for grouping and UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EffectCategory {
    /// Beneficial effects (buffs).
    Beneficial,
    /// Harmful effects (debuffs).
    #[default]
    Harmful,
    /// Neutral effects (environmental, informational).
    Neutral,
}

/// Environmental trigger condition for automatic effect application.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EnvironmentalTrigger {
    /// Trigger based on scalar field value at position.
    ScalarField {
        /// Field channel name (temperature, radiation, toxicity, etc.).
        channel: String,
        /// Threshold value.
        threshold: f32,
        /// If true, trigger when value > threshold; if false, trigger when value < threshold.
        trigger_above: bool,
    },
    /// Trigger based on hazard exposure.
    Hazard {
        /// Hazard kind name (fire, infection, frost, etc.).
        kind: String,
        /// Minimum intensity to trigger.
        min_intensity: f32,
    },
    /// Trigger based on atmosphere layer.
    Atmosphere {
        /// Layer name (vacuum, exposed, outdoor, indoor).
        layer: String,
    },
    /// Trigger based on fluid contact.
    Fluid {
        /// Fluid kind name (water, lava, slurry, etc.).
        kind: String,
        /// Minimum volume for trigger.
        min_volume: f32,
    },
    /// Trigger when a need drops below threshold.
    NeedLow {
        /// Which need.
        need_id: NeedId,
        /// Threshold value.
        threshold: f32,
    },
    /// Trigger when a need exceeds threshold.
    NeedHigh {
        /// Which need.
        need_id: NeedId,
        /// Threshold value.
        threshold: f32,
    },
    /// Trigger when another effect is present.
    HasEffect {
        /// Required effect ID.
        effect_id: StatusEffectId,
    },
    /// Trigger when another effect is absent.
    LacksEffect {
        /// Required absent effect ID.
        effect_id: StatusEffectId,
    },
}

/// A modifier definition within an effect definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModifierDef {
    /// Which need this modifier affects.
    pub need_id: NeedId,
    /// Multiplier for decay rate (1.0 = no change).
    #[serde(default = "default_multiplier")]
    pub decay_multiplier: f32,
    /// Multiplier for recovery rate.
    #[serde(default = "default_multiplier")]
    pub recovery_multiplier: f32,
    /// Flat delta per tick (can be negative for damage).
    #[serde(default)]
    pub tick_delta: f32,
    /// Whether this modifier scales with stack count.
    #[serde(default = "default_true")]
    pub scales_with_stacks: bool,
}

fn default_multiplier() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

impl From<ModifierDef> for StatusModifier {
    fn from(def: ModifierDef) -> Self {
        StatusModifier {
            need_id: def.need_id,
            decay_multiplier: def.decay_multiplier,
            recovery_multiplier: def.recovery_multiplier,
            tick_delta: def.tick_delta,
        }
    }
}

/// Data-driven definition of a status effect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusEffectDef {
    /// Unique effect identifier.
    pub id: StatusEffectId,
    /// Display name.
    pub name: String,
    /// Description text.
    #[serde(default)]
    pub description: String,
    /// Effect category.
    #[serde(default)]
    pub category: EffectCategory,
    /// Default duration in ticks (None = permanent until removed).
    pub default_duration: Option<u64>,
    /// Maximum stack count.
    #[serde(default = "default_max_stacks")]
    pub max_stacks: u32,
    /// Stacking behavior.
    #[serde(default)]
    pub stacking: StackingBehavior,
    /// Decay mode.
    #[serde(default)]
    pub decay: DecayMode,
    /// Modifiers this effect applies.
    #[serde(default)]
    pub modifiers: Vec<ModifierDef>,
    /// Effects that this one is immune to while active.
    #[serde(default)]
    pub grants_immunity_to: Vec<StatusEffectId>,
    /// Effects that prevent this one from being applied.
    #[serde(default)]
    pub blocked_by: Vec<StatusEffectId>,
    /// Effects to remove when this one is applied.
    #[serde(default)]
    pub removes: Vec<StatusEffectId>,
    /// Environmental triggers for automatic application.
    #[serde(default)]
    pub triggers: Vec<EnvironmentalTrigger>,
    /// Intensity per stack for exponential decay.
    #[serde(default = "default_multiplier")]
    pub intensity_per_stack: f32,
    /// UI icon identifier.
    #[serde(default)]
    pub icon: String,
    /// Sort priority for UI display (lower = first).
    #[serde(default)]
    pub display_priority: i32,
}

fn default_max_stacks() -> u32 {
    1
}

impl StatusEffectDef {
    /// Create a new effect definition with minimal required fields.
    #[must_use]
    pub fn new(id: impl Into<StatusEffectId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category: EffectCategory::default(),
            default_duration: Some(600), // 10 seconds at 60 ticks/sec
            max_stacks: 1,
            stacking: StackingBehavior::default(),
            decay: DecayMode::default(),
            modifiers: Vec::new(),
            grants_immunity_to: Vec::new(),
            blocked_by: Vec::new(),
            removes: Vec::new(),
            triggers: Vec::new(),
            intensity_per_stack: 1.0,
            icon: String::new(),
            display_priority: 0,
        }
    }

    /// Add a modifier to this definition.
    #[must_use]
    pub fn with_modifier(mut self, modifier: ModifierDef) -> Self {
        self.modifiers.push(modifier);
        self
    }

    /// Set the category.
    #[must_use]
    pub fn with_category(mut self, category: EffectCategory) -> Self {
        self.category = category;
        self
    }

    /// Set the default duration.
    #[must_use]
    pub fn with_duration(mut self, ticks: Option<u64>) -> Self {
        self.default_duration = ticks;
        self
    }

    /// Set max stacks.
    #[must_use]
    pub fn with_max_stacks(mut self, max: u32) -> Self {
        self.max_stacks = max;
        self
    }

    /// Set stacking behavior.
    #[must_use]
    pub fn with_stacking(mut self, behavior: StackingBehavior) -> Self {
        self.stacking = behavior;
        self
    }

    /// Set decay mode.
    #[must_use]
    pub fn with_decay(mut self, mode: DecayMode) -> Self {
        self.decay = mode;
        self
    }

    /// Add an environmental trigger.
    #[must_use]
    pub fn with_trigger(mut self, trigger: EnvironmentalTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Add an immunity grant.
    #[must_use]
    pub fn grants_immunity_to(mut self, effect_id: impl Into<StatusEffectId>) -> Self {
        self.grants_immunity_to.push(effect_id.into());
        self
    }

    /// Add a blocker.
    #[must_use]
    pub fn blocked_by(mut self, effect_id: impl Into<StatusEffectId>) -> Self {
        self.blocked_by.push(effect_id.into());
        self
    }

    /// Instantiate a status effect from this definition.
    #[must_use]
    pub fn instantiate(&self) -> StatusEffect {
        let mut effect = if self.max_stacks > 1 {
            StatusEffect::stackable(self.id.clone(), self.default_duration, self.max_stacks)
        } else {
            StatusEffect::new(self.id.clone(), self.default_duration)
        };

        for mod_def in &self.modifiers {
            effect = effect.with_modifier(mod_def.clone().into());
        }

        effect
    }
}

/// Registry of status effect definitions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatusEffectRegistry {
    /// Definitions indexed by ID.
    effects: BTreeMap<StatusEffectId, StatusEffectDef>,
    /// Effects indexed by trigger type for fast lookup.
    #[serde(skip)]
    by_trigger_type: BTreeMap<String, Vec<StatusEffectId>>,
}

impl StatusEffectRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an effect definition.
    pub fn register(&mut self, def: StatusEffectDef) {
        let id = def.id.clone();

        // Index by trigger type for fast lookup
        for trigger in &def.triggers {
            let key = trigger_type_key(trigger);
            self.by_trigger_type
                .entry(key)
                .or_default()
                .push(id.clone());
        }

        self.effects.insert(id, def);
    }

    /// Get an effect definition by ID.
    #[must_use]
    pub fn get(&self, id: &StatusEffectId) -> Option<&StatusEffectDef> {
        self.effects.get(id)
    }

    /// Get all effect definitions.
    pub fn iter(&self) -> impl Iterator<Item = &StatusEffectDef> {
        self.effects.values()
    }

    /// Get effect IDs that may trigger from a scalar field.
    pub fn effects_for_scalar_field(&self, channel: &str) -> impl Iterator<Item = &StatusEffectId> {
        self.by_trigger_type
            .get(&format!("scalar:{channel}"))
            .into_iter()
            .flatten()
    }

    /// Get effect IDs that may trigger from a hazard.
    pub fn effects_for_hazard(&self, kind: &str) -> impl Iterator<Item = &StatusEffectId> {
        self.by_trigger_type
            .get(&format!("hazard:{kind}"))
            .into_iter()
            .flatten()
    }

    /// Get effect IDs that may trigger from fluid contact.
    pub fn effects_for_fluid(&self, kind: &str) -> impl Iterator<Item = &StatusEffectId> {
        self.by_trigger_type
            .get(&format!("fluid:{kind}"))
            .into_iter()
            .flatten()
    }

    /// Get effect IDs that may trigger from atmosphere.
    pub fn effects_for_atmosphere(&self, layer: &str) -> impl Iterator<Item = &StatusEffectId> {
        self.by_trigger_type
            .get(&format!("atmo:{layer}"))
            .into_iter()
            .flatten()
    }

    /// Get effect IDs that may trigger from low needs.
    pub fn effects_for_need_low(&self, need_id: &NeedId) -> impl Iterator<Item = &StatusEffectId> {
        self.by_trigger_type
            .get(&format!("need_low:{}", need_id.as_str()))
            .into_iter()
            .flatten()
    }

    /// Get effect IDs that may trigger from high needs.
    pub fn effects_for_need_high(&self, need_id: &NeedId) -> impl Iterator<Item = &StatusEffectId> {
        self.by_trigger_type
            .get(&format!("need_high:{}", need_id.as_str()))
            .into_iter()
            .flatten()
    }

    /// Number of registered effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Rebuild trigger index (call after deserialization).
    pub fn rebuild_index(&mut self) {
        self.by_trigger_type.clear();
        for (id, def) in &self.effects {
            for trigger in &def.triggers {
                let key = trigger_type_key(trigger);
                self.by_trigger_type
                    .entry(key)
                    .or_default()
                    .push(id.clone());
            }
        }
    }
}

fn trigger_type_key(trigger: &EnvironmentalTrigger) -> String {
    match trigger {
        EnvironmentalTrigger::ScalarField { channel, .. } => format!("scalar:{channel}"),
        EnvironmentalTrigger::Hazard { kind, .. } => format!("hazard:{kind}"),
        EnvironmentalTrigger::Atmosphere { layer } => format!("atmo:{layer}"),
        EnvironmentalTrigger::Fluid { kind, .. } => format!("fluid:{kind}"),
        EnvironmentalTrigger::NeedLow { need_id, .. } => format!("need_low:{}", need_id.as_str()),
        EnvironmentalTrigger::NeedHigh { need_id, .. } => format!("need_high:{}", need_id.as_str()),
        EnvironmentalTrigger::HasEffect { effect_id } => format!("has:{}", effect_id.as_str()),
        EnvironmentalTrigger::LacksEffect { effect_id } => format!("lacks:{}", effect_id.as_str()),
    }
}

/// Set of immunities for an entity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImmunitySet {
    /// Permanent immunities (e.g., from creature type).
    permanent: BTreeSet<StatusEffectId>,
    /// Temporary immunities with tick expiry.
    temporary: BTreeMap<StatusEffectId, u64>,
}

impl ImmunitySet {
    /// Create an empty immunity set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a permanent immunity.
    pub fn add_permanent(&mut self, effect_id: impl Into<StatusEffectId>) {
        self.permanent.insert(effect_id.into());
    }

    /// Remove a permanent immunity.
    pub fn remove_permanent(&mut self, effect_id: &StatusEffectId) -> bool {
        self.permanent.remove(effect_id)
    }

    /// Add a temporary immunity with duration.
    pub fn add_temporary(&mut self, effect_id: impl Into<StatusEffectId>, duration_ticks: u64) {
        let id = effect_id.into();
        self.temporary
            .entry(id)
            .and_modify(|d| *d = (*d).max(duration_ticks))
            .or_insert(duration_ticks);
    }

    /// Check if immune to an effect.
    #[must_use]
    pub fn is_immune(&self, effect_id: &StatusEffectId) -> bool {
        self.permanent.contains(effect_id) || self.temporary.get(effect_id).is_some_and(|&d| d > 0)
    }

    /// Tick temporary immunities, removing expired ones.
    pub fn tick(&mut self) -> Vec<StatusEffectId> {
        let mut expired = Vec::new();
        self.temporary.retain(|id, remaining| {
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 {
                expired.push(id.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    /// Get all current immunities (permanent and active temporary).
    pub fn all_immunities(&self) -> impl Iterator<Item = &StatusEffectId> {
        self.permanent.iter().chain(
            self.temporary
                .keys()
                .filter(|id| self.temporary.get(*id).is_some_and(|&d| d > 0)),
        )
    }

    /// Number of active immunities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.permanent.len() + self.temporary.values().filter(|&&d| d > 0).count()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.permanent.is_empty() && self.temporary.values().all(|&d| d == 0)
    }
}

/// Result of attempting to apply an effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyResult {
    /// Effect was applied successfully.
    Applied {
        /// Resulting stack count.
        stacks: u32,
        /// Whether this refreshed an existing effect.
        refreshed: bool,
    },
    /// Effect was blocked by immunity.
    Immune,
    /// Effect was blocked by another active effect.
    Blocked {
        /// The blocking effect.
        blocker: StatusEffectId,
    },
    /// Effect was ignored due to stacking behavior.
    Ignored,
    /// Effect definition not found.
    NotFound,
}

/// Managed status effect state with framework features.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ManagedStatusSet {
    /// Active effects.
    effects: StatusSet,
    /// Immunities.
    immunities: ImmunitySet,
    /// Current intensity values for exponential decay.
    intensities: BTreeMap<StatusEffectId, f32>,
}

impl ManagedStatusSet {
    /// Create a new empty managed set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the underlying status set.
    #[must_use]
    pub fn effects(&self) -> &StatusSet {
        &self.effects
    }

    /// Get the immunity set.
    #[must_use]
    pub fn immunities(&self) -> &ImmunitySet {
        &self.immunities
    }

    /// Get mutable immunity set.
    pub fn immunities_mut(&mut self) -> &mut ImmunitySet {
        &mut self.immunities
    }

    /// Check if an effect is currently active.
    #[must_use]
    pub fn has(&self, id: &StatusEffectId) -> bool {
        self.effects.has(id)
    }

    /// Get current intensity for an effect (1.0 if not using exponential decay).
    #[must_use]
    pub fn intensity(&self, id: &StatusEffectId) -> f32 {
        self.intensities.get(id).copied().unwrap_or(1.0)
    }

    /// Attempt to apply an effect using the registry.
    pub fn apply(
        &mut self,
        registry: &StatusEffectRegistry,
        effect_id: &StatusEffectId,
    ) -> ApplyResult {
        let Some(def) = registry.get(effect_id) else {
            return ApplyResult::NotFound;
        };

        // Check immunity
        if self.immunities.is_immune(effect_id) {
            return ApplyResult::Immune;
        }

        // Check blockers
        for blocker_id in &def.blocked_by {
            if self.effects.has(blocker_id) {
                return ApplyResult::Blocked {
                    blocker: blocker_id.clone(),
                };
            }
        }

        // Handle stacking behavior
        let mut had_effect = self.effects.has(effect_id);

        match def.stacking {
            StackingBehavior::Ignore if had_effect => {
                return ApplyResult::Ignored;
            }
            StackingBehavior::Replace if had_effect => {
                self.effects.remove(effect_id);
                self.intensities.remove(effect_id);
                had_effect = false; // Treat as fresh application after removal
            }
            _ => {}
        }

        // Remove effects that this one cancels
        for remove_id in &def.removes {
            self.effects.remove(remove_id);
            self.intensities.remove(remove_id);
        }

        // Apply the effect
        let instance = def.instantiate();
        let stacks = self.effects.apply(instance);

        // Initialize intensity for exponential decay
        if matches!(def.decay, DecayMode::Exponential { .. }) {
            self.intensities
                .entry(effect_id.clone())
                .or_insert(def.intensity_per_stack);
        }

        // Grant immunities from this effect
        for immune_id in &def.grants_immunity_to {
            // Grant immunity for effect duration (or permanent if effect is permanent)
            if let Some(duration) = def.default_duration {
                self.immunities.add_temporary(immune_id.clone(), duration);
            }
        }

        ApplyResult::Applied {
            stacks,
            refreshed: had_effect,
        }
    }

    /// Remove an effect.
    pub fn remove(&mut self, id: &StatusEffectId) -> bool {
        self.intensities.remove(id);
        self.effects.remove(id).is_some()
    }

    /// Tick all effects with decay logic from registry.
    ///
    /// Returns IDs of removed effects.
    pub fn tick(&mut self, registry: &StatusEffectRegistry) -> Vec<StatusEffectId> {
        let mut removed = Vec::new();

        // Tick immunities first
        self.immunities.tick();

        // Process each active effect
        let effect_ids: Vec<_> = self.effects.effect_ids().cloned().collect();

        for effect_id in effect_ids {
            let Some(def) = registry.get(&effect_id) else {
                // Unknown effect, use default tick
                if let Some(effect) = self.effects.get_mut(&effect_id)
                    && !effect.tick()
                {
                    removed.push(effect_id.clone());
                }
                continue;
            };

            match &def.decay {
                DecayMode::Linear => {
                    if let Some(effect) = self.effects.get_mut(&effect_id)
                        && !effect.tick()
                    {
                        removed.push(effect_id.clone());
                    }
                }
                DecayMode::Exponential {
                    factor,
                    min_intensity,
                } => {
                    if let Some(intensity) = self.intensities.get_mut(&effect_id) {
                        *intensity *= factor;
                        if *intensity < *min_intensity {
                            removed.push(effect_id.clone());
                        }
                    }
                }
                DecayMode::Threshold { .. } | DecayMode::Permanent => {
                    // Threshold decay is handled externally; Permanent never decays
                }
            }
        }

        // Remove expired effects
        for id in &removed {
            self.effects.remove(id);
            self.intensities.remove(id);
        }

        removed
    }

    /// Check if a threshold-gated effect should decay given current need value.
    #[must_use]
    pub fn should_decay_threshold(
        &self,
        registry: &StatusEffectRegistry,
        effect_id: &StatusEffectId,
        need_values: &BTreeMap<NeedId, f32>,
    ) -> bool {
        let Some(def) = registry.get(effect_id) else {
            return false;
        };

        match &def.decay {
            DecayMode::Threshold {
                need_id,
                threshold,
                decay_above,
            } => {
                let value = need_values.get(need_id).copied().unwrap_or(0.0);
                if *decay_above {
                    value > *threshold
                } else {
                    value < *threshold
                }
            }
            _ => false,
        }
    }

    /// Apply threshold decay tick if conditions are met.
    pub fn tick_threshold_decay(
        &mut self,
        registry: &StatusEffectRegistry,
        need_values: &BTreeMap<NeedId, f32>,
    ) -> Vec<StatusEffectId> {
        let mut removed = Vec::new();

        let effect_ids: Vec<_> = self.effects.effect_ids().cloned().collect();

        for effect_id in effect_ids {
            if self.should_decay_threshold(registry, &effect_id, need_values)
                && let Some(effect) = self.effects.get_mut(&effect_id)
                && !effect.tick()
            {
                removed.push(effect_id.clone());
            }
        }

        for id in &removed {
            self.effects.remove(id);
            self.intensities.remove(id);
        }

        removed
    }

    /// Get combined decay multiplier for a need, scaled by intensity.
    #[must_use]
    pub fn combined_decay_multiplier(
        &self,
        registry: &StatusEffectRegistry,
        need_id: &NeedId,
    ) -> f32 {
        let mut multiplier = 1.0;

        for effect_id in self.effects.effect_ids() {
            let base_mult = self
                .effects
                .get(effect_id)
                .map_or(1.0, |e| e.decay_multiplier_for(need_id));

            // Scale by intensity for exponential decay effects
            let intensity = self.intensities.get(effect_id).copied().unwrap_or(1.0);

            // Check if modifier scales with intensity
            let scales = registry
                .get(effect_id)
                .and_then(|def| {
                    def.modifiers
                        .iter()
                        .find(|m| &m.need_id == need_id)
                        .map(|m| m.scales_with_stacks)
                })
                .unwrap_or(true);

            if scales {
                multiplier *= base_mult.powf(intensity);
            } else {
                multiplier *= base_mult;
            }
        }

        multiplier
    }

    /// Get combined recovery multiplier for a need, scaled by intensity.
    #[must_use]
    pub fn combined_recovery_multiplier(
        &self,
        registry: &StatusEffectRegistry,
        need_id: &NeedId,
    ) -> f32 {
        let mut multiplier = 1.0;

        for effect_id in self.effects.effect_ids() {
            let base_mult = self
                .effects
                .get(effect_id)
                .map_or(1.0, |e| e.recovery_multiplier_for(need_id));

            let intensity = self.intensities.get(effect_id).copied().unwrap_or(1.0);

            let scales = registry
                .get(effect_id)
                .and_then(|def| {
                    def.modifiers
                        .iter()
                        .find(|m| &m.need_id == need_id)
                        .map(|m| m.scales_with_stacks)
                })
                .unwrap_or(true);

            if scales {
                multiplier *= base_mult.powf(intensity);
            } else {
                multiplier *= base_mult;
            }
        }

        multiplier
    }

    /// Get combined tick delta for a need, scaled by intensity.
    #[must_use]
    pub fn combined_tick_delta(&self, registry: &StatusEffectRegistry, need_id: &NeedId) -> f32 {
        let mut delta = 0.0;

        for effect_id in self.effects.effect_ids() {
            let base_delta = self
                .effects
                .get(effect_id)
                .map_or(0.0, |e| e.tick_delta_for(need_id));

            let intensity = self.intensities.get(effect_id).copied().unwrap_or(1.0);

            let scales = registry
                .get(effect_id)
                .and_then(|def| {
                    def.modifiers
                        .iter()
                        .find(|m| &m.need_id == need_id)
                        .map(|m| m.scales_with_stacks)
                })
                .unwrap_or(true);

            if scales {
                delta += base_delta * intensity;
            } else {
                delta += base_delta;
            }
        }

        delta
    }
}

/// Environment snapshot for trigger evaluation.
#[derive(Clone, Debug, Default)]
pub struct EnvironmentSnapshot {
    /// Scalar field values by channel name.
    pub scalar_fields: BTreeMap<String, f32>,
    /// Hazard intensities by kind.
    pub hazards: BTreeMap<String, f32>,
    /// Current atmosphere layer.
    pub atmosphere_layer: Option<String>,
    /// Fluid volumes by kind.
    pub fluids: BTreeMap<String, f32>,
}

impl EnvironmentSnapshot {
    /// Create an empty snapshot.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a scalar field value.
    pub fn set_scalar(&mut self, channel: impl Into<String>, value: f32) {
        self.scalar_fields.insert(channel.into(), value);
    }

    /// Set a hazard intensity.
    pub fn set_hazard(&mut self, kind: impl Into<String>, intensity: f32) {
        self.hazards.insert(kind.into(), intensity);
    }

    /// Set the atmosphere layer.
    pub fn set_atmosphere(&mut self, layer: impl Into<String>) {
        self.atmosphere_layer = Some(layer.into());
    }

    /// Set a fluid volume.
    pub fn set_fluid(&mut self, kind: impl Into<String>, volume: f32) {
        self.fluids.insert(kind.into(), volume);
    }
}

/// Check if a trigger condition is satisfied.
#[must_use]
pub fn evaluate_trigger(
    trigger: &EnvironmentalTrigger,
    env: &EnvironmentSnapshot,
    needs: &BTreeMap<NeedId, f32>,
    active_effects: &StatusSet,
) -> bool {
    match trigger {
        EnvironmentalTrigger::ScalarField {
            channel,
            threshold,
            trigger_above,
        } => {
            let value = env.scalar_fields.get(channel).copied().unwrap_or(0.0);
            if *trigger_above {
                value > *threshold
            } else {
                value < *threshold
            }
        }
        EnvironmentalTrigger::Hazard {
            kind,
            min_intensity,
        } => env.hazards.get(kind).is_some_and(|&i| i >= *min_intensity),
        EnvironmentalTrigger::Atmosphere { layer } => {
            env.atmosphere_layer.as_ref().is_some_and(|l| l == layer)
        }
        EnvironmentalTrigger::Fluid { kind, min_volume } => {
            env.fluids.get(kind).is_some_and(|&v| v >= *min_volume)
        }
        EnvironmentalTrigger::NeedLow { need_id, threshold } => {
            needs.get(need_id).is_some_and(|&v| v < *threshold)
        }
        EnvironmentalTrigger::NeedHigh { need_id, threshold } => {
            needs.get(need_id).is_some_and(|&v| v > *threshold)
        }
        EnvironmentalTrigger::HasEffect { effect_id } => active_effects.has(effect_id),
        EnvironmentalTrigger::LacksEffect { effect_id } => !active_effects.has(effect_id),
    }
}

/// Find all effects whose triggers are satisfied.
pub fn find_triggered_effects<'a>(
    registry: &'a StatusEffectRegistry,
    env: &EnvironmentSnapshot,
    needs: &BTreeMap<NeedId, f32>,
    active_effects: &StatusSet,
) -> Vec<&'a StatusEffectId> {
    let mut triggered = Vec::new();

    for def in registry.iter() {
        if def.triggers.is_empty() {
            continue;
        }

        // Check if ANY trigger is satisfied
        let any_triggered = def
            .triggers
            .iter()
            .any(|t| evaluate_trigger(t, env, needs, active_effects));

        if any_triggered {
            triggered.push(&def.id);
        }
    }

    triggered
}

/// Built-in effect presets for common gameplay effects.
pub mod presets {
    use super::{
        DecayMode, EffectCategory, EnvironmentalTrigger, ModifierDef, NeedId, StackingBehavior,
        StatusEffectDef, StatusEffectRegistry,
    };

    /// Poison effect - damage over time.
    #[must_use]
    pub fn poison() -> StatusEffectDef {
        StatusEffectDef::new("poison", "Poison")
            .with_category(EffectCategory::Harmful)
            .with_duration(Some(600))
            .with_max_stacks(4)
            .with_stacking(StackingBehavior::Intensity)
            .with_modifier(ModifierDef {
                need_id: NeedId::hunger(),
                decay_multiplier: 1.5,
                recovery_multiplier: 0.5,
                tick_delta: -0.5,
                scales_with_stacks: true,
            })
    }

    /// Regeneration effect - healing over time.
    #[must_use]
    pub fn regeneration() -> StatusEffectDef {
        StatusEffectDef::new("regeneration", "Regeneration")
            .with_category(EffectCategory::Beneficial)
            .with_duration(Some(900))
            .with_max_stacks(2)
            .with_stacking(StackingBehavior::Combine)
            .with_modifier(ModifierDef {
                need_id: NeedId::hunger(),
                decay_multiplier: 0.5,
                recovery_multiplier: 2.0,
                tick_delta: 0.0,
                scales_with_stacks: true,
            })
    }

    /// Hypothermia - cold exposure.
    #[must_use]
    pub fn hypothermia() -> StatusEffectDef {
        StatusEffectDef::new("hypothermia", "Hypothermia")
            .with_category(EffectCategory::Harmful)
            .with_duration(None) // Permanent until warmed
            .with_decay(DecayMode::Threshold {
                need_id: NeedId::warmth(),
                threshold: 50.0,
                decay_above: true,
            })
            .with_trigger(EnvironmentalTrigger::ScalarField {
                channel: "temperature".to_string(),
                threshold: 10.0,
                trigger_above: false,
            })
            .with_modifier(ModifierDef {
                need_id: NeedId::warmth(),
                decay_multiplier: 2.0,
                recovery_multiplier: 0.3,
                tick_delta: -1.0,
                scales_with_stacks: false,
            })
    }

    /// Radiation sickness.
    #[must_use]
    pub fn radiation_sickness() -> StatusEffectDef {
        StatusEffectDef::new("radiation_sickness", "Radiation Sickness")
            .with_category(EffectCategory::Harmful)
            .with_duration(Some(1800))
            .with_max_stacks(5)
            .with_stacking(StackingBehavior::Intensity)
            .with_decay(DecayMode::Exponential {
                factor: 0.995,
                min_intensity: 0.1,
            })
            .with_trigger(EnvironmentalTrigger::ScalarField {
                channel: "radiation".to_string(),
                threshold: 50.0,
                trigger_above: true,
            })
            .with_modifier(ModifierDef {
                need_id: NeedId::hunger(),
                decay_multiplier: 1.8,
                recovery_multiplier: 0.4,
                tick_delta: -0.3,
                scales_with_stacks: true,
            })
    }

    /// Fire resistance - blocks burning effect.
    #[must_use]
    pub fn fire_resistance() -> StatusEffectDef {
        StatusEffectDef::new("fire_resistance", "Fire Resistance")
            .with_category(EffectCategory::Beneficial)
            .with_duration(Some(1800))
            .with_stacking(StackingBehavior::Refresh)
            .grants_immunity_to("burning")
    }

    /// Burning - fire damage over time.
    #[must_use]
    pub fn burning() -> StatusEffectDef {
        StatusEffectDef::new("burning", "Burning")
            .with_category(EffectCategory::Harmful)
            .with_duration(Some(180))
            .with_stacking(StackingBehavior::Refresh)
            .blocked_by("fire_resistance")
            .with_trigger(EnvironmentalTrigger::Hazard {
                kind: "fire".to_string(),
                min_intensity: 0.5,
            })
            .with_modifier(ModifierDef {
                need_id: NeedId::hunger(),
                decay_multiplier: 1.0,
                recovery_multiplier: 1.0,
                tick_delta: -2.0,
                scales_with_stacks: false,
            })
    }

    /// Suffocation - oxygen deprivation.
    #[must_use]
    pub fn suffocation() -> StatusEffectDef {
        StatusEffectDef::new("suffocation", "Suffocation")
            .with_category(EffectCategory::Harmful)
            .with_duration(None)
            .with_decay(DecayMode::Threshold {
                need_id: NeedId::oxygen(),
                threshold: 30.0,
                decay_above: true,
            })
            .with_trigger(EnvironmentalTrigger::Atmosphere {
                layer: "vacuum".to_string(),
            })
            .with_modifier(ModifierDef {
                need_id: NeedId::oxygen(),
                decay_multiplier: 3.0,
                recovery_multiplier: 0.0,
                tick_delta: -5.0,
                scales_with_stacks: false,
            })
    }

    /// Speed boost.
    #[must_use]
    pub fn speed() -> StatusEffectDef {
        StatusEffectDef::new("speed", "Speed")
            .with_category(EffectCategory::Beneficial)
            .with_duration(Some(1200))
            .with_max_stacks(3)
            .with_stacking(StackingBehavior::Combine)
    }

    /// Create a registry with all presets.
    #[must_use]
    pub fn create_preset_registry() -> StatusEffectRegistry {
        let mut registry = StatusEffectRegistry::new();
        registry.register(poison());
        registry.register(regeneration());
        registry.register(hypothermia());
        registry.register(radiation_sickness());
        registry.register(fire_resistance());
        registry.register(burning());
        registry.register(suffocation());
        registry.register(speed());
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplyResult, DecayMode, EffectCategory, EnvironmentSnapshot, EnvironmentalTrigger,
        ImmunitySet, ManagedStatusSet, ModifierDef, NeedId, StackingBehavior, StatusEffect,
        StatusEffectDef, StatusEffectId, StatusEffectRegistry, StatusSet, evaluate_trigger,
        find_triggered_effects, presets,
    };
    use std::collections::BTreeMap;

    #[test]
    fn test_stacking_behavior_default() {
        assert_eq!(StackingBehavior::default(), StackingBehavior::Intensity);
    }

    #[test]
    fn test_decay_mode_default() {
        assert!(matches!(DecayMode::default(), DecayMode::Linear));
    }

    #[test]
    fn test_effect_def_builder() {
        let def = StatusEffectDef::new("test", "Test Effect")
            .with_category(EffectCategory::Beneficial)
            .with_duration(Some(100))
            .with_max_stacks(5)
            .with_stacking(StackingBehavior::Combine);

        assert_eq!(def.id, StatusEffectId::new("test"));
        assert_eq!(def.name, "Test Effect");
        assert_eq!(def.category, EffectCategory::Beneficial);
        assert_eq!(def.default_duration, Some(100));
        assert_eq!(def.max_stacks, 5);
        assert_eq!(def.stacking, StackingBehavior::Combine);
    }

    #[test]
    fn test_effect_def_instantiate() {
        let def = StatusEffectDef::new("poison", "Poison")
            .with_duration(Some(60))
            .with_max_stacks(3);

        let instance = def.instantiate();

        assert_eq!(instance.id, StatusEffectId::new("poison"));
        assert_eq!(instance.remaining_ticks, Some(60));
        assert_eq!(instance.max_stacks, 3);
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = StatusEffectRegistry::new();
        registry.register(StatusEffectDef::new("test", "Test"));

        assert!(registry.get(&StatusEffectId::new("test")).is_some());
        assert!(registry.get(&StatusEffectId::new("nonexistent")).is_none());
    }

    #[test]
    fn test_registry_trigger_indexing() {
        let mut registry = StatusEffectRegistry::new();

        let def =
            StatusEffectDef::new("cold", "Cold").with_trigger(EnvironmentalTrigger::ScalarField {
                channel: "temperature".to_string(),
                threshold: 10.0,
                trigger_above: false,
            });

        registry.register(def);

        let effects: Vec<_> = registry.effects_for_scalar_field("temperature").collect();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0], &StatusEffectId::new("cold"));
    }

    #[test]
    fn test_immunity_set_permanent() {
        let mut immunities = ImmunitySet::new();
        let poison_id = StatusEffectId::new("poison");

        assert!(!immunities.is_immune(&poison_id));

        immunities.add_permanent(poison_id.clone());
        assert!(immunities.is_immune(&poison_id));

        immunities.remove_permanent(&poison_id);
        assert!(!immunities.is_immune(&poison_id));
    }

    #[test]
    fn test_immunity_set_temporary() {
        let mut immunities = ImmunitySet::new();
        let poison_id = StatusEffectId::new("poison");

        immunities.add_temporary(poison_id.clone(), 3);
        assert!(immunities.is_immune(&poison_id));

        immunities.tick();
        assert!(immunities.is_immune(&poison_id));

        immunities.tick();
        immunities.tick();
        assert!(!immunities.is_immune(&poison_id));
    }

    #[test]
    fn test_managed_set_apply_success() {
        let registry = presets::create_preset_registry();
        let mut managed = ManagedStatusSet::new();

        let result = managed.apply(&registry, &StatusEffectId::new("poison"));

        assert!(matches!(
            result,
            ApplyResult::Applied {
                stacks: 1,
                refreshed: false
            }
        ));
        assert!(managed.has(&StatusEffectId::new("poison")));
    }

    #[test]
    fn test_managed_set_apply_immune() {
        let registry = presets::create_preset_registry();
        let mut managed = ManagedStatusSet::new();

        managed
            .immunities_mut()
            .add_permanent(StatusEffectId::new("poison"));

        let result = managed.apply(&registry, &StatusEffectId::new("poison"));

        assert!(matches!(result, ApplyResult::Immune));
        assert!(!managed.has(&StatusEffectId::new("poison")));
    }

    #[test]
    fn test_managed_set_apply_blocked() {
        let mut registry = StatusEffectRegistry::new();

        // Create a blocker effect (no immunity grant)
        registry.register(StatusEffectDef::new("shield", "Shield").with_duration(Some(100)));

        // Create an effect blocked by shield
        let mut blocked_def = StatusEffectDef::new("curse", "Curse").with_duration(Some(100));
        blocked_def.blocked_by.push(StatusEffectId::new("shield"));
        registry.register(blocked_def);

        let mut managed = ManagedStatusSet::new();

        // Apply shield first
        managed.apply(&registry, &StatusEffectId::new("shield"));

        // Try to apply curse - should be blocked
        let result = managed.apply(&registry, &StatusEffectId::new("curse"));

        assert!(matches!(
            result,
            ApplyResult::Blocked { blocker } if blocker == StatusEffectId::new("shield")
        ));
    }

    #[test]
    fn test_managed_set_stacking_refresh() {
        let registry = presets::create_preset_registry();
        let mut managed = ManagedStatusSet::new();

        // Fire resistance uses Refresh stacking
        managed.apply(&registry, &StatusEffectId::new("fire_resistance"));

        let result = managed.apply(&registry, &StatusEffectId::new("fire_resistance"));

        assert!(matches!(
            result,
            ApplyResult::Applied {
                stacks: 1,
                refreshed: true
            }
        ));
    }

    #[test]
    fn test_managed_set_tick_linear_decay() {
        let mut managed = ManagedStatusSet::new();

        // Create a short duration effect
        let mut short_registry = StatusEffectRegistry::new();
        short_registry.register(StatusEffectDef::new("short", "Short").with_duration(Some(2)));

        managed.apply(&short_registry, &StatusEffectId::new("short"));
        assert!(managed.has(&StatusEffectId::new("short")));

        managed.tick(&short_registry);
        assert!(managed.has(&StatusEffectId::new("short")));

        managed.tick(&short_registry);
        assert!(!managed.has(&StatusEffectId::new("short")));
    }

    #[test]
    fn test_managed_set_tick_exponential_decay() {
        let mut registry = StatusEffectRegistry::new();
        registry.register(
            StatusEffectDef::new("radiation", "Radiation")
                .with_duration(None)
                .with_decay(DecayMode::Exponential {
                    factor: 0.5,
                    min_intensity: 0.1,
                }),
        );

        let mut managed = ManagedStatusSet::new();
        managed.apply(&registry, &StatusEffectId::new("radiation"));

        // Initial intensity
        assert!((managed.intensity(&StatusEffectId::new("radiation")) - 1.0).abs() < 0.01);

        // After one tick, intensity should be halved
        managed.tick(&registry);
        assert!((managed.intensity(&StatusEffectId::new("radiation")) - 0.5).abs() < 0.01);

        // After another tick
        managed.tick(&registry);
        assert!((managed.intensity(&StatusEffectId::new("radiation")) - 0.25).abs() < 0.01);

        // After two more ticks, should drop below min and be removed
        managed.tick(&registry);
        managed.tick(&registry);
        assert!(!managed.has(&StatusEffectId::new("radiation")));
    }

    #[test]
    fn test_evaluate_trigger_scalar_field() {
        let trigger = EnvironmentalTrigger::ScalarField {
            channel: "temperature".to_string(),
            threshold: 10.0,
            trigger_above: false,
        };

        let mut env = EnvironmentSnapshot::new();
        let needs = BTreeMap::new();
        let effects = StatusSet::new();

        // Above threshold - should not trigger
        env.set_scalar("temperature", 15.0);
        assert!(!evaluate_trigger(&trigger, &env, &needs, &effects));

        // Below threshold - should trigger
        env.set_scalar("temperature", 5.0);
        assert!(evaluate_trigger(&trigger, &env, &needs, &effects));
    }

    #[test]
    fn test_evaluate_trigger_hazard() {
        let trigger = EnvironmentalTrigger::Hazard {
            kind: "fire".to_string(),
            min_intensity: 0.5,
        };

        let mut env = EnvironmentSnapshot::new();
        let needs = BTreeMap::new();
        let effects = StatusSet::new();

        // No hazard - should not trigger
        assert!(!evaluate_trigger(&trigger, &env, &needs, &effects));

        // Below min intensity - should not trigger
        env.set_hazard("fire", 0.3);
        assert!(!evaluate_trigger(&trigger, &env, &needs, &effects));

        // Above min intensity - should trigger
        env.set_hazard("fire", 0.8);
        assert!(evaluate_trigger(&trigger, &env, &needs, &effects));
    }

    #[test]
    fn test_evaluate_trigger_need_low() {
        let trigger = EnvironmentalTrigger::NeedLow {
            need_id: NeedId::hunger(),
            threshold: 20.0,
        };

        let env = EnvironmentSnapshot::new();
        let effects = StatusSet::new();

        let mut needs = BTreeMap::new();

        // Above threshold - should not trigger
        needs.insert(NeedId::hunger(), 50.0);
        assert!(!evaluate_trigger(&trigger, &env, &needs, &effects));

        // Below threshold - should trigger
        needs.insert(NeedId::hunger(), 10.0);
        assert!(evaluate_trigger(&trigger, &env, &needs, &effects));
    }

    #[test]
    fn test_evaluate_trigger_has_effect() {
        let trigger = EnvironmentalTrigger::HasEffect {
            effect_id: StatusEffectId::new("poison"),
        };

        let env = EnvironmentSnapshot::new();
        let needs = BTreeMap::new();
        let mut effects = StatusSet::new();

        // Effect not present - should not trigger
        assert!(!evaluate_trigger(&trigger, &env, &needs, &effects));

        // Effect present - should trigger
        effects.apply(StatusEffect::new(StatusEffectId::new("poison"), Some(100)));
        assert!(evaluate_trigger(&trigger, &env, &needs, &effects));
    }

    #[test]
    fn test_find_triggered_effects() {
        let mut registry = StatusEffectRegistry::new();

        registry.register(
            StatusEffectDef::new("hypothermia", "Hypothermia").with_trigger(
                EnvironmentalTrigger::ScalarField {
                    channel: "temperature".to_string(),
                    threshold: 10.0,
                    trigger_above: false,
                },
            ),
        );

        registry.register(
            StatusEffectDef::new("hyperthermia", "Hyperthermia").with_trigger(
                EnvironmentalTrigger::ScalarField {
                    channel: "temperature".to_string(),
                    threshold: 40.0,
                    trigger_above: true,
                },
            ),
        );

        let mut env = EnvironmentSnapshot::new();
        env.set_scalar("temperature", 5.0);

        let needs = BTreeMap::new();
        let effects = StatusSet::new();

        let triggered = find_triggered_effects(&registry, &env, &needs, &effects);

        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], &StatusEffectId::new("hypothermia"));
    }

    #[test]
    fn test_threshold_decay() {
        let mut registry = StatusEffectRegistry::new();
        registry.register(
            StatusEffectDef::new("wet", "Wet")
                .with_duration(Some(100))
                .with_decay(DecayMode::Threshold {
                    need_id: NeedId::warmth(),
                    threshold: 50.0,
                    decay_above: true,
                }),
        );

        let mut managed = ManagedStatusSet::new();
        managed.apply(&registry, &StatusEffectId::new("wet"));

        let mut needs = BTreeMap::new();

        // Below threshold - should NOT decay
        needs.insert(NeedId::warmth(), 30.0);
        assert!(!managed.should_decay_threshold(&registry, &StatusEffectId::new("wet"), &needs));

        // Above threshold - should decay
        needs.insert(NeedId::warmth(), 70.0);
        assert!(managed.should_decay_threshold(&registry, &StatusEffectId::new("wet"), &needs));
    }

    #[test]
    fn test_serde_round_trip_registry() {
        let registry = presets::create_preset_registry();

        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: StatusEffectRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();

        assert_eq!(restored.len(), registry.len());
        assert!(restored.get(&StatusEffectId::new("poison")).is_some());
    }

    #[test]
    fn test_serde_round_trip_managed_set() {
        let registry = presets::create_preset_registry();
        let mut managed = ManagedStatusSet::new();

        managed.apply(&registry, &StatusEffectId::new("poison"));
        managed
            .immunities_mut()
            .add_permanent(StatusEffectId::new("burning"));

        let json = serde_json::to_string(&managed).unwrap();
        let restored: ManagedStatusSet = serde_json::from_str(&json).unwrap();

        assert!(restored.has(&StatusEffectId::new("poison")));
        assert!(
            restored
                .immunities()
                .is_immune(&StatusEffectId::new("burning"))
        );
    }

    #[test]
    fn test_preset_poison() {
        let def = presets::poison();
        assert_eq!(def.category, EffectCategory::Harmful);
        assert_eq!(def.max_stacks, 4);
        assert!(!def.modifiers.is_empty());
    }

    #[test]
    fn test_preset_fire_resistance_grants_immunity() {
        let def = presets::fire_resistance();
        assert!(
            def.grants_immunity_to
                .contains(&StatusEffectId::new("burning"))
        );
    }

    #[test]
    fn test_preset_burning_blocked_by_fire_resistance() {
        let def = presets::burning();
        assert!(
            def.blocked_by
                .contains(&StatusEffectId::new("fire_resistance"))
        );
    }

    #[test]
    fn test_combined_multipliers_with_intensity() {
        let mut registry = StatusEffectRegistry::new();
        registry.register(
            StatusEffectDef::new("weakness", "Weakness")
                .with_duration(None)
                .with_decay(DecayMode::Exponential {
                    factor: 0.9,
                    min_intensity: 0.1,
                })
                .with_modifier(ModifierDef {
                    need_id: NeedId::hunger(),
                    decay_multiplier: 2.0,
                    recovery_multiplier: 0.5,
                    tick_delta: 0.0,
                    scales_with_stacks: true,
                }),
        );

        let mut managed = ManagedStatusSet::new();
        managed.apply(&registry, &StatusEffectId::new("weakness"));

        // Initial intensity = 1.0, so multiplier = 2.0^1.0 = 2.0
        let mult = managed.combined_decay_multiplier(&registry, &NeedId::hunger());
        assert!((mult - 2.0).abs() < 0.01);

        // After tick, intensity = 0.9, so multiplier = 2.0^0.9 ~= 1.87
        managed.tick(&registry);
        let mult2 = managed.combined_decay_multiplier(&registry, &NeedId::hunger());
        assert!(mult2 < mult);
    }

    #[test]
    fn test_effect_removes_other() {
        let mut registry = StatusEffectRegistry::new();

        registry.register(StatusEffectDef::new("wet", "Wet").with_duration(Some(100)));

        let mut dry_def = StatusEffectDef::new("dry", "Dry").with_duration(Some(50));
        dry_def.removes.push(StatusEffectId::new("wet"));
        registry.register(dry_def);

        let mut managed = ManagedStatusSet::new();
        managed.apply(&registry, &StatusEffectId::new("wet"));
        assert!(managed.has(&StatusEffectId::new("wet")));

        managed.apply(&registry, &StatusEffectId::new("dry"));
        assert!(!managed.has(&StatusEffectId::new("wet")));
        assert!(managed.has(&StatusEffectId::new("dry")));
    }

    #[test]
    fn test_stacking_ignore() {
        let mut registry = StatusEffectRegistry::new();
        registry.register(
            StatusEffectDef::new("unique", "Unique")
                .with_duration(Some(100))
                .with_stacking(StackingBehavior::Ignore),
        );

        let mut managed = ManagedStatusSet::new();

        let result1 = managed.apply(&registry, &StatusEffectId::new("unique"));
        assert!(matches!(result1, ApplyResult::Applied { .. }));

        let result2 = managed.apply(&registry, &StatusEffectId::new("unique"));
        assert!(matches!(result2, ApplyResult::Ignored));
    }

    #[test]
    fn test_stacking_replace() {
        let mut registry = StatusEffectRegistry::new();
        registry.register(
            StatusEffectDef::new("replaced", "Replaced")
                .with_duration(Some(100))
                .with_stacking(StackingBehavior::Replace),
        );

        let mut managed = ManagedStatusSet::new();

        managed.apply(&registry, &StatusEffectId::new("replaced"));

        // Tick to reduce duration
        managed.tick(&registry);

        // Replace with fresh effect
        let result = managed.apply(&registry, &StatusEffectId::new("replaced"));
        assert!(matches!(
            result,
            ApplyResult::Applied {
                refreshed: false,
                ..
            }
        ));

        // Duration should be reset to full
        let effect = managed
            .effects()
            .get(&StatusEffectId::new("replaced"))
            .unwrap();
        assert_eq!(effect.remaining_ticks, Some(100));
    }
}

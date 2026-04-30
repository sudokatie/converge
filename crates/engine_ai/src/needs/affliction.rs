//! Condition/affliction system for long-term progressive effects.
//!
//! Afflictions differ from status effects in several ways:
//! - **Severity levels**: Progress through stages (mild, moderate, severe, critical)
//! - **Exposure tracking**: Accumulates before an affliction triggers
//! - **Resistance**: Per-entity resistance modifiers that affect exposure accumulation
//! - **Symptoms**: Different effects/modifiers at each severity level
//!
//! Built-in affliction kinds: frostbite, bends, infection, spores, radiation sickness, fatigue.

use super::{NeedId, StatusEffectId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;

/// Unique identifier for an affliction type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AfflictionId(pub String);

impl AfflictionId {
    pub const FROSTBITE: &'static str = "frostbite";
    pub const BENDS: &'static str = "bends";
    pub const INFECTION: &'static str = "infection";
    pub const SPORES: &'static str = "spores";
    pub const RADIATION_SICKNESS: &'static str = "radiation_sickness";
    pub const FATIGUE: &'static str = "fatigue";

    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn frostbite() -> Self {
        Self::new(Self::FROSTBITE)
    }

    #[must_use]
    pub fn bends() -> Self {
        Self::new(Self::BENDS)
    }

    #[must_use]
    pub fn infection() -> Self {
        Self::new(Self::INFECTION)
    }

    #[must_use]
    pub fn spores() -> Self {
        Self::new(Self::SPORES)
    }

    #[must_use]
    pub fn radiation_sickness() -> Self {
        Self::new(Self::RADIATION_SICKNESS)
    }

    #[must_use]
    pub fn fatigue() -> Self {
        Self::new(Self::FATIGUE)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for AfflictionId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Severity level of an affliction.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum Severity {
    /// No affliction present.
    #[default]
    None,
    /// Mild symptoms, minor penalties.
    Mild,
    /// Moderate symptoms, noticeable penalties.
    Moderate,
    /// Severe symptoms, major penalties.
    Severe,
    /// Critical symptoms, life-threatening.
    Critical,
}

impl Severity {
    /// Returns the numeric index (0-4) for threshold comparisons.
    #[must_use]
    pub fn as_index(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Mild => 1,
            Self::Moderate => 2,
            Self::Severe => 3,
            Self::Critical => 4,
        }
    }

    /// Create from numeric index, clamped to valid range.
    #[must_use]
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::None,
            1 => Self::Mild,
            2 => Self::Moderate,
            3 => Self::Severe,
            _ => Self::Critical,
        }
    }

    /// Get the next higher severity level, or Critical if already at max.
    #[must_use]
    pub fn worse(self) -> Self {
        Self::from_index(self.as_index().saturating_add(1).min(4))
    }

    /// Get the next lower severity level, or None if already at min.
    #[must_use]
    pub fn better(self) -> Self {
        Self::from_index(self.as_index().saturating_sub(1))
    }
}

/// How an affliction's exposure is triggered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExposureTrigger {
    /// Exposure from scalar field value (temperature, radiation, pressure, etc.).
    ScalarField {
        /// Field channel name.
        channel: String,
        /// Threshold for exposure to begin.
        threshold: f32,
        /// If true, trigger when value > threshold; if false, when value < threshold.
        trigger_above: bool,
        /// Exposure rate when triggered (per tick).
        rate: f32,
    },
    /// Exposure from hazard intensity.
    Hazard {
        /// Hazard kind name.
        kind: String,
        /// Minimum intensity for exposure.
        min_intensity: f32,
        /// Exposure rate multiplied by hazard intensity.
        rate_per_intensity: f32,
    },
    /// Exposure from need level.
    NeedLevel {
        /// Which need.
        need_id: NeedId,
        /// Threshold value.
        threshold: f32,
        /// If true, trigger when need > threshold; if false, when need < threshold.
        trigger_above: bool,
        /// Exposure rate when triggered.
        rate: f32,
    },
    /// Exposure from activity (exertion, depth change rate, etc.).
    Activity {
        /// Activity type identifier.
        activity: String,
        /// Base rate per unit of activity.
        rate_per_unit: f32,
    },
    /// Exposure from presence of another affliction.
    ComplicatedBy {
        /// Required affliction ID.
        affliction_id: AfflictionId,
        /// Minimum severity of the complicating affliction.
        min_severity: Severity,
        /// Exposure rate when condition is met.
        rate: f32,
    },
}

/// How an affliction recovers/decays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecoveryMode {
    /// Linear recovery when not exposed.
    Linear {
        /// Recovery rate per tick.
        rate: f32,
    },
    /// Recovery only when specific conditions are met.
    Conditional {
        /// Required need above this threshold.
        need_id: NeedId,
        /// Threshold value.
        threshold: f32,
        /// Recovery rate when condition is met.
        rate: f32,
    },
    /// Recovery only with treatment (explicit intervention required).
    TreatmentRequired,
    /// No recovery - affliction is permanent once acquired.
    Permanent,
}

impl Default for RecoveryMode {
    fn default() -> Self {
        Self::Linear { rate: 0.1 }
    }
}

/// A modifier that applies at a specific severity level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeverityModifier {
    /// Minimum severity for this modifier to apply.
    pub min_severity: Severity,
    /// Which need this affects.
    pub need_id: NeedId,
    /// Decay rate multiplier (1.0 = no change).
    #[serde(default = "default_multiplier")]
    pub decay_multiplier: f32,
    /// Recovery rate multiplier.
    #[serde(default = "default_multiplier")]
    pub recovery_multiplier: f32,
    /// Flat delta per tick.
    #[serde(default)]
    pub tick_delta: f32,
}

fn default_multiplier() -> f32 {
    1.0
}

/// Status effects to apply at different severity levels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeverityEffect {
    /// Minimum severity to apply this effect.
    pub min_severity: Severity,
    /// Status effect to apply.
    pub effect_id: StatusEffectId,
}

/// Data-driven definition of an affliction type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AfflictionDef {
    /// Unique affliction identifier.
    pub id: AfflictionId,
    /// Display name.
    pub name: String,
    /// Description text.
    #[serde(default)]
    pub description: String,
    /// Category for grouping/UI.
    #[serde(default)]
    pub category: AfflictionCategory,
    /// Exposure thresholds for each severity level (mild, moderate, severe, critical).
    pub severity_thresholds: SeverityThresholds,
    /// How exposure is triggered.
    #[serde(default)]
    pub triggers: Vec<ExposureTrigger>,
    /// How recovery works.
    #[serde(default)]
    pub recovery: RecoveryMode,
    /// Modifiers at each severity level.
    #[serde(default)]
    pub modifiers: Vec<SeverityModifier>,
    /// Status effects to apply at severity levels.
    #[serde(default)]
    pub effects: Vec<SeverityEffect>,
    /// Other afflictions that this one blocks.
    #[serde(default)]
    pub blocks: Vec<AfflictionId>,
    /// Afflictions that make this one progress faster.
    #[serde(default)]
    pub accelerated_by: Vec<AfflictionId>,
    /// UI icon identifier.
    #[serde(default)]
    pub icon: String,
    /// Sort priority for UI display (lower = first).
    #[serde(default)]
    pub display_priority: i32,
}

/// Category of affliction for grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AfflictionCategory {
    /// Environmental conditions (frostbite, heat stroke).
    #[default]
    Environmental,
    /// Pressure-related (bends, barotrauma).
    Pressure,
    /// Biological (infection, parasites).
    Biological,
    /// Chemical/radiation exposure.
    Chemical,
    /// Physical exhaustion/strain.
    Physical,
    /// Mental/psychological conditions.
    Mental,
}

/// Thresholds for severity progression.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeverityThresholds {
    /// Exposure for mild severity.
    pub mild: f32,
    /// Exposure for moderate severity.
    pub moderate: f32,
    /// Exposure for severe severity.
    pub severe: f32,
    /// Exposure for critical severity.
    pub critical: f32,
}

impl SeverityThresholds {
    #[must_use]
    pub fn new(mild: f32, moderate: f32, severe: f32, critical: f32) -> Self {
        Self {
            mild,
            moderate,
            severe,
            critical,
        }
    }

    /// Classify an exposure value into a severity level.
    #[must_use]
    pub fn classify(&self, exposure: f32) -> Severity {
        if exposure >= self.critical {
            Severity::Critical
        } else if exposure >= self.severe {
            Severity::Severe
        } else if exposure >= self.moderate {
            Severity::Moderate
        } else if exposure >= self.mild {
            Severity::Mild
        } else {
            Severity::None
        }
    }

    /// Get the threshold for a given severity.
    #[must_use]
    pub fn threshold_for(&self, severity: Severity) -> f32 {
        match severity {
            Severity::None => 0.0,
            Severity::Mild => self.mild,
            Severity::Moderate => self.moderate,
            Severity::Severe => self.severe,
            Severity::Critical => self.critical,
        }
    }
}

impl Default for SeverityThresholds {
    fn default() -> Self {
        Self {
            mild: 25.0,
            moderate: 50.0,
            severe: 75.0,
            critical: 100.0,
        }
    }
}

impl AfflictionDef {
    /// Create a new affliction definition with minimal fields.
    #[must_use]
    pub fn new(id: impl Into<AfflictionId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category: AfflictionCategory::default(),
            severity_thresholds: SeverityThresholds::default(),
            triggers: Vec::new(),
            recovery: RecoveryMode::default(),
            modifiers: Vec::new(),
            effects: Vec::new(),
            blocks: Vec::new(),
            accelerated_by: Vec::new(),
            icon: String::new(),
            display_priority: 0,
        }
    }

    /// Set the category.
    #[must_use]
    pub fn with_category(mut self, category: AfflictionCategory) -> Self {
        self.category = category;
        self
    }

    /// Set severity thresholds.
    #[must_use]
    pub fn with_thresholds(mut self, thresholds: SeverityThresholds) -> Self {
        self.severity_thresholds = thresholds;
        self
    }

    /// Add an exposure trigger.
    #[must_use]
    pub fn with_trigger(mut self, trigger: ExposureTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Set recovery mode.
    #[must_use]
    pub fn with_recovery(mut self, recovery: RecoveryMode) -> Self {
        self.recovery = recovery;
        self
    }

    /// Add a severity modifier.
    #[must_use]
    pub fn with_modifier(mut self, modifier: SeverityModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    /// Add a severity effect.
    #[must_use]
    pub fn with_effect(mut self, effect: SeverityEffect) -> Self {
        self.effects.push(effect);
        self
    }

    /// Compute a stable fingerprint for this definition.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(self.id.as_str().as_bytes());
        hasher.update(self.name.as_bytes());
        hasher.update(&self.category.as_index().to_le_bytes());
        hasher.update(&self.severity_thresholds.mild.to_le_bytes());
        hasher.update(&self.severity_thresholds.moderate.to_le_bytes());
        hasher.update(&self.severity_thresholds.severe.to_le_bytes());
        hasher.update(&self.severity_thresholds.critical.to_le_bytes());
        hasher.update(&(self.triggers.len() as u32).to_le_bytes());
        hasher.update(&(self.modifiers.len() as u32).to_le_bytes());
        hasher.update(&(self.effects.len() as u32).to_le_bytes());
        hasher.finalize()
    }
}

impl AfflictionCategory {
    fn as_index(self) -> u8 {
        match self {
            Self::Environmental => 0,
            Self::Pressure => 1,
            Self::Biological => 2,
            Self::Chemical => 3,
            Self::Physical => 4,
            Self::Mental => 5,
        }
    }
}

/// Active affliction state for a single entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveAffliction {
    /// The affliction ID.
    pub id: AfflictionId,
    /// Current exposure level (0.0 to critical threshold+).
    pub exposure: f32,
    /// Current severity based on exposure.
    pub severity: Severity,
    /// Tick when affliction was first acquired.
    pub acquired_tick: u64,
    /// Tick when current severity was reached.
    pub severity_change_tick: u64,
    /// Whether currently receiving treatment.
    pub under_treatment: bool,
}

impl ActiveAffliction {
    /// Create a new active affliction.
    #[must_use]
    pub fn new(id: AfflictionId, tick: u64) -> Self {
        Self {
            id,
            exposure: 0.0,
            severity: Severity::None,
            acquired_tick: tick,
            severity_change_tick: tick,
            under_treatment: false,
        }
    }

    /// Update exposure and recalculate severity.
    pub fn update_severity(&mut self, thresholds: &SeverityThresholds, tick: u64) {
        let new_severity = thresholds.classify(self.exposure);
        if new_severity != self.severity {
            self.severity = new_severity;
            self.severity_change_tick = tick;
        }
    }

    /// Add exposure, clamped to non-negative.
    pub fn add_exposure(&mut self, amount: f32) {
        self.exposure = (self.exposure + amount).max(0.0);
    }

    /// Remove exposure, clamped to non-negative.
    pub fn remove_exposure(&mut self, amount: f32) {
        self.exposure = (self.exposure - amount).max(0.0);
    }
}

/// Per-entity resistance configuration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResistanceSet {
    /// Resistance multipliers by affliction ID (0.0 = full resist, 1.0 = normal, >1.0 = vulnerable).
    resistances: BTreeMap<AfflictionId, f32>,
    /// Complete immunities.
    immunities: BTreeSet<AfflictionId>,
}

impl ResistanceSet {
    /// Create an empty resistance set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set resistance for an affliction (0.0 = full resist, 1.0 = normal, >1.0 = vulnerable).
    pub fn set_resistance(&mut self, id: impl Into<AfflictionId>, multiplier: f32) {
        self.resistances.insert(id.into(), multiplier.max(0.0));
    }

    /// Get resistance multiplier (defaults to 1.0 if not set).
    #[must_use]
    pub fn resistance(&self, id: &AfflictionId) -> f32 {
        if self.immunities.contains(id) {
            0.0
        } else {
            self.resistances.get(id).copied().unwrap_or(1.0)
        }
    }

    /// Add immunity to an affliction.
    pub fn add_immunity(&mut self, id: impl Into<AfflictionId>) {
        self.immunities.insert(id.into());
    }

    /// Remove immunity.
    pub fn remove_immunity(&mut self, id: &AfflictionId) -> bool {
        self.immunities.remove(id)
    }

    /// Check if immune.
    #[must_use]
    pub fn is_immune(&self, id: &AfflictionId) -> bool {
        self.immunities.contains(id)
    }

    /// Get all immunities.
    pub fn immunities(&self) -> impl Iterator<Item = &AfflictionId> {
        self.immunities.iter()
    }
}

/// Registry of affliction definitions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AfflictionRegistry {
    /// Definitions indexed by ID.
    afflictions: BTreeMap<AfflictionId, AfflictionDef>,
    /// Afflictions indexed by trigger type for fast lookup.
    #[serde(skip)]
    by_trigger_type: BTreeMap<String, Vec<AfflictionId>>,
}

impl AfflictionRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an affliction definition.
    pub fn register(&mut self, def: AfflictionDef) {
        let id = def.id.clone();

        for trigger in &def.triggers {
            let key = trigger_type_key(trigger);
            self.by_trigger_type
                .entry(key)
                .or_default()
                .push(id.clone());
        }

        self.afflictions.insert(id, def);
    }

    /// Get an affliction definition by ID.
    #[must_use]
    pub fn get(&self, id: &AfflictionId) -> Option<&AfflictionDef> {
        self.afflictions.get(id)
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = &AfflictionDef> {
        self.afflictions.values()
    }

    /// Get affliction IDs that may trigger from a scalar field.
    pub fn afflictions_for_scalar_field(
        &self,
        channel: &str,
    ) -> impl Iterator<Item = &AfflictionId> {
        self.by_trigger_type
            .get(&format!("scalar:{channel}"))
            .into_iter()
            .flatten()
    }

    /// Get affliction IDs that may trigger from a hazard.
    pub fn afflictions_for_hazard(&self, kind: &str) -> impl Iterator<Item = &AfflictionId> {
        self.by_trigger_type
            .get(&format!("hazard:{kind}"))
            .into_iter()
            .flatten()
    }

    /// Get affliction IDs that may trigger from a need level.
    pub fn afflictions_for_need(&self, need_id: &NeedId) -> impl Iterator<Item = &AfflictionId> {
        self.by_trigger_type
            .get(&format!("need:{}", need_id.as_str()))
            .into_iter()
            .flatten()
    }

    /// Get affliction IDs that may trigger from activity.
    pub fn afflictions_for_activity(&self, activity: &str) -> impl Iterator<Item = &AfflictionId> {
        self.by_trigger_type
            .get(&format!("activity:{activity}"))
            .into_iter()
            .flatten()
    }

    /// Number of registered afflictions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.afflictions.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.afflictions.is_empty()
    }

    /// Rebuild trigger index (call after deserialization).
    pub fn rebuild_index(&mut self) {
        self.by_trigger_type.clear();
        for (id, def) in &self.afflictions {
            for trigger in &def.triggers {
                let key = trigger_type_key(trigger);
                self.by_trigger_type
                    .entry(key)
                    .or_default()
                    .push(id.clone());
            }
        }
    }

    /// Compute a stable checksum of all registered definitions.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        for def in self.afflictions.values() {
            hasher.update(&def.fingerprint().to_le_bytes());
        }
        hasher.finalize()
    }
}

fn trigger_type_key(trigger: &ExposureTrigger) -> String {
    match trigger {
        ExposureTrigger::ScalarField { channel, .. } => format!("scalar:{channel}"),
        ExposureTrigger::Hazard { kind, .. } => format!("hazard:{kind}"),
        ExposureTrigger::NeedLevel { need_id, .. } => format!("need:{}", need_id.as_str()),
        ExposureTrigger::Activity { activity, .. } => format!("activity:{activity}"),
        ExposureTrigger::ComplicatedBy { affliction_id, .. } => {
            format!("complication:{}", affliction_id.as_str())
        }
    }
}

/// Environment snapshot for exposure evaluation.
#[derive(Clone, Debug, Default)]
pub struct ExposureSnapshot {
    /// Scalar field values by channel name.
    pub scalar_fields: BTreeMap<String, f32>,
    /// Hazard intensities by kind.
    pub hazards: BTreeMap<String, f32>,
    /// Need values.
    pub needs: BTreeMap<NeedId, f32>,
    /// Activity levels by type.
    pub activities: BTreeMap<String, f32>,
}

impl ExposureSnapshot {
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

    /// Set a need value.
    pub fn set_need(&mut self, need_id: NeedId, value: f32) {
        self.needs.insert(need_id, value);
    }

    /// Set an activity level.
    pub fn set_activity(&mut self, activity: impl Into<String>, level: f32) {
        self.activities.insert(activity.into(), level);
    }
}

/// Evaluate exposure rate from a trigger given the current environment.
#[must_use]
pub fn evaluate_exposure(
    trigger: &ExposureTrigger,
    env: &ExposureSnapshot,
    active_afflictions: &AfflictionSet,
) -> f32 {
    match trigger {
        ExposureTrigger::ScalarField {
            channel,
            threshold,
            trigger_above,
            rate,
        } => {
            let value = env.scalar_fields.get(channel).copied().unwrap_or(0.0);
            let triggered = if *trigger_above {
                value > *threshold
            } else {
                value < *threshold
            };
            if triggered { *rate } else { 0.0 }
        }
        ExposureTrigger::Hazard {
            kind,
            min_intensity,
            rate_per_intensity,
        } => {
            let intensity = env.hazards.get(kind).copied().unwrap_or(0.0);
            if intensity >= *min_intensity {
                intensity * rate_per_intensity
            } else {
                0.0
            }
        }
        ExposureTrigger::NeedLevel {
            need_id,
            threshold,
            trigger_above,
            rate,
        } => {
            let value = env.needs.get(need_id).copied().unwrap_or(100.0);
            let triggered = if *trigger_above {
                value > *threshold
            } else {
                value < *threshold
            };
            if triggered { *rate } else { 0.0 }
        }
        ExposureTrigger::Activity {
            activity,
            rate_per_unit,
        } => {
            let level = env.activities.get(activity).copied().unwrap_or(0.0);
            level * rate_per_unit
        }
        ExposureTrigger::ComplicatedBy {
            affliction_id,
            min_severity,
            rate,
        } => {
            let has_complication = active_afflictions
                .get(affliction_id)
                .is_some_and(|a| a.severity >= *min_severity);
            if has_complication { *rate } else { 0.0 }
        }
    }
}

/// Check if recovery should happen.
#[must_use]
pub fn should_recover(
    recovery: &RecoveryMode,
    env: &ExposureSnapshot,
    under_treatment: bool,
) -> Option<f32> {
    match recovery {
        RecoveryMode::Linear { rate } => Some(*rate),
        RecoveryMode::Conditional {
            need_id,
            threshold,
            rate,
        } => {
            let value = env.needs.get(need_id).copied().unwrap_or(0.0);
            if value >= *threshold {
                Some(*rate)
            } else {
                None
            }
        }
        RecoveryMode::TreatmentRequired => {
            if under_treatment {
                Some(1.0)
            } else {
                None
            }
        }
        RecoveryMode::Permanent => None,
    }
}

/// Result of affliction tick processing.
#[derive(Clone, Debug, PartialEq)]
pub struct AfflictionTickResult {
    /// Afflictions that changed severity.
    pub severity_changes: Vec<SeverityChange>,
    /// Afflictions that were fully cured (reached None severity).
    pub cured: Vec<AfflictionId>,
    /// Status effects to apply based on current severity levels.
    pub effects_to_apply: Vec<StatusEffectId>,
    /// Status effects to remove (severity dropped below requirement).
    pub effects_to_remove: Vec<StatusEffectId>,
}

/// Record of a severity change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeverityChange {
    pub affliction_id: AfflictionId,
    pub previous: Severity,
    pub current: Severity,
    pub tick: u64,
}

/// Collection of active afflictions for a single entity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AfflictionSet {
    /// Active afflictions by ID.
    afflictions: BTreeMap<AfflictionId, ActiveAffliction>,
    /// Resistance configuration.
    resistances: ResistanceSet,
    /// Current tick.
    current_tick: u64,
}

impl AfflictionSet {
    /// Create an empty affliction set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the resistance set.
    #[must_use]
    pub fn resistances(&self) -> &ResistanceSet {
        &self.resistances
    }

    /// Get mutable resistance set.
    pub fn resistances_mut(&mut self) -> &mut ResistanceSet {
        &mut self.resistances
    }

    /// Get an active affliction by ID.
    #[must_use]
    pub fn get(&self, id: &AfflictionId) -> Option<&ActiveAffliction> {
        self.afflictions.get(id)
    }

    /// Get mutable affliction by ID.
    pub fn get_mut(&mut self, id: &AfflictionId) -> Option<&mut ActiveAffliction> {
        self.afflictions.get_mut(id)
    }

    /// Check if an affliction is active (severity > None).
    #[must_use]
    pub fn has(&self, id: &AfflictionId) -> bool {
        self.afflictions
            .get(id)
            .is_some_and(|a| a.severity != Severity::None)
    }

    /// Check if an affliction is at or above a severity level.
    #[must_use]
    pub fn has_at_severity(&self, id: &AfflictionId, min_severity: Severity) -> bool {
        self.afflictions
            .get(id)
            .is_some_and(|a| a.severity >= min_severity)
    }

    /// Get current severity of an affliction (None if not present).
    #[must_use]
    pub fn severity(&self, id: &AfflictionId) -> Severity {
        self.afflictions
            .get(id)
            .map_or(Severity::None, |a| a.severity)
    }

    /// Get all active afflictions.
    pub fn iter(&self) -> impl Iterator<Item = &ActiveAffliction> {
        self.afflictions.values()
    }

    /// Get all active afflictions with non-None severity.
    pub fn active_iter(&self) -> impl Iterator<Item = &ActiveAffliction> {
        self.afflictions
            .values()
            .filter(|a| a.severity != Severity::None)
    }

    /// Number of afflictions (including those with None severity).
    #[must_use]
    pub fn len(&self) -> usize {
        self.afflictions.len()
    }

    /// Number of active afflictions (severity > None).
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.afflictions
            .values()
            .filter(|a| a.severity != Severity::None)
            .count()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.afflictions.is_empty()
    }

    /// Current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Apply exposure for an affliction, creating it if needed.
    pub fn apply_exposure(
        &mut self,
        registry: &AfflictionRegistry,
        id: &AfflictionId,
        amount: f32,
    ) -> Option<Severity> {
        if self.resistances.is_immune(id) {
            return None;
        }

        let resistance = self.resistances.resistance(id);
        let adjusted_amount = amount * resistance;

        if adjusted_amount <= 0.0 {
            return None;
        }

        let def = registry.get(id)?;
        let tick = self.current_tick;

        let affliction = self
            .afflictions
            .entry(id.clone())
            .or_insert_with(|| ActiveAffliction::new(id.clone(), tick));

        affliction.add_exposure(adjusted_amount);
        affliction.update_severity(&def.severity_thresholds, tick);

        Some(affliction.severity)
    }

    /// Process a full tick of affliction updates.
    pub fn tick(
        &mut self,
        registry: &AfflictionRegistry,
        env: &ExposureSnapshot,
    ) -> AfflictionTickResult {
        self.current_tick += 1;
        let tick = self.current_tick;

        let mut result = AfflictionTickResult {
            severity_changes: Vec::new(),
            cured: Vec::new(),
            effects_to_apply: Vec::new(),
            effects_to_remove: Vec::new(),
        };

        // Collect affliction IDs and their previous severities before any mutations
        let ids: Vec<_> = self.afflictions.keys().cloned().collect();
        let previous_severities: BTreeMap<AfflictionId, Severity> = self
            .afflictions
            .iter()
            .map(|(id, a)| (id.clone(), a.severity))
            .collect();

        // First pass: process exposure from environment triggers
        for id in &ids {
            let Some(def) = registry.get(id) else {
                continue;
            };

            let total_exposure: f32 = def
                .triggers
                .iter()
                .map(|t| evaluate_exposure(t, env, self))
                .sum();

            if total_exposure > 0.0 {
                self.apply_exposure(registry, id, total_exposure);
            }
        }

        // Second pass: process recovery and severity changes
        for id in &ids {
            let Some(def) = registry.get(id) else {
                continue;
            };

            let under_treatment = self.afflictions.get(id).is_some_and(|a| a.under_treatment);

            // Precompute exposure and resistance before taking mutable borrow
            let current_exposure: f32 = def
                .triggers
                .iter()
                .map(|t| evaluate_exposure(t, env, self))
                .sum();
            let resistance = self.resistances.resistance(id);

            let previous_severity = previous_severities
                .get(id)
                .copied()
                .unwrap_or(Severity::None);

            let Some(affliction) = self.afflictions.get_mut(id) else {
                continue;
            };

            // Check for recovery
            if let Some(recovery_rate) = should_recover(&def.recovery, env, under_treatment) {
                // Only recover if not currently receiving exposure
                if current_exposure <= 0.0 {
                    affliction.remove_exposure(recovery_rate / resistance.max(0.1));
                    affliction.update_severity(&def.severity_thresholds, tick);
                }
            }

            let current_severity = affliction.severity;

            // Record severity change
            if current_severity != previous_severity {
                result.severity_changes.push(SeverityChange {
                    affliction_id: id.clone(),
                    previous: previous_severity,
                    current: current_severity,
                    tick,
                });

                // Determine effects to apply/remove
                for effect in &def.effects {
                    let should_apply = current_severity >= effect.min_severity;
                    let was_applied = previous_severity >= effect.min_severity;

                    if should_apply && !was_applied {
                        result.effects_to_apply.push(effect.effect_id.clone());
                    } else if !should_apply && was_applied {
                        result.effects_to_remove.push(effect.effect_id.clone());
                    }
                }

                // Check if cured
                if current_severity == Severity::None {
                    result.cured.push(id.clone());
                }
            }
        }

        // Remove cured afflictions
        for id in &result.cured {
            self.afflictions.remove(id);
        }

        result
    }

    /// Set treatment status for an affliction.
    pub fn set_treatment(&mut self, id: &AfflictionId, under_treatment: bool) -> bool {
        if let Some(affliction) = self.afflictions.get_mut(id) {
            affliction.under_treatment = under_treatment;
            true
        } else {
            false
        }
    }

    /// Get combined decay multiplier for a need from all active afflictions.
    #[must_use]
    pub fn combined_decay_multiplier(
        &self,
        registry: &AfflictionRegistry,
        need_id: &NeedId,
    ) -> f32 {
        let mut multiplier = 1.0;

        for affliction in self.afflictions.values() {
            if let Some(def) = registry.get(&affliction.id) {
                for modifier in &def.modifiers {
                    if &modifier.need_id == need_id && affliction.severity >= modifier.min_severity
                    {
                        multiplier *= modifier.decay_multiplier;
                    }
                }
            }
        }

        multiplier
    }

    /// Get combined recovery multiplier for a need from all active afflictions.
    #[must_use]
    pub fn combined_recovery_multiplier(
        &self,
        registry: &AfflictionRegistry,
        need_id: &NeedId,
    ) -> f32 {
        let mut multiplier = 1.0;

        for affliction in self.afflictions.values() {
            if let Some(def) = registry.get(&affliction.id) {
                for modifier in &def.modifiers {
                    if &modifier.need_id == need_id && affliction.severity >= modifier.min_severity
                    {
                        multiplier *= modifier.recovery_multiplier;
                    }
                }
            }
        }

        multiplier
    }

    /// Get combined tick delta for a need from all active afflictions.
    #[must_use]
    pub fn combined_tick_delta(&self, registry: &AfflictionRegistry, need_id: &NeedId) -> f32 {
        let mut delta = 0.0;

        for affliction in self.afflictions.values() {
            if let Some(def) = registry.get(&affliction.id) {
                for modifier in &def.modifiers {
                    if &modifier.need_id == need_id && affliction.severity >= modifier.min_severity
                    {
                        delta += modifier.tick_delta;
                    }
                }
            }
        }

        delta
    }

    /// Compute a stable checksum of current state.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.current_tick.to_le_bytes());
        for (id, affliction) in &self.afflictions {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&affliction.exposure.to_le_bytes());
            hasher.update(&[affliction.severity.as_index()]);
        }
        hasher.finalize()
    }
}

/// Built-in affliction presets for common conditions.
pub mod presets {
    use super::{
        AfflictionCategory, AfflictionDef, AfflictionRegistry, ExposureTrigger, NeedId,
        RecoveryMode, Severity, SeverityEffect, SeverityModifier, SeverityThresholds,
        StatusEffectId,
    };

    /// Frostbite - cold exposure damage to extremities.
    #[must_use]
    pub fn frostbite() -> AfflictionDef {
        AfflictionDef::new("frostbite", "Frostbite")
            .with_category(AfflictionCategory::Environmental)
            .with_thresholds(SeverityThresholds::new(20.0, 50.0, 80.0, 100.0))
            .with_trigger(ExposureTrigger::ScalarField {
                channel: "temperature".to_string(),
                threshold: 5.0,
                trigger_above: false,
                rate: 0.5,
            })
            .with_recovery(RecoveryMode::Conditional {
                need_id: NeedId::warmth(),
                threshold: 60.0,
                rate: 0.2,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Mild,
                need_id: NeedId::warmth(),
                decay_multiplier: 1.2,
                recovery_multiplier: 0.9,
                tick_delta: 0.0,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Severe,
                need_id: NeedId::warmth(),
                decay_multiplier: 1.5,
                recovery_multiplier: 0.5,
                tick_delta: -0.5,
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Moderate,
                effect_id: StatusEffectId::new("movement_impaired"),
            })
    }

    /// Decompression sickness (the bends) - rapid pressure change.
    #[must_use]
    pub fn bends() -> AfflictionDef {
        AfflictionDef::new("bends", "Decompression Sickness")
            .with_category(AfflictionCategory::Pressure)
            .with_thresholds(SeverityThresholds::new(30.0, 60.0, 85.0, 100.0))
            .with_trigger(ExposureTrigger::Activity {
                activity: "rapid_ascent".to_string(),
                rate_per_unit: 2.0,
            })
            .with_recovery(RecoveryMode::TreatmentRequired)
            .with_modifier(SeverityModifier {
                min_severity: Severity::Mild,
                need_id: NeedId::oxygen(),
                decay_multiplier: 1.3,
                recovery_multiplier: 0.8,
                tick_delta: 0.0,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Severe,
                need_id: NeedId::hunger(),
                decay_multiplier: 1.0,
                recovery_multiplier: 1.0,
                tick_delta: -1.0,
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Moderate,
                effect_id: StatusEffectId::new("confused"),
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Critical,
                effect_id: StatusEffectId::new("paralyzed"),
            })
    }

    /// Infection - wound contamination.
    #[must_use]
    pub fn infection() -> AfflictionDef {
        AfflictionDef::new("infection", "Infection")
            .with_category(AfflictionCategory::Biological)
            .with_thresholds(SeverityThresholds::new(15.0, 40.0, 70.0, 100.0))
            .with_trigger(ExposureTrigger::Hazard {
                kind: "contamination".to_string(),
                min_intensity: 0.3,
                rate_per_intensity: 0.8,
            })
            .with_recovery(RecoveryMode::TreatmentRequired)
            .with_modifier(SeverityModifier {
                min_severity: Severity::Mild,
                need_id: NeedId::rest(),
                decay_multiplier: 1.2,
                recovery_multiplier: 0.9,
                tick_delta: 0.0,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Moderate,
                need_id: NeedId::hunger(),
                decay_multiplier: 1.4,
                recovery_multiplier: 0.7,
                tick_delta: -0.3,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Critical,
                need_id: NeedId::hunger(),
                decay_multiplier: 2.0,
                recovery_multiplier: 0.3,
                tick_delta: -1.5,
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Moderate,
                effect_id: StatusEffectId::new("fever"),
            })
    }

    /// Spore infection - fungal contamination.
    #[must_use]
    pub fn spores() -> AfflictionDef {
        AfflictionDef::new("spores", "Spore Infection")
            .with_category(AfflictionCategory::Biological)
            .with_thresholds(SeverityThresholds::new(25.0, 55.0, 80.0, 100.0))
            .with_trigger(ExposureTrigger::ScalarField {
                channel: "spore_density".to_string(),
                threshold: 0.2,
                trigger_above: true,
                rate: 0.3,
            })
            .with_trigger(ExposureTrigger::Hazard {
                kind: "spores".to_string(),
                min_intensity: 0.1,
                rate_per_intensity: 1.0,
            })
            .with_recovery(RecoveryMode::Linear { rate: 0.1 })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Mild,
                need_id: NeedId::oxygen(),
                decay_multiplier: 1.3,
                recovery_multiplier: 0.8,
                tick_delta: 0.0,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Severe,
                need_id: NeedId::morale(),
                decay_multiplier: 1.5,
                recovery_multiplier: 0.5,
                tick_delta: -0.5,
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Mild,
                effect_id: StatusEffectId::new("coughing"),
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Severe,
                effect_id: StatusEffectId::new("hallucinating"),
            })
    }

    /// Radiation sickness - ionizing radiation exposure.
    #[must_use]
    pub fn radiation_sickness() -> AfflictionDef {
        AfflictionDef::new("radiation_sickness", "Radiation Sickness")
            .with_category(AfflictionCategory::Chemical)
            .with_thresholds(SeverityThresholds::new(30.0, 60.0, 85.0, 100.0))
            .with_trigger(ExposureTrigger::ScalarField {
                channel: "radiation".to_string(),
                threshold: 50.0,
                trigger_above: true,
                rate: 0.4,
            })
            .with_recovery(RecoveryMode::Linear { rate: 0.05 })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Mild,
                need_id: NeedId::hunger(),
                decay_multiplier: 1.3,
                recovery_multiplier: 0.8,
                tick_delta: 0.0,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Moderate,
                need_id: NeedId::rest(),
                decay_multiplier: 1.5,
                recovery_multiplier: 0.6,
                tick_delta: -0.3,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Critical,
                need_id: NeedId::hunger(),
                decay_multiplier: 2.0,
                recovery_multiplier: 0.2,
                tick_delta: -2.0,
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Moderate,
                effect_id: StatusEffectId::new("nausea"),
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Critical,
                effect_id: StatusEffectId::new("bleeding"),
            })
    }

    /// Fatigue - physical exhaustion.
    #[must_use]
    pub fn fatigue() -> AfflictionDef {
        AfflictionDef::new("fatigue", "Fatigue")
            .with_category(AfflictionCategory::Physical)
            .with_thresholds(SeverityThresholds::new(20.0, 45.0, 70.0, 90.0))
            .with_trigger(ExposureTrigger::NeedLevel {
                need_id: NeedId::rest(),
                threshold: 30.0,
                trigger_above: false,
                rate: 0.3,
            })
            .with_trigger(ExposureTrigger::Activity {
                activity: "exertion".to_string(),
                rate_per_unit: 0.5,
            })
            .with_recovery(RecoveryMode::Conditional {
                need_id: NeedId::rest(),
                threshold: 50.0,
                rate: 0.4,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Mild,
                need_id: NeedId::rest(),
                decay_multiplier: 1.2,
                recovery_multiplier: 0.9,
                tick_delta: 0.0,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Moderate,
                need_id: NeedId::morale(),
                decay_multiplier: 1.3,
                recovery_multiplier: 0.8,
                tick_delta: -0.2,
            })
            .with_modifier(SeverityModifier {
                min_severity: Severity::Severe,
                need_id: NeedId::rest(),
                decay_multiplier: 1.8,
                recovery_multiplier: 0.4,
                tick_delta: -0.5,
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Moderate,
                effect_id: StatusEffectId::new("slowed"),
            })
            .with_effect(SeverityEffect {
                min_severity: Severity::Critical,
                effect_id: StatusEffectId::new("exhausted"),
            })
    }

    /// Create a registry with all preset afflictions.
    #[must_use]
    pub fn create_preset_registry() -> AfflictionRegistry {
        let mut registry = AfflictionRegistry::new();
        registry.register(frostbite());
        registry.register(bends());
        registry.register(infection());
        registry.register(spores());
        registry.register(radiation_sickness());
        registry.register(fatigue());
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affliction_id_constants() {
        assert_eq!(AfflictionId::frostbite().as_str(), "frostbite");
        assert_eq!(AfflictionId::bends().as_str(), "bends");
        assert_eq!(AfflictionId::infection().as_str(), "infection");
        assert_eq!(AfflictionId::spores().as_str(), "spores");
        assert_eq!(
            AfflictionId::radiation_sickness().as_str(),
            "radiation_sickness"
        );
        assert_eq!(AfflictionId::fatigue().as_str(), "fatigue");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::None < Severity::Mild);
        assert!(Severity::Mild < Severity::Moderate);
        assert!(Severity::Moderate < Severity::Severe);
        assert!(Severity::Severe < Severity::Critical);
    }

    #[test]
    fn test_severity_progression() {
        assert_eq!(Severity::None.worse(), Severity::Mild);
        assert_eq!(Severity::Mild.worse(), Severity::Moderate);
        assert_eq!(Severity::Critical.worse(), Severity::Critical);

        assert_eq!(Severity::Critical.better(), Severity::Severe);
        assert_eq!(Severity::Mild.better(), Severity::None);
        assert_eq!(Severity::None.better(), Severity::None);
    }

    #[test]
    fn test_severity_thresholds_classify() {
        let thresholds = SeverityThresholds::new(25.0, 50.0, 75.0, 100.0);

        assert_eq!(thresholds.classify(0.0), Severity::None);
        assert_eq!(thresholds.classify(24.9), Severity::None);
        assert_eq!(thresholds.classify(25.0), Severity::Mild);
        assert_eq!(thresholds.classify(49.9), Severity::Mild);
        assert_eq!(thresholds.classify(50.0), Severity::Moderate);
        assert_eq!(thresholds.classify(74.9), Severity::Moderate);
        assert_eq!(thresholds.classify(75.0), Severity::Severe);
        assert_eq!(thresholds.classify(99.9), Severity::Severe);
        assert_eq!(thresholds.classify(100.0), Severity::Critical);
        assert_eq!(thresholds.classify(150.0), Severity::Critical);
    }

    #[test]
    fn test_affliction_def_builder() {
        let def = AfflictionDef::new("test", "Test Affliction")
            .with_category(AfflictionCategory::Biological)
            .with_thresholds(SeverityThresholds::new(10.0, 20.0, 30.0, 40.0));

        assert_eq!(def.id.as_str(), "test");
        assert_eq!(def.name, "Test Affliction");
        assert_eq!(def.category, AfflictionCategory::Biological);
        assert!((def.severity_thresholds.mild - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_affliction_def_fingerprint_deterministic() {
        let def1 = AfflictionDef::new("test", "Test");
        let def2 = AfflictionDef::new("test", "Test");

        assert_eq!(def1.fingerprint(), def2.fingerprint());
    }

    #[test]
    fn test_affliction_def_fingerprint_differs() {
        let def1 = AfflictionDef::new("test1", "Test");
        let def2 = AfflictionDef::new("test2", "Test");

        assert_ne!(def1.fingerprint(), def2.fingerprint());
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = AfflictionRegistry::new();
        registry.register(AfflictionDef::new("test", "Test"));

        assert!(registry.get(&AfflictionId::new("test")).is_some());
        assert!(registry.get(&AfflictionId::new("nonexistent")).is_none());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_trigger_indexing() {
        let mut registry = AfflictionRegistry::new();

        let def = AfflictionDef::new("cold_aff", "Cold Affliction").with_trigger(
            ExposureTrigger::ScalarField {
                channel: "temperature".to_string(),
                threshold: 10.0,
                trigger_above: false,
                rate: 0.5,
            },
        );

        registry.register(def);

        let ids: Vec<_> = registry
            .afflictions_for_scalar_field("temperature")
            .collect();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].as_str(), "cold_aff");
    }

    #[test]
    fn test_registry_rebuild_index() {
        let mut registry = AfflictionRegistry::new();
        registry.register(AfflictionDef::new("test", "Test").with_trigger(
            ExposureTrigger::Hazard {
                kind: "fire".to_string(),
                min_intensity: 0.5,
                rate_per_intensity: 1.0,
            },
        ));

        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: AfflictionRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();

        let ids: Vec<_> = restored.afflictions_for_hazard("fire").collect();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn test_resistance_set() {
        let mut resistances = ResistanceSet::new();

        assert!((resistances.resistance(&AfflictionId::frostbite()) - 1.0).abs() < f32::EPSILON);

        resistances.set_resistance(AfflictionId::frostbite(), 0.5);
        assert!((resistances.resistance(&AfflictionId::frostbite()) - 0.5).abs() < f32::EPSILON);

        resistances.add_immunity(AfflictionId::infection());
        assert!(resistances.is_immune(&AfflictionId::infection()));
        assert!((resistances.resistance(&AfflictionId::infection())).abs() < f32::EPSILON);
    }

    #[test]
    fn test_active_affliction_exposure() {
        let mut affliction = ActiveAffliction::new(AfflictionId::frostbite(), 0);
        let thresholds = SeverityThresholds::default();

        assert_eq!(affliction.severity, Severity::None);

        affliction.add_exposure(30.0);
        affliction.update_severity(&thresholds, 1);
        assert_eq!(affliction.severity, Severity::Mild);

        affliction.add_exposure(30.0);
        affliction.update_severity(&thresholds, 2);
        assert_eq!(affliction.severity, Severity::Moderate);

        affliction.remove_exposure(20.0);
        affliction.update_severity(&thresholds, 3);
        assert_eq!(affliction.severity, Severity::Mild);
    }

    #[test]
    fn test_affliction_set_apply_exposure() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        let severity = set.apply_exposure(&registry, &AfflictionId::frostbite(), 30.0);
        assert_eq!(severity, Some(Severity::Mild));

        let severity = set.apply_exposure(&registry, &AfflictionId::frostbite(), 30.0);
        assert_eq!(severity, Some(Severity::Moderate));
    }

    #[test]
    fn test_affliction_set_immunity() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.resistances_mut()
            .add_immunity(AfflictionId::frostbite());

        let severity = set.apply_exposure(&registry, &AfflictionId::frostbite(), 100.0);
        assert_eq!(severity, None);
        assert!(!set.has(&AfflictionId::frostbite()));
    }

    #[test]
    fn test_affliction_set_resistance() {
        let registry = presets::create_preset_registry();
        let mut set1 = AfflictionSet::new();
        let mut set2 = AfflictionSet::new();

        set2.resistances_mut()
            .set_resistance(AfflictionId::frostbite(), 0.5);

        set1.apply_exposure(&registry, &AfflictionId::frostbite(), 40.0);
        set2.apply_exposure(&registry, &AfflictionId::frostbite(), 40.0);

        let exp1 = set1.get(&AfflictionId::frostbite()).unwrap().exposure;
        let exp2 = set2.get(&AfflictionId::frostbite()).unwrap().exposure;

        assert!((exp1 - 40.0).abs() < f32::EPSILON);
        assert!((exp2 - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_affliction_set_tick_exposure() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 10.0);

        let mut env = ExposureSnapshot::new();
        env.set_scalar("temperature", 0.0);

        set.tick(&registry, &env);

        let affliction = set.get(&AfflictionId::frostbite()).unwrap();
        assert!(affliction.exposure > 10.0);
    }

    #[test]
    fn test_affliction_set_tick_recovery() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 30.0);

        let mut env = ExposureSnapshot::new();
        env.set_scalar("temperature", 20.0);
        env.set_need(NeedId::warmth(), 70.0);

        let initial_exposure = set.get(&AfflictionId::frostbite()).unwrap().exposure;

        set.tick(&registry, &env);

        let new_exposure = set.get(&AfflictionId::frostbite()).unwrap().exposure;
        assert!(new_exposure < initial_exposure);
    }

    #[test]
    fn test_affliction_set_severity_change_events() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 19.5);

        let mut env = ExposureSnapshot::new();
        env.set_scalar("temperature", 0.0);

        let result = set.tick(&registry, &env);

        assert!(!result.severity_changes.is_empty());
        assert_eq!(result.severity_changes[0].previous, Severity::None);
        assert_eq!(result.severity_changes[0].current, Severity::Mild);
    }

    #[test]
    fn test_affliction_set_cure() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::fatigue(), 25.0);

        let mut env = ExposureSnapshot::new();
        env.set_need(NeedId::rest(), 80.0);

        for _ in 0..200 {
            let result = set.tick(&registry, &env);
            if !result.cured.is_empty() {
                assert!(result.cured.contains(&AfflictionId::fatigue()));
                assert!(!set.has(&AfflictionId::fatigue()));
                return;
            }
        }

        panic!("Affliction should have been cured");
    }

    #[test]
    fn test_affliction_set_combined_modifiers() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 60.0);

        let decay_mult = set.combined_decay_multiplier(&registry, &NeedId::warmth());
        assert!(decay_mult > 1.0);

        let recovery_mult = set.combined_recovery_multiplier(&registry, &NeedId::warmth());
        assert!(recovery_mult < 1.0);
    }

    #[test]
    fn test_affliction_set_checksum_deterministic() {
        let registry = presets::create_preset_registry();
        let mut set1 = AfflictionSet::new();
        let mut set2 = AfflictionSet::new();

        set1.apply_exposure(&registry, &AfflictionId::frostbite(), 50.0);
        set2.apply_exposure(&registry, &AfflictionId::frostbite(), 50.0);

        assert_eq!(set1.checksum(), set2.checksum());
    }

    #[test]
    fn test_evaluate_exposure_scalar_field() {
        let set = AfflictionSet::new();

        let trigger = ExposureTrigger::ScalarField {
            channel: "temperature".to_string(),
            threshold: 10.0,
            trigger_above: false,
            rate: 0.5,
        };

        let mut env = ExposureSnapshot::new();
        env.set_scalar("temperature", 5.0);

        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!((exposure - 0.5).abs() < f32::EPSILON);

        env.set_scalar("temperature", 15.0);
        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!(exposure.abs() < f32::EPSILON);
    }

    #[test]
    fn test_evaluate_exposure_hazard() {
        let set = AfflictionSet::new();

        let trigger = ExposureTrigger::Hazard {
            kind: "fire".to_string(),
            min_intensity: 0.5,
            rate_per_intensity: 2.0,
        };

        let mut env = ExposureSnapshot::new();
        env.set_hazard("fire", 0.8);

        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!((exposure - 1.6).abs() < f32::EPSILON);

        env.set_hazard("fire", 0.3);
        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!(exposure.abs() < f32::EPSILON);
    }

    #[test]
    fn test_evaluate_exposure_need_level() {
        let set = AfflictionSet::new();

        let trigger = ExposureTrigger::NeedLevel {
            need_id: NeedId::rest(),
            threshold: 30.0,
            trigger_above: false,
            rate: 0.3,
        };

        let mut env = ExposureSnapshot::new();
        env.set_need(NeedId::rest(), 20.0);

        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!((exposure - 0.3).abs() < f32::EPSILON);

        env.set_need(NeedId::rest(), 50.0);
        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!(exposure.abs() < f32::EPSILON);
    }

    #[test]
    fn test_evaluate_exposure_activity() {
        let set = AfflictionSet::new();

        let trigger = ExposureTrigger::Activity {
            activity: "exertion".to_string(),
            rate_per_unit: 0.5,
        };

        let mut env = ExposureSnapshot::new();
        env.set_activity("exertion", 2.0);

        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!((exposure - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_evaluate_exposure_complicated_by() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        let trigger = ExposureTrigger::ComplicatedBy {
            affliction_id: AfflictionId::frostbite(),
            min_severity: Severity::Mild,
            rate: 0.5,
        };

        let env = ExposureSnapshot::new();

        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!(exposure.abs() < f32::EPSILON);

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 30.0);

        let exposure = evaluate_exposure(&trigger, &env, &set);
        assert!((exposure - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_should_recover_linear() {
        let recovery = RecoveryMode::Linear { rate: 0.2 };
        let env = ExposureSnapshot::new();

        let rate = should_recover(&recovery, &env, false);
        assert_eq!(rate, Some(0.2));
    }

    #[test]
    fn test_should_recover_conditional() {
        let recovery = RecoveryMode::Conditional {
            need_id: NeedId::warmth(),
            threshold: 50.0,
            rate: 0.3,
        };

        let mut env = ExposureSnapshot::new();
        env.set_need(NeedId::warmth(), 30.0);

        let rate = should_recover(&recovery, &env, false);
        assert_eq!(rate, None);

        env.set_need(NeedId::warmth(), 60.0);
        let rate = should_recover(&recovery, &env, false);
        assert_eq!(rate, Some(0.3));
    }

    #[test]
    fn test_should_recover_treatment_required() {
        let recovery = RecoveryMode::TreatmentRequired;
        let env = ExposureSnapshot::new();

        let rate = should_recover(&recovery, &env, false);
        assert_eq!(rate, None);

        let rate = should_recover(&recovery, &env, true);
        assert_eq!(rate, Some(1.0));
    }

    #[test]
    fn test_should_recover_permanent() {
        let recovery = RecoveryMode::Permanent;
        let env = ExposureSnapshot::new();

        let rate = should_recover(&recovery, &env, true);
        assert_eq!(rate, None);
    }

    #[test]
    fn test_preset_frostbite() {
        let def = presets::frostbite();
        assert_eq!(def.category, AfflictionCategory::Environmental);
        assert!(!def.triggers.is_empty());
        assert!(!def.modifiers.is_empty());
    }

    #[test]
    fn test_preset_bends() {
        let def = presets::bends();
        assert_eq!(def.category, AfflictionCategory::Pressure);
        assert!(matches!(def.recovery, RecoveryMode::TreatmentRequired));
    }

    #[test]
    fn test_preset_infection() {
        let def = presets::infection();
        assert_eq!(def.category, AfflictionCategory::Biological);
    }

    #[test]
    fn test_preset_spores() {
        let def = presets::spores();
        assert_eq!(def.category, AfflictionCategory::Biological);
        assert_eq!(def.triggers.len(), 2);
    }

    #[test]
    fn test_preset_radiation_sickness() {
        let def = presets::radiation_sickness();
        assert_eq!(def.category, AfflictionCategory::Chemical);
    }

    #[test]
    fn test_preset_fatigue() {
        let def = presets::fatigue();
        assert_eq!(def.category, AfflictionCategory::Physical);
        assert_eq!(def.triggers.len(), 2);
    }

    #[test]
    fn test_preset_registry() {
        let registry = presets::create_preset_registry();
        assert_eq!(registry.len(), 6);
        assert!(registry.get(&AfflictionId::frostbite()).is_some());
        assert!(registry.get(&AfflictionId::bends()).is_some());
        assert!(registry.get(&AfflictionId::infection()).is_some());
        assert!(registry.get(&AfflictionId::spores()).is_some());
        assert!(registry.get(&AfflictionId::radiation_sickness()).is_some());
        assert!(registry.get(&AfflictionId::fatigue()).is_some());
    }

    #[test]
    fn test_serde_round_trip_affliction_def() {
        let def = presets::frostbite();
        let json = serde_json::to_string(&def).unwrap();
        let restored: AfflictionDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def.id, restored.id);
        assert_eq!(def.name, restored.name);
        assert_eq!(def.category, restored.category);
    }

    #[test]
    fn test_serde_round_trip_registry() {
        let registry = presets::create_preset_registry();
        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: AfflictionRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();
        assert_eq!(registry.len(), restored.len());
    }

    #[test]
    fn test_serde_round_trip_affliction_set() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();
        set.apply_exposure(&registry, &AfflictionId::frostbite(), 50.0);
        set.resistances_mut()
            .add_immunity(AfflictionId::infection());

        let json = serde_json::to_string(&set).unwrap();
        let restored: AfflictionSet = serde_json::from_str(&json).unwrap();

        assert!(restored.has(&AfflictionId::frostbite()));
        assert!(restored.resistances().is_immune(&AfflictionId::infection()));
    }

    #[test]
    fn test_affliction_set_active_count() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        assert_eq!(set.active_count(), 0);

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 30.0);
        assert_eq!(set.active_count(), 1);

        set.apply_exposure(&registry, &AfflictionId::fatigue(), 25.0);
        assert_eq!(set.active_count(), 2);
    }

    #[test]
    fn test_affliction_set_has_at_severity() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 30.0);

        assert!(set.has_at_severity(&AfflictionId::frostbite(), Severity::Mild));
        assert!(!set.has_at_severity(&AfflictionId::frostbite(), Severity::Moderate));
    }

    #[test]
    fn test_effects_applied_on_severity_change() {
        let registry = presets::create_preset_registry();
        let mut set = AfflictionSet::new();

        set.apply_exposure(&registry, &AfflictionId::frostbite(), 49.5);

        let mut env = ExposureSnapshot::new();
        env.set_scalar("temperature", 0.0);

        let result = set.tick(&registry, &env);

        assert!(
            result
                .severity_changes
                .iter()
                .any(|c| c.current == Severity::Moderate)
        );
        assert!(
            result
                .effects_to_apply
                .contains(&StatusEffectId::new("movement_impaired"))
        );
    }

    #[test]
    fn test_treatment_required_recovery() {
        let mut registry = AfflictionRegistry::new();
        registry.register(
            AfflictionDef::new("test_treatment", "Test")
                .with_recovery(RecoveryMode::TreatmentRequired)
                .with_thresholds(SeverityThresholds::new(10.0, 20.0, 30.0, 40.0)),
        );

        let mut set = AfflictionSet::new();
        let id = AfflictionId::new("test_treatment");

        set.apply_exposure(&registry, &id, 15.0);

        let env = ExposureSnapshot::new();

        let initial = set.get(&id).unwrap().exposure;
        set.tick(&registry, &env);
        let after_no_treatment = set.get(&id).unwrap().exposure;

        assert!((initial - after_no_treatment).abs() < f32::EPSILON);

        set.set_treatment(&id, true);
        set.tick(&registry, &env);
        let after_treatment = set.get(&id).unwrap().exposure;

        assert!(after_treatment < initial);
    }

    #[test]
    fn test_registry_checksum_deterministic() {
        let reg1 = presets::create_preset_registry();
        let reg2 = presets::create_preset_registry();

        assert_eq!(reg1.checksum(), reg2.checksum());
    }
}

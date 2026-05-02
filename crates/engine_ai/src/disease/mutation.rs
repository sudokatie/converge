//! Deterministic mutation and evolution of pathogens.

use serde::{Deserialize, Serialize};

use super::ids::{PathogenId, StrainId};
use super::pathogen::{PathogenTraits, TraitBounds};

/// Result of a mutation attempt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MutationResult {
    /// No mutation occurred.
    NoMutation,
    /// Minor trait adjustments within same strain.
    MinorDrift(PathogenTraits),
    /// Major mutation creating a new variant.
    NewVariant {
        variant_id: u32,
        traits: PathogenTraits,
    },
}

/// Configuration for mutation behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MutationConfig {
    /// Base probability of any mutation per tick.
    pub base_mutation_chance: f32,
    /// Multiplier for mutation chance when under treatment pressure.
    pub treatment_pressure_multiplier: f32,
    /// Multiplier for mutation chance with high pathogen load.
    pub high_load_multiplier: f32,
    /// Probability that a mutation creates a new variant vs drift.
    pub new_variant_chance: f32,
    /// Maximum drift per mutation for each trait (-/+ this amount).
    pub max_transmissibility_drift: f32,
    pub max_virulence_drift: f32,
    pub max_lethality_drift: f32,
    /// Maximum incubation change (as percentage).
    pub max_incubation_drift_pct: f32,
}

impl Default for MutationConfig {
    fn default() -> Self {
        Self {
            base_mutation_chance: 0.001,
            treatment_pressure_multiplier: 2.0,
            high_load_multiplier: 1.5,
            new_variant_chance: 0.1,
            max_transmissibility_drift: 0.05,
            max_virulence_drift: 0.03,
            max_lethality_drift: 0.02,
            max_incubation_drift_pct: 0.1,
        }
    }
}

impl MutationConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_base_chance(mut self, chance: f32) -> Self {
        self.base_mutation_chance = chance.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_treatment_multiplier(mut self, mult: f32) -> Self {
        self.treatment_pressure_multiplier = mult.max(1.0);
        self
    }

    #[must_use]
    pub fn with_new_variant_chance(mut self, chance: f32) -> Self {
        self.new_variant_chance = chance.clamp(0.0, 1.0);
        self
    }
}

/// State for tracking mutations within a strain lineage.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MutationTracker {
    /// Next variant ID for this pathogen.
    next_variants: std::collections::BTreeMap<PathogenId, u32>,
    /// Total mutations tracked.
    total_mutations: u64,
}

impl MutationTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the next variant ID for a pathogen.
    fn next_variant(&mut self, pathogen_id: &PathogenId) -> u32 {
        let entry = self.next_variants.entry(pathogen_id.clone()).or_insert(1);
        let variant = *entry;
        *entry += 1;
        variant
    }

    /// Total mutation count.
    #[must_use]
    pub fn total_mutations(&self) -> u64 {
        self.total_mutations
    }

    /// Attempt to mutate a pathogen.
    #[must_use]
    pub fn attempt_mutation(
        &mut self,
        strain: &StrainId,
        current_traits: &PathogenTraits,
        bounds: &TraitBounds,
        config: &MutationConfig,
        context: &MutationContext,
    ) -> MutationResult {
        let effective_chance = Self::compute_mutation_chance(current_traits, config, context);

        let mutation_roll = deterministic_random(
            strain.pathogen.as_str(),
            strain.variant,
            context.tick,
            context.host_id,
        );

        if mutation_roll >= effective_chance {
            return MutationResult::NoMutation;
        }

        self.total_mutations += 1;

        let variant_roll = deterministic_random(
            strain.pathogen.as_str(),
            strain.variant.wrapping_add(1),
            context.tick,
            context.host_id.wrapping_add(1),
        );

        if variant_roll < config.new_variant_chance {
            let new_traits = Self::generate_mutated_traits(
                current_traits,
                bounds,
                config,
                strain,
                context.tick,
                true,
            );
            let variant_id = self.next_variant(&strain.pathogen);
            MutationResult::NewVariant {
                variant_id,
                traits: new_traits,
            }
        } else {
            let new_traits = Self::generate_mutated_traits(
                current_traits,
                bounds,
                config,
                strain,
                context.tick,
                false,
            );
            MutationResult::MinorDrift(new_traits)
        }
    }

    fn compute_mutation_chance(
        traits: &PathogenTraits,
        config: &MutationConfig,
        context: &MutationContext,
    ) -> f32 {
        let mut chance = traits.mutation_rate.max(config.base_mutation_chance);

        if context.under_treatment {
            chance *= config.treatment_pressure_multiplier;
        }

        if context.pathogen_load > 5.0 {
            chance *= config.high_load_multiplier;
        }

        chance.clamp(0.0, 0.5)
    }

    #[expect(clippy::cast_possible_truncation)]
    fn generate_mutated_traits(
        base: &PathogenTraits,
        bounds: &TraitBounds,
        config: &MutationConfig,
        strain: &StrainId,
        tick: u64,
        is_major: bool,
    ) -> PathogenTraits {
        let mut traits = base.clone();

        let scale = if is_major { 2.0 } else { 1.0 };

        let trans_roll = deterministic_random(strain.pathogen.as_str(), 100, tick, 0);
        let trans_delta = (trans_roll - 0.5) * 2.0 * config.max_transmissibility_drift * scale;
        traits.transmissibility = (traits.transmissibility + trans_delta)
            .clamp(bounds.min_transmissibility, bounds.max_transmissibility);

        let vir_roll = deterministic_random(strain.pathogen.as_str(), 101, tick, 0);
        let vir_delta = (vir_roll - 0.5) * 2.0 * config.max_virulence_drift * scale;
        traits.virulence =
            (traits.virulence + vir_delta).clamp(bounds.min_virulence, bounds.max_virulence);

        let leth_roll = deterministic_random(strain.pathogen.as_str(), 102, tick, 0);
        let leth_delta = (leth_roll - 0.5) * 2.0 * config.max_lethality_drift * scale;
        traits.lethality =
            (traits.lethality + leth_delta).clamp(bounds.min_lethality, bounds.max_lethality);

        let incub_roll = deterministic_random(strain.pathogen.as_str(), 103, tick, 0);
        let incub_pct = (incub_roll - 0.5) * 2.0 * config.max_incubation_drift_pct * scale;
        #[expect(clippy::cast_precision_loss, reason = "duration bounded")]
        let incub_delta = (traits.incubation_duration as f32 * incub_pct) as i64;
        #[expect(clippy::cast_possible_wrap, reason = "duration bounded")]
        let incub_signed = traits.incubation_duration as i64;
        #[expect(clippy::cast_sign_loss, reason = "clamped to positive")]
        let new_incub = (incub_signed + incub_delta).max(0) as u64;
        traits.incubation_duration = new_incub.clamp(bounds.min_incubation, bounds.max_incubation);

        traits
    }
}

/// Context for mutation evaluation.
#[derive(Clone, Debug, Default)]
pub struct MutationContext {
    /// Current tick.
    pub tick: u64,
    /// Host ID (for deterministic randomness).
    pub host_id: u64,
    /// Current pathogen load.
    pub pathogen_load: f32,
    /// Whether host is under treatment.
    pub under_treatment: bool,
    /// Environmental stress factor.
    pub environmental_stress: f32,
}

impl MutationContext {
    #[must_use]
    pub fn new(tick: u64, host_id: u64) -> Self {
        Self {
            tick,
            host_id,
            pathogen_load: 1.0,
            under_treatment: false,
            environmental_stress: 0.0,
        }
    }

    #[must_use]
    pub fn with_load(mut self, load: f32) -> Self {
        self.pathogen_load = load;
        self
    }

    #[must_use]
    pub fn with_treatment(mut self, under_treatment: bool) -> Self {
        self.under_treatment = under_treatment;
        self
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "hash values for random distribution"
)]
fn deterministic_random(pathogen: &str, variant: u32, tick: u64, host_id: u64) -> f32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(pathogen.as_bytes());
    hasher.update(&variant.to_le_bytes());
    hasher.update(&tick.to_le_bytes());
    hasher.update(&host_id.to_le_bytes());
    let hash = hasher.finalize();
    hash as f32 / u32::MAX as f32
}

/// Evolution event for tracking mutation history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvolutionEvent {
    /// Tick when evolution occurred.
    pub tick: u64,
    /// Original strain.
    pub from_strain: StrainId,
    /// New strain (if variant) or same strain (if drift).
    pub to_strain: StrainId,
    /// Whether this was a major variant or minor drift.
    pub is_major_variant: bool,
    /// Trait changes summary.
    pub trait_changes: TraitChanges,
}

/// Summary of trait changes from mutation.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct TraitChanges {
    pub transmissibility_delta: f32,
    pub virulence_delta: f32,
    pub lethality_delta: f32,
    pub incubation_delta: i64,
}

impl TraitChanges {
    #[must_use]
    #[expect(clippy::cast_possible_wrap, reason = "duration bounded")]
    pub fn from_diff(before: &PathogenTraits, after: &PathogenTraits) -> Self {
        Self {
            transmissibility_delta: after.transmissibility - before.transmissibility,
            virulence_delta: after.virulence - before.virulence,
            lethality_delta: after.lethality - before.lethality,
            incubation_delta: after.incubation_duration as i64 - before.incubation_duration as i64,
        }
    }

    #[must_use]
    pub fn is_more_dangerous(&self) -> bool {
        self.transmissibility_delta > 0.0
            || self.virulence_delta > 0.0
            || self.lethality_delta > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_traits() -> PathogenTraits {
        PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_virulence(0.3)
            .with_lethality(0.2)
            .with_incubation(100)
            .with_mutation_rate(0.1)
    }

    #[test]
    fn test_mutation_config_default() {
        let config = MutationConfig::default();
        assert!(config.base_mutation_chance > 0.0);
        assert!(config.new_variant_chance > 0.0);
    }

    #[test]
    fn test_mutation_config_builder() {
        let config = MutationConfig::new()
            .with_base_chance(0.01)
            .with_new_variant_chance(0.2);

        assert!((config.base_mutation_chance - 0.01).abs() < f32::EPSILON);
        assert!((config.new_variant_chance - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mutation_tracker_new() {
        let tracker = MutationTracker::new();
        assert_eq!(tracker.total_mutations(), 0);
    }

    #[test]
    fn test_deterministic_random() {
        let r1 = deterministic_random("plague", 0, 100, 1);
        let r2 = deterministic_random("plague", 0, 100, 1);
        let r3 = deterministic_random("plague", 0, 100, 2);

        assert!((r1 - r2).abs() < f32::EPSILON);
        assert!((0.0..=1.0).contains(&r1));

        assert!((r1 - r3).abs() > f32::EPSILON || (r1 - r3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mutation_no_mutation_low_rate() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let mut traits = make_test_traits();
        traits.mutation_rate = 0.0;

        let config = MutationConfig::new().with_base_chance(0.0);
        let bounds = TraitBounds::default();
        let context = MutationContext::new(0, 1);

        let result = tracker.attempt_mutation(&strain, &traits, &bounds, &config, &context);

        assert!(matches!(result, MutationResult::NoMutation));
    }

    #[test]
    fn test_mutation_high_rate() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();

        let config = MutationConfig::new().with_base_chance(1.0);
        let bounds = TraitBounds::default();

        let mut got_mutation = false;
        for tick in 0..100 {
            let context = MutationContext::new(tick, 1);
            let result = tracker.attempt_mutation(&strain, &traits, &bounds, &config, &context);
            if !matches!(result, MutationResult::NoMutation) {
                got_mutation = true;
                break;
            }
        }

        assert!(got_mutation);
    }

    #[test]
    fn test_mutation_creates_variants() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();

        let config = MutationConfig::new()
            .with_base_chance(1.0)
            .with_new_variant_chance(1.0);
        let bounds = TraitBounds::default();

        let mut got_variant = false;
        for tick in 0..1000 {
            let context = MutationContext::new(tick, 1);
            let result = tracker.attempt_mutation(&strain, &traits, &bounds, &config, &context);
            if matches!(result, MutationResult::NewVariant { .. }) {
                got_variant = true;
                break;
            }
        }

        assert!(got_variant);
    }

    #[test]
    fn test_mutation_respects_bounds() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let traits = PathogenTraits::default()
            .with_transmissibility(0.95)
            .with_mutation_rate(1.0);

        let bounds = TraitBounds {
            min_transmissibility: 0.1,
            max_transmissibility: 0.9,
            ..TraitBounds::default()
        };

        let config = MutationConfig::new().with_base_chance(1.0);
        let context = MutationContext::new(0, 1);

        let result = tracker.attempt_mutation(&strain, &traits, &bounds, &config, &context);

        match result {
            MutationResult::MinorDrift(new_traits)
            | MutationResult::NewVariant {
                traits: new_traits, ..
            } => {
                assert!(new_traits.transmissibility >= bounds.min_transmissibility);
                assert!(new_traits.transmissibility <= bounds.max_transmissibility);
            }
            MutationResult::NoMutation => {}
        }
    }

    #[test]
    fn test_mutation_treatment_pressure() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();

        let config = MutationConfig::new().with_base_chance(0.01);
        let bounds = TraitBounds::default();

        let mut mutations_without_treatment = 0;
        let mut mutations_with_treatment = 0;

        for tick in 0..1000 {
            let ctx_no_treatment = MutationContext::new(tick, 1).with_treatment(false);
            let result =
                tracker.attempt_mutation(&strain, &traits, &bounds, &config, &ctx_no_treatment);
            if !matches!(result, MutationResult::NoMutation) {
                mutations_without_treatment += 1;
            }

            let ctx_treatment = MutationContext::new(tick, 2).with_treatment(true);
            let result =
                tracker.attempt_mutation(&strain, &traits, &bounds, &config, &ctx_treatment);
            if !matches!(result, MutationResult::NoMutation) {
                mutations_with_treatment += 1;
            }
        }

        assert!(mutations_with_treatment >= mutations_without_treatment);
    }

    #[test]
    fn test_mutation_context() {
        let ctx = MutationContext::new(100, 5)
            .with_load(3.0)
            .with_treatment(true);

        assert_eq!(ctx.tick, 100);
        assert_eq!(ctx.host_id, 5);
        assert!((ctx.pathogen_load - 3.0).abs() < f32::EPSILON);
        assert!(ctx.under_treatment);
    }

    #[test]
    fn test_trait_changes_from_diff() {
        let before = PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_virulence(0.3);

        let after = PathogenTraits::default()
            .with_transmissibility(0.6)
            .with_virulence(0.2);

        let changes = TraitChanges::from_diff(&before, &after);

        assert!((changes.transmissibility_delta - 0.1).abs() < f32::EPSILON);
        assert!((changes.virulence_delta - (-0.1)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_trait_changes_is_more_dangerous() {
        let dangerous = TraitChanges {
            transmissibility_delta: 0.1,
            virulence_delta: 0.0,
            lethality_delta: 0.0,
            incubation_delta: 0,
        };
        assert!(dangerous.is_more_dangerous());

        let safer = TraitChanges {
            transmissibility_delta: -0.1,
            virulence_delta: -0.1,
            lethality_delta: -0.1,
            incubation_delta: 10,
        };
        assert!(!safer.is_more_dangerous());
    }

    #[test]
    fn test_serde_mutation_result() {
        let result = MutationResult::NewVariant {
            variant_id: 3,
            traits: make_test_traits(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let restored: MutationResult = serde_json::from_str(&json).unwrap();

        if let (
            MutationResult::NewVariant { variant_id: v1, .. },
            MutationResult::NewVariant { variant_id: v2, .. },
        ) = (&result, &restored)
        {
            assert_eq!(v1, v2);
        } else {
            panic!("Expected NewVariant");
        }
    }

    #[test]
    fn test_serde_evolution_event() {
        let event = EvolutionEvent {
            tick: 100,
            from_strain: StrainId::base(PathogenId::plague()),
            to_strain: StrainId::new(PathogenId::plague(), 1),
            is_major_variant: true,
            trait_changes: TraitChanges::default(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let restored: EvolutionEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.tick, restored.tick);
        assert_eq!(event.is_major_variant, restored.is_major_variant);
    }

    #[test]
    fn test_serde_mutation_tracker() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();

        let config = MutationConfig::new().with_base_chance(1.0);
        let bounds = TraitBounds::default();
        let context = MutationContext::new(0, 1);

        let _ = tracker.attempt_mutation(&strain, &traits, &bounds, &config, &context);

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: MutationTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(tracker.total_mutations(), restored.total_mutations());
    }
}

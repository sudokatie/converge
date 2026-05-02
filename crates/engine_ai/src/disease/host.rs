//! Host infection state and immunity tracking.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::ids::{HostId, PathogenId, StrainId};
use super::pathogen::PathogenTraits;

/// Stage of infection for a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum InfectionStage {
    /// Not infected.
    #[default]
    Healthy,
    /// Recently exposed, not yet infected.
    Exposed,
    /// Infected but not showing symptoms.
    Incubating,
    /// Showing symptoms.
    Symptomatic,
    /// Recovering from infection.
    Recovering,
    /// Recovered and immune.
    Immune,
    /// Carrying pathogen but not sick (chronic carrier).
    Carrier,
    /// Latent infection (dormant, can reactivate).
    Latent,
    /// Dead from disease.
    Dead,
}

impl InfectionStage {
    /// Whether this stage can transmit disease.
    #[must_use]
    pub fn is_infectious(self) -> bool {
        matches!(
            self,
            Self::Incubating | Self::Symptomatic | Self::Carrier | Self::Latent
        )
    }

    /// Whether the host is actively sick.
    #[must_use]
    pub fn is_sick(self) -> bool {
        matches!(self, Self::Symptomatic | Self::Recovering)
    }

    /// Whether the host is alive.
    #[must_use]
    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Dead)
    }

    /// Whether the host can be infected.
    #[must_use]
    pub fn can_be_infected(self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Stage index for ordering/fingerprinting.
    #[must_use]
    pub fn as_index(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Exposed => 1,
            Self::Incubating => 2,
            Self::Symptomatic => 3,
            Self::Recovering => 4,
            Self::Immune => 5,
            Self::Carrier => 6,
            Self::Latent => 7,
            Self::Dead => 8,
        }
    }
}

/// Active infection state for a single pathogen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveInfection {
    /// The pathogen strain causing this infection.
    pub strain: StrainId,
    /// Current infection stage.
    pub stage: InfectionStage,
    /// Tick when this stage started.
    pub stage_start_tick: u64,
    /// Tick when initially exposed.
    pub exposure_tick: u64,
    /// Accumulated disease progression (0.0 to 1.0 within each stage).
    pub progression: f32,
    /// Severity level (0.0 = asymptomatic, 1.0 = critical).
    pub severity: f32,
    /// Current viral/bacterial load.
    pub pathogen_load: f32,
    /// Whether receiving treatment.
    pub under_treatment: bool,
    /// Modified traits for this specific infection (mutations applied).
    pub effective_traits: PathogenTraits,
}

impl ActiveInfection {
    #[must_use]
    pub fn new(strain: StrainId, traits: PathogenTraits, tick: u64) -> Self {
        Self {
            strain,
            stage: InfectionStage::Exposed,
            stage_start_tick: tick,
            exposure_tick: tick,
            progression: 0.0,
            severity: 0.0,
            pathogen_load: 0.1,
            under_treatment: false,
            effective_traits: traits,
        }
    }

    /// Duration in current stage.
    #[must_use]
    pub fn stage_duration(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.stage_start_tick)
    }

    /// Total infection duration.
    #[must_use]
    pub fn total_duration(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.exposure_tick)
    }

    /// Transition to a new stage.
    pub fn transition_to(&mut self, stage: InfectionStage, tick: u64) {
        self.stage = stage;
        self.stage_start_tick = tick;
        self.progression = 0.0;
    }

    /// Advance progression within current stage.
    pub fn advance_progression(&mut self, amount: f32) {
        self.progression = (self.progression + amount).clamp(0.0, 1.0);
    }

    /// Update severity.
    pub fn update_severity(&mut self, delta: f32) {
        self.severity = (self.severity + delta).clamp(0.0, 1.0);
    }

    /// Update pathogen load.
    pub fn update_load(&mut self, delta: f32) {
        self.pathogen_load = (self.pathogen_load + delta).clamp(0.0, 10.0);
    }

    /// Whether this infection is ready to progress to next stage.
    #[must_use]
    pub fn ready_to_progress(&self) -> bool {
        self.progression >= 1.0
    }

    /// Whether the infection is critical.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.severity >= 0.9
    }

    /// Compute transmission probability.
    #[must_use]
    pub fn transmission_probability(&self) -> f32 {
        if !self.stage.is_infectious() {
            return 0.0;
        }
        let base = self.effective_traits.transmissibility;
        let load_factor = (self.pathogen_load / 5.0).min(1.5);
        let stage_factor = match self.stage {
            InfectionStage::Incubating => 0.5,
            InfectionStage::Symptomatic => 1.0,
            InfectionStage::Carrier => 0.3,
            InfectionStage::Latent => 0.1,
            _ => 0.0,
        };
        (base * load_factor * stage_factor).clamp(0.0, 1.0)
    }
}

/// Immunity to a specific pathogen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImmunityRecord {
    /// Pathogen this immunity is for.
    pub pathogen_id: PathogenId,
    /// Tick when immunity was acquired.
    pub acquired_tick: u64,
    /// Tick when immunity expires (0 = permanent).
    pub expires_tick: u64,
    /// Strength of immunity (0.0 = none, 1.0 = full).
    pub strength: f32,
    /// Specific strain variants this immunity covers.
    pub covered_variants: BTreeSet<u32>,
}

impl ImmunityRecord {
    #[must_use]
    pub fn new(pathogen_id: PathogenId, tick: u64, duration: u64) -> Self {
        Self {
            pathogen_id,
            acquired_tick: tick,
            expires_tick: if duration == 0 { 0 } else { tick + duration },
            strength: 1.0,
            covered_variants: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn permanent(pathogen_id: PathogenId, tick: u64) -> Self {
        Self::new(pathogen_id, tick, 0)
    }

    #[must_use]
    pub fn is_permanent(&self) -> bool {
        self.expires_tick == 0
    }

    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        !self.is_permanent() && current_tick >= self.expires_tick
    }

    #[must_use]
    pub fn effective_strength(&self, current_tick: u64) -> f32 {
        if self.is_expired(current_tick) {
            return 0.0;
        }
        if self.is_permanent() {
            return self.strength;
        }
        let total_duration = self.expires_tick.saturating_sub(self.acquired_tick);
        let remaining = self.expires_tick.saturating_sub(current_tick);
        #[expect(clippy::cast_precision_loss, reason = "ticks bounded")]
        let waning = remaining as f32 / total_duration as f32;
        self.strength * waning
    }

    pub fn add_variant(&mut self, variant: u32) {
        self.covered_variants.insert(variant);
    }

    #[must_use]
    pub fn covers_variant(&self, variant: u32) -> bool {
        variant == 0 || self.covered_variants.contains(&variant)
    }
}

/// Per-host resistance to pathogens.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResistanceProfile {
    /// Resistance multipliers (0.0 = full resist, 1.0 = normal, >1.0 = vulnerable).
    resistances: BTreeMap<PathogenId, f32>,
    /// Natural immunities.
    natural_immunities: BTreeSet<PathogenId>,
}

impl ResistanceProfile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_resistance(&mut self, pathogen_id: PathogenId, multiplier: f32) {
        self.resistances.insert(pathogen_id, multiplier.max(0.0));
    }

    #[must_use]
    pub fn resistance(&self, pathogen_id: &PathogenId) -> f32 {
        if self.natural_immunities.contains(pathogen_id) {
            return 0.0;
        }
        self.resistances.get(pathogen_id).copied().unwrap_or(1.0)
    }

    pub fn add_natural_immunity(&mut self, pathogen_id: PathogenId) {
        self.natural_immunities.insert(pathogen_id);
    }

    #[must_use]
    pub fn has_natural_immunity(&self, pathogen_id: &PathogenId) -> bool {
        self.natural_immunities.contains(pathogen_id)
    }

    pub fn natural_immunities(&self) -> impl Iterator<Item = &PathogenId> {
        self.natural_immunities.iter()
    }
}

/// Complete infection state for a single host.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HostInfectionState {
    /// Host identifier.
    pub host_id: HostId,
    /// Species (for susceptibility checks).
    pub species: String,
    /// Active infections by pathogen ID.
    infections: BTreeMap<PathogenId, ActiveInfection>,
    /// Acquired immunities.
    immunities: BTreeMap<PathogenId, ImmunityRecord>,
    /// Resistance profile.
    resistance: ResistanceProfile,
    /// Current tick.
    current_tick: u64,
    /// Overall health modifier (1.0 = healthy).
    pub health_modifier: f32,
}

impl HostInfectionState {
    #[must_use]
    pub fn new(host_id: HostId, species: impl Into<String>) -> Self {
        Self {
            host_id,
            species: species.into(),
            infections: BTreeMap::new(),
            immunities: BTreeMap::new(),
            resistance: ResistanceProfile::new(),
            current_tick: 0,
            health_modifier: 1.0,
        }
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn resistance(&self) -> &ResistanceProfile {
        &self.resistance
    }

    pub fn resistance_mut(&mut self) -> &mut ResistanceProfile {
        &mut self.resistance
    }

    /// Check if host has any active infection.
    #[must_use]
    pub fn is_infected(&self) -> bool {
        !self.infections.is_empty()
    }

    /// Check if host is infectious.
    #[must_use]
    pub fn is_infectious(&self) -> bool {
        self.infections.values().any(|i| i.stage.is_infectious())
    }

    /// Check if host is sick.
    #[must_use]
    pub fn is_sick(&self) -> bool {
        self.infections.values().any(|i| i.stage.is_sick())
    }

    /// Check if host is dead.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.infections
            .values()
            .any(|i| i.stage == InfectionStage::Dead)
    }

    /// Get active infection for a pathogen.
    #[must_use]
    pub fn get_infection(&self, pathogen_id: &PathogenId) -> Option<&ActiveInfection> {
        self.infections.get(pathogen_id)
    }

    /// Get mutable infection.
    pub fn get_infection_mut(&mut self, pathogen_id: &PathogenId) -> Option<&mut ActiveInfection> {
        self.infections.get_mut(pathogen_id)
    }

    /// Get all active infections.
    pub fn infections(&self) -> impl Iterator<Item = &ActiveInfection> {
        self.infections.values()
    }

    /// Number of active infections.
    #[must_use]
    pub fn infection_count(&self) -> usize {
        self.infections.len()
    }

    /// Check if host has immunity to a pathogen.
    #[must_use]
    pub fn has_immunity(&self, pathogen_id: &PathogenId) -> bool {
        self.resistance.has_natural_immunity(pathogen_id)
            || self
                .immunities
                .get(pathogen_id)
                .is_some_and(|i| !i.is_expired(self.current_tick))
    }

    /// Get immunity strength for a pathogen.
    #[must_use]
    pub fn immunity_strength(&self, pathogen_id: &PathogenId) -> f32 {
        if self.resistance.has_natural_immunity(pathogen_id) {
            return 1.0;
        }
        self.immunities
            .get(pathogen_id)
            .map_or(0.0, |i| i.effective_strength(self.current_tick))
    }

    /// Get immunity record.
    #[must_use]
    pub fn get_immunity(&self, pathogen_id: &PathogenId) -> Option<&ImmunityRecord> {
        self.immunities.get(pathogen_id)
    }

    /// Check if host can be infected by a strain.
    #[must_use]
    pub fn can_be_infected(&self, strain: &StrainId) -> bool {
        if self.resistance.has_natural_immunity(&strain.pathogen)
            || self.infections.contains_key(&strain.pathogen)
        {
            return false;
        }
        let immunity_strength = self.immunity_strength(&strain.pathogen);
        if let Some(immunity) = self.immunities.get(&strain.pathogen)
            && immunity.covers_variant(strain.variant)
            && immunity_strength >= 0.9
        {
            return false;
        }
        true
    }

    /// Attempt to expose host to a strain.
    #[must_use]
    pub fn expose(&mut self, strain: StrainId, traits: PathogenTraits, tick: u64) -> bool {
        if !self.can_be_infected(&strain) {
            return false;
        }
        let infection = ActiveInfection::new(strain.clone(), traits, tick);
        self.infections.insert(strain.pathogen, infection);
        true
    }

    /// Grant immunity to a pathogen.
    pub fn grant_immunity(&mut self, pathogen_id: PathogenId, duration: u64, tick: u64) {
        let mut record = ImmunityRecord::new(pathogen_id.clone(), tick, duration);
        if let Some(existing) = self.immunities.get(&pathogen_id) {
            for variant in &existing.covered_variants {
                record.add_variant(*variant);
            }
        }
        self.immunities.insert(pathogen_id, record);
    }

    /// Remove an infection (cured or died).
    pub fn clear_infection(&mut self, pathogen_id: &PathogenId) -> Option<ActiveInfection> {
        self.infections.remove(pathogen_id)
    }

    /// Update current tick (call before processing).
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Compute overall severity from all infections.
    #[must_use]
    pub fn combined_severity(&self) -> f32 {
        let mut max_severity = 0.0f32;
        for infection in self.infections.values() {
            if infection.stage.is_sick() {
                max_severity = max_severity.max(infection.severity);
            }
        }
        max_severity
    }

    /// Get most severe infection.
    #[must_use]
    pub fn most_severe_infection(&self) -> Option<&ActiveInfection> {
        self.infections
            .values()
            .filter(|i| i.stage.is_sick())
            .max_by(|a, b| {
                a.severity
                    .partial_cmp(&b.severity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Clean up expired immunities.
    pub fn cleanup_expired_immunities(&mut self, tick: u64) {
        self.immunities
            .retain(|_, immunity| !immunity.is_expired(tick));
    }

    /// Compute a stable checksum.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.host_id.raw().to_le_bytes());
        hasher.update(self.species.as_bytes());
        hasher.update(&self.current_tick.to_le_bytes());

        for (id, infection) in &self.infections {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&[infection.stage.as_index()]);
            hasher.update(&infection.progression.to_le_bytes());
            hasher.update(&infection.severity.to_le_bytes());
        }

        for (id, immunity) in &self.immunities {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&immunity.expires_tick.to_le_bytes());
        }

        hasher.finalize()
    }
}

/// Result of processing host infection for one tick.
#[derive(Clone, Debug, Default)]
pub struct HostTickResult {
    /// Stage transitions that occurred.
    pub stage_transitions: Vec<StageTransition>,
    /// Host died from disease.
    pub died: bool,
    /// Host recovered from infections.
    pub recovered_from: Vec<PathogenId>,
    /// Immunities granted.
    pub immunities_granted: Vec<PathogenId>,
    /// Immunities expired.
    pub immunities_expired: Vec<PathogenId>,
}

/// Record of an infection stage transition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageTransition {
    pub host_id: HostId,
    pub pathogen_id: PathogenId,
    pub from: InfectionStage,
    pub to: InfectionStage,
    pub tick: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_traits() -> PathogenTraits {
        PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_incubation(100)
            .with_symptomatic_duration(200)
    }

    #[test]
    fn test_infection_stage_properties() {
        assert!(!InfectionStage::Healthy.is_infectious());
        assert!(InfectionStage::Symptomatic.is_infectious());
        assert!(InfectionStage::Carrier.is_infectious());

        assert!(!InfectionStage::Incubating.is_sick());
        assert!(InfectionStage::Symptomatic.is_sick());
        assert!(InfectionStage::Recovering.is_sick());

        assert!(InfectionStage::Healthy.is_alive());
        assert!(!InfectionStage::Dead.is_alive());
    }

    #[test]
    fn test_infection_stage_ordering() {
        assert_eq!(InfectionStage::Healthy.as_index(), 0);
        assert_eq!(InfectionStage::Exposed.as_index(), 1);
        assert_eq!(InfectionStage::Dead.as_index(), 8);
    }

    #[test]
    fn test_active_infection_new() {
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();
        let infection = ActiveInfection::new(strain.clone(), traits, 100);

        assert_eq!(infection.strain, strain);
        assert_eq!(infection.stage, InfectionStage::Exposed);
        assert_eq!(infection.exposure_tick, 100);
        assert!((infection.progression).abs() < f32::EPSILON);
    }

    #[test]
    fn test_active_infection_duration() {
        let strain = StrainId::base(PathogenId::plague());
        let infection = ActiveInfection::new(strain, make_test_traits(), 100);

        assert_eq!(infection.stage_duration(150), 50);
        assert_eq!(infection.total_duration(200), 100);
    }

    #[test]
    fn test_active_infection_transition() {
        let strain = StrainId::base(PathogenId::plague());
        let mut infection = ActiveInfection::new(strain, make_test_traits(), 100);
        infection.progression = 0.5;

        infection.transition_to(InfectionStage::Incubating, 150);

        assert_eq!(infection.stage, InfectionStage::Incubating);
        assert_eq!(infection.stage_start_tick, 150);
        assert!((infection.progression).abs() < f32::EPSILON);
    }

    #[test]
    fn test_active_infection_progression() {
        let strain = StrainId::base(PathogenId::plague());
        let mut infection = ActiveInfection::new(strain, make_test_traits(), 0);

        infection.advance_progression(0.3);
        assert!((infection.progression - 0.3).abs() < f32::EPSILON);

        infection.advance_progression(0.9);
        assert!((infection.progression - 1.0).abs() < f32::EPSILON);

        assert!(infection.ready_to_progress());
    }

    #[test]
    fn test_active_infection_severity() {
        let strain = StrainId::base(PathogenId::plague());
        let mut infection = ActiveInfection::new(strain, make_test_traits(), 0);

        infection.update_severity(0.5);
        assert!(!infection.is_critical());

        infection.update_severity(0.5);
        assert!(infection.is_critical());
    }

    #[test]
    fn test_active_infection_transmission_probability() {
        let strain = StrainId::base(PathogenId::plague());
        let mut infection = ActiveInfection::new(strain, make_test_traits(), 0);

        assert!(infection.transmission_probability().abs() < f32::EPSILON);

        infection.stage = InfectionStage::Symptomatic;
        infection.pathogen_load = 5.0;
        let prob = infection.transmission_probability();
        assert!(prob > 0.0);
        assert!(prob <= 1.0);
    }

    #[test]
    fn test_immunity_record_new() {
        let immunity = ImmunityRecord::new(PathogenId::plague(), 100, 500);

        assert_eq!(immunity.expires_tick, 600);
        assert!(!immunity.is_permanent());
        assert!(!immunity.is_expired(200));
        assert!(immunity.is_expired(700));
    }

    #[test]
    fn test_immunity_record_permanent() {
        let immunity = ImmunityRecord::permanent(PathogenId::plague(), 100);

        assert!(immunity.is_permanent());
        assert!(!immunity.is_expired(1_000_000));
    }

    #[test]
    fn test_immunity_record_waning() {
        let immunity = ImmunityRecord::new(PathogenId::plague(), 0, 1000);

        assert!((immunity.effective_strength(0) - 1.0).abs() < f32::EPSILON);
        assert!((immunity.effective_strength(500) - 0.5).abs() < f32::EPSILON);
        assert!((immunity.effective_strength(1000)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_immunity_record_variants() {
        let mut immunity = ImmunityRecord::new(PathogenId::plague(), 0, 1000);

        assert!(immunity.covers_variant(0));
        assert!(!immunity.covers_variant(3));

        immunity.add_variant(3);
        assert!(immunity.covers_variant(3));
    }

    #[test]
    fn test_resistance_profile() {
        let mut profile = ResistanceProfile::new();

        assert!((profile.resistance(&PathogenId::plague()) - 1.0).abs() < f32::EPSILON);

        profile.set_resistance(PathogenId::plague(), 0.5);
        assert!((profile.resistance(&PathogenId::plague()) - 0.5).abs() < f32::EPSILON);

        profile.add_natural_immunity(PathogenId::blight());
        assert!((profile.resistance(&PathogenId::blight())).abs() < f32::EPSILON);
        assert!(profile.has_natural_immunity(&PathogenId::blight()));
    }

    #[test]
    fn test_host_infection_state_new() {
        let state = HostInfectionState::new(HostId::new(1), "human");

        assert_eq!(state.host_id.raw(), 1);
        assert_eq!(state.species, "human");
        assert!(!state.is_infected());
        assert!(!state.is_dead());
    }

    #[test]
    fn test_host_infection_state_expose() {
        let mut state = HostInfectionState::new(HostId::new(1), "human");
        let strain = StrainId::base(PathogenId::plague());

        let exposed = state.expose(strain.clone(), make_test_traits(), 100);

        assert!(exposed);
        assert!(state.get_infection(&PathogenId::plague()).is_some());
        assert_eq!(state.infection_count(), 1);
    }

    #[test]
    fn test_host_infection_state_immunity_blocks() {
        let mut state = HostInfectionState::new(HostId::new(1), "human");
        state.grant_immunity(PathogenId::plague(), 1000, 0);
        state.set_tick(100);

        assert!(state.has_immunity(&PathogenId::plague()));

        let strain = StrainId::base(PathogenId::plague());
        assert!(!state.can_be_infected(&strain));

        let exposed = state.expose(strain, make_test_traits(), 100);
        assert!(!exposed);
    }

    #[test]
    fn test_host_infection_state_natural_immunity() {
        let mut state = HostInfectionState::new(HostId::new(1), "undead");
        state
            .resistance_mut()
            .add_natural_immunity(PathogenId::plague());

        let strain = StrainId::base(PathogenId::plague());
        assert!(!state.can_be_infected(&strain));
    }

    #[test]
    fn test_host_infection_state_combined_severity() {
        let mut state = HostInfectionState::new(HostId::new(1), "human");

        let _ = state.expose(StrainId::base(PathogenId::plague()), make_test_traits(), 0);
        let _ = state.expose(StrainId::base(PathogenId::fever()), make_test_traits(), 0);

        if let Some(infection) = state.get_infection_mut(&PathogenId::plague()) {
            infection.stage = InfectionStage::Symptomatic;
            infection.severity = 0.5;
        }
        if let Some(infection) = state.get_infection_mut(&PathogenId::fever()) {
            infection.stage = InfectionStage::Symptomatic;
            infection.severity = 0.3;
        }

        assert!((state.combined_severity() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_host_infection_state_cleanup_expired() {
        let mut state = HostInfectionState::new(HostId::new(1), "human");
        state.grant_immunity(PathogenId::plague(), 100, 0);
        state.grant_immunity(PathogenId::fever(), 500, 0);

        state.cleanup_expired_immunities(200);

        assert!(!state.has_immunity(&PathogenId::plague()));
        assert!(state.immunities.contains_key(&PathogenId::fever()));
    }

    #[test]
    fn test_host_infection_state_checksum_deterministic() {
        let mut state1 = HostInfectionState::new(HostId::new(1), "human");
        let mut state2 = HostInfectionState::new(HostId::new(1), "human");

        let _ = state1.expose(StrainId::base(PathogenId::plague()), make_test_traits(), 0);
        let _ = state2.expose(StrainId::base(PathogenId::plague()), make_test_traits(), 0);

        assert_eq!(state1.checksum(), state2.checksum());
    }

    #[test]
    fn test_serde_infection_stage() {
        let stage = InfectionStage::Symptomatic;
        let json = serde_json::to_string(&stage).unwrap();
        let restored: InfectionStage = serde_json::from_str(&json).unwrap();
        assert_eq!(stage, restored);
    }

    #[test]
    fn test_serde_active_infection() {
        let strain = StrainId::base(PathogenId::plague());
        let mut infection = ActiveInfection::new(strain, make_test_traits(), 100);
        infection.severity = 0.5;

        let json = serde_json::to_string(&infection).unwrap();
        let restored: ActiveInfection = serde_json::from_str(&json).unwrap();

        assert_eq!(infection.strain, restored.strain);
        assert!((infection.severity - restored.severity).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_immunity_record() {
        let mut immunity = ImmunityRecord::new(PathogenId::plague(), 100, 500);
        immunity.add_variant(3);

        let json = serde_json::to_string(&immunity).unwrap();
        let restored: ImmunityRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(immunity.pathogen_id, restored.pathogen_id);
        assert!(restored.covers_variant(3));
    }

    #[test]
    fn test_serde_host_infection_state() {
        let mut state = HostInfectionState::new(HostId::new(1), "human");
        let _ = state.expose(StrainId::base(PathogenId::plague()), make_test_traits(), 0);
        state.grant_immunity(PathogenId::fever(), 1000, 0);

        let json = serde_json::to_string(&state).unwrap();
        let restored: HostInfectionState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.host_id, restored.host_id);
        assert!(restored.get_infection(&PathogenId::plague()).is_some());
        assert!(restored.immunities.contains_key(&PathogenId::fever()));
    }
}

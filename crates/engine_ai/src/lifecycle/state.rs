//! Lifecycle state types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a lifecycle entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LifecycleId(pub u64);

impl LifecycleId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for LifecycleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lifecycle:{}", self.0)
    }
}

/// Growth phases for living creatures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GrowthPhase {
    Juvenile,
    Adult,
    Elder,
}

impl GrowthPhase {
    #[must_use]
    pub fn is_mature(&self) -> bool {
        matches!(self, Self::Adult | Self::Elder)
    }

    #[must_use]
    pub fn can_reproduce(&self) -> bool {
        matches!(self, Self::Adult)
    }

    #[must_use]
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Juvenile => Some(Self::Adult),
            Self::Adult => Some(Self::Elder),
            Self::Elder => None,
        }
    }
}

impl fmt::Display for GrowthPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Juvenile => write!(f, "juvenile"),
            Self::Adult => write!(f, "adult"),
            Self::Elder => write!(f, "elder"),
        }
    }
}

/// State of an egg during incubation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EggState {
    /// Tick when the egg was laid.
    pub laid_tick: u64,
    /// Development progress (0.0 to 1.0).
    pub development: f32,
    /// Current viability (0.0 to 1.0).
    pub viability: f32,
    /// Temperature modifier affecting development.
    pub temperature_modifier: f32,
    /// Whether hatching has begun.
    pub hatching_started: bool,
    /// Tick when hatching started (if applicable).
    pub hatching_start_tick: Option<u64>,
}

impl EggState {
    #[must_use]
    pub fn new(laid_tick: u64) -> Self {
        Self {
            laid_tick,
            development: 0.0,
            viability: 1.0,
            temperature_modifier: 1.0,
            hatching_started: false,
            hatching_start_tick: None,
        }
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.laid_tick)
    }

    #[must_use]
    pub fn is_viable(&self) -> bool {
        self.viability > 0.0
    }

    #[must_use]
    pub fn is_ready_to_hatch(&self) -> bool {
        self.development >= 1.0 && self.is_viable()
    }

    pub fn advance_development(&mut self, amount: f32) {
        self.development = (self.development + amount).clamp(0.0, 1.0);
    }

    pub fn apply_viability_damage(&mut self, amount: f32) {
        self.viability = (self.viability - amount).max(0.0);
    }

    pub fn set_temperature_modifier(&mut self, modifier: f32) {
        self.temperature_modifier = modifier.clamp(0.1, 3.0);
    }

    pub fn start_hatching(&mut self, tick: u64) {
        if !self.hatching_started {
            self.hatching_started = true;
            self.hatching_start_tick = Some(tick);
        }
    }
}

/// State of a living creature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LivingState {
    /// Tick when creature was born/hatched.
    pub birth_tick: u64,
    /// Current growth phase.
    pub phase: GrowthPhase,
    /// Tick when current phase started.
    pub phase_start_tick: u64,
    /// Current health (0.0 to 1.0).
    pub health: f32,
    /// Current size multiplier.
    pub size: f32,
    /// Growth progress within current phase (0.0 to 1.0).
    pub growth_progress: f32,
}

impl LivingState {
    #[must_use]
    pub fn new(birth_tick: u64, phase: GrowthPhase, health: f32) -> Self {
        Self {
            birth_tick,
            phase,
            phase_start_tick: birth_tick,
            health: health.clamp(0.0, 1.0),
            size: match phase {
                GrowthPhase::Juvenile => 0.5,
                GrowthPhase::Adult => 1.0,
                GrowthPhase::Elder => 0.95,
            },
            growth_progress: 0.0,
        }
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.birth_tick)
    }

    #[must_use]
    pub fn phase_age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.phase_start_tick)
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    #[must_use]
    pub fn is_mature(&self) -> bool {
        self.phase.is_mature()
    }

    pub fn advance_growth(&mut self, amount: f32) {
        self.growth_progress = (self.growth_progress + amount).clamp(0.0, 1.0);
    }

    pub fn transition_to_phase(&mut self, phase: GrowthPhase, tick: u64, size: f32) {
        self.phase = phase;
        self.phase_start_tick = tick;
        self.size = size;
        self.growth_progress = 0.0;
    }

    pub fn apply_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(1.0);
    }
}

/// State during metamorphosis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetamorphosisState {
    /// Tick when metamorphosis started.
    pub start_tick: u64,
    /// Progress through metamorphosis (0.0 to 1.0).
    pub progress: f32,
    /// Health during transformation.
    pub health: f32,
    /// Previous state before metamorphosis.
    pub previous_phase: GrowthPhase,
    /// Target phase after metamorphosis.
    pub target_phase: GrowthPhase,
}

impl MetamorphosisState {
    #[must_use]
    pub fn new(
        start_tick: u64,
        health: f32,
        previous_phase: GrowthPhase,
        target_phase: GrowthPhase,
    ) -> Self {
        Self {
            start_tick,
            progress: 0.0,
            health: health.clamp(0.0, 1.0),
            previous_phase,
            target_phase,
        }
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.start_tick)
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn advance_progress(&mut self, amount: f32) {
        self.progress = (self.progress + amount).clamp(0.0, 1.0);
    }

    pub fn apply_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
    }
}

/// State of a corpse undergoing decay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorpseState {
    /// Tick when creature died.
    pub death_tick: u64,
    /// Remaining biomass (0.0 to initial value).
    pub remaining_biomass: f32,
    /// Initial biomass at death.
    pub initial_biomass: f32,
    /// Decay progress (0.0 to 1.0).
    pub decay_progress: f32,
    /// Environmental decay modifier.
    pub decay_modifier: f32,
}

impl CorpseState {
    #[must_use]
    pub fn new(death_tick: u64, initial_biomass: f32) -> Self {
        Self {
            death_tick,
            remaining_biomass: initial_biomass,
            initial_biomass,
            decay_progress: 0.0,
            decay_modifier: 1.0,
        }
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.death_tick)
    }

    #[must_use]
    pub fn biomass_fraction(&self) -> f32 {
        if self.initial_biomass > 0.0 {
            self.remaining_biomass / self.initial_biomass
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn is_fully_decayed(&self) -> bool {
        self.decay_progress >= 1.0 || self.remaining_biomass <= 0.0
    }

    pub fn decay(&mut self, biomass_amount: f32, progress_amount: f32) {
        self.remaining_biomass = (self.remaining_biomass - biomass_amount).max(0.0);
        self.decay_progress = (self.decay_progress + progress_amount).clamp(0.0, 1.0);
    }

    pub fn set_decay_modifier(&mut self, modifier: f32) {
        self.decay_modifier = modifier.clamp(0.1, 5.0);
    }
}

/// The current lifecycle stage of a creature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LifecycleStage {
    /// Egg awaiting hatching.
    Egg(EggState),
    /// Living creature.
    Living(LivingState),
    /// Undergoing metamorphosis.
    Metamorphosis(MetamorphosisState),
    /// Dead, undergoing decay.
    Corpse(CorpseState),
}

impl LifecycleStage {
    #[must_use]
    pub fn stage_name(&self) -> &'static str {
        match self {
            Self::Egg(_) => "egg",
            Self::Living(_) => "living",
            Self::Metamorphosis(_) => "metamorphosis",
            Self::Corpse(_) => "corpse",
        }
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        matches!(self, Self::Living(_) | Self::Metamorphosis(_))
    }

    #[must_use]
    pub fn is_egg(&self) -> bool {
        matches!(self, Self::Egg(_))
    }

    #[must_use]
    pub fn is_corpse(&self) -> bool {
        matches!(self, Self::Corpse(_))
    }

    #[must_use]
    pub fn health(&self) -> Option<f32> {
        match self {
            Self::Living(state) => Some(state.health),
            Self::Metamorphosis(state) => Some(state.health),
            Self::Egg(state) => Some(state.viability),
            Self::Corpse(_) => None,
        }
    }

    #[must_use]
    pub fn as_egg(&self) -> Option<&EggState> {
        match self {
            Self::Egg(state) => Some(state),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_living(&self) -> Option<&LivingState> {
        match self {
            Self::Living(state) => Some(state),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_metamorphosis(&self) -> Option<&MetamorphosisState> {
        match self {
            Self::Metamorphosis(state) => Some(state),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_corpse(&self) -> Option<&CorpseState> {
        match self {
            Self::Corpse(state) => Some(state),
            _ => None,
        }
    }
}

impl fmt::Display for LifecycleStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Egg(state) => write!(f, "egg (dev: {:.0}%)", state.development * 100.0),
            Self::Living(state) => write!(f, "{} (hp: {:.0}%)", state.phase, state.health * 100.0),
            Self::Metamorphosis(state) => {
                write!(f, "metamorphosis (prog: {:.0}%)", state.progress * 100.0)
            }
            Self::Corpse(state) => {
                write!(f, "corpse (decay: {:.0}%)", state.decay_progress * 100.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_id() {
        let id = LifecycleId::new(42);
        assert_eq!(id.raw(), 42);
        assert_eq!(format!("{id}"), "lifecycle:42");
    }

    #[test]
    fn test_growth_phase_progression() {
        assert_eq!(GrowthPhase::Juvenile.next(), Some(GrowthPhase::Adult));
        assert_eq!(GrowthPhase::Adult.next(), Some(GrowthPhase::Elder));
        assert_eq!(GrowthPhase::Elder.next(), None);
    }

    #[test]
    fn test_growth_phase_maturity() {
        assert!(!GrowthPhase::Juvenile.is_mature());
        assert!(GrowthPhase::Adult.is_mature());
        assert!(GrowthPhase::Elder.is_mature());
    }

    #[test]
    fn test_growth_phase_reproduction() {
        assert!(!GrowthPhase::Juvenile.can_reproduce());
        assert!(GrowthPhase::Adult.can_reproduce());
        assert!(!GrowthPhase::Elder.can_reproduce());
    }

    #[test]
    fn test_egg_state() {
        let mut egg = EggState::new(0);
        assert_eq!(egg.age(100), 100);
        assert!(egg.is_viable());
        assert!(!egg.is_ready_to_hatch());

        egg.advance_development(1.0);
        assert!(egg.is_ready_to_hatch());
    }

    #[test]
    fn test_egg_hatching() {
        let mut egg = EggState::new(0);
        egg.start_hatching(100);

        assert!(egg.hatching_started);
        assert_eq!(egg.hatching_start_tick, Some(100));
    }

    #[test]
    fn test_egg_viability() {
        let mut egg = EggState::new(0);
        egg.apply_viability_damage(0.5);
        assert!((egg.viability - 0.5).abs() < f32::EPSILON);

        egg.apply_viability_damage(0.6);
        assert!(egg.viability.abs() < f32::EPSILON);
        assert!(!egg.is_viable());
    }

    #[test]
    fn test_living_state() {
        let mut living = LivingState::new(0, GrowthPhase::Juvenile, 1.0);
        assert_eq!(living.age(500), 500);
        assert!(living.is_alive());
        assert!(!living.is_mature());

        living.transition_to_phase(GrowthPhase::Adult, 500, 1.0);
        assert!(living.is_mature());
        assert_eq!(living.phase_age(600), 100);
    }

    #[test]
    fn test_living_damage_heal() {
        let mut living = LivingState::new(0, GrowthPhase::Adult, 1.0);
        living.apply_damage(0.3);
        assert!((living.health - 0.7).abs() < f32::EPSILON);

        living.heal(0.2);
        assert!((living.health - 0.9).abs() < f32::EPSILON);

        living.heal(0.5);
        assert!((living.health - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_metamorphosis_state() {
        let mut meta = MetamorphosisState::new(0, 1.0, GrowthPhase::Juvenile, GrowthPhase::Adult);
        assert!(!meta.is_complete());
        assert!(meta.is_alive());

        meta.advance_progress(1.0);
        assert!(meta.is_complete());
    }

    #[test]
    fn test_corpse_state() {
        let mut corpse = CorpseState::new(0, 100.0);
        assert_eq!(corpse.age(50), 50);
        assert!((corpse.biomass_fraction() - 1.0).abs() < f32::EPSILON);
        assert!(!corpse.is_fully_decayed());

        corpse.decay(50.0, 0.5);
        assert!((corpse.biomass_fraction() - 0.5).abs() < f32::EPSILON);

        corpse.decay(50.0, 0.5);
        assert!(corpse.is_fully_decayed());
    }

    #[test]
    fn test_lifecycle_stage_enum() {
        let egg = LifecycleStage::Egg(EggState::new(0));
        assert!(egg.is_egg());
        assert!(!egg.is_alive());
        assert_eq!(egg.stage_name(), "egg");

        let living = LifecycleStage::Living(LivingState::new(0, GrowthPhase::Adult, 1.0));
        assert!(living.is_alive());
        assert_eq!(living.health(), Some(1.0));

        let corpse = LifecycleStage::Corpse(CorpseState::new(0, 50.0));
        assert!(corpse.is_corpse());
        assert!(corpse.health().is_none());
    }

    #[test]
    fn test_lifecycle_stage_display() {
        let egg = LifecycleStage::Egg(EggState::new(0));
        assert!(format!("{egg}").contains("egg"));

        let living = LifecycleStage::Living(LivingState::new(0, GrowthPhase::Adult, 0.75));
        assert!(format!("{living}").contains("adult"));
    }

    #[test]
    fn test_growth_phase_serde() {
        let phase = GrowthPhase::Adult;
        let json = serde_json::to_string(&phase).unwrap();
        let restored: GrowthPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, phase);
    }

    #[test]
    fn test_egg_state_serde() {
        let egg = EggState::new(100);
        let json = serde_json::to_string(&egg).unwrap();
        let restored: EggState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.laid_tick, 100);
    }

    #[test]
    fn test_living_state_serde() {
        let living = LivingState::new(0, GrowthPhase::Juvenile, 0.9);
        let json = serde_json::to_string(&living).unwrap();
        let restored: LivingState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.phase, GrowthPhase::Juvenile);
    }

    #[test]
    fn test_lifecycle_stage_serde() {
        let stage = LifecycleStage::Living(LivingState::new(0, GrowthPhase::Adult, 1.0));
        let json = serde_json::to_string(&stage).unwrap();
        let restored: LifecycleStage = serde_json::from_str(&json).unwrap();
        assert!(restored.is_alive());
    }
}

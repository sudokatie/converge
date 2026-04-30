//! Lifecycle tracker for managing creature lifecycles.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::config::{LifecycleConfig, MetamorphosisTrigger};
use super::events::{LifecycleEvent, LifecycleEventKind};
use super::state::{
    CorpseState, EggState, GrowthPhase, LifecycleId, LifecycleStage, LivingState,
    MetamorphosisState,
};

/// Result of a lifecycle simulation tick.
#[derive(Clone, Debug, Default)]
pub struct LifecycleTickResult {
    pub events: Vec<LifecycleEvent>,
    pub eggs_hatched: u32,
    pub creatures_died: u32,
    pub corpses_decayed: u32,
    pub phase_transitions: u32,
    pub metamorphoses_started: u32,
    pub metamorphoses_completed: u32,
}

/// Tracker for creature lifecycle state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LifecycleTracker {
    config: LifecycleConfig,
    entities: BTreeMap<LifecycleId, LifecycleStage>,
    current_tick: u64,
}

impl LifecycleTracker {
    #[must_use]
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            config,
            entities: BTreeMap::new(),
            current_tick: 0,
        }
    }

    #[must_use]
    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    #[must_use]
    pub fn egg_count(&self) -> usize {
        self.entities
            .values()
            .filter(|s| matches!(s, LifecycleStage::Egg(_)))
            .count()
    }

    #[must_use]
    pub fn living_count(&self) -> usize {
        self.entities
            .values()
            .filter(|s| matches!(s, LifecycleStage::Living(_)))
            .count()
    }

    #[must_use]
    pub fn metamorphosis_count(&self) -> usize {
        self.entities
            .values()
            .filter(|s| matches!(s, LifecycleStage::Metamorphosis(_)))
            .count()
    }

    #[must_use]
    pub fn corpse_count(&self) -> usize {
        self.entities
            .values()
            .filter(|s| matches!(s, LifecycleStage::Corpse(_)))
            .count()
    }

    #[must_use]
    pub fn get_stage(&self, id: LifecycleId) -> Option<&LifecycleStage> {
        self.entities.get(&id)
    }

    pub fn all_ids(&self) -> impl Iterator<Item = LifecycleId> + '_ {
        self.entities.keys().copied()
    }

    pub fn spawn_egg(&mut self, id: LifecycleId, tick: u64) {
        let egg = EggState::new(tick);
        self.entities.insert(id, LifecycleStage::Egg(egg));
    }

    pub fn spawn_living(&mut self, id: LifecycleId, phase: GrowthPhase, tick: u64) {
        let living = LivingState::new(tick, phase, self.config.hatching.initial_health);
        self.entities.insert(id, LifecycleStage::Living(living));
    }

    pub fn spawn_corpse(&mut self, id: LifecycleId, biomass: f32, tick: u64) {
        let corpse = CorpseState::new(tick, biomass);
        self.entities.insert(id, LifecycleStage::Corpse(corpse));
    }

    pub fn remove(&mut self, id: LifecycleId) -> Option<LifecycleStage> {
        self.entities.remove(&id)
    }

    pub fn tick(&mut self, tick: u64) -> LifecycleTickResult {
        self.current_tick = tick;
        let mut result = LifecycleTickResult::default();

        let ids: Vec<LifecycleId> = self.entities.keys().copied().collect();

        for id in ids {
            if let Some(stage) = self.entities.remove(&id) {
                let (new_stage, events) = self.process_stage(id, stage, tick);

                for event in &events {
                    match &event.kind {
                        LifecycleEventKind::Hatched { .. } => result.eggs_hatched += 1,
                        LifecycleEventKind::NaturalDeath { .. }
                        | LifecycleEventKind::HatchingFailed { .. }
                        | LifecycleEventKind::MetamorphosisFailed { .. } => {
                            result.creatures_died += 1;
                        }
                        LifecycleEventKind::CorpseDecayed { .. } => result.corpses_decayed += 1,
                        LifecycleEventKind::PhaseTransition { .. } => result.phase_transitions += 1,
                        LifecycleEventKind::MetamorphosisStarted { .. } => {
                            result.metamorphoses_started += 1;
                        }
                        LifecycleEventKind::MetamorphosisCompleted { .. } => {
                            result.metamorphoses_completed += 1;
                        }
                        _ => {}
                    }
                }

                result.events.extend(events);

                if let Some(stage) = new_stage {
                    self.entities.insert(id, stage);
                }
            }
        }

        result
    }

    fn process_stage(
        &self,
        id: LifecycleId,
        stage: LifecycleStage,
        tick: u64,
    ) -> (Option<LifecycleStage>, Vec<LifecycleEvent>) {
        match stage {
            LifecycleStage::Egg(egg) => self.process_egg(id, egg, tick),
            LifecycleStage::Living(living) => self.process_living(id, living, tick),
            LifecycleStage::Metamorphosis(meta) => self.process_metamorphosis(id, meta, tick),
            LifecycleStage::Corpse(corpse) => self.process_corpse(id, corpse, tick),
        }
    }

    fn process_egg(
        &self,
        id: LifecycleId,
        mut egg: EggState,
        tick: u64,
    ) -> (Option<LifecycleStage>, Vec<LifecycleEvent>) {
        let mut events = Vec::new();

        if !egg.is_viable() {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::HatchingFailed { id },
            ));
            let corpse = CorpseState::new(tick, 1.0);
            return (Some(LifecycleStage::Corpse(corpse)), events);
        }

        #[expect(clippy::cast_precision_loss, reason = "duration bounded")]
        let development_rate = 1.0 / self.config.incubation.base_duration as f32;
        egg.advance_development(development_rate * egg.temperature_modifier);

        if egg.is_ready_to_hatch() && !egg.hatching_started {
            egg.start_hatching(tick);
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::HatchingStarted { id },
            ));
        }

        if egg.hatching_started {
            let hatching_start = egg.hatching_start_tick.unwrap_or(tick);
            let hatching_duration = tick.saturating_sub(hatching_start);

            if hatching_duration >= self.config.hatching.hatching_duration {
                let survival_roll = deterministic_random(id.raw(), tick);
                if survival_roll < self.config.incubation.survival_chance {
                    let initial_phase = self.config.hatching.initial_phase;
                    events.push(LifecycleEvent::new(
                        tick,
                        LifecycleEventKind::Hatched { id, initial_phase },
                    ));
                    let living =
                        LivingState::new(tick, initial_phase, self.config.hatching.initial_health);
                    return (Some(LifecycleStage::Living(living)), events);
                }
                events.push(LifecycleEvent::new(
                    tick,
                    LifecycleEventKind::HatchingFailed { id },
                ));
                let corpse = CorpseState::new(tick, 1.0);
                return (Some(LifecycleStage::Corpse(corpse)), events);
            }
        }

        let egg_age = egg.age(tick);
        if egg_age > self.config.incubation.max_duration {
            egg.apply_viability_damage(0.1);
        }

        (Some(LifecycleStage::Egg(egg)), events)
    }

    fn process_living(
        &self,
        id: LifecycleId,
        mut living: LivingState,
        tick: u64,
    ) -> (Option<LifecycleStage>, Vec<LifecycleEvent>) {
        let mut events = Vec::new();

        if !living.is_alive() {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::NaturalDeath {
                    id,
                    age: living.age(tick),
                },
            ));
            let corpse = CorpseState::new(tick, living.size * 100.0);
            return (Some(LifecycleStage::Corpse(corpse)), events);
        }

        let age = living.age(tick);
        if let Some(max_lifespan) = self.config.aging.max_lifespan
            && age >= max_lifespan
        {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::NaturalDeath { id, age },
            ));
            let corpse = CorpseState::new(tick, living.size * 100.0);
            return (Some(LifecycleStage::Corpse(corpse)), events);
        }

        if age >= self.config.aging.elder_age && living.phase != GrowthPhase::Elder {
            living.apply_damage(self.config.aging.elder_decay_rate);

            let death_roll = deterministic_random(id.raw(), tick);
            if death_roll < self.config.aging.elder_death_chance {
                events.push(LifecycleEvent::new(
                    tick,
                    LifecycleEventKind::NaturalDeath { id, age },
                ));
                let corpse = CorpseState::new(tick, living.size * 100.0);
                return (Some(LifecycleStage::Corpse(corpse)), events);
            }
        }

        if let Some(ref meta_config) = self.config.metamorphosis {
            let should_metamorphose = match &meta_config.trigger {
                MetamorphosisTrigger::Age(trigger_age) => age >= *trigger_age,
                MetamorphosisTrigger::GrowthPhase(trigger_phase) => living.phase == *trigger_phase,
                MetamorphosisTrigger::HealthThreshold(threshold) => living.health <= *threshold,
                MetamorphosisTrigger::External => false,
            };

            if should_metamorphose {
                events.push(LifecycleEvent::new(
                    tick,
                    LifecycleEventKind::MetamorphosisStarted {
                        id,
                        from_phase: living.phase,
                    },
                ));
                let meta = MetamorphosisState::new(
                    tick,
                    living.health,
                    living.phase,
                    meta_config.result_growth_phase,
                );
                return (Some(LifecycleStage::Metamorphosis(meta)), events);
            }
        }

        let phase_duration = match living.phase {
            GrowthPhase::Juvenile => self.config.growth.juvenile_duration,
            GrowthPhase::Adult => self.config.growth.adult_duration,
            GrowthPhase::Elder => u64::MAX,
        };

        #[expect(clippy::cast_precision_loss, reason = "phase duration bounded")]
        let growth_rate = self.config.growth.growth_rate / phase_duration as f32;
        living.advance_growth(growth_rate);

        if living.growth_progress >= 1.0
            && let Some(next_phase) = living.phase.next()
        {
            let from = living.phase;
            let size = match next_phase {
                GrowthPhase::Juvenile => self.config.growth.juvenile_size,
                GrowthPhase::Adult => self.config.growth.adult_size,
                GrowthPhase::Elder => self.config.growth.elder_size,
            };
            living.transition_to_phase(next_phase, tick, size);
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::PhaseTransition {
                    id,
                    from,
                    to: next_phase,
                },
            ));
        }

        (Some(LifecycleStage::Living(living)), events)
    }

    fn process_metamorphosis(
        &self,
        id: LifecycleId,
        mut meta: MetamorphosisState,
        tick: u64,
    ) -> (Option<LifecycleStage>, Vec<LifecycleEvent>) {
        let mut events = Vec::new();

        let Some(ref meta_config) = self.config.metamorphosis else {
            let living = LivingState::new(tick, meta.target_phase, meta.health);
            return (Some(LifecycleStage::Living(living)), events);
        };

        if !meta.is_alive() {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::MetamorphosisFailed { id },
            ));
            let corpse = CorpseState::new(tick, 50.0);
            return (Some(LifecycleStage::Corpse(corpse)), events);
        }

        #[expect(clippy::cast_precision_loss, reason = "duration bounded")]
        let progress_rate = 1.0 / meta_config.duration as f32;
        meta.advance_progress(progress_rate);

        if meta.is_complete() {
            let survival_roll = deterministic_random(id.raw(), tick);
            if survival_roll < meta_config.survival_chance {
                events.push(LifecycleEvent::new(
                    tick,
                    LifecycleEventKind::MetamorphosisCompleted {
                        id,
                        result_phase: meta.target_phase,
                    },
                ));
                let living = LivingState::new(tick, meta.target_phase, meta.health);
                return (Some(LifecycleStage::Living(living)), events);
            }
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::MetamorphosisFailed { id },
            ));
            let corpse = CorpseState::new(tick, 50.0);
            return (Some(LifecycleStage::Corpse(corpse)), events);
        }

        (Some(LifecycleStage::Metamorphosis(meta)), events)
    }

    fn process_corpse(
        &self,
        id: LifecycleId,
        mut corpse: CorpseState,
        tick: u64,
    ) -> (Option<LifecycleStage>, Vec<LifecycleEvent>) {
        let mut events = Vec::new();

        if corpse.is_fully_decayed() {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::CorpseDecayed { id },
            ));
            return (None, events);
        }

        let decay_rate = self.config.decay.decay_rate * corpse.decay_modifier;
        let biomass_release =
            self.config.decay.biomass_release_rate * decay_rate * corpse.initial_biomass;
        #[expect(clippy::cast_precision_loss, reason = "duration bounded")]
        let progress_rate = decay_rate / self.config.decay.full_decay_duration as f32;

        corpse.decay(biomass_release, progress_rate);

        if biomass_release > 0.01 {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::BiomassReleased {
                    id,
                    amount: biomass_release,
                    remaining: corpse.remaining_biomass,
                },
            ));
        }

        if corpse.remaining_biomass < self.config.decay.min_biomass {
            events.push(LifecycleEvent::new(
                tick,
                LifecycleEventKind::CorpseDecayed { id },
            ));
            return (None, events);
        }

        (Some(LifecycleStage::Corpse(corpse)), events)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "hash values for random distribution"
)]
fn deterministic_random(id: u64, tick: u64) -> f32 {
    let hash = id.wrapping_mul(0x517c_c1b7_2722_0a95) ^ tick.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mixed = hash ^ (hash >> 33);
    let final_hash = mixed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    (final_hash & 0xFFFF_FFFF) as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_new() {
        let tracker = LifecycleTracker::new(LifecycleConfig::standard());
        assert_eq!(tracker.entity_count(), 0);
        assert_eq!(tracker.current_tick(), 0);
    }

    #[test]
    fn test_spawn_egg() {
        let mut tracker = LifecycleTracker::new(LifecycleConfig::standard());
        tracker.spawn_egg(LifecycleId::new(1), 0);

        assert_eq!(tracker.entity_count(), 1);
        assert_eq!(tracker.egg_count(), 1);
        assert!(tracker.get_stage(LifecycleId::new(1)).is_some());
    }

    #[test]
    fn test_spawn_living() {
        let mut tracker = LifecycleTracker::new(LifecycleConfig::standard());
        tracker.spawn_living(LifecycleId::new(1), GrowthPhase::Adult, 0);

        assert_eq!(tracker.living_count(), 1);
        if let Some(LifecycleStage::Living(state)) = tracker.get_stage(LifecycleId::new(1)) {
            assert_eq!(state.phase, GrowthPhase::Adult);
        } else {
            panic!("Expected living stage");
        }
    }

    #[test]
    fn test_spawn_corpse() {
        let mut tracker = LifecycleTracker::new(LifecycleConfig::standard());
        tracker.spawn_corpse(LifecycleId::new(1), 100.0, 0);

        assert_eq!(tracker.corpse_count(), 1);
    }

    #[test]
    fn test_remove() {
        let mut tracker = LifecycleTracker::new(LifecycleConfig::standard());
        tracker.spawn_egg(LifecycleId::new(1), 0);

        let removed = tracker.remove(LifecycleId::new(1));
        assert!(removed.is_some());
        assert_eq!(tracker.entity_count(), 0);
    }

    #[test]
    fn test_tick_advances() {
        let mut tracker = LifecycleTracker::new(LifecycleConfig::standard());
        tracker.spawn_egg(LifecycleId::new(1), 0);

        let _ = tracker.tick(1);
        assert_eq!(tracker.current_tick(), 1);
    }

    #[test]
    fn test_egg_hatching_lifecycle() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config.clone());

        tracker.spawn_egg(LifecycleId::new(1), 0);

        for tick in 1..=(config.incubation.base_duration + config.hatching.hatching_duration + 10) {
            let _ = tracker.tick(tick);
        }

        let stage = tracker.get_stage(LifecycleId::new(1));
        assert!(
            matches!(stage, Some(LifecycleStage::Living(_)))
                || matches!(stage, Some(LifecycleStage::Corpse(_)))
        );
    }

    #[test]
    fn test_corpse_full_decay_lifecycle() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config.clone());

        tracker.spawn_corpse(LifecycleId::new(1), 10.0, 0);

        for tick in 1..=(config.decay.full_decay_duration + 50) {
            let _ = tracker.tick(tick);
        }

        assert!(tracker.get_stage(LifecycleId::new(1)).is_none());
    }

    #[test]
    fn test_tick_result_events() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config.clone());

        tracker.spawn_egg(LifecycleId::new(1), 0);

        let mut found_hatching_started = false;
        for tick in 1..=(config.incubation.base_duration + 5) {
            let result = tracker.tick(tick);
            for event in result.events {
                if matches!(event.kind, LifecycleEventKind::HatchingStarted { .. }) {
                    found_hatching_started = true;
                }
            }
        }

        assert!(found_hatching_started);
    }

    #[test]
    fn test_all_ids() {
        let mut tracker = LifecycleTracker::new(LifecycleConfig::standard());

        tracker.spawn_egg(LifecycleId::new(1), 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);
        tracker.spawn_corpse(LifecycleId::new(3), 50.0, 0);

        let ids: Vec<_> = tracker.all_ids().collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_deterministic_random() {
        let r1 = deterministic_random(1, 100);
        let r2 = deterministic_random(1, 100);
        assert!((r1 - r2).abs() < f32::EPSILON);

        let r3 = deterministic_random(2, 100);
        assert!((r1 - r3).abs() > f32::EPSILON || (r1 - r3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tracker_serde() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_egg(LifecycleId::new(1), 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: LifecycleTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.entity_count(), 2);
    }
}

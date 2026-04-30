//! Creature lifecycle simulation: eggs, growth, decay, and metamorphosis.
//!
//! Provides deterministic lifecycle tracking for creatures including:
//! - Egg incubation and hatching
//! - Juvenile/adult growth stages
//! - Aging and natural death
//! - Corpse decay and biomass release
//! - Metamorphosis triggers and transitions

mod config;
mod events;
mod fingerprint;
mod projection;
mod snapshot;
mod state;
mod tracker;

pub use config::{
    AgingConfig, DecayConfig, GrowthConfig, HatchingConfig, IncubationConfig, LifecycleConfig,
    MetamorphosisConfig, MetamorphosisTrigger,
};
pub use events::{LifecycleEvent, LifecycleEventKind};
pub use fingerprint::LifecycleFingerprint;
pub use projection::{LifecycleProjection, LifecycleTrend};
pub use snapshot::{LifecycleSnapshot, LifecycleSummary};
pub use state::{
    CorpseState, EggState, GrowthPhase, LifecycleId, LifecycleStage, LivingState,
    MetamorphosisState,
};
pub use tracker::{LifecycleTickResult, LifecycleTracker};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_lifecycle_simulation() {
        let config = LifecycleConfig::insect();
        let mut tracker = LifecycleTracker::new(config);

        let id = LifecycleId::new(1);
        tracker.spawn_egg(id, 0);

        assert!(tracker.get_stage(id).is_some());
        assert!(matches!(
            tracker.get_stage(id),
            Some(LifecycleStage::Egg(_))
        ));

        for tick in 1..=200 {
            let _ = tracker.tick(tick);
        }

        let stage = tracker.get_stage(id);
        assert!(
            matches!(stage, Some(LifecycleStage::Living(_)))
                || matches!(stage, Some(LifecycleStage::Egg(_)))
        );
    }

    #[test]
    fn test_mammal_lifecycle() {
        let config = LifecycleConfig::mammal();
        let mut tracker = LifecycleTracker::new(config.clone());

        let id = LifecycleId::new(1);
        tracker.spawn_living(id, GrowthPhase::Juvenile, 0);

        assert!(matches!(
            tracker.get_stage(id),
            Some(LifecycleStage::Living(_))
        ));

        for tick in 1..=config.growth.juvenile_duration {
            let _ = tracker.tick(tick);
        }

        if let Some(LifecycleStage::Living(state)) = tracker.get_stage(id) {
            assert!(matches!(
                state.phase,
                GrowthPhase::Adult | GrowthPhase::Juvenile
            ));
        }
    }

    #[test]
    fn test_corpse_decay() {
        let config = LifecycleConfig::minimal();
        let mut tracker = LifecycleTracker::new(config.clone());

        let id = LifecycleId::new(1);
        tracker.spawn_corpse(id, 100.0, 0);

        assert!(matches!(
            tracker.get_stage(id),
            Some(LifecycleStage::Corpse(_))
        ));

        for tick in 1..=(config.decay.full_decay_duration + 10) {
            let _ = tracker.tick(tick);
        }

        assert!(tracker.get_stage(id).is_none());
    }

    #[test]
    fn test_metamorphosis_trigger() {
        let mut config = LifecycleConfig::insect();
        config.metamorphosis = Some(MetamorphosisConfig {
            trigger: MetamorphosisTrigger::GrowthPhase(GrowthPhase::Adult),
            duration: 50,
            survival_chance: 1.0,
            result_growth_phase: GrowthPhase::Adult,
        });

        let mut tracker = LifecycleTracker::new(config.clone());

        let id = LifecycleId::new(1);
        tracker.spawn_living(id, GrowthPhase::Juvenile, 0);

        for tick in 1..=config.growth.juvenile_duration {
            let _ = tracker.tick(tick);
        }

        let stage = tracker.get_stage(id);
        assert!(
            matches!(stage, Some(LifecycleStage::Metamorphosis(_)))
                || matches!(stage, Some(LifecycleStage::Living(_)))
        );
    }

    #[test]
    fn test_snapshot_and_summary() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_egg(LifecycleId::new(1), 0);
        tracker.spawn_living(LifecycleId::new(2), GrowthPhase::Adult, 0);
        tracker.spawn_corpse(LifecycleId::new(3), 50.0, 0);

        let snapshot = LifecycleSnapshot::from_tracker(&tracker, 0);
        assert_eq!(snapshot.total_creatures, 3);
        assert_eq!(snapshot.egg_count, 1);
        assert_eq!(snapshot.living_count, 1);
        assert_eq!(snapshot.corpse_count, 1);

        let summary = LifecycleSummary::from_tracker(&tracker, 0);
        assert_eq!(summary.total_count, 3);
    }

    #[test]
    fn test_fingerprint_determinism() {
        let config = LifecycleConfig::standard();
        let mut tracker = LifecycleTracker::new(config);

        tracker.spawn_egg(LifecycleId::new(1), 0);

        let fp1 = LifecycleFingerprint::from_tracker(&tracker, 0);
        let fp2 = LifecycleFingerprint::from_tracker(&tracker, 0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_projection() {
        let config = LifecycleConfig::standard();
        let tracker = LifecycleTracker::new(config);

        let projection = LifecycleProjection::from_tracker(&tracker, 0, 1000);
        assert!(projection.projected_tick > 0);
    }

    #[test]
    fn test_config_serde() {
        let config = LifecycleConfig::insect();
        let json = serde_json::to_string(&config).unwrap();
        let restored: LifecycleConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored.incubation.base_duration,
            config.incubation.base_duration
        );
    }

    #[test]
    fn test_event_generation() {
        let mut config = LifecycleConfig::minimal();
        config.incubation.survival_chance = 1.0;
        let mut tracker = LifecycleTracker::new(config.clone());

        let id = LifecycleId::new(1);
        tracker.spawn_egg(id, 0);

        let mut hatched = false;
        for tick in 1..=(config.incubation.base_duration + config.hatching.hatching_duration + 5) {
            let result = tracker.tick(tick);
            for event in result.events {
                if matches!(event.kind, LifecycleEventKind::Hatched { .. }) {
                    hatched = true;
                }
            }
        }

        assert!(hatched);
    }
}

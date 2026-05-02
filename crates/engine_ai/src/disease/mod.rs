//! Disease and contamination system for procedural pathogen evolution.
//!
//! This module provides simulation of disease spread, pathogen mutation,
//! contamination zones, host infection tracking, and offline projections
//! for unloaded regions.

mod events;
mod fingerprint;
mod host;
mod ids;
mod mutation;
mod pathogen;
mod spread;
mod tracker;
mod zone;

pub use events::{DiseaseEvent, DiseaseEventKind, DiseaseTickEvents};
pub use fingerprint::DiseaseFingerprint;
pub use host::{
    ActiveInfection, HostInfectionState, HostTickResult, ImmunityRecord, InfectionStage,
    ResistanceProfile, StageTransition,
};
pub use ids::{ContaminationZoneId, DiseaseRegionId, HostId, PathogenId, StrainId};
pub use mutation::{
    EvolutionEvent, MutationConfig, MutationContext, MutationResult, MutationTracker, TraitChanges,
};
pub use pathogen::{
    PathogenCategory, PathogenDef, PathogenRegistry, PathogenTraits, TraitBounds,
    presets as pathogen_presets,
};
pub use spread::{
    CrossRegionSpread, ExposureEvent, ExposureSource, HostSpreadInfo, InfectionSpreadInfo,
    RegionPopulation, SpreadConfig, SpreadPlan, SpreadPlanSummary, SpreadPlanner, SpreadRoute,
};
pub use tracker::{
    CreateZoneRequest, DiseaseConfig, DiseaseProjection, DiseaseSnapshot, DiseaseSummary,
    DiseaseTickResult, DiseaseTracker,
};
pub use zone::{ContaminationRegistry, ContaminationSource, ContaminationZone, PathogenReservoir};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        let _ = PathogenId::plague();
        let _ = StrainId::base(PathogenId::plague());
        let _ = HostId::new(1);
        let _ = ContaminationZoneId::new(1);
        let _ = DiseaseRegionId::new("test");
    }

    #[test]
    fn test_integrated_workflow() {
        let mut tracker = DiseaseTracker::new(DiseaseConfig::default());

        let registry = pathogen_presets::create_preset_registry();
        for def in registry.iter() {
            tracker.register_pathogen(def.clone());
        }

        tracker.register_host(HostId::new(1), "human");
        tracker.register_host(HostId::new(2), "human");

        let plague_traits = tracker
            .pathogen_registry()
            .get(&PathogenId::plague())
            .unwrap()
            .base_traits
            .clone();

        tracker.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits.clone(),
            0,
        );

        assert_eq!(tracker.infected_count(), 1);

        for tick in 1..100 {
            let _result = tracker.tick(tick);
        }

        let summary = tracker.summary();
        assert!(summary.total_hosts > 0);

        let fingerprint = DiseaseFingerprint::from_tracker(&tracker, 100);
        assert!(fingerprint.raw() > 0);

        let snapshot = DiseaseSnapshot::from_tracker(&tracker, 100);
        let projection = DiseaseProjection::from_snapshot(&snapshot, 1000);
        assert_eq!(projection.start_tick, 100);
        assert_eq!(projection.end_tick, 1100);
    }

    #[test]
    fn test_contamination_workflow() {
        let mut tracker = DiseaseTracker::new(DiseaseConfig::default());

        let registry = pathogen_presets::create_preset_registry();
        for def in registry.iter() {
            tracker.register_pathogen(def.clone());
        }

        let rot_traits = tracker
            .pathogen_registry()
            .get(&PathogenId::rot())
            .unwrap()
            .base_traits
            .clone();

        let zone_id = tracker.create_contamination_zone(CreateZoneRequest::new(
            DiseaseRegionId::new("region1"),
            [10.0, 0.0, 0.0],
            5.0,
            StrainId::base(PathogenId::rot()),
            0.8,
            rot_traits,
            ContaminationSource::Corpse,
        ));

        assert_eq!(tracker.zone_count(), 1);

        let zone = tracker.contamination_registry().get(&zone_id).unwrap();
        assert!(zone.is_contaminated(0));
        assert!(zone.contains_position([10.0, 0.0, 0.0]));
        assert!(!zone.contains_position([100.0, 0.0, 0.0]));
    }

    #[test]
    fn test_spread_planning() {
        let planner = SpreadPlanner::new(SpreadConfig::default());

        let mut population = RegionPopulation::new(DiseaseRegionId::new("test"), 0);
        population.density = 0.5;

        population.add_host(HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0]));
        population.add_host(
            HostSpreadInfo::new(HostId::new(2), [0.5, 0.0, 0.0]).infectious(vec![
                InfectionSpreadInfo {
                    strain: StrainId::base(PathogenId::fever()),
                    transmission_probability: 0.5,
                    traits: PathogenTraits::default().with_transmissibility(0.7),
                },
            ]),
        );

        let plan = planner.plan_spread(&population);

        assert!(!plan.exposures.is_empty());
    }

    #[test]
    fn test_mutation_bounds() {
        let mut tracker = MutationTracker::new();
        let strain = StrainId::base(PathogenId::plague());
        let traits = PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_mutation_rate(1.0);

        let bounds = TraitBounds {
            min_transmissibility: 0.1,
            max_transmissibility: 0.9,
            min_virulence: 0.05,
            max_virulence: 0.8,
            min_incubation: 20,
            max_incubation: 500,
            min_lethality: 0.0,
            max_lethality: 0.7,
        };

        let config = MutationConfig::new().with_base_chance(1.0);

        for tick in 0..100 {
            let context = MutationContext::new(tick, 1);
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
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut tracker = DiseaseTracker::new(DiseaseConfig::default());

        let registry = pathogen_presets::create_preset_registry();
        for def in registry.iter() {
            tracker.register_pathogen(def.clone());
        }

        tracker.register_host(HostId::new(1), "human");
        let plague_traits = tracker
            .pathogen_registry()
            .get(&PathogenId::plague())
            .unwrap()
            .base_traits
            .clone();
        tracker.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits,
            0,
        );

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: DiseaseTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(tracker.host_count(), restored.host_count());
        assert_eq!(tracker.checksum(), restored.checksum());
    }

    #[test]
    fn test_bincode_roundtrip() {
        let mut tracker = DiseaseTracker::new(DiseaseConfig::default());

        let registry = pathogen_presets::create_preset_registry();
        for def in registry.iter() {
            tracker.register_pathogen(def.clone());
        }

        tracker.register_host(HostId::new(1), "human");

        let bytes = bincode::serialize(&tracker).unwrap();
        let restored: DiseaseTracker = bincode::deserialize(&bytes).unwrap();

        assert_eq!(tracker.checksum(), restored.checksum());
    }

    #[test]
    fn test_fingerprint_stability() {
        let mut tracker1 = DiseaseTracker::new(DiseaseConfig::default());
        let mut tracker2 = DiseaseTracker::new(DiseaseConfig::default());

        let registry = pathogen_presets::create_preset_registry();
        for def in registry.iter() {
            tracker1.register_pathogen(def.clone());
            tracker2.register_pathogen(def.clone());
        }

        tracker1.register_host(HostId::new(1), "human");
        tracker2.register_host(HostId::new(1), "human");

        let plague_traits = tracker1
            .pathogen_registry()
            .get(&PathogenId::plague())
            .unwrap()
            .base_traits
            .clone();

        tracker1.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits.clone(),
            0,
        );
        tracker2.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits,
            0,
        );

        let fp1 = DiseaseFingerprint::from_tracker(&tracker1, 0);
        let fp2 = DiseaseFingerprint::from_tracker(&tracker2, 0);

        assert!(fp1.matches(&fp2));
    }
}

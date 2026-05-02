//! Central disease tracking and simulation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::events::{DiseaseEvent, DiseaseEventKind, DiseaseTickEvents};
use super::host::{HostInfectionState, InfectionStage};
use super::ids::{ContaminationZoneId, DiseaseRegionId, HostId, StrainId};
use super::mutation::{
    MutationConfig, MutationContext, MutationResult, MutationTracker, TraitChanges,
};
use super::pathogen::{PathogenDef, PathogenRegistry, PathogenTraits};
use super::spread::{SpreadConfig, SpreadPlan};
use super::zone::{ContaminationRegistry, ContaminationSource};

/// Configuration for disease simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiseaseConfig {
    /// Mutation configuration.
    pub mutation: MutationConfig,
    /// Spread configuration.
    pub spread: SpreadConfig,
    /// Whether to process infections each tick.
    pub process_infections: bool,
    /// Whether to process contamination zones.
    pub process_zones: bool,
    /// Whether to allow mutations.
    pub allow_mutations: bool,
    /// Probability of death when critical (per tick).
    pub critical_death_chance: f32,
    /// Maximum hosts to process per tick (0 = unlimited).
    pub max_hosts_per_tick: usize,
}

impl Default for DiseaseConfig {
    fn default() -> Self {
        Self {
            mutation: MutationConfig::default(),
            spread: SpreadConfig::default(),
            process_infections: true,
            process_zones: true,
            allow_mutations: true,
            critical_death_chance: 0.01,
            max_hosts_per_tick: 0,
        }
    }
}

impl DiseaseConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_mutation_config(mut self, config: MutationConfig) -> Self {
        self.mutation = config;
        self
    }

    #[must_use]
    pub fn with_spread_config(mut self, config: SpreadConfig) -> Self {
        self.spread = config;
        self
    }

    #[must_use]
    pub fn with_mutations_enabled(mut self, enabled: bool) -> Self {
        self.allow_mutations = enabled;
        self
    }
}

/// Result of a disease simulation tick.
#[derive(Clone, Debug, Default)]
pub struct DiseaseTickResult {
    /// Events that occurred.
    pub events: DiseaseTickEvents,
    /// Spread plan for this tick.
    pub spread_plan: Option<SpreadPlan>,
    /// Number of hosts processed.
    pub hosts_processed: u32,
    /// Tick number.
    pub tick: u64,
}

/// Summary of disease state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiseaseSummary {
    /// Total registered hosts.
    pub total_hosts: u32,
    /// Currently infected hosts.
    pub infected_hosts: u32,
    /// Hosts showing symptoms.
    pub symptomatic_hosts: u32,
    /// Hosts that have recovered.
    pub recovered_hosts: u32,
    /// Deaths from disease.
    pub deaths: u32,
    /// Active contamination zones.
    pub active_zones: u32,
    /// Total pathogens registered.
    pub pathogen_count: u32,
    /// Active strains (including variants).
    pub active_strains: u32,
}

/// Request to create a contamination zone.
#[derive(Clone, Debug)]
pub struct CreateZoneRequest {
    /// Region ID.
    pub region_id: DiseaseRegionId,
    /// Position.
    pub position: [f32; 3],
    /// Radius.
    pub radius: f32,
    /// Strain.
    pub strain: StrainId,
    /// Concentration.
    pub concentration: f32,
    /// Traits.
    pub traits: PathogenTraits,
    /// Source.
    pub source: ContaminationSource,
}

impl CreateZoneRequest {
    #[must_use]
    pub fn new(
        region_id: DiseaseRegionId,
        position: [f32; 3],
        radius: f32,
        strain: StrainId,
        concentration: f32,
        traits: PathogenTraits,
        source: ContaminationSource,
    ) -> Self {
        Self {
            region_id,
            position,
            radius,
            strain,
            concentration,
            traits,
            source,
        }
    }
}

/// Snapshot of disease state for offline simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiseaseSnapshot {
    /// Tick when snapshot was taken.
    pub tick: u64,
    /// Summary statistics.
    pub summary: DiseaseSummary,
    /// Active infections by region.
    pub infections_by_region: BTreeMap<String, u32>,
    /// Contamination levels by region.
    pub contamination_by_region: BTreeMap<String, f32>,
    /// Strain distribution.
    pub strain_distribution: BTreeMap<String, u32>,
}

impl DiseaseSnapshot {
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "reservoir count bounded")]
    pub fn from_tracker(tracker: &DiseaseTracker, tick: u64) -> Self {
        let mut infections_by_region = BTreeMap::new();
        let mut strain_distribution = BTreeMap::new();

        for host in tracker.infected_hosts() {
            for infection in host.infections() {
                *strain_distribution
                    .entry(format!("{}", infection.strain))
                    .or_insert(0) += 1;
            }
        }

        for (_id, zone) in tracker.contamination_registry.iter() {
            let region = zone.region_id.as_str().to_string();
            *infections_by_region.entry(region).or_insert(0) += zone.reservoir_count() as u32;
        }

        Self {
            tick,
            summary: tracker.summary(),
            infections_by_region,
            contamination_by_region: BTreeMap::new(),
            strain_distribution,
        }
    }
}

/// Projection of disease spread for unloaded regions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiseaseProjection {
    /// Start tick.
    pub start_tick: u64,
    /// End tick (projected to).
    pub end_tick: u64,
    /// Projected new infections.
    pub projected_infections: u32,
    /// Projected recoveries.
    pub projected_recoveries: u32,
    /// Projected deaths.
    pub projected_deaths: u32,
    /// Projected contamination decay.
    pub projected_zone_decay: u32,
    /// Confidence in projection (0.0-1.0).
    pub confidence: f32,
}

impl DiseaseProjection {
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "approximation acceptable")]
    pub fn from_snapshot(snapshot: &DiseaseSnapshot, duration_ticks: u64) -> Self {
        let infection_rate =
            snapshot.summary.infected_hosts as f32 / snapshot.summary.total_hosts.max(1) as f32;
        let recovery_rate = 0.01;
        let death_rate = 0.001;

        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let projected_infections =
            (snapshot.summary.infected_hosts as f32 * infection_rate * duration_ticks as f32 * 0.1)
                as u32;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let projected_recoveries =
            (snapshot.summary.infected_hosts as f32 * recovery_rate * duration_ticks as f32) as u32;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let projected_deaths =
            (snapshot.summary.infected_hosts as f32 * death_rate * duration_ticks as f32) as u32;

        Self {
            start_tick: snapshot.tick,
            end_tick: snapshot.tick + duration_ticks,
            projected_infections,
            projected_recoveries,
            projected_deaths,
            projected_zone_decay: snapshot.summary.active_zones / 10,
            confidence: 0.7,
        }
    }
}

/// Central tracker for disease simulation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiseaseTracker {
    /// Configuration.
    config: DiseaseConfig,
    /// Pathogen registry.
    pathogen_registry: PathogenRegistry,
    /// Contamination zone registry.
    pub(crate) contamination_registry: ContaminationRegistry,
    /// Host infection states.
    hosts: BTreeMap<HostId, HostInfectionState>,
    /// Mutation tracker.
    mutation_tracker: MutationTracker,
    /// Current tick.
    current_tick: u64,
    /// Total deaths.
    total_deaths: u32,
    /// Total recoveries.
    total_recoveries: u32,
}

impl DiseaseTracker {
    #[must_use]
    pub fn new(config: DiseaseConfig) -> Self {
        Self {
            config,
            pathogen_registry: PathogenRegistry::new(),
            contamination_registry: ContaminationRegistry::new(),
            hosts: BTreeMap::new(),
            mutation_tracker: MutationTracker::new(),
            current_tick: 0,
            total_deaths: 0,
            total_recoveries: 0,
        }
    }

    #[must_use]
    pub fn config(&self) -> &DiseaseConfig {
        &self.config
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn register_pathogen(&mut self, def: PathogenDef) {
        self.pathogen_registry.register(def);
    }

    #[must_use]
    pub fn pathogen_registry(&self) -> &PathogenRegistry {
        &self.pathogen_registry
    }

    #[must_use]
    pub fn contamination_registry(&self) -> &ContaminationRegistry {
        &self.contamination_registry
    }

    pub fn register_host(&mut self, host_id: HostId, species: &str) {
        self.hosts
            .entry(host_id)
            .or_insert_with(|| HostInfectionState::new(host_id, species));
    }

    pub fn unregister_host(&mut self, host_id: &HostId) -> Option<HostInfectionState> {
        self.hosts.remove(host_id)
    }

    #[must_use]
    pub fn get_host(&self, host_id: &HostId) -> Option<&HostInfectionState> {
        self.hosts.get(host_id)
    }

    pub fn get_host_mut(&mut self, host_id: &HostId) -> Option<&mut HostInfectionState> {
        self.hosts.get_mut(host_id)
    }

    #[must_use]
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    #[must_use]
    pub fn infected_count(&self) -> usize {
        self.hosts.values().filter(|h| h.is_infected()).count()
    }

    #[must_use]
    pub fn zone_count(&self) -> usize {
        self.contamination_registry.len()
    }

    pub fn infected_hosts(&self) -> impl Iterator<Item = &HostInfectionState> {
        self.hosts.values().filter(|h| h.is_infected())
    }

    pub fn expose_host(
        &mut self,
        host_id: HostId,
        strain: StrainId,
        traits: PathogenTraits,
        tick: u64,
    ) -> bool {
        if let Some(host) = self.hosts.get_mut(&host_id) {
            host.expose(strain, traits, tick)
        } else {
            false
        }
    }

    pub fn create_contamination_zone(&mut self, request: CreateZoneRequest) -> ContaminationZoneId {
        let id = self.contamination_registry.create_zone(
            &request.region_id,
            request.position,
            request.radius,
            self.current_tick,
        );
        if let Some(zone) = self.contamination_registry.get_mut(&id) {
            zone.contaminate(
                request.strain,
                request.concentration,
                request.traits,
                self.current_tick,
                request.source,
            );
        }
        id
    }

    pub fn tick(&mut self, tick: u64) -> DiseaseTickResult {
        self.current_tick = tick;

        let mut result = DiseaseTickResult {
            tick,
            ..Default::default()
        };

        if self.config.process_infections {
            self.process_infections(&mut result);
        }

        if self.config.process_zones {
            self.contamination_registry.tick(tick);
        }

        result
    }

    fn process_infections(&mut self, result: &mut DiseaseTickResult) {
        let host_ids: Vec<_> = self.hosts.keys().copied().collect();
        let max_hosts = if self.config.max_hosts_per_tick > 0 {
            self.config.max_hosts_per_tick
        } else {
            host_ids.len()
        };

        for host_id in host_ids.into_iter().take(max_hosts) {
            if let Some(host) = self.hosts.get_mut(&host_id) {
                host.set_tick(self.current_tick);
                self.process_host_infection(host_id, result);
                result.hosts_processed += 1;
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process_host_infection(&mut self, host_id: HostId, result: &mut DiseaseTickResult) {
        let Some(host) = self.hosts.get_mut(&host_id) else {
            return;
        };

        let pathogen_ids: Vec<_> = host
            .infections()
            .map(|i| i.strain.pathogen.clone())
            .collect();

        for pathogen_id in pathogen_ids {
            let (bounds, immunity_duration) = {
                let def = self.pathogen_registry.get(&pathogen_id);
                (
                    def.map(|d| d.trait_bounds.clone()),
                    def.map_or(0, |d| d.base_traits.immunity_duration),
                )
            };

            let infection_info = {
                let host = self.hosts.get_mut(&host_id).unwrap();
                if let Some(infection) = host.get_infection_mut(&pathogen_id) {
                    #[expect(clippy::cast_precision_loss, reason = "duration bounded")]
                    let progression_rate =
                        1.0 / infection.effective_traits.incubation_duration.max(1) as f32;
                    infection.advance_progression(progression_rate);

                    if infection.ready_to_progress() {
                        Some((
                            infection.stage,
                            infection.is_critical(),
                            infection.effective_traits.lethality,
                            infection.strain.pathogen.as_str().to_string(),
                            infection.strain.variant,
                            infection.severity,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((stage, is_critical, lethality, pathogen_str, variant, severity)) =
                infection_info
            {
                let next_stage = self.determine_next_stage_from(
                    stage,
                    is_critical,
                    lethality,
                    &pathogen_str,
                    variant,
                );

                if next_stage != stage {
                    let tick = self.current_tick;

                    let host = self.hosts.get_mut(&host_id).unwrap();
                    if let Some(infection) = host.get_infection_mut(&pathogen_id) {
                        infection.transition_to(next_stage, tick);
                    }

                    match next_stage {
                        InfectionStage::Incubating => {
                            result.events.push(DiseaseEvent::incubation_started(
                                tick,
                                host_id,
                                pathogen_id.clone(),
                            ));
                        }
                        InfectionStage::Symptomatic => {
                            result.events.push(DiseaseEvent::symptoms_appeared(
                                tick,
                                host_id,
                                pathogen_id.clone(),
                                severity,
                            ));
                        }
                        InfectionStage::Recovering => {
                            result.events.push(DiseaseEvent::new(
                                tick,
                                DiseaseEventKind::RecoveryStarted {
                                    host_id,
                                    pathogen_id: pathogen_id.clone(),
                                },
                            ));
                        }
                        InfectionStage::Immune => {
                            let host = self.hosts.get_mut(&host_id).unwrap();
                            host.clear_infection(&pathogen_id);
                            host.grant_immunity(pathogen_id.clone(), immunity_duration, tick);
                            self.total_recoveries += 1;
                            result.events.push(DiseaseEvent::recovered(
                                tick,
                                host_id,
                                pathogen_id.clone(),
                                immunity_duration,
                            ));
                        }
                        InfectionStage::Dead => {
                            self.total_deaths += 1;
                            result.events.push(DiseaseEvent::died(
                                tick,
                                host_id,
                                pathogen_id.clone(),
                            ));
                        }
                        _ => {}
                    }
                }
            }

            if self.config.allow_mutations
                && let Some(bounds) = bounds
            {
                self.try_mutation(host_id, &pathogen_id, &bounds, result);
            }
        }
    }

    fn try_mutation(
        &mut self,
        host_id: HostId,
        pathogen_id: &super::ids::PathogenId,
        bounds: &super::pathogen::TraitBounds,
        result: &mut DiseaseTickResult,
    ) {
        let mutation_info = {
            let host = self.hosts.get(&host_id).unwrap();
            host.get_infection(pathogen_id).map(|infection| {
                (
                    MutationContext::new(self.current_tick, host_id.raw())
                        .with_load(infection.pathogen_load)
                        .with_treatment(infection.under_treatment),
                    infection.strain.clone(),
                    infection.effective_traits.clone(),
                )
            })
        };

        let Some((context, strain, traits)) = mutation_info else {
            return;
        };

        let mutation_result = self.mutation_tracker.attempt_mutation(
            &strain,
            &traits,
            bounds,
            &self.config.mutation,
            &context,
        );

        match mutation_result {
            MutationResult::MinorDrift(new_traits) => {
                let host = self.hosts.get_mut(&host_id).unwrap();
                if let Some(infection) = host.get_infection_mut(pathogen_id) {
                    let changes = TraitChanges::from_diff(&infection.effective_traits, &new_traits);
                    infection.effective_traits = new_traits;
                    result.events.push(DiseaseEvent::mutated(
                        self.current_tick,
                        host_id,
                        strain.clone(),
                        strain,
                        false,
                        changes,
                    ));
                }
            }
            MutationResult::NewVariant {
                variant_id,
                traits: new_traits,
            } => {
                let new_strain = StrainId::new(pathogen_id.clone(), variant_id);
                let host = self.hosts.get_mut(&host_id).unwrap();
                if let Some(infection) = host.get_infection_mut(pathogen_id) {
                    let changes = TraitChanges::from_diff(&infection.effective_traits, &new_traits);
                    infection.strain = new_strain.clone();
                    infection.effective_traits = new_traits;
                    result.events.push(DiseaseEvent::mutated(
                        self.current_tick,
                        host_id,
                        strain,
                        new_strain,
                        true,
                        changes,
                    ));
                }
            }
            MutationResult::NoMutation => {}
        }
    }

    fn determine_next_stage_from(
        &self,
        stage: InfectionStage,
        is_critical: bool,
        lethality: f32,
        pathogen: &str,
        variant: u32,
    ) -> InfectionStage {
        match stage {
            InfectionStage::Exposed => InfectionStage::Incubating,
            InfectionStage::Incubating => InfectionStage::Symptomatic,
            InfectionStage::Symptomatic => {
                if is_critical {
                    let roll = deterministic_roll(pathogen, self.current_tick, variant);
                    if roll < lethality {
                        return InfectionStage::Dead;
                    }
                }
                InfectionStage::Recovering
            }
            InfectionStage::Recovering => InfectionStage::Immune,
            other => other,
        }
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts bounded")]
    pub fn summary(&self) -> DiseaseSummary {
        let mut strain_set = std::collections::BTreeSet::new();
        let mut infected_hosts = 0u32;
        let mut symptomatic_hosts = 0u32;

        for host in self.hosts.values() {
            if host.is_infected() {
                infected_hosts += 1;
            }
            if host.is_sick() {
                symptomatic_hosts += 1;
            }
            for infection in host.infections() {
                strain_set.insert(format!("{}", infection.strain));
            }
        }

        DiseaseSummary {
            total_hosts: self.hosts.len() as u32,
            infected_hosts,
            symptomatic_hosts,
            recovered_hosts: self.total_recoveries,
            deaths: self.total_deaths,
            active_zones: self.contamination_registry.len() as u32,
            pathogen_count: self.pathogen_registry.len() as u32,
            active_strains: strain_set.len() as u32,
        }
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "checksum bounded")]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.current_tick.to_le_bytes());
        hasher.update(&(self.hosts.len() as u32).to_le_bytes());
        hasher.update(&self.pathogen_registry.checksum().to_le_bytes());
        hasher.update(&self.contamination_registry.checksum().to_le_bytes());
        for host in self.hosts.values() {
            hasher.update(&host.checksum().to_le_bytes());
        }
        hasher.finalize()
    }
}

#[expect(clippy::cast_precision_loss)]
fn deterministic_roll(pathogen: &str, tick: u64, variant: u32) -> f32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(pathogen.as_bytes());
    hasher.update(&tick.to_le_bytes());
    hasher.update(&variant.to_le_bytes());
    let hash = hasher.finalize();
    hash as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disease::ids::PathogenId;
    use crate::disease::pathogen::presets;

    fn make_test_tracker() -> DiseaseTracker {
        let config = DiseaseConfig::default();
        let mut tracker = DiseaseTracker::new(config);

        let registry = presets::create_preset_registry();
        for def in registry.iter() {
            tracker.register_pathogen(def.clone());
        }

        tracker
    }

    #[test]
    fn test_disease_config_default() {
        let config = DiseaseConfig::default();
        assert!(config.process_infections);
        assert!(config.allow_mutations);
    }

    #[test]
    fn test_disease_config_builder() {
        let config = DiseaseConfig::new().with_mutations_enabled(false);
        assert!(!config.allow_mutations);
    }

    #[test]
    fn test_tracker_new() {
        let tracker = make_test_tracker();
        assert_eq!(tracker.host_count(), 0);
        assert!(!tracker.pathogen_registry().is_empty());
    }

    #[test]
    fn test_tracker_register_host() {
        let mut tracker = make_test_tracker();

        tracker.register_host(HostId::new(1), "human");
        tracker.register_host(HostId::new(2), "human");

        assert_eq!(tracker.host_count(), 2);
        assert!(tracker.get_host(&HostId::new(1)).is_some());
    }

    #[test]
    fn test_tracker_expose_host() {
        let mut tracker = make_test_tracker();

        tracker.register_host(HostId::new(1), "human");

        let plague_traits = tracker
            .pathogen_registry()
            .get(&PathogenId::plague())
            .unwrap()
            .base_traits
            .clone();

        let exposed = tracker.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            plague_traits,
            0,
        );

        assert!(exposed);
        assert_eq!(tracker.infected_count(), 1);
    }

    #[test]
    fn test_tracker_expose_nonexistent_host() {
        let mut tracker = make_test_tracker();

        let exposed = tracker.expose_host(
            HostId::new(99),
            StrainId::base(PathogenId::plague()),
            PathogenTraits::default(),
            0,
        );

        assert!(!exposed);
    }

    #[test]
    fn test_tracker_tick_progresses_infection() {
        let mut tracker = make_test_tracker();
        tracker.register_host(HostId::new(1), "human");

        let traits = PathogenTraits::default()
            .with_incubation(1)
            .with_transmissibility(0.5);

        tracker.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            traits,
            0,
        );

        let result = tracker.tick(1);

        assert!(!result.events.is_empty());
        assert_eq!(result.hosts_processed, 1);
    }

    #[test]
    fn test_tracker_contamination_zone() {
        let mut tracker = make_test_tracker();

        let zone_id = tracker.create_contamination_zone(CreateZoneRequest::new(
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            5.0,
            StrainId::base(PathogenId::plague()),
            0.8,
            PathogenTraits::default(),
            ContaminationSource::Corpse,
        ));

        assert_eq!(tracker.zone_count(), 1);
        assert!(tracker.contamination_registry().get(&zone_id).is_some());
    }

    #[test]
    fn test_tracker_summary() {
        let mut tracker = make_test_tracker();

        tracker.register_host(HostId::new(1), "human");
        tracker.register_host(HostId::new(2), "human");

        let traits = PathogenTraits::default();
        tracker.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            traits,
            0,
        );

        let summary = tracker.summary();

        assert_eq!(summary.total_hosts, 2);
        assert_eq!(summary.infected_hosts, 1);
    }

    #[test]
    fn test_tracker_checksum_deterministic() {
        let mut tracker1 = make_test_tracker();
        let mut tracker2 = make_test_tracker();

        tracker1.register_host(HostId::new(1), "human");
        tracker2.register_host(HostId::new(1), "human");

        assert_eq!(tracker1.checksum(), tracker2.checksum());
    }

    #[test]
    fn test_disease_snapshot() {
        let mut tracker = make_test_tracker();
        tracker.register_host(HostId::new(1), "human");

        let snapshot = DiseaseSnapshot::from_tracker(&tracker, 0);

        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.summary.total_hosts, 1);
    }

    #[test]
    fn test_disease_projection() {
        let mut tracker = make_test_tracker();
        tracker.register_host(HostId::new(1), "human");
        tracker.register_host(HostId::new(2), "human");

        let traits = PathogenTraits::default();
        tracker.expose_host(
            HostId::new(1),
            StrainId::base(PathogenId::plague()),
            traits,
            0,
        );

        let snapshot = DiseaseSnapshot::from_tracker(&tracker, 0);
        let projection = DiseaseProjection::from_snapshot(&snapshot, 100);

        assert_eq!(projection.start_tick, 0);
        assert_eq!(projection.end_tick, 100);
        assert!(projection.confidence > 0.0);
    }

    #[test]
    fn test_serde_disease_config() {
        let config = DiseaseConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: DiseaseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, restored);
    }

    #[test]
    fn test_serde_disease_summary() {
        let summary = DiseaseSummary {
            total_hosts: 100,
            infected_hosts: 20,
            symptomatic_hosts: 10,
            recovered_hosts: 5,
            deaths: 2,
            active_zones: 3,
            pathogen_count: 6,
            active_strains: 8,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: DiseaseSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary, restored);
    }

    #[test]
    fn test_serde_tracker() {
        let mut tracker = make_test_tracker();
        tracker.register_host(HostId::new(1), "human");

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: DiseaseTracker = serde_json::from_str(&json).unwrap();

        assert_eq!(tracker.host_count(), restored.host_count());
    }

    #[test]
    fn test_bincode_tracker() {
        let mut tracker = make_test_tracker();
        tracker.register_host(HostId::new(1), "human");

        let bytes = bincode::serialize(&tracker).unwrap();
        let restored: DiseaseTracker = bincode::deserialize(&bytes).unwrap();

        assert_eq!(tracker.host_count(), restored.host_count());
        assert_eq!(tracker.checksum(), restored.checksum());
    }
}

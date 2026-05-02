//! Spread and exposure planning across contacts, regions, and zones.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::ids::{ContaminationZoneId, DiseaseRegionId, HostId, PathogenId, StrainId};
use super::pathogen::PathogenTraits;

/// A potential exposure event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExposureEvent {
    /// Target host ID.
    pub target_host: HostId,
    /// Strain being transmitted.
    pub strain: StrainId,
    /// Probability of transmission (0.0-1.0).
    pub transmission_probability: f32,
    /// Source of exposure.
    pub source: ExposureSource,
    /// Tick when exposure occurred.
    pub tick: u64,
}

/// Source of a potential exposure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExposureSource {
    /// Direct contact with infected host.
    DirectContact { source_host: HostId },
    /// Proximity/airborne from infected host.
    Proximity { source_host: HostId, distance: f32 },
    /// Contact with contamination zone.
    ContaminationZone { zone_id: ContaminationZoneId },
    /// Environmental reservoir.
    Environmental { region_id: DiseaseRegionId },
    /// Vector transmission (e.g., insect bite).
    Vector { vector_type: String },
}

impl ExposureEvent {
    #[must_use]
    pub fn direct_contact(
        target: HostId,
        source: HostId,
        strain: StrainId,
        probability: f32,
        tick: u64,
    ) -> Self {
        Self {
            target_host: target,
            strain,
            transmission_probability: probability.clamp(0.0, 1.0),
            source: ExposureSource::DirectContact {
                source_host: source,
            },
            tick,
        }
    }

    #[must_use]
    pub fn proximity(
        target: HostId,
        source: HostId,
        strain: StrainId,
        probability: f32,
        distance: f32,
        tick: u64,
    ) -> Self {
        Self {
            target_host: target,
            strain,
            transmission_probability: probability.clamp(0.0, 1.0),
            source: ExposureSource::Proximity {
                source_host: source,
                distance,
            },
            tick,
        }
    }

    #[must_use]
    pub fn from_zone(
        target: HostId,
        zone_id: ContaminationZoneId,
        strain: StrainId,
        probability: f32,
        tick: u64,
    ) -> Self {
        Self {
            target_host: target,
            strain,
            transmission_probability: probability.clamp(0.0, 1.0),
            source: ExposureSource::ContaminationZone { zone_id },
            tick,
        }
    }
}

/// Configuration for spread planning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpreadConfig {
    /// Base transmission probability for direct contact.
    pub base_contact_probability: f32,
    /// Distance at which airborne transmission is possible.
    pub max_airborne_distance: f32,
    /// Probability reduction per unit distance for airborne.
    pub distance_decay_rate: f32,
    /// Minimum population density for epidemic spread.
    pub min_epidemic_density: f32,
    /// Contacts per entity per tick for direct spread.
    pub contacts_per_tick: u32,
    /// Whether to allow cross-region spread.
    pub allow_cross_region_spread: bool,
    /// Probability of cross-region spread per tick.
    pub cross_region_spread_chance: f32,
}

impl Default for SpreadConfig {
    fn default() -> Self {
        Self {
            base_contact_probability: 0.1,
            max_airborne_distance: 10.0,
            distance_decay_rate: 0.1,
            min_epidemic_density: 0.3,
            contacts_per_tick: 3,
            allow_cross_region_spread: true,
            cross_region_spread_chance: 0.01,
        }
    }
}

impl SpreadConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_contacts_per_tick(mut self, contacts: u32) -> Self {
        self.contacts_per_tick = contacts;
        self
    }

    #[must_use]
    pub fn with_max_airborne_distance(mut self, distance: f32) -> Self {
        self.max_airborne_distance = distance.max(0.0);
        self
    }

    #[must_use]
    pub fn with_cross_region_spread(mut self, enabled: bool, chance: f32) -> Self {
        self.allow_cross_region_spread = enabled;
        self.cross_region_spread_chance = chance.clamp(0.0, 1.0);
        self
    }
}

/// Input for spread planning: hosts in a region.
#[derive(Clone, Debug, Default)]
pub struct RegionPopulation {
    /// Region identifier.
    pub region_id: DiseaseRegionId,
    /// All hosts in the region.
    pub hosts: Vec<HostSpreadInfo>,
    /// Population density (0.0-1.0).
    pub density: f32,
    /// Current tick.
    pub tick: u64,
}

impl RegionPopulation {
    #[must_use]
    pub fn new(region_id: DiseaseRegionId, tick: u64) -> Self {
        Self {
            region_id,
            hosts: Vec::new(),
            density: 0.0,
            tick,
        }
    }

    pub fn add_host(&mut self, host: HostSpreadInfo) {
        self.hosts.push(host);
    }

    #[must_use]
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    #[must_use]
    pub fn infectious_count(&self) -> usize {
        self.hosts.iter().filter(|h| h.is_infectious).count()
    }

    #[must_use]
    pub fn susceptible_count(&self) -> usize {
        self.hosts.iter().filter(|h| h.is_susceptible).count()
    }
}

/// Spread-relevant info for a single host.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HostSpreadInfo {
    /// Host identifier.
    pub host_id: HostId,
    /// Position in region.
    pub position: [f32; 3],
    /// Whether this host can transmit disease.
    pub is_infectious: bool,
    /// Whether this host can be infected.
    pub is_susceptible: bool,
    /// Current infections with their transmission info.
    pub infections: Vec<InfectionSpreadInfo>,
    /// Pathogens this host is immune to.
    pub immunities: Vec<PathogenId>,
}

impl HostSpreadInfo {
    #[must_use]
    pub fn new(host_id: HostId, position: [f32; 3]) -> Self {
        Self {
            host_id,
            position,
            is_infectious: false,
            is_susceptible: true,
            infections: Vec::new(),
            immunities: Vec::new(),
        }
    }

    #[must_use]
    pub fn infectious(mut self, infections: Vec<InfectionSpreadInfo>) -> Self {
        self.is_infectious = true;
        self.is_susceptible = false;
        self.infections = infections;
        self
    }

    #[must_use]
    pub fn immune_to(mut self, pathogens: Vec<PathogenId>) -> Self {
        self.immunities = pathogens;
        self
    }

    /// Check if host is immune to a pathogen.
    #[must_use]
    pub fn is_immune_to(&self, pathogen_id: &PathogenId) -> bool {
        self.immunities.contains(pathogen_id)
    }
}

/// Spread info for an active infection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InfectionSpreadInfo {
    /// Strain being carried.
    pub strain: StrainId,
    /// Current transmission probability.
    pub transmission_probability: f32,
    /// Effective traits for this strain.
    pub traits: PathogenTraits,
}

/// Result of spread planning for a region.
#[derive(Clone, Debug, Default)]
pub struct SpreadPlan {
    /// Potential exposure events.
    pub exposures: Vec<ExposureEvent>,
    /// Summary statistics.
    pub summary: SpreadPlanSummary,
}

impl SpreadPlan {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_exposure(&mut self, exposure: ExposureEvent) {
        self.exposures.push(exposure);
    }

    #[must_use]
    pub fn exposure_count(&self) -> usize {
        self.exposures.len()
    }
}

/// Summary of spread planning results.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpreadPlanSummary {
    /// Total potential exposures.
    pub total_exposures: u32,
    /// Exposures from direct contact.
    pub direct_contact_exposures: u32,
    /// Exposures from proximity/airborne.
    pub proximity_exposures: u32,
    /// Exposures from contamination zones.
    pub zone_exposures: u32,
    /// Unique hosts potentially exposed.
    pub unique_hosts_exposed: u32,
    /// Average transmission probability.
    pub avg_transmission_probability: f32,
    /// Highest transmission probability.
    pub max_transmission_probability: f32,
}

impl SpreadPlanSummary {
    #[expect(clippy::cast_possible_truncation, reason = "exposure count bounded")]
    pub fn update_from_exposures(&mut self, exposures: &[ExposureEvent]) {
        self.total_exposures = exposures.len() as u32;

        let mut unique_hosts = std::collections::BTreeSet::new();
        let mut prob_sum = 0.0f32;
        let mut max_prob = 0.0f32;

        for exp in exposures {
            unique_hosts.insert(exp.target_host);
            prob_sum += exp.transmission_probability;
            max_prob = max_prob.max(exp.transmission_probability);

            match &exp.source {
                ExposureSource::DirectContact { .. } => self.direct_contact_exposures += 1,
                ExposureSource::Proximity { .. } => self.proximity_exposures += 1,
                ExposureSource::ContaminationZone { .. } => self.zone_exposures += 1,
                _ => {}
            }
        }

        self.unique_hosts_exposed = unique_hosts.len() as u32;
        #[expect(clippy::cast_precision_loss, reason = "exposure count bounded")]
        let avg = if exposures.is_empty() {
            0.0
        } else {
            prob_sum / exposures.len() as f32
        };
        self.avg_transmission_probability = avg;
        self.max_transmission_probability = max_prob;
    }
}

/// Spread planner for a region.
pub struct SpreadPlanner {
    config: SpreadConfig,
}

impl SpreadPlanner {
    #[must_use]
    pub fn new(config: SpreadConfig) -> Self {
        Self { config }
    }

    /// Plan spread events for a region.
    #[must_use]
    pub fn plan_spread(&self, population: &RegionPopulation) -> SpreadPlan {
        let mut plan = SpreadPlan::new();

        if population.density < self.config.min_epidemic_density {
            return plan;
        }

        let infectious: Vec<_> = population
            .hosts
            .iter()
            .filter(|h| h.is_infectious)
            .collect();

        let susceptible: Vec<_> = population
            .hosts
            .iter()
            .filter(|h| h.is_susceptible)
            .collect();

        for source in &infectious {
            for infection in &source.infections {
                self.plan_from_infectious_host(
                    source,
                    infection,
                    &susceptible,
                    population,
                    &mut plan,
                );
            }
        }

        plan.summary.update_from_exposures(&plan.exposures);
        plan
    }

    fn plan_from_infectious_host(
        &self,
        source: &HostSpreadInfo,
        infection: &InfectionSpreadInfo,
        susceptible: &[&HostSpreadInfo],
        population: &RegionPopulation,
        plan: &mut SpreadPlan,
    ) {
        let mut contacts_made = 0u32;

        for target in susceptible {
            if contacts_made >= self.config.contacts_per_tick {
                break;
            }

            if target.is_immune_to(&infection.strain.pathogen) {
                continue;
            }

            let distance = distance_3d(source.position, target.position);

            if distance <= 1.0 {
                let prob = Self::compute_contact_probability(
                    infection.transmission_probability,
                    &infection.traits,
                    population.density,
                );

                if prob > 0.0 {
                    plan.add_exposure(ExposureEvent::direct_contact(
                        target.host_id,
                        source.host_id,
                        infection.strain.clone(),
                        prob,
                        population.tick,
                    ));
                    contacts_made += 1;
                }
            } else if infection.traits.is_airborne()
                && distance <= self.config.max_airborne_distance
            {
                let prob = self.compute_airborne_probability(
                    infection.transmission_probability,
                    &infection.traits,
                    distance,
                );

                if prob > 0.0 {
                    plan.add_exposure(ExposureEvent::proximity(
                        target.host_id,
                        source.host_id,
                        infection.strain.clone(),
                        prob,
                        distance,
                        population.tick,
                    ));
                }
            }
        }
    }

    fn compute_contact_probability(base_prob: f32, traits: &PathogenTraits, density: f32) -> f32 {
        let density_factor = density.sqrt();
        let prob = base_prob * traits.transmissibility * density_factor;
        prob.clamp(0.0, 0.95)
    }

    fn compute_airborne_probability(
        &self,
        base_prob: f32,
        traits: &PathogenTraits,
        distance: f32,
    ) -> f32 {
        let distance_factor = (-distance * self.config.distance_decay_rate).exp();
        let range_factor = traits.transmission_range / 5.0;
        let prob = base_prob * distance_factor * range_factor;
        prob.clamp(0.0, 0.5)
    }

    /// Plan spread from contamination zones.
    #[must_use]
    pub fn plan_zone_exposures(
        &self,
        hosts: &[HostSpreadInfo],
        zones: &[(ContaminationZoneId, [f32; 3], f32, StrainId, f32)],
        tick: u64,
    ) -> Vec<ExposureEvent> {
        let mut exposures = Vec::new();

        for host in hosts {
            if !host.is_susceptible {
                continue;
            }

            for (zone_id, zone_pos, zone_radius, strain, zone_prob) in zones {
                if host.is_immune_to(&strain.pathogen) {
                    continue;
                }

                let distance = distance_3d(host.position, *zone_pos);
                if distance <= *zone_radius {
                    let distance_factor = 1.0 - (distance / zone_radius).powi(2);
                    let prob = zone_prob * distance_factor;

                    if prob > 0.01 {
                        exposures.push(ExposureEvent::from_zone(
                            host.host_id,
                            zone_id.clone(),
                            strain.clone(),
                            prob,
                            tick,
                        ));
                    }
                }
            }
        }

        exposures
    }
}

fn distance_3d(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Cross-region spread tracking.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CrossRegionSpread {
    /// Active spread routes between regions.
    routes: BTreeMap<(String, String), SpreadRoute>,
}

impl CrossRegionSpread {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a spread route between regions.
    pub fn add_route(&mut self, from: DiseaseRegionId, to: DiseaseRegionId, capacity: f32) {
        let key = (from.as_str().to_string(), to.as_str().to_string());
        self.routes.insert(
            key,
            SpreadRoute {
                from_region: from,
                to_region: to,
                capacity,
                active_strains: Vec::new(),
            },
        );
    }

    /// Add an active strain to a route.
    pub fn activate_strain(
        &mut self,
        from: &DiseaseRegionId,
        to: &DiseaseRegionId,
        strain: StrainId,
        intensity: f32,
    ) {
        let key = (from.as_str().to_string(), to.as_str().to_string());
        if let Some(route) = self.routes.get_mut(&key) {
            route.active_strains.push((strain, intensity));
        }
    }

    /// Get routes from a region.
    pub fn routes_from(&self, region: &DiseaseRegionId) -> impl Iterator<Item = &SpreadRoute> {
        let prefix = region.as_str().to_string();
        self.routes
            .iter()
            .filter(move |((from, _), _)| from == &prefix)
            .map(|(_, route)| route)
    }

    /// Clear expired strains.
    pub fn clear_inactive(&mut self) {
        for route in self.routes.values_mut() {
            route
                .active_strains
                .retain(|(_, intensity)| *intensity > 0.01);
        }
    }
}

/// A route for disease spread between regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpreadRoute {
    pub from_region: DiseaseRegionId,
    pub to_region: DiseaseRegionId,
    pub capacity: f32,
    pub active_strains: Vec<(StrainId, f32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_traits() -> PathogenTraits {
        PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_transmission_range(1.0)
    }

    fn make_airborne_traits() -> PathogenTraits {
        PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_transmission_range(5.0)
    }

    #[test]
    fn test_exposure_event_direct_contact() {
        let event = ExposureEvent::direct_contact(
            HostId::new(1),
            HostId::new(2),
            StrainId::base(PathogenId::plague()),
            0.5,
            100,
        );

        assert_eq!(event.target_host.raw(), 1);
        assert!((event.transmission_probability - 0.5).abs() < f32::EPSILON);
        assert!(matches!(event.source, ExposureSource::DirectContact { .. }));
    }

    #[test]
    fn test_exposure_event_proximity() {
        let event = ExposureEvent::proximity(
            HostId::new(1),
            HostId::new(2),
            StrainId::base(PathogenId::fever()),
            0.3,
            5.0,
            100,
        );

        if let ExposureSource::Proximity { distance, .. } = event.source {
            assert!((distance - 5.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected Proximity source");
        }
    }

    #[test]
    fn test_spread_config_default() {
        let config = SpreadConfig::default();
        assert!(config.contacts_per_tick > 0);
        assert!(config.max_airborne_distance > 0.0);
    }

    #[test]
    fn test_spread_config_builder() {
        let config = SpreadConfig::new()
            .with_contacts_per_tick(5)
            .with_max_airborne_distance(15.0)
            .with_cross_region_spread(false, 0.0);

        assert_eq!(config.contacts_per_tick, 5);
        assert!((config.max_airborne_distance - 15.0).abs() < f32::EPSILON);
        assert!(!config.allow_cross_region_spread);
    }

    #[test]
    fn test_region_population() {
        let mut pop = RegionPopulation::new(DiseaseRegionId::new("test"), 0);
        pop.density = 0.5;

        pop.add_host(HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0]));
        pop.add_host(
            HostSpreadInfo::new(HostId::new(2), [1.0, 0.0, 0.0]).infectious(vec![
                InfectionSpreadInfo {
                    strain: StrainId::base(PathogenId::plague()),
                    transmission_probability: 0.5,
                    traits: make_test_traits(),
                },
            ]),
        );

        assert_eq!(pop.host_count(), 2);
        assert_eq!(pop.infectious_count(), 1);
        assert_eq!(pop.susceptible_count(), 1);
    }

    #[test]
    fn test_host_spread_info() {
        let host = HostSpreadInfo::new(HostId::new(1), [1.0, 2.0, 3.0])
            .immune_to(vec![PathogenId::plague()]);

        assert!(host.is_immune_to(&PathogenId::plague()));
        assert!(!host.is_immune_to(&PathogenId::fever()));
    }

    #[test]
    fn test_spread_planner_no_spread_low_density() {
        let planner = SpreadPlanner::new(SpreadConfig::default());

        let mut pop = RegionPopulation::new(DiseaseRegionId::new("test"), 0);
        pop.density = 0.1;

        pop.add_host(HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0]));
        pop.add_host(
            HostSpreadInfo::new(HostId::new(2), [0.5, 0.0, 0.0]).infectious(vec![
                InfectionSpreadInfo {
                    strain: StrainId::base(PathogenId::plague()),
                    transmission_probability: 0.8,
                    traits: make_test_traits(),
                },
            ]),
        );

        let plan = planner.plan_spread(&pop);

        assert_eq!(plan.exposure_count(), 0);
    }

    #[test]
    fn test_spread_planner_direct_contact() {
        let planner = SpreadPlanner::new(SpreadConfig::default());

        let mut pop = RegionPopulation::new(DiseaseRegionId::new("test"), 0);
        pop.density = 0.5;

        pop.add_host(HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0]));
        pop.add_host(
            HostSpreadInfo::new(HostId::new(2), [0.5, 0.0, 0.0]).infectious(vec![
                InfectionSpreadInfo {
                    strain: StrainId::base(PathogenId::plague()),
                    transmission_probability: 0.8,
                    traits: make_test_traits(),
                },
            ]),
        );

        let plan = planner.plan_spread(&pop);

        assert!(!plan.exposures.is_empty());
        assert!(plan.summary.direct_contact_exposures > 0);
    }

    #[test]
    fn test_spread_planner_airborne() {
        let config = SpreadConfig::new().with_max_airborne_distance(20.0);
        let planner = SpreadPlanner::new(config);

        let mut pop = RegionPopulation::new(DiseaseRegionId::new("test"), 0);
        pop.density = 0.5;

        pop.add_host(HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0]));
        pop.add_host(
            HostSpreadInfo::new(HostId::new(2), [5.0, 0.0, 0.0]).infectious(vec![
                InfectionSpreadInfo {
                    strain: StrainId::base(PathogenId::fever()),
                    transmission_probability: 0.5,
                    traits: make_airborne_traits(),
                },
            ]),
        );

        let plan = planner.plan_spread(&pop);

        assert!(plan.summary.proximity_exposures > 0 || plan.summary.direct_contact_exposures > 0);
    }

    #[test]
    fn test_spread_planner_immune_hosts_skipped() {
        let planner = SpreadPlanner::new(SpreadConfig::default());

        let mut pop = RegionPopulation::new(DiseaseRegionId::new("test"), 0);
        pop.density = 0.5;

        pop.add_host(
            HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0])
                .immune_to(vec![PathogenId::plague()]),
        );
        pop.add_host(
            HostSpreadInfo::new(HostId::new(2), [0.5, 0.0, 0.0]).infectious(vec![
                InfectionSpreadInfo {
                    strain: StrainId::base(PathogenId::plague()),
                    transmission_probability: 0.8,
                    traits: make_test_traits(),
                },
            ]),
        );

        let plan = planner.plan_spread(&pop);

        assert!(plan.exposures.is_empty());
    }

    #[test]
    fn test_spread_planner_zone_exposures() {
        let planner = SpreadPlanner::new(SpreadConfig::default());

        let hosts = vec![
            HostSpreadInfo::new(HostId::new(1), [0.0, 0.0, 0.0]),
            HostSpreadInfo::new(HostId::new(2), [20.0, 0.0, 0.0]),
        ];

        let zones = vec![(
            ContaminationZoneId::new(1),
            [0.0, 0.0, 0.0],
            5.0,
            StrainId::base(PathogenId::plague()),
            0.5,
        )];

        let exposures = planner.plan_zone_exposures(&hosts, &zones, 0);

        assert_eq!(exposures.len(), 1);
        assert_eq!(exposures[0].target_host.raw(), 1);
    }

    #[test]
    fn test_spread_plan_summary() {
        let exposures = vec![
            ExposureEvent::direct_contact(
                HostId::new(1),
                HostId::new(10),
                StrainId::base(PathogenId::plague()),
                0.5,
                0,
            ),
            ExposureEvent::proximity(
                HostId::new(2),
                HostId::new(10),
                StrainId::base(PathogenId::plague()),
                0.3,
                5.0,
                0,
            ),
            ExposureEvent::from_zone(
                HostId::new(1),
                ContaminationZoneId::new(1),
                StrainId::base(PathogenId::fever()),
                0.4,
                0,
            ),
        ];

        let mut summary = SpreadPlanSummary::default();
        summary.update_from_exposures(&exposures);

        assert_eq!(summary.total_exposures, 3);
        assert_eq!(summary.direct_contact_exposures, 1);
        assert_eq!(summary.proximity_exposures, 1);
        assert_eq!(summary.zone_exposures, 1);
        assert_eq!(summary.unique_hosts_exposed, 2);
        assert!((summary.max_transmission_probability - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cross_region_spread() {
        let mut cross = CrossRegionSpread::new();

        cross.add_route(
            DiseaseRegionId::new("region1"),
            DiseaseRegionId::new("region2"),
            0.5,
        );

        cross.activate_strain(
            &DiseaseRegionId::new("region1"),
            &DiseaseRegionId::new("region2"),
            StrainId::base(PathogenId::plague()),
            0.3,
        );

        let routes: Vec<_> = cross
            .routes_from(&DiseaseRegionId::new("region1"))
            .collect();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].active_strains.len(), 1);
    }

    #[test]
    fn test_distance_3d() {
        let d = distance_3d([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]);
        assert!((d - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_exposure_event() {
        let event = ExposureEvent::direct_contact(
            HostId::new(1),
            HostId::new(2),
            StrainId::base(PathogenId::plague()),
            0.5,
            100,
        );

        let json = serde_json::to_string(&event).unwrap();
        let restored: ExposureEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.target_host, restored.target_host);
        assert_eq!(event.tick, restored.tick);
    }

    #[test]
    fn test_serde_spread_plan_summary() {
        let summary = SpreadPlanSummary {
            total_exposures: 10,
            direct_contact_exposures: 5,
            proximity_exposures: 3,
            zone_exposures: 2,
            unique_hosts_exposed: 8,
            avg_transmission_probability: 0.4,
            max_transmission_probability: 0.7,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: SpreadPlanSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(summary, restored);
    }
}

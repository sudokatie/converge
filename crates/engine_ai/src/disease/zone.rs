//! Contamination zones and environmental reservoirs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::ids::{ContaminationZoneId, DiseaseRegionId, PathogenId, StrainId};
use super::pathogen::PathogenTraits;

/// Type of contamination source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ContaminationSource {
    /// Contaminated corpse or remains.
    #[default]
    Corpse,
    /// Infected host shedding pathogen.
    InfectedHost,
    /// Environmental reservoir (water, soil).
    Environmental,
    /// Spore colony or growth.
    SporeColony,
    /// Contaminated object or surface.
    Fomite,
    /// Natural outbreak source.
    NaturalOutbreak,
}

impl ContaminationSource {
    #[must_use]
    pub fn as_index(self) -> u8 {
        match self {
            Self::Corpse => 0,
            Self::InfectedHost => 1,
            Self::Environmental => 2,
            Self::SporeColony => 3,
            Self::Fomite => 4,
            Self::NaturalOutbreak => 5,
        }
    }
}

/// A reservoir of pathogen contamination.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathogenReservoir {
    /// Strain in this reservoir.
    pub strain: StrainId,
    /// Pathogen concentration (0.0 to 1.0+).
    pub concentration: f32,
    /// Tick when contamination started.
    pub start_tick: u64,
    /// Tick when contamination will decay to zero.
    pub decay_end_tick: u64,
    /// Source of contamination.
    pub source: ContaminationSource,
    /// Effective traits for this reservoir's strain.
    pub traits: PathogenTraits,
}

impl PathogenReservoir {
    #[must_use]
    pub fn new(
        strain: StrainId,
        concentration: f32,
        traits: PathogenTraits,
        tick: u64,
        source: ContaminationSource,
    ) -> Self {
        let persistence = traits.environmental_persistence;
        Self {
            strain,
            concentration: concentration.clamp(0.0, 10.0),
            start_tick: tick,
            decay_end_tick: tick + persistence,
            source,
            traits,
        }
    }

    /// Age of this reservoir.
    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.start_tick)
    }

    /// Whether reservoir has fully decayed.
    #[must_use]
    pub fn is_decayed(&self, current_tick: u64) -> bool {
        current_tick >= self.decay_end_tick || self.concentration <= 0.001
    }

    /// Current effective concentration (accounting for decay).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "tick values bounded")]
    pub fn effective_concentration(&self, current_tick: u64) -> f32 {
        if current_tick >= self.decay_end_tick {
            return 0.0;
        }
        let total_duration = self.decay_end_tick.saturating_sub(self.start_tick);
        let remaining = self.decay_end_tick.saturating_sub(current_tick);
        let decay_factor = remaining as f32 / total_duration as f32;
        self.concentration * decay_factor
    }

    /// Exposure risk for contacts.
    #[must_use]
    pub fn exposure_risk(&self, current_tick: u64) -> f32 {
        let conc = self.effective_concentration(current_tick);
        let base_risk = self.traits.transmissibility;
        (conc * base_risk).clamp(0.0, 1.0)
    }

    /// Apply decay for a tick.
    pub fn apply_decay(&mut self, current_tick: u64) {
        if current_tick >= self.decay_end_tick {
            self.concentration = 0.0;
        }
    }

    /// Boost concentration (e.g., from additional contamination).
    pub fn boost(&mut self, amount: f32, tick: u64) {
        self.concentration = (self.concentration + amount).min(10.0);
        let persistence = self.traits.environmental_persistence;
        self.decay_end_tick = tick + persistence;
    }
}

/// A contamination zone with multiple potential pathogens.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContaminationZone {
    /// Zone identifier.
    pub id: ContaminationZoneId,
    /// Region this zone is in.
    pub region_id: DiseaseRegionId,
    /// Position (x, y, z) center of zone.
    pub position: [f32; 3],
    /// Radius of contamination.
    pub radius: f32,
    /// Active pathogen reservoirs.
    reservoirs: BTreeMap<PathogenId, PathogenReservoir>,
    /// Tick when zone was created.
    pub created_tick: u64,
    /// Whether zone is currently active.
    pub active: bool,
}

impl ContaminationZone {
    #[must_use]
    pub fn new(
        id: ContaminationZoneId,
        region_id: DiseaseRegionId,
        position: [f32; 3],
        tick: u64,
    ) -> Self {
        Self {
            id,
            region_id,
            position,
            radius: 5.0,
            reservoirs: BTreeMap::new(),
            created_tick: tick,
            active: true,
        }
    }

    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.1);
        self
    }

    /// Add or boost a pathogen reservoir.
    pub fn contaminate(
        &mut self,
        strain: StrainId,
        concentration: f32,
        traits: PathogenTraits,
        tick: u64,
        source: ContaminationSource,
    ) {
        let pathogen_id = strain.pathogen.clone();
        if let Some(existing) = self.reservoirs.get_mut(&pathogen_id) {
            existing.boost(concentration, tick);
        } else {
            let reservoir = PathogenReservoir::new(strain, concentration, traits, tick, source);
            self.reservoirs.insert(pathogen_id, reservoir);
        }
    }

    /// Get reservoir for a pathogen.
    #[must_use]
    pub fn get_reservoir(&self, pathogen_id: &PathogenId) -> Option<&PathogenReservoir> {
        self.reservoirs.get(pathogen_id)
    }

    /// Iterate over active reservoirs.
    pub fn reservoirs(&self) -> impl Iterator<Item = &PathogenReservoir> {
        self.reservoirs.values()
    }

    /// Number of active reservoirs.
    #[must_use]
    pub fn reservoir_count(&self) -> usize {
        self.reservoirs.len()
    }

    /// Check if zone has any active contamination.
    #[must_use]
    pub fn is_contaminated(&self, current_tick: u64) -> bool {
        self.reservoirs
            .values()
            .any(|r| !r.is_decayed(current_tick))
    }

    /// Get highest exposure risk across all pathogens.
    #[must_use]
    pub fn max_exposure_risk(&self, current_tick: u64) -> f32 {
        self.reservoirs
            .values()
            .map(|r| r.exposure_risk(current_tick))
            .fold(0.0f32, f32::max)
    }

    /// Get exposure risk for a specific pathogen.
    #[must_use]
    pub fn exposure_risk_for(&self, pathogen_id: &PathogenId, current_tick: u64) -> f32 {
        self.reservoirs
            .get(pathogen_id)
            .map_or(0.0, |r| r.exposure_risk(current_tick))
    }

    /// Check if a position is within this zone.
    #[must_use]
    pub fn contains_position(&self, pos: [f32; 3]) -> bool {
        let dx = pos[0] - self.position[0];
        let dy = pos[1] - self.position[1];
        let dz = pos[2] - self.position[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        dist_sq <= self.radius * self.radius
    }

    /// Distance from zone center to a position.
    #[must_use]
    pub fn distance_to(&self, pos: [f32; 3]) -> f32 {
        let dx = pos[0] - self.position[0];
        let dy = pos[1] - self.position[1];
        let dz = pos[2] - self.position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Process decay for all reservoirs.
    pub fn tick(&mut self, current_tick: u64) {
        for reservoir in self.reservoirs.values_mut() {
            reservoir.apply_decay(current_tick);
        }
        self.reservoirs.retain(|_, r| !r.is_decayed(current_tick));
        if self.reservoirs.is_empty() {
            self.active = false;
        }
    }

    /// Compute stable checksum.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(self.region_id.as_str().as_bytes());
        hasher.update(&self.position[0].to_le_bytes());
        hasher.update(&self.position[1].to_le_bytes());
        hasher.update(&self.position[2].to_le_bytes());
        hasher.update(&self.radius.to_le_bytes());
        hasher.update(&(self.reservoirs.len() as u32).to_le_bytes());
        for (id, reservoir) in &self.reservoirs {
            hasher.update(id.as_str().as_bytes());
            hasher.update(&reservoir.concentration.to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Registry of all contamination zones.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContaminationRegistry {
    zones: BTreeMap<ContaminationZoneId, ContaminationZone>,
    by_region: BTreeMap<String, Vec<ContaminationZoneId>>,
    next_zone_id: u64,
}

impl ContaminationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new contamination zone.
    pub fn create_zone(
        &mut self,
        region_id: &DiseaseRegionId,
        position: [f32; 3],
        radius: f32,
        tick: u64,
    ) -> ContaminationZoneId {
        let id = ContaminationZoneId::new(self.next_zone_id);
        self.next_zone_id += 1;

        let zone = ContaminationZone::new(id.clone(), region_id.clone(), position, tick)
            .with_radius(radius);

        self.by_region
            .entry(region_id.as_str().to_string())
            .or_default()
            .push(id.clone());
        self.zones.insert(id.clone(), zone);

        id
    }

    /// Get a zone by ID.
    #[must_use]
    pub fn get(&self, id: &ContaminationZoneId) -> Option<&ContaminationZone> {
        self.zones.get(id)
    }

    /// Get a mutable zone by ID.
    pub fn get_mut(&mut self, id: &ContaminationZoneId) -> Option<&mut ContaminationZone> {
        self.zones.get_mut(id)
    }

    /// Remove a zone.
    pub fn remove(&mut self, id: &ContaminationZoneId) -> Option<ContaminationZone> {
        if let Some(zone) = self.zones.remove(id) {
            if let Some(region_zones) = self.by_region.get_mut(zone.region_id.as_str()) {
                region_zones.retain(|z| z != id);
            }
            Some(zone)
        } else {
            None
        }
    }

    /// Get zones in a region.
    pub fn zones_in_region(
        &self,
        region_id: &DiseaseRegionId,
    ) -> impl Iterator<Item = &ContaminationZone> {
        self.by_region
            .get(region_id.as_str())
            .into_iter()
            .flatten()
            .filter_map(|id| self.zones.get(id))
    }

    /// Get zones containing a position.
    pub fn zones_at_position(
        &self,
        region_id: &DiseaseRegionId,
        position: [f32; 3],
    ) -> impl Iterator<Item = &ContaminationZone> {
        self.zones_in_region(region_id)
            .filter(move |z| z.contains_position(position))
    }

    /// Get zones within range of a position.
    pub fn zones_in_range(
        &self,
        region_id: &DiseaseRegionId,
        position: [f32; 3],
        range: f32,
    ) -> impl Iterator<Item = &ContaminationZone> {
        self.zones_in_region(region_id)
            .filter(move |z| z.distance_to(position) <= range + z.radius)
    }

    /// Total number of zones.
    #[must_use]
    pub fn len(&self) -> usize {
        self.zones.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Iterate over all zones.
    pub fn iter(&self) -> impl Iterator<Item = (&ContaminationZoneId, &ContaminationZone)> {
        self.zones.iter()
    }

    /// Process tick for all zones.
    pub fn tick(&mut self, current_tick: u64) {
        let mut to_remove = Vec::new();

        for (id, zone) in &mut self.zones {
            zone.tick(current_tick);
            if !zone.active {
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.remove(&id);
        }
    }

    /// Get total contamination level across all zones in a region.
    #[must_use]
    pub fn region_contamination_level(&self, region_id: &DiseaseRegionId, tick: u64) -> f32 {
        self.zones_in_region(region_id)
            .map(|z| z.max_exposure_risk(tick))
            .sum()
    }

    /// Rebuild the region index after deserialization.
    pub fn rebuild_index(&mut self) {
        self.by_region.clear();
        for (id, zone) in &self.zones {
            self.by_region
                .entry(zone.region_id.as_str().to_string())
                .or_default()
                .push(id.clone());
        }
    }

    /// Compute stable checksum.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&(self.zones.len() as u32).to_le_bytes());
        for zone in self.zones.values() {
            hasher.update(&zone.checksum().to_le_bytes());
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_traits() -> PathogenTraits {
        PathogenTraits::default()
            .with_transmissibility(0.5)
            .with_environmental_persistence(100)
    }

    #[test]
    fn test_contamination_source() {
        assert_eq!(ContaminationSource::Corpse.as_index(), 0);
        assert_eq!(ContaminationSource::SporeColony.as_index(), 3);
    }

    #[test]
    fn test_pathogen_reservoir_new() {
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();
        let reservoir = PathogenReservoir::new(
            strain.clone(),
            0.8,
            traits,
            100,
            ContaminationSource::Corpse,
        );

        assert_eq!(reservoir.strain, strain);
        assert!((reservoir.concentration - 0.8).abs() < f32::EPSILON);
        assert_eq!(reservoir.decay_end_tick, 200);
    }

    #[test]
    fn test_pathogen_reservoir_decay() {
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();
        let reservoir = PathogenReservoir::new(strain, 1.0, traits, 0, ContaminationSource::Corpse);

        assert!(!reservoir.is_decayed(50));
        assert!(reservoir.is_decayed(100));

        let conc_50 = reservoir.effective_concentration(50);
        assert!((conc_50 - 0.5).abs() < f32::EPSILON);

        let conc_100 = reservoir.effective_concentration(100);
        assert!(conc_100.abs() < f32::EPSILON);
    }

    #[test]
    fn test_pathogen_reservoir_boost() {
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();
        let mut reservoir =
            PathogenReservoir::new(strain, 0.5, traits, 0, ContaminationSource::Corpse);

        reservoir.boost(0.3, 50);

        assert!((reservoir.concentration - 0.8).abs() < f32::EPSILON);
        assert_eq!(reservoir.decay_end_tick, 150);
    }

    #[test]
    fn test_pathogen_reservoir_exposure_risk() {
        let strain = StrainId::base(PathogenId::plague());
        let traits = make_test_traits();
        let reservoir = PathogenReservoir::new(strain, 1.0, traits, 0, ContaminationSource::Corpse);

        let risk = reservoir.exposure_risk(0);
        assert!((risk - 0.5).abs() < f32::EPSILON);

        let risk_50 = reservoir.exposure_risk(50);
        assert!((risk_50 - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_contamination_zone_new() {
        let zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [10.0, 20.0, 0.0],
            0,
        )
        .with_radius(10.0);

        assert_eq!(zone.id.raw(), 1);
        assert!((zone.radius - 10.0).abs() < f32::EPSILON);
        assert!(zone.active);
    }

    #[test]
    fn test_contamination_zone_contaminate() {
        let mut zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        );

        zone.contaminate(
            StrainId::base(PathogenId::plague()),
            0.8,
            make_test_traits(),
            0,
            ContaminationSource::Corpse,
        );

        assert!(zone.is_contaminated(0));
        assert_eq!(zone.reservoir_count(), 1);
        assert!(zone.get_reservoir(&PathogenId::plague()).is_some());
    }

    #[test]
    fn test_contamination_zone_boost_existing() {
        let mut zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        );

        zone.contaminate(
            StrainId::base(PathogenId::plague()),
            0.5,
            make_test_traits(),
            0,
            ContaminationSource::Corpse,
        );

        zone.contaminate(
            StrainId::base(PathogenId::plague()),
            0.3,
            make_test_traits(),
            10,
            ContaminationSource::InfectedHost,
        );

        assert_eq!(zone.reservoir_count(), 1);
        let reservoir = zone.get_reservoir(&PathogenId::plague()).unwrap();
        assert!((reservoir.concentration - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_contamination_zone_contains_position() {
        let zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        )
        .with_radius(5.0);

        assert!(zone.contains_position([0.0, 0.0, 0.0]));
        assert!(zone.contains_position([3.0, 3.0, 0.0]));
        assert!(!zone.contains_position([10.0, 0.0, 0.0]));
    }

    #[test]
    fn test_contamination_zone_distance() {
        let zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        );

        let dist = zone.distance_to([3.0, 4.0, 0.0]);
        assert!((dist - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_contamination_zone_tick_decay() {
        let mut zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        );

        zone.contaminate(
            StrainId::base(PathogenId::plague()),
            1.0,
            make_test_traits(),
            0,
            ContaminationSource::Corpse,
        );

        assert!(zone.active);

        zone.tick(150);

        assert!(!zone.active);
        assert_eq!(zone.reservoir_count(), 0);
    }

    #[test]
    fn test_contamination_registry_create() {
        let mut registry = ContaminationRegistry::new();

        let id = registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);

        assert_eq!(registry.len(), 1);
        assert!(registry.get(&id).is_some());
    }

    #[test]
    fn test_contamination_registry_zones_in_region() {
        let mut registry = ContaminationRegistry::new();

        registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);
        registry.create_zone(&DiseaseRegionId::new("region1"), [10.0, 0.0, 0.0], 5.0, 0);
        registry.create_zone(&DiseaseRegionId::new("region2"), [0.0, 0.0, 0.0], 5.0, 0);

        let r1_zones: Vec<_> = registry
            .zones_in_region(&DiseaseRegionId::new("region1"))
            .collect();
        assert_eq!(r1_zones.len(), 2);

        let r2_zones: Vec<_> = registry
            .zones_in_region(&DiseaseRegionId::new("region2"))
            .collect();
        assert_eq!(r2_zones.len(), 1);
    }

    #[test]
    fn test_contamination_registry_zones_at_position() {
        let mut registry = ContaminationRegistry::new();

        let id1 = registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);
        let id2 = registry.create_zone(&DiseaseRegionId::new("region1"), [3.0, 0.0, 0.0], 5.0, 0);
        let _id3 = registry.create_zone(&DiseaseRegionId::new("region1"), [20.0, 0.0, 0.0], 5.0, 0);

        let zones: Vec<_> = registry
            .zones_at_position(&DiseaseRegionId::new("region1"), [1.0, 0.0, 0.0])
            .collect();

        assert_eq!(zones.len(), 2);
        assert!(zones.iter().any(|z| z.id == id1));
        assert!(zones.iter().any(|z| z.id == id2));
    }

    #[test]
    fn test_contamination_registry_remove() {
        let mut registry = ContaminationRegistry::new();

        let id = registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);

        let removed = registry.remove(&id);
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_contamination_registry_tick() {
        let mut registry = ContaminationRegistry::new();

        let id = registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);

        if let Some(zone) = registry.get_mut(&id) {
            zone.contaminate(
                StrainId::base(PathogenId::plague()),
                1.0,
                make_test_traits(),
                0,
                ContaminationSource::Corpse,
            );
        }

        assert_eq!(registry.len(), 1);

        registry.tick(150);

        assert!(registry.is_empty());
    }

    #[test]
    fn test_contamination_registry_rebuild_index() {
        let mut registry = ContaminationRegistry::new();
        registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);

        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: ContaminationRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();

        let zones: Vec<_> = restored
            .zones_in_region(&DiseaseRegionId::new("region1"))
            .collect();
        assert_eq!(zones.len(), 1);
    }

    #[test]
    fn test_contamination_zone_checksum() {
        let mut zone1 = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        );
        let mut zone2 = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [0.0, 0.0, 0.0],
            0,
        );

        zone1.contaminate(
            StrainId::base(PathogenId::plague()),
            0.5,
            make_test_traits(),
            0,
            ContaminationSource::Corpse,
        );
        zone2.contaminate(
            StrainId::base(PathogenId::plague()),
            0.5,
            make_test_traits(),
            0,
            ContaminationSource::Corpse,
        );

        assert_eq!(zone1.checksum(), zone2.checksum());
    }

    #[test]
    fn test_serde_pathogen_reservoir() {
        let strain = StrainId::base(PathogenId::plague());
        let reservoir = PathogenReservoir::new(
            strain,
            0.8,
            make_test_traits(),
            100,
            ContaminationSource::SporeColony,
        );

        let json = serde_json::to_string(&reservoir).unwrap();
        let restored: PathogenReservoir = serde_json::from_str(&json).unwrap();

        assert!((reservoir.concentration - restored.concentration).abs() < f32::EPSILON);
        assert_eq!(reservoir.source, restored.source);
    }

    #[test]
    fn test_serde_contamination_zone() {
        let mut zone = ContaminationZone::new(
            ContaminationZoneId::new(1),
            DiseaseRegionId::new("region1"),
            [10.0, 20.0, 30.0],
            0,
        )
        .with_radius(15.0);

        zone.contaminate(
            StrainId::base(PathogenId::plague()),
            0.7,
            make_test_traits(),
            0,
            ContaminationSource::Corpse,
        );

        let json = serde_json::to_string(&zone).unwrap();
        let restored: ContaminationZone = serde_json::from_str(&json).unwrap();

        assert_eq!(zone.id, restored.id);
        assert!((zone.radius - restored.radius).abs() < f32::EPSILON);
        assert_eq!(zone.reservoir_count(), restored.reservoir_count());
    }

    #[test]
    fn test_serde_contamination_registry() {
        let mut registry = ContaminationRegistry::new();

        let id = registry.create_zone(&DiseaseRegionId::new("region1"), [0.0, 0.0, 0.0], 5.0, 0);

        if let Some(zone) = registry.get_mut(&id) {
            zone.contaminate(
                StrainId::base(PathogenId::plague()),
                0.8,
                make_test_traits(),
                0,
                ContaminationSource::Corpse,
            );
        }

        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: ContaminationRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();

        assert_eq!(registry.len(), restored.len());
    }
}

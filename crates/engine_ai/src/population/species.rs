//! Species and group population caps.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identifier for a species cap.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesCapId(pub String);

impl SpeciesCapId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for SpeciesCapId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Identifier for a group cap.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupCapId(pub String);

impl GroupCapId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for GroupCapId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Population cap for a species type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesCap {
    /// Species identifier.
    pub id: SpeciesCapId,
    /// Display name.
    pub name: String,
    /// Minimum population floor.
    pub minimum: u32,
    /// Soft cap (spawn pressure reduced above this).
    pub soft_cap: u32,
    /// Hard cap (no spawns above this).
    pub hard_cap: u32,
    /// Current population count.
    current: u32,
    /// Whether this species is hostile.
    pub hostile: bool,
    /// Spawn weight relative to other species.
    pub spawn_weight: f32,
    /// Groups this species belongs to.
    groups: Vec<GroupCapId>,
}

impl SpeciesCap {
    /// Create a new species cap.
    #[must_use]
    pub fn new(id: SpeciesCapId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            minimum: 0,
            soft_cap: 50,
            hard_cap: 100,
            current: 0,
            hostile: false,
            spawn_weight: 1.0,
            groups: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_caps(mut self, minimum: u32, soft_cap: u32, hard_cap: u32) -> Self {
        self.minimum = minimum;
        self.soft_cap = soft_cap;
        self.hard_cap = hard_cap;
        self
    }

    #[must_use]
    pub fn with_hostile(mut self, hostile: bool) -> Self {
        self.hostile = hostile;
        self
    }

    #[must_use]
    pub fn with_spawn_weight(mut self, weight: f32) -> Self {
        self.spawn_weight = weight.max(0.0);
        self
    }

    #[must_use]
    pub fn with_group(mut self, group: GroupCapId) -> Self {
        if !self.groups.contains(&group) {
            self.groups.push(group);
            self.groups.sort();
        }
        self
    }

    /// Get current population.
    #[must_use]
    pub fn current(&self) -> u32 {
        self.current
    }

    /// Set current population.
    pub fn set_current(&mut self, count: u32) {
        self.current = count;
    }

    /// Increment population.
    pub fn increment(&mut self) {
        self.current = self.current.saturating_add(1);
    }

    /// Decrement population.
    pub fn decrement(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    /// Check if at hard cap.
    #[must_use]
    pub fn at_hard_cap(&self) -> bool {
        self.current >= self.hard_cap
    }

    /// Check if above soft cap.
    #[must_use]
    pub fn above_soft_cap(&self) -> bool {
        self.current > self.soft_cap
    }

    /// Check if below minimum.
    #[must_use]
    pub fn below_minimum(&self) -> bool {
        self.current < self.minimum
    }

    /// Get spawn pressure modifier based on current population.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "population counts are bounded")]
    pub fn spawn_pressure_modifier(&self) -> f32 {
        if self.current < self.minimum {
            2.0
        } else if self.current >= self.hard_cap {
            0.0
        } else if self.current > self.soft_cap {
            let excess = (self.current - self.soft_cap) as f32;
            let range = (self.hard_cap - self.soft_cap) as f32;
            if range > 0.0 {
                1.0 - (excess / range)
            } else {
                0.0
            }
        } else {
            1.0
        }
    }

    /// Get deficit from minimum.
    #[must_use]
    pub fn deficit(&self) -> u32 {
        self.minimum.saturating_sub(self.current)
    }

    /// Get excess over soft cap.
    #[must_use]
    pub fn excess(&self) -> u32 {
        self.current.saturating_sub(self.soft_cap)
    }

    /// Get groups this species belongs to.
    pub fn groups(&self) -> &[GroupCapId] {
        &self.groups
    }
}

/// Population cap for a group of species.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupCap {
    /// Group identifier.
    pub id: GroupCapId,
    /// Display name.
    pub name: String,
    /// Soft cap for the entire group.
    pub soft_cap: u32,
    /// Hard cap for the entire group.
    pub hard_cap: u32,
}

impl GroupCap {
    /// Create a new group cap.
    #[must_use]
    pub fn new(id: GroupCapId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            soft_cap: 200,
            hard_cap: 400,
        }
    }

    #[must_use]
    pub fn with_caps(mut self, soft_cap: u32, hard_cap: u32) -> Self {
        self.soft_cap = soft_cap;
        self.hard_cap = hard_cap;
        self
    }
}

/// Registry managing species and group caps.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpeciesRegistry {
    species: BTreeMap<SpeciesCapId, SpeciesCap>,
    groups: BTreeMap<GroupCapId, GroupCap>,
}

impl SpeciesRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a species cap.
    pub fn register_species(&mut self, cap: SpeciesCap) {
        self.species.insert(cap.id.clone(), cap);
    }

    /// Register a group cap.
    pub fn register_group(&mut self, cap: GroupCap) {
        self.groups.insert(cap.id.clone(), cap);
    }

    /// Get a species cap.
    #[must_use]
    pub fn get_species(&self, id: &SpeciesCapId) -> Option<&SpeciesCap> {
        self.species.get(id)
    }

    /// Get a mutable species cap.
    pub fn get_species_mut(&mut self, id: &SpeciesCapId) -> Option<&mut SpeciesCap> {
        self.species.get_mut(id)
    }

    /// Get a group cap.
    #[must_use]
    pub fn get_group(&self, id: &GroupCapId) -> Option<&GroupCap> {
        self.groups.get(id)
    }

    /// Calculate total population for a group.
    #[must_use]
    pub fn group_population(&self, group_id: &GroupCapId) -> u32 {
        self.species
            .values()
            .filter(|s| s.groups.contains(group_id))
            .map(|s| s.current)
            .sum()
    }

    /// Check if group is at hard cap.
    #[must_use]
    pub fn group_at_hard_cap(&self, group_id: &GroupCapId) -> bool {
        self.groups
            .get(group_id)
            .is_some_and(|g| self.group_population(group_id) >= g.hard_cap)
    }

    /// Check if group is above soft cap.
    #[must_use]
    pub fn group_above_soft_cap(&self, group_id: &GroupCapId) -> bool {
        self.groups
            .get(group_id)
            .is_some_and(|g| self.group_population(group_id) > g.soft_cap)
    }

    /// Get spawn pressure modifier for a group.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "population counts are bounded")]
    pub fn group_spawn_pressure(&self, group_id: &GroupCapId) -> f32 {
        let Some(group) = self.groups.get(group_id) else {
            return 1.0;
        };

        let population = self.group_population(group_id);
        if population >= group.hard_cap {
            0.0
        } else if population > group.soft_cap {
            let excess = (population - group.soft_cap) as f32;
            let range = (group.hard_cap - group.soft_cap) as f32;
            if range > 0.0 {
                1.0 - (excess / range)
            } else {
                0.0
            }
        } else {
            1.0
        }
    }

    /// Check if a species can spawn considering its caps and group caps.
    #[must_use]
    pub fn can_spawn(&self, species_id: &SpeciesCapId) -> bool {
        let Some(species) = self.species.get(species_id) else {
            return false;
        };

        if species.at_hard_cap() {
            return false;
        }

        for group_id in &species.groups {
            if self.group_at_hard_cap(group_id) {
                return false;
            }
        }

        true
    }

    /// Get effective spawn weight for a species.
    #[must_use]
    pub fn effective_spawn_weight(&self, species_id: &SpeciesCapId) -> f32 {
        let Some(species) = self.species.get(species_id) else {
            return 0.0;
        };

        let mut weight = species.spawn_weight * species.spawn_pressure_modifier();

        for group_id in &species.groups {
            weight *= self.group_spawn_pressure(group_id);
        }

        weight
    }

    /// Get all species below minimum.
    pub fn species_below_minimum(&self) -> impl Iterator<Item = &SpeciesCap> {
        self.species.values().filter(|s| s.below_minimum())
    }

    /// Get all species above soft cap.
    pub fn species_above_soft_cap(&self) -> impl Iterator<Item = &SpeciesCap> {
        self.species.values().filter(|s| s.above_soft_cap())
    }

    /// Get total population across all species.
    #[must_use]
    pub fn total_population(&self) -> u32 {
        self.species.values().map(SpeciesCap::current).sum()
    }

    /// Get total hostile population.
    #[must_use]
    pub fn hostile_population(&self) -> u32 {
        self.species
            .values()
            .filter(|s| s.hostile)
            .map(SpeciesCap::current)
            .sum()
    }

    /// Get total passive population.
    #[must_use]
    pub fn passive_population(&self) -> u32 {
        self.species
            .values()
            .filter(|s| !s.hostile)
            .map(SpeciesCap::current)
            .sum()
    }

    /// Iterate over all species (deterministic order).
    pub fn iter_species(&self) -> impl Iterator<Item = &SpeciesCap> {
        self.species.values()
    }

    /// Get species IDs (deterministic order).
    pub fn species_ids(&self) -> impl Iterator<Item = &SpeciesCapId> {
        self.species.keys()
    }

    /// Number of registered species.
    #[must_use]
    pub fn species_count(&self) -> usize {
        self.species.len()
    }

    /// Number of registered groups.
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_species(id: &str) -> SpeciesCap {
        SpeciesCap::new(SpeciesCapId::new(id), id)
    }

    #[test]
    fn test_species_cap_id() {
        let id = SpeciesCapId::new("wolf");
        assert_eq!(id.as_str(), "wolf");

        let id2: SpeciesCapId = "deer".into();
        assert_eq!(id2.as_str(), "deer");
    }

    #[test]
    fn test_species_cap_new() {
        let cap = make_species("wolf");

        assert_eq!(cap.id.as_str(), "wolf");
        assert_eq!(cap.current(), 0);
        assert!(!cap.at_hard_cap());
        assert!(!cap.above_soft_cap());
    }

    #[test]
    fn test_species_cap_with_caps() {
        let cap = make_species("wolf").with_caps(5, 20, 40);

        assert_eq!(cap.minimum, 5);
        assert_eq!(cap.soft_cap, 20);
        assert_eq!(cap.hard_cap, 40);
    }

    #[test]
    fn test_species_cap_increment_decrement() {
        let mut cap = make_species("wolf");

        cap.increment();
        assert_eq!(cap.current(), 1);

        cap.increment();
        assert_eq!(cap.current(), 2);

        cap.decrement();
        assert_eq!(cap.current(), 1);
    }

    #[test]
    fn test_species_cap_bounds() {
        let mut cap = make_species("wolf").with_caps(5, 50, 100);
        cap.set_current(3);

        assert!(cap.below_minimum());
        assert!(!cap.above_soft_cap());
        assert!(!cap.at_hard_cap());

        cap.set_current(60);
        assert!(!cap.below_minimum());
        assert!(cap.above_soft_cap());
        assert!(!cap.at_hard_cap());

        cap.set_current(100);
        assert!(cap.at_hard_cap());
    }

    #[test]
    fn test_species_spawn_pressure_modifier() {
        let mut cap = make_species("wolf").with_caps(10, 50, 100);

        cap.set_current(5);
        assert!((cap.spawn_pressure_modifier() - 2.0).abs() < f32::EPSILON);

        cap.set_current(30);
        assert!((cap.spawn_pressure_modifier() - 1.0).abs() < f32::EPSILON);

        cap.set_current(75);
        let modifier = cap.spawn_pressure_modifier();
        assert!(modifier > 0.0 && modifier < 1.0);

        cap.set_current(100);
        assert!((cap.spawn_pressure_modifier()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_species_deficit_excess() {
        let mut cap = make_species("wolf").with_caps(10, 50, 100);

        cap.set_current(5);
        assert_eq!(cap.deficit(), 5);
        assert_eq!(cap.excess(), 0);

        cap.set_current(60);
        assert_eq!(cap.deficit(), 0);
        assert_eq!(cap.excess(), 10);
    }

    #[test]
    fn test_species_groups() {
        let cap = make_species("wolf")
            .with_group(GroupCapId::new("predators"))
            .with_group(GroupCapId::new("mammals"));

        assert_eq!(cap.groups().len(), 2);
    }

    #[test]
    fn test_group_cap() {
        let cap = GroupCap::new(GroupCapId::new("predators"), "Predators").with_caps(100, 200);

        assert_eq!(cap.id.as_str(), "predators");
        assert_eq!(cap.soft_cap, 100);
        assert_eq!(cap.hard_cap, 200);
    }

    #[test]
    fn test_registry_species() {
        let mut registry = SpeciesRegistry::new();

        registry.register_species(make_species("wolf"));
        registry.register_species(make_species("deer"));

        assert_eq!(registry.species_count(), 2);
        assert!(registry.get_species(&SpeciesCapId::new("wolf")).is_some());
    }

    #[test]
    fn test_registry_group_population() {
        let mut registry = SpeciesRegistry::new();

        registry.register_group(GroupCap::new(GroupCapId::new("predators"), "Predators"));

        let mut wolf = make_species("wolf").with_group(GroupCapId::new("predators"));
        wolf.set_current(10);
        registry.register_species(wolf);

        let mut bear = make_species("bear").with_group(GroupCapId::new("predators"));
        bear.set_current(5);
        registry.register_species(bear);

        assert_eq!(registry.group_population(&GroupCapId::new("predators")), 15);
    }

    #[test]
    fn test_registry_can_spawn() {
        let mut registry = SpeciesRegistry::new();

        registry.register_group(
            GroupCap::new(GroupCapId::new("predators"), "Predators").with_caps(10, 20),
        );

        let mut wolf = make_species("wolf")
            .with_caps(0, 50, 100)
            .with_group(GroupCapId::new("predators"));
        wolf.set_current(5);
        registry.register_species(wolf);

        assert!(registry.can_spawn(&SpeciesCapId::new("wolf")));

        if let Some(wolf) = registry.get_species_mut(&SpeciesCapId::new("wolf")) {
            wolf.set_current(20);
        }

        assert!(!registry.can_spawn(&SpeciesCapId::new("wolf")));
    }

    #[test]
    fn test_registry_effective_spawn_weight() {
        let mut registry = SpeciesRegistry::new();

        let wolf = make_species("wolf")
            .with_caps(0, 50, 100)
            .with_spawn_weight(2.0);
        registry.register_species(wolf);

        let weight = registry.effective_spawn_weight(&SpeciesCapId::new("wolf"));
        assert!((weight - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_registry_population_queries() {
        let mut registry = SpeciesRegistry::new();

        let mut wolf = make_species("wolf").with_hostile(true);
        wolf.set_current(10);
        registry.register_species(wolf);

        let mut deer = make_species("deer").with_hostile(false);
        deer.set_current(20);
        registry.register_species(deer);

        assert_eq!(registry.total_population(), 30);
        assert_eq!(registry.hostile_population(), 10);
        assert_eq!(registry.passive_population(), 20);
    }

    #[test]
    fn test_registry_deterministic_order() {
        let mut registry = SpeciesRegistry::new();

        registry.register_species(make_species("zebra"));
        registry.register_species(make_species("ant"));
        registry.register_species(make_species("moose"));

        let ids: Vec<_> = registry.species_ids().map(SpeciesCapId::as_str).collect();
        assert_eq!(ids, vec!["ant", "moose", "zebra"]);
    }

    #[test]
    fn test_serde_species_cap() {
        let cap = make_species("wolf")
            .with_caps(5, 50, 100)
            .with_hostile(true)
            .with_group(GroupCapId::new("predators"));

        let json = serde_json::to_string(&cap).unwrap();
        let restored: SpeciesCap = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, cap.id);
        assert_eq!(restored.hard_cap, 100);
        assert!(restored.hostile);
    }

    #[test]
    fn test_serde_registry() {
        let mut registry = SpeciesRegistry::new();
        registry.register_species(make_species("wolf"));
        registry.register_group(GroupCap::new(GroupCapId::new("predators"), "Predators"));

        let json = serde_json::to_string(&registry).unwrap();
        let restored: SpeciesRegistry = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.species_count(), 1);
        assert_eq!(restored.group_count(), 1);
    }
}

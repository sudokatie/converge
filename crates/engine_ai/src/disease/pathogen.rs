//! Pathogen definitions and trait configurations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::ids::PathogenId;

/// Pathogen traits controlling disease behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathogenTraits {
    /// Base probability of transmission per contact (0.0-1.0).
    pub transmissibility: f32,
    /// Damage/severity rate once symptomatic (per tick).
    pub virulence: f32,
    /// Duration of incubation period in ticks.
    pub incubation_duration: u64,
    /// Duration of symptomatic period before recovery/death in ticks.
    pub symptomatic_duration: u64,
    /// Duration of recovery period in ticks.
    pub recovery_duration: u64,
    /// Duration of immunity after recovery in ticks (0 = permanent immunity).
    pub immunity_duration: u64,
    /// How long the pathogen survives in the environment (ticks).
    pub environmental_persistence: u64,
    /// Base mutation rate per tick (0.0-1.0).
    pub mutation_rate: f32,
    /// Whether hosts can become carriers after recovery.
    pub can_become_carrier: bool,
    /// Probability of becoming a carrier (0.0-1.0).
    pub carrier_probability: f32,
    /// Whether this pathogen can remain latent.
    pub can_go_latent: bool,
    /// Probability of going latent instead of progressing (0.0-1.0).
    pub latency_probability: f32,
    /// Transmission distance multiplier (1.0 = contact only, higher = airborne).
    pub transmission_range: f32,
    /// Minimum population density for transmission.
    pub min_density_for_spread: f32,
    /// Lethality rate when critical (0.0-1.0).
    pub lethality: f32,
}

impl Default for PathogenTraits {
    fn default() -> Self {
        Self {
            transmissibility: 0.3,
            virulence: 0.1,
            incubation_duration: 100,
            symptomatic_duration: 200,
            recovery_duration: 100,
            immunity_duration: 0,
            environmental_persistence: 50,
            mutation_rate: 0.001,
            can_become_carrier: false,
            carrier_probability: 0.0,
            can_go_latent: false,
            latency_probability: 0.0,
            transmission_range: 1.0,
            min_density_for_spread: 0.0,
            lethality: 0.1,
        }
    }
}

impl PathogenTraits {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_transmissibility(mut self, value: f32) -> Self {
        self.transmissibility = value.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_virulence(mut self, value: f32) -> Self {
        self.virulence = value.max(0.0);
        self
    }

    #[must_use]
    pub fn with_incubation(mut self, ticks: u64) -> Self {
        self.incubation_duration = ticks;
        self
    }

    #[must_use]
    pub fn with_symptomatic_duration(mut self, ticks: u64) -> Self {
        self.symptomatic_duration = ticks;
        self
    }

    #[must_use]
    pub fn with_recovery_duration(mut self, ticks: u64) -> Self {
        self.recovery_duration = ticks;
        self
    }

    #[must_use]
    pub fn with_immunity_duration(mut self, ticks: u64) -> Self {
        self.immunity_duration = ticks;
        self
    }

    #[must_use]
    pub fn with_environmental_persistence(mut self, ticks: u64) -> Self {
        self.environmental_persistence = ticks;
        self
    }

    #[must_use]
    pub fn with_mutation_rate(mut self, rate: f32) -> Self {
        self.mutation_rate = rate.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_carrier_behavior(mut self, can_become: bool, probability: f32) -> Self {
        self.can_become_carrier = can_become;
        self.carrier_probability = probability.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_latency(mut self, can_go_latent: bool, probability: f32) -> Self {
        self.can_go_latent = can_go_latent;
        self.latency_probability = probability.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_transmission_range(mut self, range: f32) -> Self {
        self.transmission_range = range.max(0.0);
        self
    }

    #[must_use]
    pub fn with_lethality(mut self, lethality: f32) -> Self {
        self.lethality = lethality.clamp(0.0, 1.0);
        self
    }

    /// Total disease duration from exposure to recovery.
    #[must_use]
    pub fn total_duration(&self) -> u64 {
        self.incubation_duration + self.symptomatic_duration + self.recovery_duration
    }

    /// Whether the pathogen is highly transmissible.
    #[must_use]
    pub fn is_highly_transmissible(&self) -> bool {
        self.transmissibility > 0.7
    }

    /// Whether the pathogen is highly lethal.
    #[must_use]
    pub fn is_highly_lethal(&self) -> bool {
        self.lethality > 0.5
    }

    /// Whether the pathogen is airborne.
    #[must_use]
    pub fn is_airborne(&self) -> bool {
        self.transmission_range > 2.0
    }

    /// Compute a stable fingerprint for these traits.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.transmissibility.to_le_bytes());
        hasher.update(&self.virulence.to_le_bytes());
        hasher.update(&self.incubation_duration.to_le_bytes());
        hasher.update(&self.symptomatic_duration.to_le_bytes());
        hasher.update(&self.recovery_duration.to_le_bytes());
        hasher.update(&self.immunity_duration.to_le_bytes());
        hasher.update(&self.environmental_persistence.to_le_bytes());
        hasher.update(&self.mutation_rate.to_le_bytes());
        hasher.update(&[u8::from(self.can_become_carrier)]);
        hasher.update(&self.carrier_probability.to_le_bytes());
        hasher.update(&[u8::from(self.can_go_latent)]);
        hasher.update(&self.latency_probability.to_le_bytes());
        hasher.update(&self.transmission_range.to_le_bytes());
        hasher.update(&self.lethality.to_le_bytes());
        hasher.finalize()
    }
}

/// Bounds for trait values during mutation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraitBounds {
    pub min_transmissibility: f32,
    pub max_transmissibility: f32,
    pub min_virulence: f32,
    pub max_virulence: f32,
    pub min_incubation: u64,
    pub max_incubation: u64,
    pub min_lethality: f32,
    pub max_lethality: f32,
}

impl Default for TraitBounds {
    fn default() -> Self {
        Self {
            min_transmissibility: 0.05,
            max_transmissibility: 0.95,
            min_virulence: 0.01,
            max_virulence: 1.0,
            min_incubation: 10,
            max_incubation: 1000,
            min_lethality: 0.0,
            max_lethality: 1.0,
        }
    }
}

impl TraitBounds {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clamp_traits(&self, traits: &mut PathogenTraits) {
        traits.transmissibility = traits
            .transmissibility
            .clamp(self.min_transmissibility, self.max_transmissibility);
        traits.virulence = traits
            .virulence
            .clamp(self.min_virulence, self.max_virulence);
        traits.incubation_duration = traits
            .incubation_duration
            .clamp(self.min_incubation, self.max_incubation);
        traits.lethality = traits
            .lethality
            .clamp(self.min_lethality, self.max_lethality);
    }
}

/// Category of pathogen for grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PathogenCategory {
    /// Bacterial infection.
    #[default]
    Bacterial,
    /// Viral infection.
    Viral,
    /// Fungal infection.
    Fungal,
    /// Parasitic infection.
    Parasitic,
    /// Prion disease.
    Prion,
    /// Magical/supernatural plague.
    Arcane,
}

impl PathogenCategory {
    #[must_use]
    pub fn as_index(self) -> u8 {
        match self {
            Self::Bacterial => 0,
            Self::Viral => 1,
            Self::Fungal => 2,
            Self::Parasitic => 3,
            Self::Prion => 4,
            Self::Arcane => 5,
        }
    }
}

/// Full pathogen definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathogenDef {
    /// Unique identifier.
    pub id: PathogenId,
    /// Display name.
    pub name: String,
    /// Description text.
    #[serde(default)]
    pub description: String,
    /// Category for grouping.
    #[serde(default)]
    pub category: PathogenCategory,
    /// Base traits (can be mutated from these).
    pub base_traits: PathogenTraits,
    /// Bounds for trait mutations.
    #[serde(default)]
    pub trait_bounds: TraitBounds,
    /// Species that can be infected.
    #[serde(default)]
    pub susceptible_species: Vec<String>,
    /// Species that are immune.
    #[serde(default)]
    pub immune_species: Vec<String>,
    /// Environmental conditions that boost spread.
    #[serde(default)]
    pub favorable_conditions: Vec<String>,
    /// Environmental conditions that inhibit spread.
    #[serde(default)]
    pub inhibiting_conditions: Vec<String>,
}

impl PathogenDef {
    #[must_use]
    pub fn new(id: impl Into<PathogenId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category: PathogenCategory::default(),
            base_traits: PathogenTraits::default(),
            trait_bounds: TraitBounds::default(),
            susceptible_species: Vec::new(),
            immune_species: Vec::new(),
            favorable_conditions: Vec::new(),
            inhibiting_conditions: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    #[must_use]
    pub fn with_category(mut self, category: PathogenCategory) -> Self {
        self.category = category;
        self
    }

    #[must_use]
    pub fn with_traits(mut self, traits: PathogenTraits) -> Self {
        self.base_traits = traits;
        self
    }

    #[must_use]
    pub fn with_bounds(mut self, bounds: TraitBounds) -> Self {
        self.trait_bounds = bounds;
        self
    }

    #[must_use]
    pub fn with_susceptible(mut self, species: impl Into<String>) -> Self {
        self.susceptible_species.push(species.into());
        self
    }

    #[must_use]
    pub fn with_immune(mut self, species: impl Into<String>) -> Self {
        self.immune_species.push(species.into());
        self
    }

    #[must_use]
    pub fn with_favorable_condition(mut self, condition: impl Into<String>) -> Self {
        self.favorable_conditions.push(condition.into());
        self
    }

    #[must_use]
    pub fn with_inhibiting_condition(mut self, condition: impl Into<String>) -> Self {
        self.inhibiting_conditions.push(condition.into());
        self
    }

    /// Check if a species is susceptible.
    #[must_use]
    pub fn is_susceptible(&self, species: &str) -> bool {
        if self.immune_species.iter().any(|s| s == species) {
            return false;
        }
        self.susceptible_species.is_empty() || self.susceptible_species.iter().any(|s| s == species)
    }

    /// Compute stable fingerprint.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(self.id.as_str().as_bytes());
        hasher.update(self.name.as_bytes());
        hasher.update(&self.category.as_index().to_le_bytes());
        hasher.update(&self.base_traits.fingerprint().to_le_bytes());
        hasher.update(&(self.susceptible_species.len() as u32).to_le_bytes());
        hasher.update(&(self.immune_species.len() as u32).to_le_bytes());
        hasher.finalize()
    }
}

/// Registry of pathogen definitions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PathogenRegistry {
    pathogens: BTreeMap<PathogenId, PathogenDef>,
    #[serde(skip)]
    by_category: BTreeMap<u8, Vec<PathogenId>>,
}

impl PathogenRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: PathogenDef) {
        let id = def.id.clone();
        let category = def.category.as_index();
        self.by_category
            .entry(category)
            .or_default()
            .push(id.clone());
        self.pathogens.insert(id, def);
    }

    #[must_use]
    pub fn get(&self, id: &PathogenId) -> Option<&PathogenDef> {
        self.pathogens.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathogenDef> {
        self.pathogens.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pathogens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pathogens.is_empty()
    }

    pub fn pathogens_by_category(
        &self,
        category: PathogenCategory,
    ) -> impl Iterator<Item = &PathogenId> {
        self.by_category
            .get(&category.as_index())
            .into_iter()
            .flatten()
    }

    pub fn rebuild_index(&mut self) {
        self.by_category.clear();
        for (id, def) in &self.pathogens {
            self.by_category
                .entry(def.category.as_index())
                .or_default()
                .push(id.clone());
        }
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        for def in self.pathogens.values() {
            hasher.update(&def.fingerprint().to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Built-in pathogen presets.
pub mod presets {
    use super::{PathogenCategory, PathogenDef, PathogenId, PathogenRegistry, PathogenTraits};

    #[must_use]
    pub fn plague() -> PathogenDef {
        PathogenDef::new(PathogenId::plague(), "Plague")
            .with_description("Highly contagious bacterial infection")
            .with_category(PathogenCategory::Bacterial)
            .with_traits(
                PathogenTraits::new()
                    .with_transmissibility(0.6)
                    .with_virulence(0.3)
                    .with_incubation(50)
                    .with_symptomatic_duration(300)
                    .with_recovery_duration(150)
                    .with_immunity_duration(5000)
                    .with_environmental_persistence(100)
                    .with_mutation_rate(0.005)
                    .with_lethality(0.4),
            )
    }

    #[must_use]
    pub fn blight() -> PathogenDef {
        PathogenDef::new(PathogenId::blight(), "Blight")
            .with_description("Plant-affecting fungal disease that can spread to humanoids")
            .with_category(PathogenCategory::Fungal)
            .with_traits(
                PathogenTraits::new()
                    .with_transmissibility(0.4)
                    .with_virulence(0.15)
                    .with_incubation(200)
                    .with_symptomatic_duration(500)
                    .with_recovery_duration(200)
                    .with_environmental_persistence(500)
                    .with_mutation_rate(0.002)
                    .with_carrier_behavior(true, 0.3)
                    .with_lethality(0.2),
            )
    }

    #[must_use]
    pub fn rot() -> PathogenDef {
        PathogenDef::new(PathogenId::rot(), "Rot")
            .with_description("Necrotic infection that spreads through wounds")
            .with_category(PathogenCategory::Bacterial)
            .with_traits(
                PathogenTraits::new()
                    .with_transmissibility(0.2)
                    .with_virulence(0.5)
                    .with_incubation(30)
                    .with_symptomatic_duration(200)
                    .with_recovery_duration(300)
                    .with_environmental_persistence(200)
                    .with_mutation_rate(0.001)
                    .with_lethality(0.6),
            )
    }

    #[must_use]
    pub fn spore_lung() -> PathogenDef {
        PathogenDef::new(PathogenId::spore_lung(), "Spore Lung")
            .with_description("Airborne fungal infection")
            .with_category(PathogenCategory::Fungal)
            .with_traits(
                PathogenTraits::new()
                    .with_transmissibility(0.5)
                    .with_virulence(0.2)
                    .with_incubation(100)
                    .with_symptomatic_duration(400)
                    .with_recovery_duration(100)
                    .with_environmental_persistence(300)
                    .with_mutation_rate(0.003)
                    .with_transmission_range(3.0)
                    .with_latency(true, 0.2)
                    .with_lethality(0.3),
            )
    }

    #[must_use]
    pub fn wasting() -> PathogenDef {
        PathogenDef::new(PathogenId::wasting(), "Wasting")
            .with_description("Slow-acting prion disease")
            .with_category(PathogenCategory::Prion)
            .with_traits(
                PathogenTraits::new()
                    .with_transmissibility(0.1)
                    .with_virulence(0.05)
                    .with_incubation(1000)
                    .with_symptomatic_duration(2000)
                    .with_recovery_duration(0)
                    .with_immunity_duration(0)
                    .with_environmental_persistence(1000)
                    .with_mutation_rate(0.0)
                    .with_lethality(0.95),
            )
    }

    #[must_use]
    pub fn fever() -> PathogenDef {
        PathogenDef::new(PathogenId::fever(), "Fever")
            .with_description("Common viral infection")
            .with_category(PathogenCategory::Viral)
            .with_traits(
                PathogenTraits::new()
                    .with_transmissibility(0.7)
                    .with_virulence(0.1)
                    .with_incubation(20)
                    .with_symptomatic_duration(100)
                    .with_recovery_duration(50)
                    .with_immunity_duration(2000)
                    .with_environmental_persistence(30)
                    .with_mutation_rate(0.01)
                    .with_lethality(0.05),
            )
    }

    #[must_use]
    pub fn create_preset_registry() -> PathogenRegistry {
        let mut registry = PathogenRegistry::new();
        registry.register(plague());
        registry.register(blight());
        registry.register(rot());
        registry.register(spore_lung());
        registry.register(wasting());
        registry.register(fever());
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PathogenCategory, PathogenDef, PathogenId, PathogenRegistry, PathogenTraits, TraitBounds,
        presets,
    };

    #[test]
    fn test_pathogen_traits_default() {
        let traits = PathogenTraits::default();
        assert!((traits.transmissibility - 0.3).abs() < f32::EPSILON);
        assert!(traits.incubation_duration > 0);
    }

    #[test]
    fn test_pathogen_traits_builder() {
        let traits = PathogenTraits::new()
            .with_transmissibility(0.8)
            .with_virulence(0.5)
            .with_incubation(50)
            .with_lethality(0.7);

        assert!((traits.transmissibility - 0.8).abs() < f32::EPSILON);
        assert!((traits.virulence - 0.5).abs() < f32::EPSILON);
        assert_eq!(traits.incubation_duration, 50);
        assert!((traits.lethality - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pathogen_traits_clamping() {
        let traits = PathogenTraits::new()
            .with_transmissibility(1.5)
            .with_lethality(-0.5);

        assert!((traits.transmissibility - 1.0).abs() < f32::EPSILON);
        assert!(traits.lethality.abs() < f32::EPSILON);
    }

    #[test]
    fn test_pathogen_traits_total_duration() {
        let traits = PathogenTraits::new()
            .with_incubation(100)
            .with_symptomatic_duration(200)
            .with_recovery_duration(50);

        assert_eq!(traits.total_duration(), 350);
    }

    #[test]
    fn test_pathogen_traits_classification() {
        let mild = PathogenTraits::new()
            .with_transmissibility(0.3)
            .with_lethality(0.1)
            .with_transmission_range(1.0);

        assert!(!mild.is_highly_transmissible());
        assert!(!mild.is_highly_lethal());
        assert!(!mild.is_airborne());

        let severe = PathogenTraits::new()
            .with_transmissibility(0.9)
            .with_lethality(0.8)
            .with_transmission_range(5.0);

        assert!(severe.is_highly_transmissible());
        assert!(severe.is_highly_lethal());
        assert!(severe.is_airborne());
    }

    #[test]
    fn test_pathogen_traits_fingerprint_deterministic() {
        let traits1 = PathogenTraits::new().with_transmissibility(0.5);
        let traits2 = PathogenTraits::new().with_transmissibility(0.5);

        assert_eq!(traits1.fingerprint(), traits2.fingerprint());
    }

    #[test]
    fn test_pathogen_traits_fingerprint_differs() {
        let traits1 = PathogenTraits::new().with_transmissibility(0.5);
        let traits2 = PathogenTraits::new().with_transmissibility(0.6);

        assert_ne!(traits1.fingerprint(), traits2.fingerprint());
    }

    #[test]
    fn test_trait_bounds_clamp() {
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

        let mut traits = PathogenTraits::new()
            .with_transmissibility(0.95)
            .with_virulence(0.01)
            .with_incubation(10)
            .with_lethality(0.9);

        bounds.clamp_traits(&mut traits);

        assert!((traits.transmissibility - 0.9).abs() < f32::EPSILON);
        assert!((traits.virulence - 0.05).abs() < f32::EPSILON);
        assert_eq!(traits.incubation_duration, 20);
        assert!((traits.lethality - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pathogen_category() {
        assert_eq!(PathogenCategory::Bacterial.as_index(), 0);
        assert_eq!(PathogenCategory::Viral.as_index(), 1);
        assert_eq!(PathogenCategory::Fungal.as_index(), 2);
    }

    #[test]
    fn test_pathogen_def_builder() {
        let def = PathogenDef::new("custom", "Custom Disease")
            .with_description("A test disease")
            .with_category(PathogenCategory::Viral)
            .with_susceptible("human")
            .with_immune("robot");

        assert_eq!(def.id.as_str(), "custom");
        assert_eq!(def.name, "Custom Disease");
        assert_eq!(def.category, PathogenCategory::Viral);
        assert!(def.is_susceptible("human"));
        assert!(!def.is_susceptible("robot"));
    }

    #[test]
    fn test_pathogen_def_susceptibility() {
        let def = PathogenDef::new("test", "Test");
        assert!(def.is_susceptible("anything"));

        let def_with_list = PathogenDef::new("test2", "Test2").with_susceptible("human");
        assert!(def_with_list.is_susceptible("human"));
        assert!(!def_with_list.is_susceptible("orc"));
    }

    #[test]
    fn test_pathogen_def_fingerprint() {
        let def1 = PathogenDef::new("test", "Test");
        let def2 = PathogenDef::new("test", "Test");

        assert_eq!(def1.fingerprint(), def2.fingerprint());
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = PathogenRegistry::new();
        registry.register(PathogenDef::new("test", "Test"));

        assert!(registry.get(&PathogenId::new("test")).is_some());
        assert!(registry.get(&PathogenId::new("nonexistent")).is_none());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_by_category() {
        let mut registry = PathogenRegistry::new();
        registry
            .register(PathogenDef::new("viral1", "Viral1").with_category(PathogenCategory::Viral));
        registry
            .register(PathogenDef::new("viral2", "Viral2").with_category(PathogenCategory::Viral));
        registry.register(
            PathogenDef::new("bacterial1", "Bacterial1").with_category(PathogenCategory::Bacterial),
        );

        let viral: Vec<_> = registry
            .pathogens_by_category(PathogenCategory::Viral)
            .collect();
        assert_eq!(viral.len(), 2);

        let bacterial: Vec<_> = registry
            .pathogens_by_category(PathogenCategory::Bacterial)
            .collect();
        assert_eq!(bacterial.len(), 1);
    }

    #[test]
    fn test_registry_rebuild_index() {
        let mut registry = PathogenRegistry::new();
        registry
            .register(PathogenDef::new("viral1", "Viral1").with_category(PathogenCategory::Viral));

        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: PathogenRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();

        let viral: Vec<_> = restored
            .pathogens_by_category(PathogenCategory::Viral)
            .collect();
        assert_eq!(viral.len(), 1);
    }

    #[test]
    fn test_registry_checksum() {
        let reg1 = presets::create_preset_registry();
        let reg2 = presets::create_preset_registry();

        assert_eq!(reg1.checksum(), reg2.checksum());
    }

    #[test]
    fn test_presets() {
        let registry = presets::create_preset_registry();
        assert_eq!(registry.len(), 6);

        assert!(registry.get(&PathogenId::plague()).is_some());
        assert!(registry.get(&PathogenId::blight()).is_some());
        assert!(registry.get(&PathogenId::rot()).is_some());
        assert!(registry.get(&PathogenId::spore_lung()).is_some());
        assert!(registry.get(&PathogenId::wasting()).is_some());
        assert!(registry.get(&PathogenId::fever()).is_some());
    }

    #[test]
    fn test_preset_plague() {
        let def = presets::plague();
        assert_eq!(def.category, PathogenCategory::Bacterial);
        assert!(
            def.base_traits.is_highly_transmissible() || def.base_traits.transmissibility > 0.5
        );
    }

    #[test]
    fn test_preset_wasting() {
        let def = presets::wasting();
        assert_eq!(def.category, PathogenCategory::Prion);
        assert!(def.base_traits.is_highly_lethal());
        assert!(def.base_traits.incubation_duration >= 1000);
    }

    #[test]
    fn test_serde_pathogen_traits() {
        let traits = PathogenTraits::new()
            .with_transmissibility(0.7)
            .with_carrier_behavior(true, 0.5);

        let json = serde_json::to_string(&traits).unwrap();
        let restored: PathogenTraits = serde_json::from_str(&json).unwrap();

        assert!((traits.transmissibility - restored.transmissibility).abs() < f32::EPSILON);
        assert_eq!(traits.can_become_carrier, restored.can_become_carrier);
    }

    #[test]
    fn test_serde_pathogen_def() {
        let def = presets::plague();
        let json = serde_json::to_string(&def).unwrap();
        let restored: PathogenDef = serde_json::from_str(&json).unwrap();

        assert_eq!(def.id, restored.id);
        assert_eq!(def.name, restored.name);
        assert_eq!(def.category, restored.category);
    }

    #[test]
    fn test_serde_registry() {
        let registry = presets::create_preset_registry();
        let json = serde_json::to_string(&registry).unwrap();
        let mut restored: PathogenRegistry = serde_json::from_str(&json).unwrap();
        restored.rebuild_index();

        assert_eq!(registry.len(), restored.len());
    }
}

//! Configuration types for creature lifecycle.

use super::state::GrowthPhase;
use serde::{Deserialize, Serialize};

/// Configuration for egg incubation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IncubationConfig {
    /// Base duration in ticks for incubation.
    pub base_duration: u64,
    /// Temperature sensitivity modifier (0.0 = no effect, 1.0 = full effect).
    pub temperature_sensitivity: f32,
    /// Minimum viable incubation duration.
    pub min_duration: u64,
    /// Maximum incubation duration before egg dies.
    pub max_duration: u64,
    /// Base survival chance for hatching (0.0 to 1.0).
    pub survival_chance: f32,
}

impl IncubationConfig {
    #[must_use]
    pub fn new(base_duration: u64) -> Self {
        Self {
            base_duration,
            temperature_sensitivity: 0.5,
            min_duration: base_duration / 2,
            max_duration: base_duration * 2,
            survival_chance: 0.9,
        }
    }

    #[must_use]
    pub fn with_temperature_sensitivity(mut self, sensitivity: f32) -> Self {
        self.temperature_sensitivity = sensitivity.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_survival_chance(mut self, chance: f32) -> Self {
        self.survival_chance = chance.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_duration_bounds(mut self, min: u64, max: u64) -> Self {
        self.min_duration = min;
        self.max_duration = max.max(min);
        self
    }

    #[must_use]
    pub fn rapid() -> Self {
        Self {
            base_duration: 100,
            temperature_sensitivity: 0.8,
            min_duration: 50,
            max_duration: 200,
            survival_chance: 0.85,
        }
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            base_duration: 500,
            temperature_sensitivity: 0.5,
            min_duration: 300,
            max_duration: 800,
            survival_chance: 0.9,
        }
    }

    #[must_use]
    pub fn slow() -> Self {
        Self {
            base_duration: 2000,
            temperature_sensitivity: 0.3,
            min_duration: 1500,
            max_duration: 3000,
            survival_chance: 0.95,
        }
    }
}

impl Default for IncubationConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Configuration for hatching process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HatchingConfig {
    /// Duration of the hatching process itself.
    pub hatching_duration: u64,
    /// Initial growth phase after hatching.
    pub initial_phase: GrowthPhase,
    /// Initial health percentage (0.0 to 1.0).
    pub initial_health: f32,
}

impl HatchingConfig {
    #[must_use]
    pub fn new(hatching_duration: u64, initial_phase: GrowthPhase) -> Self {
        Self {
            hatching_duration,
            initial_phase,
            initial_health: 1.0,
        }
    }

    #[must_use]
    pub fn with_initial_health(mut self, health: f32) -> Self {
        self.initial_health = health.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            hatching_duration: 10,
            initial_phase: GrowthPhase::Juvenile,
            initial_health: 1.0,
        }
    }
}

impl Default for HatchingConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Configuration for growth stages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrowthConfig {
    /// Duration of juvenile phase in ticks.
    pub juvenile_duration: u64,
    /// Duration of adult phase in ticks (before elder).
    pub adult_duration: u64,
    /// Growth rate multiplier.
    pub growth_rate: f32,
    /// Size multiplier at juvenile stage.
    pub juvenile_size: f32,
    /// Size multiplier at adult stage.
    pub adult_size: f32,
    /// Size multiplier at elder stage.
    pub elder_size: f32,
}

impl GrowthConfig {
    #[must_use]
    pub fn new(juvenile_duration: u64, adult_duration: u64) -> Self {
        Self {
            juvenile_duration,
            adult_duration,
            growth_rate: 1.0,
            juvenile_size: 0.5,
            adult_size: 1.0,
            elder_size: 0.95,
        }
    }

    #[must_use]
    pub fn with_growth_rate(mut self, rate: f32) -> Self {
        self.growth_rate = rate.max(0.01);
        self
    }

    #[must_use]
    pub fn with_sizes(mut self, juvenile: f32, adult: f32, elder: f32) -> Self {
        self.juvenile_size = juvenile.clamp(0.1, 2.0);
        self.adult_size = adult.clamp(0.1, 3.0);
        self.elder_size = elder.clamp(0.1, 3.0);
        self
    }

    #[must_use]
    pub fn rapid() -> Self {
        Self {
            juvenile_duration: 200,
            adult_duration: 1000,
            growth_rate: 1.5,
            juvenile_size: 0.6,
            adult_size: 1.0,
            elder_size: 0.9,
        }
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            juvenile_duration: 1000,
            adult_duration: 5000,
            growth_rate: 1.0,
            juvenile_size: 0.5,
            adult_size: 1.0,
            elder_size: 0.95,
        }
    }

    #[must_use]
    pub fn slow() -> Self {
        Self {
            juvenile_duration: 5000,
            adult_duration: 20000,
            growth_rate: 0.5,
            juvenile_size: 0.3,
            adult_size: 1.0,
            elder_size: 1.0,
        }
    }
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Configuration for aging and natural death.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgingConfig {
    /// Maximum lifespan in ticks (None = immortal).
    pub max_lifespan: Option<u64>,
    /// Age at which elder decline begins.
    pub elder_age: u64,
    /// Health decay rate per tick during elder phase.
    pub elder_decay_rate: f32,
    /// Chance of natural death per tick in elder phase.
    pub elder_death_chance: f32,
}

impl AgingConfig {
    #[must_use]
    pub fn new(max_lifespan: Option<u64>) -> Self {
        Self {
            max_lifespan,
            elder_age: max_lifespan.map_or(10000, |l| l * 3 / 4),
            elder_decay_rate: 0.001,
            elder_death_chance: 0.0001,
        }
    }

    #[must_use]
    pub fn with_elder_settings(mut self, age: u64, decay_rate: f32, death_chance: f32) -> Self {
        self.elder_age = age;
        self.elder_decay_rate = decay_rate.clamp(0.0, 1.0);
        self.elder_death_chance = death_chance.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn immortal() -> Self {
        Self {
            max_lifespan: None,
            elder_age: u64::MAX,
            elder_decay_rate: 0.0,
            elder_death_chance: 0.0,
        }
    }

    #[must_use]
    pub fn short_lived() -> Self {
        Self {
            max_lifespan: Some(2000),
            elder_age: 1500,
            elder_decay_rate: 0.005,
            elder_death_chance: 0.001,
        }
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            max_lifespan: Some(10000),
            elder_age: 7500,
            elder_decay_rate: 0.001,
            elder_death_chance: 0.0001,
        }
    }

    #[must_use]
    pub fn long_lived() -> Self {
        Self {
            max_lifespan: Some(50000),
            elder_age: 40000,
            elder_decay_rate: 0.0005,
            elder_death_chance: 0.00005,
        }
    }
}

impl Default for AgingConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Configuration for corpse decay and biomass release.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Duration for complete decay in ticks.
    pub full_decay_duration: u64,
    /// Biomass release rate per tick.
    pub biomass_release_rate: f32,
    /// Decay rate multiplier (affected by environment).
    pub decay_rate: f32,
    /// Minimum biomass before corpse disappears.
    pub min_biomass: f32,
}

impl DecayConfig {
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "duration bounded")]
    pub fn new(full_decay_duration: u64) -> Self {
        Self {
            full_decay_duration,
            biomass_release_rate: 1.0 / full_decay_duration as f32,
            decay_rate: 1.0,
            min_biomass: 0.01,
        }
    }

    #[must_use]
    pub fn with_decay_rate(mut self, rate: f32) -> Self {
        self.decay_rate = rate.max(0.01);
        self
    }

    #[must_use]
    pub fn with_min_biomass(mut self, min: f32) -> Self {
        self.min_biomass = min.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn rapid() -> Self {
        Self {
            full_decay_duration: 100,
            biomass_release_rate: 0.01,
            decay_rate: 2.0,
            min_biomass: 0.01,
        }
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            full_decay_duration: 500,
            biomass_release_rate: 0.002,
            decay_rate: 1.0,
            min_biomass: 0.01,
        }
    }

    #[must_use]
    pub fn slow() -> Self {
        Self {
            full_decay_duration: 2000,
            biomass_release_rate: 0.0005,
            decay_rate: 0.5,
            min_biomass: 0.005,
        }
    }
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Trigger condition for metamorphosis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MetamorphosisTrigger {
    /// Triggered at a specific age.
    Age(u64),
    /// Triggered when reaching a growth phase.
    GrowthPhase(GrowthPhase),
    /// Triggered by external signal (manual).
    External,
    /// Triggered when health drops below threshold.
    HealthThreshold(f32),
}

/// Configuration for metamorphosis (complete transformation).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetamorphosisConfig {
    /// What triggers metamorphosis.
    pub trigger: MetamorphosisTrigger,
    /// Duration of metamorphosis in ticks.
    pub duration: u64,
    /// Survival chance during metamorphosis (0.0 to 1.0).
    pub survival_chance: f32,
    /// Growth phase after metamorphosis completes.
    pub result_growth_phase: GrowthPhase,
}

impl MetamorphosisConfig {
    #[must_use]
    pub fn new(trigger: MetamorphosisTrigger, duration: u64) -> Self {
        Self {
            trigger,
            duration,
            survival_chance: 0.95,
            result_growth_phase: GrowthPhase::Adult,
        }
    }

    #[must_use]
    pub fn with_survival_chance(mut self, chance: f32) -> Self {
        self.survival_chance = chance.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_result_phase(mut self, phase: GrowthPhase) -> Self {
        self.result_growth_phase = phase;
        self
    }

    #[must_use]
    pub fn insect() -> Self {
        Self {
            trigger: MetamorphosisTrigger::GrowthPhase(GrowthPhase::Adult),
            duration: 200,
            survival_chance: 0.85,
            result_growth_phase: GrowthPhase::Adult,
        }
    }

    #[must_use]
    pub fn amphibian() -> Self {
        Self {
            trigger: MetamorphosisTrigger::Age(500),
            duration: 300,
            survival_chance: 0.9,
            result_growth_phase: GrowthPhase::Juvenile,
        }
    }
}

/// Complete lifecycle configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Incubation settings.
    pub incubation: IncubationConfig,
    /// Hatching settings.
    pub hatching: HatchingConfig,
    /// Growth settings.
    pub growth: GrowthConfig,
    /// Aging settings.
    pub aging: AgingConfig,
    /// Corpse decay settings.
    pub decay: DecayConfig,
    /// Optional metamorphosis configuration.
    pub metamorphosis: Option<MetamorphosisConfig>,
}

impl LifecycleConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_incubation(mut self, config: IncubationConfig) -> Self {
        self.incubation = config;
        self
    }

    #[must_use]
    pub fn with_hatching(mut self, config: HatchingConfig) -> Self {
        self.hatching = config;
        self
    }

    #[must_use]
    pub fn with_growth(mut self, config: GrowthConfig) -> Self {
        self.growth = config;
        self
    }

    #[must_use]
    pub fn with_aging(mut self, config: AgingConfig) -> Self {
        self.aging = config;
        self
    }

    #[must_use]
    pub fn with_decay(mut self, config: DecayConfig) -> Self {
        self.decay = config;
        self
    }

    #[must_use]
    pub fn with_metamorphosis(mut self, config: MetamorphosisConfig) -> Self {
        self.metamorphosis = Some(config);
        self
    }

    #[must_use]
    pub fn minimal() -> Self {
        Self {
            incubation: IncubationConfig::rapid(),
            hatching: HatchingConfig::standard(),
            growth: GrowthConfig::rapid(),
            aging: AgingConfig::short_lived(),
            decay: DecayConfig::rapid(),
            metamorphosis: None,
        }
    }

    #[must_use]
    pub fn standard() -> Self {
        Self {
            incubation: IncubationConfig::standard(),
            hatching: HatchingConfig::standard(),
            growth: GrowthConfig::standard(),
            aging: AgingConfig::standard(),
            decay: DecayConfig::standard(),
            metamorphosis: None,
        }
    }

    #[must_use]
    pub fn insect() -> Self {
        Self {
            incubation: IncubationConfig::rapid(),
            hatching: HatchingConfig::new(5, GrowthPhase::Juvenile),
            growth: GrowthConfig::rapid(),
            aging: AgingConfig::short_lived(),
            decay: DecayConfig::rapid(),
            metamorphosis: Some(MetamorphosisConfig::insect()),
        }
    }

    #[must_use]
    pub fn mammal() -> Self {
        Self {
            incubation: IncubationConfig::slow(),
            hatching: HatchingConfig::new(50, GrowthPhase::Juvenile).with_initial_health(0.8),
            growth: GrowthConfig::slow(),
            aging: AgingConfig::long_lived(),
            decay: DecayConfig::slow(),
            metamorphosis: None,
        }
    }

    #[must_use]
    pub fn amphibian() -> Self {
        Self {
            incubation: IncubationConfig::rapid().with_temperature_sensitivity(0.9),
            hatching: HatchingConfig::new(10, GrowthPhase::Juvenile),
            growth: GrowthConfig::standard(),
            aging: AgingConfig::standard(),
            decay: DecayConfig::standard(),
            metamorphosis: Some(MetamorphosisConfig::amphibian()),
        }
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incubation_config_new() {
        let config = IncubationConfig::new(300);
        assert_eq!(config.base_duration, 300);
        assert_eq!(config.min_duration, 150);
        assert_eq!(config.max_duration, 600);
    }

    #[test]
    fn test_incubation_config_clamp() {
        let config = IncubationConfig::new(100).with_survival_chance(1.5);
        assert!((config.survival_chance - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_incubation_presets() {
        let rapid = IncubationConfig::rapid();
        let standard = IncubationConfig::standard();
        let slow = IncubationConfig::slow();

        assert!(rapid.base_duration < standard.base_duration);
        assert!(standard.base_duration < slow.base_duration);
    }

    #[test]
    fn test_growth_config_builder() {
        let config = GrowthConfig::new(500, 2000)
            .with_growth_rate(1.5)
            .with_sizes(0.4, 1.2, 1.0);

        assert_eq!(config.juvenile_duration, 500);
        assert!((config.growth_rate - 1.5).abs() < f32::EPSILON);
        assert!((config.juvenile_size - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_aging_config_immortal() {
        let config = AgingConfig::immortal();
        assert!(config.max_lifespan.is_none());
        assert!((config.elder_death_chance).abs() < f32::EPSILON);
    }

    #[test]
    fn test_decay_config_new() {
        let config = DecayConfig::new(1000);
        assert_eq!(config.full_decay_duration, 1000);
        assert!(config.biomass_release_rate > 0.0);
    }

    #[test]
    fn test_metamorphosis_trigger_variants() {
        let age_trigger = MetamorphosisTrigger::Age(1000);
        let phase_trigger = MetamorphosisTrigger::GrowthPhase(GrowthPhase::Adult);
        let external_trigger = MetamorphosisTrigger::External;
        let health_trigger = MetamorphosisTrigger::HealthThreshold(0.3);

        assert!(matches!(age_trigger, MetamorphosisTrigger::Age(1000)));
        assert!(matches!(
            phase_trigger,
            MetamorphosisTrigger::GrowthPhase(GrowthPhase::Adult)
        ));
        assert!(matches!(external_trigger, MetamorphosisTrigger::External));
        assert!(matches!(
            health_trigger,
            MetamorphosisTrigger::HealthThreshold(_)
        ));
    }

    #[test]
    fn test_lifecycle_config_presets() {
        let minimal = LifecycleConfig::minimal();
        let standard = LifecycleConfig::standard();
        let insect = LifecycleConfig::insect();
        let mammal = LifecycleConfig::mammal();

        assert!(minimal.incubation.base_duration < standard.incubation.base_duration);
        assert!(insect.metamorphosis.is_some());
        assert!(mammal.metamorphosis.is_none());
    }

    #[test]
    fn test_lifecycle_config_builder() {
        let config = LifecycleConfig::new()
            .with_incubation(IncubationConfig::rapid())
            .with_growth(GrowthConfig::slow())
            .with_metamorphosis(MetamorphosisConfig::insect());

        assert_eq!(
            config.incubation.base_duration,
            IncubationConfig::rapid().base_duration
        );
        assert!(config.metamorphosis.is_some());
    }

    #[test]
    fn test_incubation_config_serde() {
        let config = IncubationConfig::standard();
        let json = serde_json::to_string(&config).unwrap();
        let restored: IncubationConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.base_duration, config.base_duration);
    }

    #[test]
    fn test_growth_config_serde() {
        let config = GrowthConfig::rapid();
        let json = serde_json::to_string(&config).unwrap();
        let restored: GrowthConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.juvenile_duration, config.juvenile_duration);
    }

    #[test]
    fn test_lifecycle_config_serde() {
        let config = LifecycleConfig::insect();
        let json = serde_json::to_string(&config).unwrap();
        let restored: LifecycleConfig = serde_json::from_str(&json).unwrap();

        assert!(restored.metamorphosis.is_some());
        assert_eq!(
            restored.incubation.base_duration,
            config.incubation.base_duration
        );
    }
}

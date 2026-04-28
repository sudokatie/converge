//! Need profiles and per-creature configurations.

use super::{Need, NeedId, NeedSet, Threshold};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identifier for a need profile (creature type).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProfileId(pub String);

impl ProfileId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ProfileId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

/// Configuration for a single need within a profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeedConfig {
    /// Maximum value for this need.
    pub max: f32,
    /// Starting value (defaults to max if not set).
    pub initial: Option<f32>,
    /// Decay rate per tick.
    pub decay_rate: f32,
    /// Recovery rate per tick.
    pub recovery_rate: f32,
    /// Thresholds for state classification.
    pub thresholds: Threshold,
    /// Priority weight for urgency scoring.
    pub priority_weight: f32,
}

impl NeedConfig {
    /// Create a basic config with sensible defaults.
    #[must_use]
    pub fn basic(max: f32, decay_rate: f32) -> Self {
        Self {
            max,
            initial: None,
            decay_rate,
            recovery_rate: decay_rate * 2.0,
            thresholds: Threshold::default(),
            priority_weight: 1.0,
        }
    }

    /// Create with custom initial value.
    #[must_use]
    pub fn with_initial(mut self, initial: f32) -> Self {
        self.initial = Some(initial);
        self
    }

    /// Create with custom recovery rate.
    #[must_use]
    pub fn with_recovery(mut self, rate: f32) -> Self {
        self.recovery_rate = rate;
        self
    }

    /// Create with custom thresholds.
    #[must_use]
    pub fn with_thresholds(mut self, thresholds: Threshold) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Create with custom priority weight.
    #[must_use]
    pub fn with_priority(mut self, weight: f32) -> Self {
        self.priority_weight = weight;
        self
    }

    /// Create a Need instance from this config.
    #[must_use]
    pub fn create_need(&self, id: NeedId) -> Need {
        let initial = self.initial.unwrap_or(self.max);
        Need::with_config(
            id,
            initial,
            self.max,
            self.decay_rate,
            self.recovery_rate,
            self.thresholds.clone(),
            self.priority_weight,
        )
    }
}

impl Default for NeedConfig {
    fn default() -> Self {
        Self::basic(100.0, 1.0)
    }
}

/// A profile defining the needs configuration for a creature type.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NeedProfile {
    /// Unique identifier for this profile.
    pub id: ProfileId,
    /// Configuration for each need type.
    configs: BTreeMap<NeedId, NeedConfig>,
}

impl NeedProfile {
    /// Create a new empty profile.
    #[must_use]
    pub fn new(id: ProfileId) -> Self {
        Self {
            id,
            configs: BTreeMap::new(),
        }
    }

    /// Add a need configuration.
    pub fn add_need(&mut self, id: NeedId, config: NeedConfig) {
        self.configs.insert(id, config);
    }

    /// Builder method to add a need.
    #[must_use]
    pub fn with_need(mut self, id: NeedId, config: NeedConfig) -> Self {
        self.add_need(id, config);
        self
    }

    /// Get a need config by ID.
    #[must_use]
    pub fn get_config(&self, id: &NeedId) -> Option<&NeedConfig> {
        self.configs.get(id)
    }

    /// Get a mutable need config by ID.
    pub fn get_config_mut(&mut self, id: &NeedId) -> Option<&mut NeedConfig> {
        self.configs.get_mut(id)
    }

    /// Check if profile has a need.
    #[must_use]
    pub fn has_need(&self, id: &NeedId) -> bool {
        self.configs.contains_key(id)
    }

    /// Get the number of needs in this profile.
    #[must_use]
    pub fn len(&self) -> usize {
        self.configs.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    /// Iterate over need IDs.
    pub fn need_ids(&self) -> impl Iterator<Item = &NeedId> {
        self.configs.keys()
    }

    /// Iterate over configs.
    pub fn configs(&self) -> impl Iterator<Item = (&NeedId, &NeedConfig)> {
        self.configs.iter()
    }

    /// Create a [`NeedSet`] from this profile.
    #[must_use]
    pub fn create_need_set(&self) -> NeedSet {
        let needs = self
            .configs
            .iter()
            .map(|(id, config)| config.create_need(id.clone()));
        NeedSet::from_needs(needs)
    }

    /// Create a standard humanoid profile.
    #[must_use]
    pub fn humanoid() -> Self {
        Self::new(ProfileId::new("humanoid"))
            .with_need(
                NeedId::hunger(),
                NeedConfig::basic(100.0, 0.5).with_priority(1.2),
            )
            .with_need(
                NeedId::thirst(),
                NeedConfig::basic(100.0, 0.8).with_priority(1.5),
            )
            .with_need(
                NeedId::rest(),
                NeedConfig::basic(100.0, 0.3).with_priority(1.0),
            )
            .with_need(
                NeedId::morale(),
                NeedConfig::basic(100.0, 0.1)
                    .with_priority(0.5)
                    .with_thresholds(Threshold::new(20.0, 40.0, 70.0)),
            )
    }

    /// Create a profile for creatures needing oxygen (underwater, space).
    #[must_use]
    pub fn oxygen_dependent() -> Self {
        Self::new(ProfileId::new("oxygen_dependent"))
            .with_need(
                NeedId::oxygen(),
                NeedConfig::basic(100.0, 5.0)
                    .with_priority(10.0)
                    .with_thresholds(Threshold::new(5.0, 20.0, 90.0)),
            )
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 0.5))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 0.8))
    }

    /// Create a profile for creatures needing warmth.
    #[must_use]
    pub fn cold_sensitive() -> Self {
        Self::new(ProfileId::new("cold_sensitive"))
            .with_need(
                NeedId::warmth(),
                NeedConfig::basic(100.0, 1.0)
                    .with_priority(2.0)
                    .with_thresholds(Threshold::new(15.0, 35.0, 75.0)),
            )
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 0.7))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 0.5))
    }

    /// Create a simple animal profile.
    #[must_use]
    pub fn simple_animal() -> Self {
        Self::new(ProfileId::new("simple_animal"))
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 0.3))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 0.4))
    }

    /// Create a social creature profile.
    #[must_use]
    pub fn social_creature() -> Self {
        Self::new(ProfileId::new("social_creature"))
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 0.5))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 0.6))
            .with_need(
                NeedId::social(),
                NeedConfig::basic(100.0, 0.2)
                    .with_priority(0.8)
                    .with_thresholds(Threshold::new(10.0, 30.0, 60.0)),
            )
            .with_need(
                NeedId::safety(),
                NeedConfig::basic(100.0, 0.1)
                    .with_priority(1.5)
                    .with_initial(80.0),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_id() {
        let id = ProfileId::new("test_profile");
        assert_eq!(id.as_str(), "test_profile");
    }

    #[test]
    fn test_need_config_basic() {
        let config = NeedConfig::basic(100.0, 2.0);

        assert!((config.max - 100.0).abs() < f32::EPSILON);
        assert!((config.decay_rate - 2.0).abs() < f32::EPSILON);
        assert!((config.recovery_rate - 4.0).abs() < f32::EPSILON);
        assert!(config.initial.is_none());
    }

    #[test]
    fn test_need_config_builder() {
        let config = NeedConfig::basic(100.0, 1.0)
            .with_initial(50.0)
            .with_recovery(3.0)
            .with_priority(2.0);

        assert_eq!(config.initial, Some(50.0));
        assert!((config.recovery_rate - 3.0).abs() < f32::EPSILON);
        assert!((config.priority_weight - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_need_config_create_need() {
        let config = NeedConfig::basic(100.0, 1.0).with_initial(75.0);
        let need = config.create_need(NeedId::hunger());

        assert_eq!(need.id, NeedId::hunger());
        assert!((need.value() - 75.0).abs() < f32::EPSILON);
        assert!((need.max() - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_profile_new() {
        let profile = NeedProfile::new(ProfileId::new("test"));

        assert_eq!(profile.id.as_str(), "test");
        assert!(profile.is_empty());
    }

    #[test]
    fn test_profile_with_need() {
        let profile = NeedProfile::new(ProfileId::new("test"))
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 1.0))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 2.0));

        assert_eq!(profile.len(), 2);
        assert!(profile.has_need(&NeedId::hunger()));
        assert!(profile.has_need(&NeedId::thirst()));
        assert!(!profile.has_need(&NeedId::oxygen()));
    }

    #[test]
    fn test_profile_create_need_set() {
        let profile = NeedProfile::new(ProfileId::new("test"))
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 1.0))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 2.0));

        let need_set = profile.create_need_set();

        assert_eq!(need_set.len(), 2);
        assert!(need_set.contains(&NeedId::hunger()));
        assert!(need_set.contains(&NeedId::thirst()));
    }

    #[test]
    fn test_humanoid_profile() {
        let profile = NeedProfile::humanoid();

        assert!(profile.has_need(&NeedId::hunger()));
        assert!(profile.has_need(&NeedId::thirst()));
        assert!(profile.has_need(&NeedId::rest()));
        assert!(profile.has_need(&NeedId::morale()));
    }

    #[test]
    fn test_oxygen_dependent_profile() {
        let profile = NeedProfile::oxygen_dependent();

        assert!(profile.has_need(&NeedId::oxygen()));

        let oxygen_config = profile.get_config(&NeedId::oxygen()).unwrap();
        assert!((oxygen_config.priority_weight - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_social_creature_profile() {
        let profile = NeedProfile::social_creature();

        assert!(profile.has_need(&NeedId::social()));
        assert!(profile.has_need(&NeedId::safety()));
    }

    #[test]
    fn test_profile_serde() {
        let profile = NeedProfile::humanoid();

        let json = serde_json::to_string(&profile).unwrap();
        let restored: NeedProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, profile.id);
        assert_eq!(restored.len(), profile.len());
        assert!(restored.has_need(&NeedId::hunger()));
    }

    #[test]
    fn test_configs_iterator() {
        let profile = NeedProfile::new(ProfileId::new("test"))
            .with_need(NeedId::hunger(), NeedConfig::basic(100.0, 1.0))
            .with_need(NeedId::thirst(), NeedConfig::basic(100.0, 2.0));

        let ids: Vec<_> = profile.need_ids().collect();
        assert_eq!(ids.len(), 2);
    }
}

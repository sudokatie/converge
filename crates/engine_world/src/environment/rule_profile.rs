//! Runtime world rule configuration profiles.
//!
//! Profiles bundle simulation rules for atmosphere, hazards, fluids, structural
//! integrity, gravity, timeline, scheduler, and conduits. Games can swap profiles
//! at runtime without code changes.
//!
//! # Features
//!
//! - Named profiles with unique IDs
//! - Profile inheritance via parent references and overrides
//! - Deterministic fingerprints for network/save compatibility
//! - Validation with descriptive errors

use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    AtmosphereConfig, ConduitNetworkConfig, FluidTransportConfig, GravityModel, PropagationConfig,
    StructuralConfig, TransitionRules,
};
use crate::scheduler::SchedulerConfig;
use crate::world_state::TimelineConfig;

/// Unique identifier for a rule profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProfileId(u64);

impl ProfileId {
    /// Create a profile ID from a raw value.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Reserved ID for the default profile.
    pub const DEFAULT: Self = Self(0);

    /// Reserved ID for space environments.
    pub const SPACE: Self = Self(1);

    /// Reserved ID for underground environments.
    pub const UNDERGROUND: Self = Self(2);

    /// First ID available for custom profiles.
    pub const CUSTOM_START: Self = Self(1000);
}

impl Default for ProfileId {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "profile:{}", self.0)
    }
}

/// Complete bundle of all world simulation rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleBundle {
    /// Atmosphere layer behavior and transitions.
    pub atmosphere: AtmosphereConfig,
    /// Atmosphere layer transition rules.
    pub transitions: TransitionRules,
    /// Hazard propagation configs (indexed by `HazardKind`).
    pub hazards: Vec<PropagationConfig>,
    /// Fluid transport configs (indexed by `FluidKind`).
    pub fluids: Vec<FluidTransportConfig>,
    /// Structural integrity simulation.
    pub structural: StructuralConfig,
    /// Gravity model.
    pub gravity: GravityModel,
    /// World event timeline behavior.
    pub timeline: TimelineConfig,
    /// Simulation scheduler behavior.
    pub scheduler: SchedulerConfig,
    /// Conduit network configs (indexed by `ConduitKind`).
    pub conduits: Vec<ConduitNetworkConfig>,
}

impl RuleBundle {
    /// Create a bundle with default values for all systems.
    ///
    /// # Panics
    ///
    /// Panics if internal `HazardKind` or `FluidKind` enum indices are inconsistent.
    #[must_use]
    pub fn new() -> Self {
        use super::{ConduitKind, FluidKind, HazardKind};

        Self {
            atmosphere: AtmosphereConfig::default(),
            transitions: TransitionRules::default(),
            hazards: (0..HazardKind::COUNT)
                .map(|i| PropagationConfig::new(HazardKind::from_index(i).unwrap()))
                .collect(),
            fluids: (0..FluidKind::COUNT)
                .map(|i| FluidTransportConfig::for_kind(FluidKind::from_index(i).unwrap()))
                .collect(),
            structural: StructuralConfig::default(),
            gravity: GravityModel::default(),
            timeline: TimelineConfig::default(),
            scheduler: SchedulerConfig::default(),
            conduits: ConduitKind::ALL
                .iter()
                .map(|&k| ConduitNetworkConfig::for_kind(k))
                .collect(),
        }
    }

    /// Create a bundle optimized for space environments.
    ///
    /// # Panics
    ///
    /// Panics if internal `HazardKind` or `FluidKind` enum indices are inconsistent.
    #[must_use]
    pub fn space() -> Self {
        use super::{ConduitKind, FluidKind, HazardKind};

        Self {
            atmosphere: AtmosphereConfig::space(),
            transitions: TransitionRules::strict(),
            hazards: (0..HazardKind::COUNT)
                .map(|i| {
                    let kind = HazardKind::from_index(i).unwrap();
                    let mut cfg = PropagationConfig::new(kind);
                    if kind == HazardKind::Vacuum {
                        cfg.spread.rate *= 2.0;
                    }
                    cfg
                })
                .collect(),
            fluids: (0..FluidKind::COUNT)
                .map(|i| FluidTransportConfig::for_kind(FluidKind::from_index(i).unwrap()))
                .collect(),
            structural: StructuralConfig::SPACE,
            gravity: GravityModel::ZERO_G,
            timeline: TimelineConfig::default(),
            scheduler: SchedulerConfig::dense(),
            conduits: ConduitKind::ALL
                .iter()
                .map(|&k| ConduitNetworkConfig::for_kind(k))
                .collect(),
        }
    }

    /// Create a bundle optimized for underground environments.
    ///
    /// # Panics
    ///
    /// Panics if internal `HazardKind` or `FluidKind` enum indices are inconsistent.
    #[must_use]
    pub fn underground() -> Self {
        use super::{ConduitKind, FluidKind, HazardKind};

        Self {
            atmosphere: AtmosphereConfig::underground(),
            transitions: TransitionRules::default(),
            hazards: (0..HazardKind::COUNT)
                .map(|i| {
                    let kind = HazardKind::from_index(i).unwrap();
                    let mut cfg = PropagationConfig::new(kind);
                    if kind == HazardKind::Flood {
                        cfg.spread.gravity_multiplier *= 1.5;
                    }
                    cfg
                })
                .collect(),
            fluids: (0..FluidKind::COUNT)
                .map(|i| {
                    let kind = FluidKind::from_index(i).unwrap();
                    let mut cfg = FluidTransportConfig::for_kind(kind);
                    cfg.gravity_bias *= 1.2;
                    cfg
                })
                .collect(),
            structural: StructuralConfig::UNDERGROUND,
            gravity: GravityModel::EARTH,
            timeline: TimelineConfig::no_seasons(),
            scheduler: SchedulerConfig::default(),
            conduits: ConduitKind::ALL
                .iter()
                .map(|&k| ConduitNetworkConfig::for_kind(k))
                .collect(),
        }
    }

    /// Compute a deterministic fingerprint for this bundle.
    ///
    /// Uses CRC32 for stable cross-platform network/save compatibility.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let json = serde_json::to_string(self).unwrap_or_default();
        u64::from(crc32fast::hash(json.as_bytes()))
    }
}

impl Default for RuleBundle {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional overrides for individual rule systems.
///
/// Used for profile inheritance: child profiles only specify fields they want
/// to override from their parent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleOverrides {
    /// Override atmosphere config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atmosphere: Option<AtmosphereConfig>,
    /// Override transition rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitions: Option<TransitionRules>,
    /// Override specific hazard configs (sparse by index).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub hazards: BTreeMap<usize, PropagationConfig>,
    /// Override specific fluid configs (sparse by index).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub fluids: BTreeMap<usize, FluidTransportConfig>,
    /// Override structural config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural: Option<StructuralConfig>,
    /// Override gravity model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravity: Option<GravityModel>,
    /// Override timeline config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelineConfig>,
    /// Override scheduler config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<SchedulerConfig>,
    /// Override specific conduit configs (sparse by index).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub conduits: BTreeMap<usize, ConduitNetworkConfig>,
}

impl RuleOverrides {
    /// Create empty overrides.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any overrides are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.atmosphere.is_none()
            && self.transitions.is_none()
            && self.hazards.is_empty()
            && self.fluids.is_empty()
            && self.structural.is_none()
            && self.gravity.is_none()
            && self.timeline.is_none()
            && self.scheduler.is_none()
            && self.conduits.is_empty()
    }

    /// Apply these overrides to a base bundle, producing a new bundle.
    #[must_use]
    pub fn apply_to(&self, base: &RuleBundle) -> RuleBundle {
        let mut result = base.clone();

        if let Some(ref atm) = self.atmosphere {
            result.atmosphere = atm.clone();
        }
        if let Some(ref trans) = self.transitions {
            result.transitions = trans.clone();
        }
        for (&idx, cfg) in &self.hazards {
            if idx < result.hazards.len() {
                result.hazards[idx] = cfg.clone();
            }
        }
        for (&idx, cfg) in &self.fluids {
            if idx < result.fluids.len() {
                result.fluids[idx] = cfg.clone();
            }
        }
        if let Some(ref struc) = self.structural {
            result.structural = struc.clone();
        }
        if let Some(ref grav) = self.gravity {
            result.gravity = grav.clone();
        }
        if let Some(ref tl) = self.timeline {
            result.timeline = tl.clone();
        }
        if let Some(ref sched) = self.scheduler {
            result.scheduler = sched.clone();
        }
        for (&idx, cfg) in &self.conduits {
            if idx < result.conduits.len() {
                result.conduits[idx] = cfg.clone();
            }
        }

        result
    }

    /// Merge another set of overrides on top of this one.
    pub fn merge(&mut self, other: &RuleOverrides) {
        if other.atmosphere.is_some() {
            self.atmosphere.clone_from(&other.atmosphere);
        }
        if other.transitions.is_some() {
            self.transitions.clone_from(&other.transitions);
        }
        for (&idx, cfg) in &other.hazards {
            self.hazards.insert(idx, cfg.clone());
        }
        for (&idx, cfg) in &other.fluids {
            self.fluids.insert(idx, cfg.clone());
        }
        if other.structural.is_some() {
            self.structural.clone_from(&other.structural);
        }
        if other.gravity.is_some() {
            self.gravity.clone_from(&other.gravity);
        }
        if other.timeline.is_some() {
            self.timeline.clone_from(&other.timeline);
        }
        if other.scheduler.is_some() {
            self.scheduler.clone_from(&other.scheduler);
        }
        for (&idx, cfg) in &other.conduits {
            self.conduits.insert(idx, cfg.clone());
        }
    }
}

/// A named world rule profile with optional inheritance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldRuleProfile {
    /// Unique identifier.
    id: ProfileId,
    /// Human-readable name.
    name: Cow<'static, str>,
    /// Optional parent profile ID for inheritance.
    parent: Option<ProfileId>,
    /// Rule overrides (applied on top of parent or defaults).
    overrides: RuleOverrides,
    /// Cached resolved bundle (None until resolved).
    #[serde(skip)]
    resolved: Option<Box<RuleBundle>>,
}

impl WorldRuleProfile {
    /// Create a new profile with no parent (inherits from defaults).
    #[must_use]
    pub fn new(id: ProfileId, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id,
            name: name.into(),
            parent: None,
            overrides: RuleOverrides::new(),
            resolved: None,
        }
    }

    /// Create a profile that inherits from a parent.
    #[must_use]
    pub fn with_parent(
        id: ProfileId,
        name: impl Into<Cow<'static, str>>,
        parent: ProfileId,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            parent: Some(parent),
            overrides: RuleOverrides::new(),
            resolved: None,
        }
    }

    /// Create a profile from a complete bundle (no inheritance).
    #[must_use]
    pub fn from_bundle(
        id: ProfileId,
        name: impl Into<Cow<'static, str>>,
        bundle: RuleBundle,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            parent: None,
            overrides: RuleOverrides::new(),
            resolved: Some(Box::new(bundle)),
        }
    }

    /// Get the profile ID.
    #[must_use]
    pub fn id(&self) -> ProfileId {
        self.id
    }

    /// Get the profile name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the parent profile ID, if any.
    #[must_use]
    pub fn parent(&self) -> Option<ProfileId> {
        self.parent
    }

    /// Get the rule overrides.
    #[must_use]
    pub fn overrides(&self) -> &RuleOverrides {
        &self.overrides
    }

    /// Get a mutable reference to the rule overrides.
    pub fn overrides_mut(&mut self) -> &mut RuleOverrides {
        self.resolved = None;
        &mut self.overrides
    }

    /// Set the parent profile.
    pub fn set_parent(&mut self, parent: Option<ProfileId>) {
        self.parent = parent;
        self.resolved = None;
    }

    /// Check if this profile has been resolved.
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.resolved.is_some()
    }

    /// Get the resolved bundle, if available.
    #[must_use]
    pub fn resolved(&self) -> Option<&RuleBundle> {
        self.resolved.as_deref()
    }

    /// Clear the resolved cache.
    pub fn invalidate(&mut self) {
        self.resolved = None;
    }
}

/// Validation error for profiles.
#[derive(Clone, Debug, PartialEq)]
pub enum ProfileError {
    /// Profile ID already exists.
    DuplicateId(ProfileId),
    /// Referenced parent profile not found.
    ParentNotFound(ProfileId),
    /// Circular inheritance detected.
    CircularInheritance(Vec<ProfileId>),
    /// Invalid hazard index in overrides.
    InvalidHazardIndex(usize),
    /// Invalid fluid index in overrides.
    InvalidFluidIndex(usize),
    /// Invalid conduit index in overrides.
    InvalidConduitIndex(usize),
    /// Structural config validation failed.
    InvalidStructuralConfig(String),
    /// Conduit config validation failed.
    InvalidConduitConfig(usize, String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "duplicate profile ID: {id}"),
            Self::ParentNotFound(id) => write!(f, "parent profile not found: {id}"),
            Self::CircularInheritance(chain) => {
                write!(f, "circular inheritance: ")?;
                for (i, id) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " -> ")?;
                    }
                    write!(f, "{id}")?;
                }
                Ok(())
            }
            Self::InvalidHazardIndex(idx) => write!(f, "invalid hazard index: {idx}"),
            Self::InvalidFluidIndex(idx) => write!(f, "invalid fluid index: {idx}"),
            Self::InvalidConduitIndex(idx) => write!(f, "invalid conduit index: {idx}"),
            Self::InvalidStructuralConfig(msg) => write!(f, "invalid structural config: {msg}"),
            Self::InvalidConduitConfig(idx, msg) => {
                write!(f, "invalid conduit config at index {idx}: {msg}")
            }
        }
    }
}

impl std::error::Error for ProfileError {}

/// Registry of world rule profiles.
///
/// Manages profile storage, inheritance resolution, and validation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileRegistry {
    /// Profiles indexed by ID (`BTreeMap` for deterministic iteration).
    profiles: BTreeMap<ProfileId, WorldRuleProfile>,
    /// Currently active profile ID.
    active: ProfileId,
}

impl ProfileRegistry {
    /// Create a registry with built-in default profiles.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            profiles: BTreeMap::new(),
            active: ProfileId::DEFAULT,
        };

        registry.profiles.insert(
            ProfileId::DEFAULT,
            WorldRuleProfile::from_bundle(ProfileId::DEFAULT, "Default", RuleBundle::new()),
        );
        registry.profiles.insert(
            ProfileId::SPACE,
            WorldRuleProfile::from_bundle(ProfileId::SPACE, "Space", RuleBundle::space()),
        );
        registry.profiles.insert(
            ProfileId::UNDERGROUND,
            WorldRuleProfile::from_bundle(
                ProfileId::UNDERGROUND,
                "Underground",
                RuleBundle::underground(),
            ),
        );

        registry
    }

    /// Get the number of profiles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Get the active profile ID.
    #[must_use]
    pub fn active_id(&self) -> ProfileId {
        self.active
    }

    /// Get the active profile's resolved bundle.
    #[must_use]
    pub fn active(&self) -> Option<&RuleBundle> {
        self.profiles.get(&self.active).and_then(|p| p.resolved())
    }

    /// Set the active profile.
    ///
    /// # Errors
    ///
    /// Returns [`ProfileError::ParentNotFound`] if the profile doesn't exist.
    pub fn set_active(&mut self, id: ProfileId) -> Result<(), ProfileError> {
        if !self.profiles.contains_key(&id) {
            return Err(ProfileError::ParentNotFound(id));
        }
        self.active = id;
        Ok(())
    }

    /// Check if a profile exists.
    #[must_use]
    pub fn contains(&self, id: ProfileId) -> bool {
        self.profiles.contains_key(&id)
    }

    /// Get a profile by ID.
    #[must_use]
    pub fn get(&self, id: ProfileId) -> Option<&WorldRuleProfile> {
        self.profiles.get(&id)
    }

    /// Get a mutable profile by ID.
    pub fn get_mut(&mut self, id: ProfileId) -> Option<&mut WorldRuleProfile> {
        self.profiles.get_mut(&id)
    }

    /// Get a resolved bundle by profile ID.
    #[must_use]
    pub fn get_bundle(&self, id: ProfileId) -> Option<&RuleBundle> {
        self.profiles.get(&id).and_then(|p| p.resolved())
    }

    /// Register a new profile.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile ID is a duplicate or validation fails.
    pub fn register(&mut self, profile: WorldRuleProfile) -> Result<(), ProfileError> {
        let id = profile.id();
        if self.profiles.contains_key(&id) {
            return Err(ProfileError::DuplicateId(id));
        }
        self.validate_profile(&profile)?;
        self.profiles.insert(id, profile);
        Ok(())
    }

    /// Remove a profile by ID.
    ///
    /// Built-in profiles (DEFAULT, SPACE, UNDERGROUND) cannot be removed.
    pub fn remove(&mut self, id: ProfileId) -> Option<WorldRuleProfile> {
        if id == ProfileId::DEFAULT || id == ProfileId::SPACE || id == ProfileId::UNDERGROUND {
            return None;
        }
        if self.active == id {
            self.active = ProfileId::DEFAULT;
        }
        self.profiles.remove(&id)
    }

    /// Iterate over all profiles in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&ProfileId, &WorldRuleProfile)> {
        self.profiles.iter()
    }

    /// Iterate over profile IDs in deterministic order.
    pub fn ids(&self) -> impl Iterator<Item = ProfileId> + '_ {
        self.profiles.keys().copied()
    }

    /// Resolve a profile's inheritance chain and cache the result.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile is not found or has circular inheritance.
    ///
    /// # Panics
    ///
    /// Panics if internal state is inconsistent after successful resolution.
    pub fn resolve(&mut self, id: ProfileId) -> Result<&RuleBundle, ProfileError> {
        self.check_circular(id)?;

        let chain = self.inheritance_chain(id)?;
        let mut bundle = RuleBundle::new();

        for &chain_id in &chain {
            if let Some(profile) = self.profiles.get(&chain_id) {
                if let Some(resolved) = &profile.resolved {
                    bundle = (**resolved).clone();
                } else {
                    bundle = profile.overrides.apply_to(&bundle);
                }
            }
        }

        if let Some(profile) = self.profiles.get_mut(&id) {
            profile.resolved = Some(Box::new(bundle));
        }

        Ok(self.profiles.get(&id).unwrap().resolved.as_ref().unwrap())
    }

    /// Resolve all profiles.
    ///
    /// # Errors
    ///
    /// Returns an error if any profile has circular inheritance or missing parent.
    pub fn resolve_all(&mut self) -> Result<(), ProfileError> {
        let ids: Vec<_> = self.profiles.keys().copied().collect();
        for id in ids {
            self.resolve(id)?;
        }
        Ok(())
    }

    /// Invalidate a profile and all profiles that inherit from it.
    pub fn invalidate(&mut self, id: ProfileId) {
        let mut to_invalidate = vec![id];
        let mut idx = 0;

        while idx < to_invalidate.len() {
            let current = to_invalidate[idx];
            for (pid, profile) in &self.profiles {
                if profile.parent == Some(current) && !to_invalidate.contains(pid) {
                    to_invalidate.push(*pid);
                }
            }
            idx += 1;
        }

        for pid in to_invalidate {
            if let Some(profile) = self.profiles.get_mut(&pid) {
                profile.invalidate();
            }
        }
    }

    /// Compute a combined fingerprint for all resolved profiles.
    ///
    /// Uses CRC32 for stable cross-platform network/save compatibility.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = crc32fast::Hasher::new();
        for (id, profile) in &self.profiles {
            hasher.update(&id.raw().to_le_bytes());
            if let Some(bundle) = profile.resolved() {
                hasher.update(&bundle.fingerprint().to_le_bytes());
            }
        }
        u64::from(hasher.finalize())
    }

    fn validate_profile(&self, profile: &WorldRuleProfile) -> Result<(), ProfileError> {
        use super::{ConduitKind, FluidKind, HazardKind};

        if let Some(parent) = profile.parent
            && !self.profiles.contains_key(&parent)
        {
            return Err(ProfileError::ParentNotFound(parent));
        }

        for &idx in profile.overrides.hazards.keys() {
            if idx >= HazardKind::COUNT {
                return Err(ProfileError::InvalidHazardIndex(idx));
            }
        }

        for &idx in profile.overrides.fluids.keys() {
            if idx >= FluidKind::COUNT {
                return Err(ProfileError::InvalidFluidIndex(idx));
            }
        }

        for &idx in profile.overrides.conduits.keys() {
            if idx >= ConduitKind::ALL.len() {
                return Err(ProfileError::InvalidConduitIndex(idx));
            }
        }

        if let Some(ref struc) = profile.overrides.structural
            && !struc.is_valid()
        {
            return Err(ProfileError::InvalidStructuralConfig(
                "failed validation".into(),
            ));
        }

        for (&idx, cfg) in &profile.overrides.conduits {
            if !cfg.is_valid() {
                return Err(ProfileError::InvalidConduitConfig(
                    idx,
                    "failed validation".into(),
                ));
            }
        }

        Ok(())
    }

    fn inheritance_chain(&self, id: ProfileId) -> Result<Vec<ProfileId>, ProfileError> {
        let mut chain = Vec::new();
        let mut current = Some(id);
        let mut visited = Vec::new();

        while let Some(cid) = current {
            if visited.contains(&cid) {
                visited.push(cid);
                return Err(ProfileError::CircularInheritance(visited));
            }
            visited.push(cid);

            if let Some(profile) = self.profiles.get(&cid) {
                chain.push(cid);
                current = profile.parent;
            } else {
                return Err(ProfileError::ParentNotFound(cid));
            }
        }

        chain.reverse();
        Ok(chain)
    }

    fn check_circular(&self, id: ProfileId) -> Result<(), ProfileError> {
        self.inheritance_chain(id)?;
        Ok(())
    }
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::HazardKind;

    #[test]
    fn profile_id_display() {
        assert_eq!(ProfileId::DEFAULT.to_string(), "profile:0");
        assert_eq!(ProfileId::new(42).to_string(), "profile:42");
    }

    #[test]
    fn rule_bundle_defaults() {
        let bundle = RuleBundle::new();
        assert_eq!(bundle.hazards.len(), HazardKind::COUNT);
        assert!(!bundle.hazards.is_empty());
        assert!(!bundle.fluids.is_empty());
        assert!(!bundle.conduits.is_empty());
    }

    #[test]
    fn rule_bundle_presets() {
        let default = RuleBundle::new();
        let space = RuleBundle::space();
        let underground = RuleBundle::underground();

        assert_ne!(default.atmosphere, space.atmosphere);
        assert_ne!(default.structural, underground.structural);
        assert!(matches!(space.gravity, GravityModel::ZeroG));
    }

    #[test]
    fn rule_bundle_fingerprint_deterministic() {
        let b1 = RuleBundle::new();
        let b2 = RuleBundle::new();
        assert_eq!(b1.fingerprint(), b2.fingerprint());

        let b3 = RuleBundle::space();
        assert_ne!(b1.fingerprint(), b3.fingerprint());
    }

    #[test]
    fn rule_overrides_empty() {
        let overrides = RuleOverrides::new();
        assert!(overrides.is_empty());
    }

    #[test]
    fn rule_overrides_apply() {
        let base = RuleBundle::new();
        let mut overrides = RuleOverrides::new();
        overrides.gravity = Some(GravityModel::LUNAR);

        let result = overrides.apply_to(&base);
        assert_eq!(result.gravity, GravityModel::LUNAR);
        assert_eq!(result.atmosphere, base.atmosphere);
    }

    #[test]
    fn rule_overrides_merge() {
        let mut o1 = RuleOverrides::new();
        o1.gravity = Some(GravityModel::LUNAR);

        let mut o2 = RuleOverrides::new();
        o2.structural = Some(StructuralConfig::SPACE);
        o2.gravity = Some(GravityModel::MARS);

        o1.merge(&o2);
        assert_eq!(o1.gravity, Some(GravityModel::MARS));
        assert!(o1.structural.is_some());
    }

    #[test]
    fn profile_creation() {
        let profile = WorldRuleProfile::new(ProfileId::new(100), "Test");
        assert_eq!(profile.id().raw(), 100);
        assert_eq!(profile.name(), "Test");
        assert!(profile.parent().is_none());
    }

    #[test]
    fn profile_with_parent() {
        let profile =
            WorldRuleProfile::with_parent(ProfileId::new(101), "Child", ProfileId::DEFAULT);
        assert_eq!(profile.parent(), Some(ProfileId::DEFAULT));
    }

    #[test]
    fn profile_from_bundle() {
        let bundle = RuleBundle::space();
        let fp = bundle.fingerprint();
        let profile = WorldRuleProfile::from_bundle(ProfileId::new(102), "Custom", bundle);
        assert!(profile.is_resolved());
        assert_eq!(profile.resolved().unwrap().fingerprint(), fp);
    }

    #[test]
    fn registry_new_has_builtins() {
        let registry = ProfileRegistry::new();
        assert!(registry.contains(ProfileId::DEFAULT));
        assert!(registry.contains(ProfileId::SPACE));
        assert!(registry.contains(ProfileId::UNDERGROUND));
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn registry_register_and_get() {
        let mut registry = ProfileRegistry::new();
        let profile = WorldRuleProfile::new(ProfileId::new(1000), "Custom");

        registry.register(profile).unwrap();
        assert!(registry.contains(ProfileId::new(1000)));
        assert_eq!(registry.get(ProfileId::new(1000)).unwrap().name(), "Custom");
    }

    #[test]
    fn registry_duplicate_id_error() {
        let mut registry = ProfileRegistry::new();
        let profile = WorldRuleProfile::new(ProfileId::DEFAULT, "Duplicate");

        let result = registry.register(profile);
        assert!(matches!(result, Err(ProfileError::DuplicateId(_))));
    }

    #[test]
    fn registry_parent_not_found_error() {
        let mut registry = ProfileRegistry::new();
        let profile =
            WorldRuleProfile::with_parent(ProfileId::new(1000), "Orphan", ProfileId::new(9999));

        let result = registry.register(profile);
        assert!(matches!(result, Err(ProfileError::ParentNotFound(_))));
    }

    #[test]
    fn registry_resolve_inheritance() {
        let mut registry = ProfileRegistry::new();

        let mut child =
            WorldRuleProfile::with_parent(ProfileId::new(1000), "Child", ProfileId::DEFAULT);
        child.overrides_mut().gravity = Some(GravityModel::LUNAR);
        registry.register(child).unwrap();

        let bundle = registry.resolve(ProfileId::new(1000)).unwrap();
        assert_eq!(bundle.gravity, GravityModel::LUNAR);
    }

    #[test]
    fn registry_resolve_chain() {
        let mut registry = ProfileRegistry::new();

        let mut level1 =
            WorldRuleProfile::with_parent(ProfileId::new(1000), "Level1", ProfileId::DEFAULT);
        level1.overrides_mut().gravity = Some(GravityModel::LUNAR);
        registry.register(level1).unwrap();

        let mut level2 =
            WorldRuleProfile::with_parent(ProfileId::new(1001), "Level2", ProfileId::new(1000));
        level2.overrides_mut().structural = Some(StructuralConfig::SPACE);
        registry.register(level2).unwrap();

        let bundle = registry.resolve(ProfileId::new(1001)).unwrap();
        assert_eq!(bundle.gravity, GravityModel::LUNAR);
        assert_eq!(bundle.structural, StructuralConfig::SPACE);
    }

    #[test]
    fn registry_circular_inheritance_error() {
        let mut registry = ProfileRegistry::new();

        let p1 = WorldRuleProfile::with_parent(ProfileId::new(1000), "A", ProfileId::new(1001));
        let p2 = WorldRuleProfile::with_parent(ProfileId::new(1001), "B", ProfileId::new(1000));

        registry.profiles.insert(ProfileId::new(1000), p1);
        registry.profiles.insert(ProfileId::new(1001), p2);

        let result = registry.resolve(ProfileId::new(1000));
        assert!(matches!(result, Err(ProfileError::CircularInheritance(_))));
    }

    #[test]
    fn registry_set_active() {
        let mut registry = ProfileRegistry::new();
        registry.resolve_all().unwrap();

        assert!(registry.set_active(ProfileId::SPACE).is_ok());
        assert_eq!(registry.active_id(), ProfileId::SPACE);

        assert!(registry.set_active(ProfileId::new(9999)).is_err());
    }

    #[test]
    fn registry_remove() {
        let mut registry = ProfileRegistry::new();
        let profile = WorldRuleProfile::new(ProfileId::new(1000), "Custom");
        registry.register(profile).unwrap();

        assert!(registry.remove(ProfileId::new(1000)).is_some());
        assert!(!registry.contains(ProfileId::new(1000)));

        assert!(registry.remove(ProfileId::DEFAULT).is_none());
    }

    #[test]
    fn registry_invalidate_cascade() {
        let mut registry = ProfileRegistry::new();

        let parent =
            WorldRuleProfile::with_parent(ProfileId::new(1000), "Parent", ProfileId::DEFAULT);
        let child =
            WorldRuleProfile::with_parent(ProfileId::new(1001), "Child", ProfileId::new(1000));

        registry.register(parent).unwrap();
        registry.register(child).unwrap();
        registry.resolve_all().unwrap();

        assert!(registry.get(ProfileId::new(1000)).unwrap().is_resolved());
        assert!(registry.get(ProfileId::new(1001)).unwrap().is_resolved());

        registry.invalidate(ProfileId::new(1000));

        assert!(!registry.get(ProfileId::new(1000)).unwrap().is_resolved());
        assert!(!registry.get(ProfileId::new(1001)).unwrap().is_resolved());
    }

    #[test]
    fn registry_fingerprint_deterministic() {
        let mut r1 = ProfileRegistry::new();
        let mut r2 = ProfileRegistry::new();

        r1.resolve_all().unwrap();
        r2.resolve_all().unwrap();

        assert_eq!(r1.fingerprint(), r2.fingerprint());
    }

    #[test]
    fn registry_iter_deterministic() {
        let registry = ProfileRegistry::new();
        let ids1: Vec<_> = registry.ids().collect();
        let ids2: Vec<_> = registry.ids().collect();
        assert_eq!(ids1, ids2);
    }

    #[test]
    fn invalid_hazard_index_error() {
        let mut registry = ProfileRegistry::new();
        let mut profile = WorldRuleProfile::new(ProfileId::new(1000), "Bad");
        profile
            .overrides_mut()
            .hazards
            .insert(999, PropagationConfig::new(HazardKind::Fire));

        let result = registry.register(profile);
        assert!(matches!(result, Err(ProfileError::InvalidHazardIndex(999))));
    }

    #[test]
    fn serde_profile_id() {
        let id = ProfileId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: ProfileId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, id);
    }

    #[test]
    fn serde_rule_bundle() {
        let bundle = RuleBundle::space();
        let json = serde_json::to_string(&bundle).unwrap();
        let recovered: RuleBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.fingerprint(), bundle.fingerprint());
    }

    #[test]
    fn serde_rule_overrides() {
        let mut overrides = RuleOverrides::new();
        overrides.gravity = Some(GravityModel::MARS);
        overrides
            .hazards
            .insert(0, PropagationConfig::new(HazardKind::Fire));

        let json = serde_json::to_string(&overrides).unwrap();
        let recovered: RuleOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.gravity, overrides.gravity);
        assert_eq!(recovered.hazards.len(), 1);
    }

    #[test]
    fn serde_world_rule_profile() {
        let profile =
            WorldRuleProfile::from_bundle(ProfileId::new(100), "Test", RuleBundle::underground());

        let json = serde_json::to_string(&profile).unwrap();
        let recovered: WorldRuleProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.id(), profile.id());
        assert_eq!(recovered.name(), profile.name());
    }

    #[test]
    fn serde_profile_registry() {
        let mut registry = ProfileRegistry::new();
        registry.resolve_all().unwrap();

        let json = serde_json::to_string(&registry).unwrap();
        let recovered: ProfileRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), registry.len());
        assert_eq!(recovered.active_id(), registry.active_id());
    }
}

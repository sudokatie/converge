//! Mod descriptor types for sandbox and compatibility checking.
//!
//! Descriptors capture mod metadata, capability requirements, dependencies,
//! and content declarations for compatibility analysis.

use serde::{Deserialize, Serialize};

use crate::game_pack::{Capability, PackDependency, PackVersion};

use super::{budget::ResourceBudget, capability::CapabilityRequirements, id::ModId};

/// API version range that a mod supports.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiVersionRange {
    pub min: PackVersion,
    #[serde(default)]
    pub max: Option<PackVersion>,
}

impl ApiVersionRange {
    #[must_use]
    pub fn new(min: PackVersion) -> Self {
        Self { min, max: None }
    }

    #[must_use]
    pub fn with_max(mut self, max: PackVersion) -> Self {
        self.max = Some(max);
        self
    }

    #[must_use]
    pub fn exact(version: PackVersion) -> Self {
        Self {
            min: version.clone(),
            max: Some(version),
        }
    }

    /// Check if a version is within this range.
    #[must_use]
    pub fn contains(&self, version: &PackVersion) -> bool {
        if version < &self.min {
            return false;
        }
        if let Some(ref max) = self.max
            && version > max
        {
            return false;
        }
        true
    }

    /// Check if this range overlaps with another.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        const MAX_VERSION: PackVersion = PackVersion::new(u32::MAX, u32::MAX, u32::MAX);
        let other_max = other.max.as_ref().unwrap_or(&MAX_VERSION);
        let self_max = self.max.as_ref().unwrap_or(&MAX_VERSION);

        self.min <= *other_max && other.min <= *self_max
    }
}

/// Conflict declaration for incompatible mods.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModConflict {
    pub mod_name: String,
    #[serde(default)]
    pub version_range: Option<ApiVersionRange>,
    #[serde(default)]
    pub reason: Option<String>,
}

impl ModConflict {
    #[must_use]
    pub fn with_mod(name: impl Into<String>) -> Self {
        Self {
            mod_name: name.into(),
            version_range: None,
            reason: None,
        }
    }

    #[must_use]
    pub fn with_version_range(mut self, range: ApiVersionRange) -> Self {
        self.version_range = Some(range);
        self
    }

    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Check if this conflict applies to a specific mod version.
    #[must_use]
    pub fn applies_to(&self, mod_name: &str, version: &PackVersion) -> bool {
        if self.mod_name != mod_name {
            return false;
        }
        match &self.version_range {
            Some(range) => range.contains(version),
            None => true,
        }
    }
}

/// Load order constraint.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadOrderConstraint {
    Before(String),
    After(String),
    First,
    Last,
}

/// Content hook reference for integration validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHookRef {
    pub hook_name: String,
    #[serde(default)]
    pub event_trigger: Option<String>,
    #[serde(default)]
    pub priority: i32,
}

impl ContentHookRef {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            hook_name: name.into(),
            event_trigger: None,
            priority: 0,
        }
    }

    #[must_use]
    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.event_trigger = Some(trigger.into());
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Game pack integration reference.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GamePackRef {
    pub pack_name: String,
    #[serde(default)]
    pub capabilities_used: Vec<Capability>,
}

impl GamePackRef {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            pack_name: name.into(),
            capabilities_used: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities_used.push(cap);
        self
    }
}

/// Complete mod descriptor for sandbox and compatibility checking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModDescriptor {
    pub id: ModId,
    pub name: String,
    pub version: PackVersion,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,

    // Version compatibility
    #[serde(default)]
    pub engine_version: Option<ApiVersionRange>,
    #[serde(default)]
    pub api_version: Option<ApiVersionRange>,

    // Dependencies and conflicts
    #[serde(default)]
    pub dependencies: Vec<PackDependency>,
    #[serde(default)]
    pub conflicts: Vec<ModConflict>,
    #[serde(default)]
    pub load_order: Vec<LoadOrderConstraint>,

    // Sandbox requirements
    #[serde(default)]
    pub sandbox_capabilities: CapabilityRequirements,
    #[serde(default)]
    pub requested_budget: Option<ResourceBudget>,

    // Content integration
    #[serde(default)]
    pub content_hooks: Vec<ContentHookRef>,
    #[serde(default)]
    pub game_packs: Vec<GamePackRef>,
    #[serde(default)]
    pub provides: Vec<Capability>,

    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ModDescriptor {
    #[must_use]
    pub fn new(id: ModId, name: impl Into<String>, version: PackVersion) -> Self {
        Self {
            id,
            name: name.into(),
            version,
            display_name: None,
            description: String::new(),
            authors: Vec::new(),
            license: None,
            homepage: None,
            engine_version: None,
            api_version: None,
            dependencies: Vec::new(),
            conflicts: Vec::new(),
            load_order: Vec::new(),
            sandbox_capabilities: CapabilityRequirements::default(),
            requested_budget: None,
            content_hooks: Vec::new(),
            game_packs: Vec::new(),
            provides: Vec::new(),
            enabled: true,
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self
    }

    #[must_use]
    pub fn with_engine_version(mut self, range: ApiVersionRange) -> Self {
        self.engine_version = Some(range);
        self
    }

    #[must_use]
    pub fn with_api_version(mut self, range: ApiVersionRange) -> Self {
        self.api_version = Some(range);
        self
    }

    #[must_use]
    pub fn with_dependency(mut self, dep: PackDependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    #[must_use]
    pub fn with_conflict(mut self, conflict: ModConflict) -> Self {
        self.conflicts.push(conflict);
        self
    }

    #[must_use]
    pub fn with_load_order(mut self, constraint: LoadOrderConstraint) -> Self {
        self.load_order.push(constraint);
        self
    }

    #[must_use]
    pub fn with_capabilities(mut self, caps: CapabilityRequirements) -> Self {
        self.sandbox_capabilities = caps;
        self
    }

    #[must_use]
    pub fn with_budget(mut self, budget: ResourceBudget) -> Self {
        self.requested_budget = Some(budget);
        self
    }

    #[must_use]
    pub fn with_content_hook(mut self, hook: ContentHookRef) -> Self {
        self.content_hooks.push(hook);
        self
    }

    #[must_use]
    pub fn with_game_pack(mut self, pack: GamePackRef) -> Self {
        self.game_packs.push(pack);
        self
    }

    #[must_use]
    pub fn with_provides(mut self, cap: Capability) -> Self {
        self.provides.push(cap);
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check if this mod requires high-risk sandbox capabilities.
    #[must_use]
    pub fn requires_high_risk(&self) -> bool {
        self.sandbox_capabilities.has_high_risk_required()
    }

    /// Get all dependency names.
    #[must_use]
    pub fn dependency_names(&self) -> Vec<&str> {
        self.dependencies.iter().map(|d| d.name.as_str()).collect()
    }

    /// Check if this mod conflicts with another by name and version.
    #[must_use]
    pub fn conflicts_with(&self, mod_name: &str, version: &PackVersion) -> bool {
        self.conflicts
            .iter()
            .any(|c| c.applies_to(mod_name, version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mod_sandbox::capability::SandboxCapability;

    #[test]
    fn api_version_range_contains() {
        let range =
            ApiVersionRange::new(PackVersion::new(1, 0, 0)).with_max(PackVersion::new(2, 0, 0));

        assert!(range.contains(&PackVersion::new(1, 0, 0)));
        assert!(range.contains(&PackVersion::new(1, 5, 0)));
        assert!(range.contains(&PackVersion::new(2, 0, 0)));
        assert!(!range.contains(&PackVersion::new(0, 9, 0)));
        assert!(!range.contains(&PackVersion::new(2, 0, 1)));
    }

    #[test]
    fn api_version_range_unbounded() {
        let range = ApiVersionRange::new(PackVersion::new(1, 0, 0));

        assert!(range.contains(&PackVersion::new(1, 0, 0)));
        assert!(range.contains(&PackVersion::new(99, 0, 0)));
        assert!(!range.contains(&PackVersion::new(0, 9, 0)));
    }

    #[test]
    fn api_version_range_overlaps() {
        let range1 =
            ApiVersionRange::new(PackVersion::new(1, 0, 0)).with_max(PackVersion::new(2, 0, 0));
        let range2 =
            ApiVersionRange::new(PackVersion::new(1, 5, 0)).with_max(PackVersion::new(3, 0, 0));
        let range3 = ApiVersionRange::new(PackVersion::new(3, 0, 0));

        assert!(range1.overlaps(&range2));
        assert!(!range1.overlaps(&range3));
    }

    #[test]
    fn mod_conflict_applies() {
        let conflict = ModConflict::with_mod("broken_mod").with_version_range(
            ApiVersionRange::new(PackVersion::new(1, 0, 0)).with_max(PackVersion::new(1, 5, 0)),
        );

        assert!(conflict.applies_to("broken_mod", &PackVersion::new(1, 2, 0)));
        assert!(!conflict.applies_to("broken_mod", &PackVersion::new(2, 0, 0)));
        assert!(!conflict.applies_to("other_mod", &PackVersion::new(1, 2, 0)));
    }

    #[test]
    fn mod_descriptor_builder() {
        let desc = ModDescriptor::new(ModId::new(1, 1), "test_mod", PackVersion::new(1, 0, 0))
            .with_display_name("Test Mod")
            .with_author("Test Author")
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)))
            .with_capabilities(
                CapabilityRequirements::new().with_required(SandboxCapability::ReadOwnData),
            )
            .with_tag("gameplay");

        assert_eq!(desc.name, "test_mod");
        assert_eq!(desc.display_name, Some("Test Mod".to_string()));
        assert_eq!(desc.authors, vec!["Test Author"]);
        assert_eq!(desc.dependencies.len(), 1);
        assert_eq!(desc.tags, vec!["gameplay"]);
    }

    #[test]
    fn mod_descriptor_conflicts_check() {
        let desc = ModDescriptor::new(ModId::new(1, 1), "my_mod", PackVersion::new(1, 0, 0))
            .with_conflict(ModConflict::with_mod("incompatible_mod"));

        assert!(desc.conflicts_with("incompatible_mod", &PackVersion::new(1, 0, 0)));
        assert!(!desc.conflicts_with("compatible_mod", &PackVersion::new(1, 0, 0)));
    }

    #[test]
    fn mod_descriptor_serde_roundtrip() {
        let desc = ModDescriptor::new(ModId::new(1, 1), "test_mod", PackVersion::new(1, 2, 3))
            .with_engine_version(ApiVersionRange::new(PackVersion::new(0, 2, 0)))
            .with_content_hook(ContentHookRef::new("on_player_join").with_priority(10));

        let json = serde_json::to_string(&desc).unwrap();
        let restored: ModDescriptor = serde_json::from_str(&json).unwrap();

        assert_eq!(desc.id, restored.id);
        assert_eq!(desc.name, restored.name);
        assert_eq!(desc.version, restored.version);
    }

    #[test]
    fn mod_descriptor_bincode_roundtrip() {
        let desc = ModDescriptor::new(ModId::new(1, 1), "test_mod", PackVersion::new(1, 0, 0))
            .with_game_pack(
                GamePackRef::new("base_pack").with_capability(Capability::OverrideBlocks),
            );

        let bytes = bincode::serialize(&desc).unwrap();
        let restored: ModDescriptor = bincode::deserialize(&bytes).unwrap();

        assert_eq!(desc.id, restored.id);
        assert_eq!(desc.game_packs.len(), restored.game_packs.len());
    }
}

//! Pack manifest containing metadata, version, dependencies, and capabilities.

use serde::{Deserialize, Serialize};

/// Semantic version for pack compatibility.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PackVersion {
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    #[must_use]
    pub fn is_compatible_with(&self, required: &Self) -> bool {
        if self.major != required.major {
            return false;
        }
        if self.major == 0 {
            self.minor == required.minor && self.patch >= required.patch
        } else {
            self.minor > required.minor
                || (self.minor == required.minor && self.patch >= required.patch)
        }
    }
}

impl std::fmt::Display for PackVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A dependency on another pack.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackDependency {
    pub name: String,
    pub min_version: PackVersion,
    #[serde(default)]
    pub optional: bool,
}

impl PackDependency {
    #[must_use]
    pub fn required(name: impl Into<String>, min_version: PackVersion) -> Self {
        Self {
            name: name.into(),
            min_version,
            optional: false,
        }
    }

    #[must_use]
    pub fn optional(name: impl Into<String>, min_version: PackVersion) -> Self {
        Self {
            name: name.into(),
            min_version,
            optional: true,
        }
    }
}

/// Capability flags that a pack can declare or require.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    OverrideBlocks,
    OverrideSystems,
    OverrideHazards,
    OverrideShaders,
    OverrideRules,
    ExclusiveWorldRules,
    Custom(String),
}

/// Manifest describing a game pack's metadata and requirements.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackManifest {
    pub name: String,
    pub version: PackVersion,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<PackDependency>,
    #[serde(default)]
    pub provides: Vec<Capability>,
    #[serde(default)]
    pub requires: Vec<Capability>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub load_priority: i32,
}

impl PackManifest {
    #[must_use]
    pub fn new(name: impl Into<String>, version: PackVersion) -> Self {
        Self {
            name: name.into(),
            version,
            description: String::new(),
            authors: Vec::new(),
            dependencies: Vec::new(),
            provides: Vec::new(),
            requires: Vec::new(),
            enabled: true,
            load_priority: 0,
        }
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self
    }

    #[must_use]
    pub fn with_dependency(mut self, dep: PackDependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    #[must_use]
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.provides.push(cap);
        self
    }

    #[must_use]
    pub fn with_requirement(mut self, cap: Capability) -> Self {
        self.requires.push(cap);
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.load_priority = priority;
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_display() {
        let v = PackVersion::new(1, 2, 3);
        assert_eq!(format!("{v}"), "1.2.3");
    }

    #[test]
    fn version_ordering() {
        let v1 = PackVersion::new(1, 0, 0);
        let v2 = PackVersion::new(1, 1, 0);
        let v3 = PackVersion::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn version_compatibility_major_zero() {
        let v010 = PackVersion::new(0, 1, 0);
        let v011 = PackVersion::new(0, 1, 1);
        let v020 = PackVersion::new(0, 2, 0);

        assert!(v011.is_compatible_with(&v010));
        assert!(!v010.is_compatible_with(&v011));
        assert!(!v020.is_compatible_with(&v010));
    }

    #[test]
    fn version_compatibility_major_nonzero() {
        let v100 = PackVersion::new(1, 0, 0);
        let v110 = PackVersion::new(1, 1, 0);
        let v111 = PackVersion::new(1, 1, 1);
        let v200 = PackVersion::new(2, 0, 0);

        assert!(v110.is_compatible_with(&v100));
        assert!(v111.is_compatible_with(&v110));
        assert!(!v100.is_compatible_with(&v110));
        assert!(!v200.is_compatible_with(&v100));
    }

    #[test]
    fn manifest_builder() {
        let manifest = PackManifest::new("test-pack", PackVersion::new(1, 0, 0))
            .with_description("A test pack")
            .with_author("Test Author")
            .with_priority(10)
            .with_capability(Capability::OverrideBlocks);

        assert_eq!(manifest.name, "test-pack");
        assert_eq!(manifest.description, "A test pack");
        assert_eq!(manifest.authors, vec!["Test Author"]);
        assert_eq!(manifest.load_priority, 10);
        assert!(manifest.provides.contains(&Capability::OverrideBlocks));
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let manifest = PackManifest::new("test-pack", PackVersion::new(1, 2, 3))
            .with_description("Test")
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)));

        let serialized = serde_json::to_string(&manifest).unwrap();
        let deserialized: PackManifest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(manifest.name, deserialized.name);
        assert_eq!(manifest.version, deserialized.version);
        assert_eq!(manifest.dependencies.len(), deserialized.dependencies.len());
    }

    #[test]
    fn manifest_bincode_roundtrip() {
        let manifest = PackManifest::new("test-pack", PackVersion::new(1, 2, 3))
            .with_description("Test")
            .with_capability(Capability::Custom("custom_cap".to_string()));

        let serialized = bincode::serialize(&manifest).unwrap();
        let deserialized: PackManifest = bincode::deserialize(&serialized).unwrap();

        assert_eq!(manifest.name, deserialized.name);
        assert_eq!(manifest.provides, deserialized.provides);
    }
}

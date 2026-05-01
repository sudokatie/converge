//! Asset manifest generation and versioning for Lattice content packs.
//!
//! Provides deterministic manifest generation with stable asset IDs, content hashing,
//! kind detection, dependency tracking, and version diffing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema version for the manifest format itself.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Known asset/data file kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Items,
    Recipes,
    Blocks,
    Creatures,
    Biomes,
    GamePack,
    Texture,
    Audio,
    Shader,
    Config,
    Unknown,
}

impl AssetKind {
    /// Detect asset kind from file path and extension.
    #[must_use]
    pub fn detect(path: &Path) -> Self {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match (name.as_str(), ext.as_str()) {
            ("items", "ron") => Self::Items,
            ("recipes", "ron") => Self::Recipes,
            ("blocks", "ron") => Self::Blocks,
            ("creatures", "ron") => Self::Creatures,
            ("biomes", "ron") => Self::Biomes,
            (_, "ron" | "toml" | "json") => Self::Config,
            ("gamepack" | "pack", _) => Self::GamePack,
            (_, "png" | "jpg" | "jpeg" | "bmp" | "tga" | "dds") => Self::Texture,
            (_, "wav" | "ogg" | "mp3" | "flac") => Self::Audio,
            (_, "wgsl" | "glsl" | "hlsl" | "spv") => Self::Shader,
            _ => Self::Unknown,
        }
    }
}

/// Deterministic content fingerprint combining hash and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash {
    /// CRC32 hash of file contents.
    pub crc32: u32,
    /// File size in bytes.
    pub size: u64,
}

impl ContentHash {
    /// Compute content hash from raw bytes.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            crc32: crc32fast::hash(data),
            size: data.len() as u64,
        }
    }

    /// Compute content hash from a file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened or read.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut hasher = crc32fast::Hasher::new();
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(Self {
            crc32: hasher.finalize(),
            size,
        })
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}:{}", self.crc32, self.size)
    }
}

/// Stable identifier for an asset, derived from its relative path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct AssetId(pub String);

impl AssetId {
    /// Create a stable asset ID from a relative path.
    /// Normalizes path separators for cross-platform consistency.
    #[must_use]
    pub fn from_path(relative_path: &Path) -> Self {
        let normalized = relative_path
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        Self(normalized)
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single asset entry in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetEntry {
    /// Stable asset identifier.
    pub id: AssetId,
    /// Relative path from content root.
    pub path: PathBuf,
    /// Detected asset kind.
    pub kind: AssetKind,
    /// Content hash/fingerprint.
    pub hash: ContentHash,
    /// Optional dependencies on other assets (by ID).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub dependencies: BTreeSet<AssetId>,
}

/// Version metadata for a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestVersion {
    /// Schema version of the manifest format.
    pub schema: u32,
    /// Content pack version string.
    pub pack_version: String,
    /// Optional compatibility minimum version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_compatible_version: Option<String>,
}

impl ManifestVersion {
    /// Create a new manifest version.
    #[must_use]
    pub fn new(pack_version: impl Into<String>) -> Self {
        Self {
            schema: MANIFEST_SCHEMA_VERSION,
            pack_version: pack_version.into(),
            min_compatible_version: None,
        }
    }

    /// Set minimum compatible version.
    #[must_use]
    pub fn with_min_compatible(mut self, version: impl Into<String>) -> Self {
        self.min_compatible_version = Some(version.into());
        self
    }

    /// Check if this manifest is compatible with the given version.
    #[must_use]
    pub fn is_compatible_with(&self, other_version: &str) -> bool {
        match &self.min_compatible_version {
            Some(min) => version_cmp(other_version, min) >= std::cmp::Ordering::Equal,
            None => true,
        }
    }
}

/// Simple semver-like comparison (major.minor.patch).
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
    let av = parse(a);
    let bv = parse(b);
    av.cmp(&bv)
}

/// The complete asset manifest for a content pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Version metadata.
    pub version: ManifestVersion,
    /// Assets indexed by ID (`BTreeMap` for deterministic ordering).
    pub assets: BTreeMap<AssetId, AssetEntry>,
}

impl AssetManifest {
    /// Create an empty manifest with the given version.
    #[must_use]
    pub fn new(version: ManifestVersion) -> Self {
        Self {
            version,
            assets: BTreeMap::new(),
        }
    }

    /// Add an asset entry to the manifest.
    pub fn add_asset(&mut self, entry: AssetEntry) {
        self.assets.insert(entry.id.clone(), entry);
    }

    /// Get an asset by ID.
    #[must_use]
    pub fn get_asset(&self, id: &AssetId) -> Option<&AssetEntry> {
        self.assets.get(id)
    }

    /// Compute the diff from this manifest to another.
    #[must_use]
    pub fn diff(&self, other: &Self) -> ManifestDiff {
        let mut diff = ManifestDiff::default();

        for (id, entry) in &other.assets {
            match self.assets.get(id) {
                None => {
                    diff.added.insert(id.clone(), entry.clone());
                }
                Some(old_entry) if old_entry.hash != entry.hash => {
                    diff.modified.insert(
                        id.clone(),
                        ModifiedAsset {
                            old: old_entry.clone(),
                            new: entry.clone(),
                        },
                    );
                }
                _ => {}
            }
        }

        for id in self.assets.keys() {
            if !other.assets.contains_key(id) {
                diff.removed.insert(id.clone());
            }
        }

        diff
    }

    /// Serialize to JSON.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    ///
    /// # Errors
    /// Returns an error if the JSON is invalid or doesn't match the schema.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// A modified asset showing old and new state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifiedAsset {
    pub old: AssetEntry,
    pub new: AssetEntry,
}

/// Difference between two manifests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDiff {
    /// Assets added in the new manifest.
    pub added: BTreeMap<AssetId, AssetEntry>,
    /// Assets removed from the old manifest.
    pub removed: BTreeSet<AssetId>,
    /// Assets modified between versions.
    pub modified: BTreeMap<AssetId, ModifiedAsset>,
}

impl ManifestDiff {
    /// Check if there are any changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Total number of changes.
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

impl std::fmt::Display for ManifestDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Manifest Diff")?;
        writeln!(f, "=============")?;

        if self.is_empty() {
            writeln!(f, "No changes.")?;
            return Ok(());
        }

        writeln!(
            f,
            "{} added, {} removed, {} modified",
            self.added.len(),
            self.removed.len(),
            self.modified.len()
        )?;
        writeln!(f)?;

        if !self.added.is_empty() {
            writeln!(f, "Added:")?;
            for (id, entry) in &self.added {
                writeln!(f, "  + {id} ({:?}, {})", entry.kind, entry.hash)?;
            }
        }

        if !self.removed.is_empty() {
            writeln!(f, "Removed:")?;
            for id in &self.removed {
                writeln!(f, "  - {id}")?;
            }
        }

        if !self.modified.is_empty() {
            writeln!(f, "Modified:")?;
            for (id, change) in &self.modified {
                writeln!(f, "  ~ {id} ({} -> {})", change.old.hash, change.new.hash)?;
            }
        }

        Ok(())
    }
}

/// Errors that can occur during manifest generation.
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Content root does not exist: {0}")]
    ContentRootNotFound(PathBuf),
    #[error("Content root is not a directory: {0}")]
    ContentRootNotDirectory(PathBuf),
}

/// Builder for generating asset manifests from a content directory.
pub struct ManifestGenerator {
    content_root: PathBuf,
    version: ManifestVersion,
    include_unknown: bool,
}

impl ManifestGenerator {
    /// Create a new manifest generator.
    pub fn new(content_root: impl AsRef<Path>, version: ManifestVersion) -> Self {
        Self {
            content_root: content_root.as_ref().to_path_buf(),
            version,
            include_unknown: false,
        }
    }

    /// Include files with unknown asset kind.
    #[must_use]
    pub fn include_unknown(mut self, include: bool) -> Self {
        self.include_unknown = include;
        self
    }

    /// Generate the manifest by scanning the content directory.
    ///
    /// # Errors
    /// Returns an error if the content root doesn't exist, isn't a directory, or if any file cannot be read.
    pub fn generate(&self) -> Result<AssetManifest, ManifestError> {
        if !self.content_root.exists() {
            return Err(ManifestError::ContentRootNotFound(
                self.content_root.clone(),
            ));
        }
        if !self.content_root.is_dir() {
            return Err(ManifestError::ContentRootNotDirectory(
                self.content_root.clone(),
            ));
        }

        let mut manifest = AssetManifest::new(self.version.clone());
        self.scan_directory(&self.content_root, &mut manifest)?;
        Self::resolve_dependencies(&mut manifest);

        Ok(manifest)
    }

    fn scan_directory(
        &self,
        dir: &Path,
        manifest: &mut AssetManifest,
    ) -> Result<(), ManifestError> {
        let mut entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(std::result::Result::ok)
            .collect();
        entries.sort_by_key(std::fs::DirEntry::path);

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                self.scan_directory(&path, manifest)?;
            } else if path.is_file() {
                let kind = AssetKind::detect(&path);
                if kind == AssetKind::Unknown && !self.include_unknown {
                    continue;
                }

                let relative_path = path
                    .strip_prefix(&self.content_root)
                    .unwrap_or(&path)
                    .to_path_buf();

                let hash = ContentHash::from_file(&path)?;
                let id = AssetId::from_path(&relative_path);

                let asset_entry = AssetEntry {
                    id,
                    path: relative_path,
                    kind,
                    hash,
                    dependencies: BTreeSet::new(),
                };

                manifest.add_asset(asset_entry);
            }
        }

        Ok(())
    }

    fn resolve_dependencies(manifest: &mut AssetManifest) {
        let items_id = AssetId("items.ron".to_string());
        let recipes_id = AssetId("recipes.ron".to_string());
        let blocks_id = AssetId("blocks.ron".to_string());
        let biomes_id = AssetId("biomes.ron".to_string());

        let has_items = manifest.assets.contains_key(&items_id);
        let has_blocks = manifest.assets.contains_key(&blocks_id);

        if has_items && let Some(entry) = manifest.assets.get_mut(&recipes_id) {
            entry.dependencies.insert(items_id.clone());
        }

        if has_blocks {
            if let Some(entry) = manifest.assets.get_mut(&biomes_id) {
                entry.dependencies.insert(blocks_id.clone());
            }
            if let Some(entry) = manifest.assets.get_mut(&items_id) {
                entry.dependencies.insert(blocks_id.clone());
            }
        }
    }
}

/// Check compatibility between two manifest versions.
#[must_use]
pub fn check_compatibility(
    old_manifest: &AssetManifest,
    new_manifest: &AssetManifest,
) -> CompatibilityResult {
    let mut result = CompatibilityResult {
        compatible: true,
        issues: Vec::new(),
    };

    if old_manifest.version.schema != new_manifest.version.schema {
        result.compatible = false;
        result.issues.push(format!(
            "Schema version mismatch: {} vs {}",
            old_manifest.version.schema, new_manifest.version.schema,
        ));
    }

    if let Some(min_version) = &new_manifest.version.min_compatible_version
        && !new_manifest
            .version
            .is_compatible_with(&old_manifest.version.pack_version)
    {
        result.compatible = false;
        result.issues.push(format!(
            "Pack version {} is below minimum compatible version {min_version}",
            old_manifest.version.pack_version
        ));
    }

    let diff = old_manifest.diff(new_manifest);

    for id in &diff.removed {
        let Some(old_entry) = old_manifest.get_asset(id) else {
            continue;
        };
        for entry in new_manifest.assets.values() {
            if entry.dependencies.contains(id) {
                result.compatible = false;
                result.issues.push(format!(
                    "Removed asset '{id}' is still referenced by '{}'",
                    entry.id
                ));
            }
        }

        if old_entry.kind != AssetKind::Unknown {
            result
                .issues
                .push(format!("Warning: Removed known asset type '{id}'"));
        }
    }

    result
}

/// Result of compatibility checking.
#[derive(Debug, Clone)]
pub struct CompatibilityResult {
    /// Whether the manifests are compatible.
    pub compatible: bool,
    /// List of compatibility issues found.
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_content_hash_determinism() {
        let data = b"test content for hashing";
        let hash1 = ContentHash::from_bytes(data);
        let hash2 = ContentHash::from_bytes(data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.size, data.len() as u64);
    }

    #[test]
    fn test_content_hash_different_content() {
        let hash1 = ContentHash::from_bytes(b"content a");
        let hash2 = ContentHash::from_bytes(b"content b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_from_file() {
        let dir = create_test_dir();
        let content = "file content for testing";
        write_file(dir.path(), "test.txt", content);

        let hash = ContentHash::from_file(&dir.path().join("test.txt")).unwrap();
        let expected = ContentHash::from_bytes(content.as_bytes());
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_asset_id_from_path() {
        let id = AssetId::from_path(Path::new("data/items.ron"));
        assert_eq!(id.0, "data/items.ron");

        let id2 = AssetId::from_path(Path::new("items.ron"));
        assert_eq!(id2.0, "items.ron");
    }

    #[test]
    fn test_asset_kind_detection() {
        assert_eq!(AssetKind::detect(Path::new("items.ron")), AssetKind::Items);
        assert_eq!(
            AssetKind::detect(Path::new("recipes.ron")),
            AssetKind::Recipes
        );
        assert_eq!(
            AssetKind::detect(Path::new("blocks.ron")),
            AssetKind::Blocks
        );
        assert_eq!(
            AssetKind::detect(Path::new("creatures.ron")),
            AssetKind::Creatures
        );
        assert_eq!(
            AssetKind::detect(Path::new("biomes.ron")),
            AssetKind::Biomes
        );
        assert_eq!(
            AssetKind::detect(Path::new("texture.png")),
            AssetKind::Texture
        );
        assert_eq!(AssetKind::detect(Path::new("sound.ogg")), AssetKind::Audio);
        assert_eq!(
            AssetKind::detect(Path::new("shader.wgsl")),
            AssetKind::Shader
        );
        assert_eq!(
            AssetKind::detect(Path::new("config.toml")),
            AssetKind::Config
        );
        assert_eq!(
            AssetKind::detect(Path::new("random.xyz")),
            AssetKind::Unknown
        );
        assert_eq!(
            AssetKind::detect(Path::new("gamepack.dat")),
            AssetKind::GamePack
        );
    }

    #[test]
    fn test_manifest_generation() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[(id: 1, name: \"Test\")]");
        write_file(dir.path(), "recipes.ron", "[]");

        let version = ManifestVersion::new("1.0.0");
        let generator = ManifestGenerator::new(dir.path(), version);
        let manifest = generator.generate().unwrap();

        assert_eq!(manifest.assets.len(), 2);
        assert!(
            manifest
                .assets
                .contains_key(&AssetId("items.ron".to_string()))
        );
        assert!(
            manifest
                .assets
                .contains_key(&AssetId("recipes.ron".to_string()))
        );
    }

    #[test]
    fn test_manifest_generation_determinism() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[(id: 1)]");
        write_file(dir.path(), "recipes.ron", "[]");
        write_file(dir.path(), "blocks.ron", "{}");

        let version = ManifestVersion::new("1.0.0");

        let gen1 = ManifestGenerator::new(dir.path(), version.clone());
        let manifest1 = gen1.generate().unwrap();

        let gen2 = ManifestGenerator::new(dir.path(), version);
        let manifest2 = gen2.generate().unwrap();

        assert_eq!(manifest1, manifest2);

        let json1 = manifest1.to_json().unwrap();
        let json2 = manifest2.to_json().unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_manifest_diff_added() {
        let version = ManifestVersion::new("1.0.0");
        let mut old = AssetManifest::new(version.clone());
        let mut new = AssetManifest::new(version);

        old.add_asset(AssetEntry {
            id: AssetId("a.ron".to_string()),
            path: PathBuf::from("a.ron"),
            kind: AssetKind::Config,
            hash: ContentHash::from_bytes(b"a"),
            dependencies: BTreeSet::new(),
        });

        new.add_asset(AssetEntry {
            id: AssetId("a.ron".to_string()),
            path: PathBuf::from("a.ron"),
            kind: AssetKind::Config,
            hash: ContentHash::from_bytes(b"a"),
            dependencies: BTreeSet::new(),
        });

        new.add_asset(AssetEntry {
            id: AssetId("b.ron".to_string()),
            path: PathBuf::from("b.ron"),
            kind: AssetKind::Config,
            hash: ContentHash::from_bytes(b"b"),
            dependencies: BTreeSet::new(),
        });

        let diff = old.diff(&new);
        assert_eq!(diff.added.len(), 1);
        assert!(diff.added.contains_key(&AssetId("b.ron".to_string())));
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn test_manifest_diff_removed() {
        let version = ManifestVersion::new("1.0.0");
        let mut old = AssetManifest::new(version.clone());
        let new = AssetManifest::new(version);

        old.add_asset(AssetEntry {
            id: AssetId("a.ron".to_string()),
            path: PathBuf::from("a.ron"),
            kind: AssetKind::Config,
            hash: ContentHash::from_bytes(b"a"),
            dependencies: BTreeSet::new(),
        });

        let diff = old.diff(&new);
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 1);
        assert!(diff.removed.contains(&AssetId("a.ron".to_string())));
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn test_manifest_diff_modified() {
        let version = ManifestVersion::new("1.0.0");
        let mut old = AssetManifest::new(version.clone());
        let mut new = AssetManifest::new(version);

        old.add_asset(AssetEntry {
            id: AssetId("a.ron".to_string()),
            path: PathBuf::from("a.ron"),
            kind: AssetKind::Config,
            hash: ContentHash::from_bytes(b"old content"),
            dependencies: BTreeSet::new(),
        });

        new.add_asset(AssetEntry {
            id: AssetId("a.ron".to_string()),
            path: PathBuf::from("a.ron"),
            kind: AssetKind::Config,
            hash: ContentHash::from_bytes(b"new content"),
            dependencies: BTreeSet::new(),
        });

        let diff = old.diff(&new);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.modified.len(), 1);
        assert!(diff.modified.contains_key(&AssetId("a.ron".to_string())));
    }

    #[test]
    fn test_manifest_json_roundtrip() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[]");

        let version = ManifestVersion::new("1.0.0").with_min_compatible("0.9.0");
        let generator = ManifestGenerator::new(dir.path(), version);
        let manifest = generator.generate().unwrap();

        let json = manifest.to_json().unwrap();
        let parsed = AssetManifest::from_json(&json).unwrap();

        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_version_compatibility() {
        let version = ManifestVersion::new("1.0.0").with_min_compatible("0.5.0");
        assert!(version.is_compatible_with("0.5.0"));
        assert!(version.is_compatible_with("0.6.0"));
        assert!(version.is_compatible_with("1.0.0"));
        assert!(!version.is_compatible_with("0.4.0"));
        assert!(!version.is_compatible_with("0.4.9"));
    }

    #[test]
    fn test_compatibility_check_schema_mismatch() {
        let old = AssetManifest {
            version: ManifestVersion {
                schema: 1,
                pack_version: "1.0.0".to_string(),
                min_compatible_version: None,
            },
            assets: BTreeMap::new(),
        };

        let new = AssetManifest {
            version: ManifestVersion {
                schema: 2,
                pack_version: "2.0.0".to_string(),
                min_compatible_version: None,
            },
            assets: BTreeMap::new(),
        };

        let result = check_compatibility(&old, &new);
        assert!(!result.compatible);
        assert!(result.issues.iter().any(|i| i.contains("Schema version")));
    }

    #[test]
    fn test_dependencies_resolved() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[]");
        write_file(dir.path(), "recipes.ron", "[]");
        write_file(dir.path(), "blocks.ron", "{}");
        write_file(dir.path(), "biomes.ron", "[]");

        let version = ManifestVersion::new("1.0.0");
        let generator = ManifestGenerator::new(dir.path(), version);
        let manifest = generator.generate().unwrap();

        let recipes = manifest
            .get_asset(&AssetId("recipes.ron".to_string()))
            .unwrap();
        assert!(
            recipes
                .dependencies
                .contains(&AssetId("items.ron".to_string()))
        );

        let items = manifest
            .get_asset(&AssetId("items.ron".to_string()))
            .unwrap();
        assert!(
            items
                .dependencies
                .contains(&AssetId("blocks.ron".to_string()))
        );

        let biomes = manifest
            .get_asset(&AssetId("biomes.ron".to_string()))
            .unwrap();
        assert!(
            biomes
                .dependencies
                .contains(&AssetId("blocks.ron".to_string()))
        );
    }

    #[test]
    fn test_nested_directory_scanning() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[]");
        write_file(dir.path(), "subdir/config.ron", "()");

        let version = ManifestVersion::new("1.0.0");
        let generator = ManifestGenerator::new(dir.path(), version);
        let manifest = generator.generate().unwrap();

        assert_eq!(manifest.assets.len(), 2);
        assert!(
            manifest
                .assets
                .contains_key(&AssetId("subdir/config.ron".to_string()))
        );
    }

    #[test]
    fn test_exclude_unknown_by_default() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[]");
        write_file(dir.path(), "unknown.xyz", "data");

        let version = ManifestVersion::new("1.0.0");
        let generator = ManifestGenerator::new(dir.path(), version);
        let manifest = generator.generate().unwrap();

        assert_eq!(manifest.assets.len(), 1);
    }

    #[test]
    fn test_include_unknown_when_enabled() {
        let dir = create_test_dir();
        write_file(dir.path(), "items.ron", "[]");
        write_file(dir.path(), "unknown.xyz", "data");

        let version = ManifestVersion::new("1.0.0");
        let generator = ManifestGenerator::new(dir.path(), version).include_unknown(true);
        let manifest = generator.generate().unwrap();

        assert_eq!(manifest.assets.len(), 2);
    }

    #[test]
    fn test_content_root_not_found() {
        let version = ManifestVersion::new("1.0.0");
        let generator = ManifestGenerator::new("/nonexistent/path", version);
        let result = generator.generate();
        assert!(matches!(result, Err(ManifestError::ContentRootNotFound(_))));
    }

    #[test]
    fn test_diff_display() {
        let diff = ManifestDiff {
            added: [(
                AssetId("new.ron".to_string()),
                AssetEntry {
                    id: AssetId("new.ron".to_string()),
                    path: PathBuf::from("new.ron"),
                    kind: AssetKind::Config,
                    hash: ContentHash::from_bytes(b"new"),
                    dependencies: BTreeSet::new(),
                },
            )]
            .into_iter()
            .collect(),
            removed: [AssetId("old.ron".to_string())].into_iter().collect(),
            modified: BTreeMap::new(),
        };

        let display = format!("{diff}");
        assert!(display.contains("1 added"));
        assert!(display.contains("1 removed"));
        assert!(display.contains("new.ron"));
        assert!(display.contains("old.ron"));
    }
}

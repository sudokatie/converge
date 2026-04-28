//! Biome pipeline extension system for game-specific terrain logic.
//!
//! The biome pipeline allows games to customize world generation without forking
//! the core generation code. Hooks can influence biome selection, terrain layers,
//! features, and resource placement while preserving deterministic ordering.
//!
//! # Deterministic Ordering
//!
//! All hooks are processed in a deterministic order based on their priority
//! and registration order. This ensures identical worlds are generated from
//! the same seed regardless of hook registration timing.
//!
//! # Example
//!
//! ```ignore
//! use engine_world::generation::{BiomePipeline, BiomePipelineHook, PipelineContext, BiomeInfluence};
//!
//! struct VolcanicBiomeHook;
//!
//! impl BiomePipelineHook for VolcanicBiomeHook {
//!     fn name(&self) -> &str { "volcanic" }
//!     fn priority(&self) -> HookPriority { HookPriority::LATE }
//!     fn process(&self, ctx: &PipelineContext) -> Option<BiomeInfluence> {
//!         if ctx.temperature() > 0.9 {
//!             Some(BiomeInfluence::new().with_surface_block(11)) // Basalt
//!         } else {
//!             None
//!         }
//!     }
//! }
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Biome, BiomeSelector};

/// Priority level for hook execution order.
///
/// Hooks with lower priority values execute first. Within the same priority,
/// hooks execute in registration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HookPriority(i32);

impl HookPriority {
    /// Earliest execution priority.
    pub const EARLIEST: Self = Self(-1000);
    /// Early execution priority.
    pub const EARLY: Self = Self(-100);
    /// Normal execution priority (default).
    pub const NORMAL: Self = Self(0);
    /// Late execution priority.
    pub const LATE: Self = Self(100);
    /// Latest execution priority.
    pub const LATEST: Self = Self(1000);

    /// Create a custom priority value.
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    /// Get the raw priority value.
    #[must_use]
    pub const fn value(&self) -> i32 {
        self.0
    }
}

impl Default for HookPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Terrain layer modification for a position.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TerrainLayers {
    /// Override surface block (topmost solid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<u16>,
    /// Override subsurface block (below surface).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsurface: Option<u16>,
    /// Override deep block (below subsurface).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep: Option<u16>,
    /// Depth of subsurface layer in blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsurface_depth: Option<u32>,
}

impl TerrainLayers {
    /// Create empty terrain layers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the surface block.
    #[must_use]
    pub fn with_surface(mut self, block: u16) -> Self {
        self.surface = Some(block);
        self
    }

    /// Set the subsurface block.
    #[must_use]
    pub fn with_subsurface(mut self, block: u16) -> Self {
        self.subsurface = Some(block);
        self
    }

    /// Set the deep block.
    #[must_use]
    pub fn with_deep(mut self, block: u16) -> Self {
        self.deep = Some(block);
        self
    }

    /// Set the subsurface depth.
    #[must_use]
    pub fn with_subsurface_depth(mut self, depth: u32) -> Self {
        self.subsurface_depth = Some(depth);
        self
    }

    /// Check if any layers are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.surface.is_none()
            && self.subsurface.is_none()
            && self.deep.is_none()
            && self.subsurface_depth.is_none()
    }

    /// Merge another layer specification, with `other` taking precedence.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            surface: other.surface.or(self.surface),
            subsurface: other.subsurface.or(self.subsurface),
            deep: other.deep.or(self.deep),
            subsurface_depth: other.subsurface_depth.or(self.subsurface_depth),
        }
    }
}

/// A feature to be placed at a location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeaturePlacement {
    /// Feature identifier (e.g., `oak_tree`, `boulder`).
    pub feature_id: String,
    /// Placement weight (higher = more likely when competing).
    pub weight: f32,
    /// Whether to replace existing features.
    pub replace_existing: bool,
}

impl FeaturePlacement {
    /// Create a new feature placement.
    #[must_use]
    pub fn new(feature_id: impl Into<String>) -> Self {
        Self {
            feature_id: feature_id.into(),
            weight: 1.0,
            replace_existing: false,
        }
    }

    /// Set the placement weight.
    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Set whether to replace existing features.
    #[must_use]
    pub fn replacing(mut self) -> Self {
        self.replace_existing = true;
        self
    }
}

/// A resource deposit at a location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceDeposit {
    /// Resource identifier (e.g., `iron_ore`, `coal`).
    pub resource_id: String,
    /// Density multiplier (1.0 = normal).
    pub density: f32,
    /// Minimum Y level for this deposit.
    pub min_y: Option<i32>,
    /// Maximum Y level for this deposit.
    pub max_y: Option<i32>,
}

impl ResourceDeposit {
    /// Create a new resource deposit.
    #[must_use]
    pub fn new(resource_id: impl Into<String>) -> Self {
        Self {
            resource_id: resource_id.into(),
            density: 1.0,
            min_y: None,
            max_y: None,
        }
    }

    /// Set the density multiplier.
    #[must_use]
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Set the Y level range.
    #[must_use]
    pub fn with_y_range(mut self, min: i32, max: i32) -> Self {
        self.min_y = Some(min);
        self.max_y = Some(max);
        self
    }
}

/// Output from a biome pipeline hook.
///
/// Influences are accumulated from all hooks and merged to produce the final
/// generation parameters for each position.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BiomeInfluence {
    /// Override the selected biome.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub biome_override: Option<Biome>,
    /// Height modifier to add to base terrain height.
    pub height_modifier: f64,
    /// Height scale multiplier (1.0 = no change).
    pub height_scale: f64,
    /// Terrain layer overrides.
    #[serde(skip_serializing_if = "TerrainLayers::is_empty", default)]
    pub layers: TerrainLayers,
    /// Features to potentially place.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub features: Vec<FeaturePlacement>,
    /// Resource deposits to potentially generate.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub resources: Vec<ResourceDeposit>,
    /// Tree density multiplier (1.0 = biome default).
    pub tree_density_multiplier: f64,
    /// Whether this location should have water at sea level.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub has_water: Option<bool>,
}

impl BiomeInfluence {
    /// Create an empty influence (no modifications).
    #[must_use]
    pub fn new() -> Self {
        Self {
            height_scale: 1.0,
            tree_density_multiplier: 1.0,
            ..Default::default()
        }
    }

    /// Override the biome selection.
    #[must_use]
    pub fn with_biome(mut self, biome: Biome) -> Self {
        self.biome_override = Some(biome);
        self
    }

    /// Add a height modifier.
    #[must_use]
    pub fn with_height_modifier(mut self, modifier: f64) -> Self {
        self.height_modifier = modifier;
        self
    }

    /// Set the height scale multiplier.
    #[must_use]
    pub fn with_height_scale(mut self, scale: f64) -> Self {
        self.height_scale = scale;
        self
    }

    /// Set the surface block.
    #[must_use]
    pub fn with_surface_block(mut self, block: u16) -> Self {
        self.layers.surface = Some(block);
        self
    }

    /// Set terrain layers.
    #[must_use]
    pub fn with_layers(mut self, layers: TerrainLayers) -> Self {
        self.layers = layers;
        self
    }

    /// Add a feature placement.
    #[must_use]
    pub fn with_feature(mut self, feature: FeaturePlacement) -> Self {
        self.features.push(feature);
        self
    }

    /// Add a resource deposit.
    #[must_use]
    pub fn with_resource(mut self, resource: ResourceDeposit) -> Self {
        self.resources.push(resource);
        self
    }

    /// Set the tree density multiplier.
    #[must_use]
    pub fn with_tree_density(mut self, multiplier: f64) -> Self {
        self.tree_density_multiplier = multiplier;
        self
    }

    /// Set whether the location has water.
    #[must_use]
    pub fn with_water(mut self, has_water: bool) -> Self {
        self.has_water = Some(has_water);
        self
    }

    /// Merge another influence into this one.
    ///
    /// Later influences take precedence for overrides. Modifiers and multipliers
    /// are combined additively/multiplicatively.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            biome_override: other.biome_override.or(self.biome_override),
            height_modifier: self.height_modifier + other.height_modifier,
            height_scale: self.height_scale * other.height_scale,
            layers: self.layers.merge(&other.layers),
            features: {
                let mut f = self.features.clone();
                f.extend(other.features.iter().cloned());
                f
            },
            resources: {
                let mut r = self.resources.clone();
                r.extend(other.resources.iter().cloned());
                r
            },
            tree_density_multiplier: self.tree_density_multiplier * other.tree_density_multiplier,
            has_water: other.has_water.or(self.has_water),
        }
    }

    /// Check if this influence makes any changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.biome_override.is_none()
            && (self.height_modifier.abs() < f64::EPSILON)
            && ((self.height_scale - 1.0).abs() < f64::EPSILON)
            && self.layers.is_empty()
            && self.features.is_empty()
            && self.resources.is_empty()
            && ((self.tree_density_multiplier - 1.0).abs() < f64::EPSILON)
            && self.has_water.is_none()
    }
}

/// Context provided to pipeline hooks for making decisions.
#[derive(Clone, Debug)]
pub struct PipelineContext<'a> {
    /// World X coordinate.
    pub x: f64,
    /// World Z coordinate.
    pub z: f64,
    /// World seed.
    pub seed: u64,
    /// Temperature at this location (0 = cold, 1 = hot).
    pub temperature: f64,
    /// Humidity at this location (0 = dry, 1 = wet).
    pub humidity: f64,
    /// Base biome selection (before hooks).
    pub base_biome: Biome,
    /// Reference to the biome selector.
    selector: &'a BiomeSelector,
}

impl<'a> PipelineContext<'a> {
    /// Create a new pipeline context.
    #[must_use]
    pub fn new(x: f64, z: f64, seed: u64, selector: &'a BiomeSelector) -> Self {
        let (temperature, humidity) = selector.sample(x, z);
        let base_biome = BiomeSelector::select_biome(temperature, humidity);

        Self {
            x,
            z,
            seed,
            temperature,
            humidity,
            base_biome,
            selector,
        }
    }

    /// Get the temperature value.
    #[must_use]
    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    /// Get the humidity value.
    #[must_use]
    pub fn humidity(&self) -> f64 {
        self.humidity
    }

    /// Get the base biome (before hook modifications).
    #[must_use]
    pub fn base_biome(&self) -> Biome {
        self.base_biome
    }

    /// Sample temperature at a nearby position.
    #[must_use]
    pub fn temperature_at(&self, x: f64, z: f64) -> f64 {
        self.selector.temperature_at(x, z)
    }

    /// Sample humidity at a nearby position.
    #[must_use]
    pub fn humidity_at(&self, x: f64, z: f64) -> f64 {
        self.selector.humidity_at(x, z)
    }

    /// Get a deterministic random value for this position.
    ///
    /// The value is in range [0, 1) and is seeded by position and world seed.
    #[must_use]
    #[expect(
        clippy::cast_sign_loss,
        reason = "hash function uses wrapping arithmetic"
    )]
    #[expect(
        clippy::cast_precision_loss,
        reason = "16-bit value fits in f64 mantissa"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation to i64 for hash is intentional"
    )]
    pub fn random(&self) -> f64 {
        let mut h = self.seed.wrapping_mul(31337);
        h = h.wrapping_add((self.x as i64) as u64 * 73_856_093);
        h = h.wrapping_add((self.z as i64) as u64 * 19_349_663);
        h ^= h >> 17;
        h = h.wrapping_mul(0xed5a_d4bb);
        h ^= h >> 11;
        (h & 0xFFFF) as f64 / 65536.0
    }

    /// Get a deterministic random value with an additional salt.
    #[must_use]
    #[expect(
        clippy::cast_sign_loss,
        reason = "hash function uses wrapping arithmetic"
    )]
    #[expect(
        clippy::cast_precision_loss,
        reason = "16-bit value fits in f64 mantissa"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation to i64 for hash is intentional"
    )]
    pub fn random_with_salt(&self, salt: u64) -> f64 {
        let mut h = self.seed.wrapping_mul(31337).wrapping_add(salt);
        h = h.wrapping_add((self.x as i64) as u64 * 73_856_093);
        h = h.wrapping_add((self.z as i64) as u64 * 19_349_663);
        h ^= h >> 17;
        h = h.wrapping_mul(0xed5a_d4bb);
        h ^= h >> 11;
        (h & 0xFFFF) as f64 / 65536.0
    }
}

/// Trait for biome pipeline hooks.
///
/// Hooks are called in priority order for each position during world generation.
/// They can influence biome selection, terrain layers, features, and resources.
pub trait BiomePipelineHook: Send + Sync {
    /// Unique name for this hook.
    fn name(&self) -> &str;

    /// Execution priority (lower values execute first).
    fn priority(&self) -> HookPriority {
        HookPriority::NORMAL
    }

    /// Check if this hook is enabled for the given context.
    ///
    /// Return `false` to skip processing entirely.
    fn is_enabled(&self, _ctx: &PipelineContext<'_>) -> bool {
        true
    }

    /// Process the context and optionally return an influence.
    ///
    /// Return `None` if this hook doesn't want to influence this position.
    fn process(&self, ctx: &PipelineContext<'_>) -> Option<BiomeInfluence>;
}

/// Hook registration entry with ordering metadata.
struct HookEntry {
    hook: Box<dyn BiomePipelineHook>,
    priority: HookPriority,
    registration_order: u64,
    enabled: bool,
}

/// Configuration for a hook's enabled state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HookConfig {
    /// Whether the hook is enabled.
    pub enabled: bool,
}

impl HookConfig {
    /// Create a new enabled hook config.
    #[must_use]
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Create a new disabled hook config.
    #[must_use]
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

/// Serializable configuration for the biome pipeline.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BiomePipelineConfig {
    /// Per-hook enabled state, keyed by hook name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub hooks: BTreeMap<String, HookConfig>,
}

impl BiomePipelineConfig {
    /// Create an empty config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the enabled state for a hook.
    pub fn set_hook_enabled(&mut self, name: &str, enabled: bool) {
        self.hooks
            .entry(name.to_string())
            .or_insert_with(HookConfig::enabled)
            .enabled = enabled;
    }

    /// Get the enabled state for a hook.
    #[must_use]
    pub fn is_hook_enabled(&self, name: &str) -> Option<bool> {
        self.hooks.get(name).map(|c| c.enabled)
    }
}

/// The biome pipeline registry and processor.
///
/// Manages hooks and processes them in deterministic order to produce
/// combined influences for world generation.
pub struct BiomePipeline {
    hooks: Vec<HookEntry>,
    next_order: u64,
    config: BiomePipelineConfig,
    sorted: bool,
}

impl BiomePipeline {
    /// Create a new empty pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
            next_order: 0,
            config: BiomePipelineConfig::new(),
            sorted: true,
        }
    }

    /// Create a pipeline with the given configuration.
    #[must_use]
    pub fn with_config(config: BiomePipelineConfig) -> Self {
        Self {
            hooks: Vec::new(),
            next_order: 0,
            config,
            sorted: true,
        }
    }

    /// Get the pipeline configuration.
    #[must_use]
    pub fn config(&self) -> &BiomePipelineConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut BiomePipelineConfig {
        &mut self.config
    }

    /// Set the pipeline configuration.
    pub fn set_config(&mut self, config: BiomePipelineConfig) {
        self.config = config;
        self.apply_config();
    }

    /// Register a new hook.
    ///
    /// Returns the hook name for reference.
    pub fn register(&mut self, hook: impl BiomePipelineHook + 'static) -> String {
        let name = hook.name().to_string();
        let priority = hook.priority();

        let enabled = self.config.is_hook_enabled(&name).unwrap_or(true);

        self.hooks.push(HookEntry {
            hook: Box::new(hook),
            priority,
            registration_order: self.next_order,
            enabled,
        });

        self.next_order += 1;
        self.sorted = false;

        name
    }

    /// Unregister a hook by name.
    ///
    /// Returns `true` if a hook was removed.
    pub fn unregister(&mut self, name: &str) -> bool {
        let before = self.hooks.len();
        self.hooks.retain(|e| e.hook.name() != name);
        self.hooks.len() < before
    }

    /// Get the number of registered hooks.
    #[must_use]
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Check if a hook is registered.
    #[must_use]
    pub fn has_hook(&self, name: &str) -> bool {
        self.hooks.iter().any(|e| e.hook.name() == name)
    }

    /// Get hook names in execution order.
    #[must_use]
    pub fn hook_names(&mut self) -> Vec<String> {
        self.ensure_sorted();
        self.hooks
            .iter()
            .map(|e| e.hook.name().to_string())
            .collect()
    }

    /// Enable or disable a hook by name.
    ///
    /// Returns `true` if the hook was found.
    pub fn set_hook_enabled(&mut self, name: &str, enabled: bool) -> bool {
        self.config.set_hook_enabled(name, enabled);

        for entry in &mut self.hooks {
            if entry.hook.name() == name {
                entry.enabled = enabled;
                return true;
            }
        }
        false
    }

    /// Check if a hook is enabled.
    #[must_use]
    pub fn is_hook_enabled(&self, name: &str) -> Option<bool> {
        self.hooks
            .iter()
            .find(|e| e.hook.name() == name)
            .map(|e| e.enabled)
    }

    /// Process all hooks for a position and return the combined influence.
    #[must_use]
    pub fn process(&mut self, ctx: &PipelineContext<'_>) -> BiomeInfluence {
        self.ensure_sorted();

        let mut result = BiomeInfluence::new();

        for entry in &self.hooks {
            if !entry.enabled {
                continue;
            }

            if !entry.hook.is_enabled(ctx) {
                continue;
            }

            if let Some(influence) = entry.hook.process(ctx) {
                result = result.merge(&influence);
            }
        }

        result
    }

    /// Process and get the final biome for a position.
    ///
    /// Convenience method that processes hooks and returns the effective biome.
    #[must_use]
    pub fn biome_at(&mut self, ctx: &PipelineContext<'_>) -> Biome {
        let influence = self.process(ctx);
        influence.biome_override.unwrap_or(ctx.base_biome)
    }

    /// Clear all registered hooks.
    pub fn clear(&mut self) {
        self.hooks.clear();
        self.next_order = 0;
        self.sorted = true;
    }

    fn ensure_sorted(&mut self) {
        if !self.sorted {
            self.hooks.sort_by(|a, b| {
                a.priority
                    .cmp(&b.priority)
                    .then_with(|| a.registration_order.cmp(&b.registration_order))
            });
            self.sorted = true;
        }
    }

    fn apply_config(&mut self) {
        for entry in &mut self.hooks {
            if let Some(enabled) = self.config.is_hook_enabled(entry.hook.name()) {
                entry.enabled = enabled;
            }
        }
    }
}

impl Default for BiomePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BiomePipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BiomePipeline")
            .field("hook_count", &self.hooks.len())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHook {
        name: String,
        priority: HookPriority,
        influence: BiomeInfluence,
    }

    impl TestHook {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                priority: HookPriority::NORMAL,
                influence: BiomeInfluence::new(),
            }
        }

        fn with_priority(mut self, priority: HookPriority) -> Self {
            self.priority = priority;
            self
        }

        fn with_influence(mut self, influence: BiomeInfluence) -> Self {
            self.influence = influence;
            self
        }
    }

    impl BiomePipelineHook for TestHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> HookPriority {
            self.priority
        }

        fn process(&self, _ctx: &PipelineContext<'_>) -> Option<BiomeInfluence> {
            Some(self.influence.clone())
        }
    }

    struct ConditionalHook {
        name: String,
        temp_threshold: f64,
    }

    impl ConditionalHook {
        fn new(name: &str, temp_threshold: f64) -> Self {
            Self {
                name: name.to_string(),
                temp_threshold,
            }
        }
    }

    impl BiomePipelineHook for ConditionalHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn is_enabled(&self, ctx: &PipelineContext<'_>) -> bool {
            ctx.temperature() > self.temp_threshold
        }

        fn process(&self, _ctx: &PipelineContext<'_>) -> Option<BiomeInfluence> {
            Some(BiomeInfluence::new().with_biome(Biome::Desert))
        }
    }

    fn make_context(x: f64, z: f64) -> (BiomeSelector, f64, f64, u64) {
        let seed = 12345_u64;
        let selector = BiomeSelector::new(seed);
        (selector, x, z, seed)
    }

    #[test]
    fn hook_priority_ordering() {
        assert!(HookPriority::EARLIEST < HookPriority::EARLY);
        assert!(HookPriority::EARLY < HookPriority::NORMAL);
        assert!(HookPriority::NORMAL < HookPriority::LATE);
        assert!(HookPriority::LATE < HookPriority::LATEST);
    }

    #[test]
    fn terrain_layers_empty() {
        let layers = TerrainLayers::new();
        assert!(layers.is_empty());
    }

    #[test]
    fn terrain_layers_merge() {
        let base = TerrainLayers::new().with_surface(1).with_subsurface(2);
        let overlay = TerrainLayers::new().with_surface(10);

        let merged = base.merge(&overlay);
        assert_eq!(merged.surface, Some(10));
        assert_eq!(merged.subsurface, Some(2));
    }

    #[test]
    fn biome_influence_empty() {
        let influence = BiomeInfluence::new();
        assert!(influence.is_empty());
    }

    #[test]
    fn biome_influence_merge() {
        let i1 = BiomeInfluence::new()
            .with_height_modifier(5.0)
            .with_height_scale(1.5);

        let i2 = BiomeInfluence::new()
            .with_height_modifier(3.0)
            .with_biome(Biome::Mountains);

        let merged = i1.merge(&i2);
        assert!((merged.height_modifier - 8.0).abs() < f64::EPSILON);
        assert!((merged.height_scale - 1.5).abs() < f64::EPSILON);
        assert_eq!(merged.biome_override, Some(Biome::Mountains));
    }

    #[test]
    fn biome_influence_features_accumulate() {
        let i1 = BiomeInfluence::new().with_feature(FeaturePlacement::new("tree"));
        let i2 = BiomeInfluence::new().with_feature(FeaturePlacement::new("rock"));

        let merged = i1.merge(&i2);
        assert_eq!(merged.features.len(), 2);
    }

    #[test]
    fn pipeline_context_deterministic_random() {
        let (selector, x, z, seed) = make_context(100.0, 200.0);
        let ctx1 = PipelineContext::new(x, z, seed, &selector);
        let ctx2 = PipelineContext::new(x, z, seed, &selector);

        assert!((ctx1.random() - ctx2.random()).abs() < f64::EPSILON);
    }

    #[test]
    fn pipeline_context_random_varies_with_position() {
        let (selector, _, _, seed) = make_context(100.0, 200.0);
        let ctx1 = PipelineContext::new(100.0, 200.0, seed, &selector);
        let ctx2 = PipelineContext::new(100.0, 201.0, seed, &selector);

        assert!((ctx1.random() - ctx2.random()).abs() > f64::EPSILON);
    }

    #[test]
    fn pipeline_context_random_with_salt() {
        let (selector, x, z, seed) = make_context(100.0, 200.0);
        let ctx = PipelineContext::new(x, z, seed, &selector);

        let r1 = ctx.random_with_salt(1);
        let r2 = ctx.random_with_salt(2);

        assert!((r1 - r2).abs() > f64::EPSILON);
    }

    #[test]
    fn pipeline_register_and_process() {
        let mut pipeline = BiomePipeline::new();
        let hook =
            TestHook::new("test").with_influence(BiomeInfluence::new().with_biome(Biome::Forest));

        pipeline.register(hook);
        assert_eq!(pipeline.hook_count(), 1);

        let (selector, x, z, seed) = make_context(100.0, 200.0);
        let ctx = PipelineContext::new(x, z, seed, &selector);

        let biome = pipeline.biome_at(&ctx);
        assert_eq!(biome, Biome::Forest);
    }

    #[test]
    fn pipeline_deterministic_ordering() {
        let mut pipeline1 = BiomePipeline::new();
        let mut pipeline2 = BiomePipeline::new();

        for i in 0..5 {
            let hook = TestHook::new(&format!("hook_{i}"));
            pipeline1.register(hook);
        }
        for i in 0..5 {
            let hook = TestHook::new(&format!("hook_{i}"));
            pipeline2.register(hook);
        }

        let names1 = pipeline1.hook_names();
        let names2 = pipeline2.hook_names();
        assert_eq!(names1, names2);
    }

    #[test]
    fn pipeline_priority_ordering() {
        let mut pipeline = BiomePipeline::new();

        pipeline.register(TestHook::new("late").with_priority(HookPriority::LATE));
        pipeline.register(TestHook::new("early").with_priority(HookPriority::EARLY));
        pipeline.register(TestHook::new("normal").with_priority(HookPriority::NORMAL));

        let names = pipeline.hook_names();
        assert_eq!(names, vec!["early", "normal", "late"]);
    }

    #[test]
    fn pipeline_registration_order_tiebreak() {
        let mut pipeline = BiomePipeline::new();

        pipeline.register(TestHook::new("first"));
        pipeline.register(TestHook::new("second"));
        pipeline.register(TestHook::new("third"));

        let names = pipeline.hook_names();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn pipeline_unregister() {
        let mut pipeline = BiomePipeline::new();
        pipeline.register(TestHook::new("keep"));
        pipeline.register(TestHook::new("remove"));

        assert!(pipeline.unregister("remove"));
        assert!(!pipeline.unregister("nonexistent"));
        assert_eq!(pipeline.hook_count(), 1);
        assert!(pipeline.has_hook("keep"));
        assert!(!pipeline.has_hook("remove"));
    }

    #[test]
    fn pipeline_disable_hook() {
        let mut pipeline = BiomePipeline::new();
        pipeline.register(
            TestHook::new("modifying")
                .with_influence(BiomeInfluence::new().with_biome(Biome::Desert)),
        );

        let (selector, x, z, seed) = make_context(100.0, 200.0);
        let ctx = PipelineContext::new(x, z, seed, &selector);

        let biome1 = pipeline.biome_at(&ctx);
        assert_eq!(biome1, Biome::Desert);

        pipeline.set_hook_enabled("modifying", false);
        let ctx = PipelineContext::new(x, z, seed, &selector);
        let biome2 = pipeline.biome_at(&ctx);
        assert_eq!(biome2, ctx.base_biome);
    }

    #[test]
    fn pipeline_conditional_hook() {
        let mut pipeline = BiomePipeline::new();
        pipeline.register(ConditionalHook::new("hot_only", 0.99));

        let selector = BiomeSelector::new(12345);

        let ctx_cold = PipelineContext::new(0.0, 0.0, 12345, &selector);
        let biome_cold = pipeline.biome_at(&ctx_cold);
        assert_eq!(biome_cold, ctx_cold.base_biome);
    }

    #[test]
    fn pipeline_config_persistence() {
        let mut config = BiomePipelineConfig::new();
        config.set_hook_enabled("test", false);

        let mut pipeline = BiomePipeline::with_config(config.clone());
        pipeline.register(
            TestHook::new("test").with_influence(BiomeInfluence::new().with_biome(Biome::Ocean)),
        );

        assert_eq!(pipeline.is_hook_enabled("test"), Some(false));

        let (selector, x, z, seed) = make_context(100.0, 200.0);
        let ctx = PipelineContext::new(x, z, seed, &selector);
        let biome = pipeline.biome_at(&ctx);
        assert_eq!(biome, ctx.base_biome);
    }

    #[test]
    fn pipeline_default_behavior() {
        let mut pipeline = BiomePipeline::new();

        let (selector, x, z, seed) = make_context(100.0, 200.0);
        let ctx = PipelineContext::new(x, z, seed, &selector);

        let influence = pipeline.process(&ctx);
        assert!(influence.is_empty());

        let biome = pipeline.biome_at(&ctx);
        assert_eq!(biome, ctx.base_biome);
    }

    #[test]
    fn pipeline_clear() {
        let mut pipeline = BiomePipeline::new();
        pipeline.register(TestHook::new("a"));
        pipeline.register(TestHook::new("b"));

        pipeline.clear();
        assert_eq!(pipeline.hook_count(), 0);
    }

    #[test]
    fn feature_placement_builder() {
        let feature = FeaturePlacement::new("oak_tree")
            .with_weight(2.0)
            .replacing();

        assert_eq!(feature.feature_id, "oak_tree");
        assert!((feature.weight - 2.0).abs() < f32::EPSILON);
        assert!(feature.replace_existing);
    }

    #[test]
    fn resource_deposit_builder() {
        let resource = ResourceDeposit::new("iron_ore")
            .with_density(1.5)
            .with_y_range(0, 64);

        assert_eq!(resource.resource_id, "iron_ore");
        assert!((resource.density - 1.5).abs() < f32::EPSILON);
        assert_eq!(resource.min_y, Some(0));
        assert_eq!(resource.max_y, Some(64));
    }

    #[test]
    fn serde_hook_priority() {
        let priority = HookPriority::LATE;
        let json = serde_json::to_string(&priority).unwrap();
        let recovered: HookPriority = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, priority);
    }

    #[test]
    fn serde_terrain_layers() {
        let layers = TerrainLayers::new()
            .with_surface(1)
            .with_subsurface(2)
            .with_subsurface_depth(4);

        let json = serde_json::to_string(&layers).unwrap();
        let recovered: TerrainLayers = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, layers);
    }

    #[test]
    fn serde_biome_influence() {
        let influence = BiomeInfluence::new()
            .with_biome(Biome::Mountains)
            .with_height_modifier(10.0)
            .with_feature(FeaturePlacement::new("tree"))
            .with_resource(ResourceDeposit::new("coal"));

        let json = serde_json::to_string(&influence).unwrap();
        let recovered: BiomeInfluence = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.biome_override, influence.biome_override);
        assert!((recovered.height_modifier - influence.height_modifier).abs() < f64::EPSILON);
        assert_eq!(recovered.features.len(), 1);
        assert_eq!(recovered.resources.len(), 1);
    }

    #[test]
    fn serde_pipeline_config() {
        let mut config = BiomePipelineConfig::new();
        config.set_hook_enabled("hook_a", true);
        config.set_hook_enabled("hook_b", false);

        let json = serde_json::to_string(&config).unwrap();
        let recovered: BiomePipelineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.is_hook_enabled("hook_a"), Some(true));
        assert_eq!(recovered.is_hook_enabled("hook_b"), Some(false));
    }

    #[test]
    fn influence_tree_density_multiplier() {
        let i1 = BiomeInfluence::new().with_tree_density(0.5);
        let i2 = BiomeInfluence::new().with_tree_density(0.5);

        let merged = i1.merge(&i2);
        assert!((merged.tree_density_multiplier - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn influence_water_override() {
        let i1 = BiomeInfluence::new().with_water(false);
        let i2 = BiomeInfluence::new().with_water(true);

        let merged = i1.merge(&i2);
        assert_eq!(merged.has_water, Some(true));
    }
}

//! Game pack registry for managing registered packs and their content.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{
    activation::{ActivationPlan, DependencyResolver},
    descriptor::{
        BlockDescriptor, HazardDescriptor, RuleProfileDescriptor, ShaderDescriptor,
        SystemDescriptor,
    },
    error::{GamePackError, GamePackResult},
    fingerprint::{FingerprintBuilder, PackFingerprint},
    id::{BlockId, HazardId, PackId, RuleProfileId, ShaderId, SystemId},
    manifest::PackManifest,
};

/// ID generator for deterministic ID allocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IdGenerator {
    seed: u32,
    next_sequence: u32,
}

impl IdGenerator {
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            next_sequence: 0,
        }
    }

    pub fn generate_pack_id(&mut self) -> PackId {
        let id = PackId::new(self.seed, self.next_sequence);
        self.next_sequence = self.next_sequence.wrapping_add(1);
        id
    }
}

/// A registered game pack with all its content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredPack {
    pub id: PackId,
    pub manifest: PackManifest,
    pub blocks: Vec<BlockDescriptor>,
    pub systems: Vec<SystemDescriptor>,
    pub hazards: Vec<HazardDescriptor>,
    pub shaders: Vec<ShaderDescriptor>,
    pub rule_profiles: Vec<RuleProfileDescriptor>,
    pub fingerprint: PackFingerprint,
}

impl RegisteredPack {
    #[must_use]
    pub fn new(id: PackId, manifest: PackManifest) -> Self {
        Self {
            id,
            manifest,
            blocks: Vec::new(),
            systems: Vec::new(),
            hazards: Vec::new(),
            shaders: Vec::new(),
            rule_profiles: Vec::new(),
            fingerprint: PackFingerprint::default(),
        }
    }

    pub fn add_block(&mut self, block: BlockDescriptor) {
        self.blocks.push(block);
    }

    pub fn add_system(&mut self, system: SystemDescriptor) {
        self.systems.push(system);
    }

    pub fn add_hazard(&mut self, hazard: HazardDescriptor) {
        self.hazards.push(hazard);
    }

    pub fn add_shader(&mut self, shader: ShaderDescriptor) {
        self.shaders.push(shader);
    }

    pub fn add_rule_profile(&mut self, profile: RuleProfileDescriptor) {
        self.rule_profiles.push(profile);
    }

    pub fn compute_fingerprint(&mut self) {
        let mut builder = FingerprintBuilder::new();

        builder.add(&self.manifest.name);
        builder.add(&self.manifest.version);

        for block in &self.blocks {
            builder.add(&block.id.raw());
            builder.add(&block.name);
        }

        for system in &self.systems {
            builder.add(&system.id.raw());
            builder.add(&system.name);
        }

        for hazard in &self.hazards {
            builder.add(&hazard.id.raw());
            builder.add(&hazard.name);
        }

        for shader in &self.shaders {
            builder.add(&shader.id.raw());
            builder.add(&shader.name);
        }

        for profile in &self.rule_profiles {
            builder.add(&profile.id.raw());
            builder.add(&profile.name);
        }

        self.fingerprint = builder.finish();
    }

    #[must_use]
    pub fn content_count(&self) -> usize {
        self.blocks.len()
            + self.systems.len()
            + self.hazards.len()
            + self.shaders.len()
            + self.rule_profiles.len()
    }
}

/// Query parameters for searching packs.
#[derive(Clone, Debug, Default)]
pub struct PackQuery {
    pub enabled_only: bool,
    pub name_contains: Option<String>,
    pub has_blocks: Option<bool>,
    pub has_systems: Option<bool>,
    pub min_version: Option<super::manifest::PackVersion>,
}

impl PackQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn enabled_only(mut self) -> Self {
        self.enabled_only = true;
        self
    }

    #[must_use]
    pub fn name_contains(mut self, pattern: impl Into<String>) -> Self {
        self.name_contains = Some(pattern.into());
        self
    }

    #[must_use]
    pub fn with_blocks(mut self) -> Self {
        self.has_blocks = Some(true);
        self
    }

    #[must_use]
    pub fn with_systems(mut self) -> Self {
        self.has_systems = Some(true);
        self
    }

    fn matches(&self, pack: &RegisteredPack) -> bool {
        if self.enabled_only && !pack.manifest.enabled {
            return false;
        }

        if let Some(ref pattern) = self.name_contains
            && !pack.manifest.name.contains(pattern)
        {
            return false;
        }

        if let Some(needs_blocks) = self.has_blocks
            && needs_blocks == pack.blocks.is_empty()
        {
            return false;
        }

        if let Some(needs_systems) = self.has_systems
            && needs_systems == pack.systems.is_empty()
        {
            return false;
        }

        if let Some(ref min_ver) = self.min_version
            && &pack.manifest.version < min_ver
        {
            return false;
        }

        true
    }
}

/// Compatibility report for a set of packs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CompatibilityReport {
    #[must_use]
    pub fn ok() -> Self {
        Self {
            compatible: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.compatible = false;
        self.errors.push(error.into());
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    #[must_use]
    pub const fn is_compatible(&self) -> bool {
        self.compatible
    }
}

/// Main registry for game packs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GamePackRegistry {
    packs: BTreeMap<PackId, RegisteredPack>,
    name_index: HashMap<String, PackId>,
    block_index: HashMap<BlockId, PackId>,
    system_index: HashMap<SystemId, PackId>,
    hazard_index: HashMap<HazardId, PackId>,
    shader_index: HashMap<ShaderId, PackId>,
    rule_profile_index: HashMap<RuleProfileId, PackId>,
    id_gen: IdGenerator,
}

impl GamePackRegistry {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            packs: BTreeMap::new(),
            name_index: HashMap::new(),
            block_index: HashMap::new(),
            system_index: HashMap::new(),
            hazard_index: HashMap::new(),
            shader_index: HashMap::new(),
            rule_profile_index: HashMap::new(),
            id_gen: IdGenerator::new(seed),
        }
    }

    /// Generate a new pack ID.
    pub fn generate_id(&mut self) -> PackId {
        self.id_gen.generate_pack_id()
    }

    /// Register a new game pack.
    ///
    /// # Errors
    /// Returns an error if the pack name or ID is already registered, or if any
    /// content IDs conflict with existing content.
    pub fn register(&mut self, mut pack: RegisteredPack) -> GamePackResult<PackId> {
        if self.name_index.contains_key(&pack.manifest.name) {
            return Err(GamePackError::DuplicatePackName(pack.manifest.name.clone()));
        }

        if self.packs.contains_key(&pack.id) {
            return Err(GamePackError::DuplicatePackId(pack.id));
        }

        self.validate_no_duplicate_content(&pack)?;

        pack.compute_fingerprint();

        let pack_id = pack.id;
        let pack_name = pack.manifest.name.clone();

        for block in &pack.blocks {
            self.block_index.insert(block.id, pack_id);
        }
        for system in &pack.systems {
            self.system_index.insert(system.id, pack_id);
        }
        for hazard in &pack.hazards {
            self.hazard_index.insert(hazard.id, pack_id);
        }
        for shader in &pack.shaders {
            self.shader_index.insert(shader.id, pack_id);
        }
        for profile in &pack.rule_profiles {
            self.rule_profile_index.insert(profile.id, pack_id);
        }

        self.name_index.insert(pack_name, pack_id);
        self.packs.insert(pack_id, pack);

        Ok(pack_id)
    }

    fn validate_no_duplicate_content(&self, pack: &RegisteredPack) -> GamePackResult<()> {
        let mut seen_blocks: HashSet<BlockId> = HashSet::new();
        for block in &pack.blocks {
            if !seen_blocks.insert(block.id) {
                return Err(GamePackError::DuplicateBlockId {
                    pack: pack.id,
                    block: block.name.clone(),
                });
            }
            if self.block_index.contains_key(&block.id) {
                return Err(GamePackError::DuplicateBlockId {
                    pack: pack.id,
                    block: block.name.clone(),
                });
            }
        }

        let mut seen_systems: HashSet<SystemId> = HashSet::new();
        for system in &pack.systems {
            if !seen_systems.insert(system.id) {
                return Err(GamePackError::DuplicateSystemId {
                    pack: pack.id,
                    system: system.name.clone(),
                });
            }
            if self.system_index.contains_key(&system.id) {
                return Err(GamePackError::DuplicateSystemId {
                    pack: pack.id,
                    system: system.name.clone(),
                });
            }
        }

        let mut seen_hazards: HashSet<HazardId> = HashSet::new();
        for hazard in &pack.hazards {
            if !seen_hazards.insert(hazard.id) {
                return Err(GamePackError::DuplicateHazardId {
                    pack: pack.id,
                    hazard: hazard.name.clone(),
                });
            }
            if self.hazard_index.contains_key(&hazard.id) {
                return Err(GamePackError::DuplicateHazardId {
                    pack: pack.id,
                    hazard: hazard.name.clone(),
                });
            }
        }

        let mut seen_shaders: HashSet<ShaderId> = HashSet::new();
        for shader in &pack.shaders {
            if !seen_shaders.insert(shader.id) {
                return Err(GamePackError::DuplicateShaderId {
                    pack: pack.id,
                    shader: shader.name.clone(),
                });
            }
            if self.shader_index.contains_key(&shader.id) {
                return Err(GamePackError::DuplicateShaderId {
                    pack: pack.id,
                    shader: shader.name.clone(),
                });
            }
        }

        let mut seen_profiles: HashSet<RuleProfileId> = HashSet::new();
        for profile in &pack.rule_profiles {
            if !seen_profiles.insert(profile.id) {
                return Err(GamePackError::DuplicateRuleProfileId {
                    pack: pack.id,
                    profile: profile.name.clone(),
                });
            }
            if self.rule_profile_index.contains_key(&profile.id) {
                return Err(GamePackError::DuplicateRuleProfileId {
                    pack: pack.id,
                    profile: profile.name.clone(),
                });
            }
        }

        Ok(())
    }

    /// Unregister a game pack.
    pub fn unregister(&mut self, id: PackId) -> Option<RegisteredPack> {
        if let Some(pack) = self.packs.remove(&id) {
            self.name_index.remove(&pack.manifest.name);

            for block in &pack.blocks {
                self.block_index.remove(&block.id);
            }
            for system in &pack.systems {
                self.system_index.remove(&system.id);
            }
            for hazard in &pack.hazards {
                self.hazard_index.remove(&hazard.id);
            }
            for shader in &pack.shaders {
                self.shader_index.remove(&shader.id);
            }
            for profile in &pack.rule_profiles {
                self.rule_profile_index.remove(&profile.id);
            }

            Some(pack)
        } else {
            None
        }
    }

    /// Get a pack by ID.
    #[must_use]
    pub fn get(&self, id: PackId) -> Option<&RegisteredPack> {
        self.packs.get(&id)
    }

    /// Get a mutable pack by ID.
    pub fn get_mut(&mut self, id: PackId) -> Option<&mut RegisteredPack> {
        self.packs.get_mut(&id)
    }

    /// Get a pack by name.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<&RegisteredPack> {
        self.name_index.get(name).and_then(|id| self.packs.get(id))
    }

    /// Get pack count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    /// Check if registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Iterate over all packs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &RegisteredPack> {
        self.packs.values()
    }

    /// Iterate over pack IDs in deterministic order.
    pub fn iter_ids(&self) -> impl Iterator<Item = PackId> + '_ {
        self.packs.keys().copied()
    }

    /// Query packs with filter.
    #[must_use]
    pub fn query(&self, query: &PackQuery) -> Vec<&RegisteredPack> {
        let mut results: Vec<_> = self.packs.values().filter(|p| query.matches(p)).collect();

        results.sort_by(|a, b| {
            a.manifest
                .load_priority
                .cmp(&b.manifest.load_priority)
                .reverse()
                .then(a.manifest.name.cmp(&b.manifest.name))
        });

        results
    }

    /// Get all blocks across all packs, sorted.
    #[must_use]
    pub fn all_blocks(&self) -> Vec<&BlockDescriptor> {
        let mut blocks: Vec<_> = self.packs.values().flat_map(|p| &p.blocks).collect();
        blocks.sort_by_key(|b| b.id);
        blocks
    }

    /// Get all systems across all packs, sorted by phase and order.
    #[must_use]
    pub fn all_systems(&self) -> Vec<&SystemDescriptor> {
        let mut systems: Vec<_> = self.packs.values().flat_map(|p| &p.systems).collect();
        systems.sort_by_key(|s| (s.effective_order(), s.id));
        systems
    }

    /// Get all hazards across all packs, sorted.
    #[must_use]
    pub fn all_hazards(&self) -> Vec<&HazardDescriptor> {
        let mut hazards: Vec<_> = self.packs.values().flat_map(|p| &p.hazards).collect();
        hazards.sort_by_key(|h| h.id);
        hazards
    }

    /// Get all shaders across all packs, sorted.
    #[must_use]
    pub fn all_shaders(&self) -> Vec<&ShaderDescriptor> {
        let mut shaders: Vec<_> = self.packs.values().flat_map(|p| &p.shaders).collect();
        shaders.sort_by_key(|s| s.id);
        shaders
    }

    /// Get all rule profiles across all packs, sorted.
    #[must_use]
    pub fn all_rule_profiles(&self) -> Vec<&RuleProfileDescriptor> {
        let mut profiles: Vec<_> = self.packs.values().flat_map(|p| &p.rule_profiles).collect();
        profiles.sort_by_key(|p| p.id);
        profiles
    }

    /// Find which pack owns a block.
    #[must_use]
    pub fn block_owner(&self, id: BlockId) -> Option<PackId> {
        self.block_index.get(&id).copied()
    }

    /// Find which pack owns a system.
    #[must_use]
    pub fn system_owner(&self, id: SystemId) -> Option<PackId> {
        self.system_index.get(&id).copied()
    }

    /// Generate an activation plan using dependency resolution.
    ///
    /// # Errors
    /// Returns an error if dependency resolution fails (e.g., due to cycles).
    pub fn generate_activation_plan(&self) -> GamePackResult<Vec<ActivationPlan>> {
        let mut resolver = DependencyResolver::new();

        for pack in self.packs.values() {
            resolver.add_pack(pack.id, pack.manifest.clone())?;
        }

        resolver.resolve()
    }

    /// Check compatibility of all registered packs.
    #[must_use]
    pub fn check_compatibility(&self) -> CompatibilityReport {
        let mut report = CompatibilityReport::ok();

        let mut resolver = DependencyResolver::new();
        for pack in self.packs.values() {
            if let Err(e) = resolver.add_pack(pack.id, pack.manifest.clone()) {
                report.add_error(format!("Failed to add pack {}: {e}", pack.manifest.name));
            }
        }

        match resolver.resolve() {
            Ok(plans) => {
                for plan in plans {
                    match &plan.status {
                        super::activation::ActivationStatus::Pending { waiting_on } => {
                            report.add_warning(format!(
                                "Pack {} waiting on: {}",
                                plan.pack_name,
                                waiting_on.join(", ")
                            ));
                        }
                        super::activation::ActivationStatus::Failed { reason } => {
                            report.add_error(format!("Pack {} failed: {reason}", plan.pack_name));
                        }
                        super::activation::ActivationStatus::Incompatible { conflicts } => {
                            report.add_error(format!(
                                "Pack {} incompatible: {}",
                                plan.pack_name,
                                conflicts.join(", ")
                            ));
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                report.add_error(format!("Dependency resolution failed: {e}"));
            }
        }

        let conflicts = resolver.check_capability_conflicts();
        for (cap, pack_ids) in conflicts {
            let names: Vec<_> = pack_ids
                .iter()
                .filter_map(|id| self.packs.get(id).map(|p| p.manifest.name.as_str()))
                .collect();
            report.add_error(format!(
                "Capability conflict {:?}: {}",
                cap,
                names.join(", ")
            ));
        }

        report
    }

    /// Compute a combined fingerprint for all packs.
    #[must_use]
    pub fn combined_fingerprint(&self) -> PackFingerprint {
        let fingerprints: Vec<_> = self.packs.values().map(|p| p.fingerprint).collect();
        PackFingerprint::combine(&fingerprints)
    }

    /// Enable or disable a pack.
    pub fn set_enabled(&mut self, id: PackId, enabled: bool) -> bool {
        if let Some(pack) = self.packs.get_mut(&id) {
            pack.manifest.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get enabled packs in activation order.
    #[must_use]
    pub fn enabled_in_order(&self) -> Vec<&RegisteredPack> {
        self.generate_activation_plan()
            .map(|plans| {
                plans
                    .into_iter()
                    .filter(|p| p.status.is_ready())
                    .filter_map(|p| self.packs.get(&p.pack_id))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_pack::manifest::{PackDependency, PackVersion};

    fn make_pack(registry: &mut GamePackRegistry, name: &str) -> RegisteredPack {
        let id = registry.generate_id();
        RegisteredPack::new(id, PackManifest::new(name, PackVersion::new(1, 0, 0)))
    }

    #[test]
    fn register_and_get() {
        let mut registry = GamePackRegistry::new(42);
        let pack = make_pack(&mut registry, "test_pack");
        let id = pack.id;

        registry.register(pack).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get(id).is_some());
        assert!(registry.get_by_name("test_pack").is_some());
    }

    #[test]
    fn duplicate_name_rejected() {
        let mut registry = GamePackRegistry::new(42);

        let pack1 = make_pack(&mut registry, "same_name");
        let pack2 = make_pack(&mut registry, "same_name");

        registry.register(pack1).unwrap();
        let result = registry.register(pack2);

        assert!(matches!(result, Err(GamePackError::DuplicatePackName(_))));
    }

    #[test]
    fn duplicate_block_id_rejected() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_block(BlockDescriptor::new(BlockId::new(1, 1), "stone"));
        pack.add_block(BlockDescriptor::new(BlockId::new(1, 1), "dirt"));

        let result = registry.register(pack);
        assert!(matches!(
            result,
            Err(GamePackError::DuplicateBlockId { .. })
        ));
    }

    #[test]
    fn duplicate_system_id_rejected() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_system(SystemDescriptor::new(SystemId::new(1, 1), "physics"));
        pack.add_system(SystemDescriptor::new(SystemId::new(1, 1), "rendering"));

        let result = registry.register(pack);
        assert!(matches!(
            result,
            Err(GamePackError::DuplicateSystemId { .. })
        ));
    }

    #[test]
    fn duplicate_hazard_id_rejected() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_hazard(HazardDescriptor::new(HazardId::new(1, 1), "fire"));
        pack.add_hazard(HazardDescriptor::new(HazardId::new(1, 1), "lava"));

        let result = registry.register(pack);
        assert!(matches!(
            result,
            Err(GamePackError::DuplicateHazardId { .. })
        ));
    }

    #[test]
    fn duplicate_shader_id_rejected() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_shader(ShaderDescriptor::new(ShaderId::new(1, 1), "bloom"));
        pack.add_shader(ShaderDescriptor::new(ShaderId::new(1, 1), "blur"));

        let result = registry.register(pack);
        assert!(matches!(
            result,
            Err(GamePackError::DuplicateShaderId { .. })
        ));
    }

    #[test]
    fn duplicate_rule_profile_id_rejected() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_rule_profile(RuleProfileDescriptor::new(RuleProfileId::new(1, 1), "easy"));
        pack.add_rule_profile(RuleProfileDescriptor::new(RuleProfileId::new(1, 1), "hard"));

        let result = registry.register(pack);
        assert!(matches!(
            result,
            Err(GamePackError::DuplicateRuleProfileId { .. })
        ));
    }

    #[test]
    fn unregister() {
        let mut registry = GamePackRegistry::new(42);
        let pack = make_pack(&mut registry, "test_pack");
        let id = pack.id;

        registry.register(pack).unwrap();
        let removed = registry.unregister(id);

        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn query_packs() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack_with_blocks = make_pack(&mut registry, "blocks_pack");
        pack_with_blocks.add_block(BlockDescriptor::new(BlockId::new(1, 1), "test"));
        registry.register(pack_with_blocks).unwrap();

        let pack_empty = make_pack(&mut registry, "empty_pack");
        registry.register(pack_empty).unwrap();

        let query = PackQuery::new().with_blocks();
        let results = registry.query(&query);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "blocks_pack");
    }

    #[test]
    fn all_blocks_sorted() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_block(BlockDescriptor::new(BlockId::new(1, 3), "c"));
        pack.add_block(BlockDescriptor::new(BlockId::new(1, 1), "a"));
        pack.add_block(BlockDescriptor::new(BlockId::new(1, 2), "b"));
        registry.register(pack).unwrap();

        let blocks = registry.all_blocks();
        assert_eq!(blocks[0].name, "a");
        assert_eq!(blocks[1].name, "b");
        assert_eq!(blocks[2].name, "c");
    }

    #[test]
    fn systems_sorted_by_phase() {
        let mut registry = GamePackRegistry::new(42);

        let mut pack = make_pack(&mut registry, "test");
        pack.add_system(
            SystemDescriptor::new(SystemId::new(1, 1), "render")
                .with_phase(super::super::descriptor::SystemPhase::Render),
        );
        pack.add_system(
            SystemDescriptor::new(SystemId::new(1, 2), "update")
                .with_phase(super::super::descriptor::SystemPhase::Update),
        );
        pack.add_system(
            SystemDescriptor::new(SystemId::new(1, 3), "init")
                .with_phase(super::super::descriptor::SystemPhase::Init),
        );
        registry.register(pack).unwrap();

        let systems = registry.all_systems();
        assert_eq!(systems[0].name, "init");
        assert_eq!(systems[1].name, "update");
        assert_eq!(systems[2].name, "render");
    }

    #[test]
    fn fingerprint_stability() {
        let mut registry1 = GamePackRegistry::new(42);
        let mut pack1 = make_pack(&mut registry1, "test");
        pack1.add_block(BlockDescriptor::new(BlockId::new(1, 1), "stone"));
        registry1.register(pack1).unwrap();

        let mut registry2 = GamePackRegistry::new(42);
        let mut pack2 = make_pack(&mut registry2, "test");
        pack2.add_block(BlockDescriptor::new(BlockId::new(1, 1), "stone"));
        registry2.register(pack2).unwrap();

        assert_eq!(
            registry1.combined_fingerprint(),
            registry2.combined_fingerprint()
        );
    }

    #[test]
    fn fingerprint_changes() {
        let mut registry1 = GamePackRegistry::new(42);
        let mut pack1 = make_pack(&mut registry1, "test");
        pack1.add_block(BlockDescriptor::new(BlockId::new(1, 1), "stone"));
        registry1.register(pack1).unwrap();

        let mut registry2 = GamePackRegistry::new(42);
        let mut pack2 = make_pack(&mut registry2, "test");
        pack2.add_block(BlockDescriptor::new(BlockId::new(1, 1), "dirt"));
        registry2.register(pack2).unwrap();

        assert_ne!(
            registry1.combined_fingerprint(),
            registry2.combined_fingerprint()
        );
    }

    #[test]
    fn compatibility_report_ok() {
        let mut registry = GamePackRegistry::new(42);

        let base = make_pack(&mut registry, "base");
        registry.register(base).unwrap();

        let addon_manifest = PackManifest::new("addon", PackVersion::new(1, 0, 0))
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)));
        let addon_id = registry.generate_id();
        let addon = RegisteredPack::new(addon_id, addon_manifest);
        registry.register(addon).unwrap();

        let report = registry.check_compatibility();
        assert!(report.is_compatible());
    }

    #[test]
    fn compatibility_report_missing_dependency() {
        let mut registry = GamePackRegistry::new(42);

        let addon_manifest = PackManifest::new("addon", PackVersion::new(1, 0, 0)).with_dependency(
            PackDependency::required("missing", PackVersion::new(1, 0, 0)),
        );
        let addon_id = registry.generate_id();
        let addon = RegisteredPack::new(addon_id, addon_manifest);
        registry.register(addon).unwrap();

        let report = registry.check_compatibility();
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn set_enabled() {
        let mut registry = GamePackRegistry::new(42);
        let pack = make_pack(&mut registry, "test");
        let id = pack.id;
        registry.register(pack).unwrap();

        assert!(registry.get(id).unwrap().manifest.enabled);

        registry.set_enabled(id, false);
        assert!(!registry.get(id).unwrap().manifest.enabled);
    }

    #[test]
    fn iter_deterministic() {
        let mut registry = GamePackRegistry::new(42);

        for i in 0..5 {
            let pack = make_pack(&mut registry, &format!("pack_{i}"));
            registry.register(pack).unwrap();
        }

        let ids1: Vec<_> = registry.iter_ids().collect();
        let ids2: Vec<_> = registry.iter_ids().collect();

        assert_eq!(ids1, ids2);
    }

    #[test]
    fn serde_roundtrip() {
        let mut registry = GamePackRegistry::new(42);
        let mut pack = make_pack(&mut registry, "test");
        pack.add_block(BlockDescriptor::new(BlockId::new(1, 1), "stone"));
        registry.register(pack).unwrap();

        let bytes = bincode::serialize(&registry).unwrap();
        let restored: GamePackRegistry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(registry.len(), restored.len());
        assert_eq!(
            registry.combined_fingerprint(),
            restored.combined_fingerprint()
        );
    }

    #[test]
    fn activation_order_respects_dependencies() {
        let mut registry = GamePackRegistry::new(42);

        let base = make_pack(&mut registry, "base");
        registry.register(base).unwrap();

        let addon_manifest = PackManifest::new("addon", PackVersion::new(1, 0, 0))
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)));
        let addon_id = registry.generate_id();
        let addon = RegisteredPack::new(addon_id, addon_manifest);
        registry.register(addon).unwrap();

        let plans = registry.generate_activation_plan().unwrap();
        let ready: Vec<_> = plans.iter().filter(|p| p.status.is_ready()).collect();

        let base_idx = ready.iter().position(|p| p.pack_name == "base").unwrap();
        let addon_idx = ready.iter().position(|p| p.pack_name == "addon").unwrap();

        assert!(base_idx < addon_idx);
    }
}

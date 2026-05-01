//! Activation planning and dependency resolution for game packs.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{
    error::{GamePackError, GamePackResult},
    id::PackId,
    manifest::{Capability, PackManifest, PackVersion},
};

/// Status of a pack in the activation plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationStatus {
    Ready,
    Pending { waiting_on: Vec<String> },
    Disabled,
    Failed { reason: String },
    Incompatible { conflicts: Vec<String> },
}

impl ActivationStatus {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    #[must_use]
    pub const fn can_activate(&self) -> bool {
        matches!(self, Self::Ready | Self::Pending { .. })
    }
}

/// An entry in the activation plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivationPlan {
    pub pack_id: PackId,
    pub pack_name: String,
    pub status: ActivationStatus,
    pub load_order: u32,
}

impl ActivationPlan {
    #[must_use]
    pub fn ready(pack_id: PackId, pack_name: String, load_order: u32) -> Self {
        Self {
            pack_id,
            pack_name,
            status: ActivationStatus::Ready,
            load_order,
        }
    }

    #[must_use]
    pub fn pending(pack_id: PackId, pack_name: String, waiting_on: Vec<String>) -> Self {
        Self {
            pack_id,
            pack_name,
            status: ActivationStatus::Pending { waiting_on },
            load_order: 0,
        }
    }

    #[must_use]
    pub fn disabled(pack_id: PackId, pack_name: String) -> Self {
        Self {
            pack_id,
            pack_name,
            status: ActivationStatus::Disabled,
            load_order: 0,
        }
    }

    #[must_use]
    pub fn failed(pack_id: PackId, pack_name: String, reason: String) -> Self {
        Self {
            pack_id,
            pack_name,
            status: ActivationStatus::Failed { reason },
            load_order: 0,
        }
    }

    #[must_use]
    pub fn incompatible(pack_id: PackId, pack_name: String, conflicts: Vec<String>) -> Self {
        Self {
            pack_id,
            pack_name,
            status: ActivationStatus::Incompatible { conflicts },
            load_order: 0,
        }
    }
}

/// Input for dependency resolution.
#[derive(Clone, Debug)]
pub struct PackInfo {
    pub id: PackId,
    pub manifest: PackManifest,
}

/// Resolves dependencies and plans pack activation order.
#[derive(Default)]
pub struct DependencyResolver {
    packs: BTreeMap<PackId, PackInfo>,
    name_to_id: HashMap<String, PackId>,
}

impl DependencyResolver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pack to the resolver.
    ///
    /// # Errors
    /// Returns an error if a pack with the same name already exists.
    pub fn add_pack(&mut self, id: PackId, manifest: PackManifest) -> GamePackResult<()> {
        if self.name_to_id.contains_key(&manifest.name) {
            return Err(GamePackError::DuplicatePackName(manifest.name.clone()));
        }

        self.name_to_id.insert(manifest.name.clone(), id);
        self.packs.insert(id, PackInfo { id, manifest });
        Ok(())
    }

    /// Remove a pack from the resolver.
    pub fn remove_pack(&mut self, id: PackId) -> Option<PackInfo> {
        if let Some(info) = self.packs.remove(&id) {
            self.name_to_id.remove(&info.manifest.name);
            Some(info)
        } else {
            None
        }
    }

    /// Check if a pack exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Get pack version by name.
    #[must_use]
    pub fn get_version(&self, name: &str) -> Option<&PackVersion> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.packs.get(id))
            .map(|info| &info.manifest.version)
    }

    /// Resolve dependencies and generate an activation plan.
    ///
    /// # Errors
    /// Returns an error if a dependency cycle is detected.
    pub fn resolve(&self) -> GamePackResult<Vec<ActivationPlan>> {
        let mut plans = Vec::new();
        let mut resolved: HashSet<PackId> = HashSet::new();
        let mut in_progress: HashSet<PackId> = HashSet::new();

        let enabled_packs: Vec<_> = self.packs.values().filter(|p| p.manifest.enabled).collect();

        for pack in self.packs.values() {
            if !pack.manifest.enabled {
                plans.push(ActivationPlan::disabled(
                    pack.id,
                    pack.manifest.name.clone(),
                ));
            }
        }

        let mut sorted: Vec<PackId> = Vec::new();
        for pack in &enabled_packs {
            self.topological_visit(pack.id, &mut resolved, &mut in_progress, &mut sorted)?;
        }

        for (order, &pack_id) in sorted.iter().enumerate() {
            let pack = &self.packs[&pack_id];

            let missing_deps: Vec<String> = pack
                .manifest
                .dependencies
                .iter()
                .filter(|dep| !dep.optional && !self.name_to_id.contains_key(&dep.name))
                .map(|dep| dep.name.clone())
                .collect();

            let version_issues: Vec<String> = pack
                .manifest
                .dependencies
                .iter()
                .filter_map(|dep| {
                    if let Some(version) = self.get_version(&dep.name)
                        && !version.is_compatible_with(&dep.min_version)
                    {
                        return Some(format!(
                            "{} requires {}, found {}",
                            dep.name, dep.min_version, version
                        ));
                    }
                    None
                })
                .collect();

            if !missing_deps.is_empty() {
                plans.push(ActivationPlan::pending(
                    pack.id,
                    pack.manifest.name.clone(),
                    missing_deps,
                ));
            } else if !version_issues.is_empty() {
                plans.push(ActivationPlan::incompatible(
                    pack.id,
                    pack.manifest.name.clone(),
                    version_issues,
                ));
            } else {
                let load_order = u32::try_from(order).unwrap_or(u32::MAX);
                plans.push(ActivationPlan::ready(
                    pack.id,
                    pack.manifest.name.clone(),
                    load_order,
                ));
            }
        }

        plans.sort_by(|a, b| {
            let status_order = |s: &ActivationStatus| -> u8 {
                match s {
                    ActivationStatus::Ready => 0,
                    ActivationStatus::Pending { .. } => 1,
                    ActivationStatus::Incompatible { .. } => 2,
                    ActivationStatus::Failed { .. } => 3,
                    ActivationStatus::Disabled => 4,
                }
            };

            status_order(&a.status)
                .cmp(&status_order(&b.status))
                .then(a.load_order.cmp(&b.load_order))
        });

        Ok(plans)
    }

    /// Check for capability conflicts.
    #[must_use]
    pub fn check_capability_conflicts(&self) -> Vec<(Capability, Vec<PackId>)> {
        let mut exclusive_caps: HashMap<String, Vec<PackId>> = HashMap::new();

        for pack in self.packs.values() {
            if !pack.manifest.enabled {
                continue;
            }

            for cap in &pack.manifest.provides {
                if matches!(cap, Capability::ExclusiveWorldRules) {
                    exclusive_caps
                        .entry("ExclusiveWorldRules".to_string())
                        .or_default()
                        .push(pack.id);
                }
            }
        }

        exclusive_caps
            .into_iter()
            .filter(|(_, packs)| packs.len() > 1)
            .map(|(cap, packs)| (Capability::Custom(cap), packs))
            .collect()
    }

    fn topological_visit(
        &self,
        pack_id: PackId,
        resolved: &mut HashSet<PackId>,
        in_progress: &mut HashSet<PackId>,
        sorted: &mut Vec<PackId>,
    ) -> GamePackResult<()> {
        if resolved.contains(&pack_id) {
            return Ok(());
        }

        if in_progress.contains(&pack_id) {
            return Err(GamePackError::DependencyCycle(pack_id));
        }

        in_progress.insert(pack_id);

        let pack = &self.packs[&pack_id];
        for dep in &pack.manifest.dependencies {
            if dep.optional {
                continue;
            }

            if let Some(&dep_id) = self.name_to_id.get(&dep.name)
                && self.packs.get(&dep_id).is_some_and(|p| p.manifest.enabled)
            {
                self.topological_visit(dep_id, resolved, in_progress, sorted)?;
            }
        }

        in_progress.remove(&pack_id);
        resolved.insert(pack_id);
        sorted.push(pack_id);

        Ok(())
    }

    /// Get packs in activation order (ready packs only).
    #[must_use]
    pub fn activation_order(&self) -> Vec<PackId> {
        self.resolve()
            .map(|plans| {
                plans
                    .into_iter()
                    .filter(|p| p.status.is_ready())
                    .map(|p| p.pack_id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find packs that depend on a given pack.
    #[must_use]
    pub fn find_dependents(&self, pack_name: &str) -> Vec<PackId> {
        self.packs
            .values()
            .filter(|p| p.manifest.dependencies.iter().any(|d| d.name == pack_name))
            .map(|p| p.id)
            .collect()
    }

    /// Get pack count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_pack::manifest::PackDependency;

    fn make_manifest(name: &str, version: (u32, u32, u32)) -> PackManifest {
        PackManifest::new(name, PackVersion::new(version.0, version.1, version.2))
    }

    #[test]
    fn resolve_no_dependencies() {
        let mut resolver = DependencyResolver::new();

        resolver
            .add_pack(PackId::new(1, 1), make_manifest("pack_a", (1, 0, 0)))
            .unwrap();
        resolver
            .add_pack(PackId::new(1, 2), make_manifest("pack_b", (1, 0, 0)))
            .unwrap();

        let plans = resolver.resolve().unwrap();
        let ready: Vec<_> = plans.iter().filter(|p| p.status.is_ready()).collect();

        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn resolve_with_dependencies() {
        let mut resolver = DependencyResolver::new();

        let base = make_manifest("base", (1, 0, 0));
        let addon = make_manifest("addon", (1, 0, 0))
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)));

        resolver.add_pack(PackId::new(1, 1), base).unwrap();
        resolver.add_pack(PackId::new(1, 2), addon).unwrap();

        let plans = resolver.resolve().unwrap();
        let ready: Vec<_> = plans.iter().filter(|p| p.status.is_ready()).collect();

        assert_eq!(ready.len(), 2);

        let base_order = ready.iter().find(|p| p.pack_name == "base").unwrap();
        let addon_order = ready.iter().find(|p| p.pack_name == "addon").unwrap();

        assert!(base_order.load_order < addon_order.load_order);
    }

    #[test]
    fn resolve_missing_dependency() {
        let mut resolver = DependencyResolver::new();

        let addon = make_manifest("addon", (1, 0, 0)).with_dependency(PackDependency::required(
            "missing",
            PackVersion::new(1, 0, 0),
        ));

        resolver.add_pack(PackId::new(1, 1), addon).unwrap();

        let plans = resolver.resolve().unwrap();
        let pending: Vec<_> = plans
            .iter()
            .filter(|p| matches!(p.status, ActivationStatus::Pending { .. }))
            .collect();

        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn resolve_optional_dependency_missing() {
        let mut resolver = DependencyResolver::new();

        let addon = make_manifest("addon", (1, 0, 0)).with_dependency(PackDependency::optional(
            "optional",
            PackVersion::new(1, 0, 0),
        ));

        resolver.add_pack(PackId::new(1, 1), addon).unwrap();

        let plans = resolver.resolve().unwrap();
        let ready: Vec<_> = plans.iter().filter(|p| p.status.is_ready()).collect();

        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn resolve_version_incompatible() {
        let mut resolver = DependencyResolver::new();

        let base = make_manifest("base", (1, 0, 0));
        let addon = make_manifest("addon", (1, 0, 0))
            .with_dependency(PackDependency::required("base", PackVersion::new(2, 0, 0)));

        resolver.add_pack(PackId::new(1, 1), base).unwrap();
        resolver.add_pack(PackId::new(1, 2), addon).unwrap();

        let plans = resolver.resolve().unwrap();
        let incompatible: Vec<_> = plans
            .iter()
            .filter(|p| matches!(p.status, ActivationStatus::Incompatible { .. }))
            .collect();

        assert_eq!(incompatible.len(), 1);
        assert_eq!(incompatible[0].pack_name, "addon");
    }

    #[test]
    fn detect_dependency_cycle() {
        let mut resolver = DependencyResolver::new();

        let pack_a = make_manifest("pack_a", (1, 0, 0)).with_dependency(PackDependency::required(
            "pack_b",
            PackVersion::new(1, 0, 0),
        ));
        let pack_b = make_manifest("pack_b", (1, 0, 0)).with_dependency(PackDependency::required(
            "pack_a",
            PackVersion::new(1, 0, 0),
        ));

        resolver.add_pack(PackId::new(1, 1), pack_a).unwrap();
        resolver.add_pack(PackId::new(1, 2), pack_b).unwrap();

        let result = resolver.resolve();
        assert!(matches!(result, Err(GamePackError::DependencyCycle(_))));
    }

    #[test]
    fn disabled_packs_excluded() {
        let mut resolver = DependencyResolver::new();

        resolver
            .add_pack(PackId::new(1, 1), make_manifest("enabled", (1, 0, 0)))
            .unwrap();
        resolver
            .add_pack(
                PackId::new(1, 2),
                make_manifest("disabled", (1, 0, 0)).disabled(),
            )
            .unwrap();

        let plans = resolver.resolve().unwrap();
        let ready: Vec<_> = plans.iter().filter(|p| p.status.is_ready()).collect();
        let disabled: Vec<_> = plans
            .iter()
            .filter(|p| matches!(p.status, ActivationStatus::Disabled))
            .collect();

        assert_eq!(ready.len(), 1);
        assert_eq!(disabled.len(), 1);
    }

    #[test]
    fn find_dependents() {
        let mut resolver = DependencyResolver::new();

        let base = make_manifest("base", (1, 0, 0));
        let addon1 = make_manifest("addon1", (1, 0, 0))
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)));
        let addon2 = make_manifest("addon2", (1, 0, 0))
            .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0)));
        let unrelated = make_manifest("unrelated", (1, 0, 0));

        resolver.add_pack(PackId::new(1, 1), base).unwrap();
        resolver.add_pack(PackId::new(1, 2), addon1).unwrap();
        resolver.add_pack(PackId::new(1, 3), addon2).unwrap();
        resolver.add_pack(PackId::new(1, 4), unrelated).unwrap();

        let dependents = resolver.find_dependents("base");
        assert_eq!(dependents.len(), 2);
    }

    #[test]
    fn duplicate_pack_name_rejected() {
        let mut resolver = DependencyResolver::new();

        resolver
            .add_pack(PackId::new(1, 1), make_manifest("same_name", (1, 0, 0)))
            .unwrap();

        let result = resolver.add_pack(PackId::new(1, 2), make_manifest("same_name", (2, 0, 0)));
        assert!(matches!(result, Err(GamePackError::DuplicatePackName(_))));
    }

    #[test]
    fn activation_order_deterministic() {
        let mut resolver = DependencyResolver::new();

        for i in 0..10 {
            let manifest = make_manifest(&format!("pack_{i}"), (1, 0, 0));
            resolver.add_pack(PackId::new(1, i), manifest).unwrap();
        }

        let order1 = resolver.activation_order();
        let order2 = resolver.activation_order();

        assert_eq!(order1, order2);
    }

    #[test]
    fn activation_plan_serde() {
        let plan = ActivationPlan::ready(PackId::new(1, 1), "test".to_string(), 0);

        let json = serde_json::to_string(&plan).unwrap();
        let restored: ActivationPlan = serde_json::from_str(&json).unwrap();

        assert_eq!(plan.pack_id, restored.pack_id);
        assert_eq!(plan.pack_name, restored.pack_name);
    }
}

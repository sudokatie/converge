//! Machine registry for managing machine instances.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::config::MachineConfig;
use super::identity::{MachineCategory, MachineId, MachineTier};
use super::state::MachineState;
use super::tick::{MachineFingerprint, MachineTickResult, MachineTickStats, tick_machine};

/// Error type for registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("machine not found: {0:?}")]
    NotFound(MachineId),
    #[error("machine already exists: {0:?}")]
    AlreadyExists(MachineId),
    #[error("config not registered: {0}")]
    ConfigNotRegistered(String),
}

/// Query filter for registry lookups.
#[derive(Clone, Debug, Default)]
pub struct RegistryQuery {
    /// Filter by category.
    pub category: Option<MachineCategory>,
    /// Filter by tier.
    pub tier: Option<MachineTier>,
    /// Filter by fault state.
    pub faulted: Option<bool>,
    /// Filter by idle state.
    pub idle: Option<bool>,
    /// Filter by enabled state.
    pub enabled: Option<bool>,
}

impl RegistryQuery {
    #[must_use]
    pub fn all() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn category(mut self, category: MachineCategory) -> Self {
        self.category = Some(category);
        self
    }

    #[must_use]
    pub fn tier(mut self, tier: MachineTier) -> Self {
        self.tier = Some(tier);
        self
    }

    #[must_use]
    pub fn faulted(mut self) -> Self {
        self.faulted = Some(true);
        self
    }

    #[must_use]
    pub fn not_faulted(mut self) -> Self {
        self.faulted = Some(false);
        self
    }

    #[must_use]
    pub fn idle(mut self) -> Self {
        self.idle = Some(true);
        self
    }

    #[must_use]
    pub fn active(mut self) -> Self {
        self.idle = Some(false);
        self
    }

    #[must_use]
    pub fn enabled(mut self) -> Self {
        self.enabled = Some(true);
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = Some(false);
        self
    }

    fn matches(&self, config: &MachineConfig, state: &MachineState) -> bool {
        if let Some(cat) = self.category
            && config.category != cat
        {
            return false;
        }
        if let Some(tier) = self.tier
            && state.tier != tier
        {
            return false;
        }
        if let Some(faulted) = self.faulted
            && state.is_faulted() != faulted
        {
            return false;
        }
        if let Some(idle) = self.idle
            && state.is_idle() != idle
        {
            return false;
        }
        if let Some(enabled) = self.enabled
            && state.enabled != enabled
        {
            return false;
        }
        true
    }
}

/// Summary of registry state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RegistrySummary {
    pub total_machines: u32,
    pub active_machines: u32,
    pub idle_machines: u32,
    pub faulted_machines: u32,
    pub by_category: HashMap<String, u32>,
    pub by_fault: HashMap<String, u32>,
}

/// Machine instance stored in the registry.
#[derive(Clone, Debug)]
struct MachineEntry {
    config_name: String,
    state: MachineState,
    position: (i32, i32, i32),
}

impl MachineEntry {
    fn sort_key(&self) -> (i32, i32, i32, u64) {
        (self.position.0, self.position.1, self.position.2, 0)
    }
}

/// Central registry for managing all machine instances.
#[derive(Debug, Default)]
pub struct MachineRegistry {
    configs: HashMap<String, MachineConfig>,
    machines: HashMap<MachineId, MachineEntry>,
    next_id: u64,
}

impl MachineRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_config(&mut self, config: MachineConfig) {
        self.configs.insert(config.name.clone(), config);
    }

    #[must_use]
    pub fn get_config(&self, name: &str) -> Option<&MachineConfig> {
        self.configs.get(name)
    }

    #[must_use]
    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    /// Install a new machine instance.
    ///
    /// # Errors
    /// Returns `RegistryError::ConfigNotRegistered` if config name not found.
    pub fn install(
        &mut self,
        config_name: &str,
        tier: MachineTier,
        position: (i32, i32, i32),
    ) -> Result<MachineId, RegistryError> {
        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| RegistryError::ConfigNotRegistered(config_name.to_string()))?;

        let id = MachineId::new(self.next_id);
        self.next_id += 1;

        let state = MachineState::new(config, tier);
        let entry = MachineEntry {
            config_name: config_name.to_string(),
            state,
            position,
        };

        self.machines.insert(id, entry);
        Ok(id)
    }

    pub fn uninstall(&mut self, id: MachineId) -> bool {
        self.machines.remove(&id).is_some()
    }

    #[must_use]
    pub fn machine_count(&self) -> usize {
        self.machines.len()
    }

    #[must_use]
    pub fn get_state(&self, id: MachineId) -> Option<&MachineState> {
        self.machines.get(&id).map(|e| &e.state)
    }

    pub fn get_state_mut(&mut self, id: MachineId) -> Option<&mut MachineState> {
        self.machines.get_mut(&id).map(|e| &mut e.state)
    }

    #[must_use]
    pub fn get_config_for(&self, id: MachineId) -> Option<&MachineConfig> {
        self.machines
            .get(&id)
            .and_then(|e| self.configs.get(&e.config_name))
    }

    #[must_use]
    pub fn get_position(&self, id: MachineId) -> Option<(i32, i32, i32)> {
        self.machines.get(&id).map(|e| e.position)
    }

    pub fn query(&self, query: &RegistryQuery) -> Vec<MachineId> {
        let mut results: Vec<_> = self
            .machines
            .iter()
            .filter(|(_, entry)| {
                if let Some(config) = self.configs.get(&entry.config_name) {
                    query.matches(config, &entry.state)
                } else {
                    false
                }
            })
            .map(|(id, entry)| (*id, entry.sort_key()))
            .collect();

        results.sort_by_key(|(_, key)| *key);
        results.into_iter().map(|(id, _)| id).collect()
    }

    pub fn query_by_category(&self, category: MachineCategory) -> Vec<MachineId> {
        self.query(&RegistryQuery::all().category(category))
    }

    pub fn query_faulted(&self) -> Vec<MachineId> {
        self.query(&RegistryQuery::all().faulted())
    }

    pub fn query_idle(&self) -> Vec<MachineId> {
        self.query(&RegistryQuery::all().idle())
    }

    pub fn query_active(&self) -> Vec<MachineId> {
        self.query(&RegistryQuery::all().active())
    }

    #[expect(clippy::missing_panics_doc)]
    pub fn tick_all(
        &mut self,
        tick: u64,
        power_grid: &HashMap<MachineId, f32>,
    ) -> (Vec<MachineTickResult>, MachineTickStats) {
        let mut results = Vec::with_capacity(self.machines.len());
        let mut stats = MachineTickStats::default();

        let mut ids: Vec<_> = self.machines.keys().copied().collect();
        ids.sort();

        for id in ids {
            let entry = self.machines.get_mut(&id).unwrap();
            let Some(config) = self.configs.get(&entry.config_name) else {
                continue;
            };

            let available_power = power_grid.get(&id).copied().unwrap_or(0.0);
            let result = tick_machine(id, config, &mut entry.state, tick, available_power);

            if entry.state.is_active() {
                stats.active_count += 1;
            } else if entry.state.is_idle() {
                stats.idle_count += 1;
            }
            if entry.state.is_faulted() {
                stats.faulted_count += 1;
            }

            stats.power_consumed += result.power_consumed;
            stats.power_generated += result.power_generated;
            stats.heat_produced += result.heat_produced;
            if result.process_completed {
                stats.processes_completed += 1;
            }

            results.push(result);
        }

        (results, stats)
    }

    pub fn tick_machine(
        &mut self,
        id: MachineId,
        tick: u64,
        available_power: f32,
    ) -> Option<MachineTickResult> {
        let entry = self.machines.get_mut(&id)?;
        let config = self.configs.get(&entry.config_name)?;
        Some(tick_machine(
            id,
            config,
            &mut entry.state,
            tick,
            available_power,
        ))
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn summary(&self) -> RegistrySummary {
        let mut summary = RegistrySummary {
            total_machines: self.machines.len() as u32,
            ..Default::default()
        };

        for entry in self.machines.values() {
            if entry.state.is_active() {
                summary.active_machines += 1;
            } else if entry.state.is_idle() {
                summary.idle_machines += 1;
            }
            if entry.state.is_faulted() {
                summary.faulted_machines += 1;
                *summary
                    .by_fault
                    .entry(entry.state.fault.name().to_string())
                    .or_insert(0) += 1;
            }

            if let Some(config) = self.configs.get(&entry.config_name) {
                *summary
                    .by_category
                    .entry(config.category.name().to_string())
                    .or_insert(0) += 1;
            }
        }

        summary
    }

    pub fn fingerprint(&self) -> MachineFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        let mut ids: Vec<_> = self.machines.keys().copied().collect();
        ids.sort();

        #[expect(clippy::cast_possible_truncation)]
        let count = ids.len() as u32;
        hasher.update(&count.to_le_bytes());

        for id in ids {
            hasher.update(&id.raw().to_le_bytes());
            if let Some(entry) = self.machines.get(&id) {
                hasher.update(&entry.state.fingerprint().to_le_bytes());
            }
        }

        MachineFingerprint(hasher.finalize())
    }

    pub fn all_ids(&self) -> Vec<MachineId> {
        let mut ids: Vec<_> = self.machines.keys().copied().collect();
        ids.sort();
        ids
    }

    pub fn iter(&self) -> impl Iterator<Item = (MachineId, &MachineState)> {
        self.machines.iter().map(|(id, entry)| (*id, &entry.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::config::{AtmosphereEffect, MachineConfig, ProcessDefinition, ProcessId};
    use crate::machine::state::FaultKind;

    fn setup_registry() -> MachineRegistry {
        let mut registry = MachineRegistry::new();

        registry.register_config(
            MachineConfig::processor("Smelter", 10.0).with_process(ProcessDefinition::new(
                ProcessId::new(1),
                "Smelt",
                20,
            )),
        );
        registry.register_config(MachineConfig::reactor("Generator", 100.0, 50.0));
        registry.register_config(MachineConfig::life_support(
            "Scrubber",
            20.0,
            AtmosphereEffect::scrubber(5.0),
        ));

        registry
    }

    #[test]
    fn install_machine() {
        let mut registry = setup_registry();
        let id = registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();

        assert!(registry.get_state(id).is_some());
        assert_eq!(registry.machine_count(), 1);
    }

    #[test]
    fn install_unknown_config() {
        let mut registry = setup_registry();
        let result = registry.install("Unknown", MachineTier::Basic, (0, 0, 0));

        assert!(matches!(result, Err(RegistryError::ConfigNotRegistered(_))));
    }

    #[test]
    fn uninstall_machine() {
        let mut registry = setup_registry();
        let id = registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();

        assert!(registry.uninstall(id));
        assert!(registry.get_state(id).is_none());
        assert_eq!(registry.machine_count(), 0);
    }

    #[test]
    fn query_by_category() {
        let mut registry = setup_registry();
        registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        registry
            .install("Generator", MachineTier::Standard, (1, 0, 0))
            .unwrap();
        registry
            .install("Scrubber", MachineTier::Basic, (2, 0, 0))
            .unwrap();

        let processors = registry.query_by_category(MachineCategory::Processor);
        assert_eq!(processors.len(), 1);

        let reactors = registry.query_by_category(MachineCategory::Reactor);
        assert_eq!(reactors.len(), 1);
    }

    #[test]
    fn query_faulted() {
        let mut registry = setup_registry();
        let id1 = registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        let _ = registry
            .install("Smelter", MachineTier::Basic, (1, 0, 0))
            .unwrap();

        registry
            .get_state_mut(id1)
            .unwrap()
            .set_fault(FaultKind::NoPower);

        let faulted = registry.query_faulted();
        assert_eq!(faulted.len(), 1);
        assert_eq!(faulted[0], id1);
    }

    #[test]
    fn query_composite() {
        let mut registry = setup_registry();
        registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        let id2 = registry
            .install("Smelter", MachineTier::Advanced, (1, 0, 0))
            .unwrap();

        let query = RegistryQuery::all()
            .category(MachineCategory::Processor)
            .tier(MachineTier::Advanced);

        let results = registry.query(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], id2);
    }

    #[test]
    fn tick_all_machines() {
        let mut registry = setup_registry();
        registry
            .install("Generator", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        registry
            .install("Generator", MachineTier::Basic, (1, 0, 0))
            .unwrap();

        let power_grid = HashMap::new();
        let (results, stats) = registry.tick_all(1, &power_grid);

        assert_eq!(results.len(), 2);
        assert!(stats.power_generated > 0.0);
    }

    #[test]
    fn summary_counts() {
        let mut registry = setup_registry();
        let id1 = registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        registry
            .install("Generator", MachineTier::Basic, (1, 0, 0))
            .unwrap();

        registry
            .get_state_mut(id1)
            .unwrap()
            .set_fault(FaultKind::Damaged);

        let summary = registry.summary();
        assert_eq!(summary.total_machines, 2);
        assert_eq!(summary.faulted_machines, 1);
        assert_eq!(summary.by_category.get("processor"), Some(&1));
        assert_eq!(summary.by_category.get("reactor"), Some(&1));
    }

    #[test]
    fn fingerprint_deterministic() {
        let mut r1 = setup_registry();
        let mut r2 = setup_registry();

        r1.install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        r2.install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();

        assert_eq!(r1.fingerprint(), r2.fingerprint());
    }

    #[test]
    fn fingerprint_changes() {
        let mut registry = setup_registry();
        let id = registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();

        let fp1 = registry.fingerprint();
        registry.get_state_mut(id).unwrap().power.add(50.0);
        let fp2 = registry.fingerprint();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn stable_id_ordering() {
        let mut registry = setup_registry();
        for i in 0..10 {
            registry
                .install("Smelter", MachineTier::Basic, (i, 0, 0))
                .unwrap();
        }

        let ids = registry.all_ids();
        for i in 1..ids.len() {
            assert!(ids[i - 1] < ids[i]);
        }
    }

    #[test]
    fn iter_machines() {
        let mut registry = setup_registry();
        registry
            .install("Smelter", MachineTier::Basic, (0, 0, 0))
            .unwrap();
        registry
            .install("Generator", MachineTier::Basic, (1, 0, 0))
            .unwrap();

        let count = registry.iter().count();
        assert_eq!(count, 2);
    }
}

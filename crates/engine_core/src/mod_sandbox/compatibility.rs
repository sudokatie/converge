//! Compatibility reporting for mod sets.
//!
//! Provides deterministic compatibility analysis for multiple mods, including
//! version checking, dependency resolution, conflict detection, load ordering,
//! and capability/policy validation.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::game_pack::PackVersion;

use super::{
    descriptor::{LoadOrderConstraint, ModDescriptor},
    fingerprint::SandboxFingerprint,
    id::ModId,
    policy::SandboxPolicy,
};

/// Severity level for compatibility issues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// A single compatibility issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub mod_id: Option<ModId>,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}

impl CompatibilityIssue {
    #[must_use]
    pub fn info(category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Info,
            category,
            mod_id: None,
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn warning(category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            category,
            mod_id: None,
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn error(category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Error,
            category,
            mod_id: None,
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn critical(category: IssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Critical,
            category,
            mod_id: None,
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn with_mod(mut self, mod_id: ModId) -> Self {
        self.mod_id = Some(mod_id);
        self
    }

    #[must_use]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Category of compatibility issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueCategory {
    EngineVersion,
    ApiVersion,
    Dependency,
    Conflict,
    LoadOrder,
    Capability,
    Budget,
    ContentHook,
    GamePack,
    Policy,
}

/// Load order entry for a mod.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadOrderEntry {
    pub mod_id: ModId,
    pub mod_name: String,
    pub order: u32,
    pub status: LoadStatus,
}

/// Status of a mod in the load order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadStatus {
    Ready,
    Disabled,
    MissingDependencies { missing: Vec<String> },
    Conflicted { with: Vec<String> },
    PolicyDenied { reasons: Vec<String> },
    BudgetExceeded { violations: Vec<String> },
    VersionIncompatible { reason: String },
}

impl LoadStatus {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    #[must_use]
    pub const fn is_loadable(&self) -> bool {
        matches!(self, Self::Ready | Self::Disabled)
    }
}

/// Complete compatibility report for a set of mods.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModCompatibilityReport {
    pub compatible: bool,
    pub issues: Vec<CompatibilityIssue>,
    pub load_order: Vec<LoadOrderEntry>,
    pub fingerprint: SandboxFingerprint,

    #[serde(default)]
    pub engine_version_checked: Option<PackVersion>,
    #[serde(default)]
    pub api_version_checked: Option<PackVersion>,
    #[serde(default)]
    pub policy_applied: Option<String>,

    #[serde(default)]
    pub mods_checked: u32,
    #[serde(default)]
    pub mods_compatible: u32,
    #[serde(default)]
    pub mods_incompatible: u32,
}

impl ModCompatibilityReport {
    #[must_use]
    pub fn new() -> Self {
        Self {
            compatible: true,
            issues: Vec::new(),
            load_order: Vec::new(),
            fingerprint: SandboxFingerprint::default(),
            engine_version_checked: None,
            api_version_checked: None,
            policy_applied: None,
            mods_checked: 0,
            mods_compatible: 0,
            mods_incompatible: 0,
        }
    }

    pub fn add_issue(&mut self, issue: CompatibilityIssue) {
        if issue.severity >= IssueSeverity::Error {
            self.compatible = false;
        }
        self.issues.push(issue);
    }

    /// Get issues filtered by severity.
    #[must_use]
    pub fn issues_by_severity(&self, severity: IssueSeverity) -> Vec<&CompatibilityIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .collect()
    }

    /// Get issues filtered by category.
    #[must_use]
    pub fn issues_by_category(&self, category: IssueCategory) -> Vec<&CompatibilityIssue> {
        self.issues
            .iter()
            .filter(|i| i.category == category)
            .collect()
    }

    /// Get error count.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity >= IssueSeverity::Error)
            .count()
    }

    /// Get warning count.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .count()
    }

    /// Check if all mods can be loaded.
    #[must_use]
    pub fn all_loadable(&self) -> bool {
        self.load_order.iter().all(|e| e.status.is_loadable())
    }

    /// Get mods ready to load in order.
    #[must_use]
    pub fn ready_mods(&self) -> Vec<&LoadOrderEntry> {
        self.load_order
            .iter()
            .filter(|e| e.status.is_ready())
            .collect()
    }
}

/// Builder for generating compatibility reports.
pub struct CompatibilityChecker {
    engine_version: Option<PackVersion>,
    api_version: Option<PackVersion>,
    policy: Option<SandboxPolicy>,
    mods: BTreeMap<ModId, ModDescriptor>,
}

impl CompatibilityChecker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine_version: None,
            api_version: None,
            policy: None,
            mods: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_engine_version(mut self, version: PackVersion) -> Self {
        self.engine_version = Some(version);
        self
    }

    #[must_use]
    pub fn with_api_version(mut self, version: PackVersion) -> Self {
        self.api_version = Some(version);
        self
    }

    #[must_use]
    pub fn with_policy(mut self, policy: SandboxPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    pub fn add_mod(&mut self, desc: ModDescriptor) {
        self.mods.insert(desc.id, desc);
    }

    /// Generate a full compatibility report.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "mod counts won't exceed u32::MAX"
    )]
    pub fn check(&self) -> ModCompatibilityReport {
        let mut report = ModCompatibilityReport::new();
        report
            .engine_version_checked
            .clone_from(&self.engine_version);
        report.api_version_checked.clone_from(&self.api_version);
        report.policy_applied = self.policy.as_ref().map(|p| p.name.clone());
        report.mods_checked = self.mods.len() as u32;

        let name_to_id: HashMap<&str, ModId> = self
            .mods
            .values()
            .map(|m| (m.name.as_str(), m.id))
            .collect();

        self.check_version_compatibility(&mut report);
        self.check_dependencies(&mut report, &name_to_id);
        self.check_conflicts(&mut report, &name_to_id);
        self.check_policy(&mut report);
        self.compute_load_order(&mut report, &name_to_id);
        self.compute_fingerprint(&mut report);

        report.mods_compatible = report
            .load_order
            .iter()
            .filter(|e| e.status.is_ready())
            .count() as u32;
        report.mods_incompatible = report.mods_checked - report.mods_compatible;

        report
    }

    fn check_version_compatibility(&self, report: &mut ModCompatibilityReport) {
        for desc in self.mods.values() {
            if let Some(ref engine_range) = desc.engine_version
                && let Some(ref engine_ver) = self.engine_version
                && !engine_range.contains(engine_ver)
            {
                report.add_issue(
                    CompatibilityIssue::error(
                        IssueCategory::EngineVersion,
                        format!(
                            "Mod '{}' requires engine version {}-{}, found {}",
                            desc.name,
                            engine_range.min,
                            engine_range
                                .max
                                .as_ref()
                                .map_or("*".to_string(), ToString::to_string),
                            engine_ver
                        ),
                    )
                    .with_mod(desc.id),
                );
            }

            if let Some(ref api_range) = desc.api_version
                && let Some(ref api_ver) = self.api_version
                && !api_range.contains(api_ver)
            {
                report.add_issue(
                    CompatibilityIssue::error(
                        IssueCategory::ApiVersion,
                        format!(
                            "Mod '{}' requires API version {}-{}, found {}",
                            desc.name,
                            api_range.min,
                            api_range
                                .max
                                .as_ref()
                                .map_or("*".to_string(), ToString::to_string),
                            api_ver
                        ),
                    )
                    .with_mod(desc.id),
                );
            }
        }
    }

    fn check_dependencies(
        &self,
        report: &mut ModCompatibilityReport,
        name_to_id: &HashMap<&str, ModId>,
    ) {
        for desc in self.mods.values() {
            for dep in &desc.dependencies {
                match name_to_id.get(dep.name.as_str()) {
                    Some(&dep_id) => {
                        let dep_mod = &self.mods[&dep_id];
                        if !dep_mod.version.is_compatible_with(&dep.min_version) {
                            report.add_issue(
                                CompatibilityIssue::error(
                                    IssueCategory::Dependency,
                                    format!(
                                        "Mod '{}' requires '{}' version >= {}, found {}",
                                        desc.name, dep.name, dep.min_version, dep_mod.version
                                    ),
                                )
                                .with_mod(desc.id),
                            );
                        }
                    }
                    None => {
                        if dep.optional {
                            report.add_issue(
                                CompatibilityIssue::info(
                                    IssueCategory::Dependency,
                                    format!(
                                        "Mod '{}' has optional dependency '{}' which is not present",
                                        desc.name, dep.name
                                    ),
                                )
                                .with_mod(desc.id),
                            );
                        } else {
                            report.add_issue(
                                CompatibilityIssue::error(
                                    IssueCategory::Dependency,
                                    format!(
                                        "Mod '{}' requires missing dependency '{}'",
                                        desc.name, dep.name
                                    ),
                                )
                                .with_mod(desc.id),
                            );
                        }
                    }
                }
            }
        }
    }

    fn check_conflicts(
        &self,
        report: &mut ModCompatibilityReport,
        name_to_id: &HashMap<&str, ModId>,
    ) {
        for desc in self.mods.values() {
            for conflict in &desc.conflicts {
                if let Some(&conflict_id) = name_to_id.get(conflict.mod_name.as_str()) {
                    let conflict_mod = &self.mods[&conflict_id];
                    if conflict.applies_to(&conflict_mod.name, &conflict_mod.version) {
                        let msg = match &conflict.reason {
                            Some(reason) => format!(
                                "Mod '{}' conflicts with '{}': {}",
                                desc.name, conflict.mod_name, reason
                            ),
                            None => format!(
                                "Mod '{}' conflicts with '{}'",
                                desc.name, conflict.mod_name
                            ),
                        };
                        report.add_issue(
                            CompatibilityIssue::error(IssueCategory::Conflict, msg)
                                .with_mod(desc.id),
                        );
                    }
                }
            }
        }
    }

    fn check_policy(&self, report: &mut ModCompatibilityReport) {
        let Some(ref policy) = self.policy else {
            return;
        };

        for desc in self.mods.values() {
            let cap_validation = policy.validate_requirements(&desc.sandbox_capabilities);

            if !cap_validation.is_allowed() {
                for denied in &cap_validation.denied {
                    report.add_issue(
                        CompatibilityIssue::error(
                            IssueCategory::Capability,
                            format!(
                                "Mod '{}' requires capability '{:?}' which is denied by policy",
                                desc.name, denied
                            ),
                        )
                        .with_mod(desc.id),
                    );
                }
            }

            if cap_validation.needs_user_prompt() {
                for prompted in &cap_validation.needs_prompt {
                    report.add_issue(
                        CompatibilityIssue::warning(
                            IssueCategory::Capability,
                            format!(
                                "Mod '{}' requests capability '{:?}' which requires user approval",
                                desc.name, prompted
                            ),
                        )
                        .with_mod(desc.id),
                    );
                }
            }

            if let Some(ref budget) = desc.requested_budget {
                let budget_validation = budget.validate_against(&policy.budget_limits);
                if !budget_validation.is_valid() {
                    for violation in &budget_validation.violations {
                        report.add_issue(
                            CompatibilityIssue::error(
                                IssueCategory::Budget,
                                format!("Mod '{}': {}", desc.name, violation),
                            )
                            .with_mod(desc.id),
                        );
                    }
                }
            }
        }
    }

    fn compute_load_order(
        &self,
        report: &mut ModCompatibilityReport,
        name_to_id: &HashMap<&str, ModId>,
    ) {
        let mut order_map: BTreeMap<ModId, u32> = BTreeMap::new();
        let mut statuses: HashMap<ModId, LoadStatus> = HashMap::new();

        for desc in self.mods.values() {
            statuses.insert(
                desc.id,
                Self::determine_load_status(desc, report, name_to_id),
            );
        }

        let sorted = self.topological_sort(name_to_id);

        for (order, mod_id) in sorted.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            order_map.insert(*mod_id, order as u32);
        }

        report.load_order = sorted
            .into_iter()
            .map(|mod_id| {
                let desc = &self.mods[&mod_id];
                LoadOrderEntry {
                    mod_id,
                    mod_name: desc.name.clone(),
                    order: order_map.get(&mod_id).copied().unwrap_or(0),
                    status: statuses.remove(&mod_id).unwrap_or(LoadStatus::Ready),
                }
            })
            .collect();
    }

    fn determine_load_status(
        desc: &ModDescriptor,
        report: &ModCompatibilityReport,
        name_to_id: &HashMap<&str, ModId>,
    ) -> LoadStatus {
        if !desc.enabled {
            return LoadStatus::Disabled;
        }

        let mod_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.mod_id == Some(desc.id) && i.severity >= IssueSeverity::Error)
            .collect();

        let missing: Vec<_> = mod_issues
            .iter()
            .filter(|i| i.category == IssueCategory::Dependency)
            .filter_map(|_| {
                desc.dependencies
                    .iter()
                    .find(|d| !name_to_id.contains_key(d.name.as_str()) && !d.optional)
                    .map(|d| d.name.clone())
            })
            .collect();

        if !missing.is_empty() {
            return LoadStatus::MissingDependencies { missing };
        }

        let conflicts: Vec<_> = mod_issues
            .iter()
            .filter(|i| i.category == IssueCategory::Conflict)
            .filter_map(|_| {
                desc.conflicts
                    .iter()
                    .find(|c| name_to_id.contains_key(c.mod_name.as_str()))
                    .map(|c| c.mod_name.clone())
            })
            .collect();

        if !conflicts.is_empty() {
            return LoadStatus::Conflicted { with: conflicts };
        }

        let policy_issues: Vec<_> = mod_issues
            .iter()
            .filter(|i| {
                i.category == IssueCategory::Capability || i.category == IssueCategory::Policy
            })
            .map(|i| i.message.clone())
            .collect();

        if !policy_issues.is_empty() {
            return LoadStatus::PolicyDenied {
                reasons: policy_issues,
            };
        }

        let budget_issues: Vec<_> = mod_issues
            .iter()
            .filter(|i| i.category == IssueCategory::Budget)
            .map(|i| i.message.clone())
            .collect();

        if !budget_issues.is_empty() {
            return LoadStatus::BudgetExceeded {
                violations: budget_issues,
            };
        }

        let version_issue = mod_issues.iter().find(|i| {
            i.category == IssueCategory::EngineVersion || i.category == IssueCategory::ApiVersion
        });

        if let Some(issue) = version_issue {
            return LoadStatus::VersionIncompatible {
                reason: issue.message.clone(),
            };
        }

        LoadStatus::Ready
    }

    fn topological_sort(&self, name_to_id: &HashMap<&str, ModId>) -> Vec<ModId> {
        let mut in_degree: HashMap<ModId, usize> = HashMap::new();
        let mut graph: HashMap<ModId, Vec<ModId>> = HashMap::new();

        for desc in self.mods.values() {
            in_degree.entry(desc.id).or_insert(0);
            graph.entry(desc.id).or_default();

            for dep in &desc.dependencies {
                if let Some(&dep_id) = name_to_id.get(dep.name.as_str()) {
                    graph.entry(dep_id).or_default().push(desc.id);
                    *in_degree.entry(desc.id).or_insert(0) += 1;
                }
            }

            for constraint in &desc.load_order {
                match constraint {
                    LoadOrderConstraint::After(name) => {
                        if let Some(&before_id) = name_to_id.get(name.as_str()) {
                            graph.entry(before_id).or_default().push(desc.id);
                            *in_degree.entry(desc.id).or_insert(0) += 1;
                        }
                    }
                    LoadOrderConstraint::Before(name) => {
                        if let Some(&after_id) = name_to_id.get(name.as_str()) {
                            graph.entry(desc.id).or_default().push(after_id);
                            *in_degree.entry(after_id).or_insert(0) += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut queue: Vec<ModId> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();

        let mut result = Vec::new();

        while let Some(mod_id) = queue.pop() {
            result.push(mod_id);

            if let Some(dependents) = graph.get(&mod_id) {
                for &dep_id in dependents {
                    if let Some(deg) = in_degree.get_mut(&dep_id) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push(dep_id);
                            queue.sort();
                        }
                    }
                }
            }
        }

        for &mod_id in self.mods.keys() {
            if !result.contains(&mod_id) {
                result.push(mod_id);
            }
        }

        result
    }

    fn compute_fingerprint(&self, report: &mut ModCompatibilityReport) {
        use super::fingerprint::SandboxFingerprintBuilder;

        let mut builder = SandboxFingerprintBuilder::new();

        if let Some(ref ver) = self.engine_version {
            builder.add(&("engine", ver.major, ver.minor, ver.patch));
        }
        if let Some(ref ver) = self.api_version {
            builder.add(&("api", ver.major, ver.minor, ver.patch));
        }

        for entry in &report.load_order {
            if let Some(desc) = self.mods.get(&entry.mod_id) {
                builder.add(&desc.name);
                builder.add(&(desc.version.major, desc.version.minor, desc.version.patch));
            }
        }

        report.fingerprint = builder.finish();
    }
}

impl Default for CompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_pack::PackDependency;
    use crate::mod_sandbox::{
        budget::{MemoryBudget, ResourceBudget},
        capability::{CapabilityRequirements, SandboxCapability},
        id::{ModId, PolicyId},
        policy::SandboxPolicy,
    };

    fn make_mod(id: u32, name: &str) -> ModDescriptor {
        ModDescriptor::new(ModId::new(1, id), name, PackVersion::new(1, 0, 0))
    }

    #[test]
    fn compatibility_check_empty() {
        let checker = CompatibilityChecker::new();
        let report = checker.check();

        assert!(report.compatible);
        assert!(report.issues.is_empty());
        assert!(report.load_order.is_empty());
    }

    #[test]
    fn compatibility_check_single_mod() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(make_mod(1, "test_mod"));

        let report = checker.check();

        assert!(report.compatible);
        assert_eq!(report.load_order.len(), 1);
        assert_eq!(report.mods_checked, 1);
        assert_eq!(report.mods_compatible, 1);
    }

    #[test]
    fn compatibility_check_missing_dependency() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(
            make_mod(1, "dependent").with_dependency(PackDependency::required(
                "missing",
                PackVersion::new(1, 0, 0),
            )),
        );

        let report = checker.check();

        assert!(!report.compatible);
        assert_eq!(report.error_count(), 1);
        assert!(
            !report
                .issues_by_category(IssueCategory::Dependency)
                .is_empty()
        );
    }

    #[test]
    fn compatibility_check_optional_dependency_missing() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(
            make_mod(1, "dependent").with_dependency(PackDependency::optional(
                "optional",
                PackVersion::new(1, 0, 0),
            )),
        );

        let report = checker.check();

        assert!(report.compatible);
        assert_eq!(report.warning_count(), 0);
        let infos = report.issues_by_severity(IssueSeverity::Info);
        assert_eq!(infos.len(), 1);
    }

    #[test]
    fn compatibility_check_dependency_satisfied() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(make_mod(1, "base"));
        checker.add_mod(
            make_mod(2, "addon")
                .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0))),
        );

        let report = checker.check();

        assert!(report.compatible);
        assert!(
            report
                .issues_by_category(IssueCategory::Dependency)
                .is_empty()
        );
    }

    #[test]
    fn compatibility_check_conflict_detected() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(make_mod(1, "mod_a"));
        checker.add_mod(
            make_mod(2, "mod_b")
                .with_conflict(super::super::descriptor::ModConflict::with_mod("mod_a")),
        );

        let report = checker.check();

        assert!(!report.compatible);
        assert!(
            !report
                .issues_by_category(IssueCategory::Conflict)
                .is_empty()
        );
    }

    #[test]
    fn compatibility_check_engine_version() {
        let mut checker =
            CompatibilityChecker::new().with_engine_version(PackVersion::new(1, 0, 0));

        checker.add_mod(make_mod(1, "old_mod").with_engine_version(
            super::super::descriptor::ApiVersionRange::new(PackVersion::new(2, 0, 0)),
        ));

        let report = checker.check();

        assert!(!report.compatible);
        assert!(
            !report
                .issues_by_category(IssueCategory::EngineVersion)
                .is_empty()
        );
    }

    #[test]
    fn compatibility_check_policy_denied() {
        let policy = SandboxPolicy::restrictive(PolicyId::new(1, 1), "strict");

        let mut checker = CompatibilityChecker::new().with_policy(policy);
        checker.add_mod(make_mod(1, "needy_mod").with_capabilities(
            CapabilityRequirements::new().with_required(SandboxCapability::MutateBlocks),
        ));

        let report = checker.check();

        assert!(!report.compatible);
        assert!(
            !report
                .issues_by_category(IssueCategory::Capability)
                .is_empty()
        );
    }

    #[test]
    fn compatibility_check_policy_allowed() {
        let policy = SandboxPolicy::permissive(PolicyId::new(1, 1), "loose");

        let mut checker = CompatibilityChecker::new().with_policy(policy);
        checker.add_mod(make_mod(1, "normal_mod").with_capabilities(
            CapabilityRequirements::new().with_required(SandboxCapability::ReadOwnData),
        ));

        let report = checker.check();

        assert!(report.compatible);
    }

    #[test]
    fn compatibility_check_budget_exceeded() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "limited").with_budget_limits(
            ResourceBudget::new().with_memory(MemoryBudget::new(10 * 1024 * 1024)),
        );

        let mut checker = CompatibilityChecker::new().with_policy(policy);
        checker.add_mod(
            make_mod(1, "hungry_mod").with_budget(
                ResourceBudget::new().with_memory(MemoryBudget::new(100 * 1024 * 1024)),
            ),
        );

        let report = checker.check();

        assert!(!report.compatible);
        assert!(!report.issues_by_category(IssueCategory::Budget).is_empty());
    }

    #[test]
    fn load_order_respects_dependencies() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(
            make_mod(2, "addon")
                .with_dependency(PackDependency::required("base", PackVersion::new(1, 0, 0))),
        );
        checker.add_mod(make_mod(1, "base"));

        let report = checker.check();

        let base_order = report
            .load_order
            .iter()
            .find(|e| e.mod_name == "base")
            .unwrap()
            .order;
        let addon_order = report
            .load_order
            .iter()
            .find(|e| e.mod_name == "addon")
            .unwrap()
            .order;

        assert!(base_order < addon_order);
    }

    #[test]
    fn fingerprint_deterministic() {
        let mut checker1 =
            CompatibilityChecker::new().with_engine_version(PackVersion::new(1, 0, 0));
        checker1.add_mod(make_mod(1, "mod_a"));
        checker1.add_mod(make_mod(2, "mod_b"));

        let mut checker2 =
            CompatibilityChecker::new().with_engine_version(PackVersion::new(1, 0, 0));
        checker2.add_mod(make_mod(1, "mod_a"));
        checker2.add_mod(make_mod(2, "mod_b"));

        let report1 = checker1.check();
        let report2 = checker2.check();

        assert_eq!(report1.fingerprint, report2.fingerprint);
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let mut checker1 = CompatibilityChecker::new();
        checker1.add_mod(make_mod(1, "mod_a"));

        let mut checker2 = CompatibilityChecker::new();
        checker2.add_mod(make_mod(1, "mod_b"));

        let report1 = checker1.check();
        let report2 = checker2.check();

        assert_ne!(report1.fingerprint, report2.fingerprint);
    }

    #[test]
    fn report_serde_roundtrip() {
        let mut checker =
            CompatibilityChecker::new().with_engine_version(PackVersion::new(1, 0, 0));
        checker.add_mod(make_mod(1, "test"));

        let report = checker.check();
        let json = serde_json::to_string(&report).unwrap();
        let restored: ModCompatibilityReport = serde_json::from_str(&json).unwrap();

        assert_eq!(report.compatible, restored.compatible);
        assert_eq!(report.fingerprint, restored.fingerprint);
    }

    #[test]
    fn report_bincode_roundtrip() {
        let mut checker = CompatibilityChecker::new();
        checker.add_mod(make_mod(1, "test"));

        let report = checker.check();
        let bytes = bincode::serialize(&report).unwrap();
        let restored: ModCompatibilityReport = bincode::deserialize(&bytes).unwrap();

        assert_eq!(report.mods_checked, restored.mods_checked);
        assert_eq!(report.load_order.len(), restored.load_order.len());
    }
}

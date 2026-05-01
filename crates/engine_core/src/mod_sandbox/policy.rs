//! Sandbox policy configuration.
//!
//! Policies define what capabilities are allowed or denied for mods,
//! along with resource budget limits.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    budget::ResourceBudget,
    capability::{CapabilityCategory, SandboxCapability},
    id::PolicyId,
};

/// Decision on a capability request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityDecision {
    Allow,
    #[default]
    Deny,
    Prompt,
}

/// Rule for capability decisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRule {
    pub capability: SandboxCapability,
    pub decision: CapabilityDecision,
    #[serde(default)]
    pub reason: Option<String>,
}

impl CapabilityRule {
    #[must_use]
    pub fn allow(capability: SandboxCapability) -> Self {
        Self {
            capability,
            decision: CapabilityDecision::Allow,
            reason: None,
        }
    }

    #[must_use]
    pub fn deny(capability: SandboxCapability) -> Self {
        Self {
            capability,
            decision: CapabilityDecision::Deny,
            reason: None,
        }
    }

    #[must_use]
    pub fn prompt(capability: SandboxCapability) -> Self {
        Self {
            capability,
            decision: CapabilityDecision::Prompt,
            reason: None,
        }
    }

    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// A complete sandbox policy configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub id: PolicyId,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<CapabilityRule>,
    #[serde(default)]
    pub category_defaults: Vec<(CapabilityCategory, CapabilityDecision)>,
    #[serde(default)]
    pub default_decision: CapabilityDecision,
    #[serde(default)]
    pub budget_limits: ResourceBudget,
    #[serde(default)]
    pub allow_high_risk: bool,
    #[serde(default)]
    pub trusted_mods: HashSet<String>,
}

impl SandboxPolicy {
    #[must_use]
    pub fn new(id: PolicyId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: String::new(),
            enabled: true,
            rules: Vec::new(),
            category_defaults: Vec::new(),
            default_decision: CapabilityDecision::Deny,
            budget_limits: ResourceBudget::default(),
            allow_high_risk: false,
            trusted_mods: HashSet::new(),
        }
    }

    #[must_use]
    pub fn permissive(id: PolicyId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: "Permissive policy allowing most capabilities".to_string(),
            enabled: true,
            rules: Vec::new(),
            category_defaults: Vec::new(),
            default_decision: CapabilityDecision::Allow,
            budget_limits: ResourceBudget::default(),
            allow_high_risk: false,
            trusted_mods: HashSet::new(),
        }
    }

    #[must_use]
    pub fn restrictive(id: PolicyId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: "Restrictive policy denying most capabilities".to_string(),
            enabled: true,
            rules: Vec::new(),
            category_defaults: Vec::new(),
            default_decision: CapabilityDecision::Deny,
            budget_limits: ResourceBudget::default(),
            allow_high_risk: false,
            trusted_mods: HashSet::new(),
        }
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    #[must_use]
    pub fn with_rule(mut self, rule: CapabilityRule) -> Self {
        self.rules.push(rule);
        self
    }

    #[must_use]
    pub fn with_category_default(
        mut self,
        category: CapabilityCategory,
        decision: CapabilityDecision,
    ) -> Self {
        self.category_defaults.push((category, decision));
        self
    }

    #[must_use]
    pub fn with_budget_limits(mut self, limits: ResourceBudget) -> Self {
        self.budget_limits = limits;
        self
    }

    #[must_use]
    pub fn allowing_high_risk(mut self) -> Self {
        self.allow_high_risk = true;
        self
    }

    #[must_use]
    pub fn with_trusted_mod(mut self, mod_name: impl Into<String>) -> Self {
        self.trusted_mods.insert(mod_name.into());
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check the decision for a specific capability.
    #[must_use]
    pub fn check_capability(&self, cap: &SandboxCapability) -> CapabilityDecision {
        if cap.is_high_risk() && !self.allow_high_risk {
            return CapabilityDecision::Deny;
        }

        for rule in &self.rules {
            if &rule.capability == cap {
                return rule.decision;
            }
        }

        let category = cap.category();
        for (cat, decision) in &self.category_defaults {
            if *cat == category {
                return *decision;
            }
        }

        self.default_decision
    }

    /// Check if a mod is trusted by this policy.
    #[must_use]
    pub fn is_mod_trusted(&self, mod_name: &str) -> bool {
        self.trusted_mods.contains(mod_name)
    }

    /// Validate a set of capability requirements against this policy.
    #[must_use]
    pub fn validate_requirements(
        &self,
        requirements: &super::capability::CapabilityRequirements,
    ) -> PolicyValidation {
        let mut validation = PolicyValidation::ok();

        for cap in &requirements.required {
            match self.check_capability(cap) {
                CapabilityDecision::Allow => {}
                CapabilityDecision::Deny => {
                    validation.add_denied(cap.clone());
                }
                CapabilityDecision::Prompt => {
                    validation.add_needs_prompt(cap.clone());
                }
            }
        }

        for cap in &requirements.optional {
            match self.check_capability(cap) {
                CapabilityDecision::Allow => {
                    validation.add_granted(cap.clone());
                }
                CapabilityDecision::Deny => {}
                CapabilityDecision::Prompt => {
                    validation.add_needs_prompt(cap.clone());
                }
            }
        }

        validation
    }
}

/// Result of validating capability requirements against a policy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyValidation {
    pub allowed: bool,
    pub granted: Vec<SandboxCapability>,
    pub denied: Vec<SandboxCapability>,
    pub needs_prompt: Vec<SandboxCapability>,
}

impl PolicyValidation {
    #[must_use]
    pub fn ok() -> Self {
        Self {
            allowed: true,
            granted: Vec::new(),
            denied: Vec::new(),
            needs_prompt: Vec::new(),
        }
    }

    pub fn add_granted(&mut self, cap: SandboxCapability) {
        self.granted.push(cap);
    }

    pub fn add_denied(&mut self, cap: SandboxCapability) {
        self.allowed = false;
        self.denied.push(cap);
    }

    pub fn add_needs_prompt(&mut self, cap: SandboxCapability) {
        self.needs_prompt.push(cap);
    }

    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        self.allowed
    }

    #[must_use]
    pub fn needs_user_prompt(&self) -> bool {
        !self.needs_prompt.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mod_sandbox::capability::CapabilityRequirements;

    #[test]
    fn policy_default_deny() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test");

        assert_eq!(
            policy.check_capability(&SandboxCapability::ReadOwnData),
            CapabilityDecision::Deny
        );
    }

    #[test]
    fn policy_permissive_default_allow() {
        let policy = SandboxPolicy::permissive(PolicyId::new(1, 1), "test");

        assert_eq!(
            policy.check_capability(&SandboxCapability::ReadOwnData),
            CapabilityDecision::Allow
        );
    }

    #[test]
    fn policy_specific_rule_overrides() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test")
            .with_rule(CapabilityRule::allow(SandboxCapability::ReadOwnData));

        assert_eq!(
            policy.check_capability(&SandboxCapability::ReadOwnData),
            CapabilityDecision::Allow
        );
        assert_eq!(
            policy.check_capability(&SandboxCapability::WriteOwnData),
            CapabilityDecision::Deny
        );
    }

    #[test]
    fn policy_category_default() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test")
            .with_category_default(CapabilityCategory::Filesystem, CapabilityDecision::Allow);

        assert_eq!(
            policy.check_capability(&SandboxCapability::ReadOwnData),
            CapabilityDecision::Allow
        );
        assert_eq!(
            policy.check_capability(&SandboxCapability::WriteOwnData),
            CapabilityDecision::Allow
        );
        assert_eq!(
            policy.check_capability(&SandboxCapability::NetworkLocalhost),
            CapabilityDecision::Deny
        );
    }

    #[test]
    fn policy_high_risk_denied_by_default() {
        let policy = SandboxPolicy::permissive(PolicyId::new(1, 1), "test");

        assert_eq!(
            policy.check_capability(&SandboxCapability::NetworkInternet),
            CapabilityDecision::Deny
        );
    }

    #[test]
    fn policy_high_risk_allowed_when_enabled() {
        let policy = SandboxPolicy::permissive(PolicyId::new(1, 1), "test").allowing_high_risk();

        assert_eq!(
            policy.check_capability(&SandboxCapability::NetworkInternet),
            CapabilityDecision::Allow
        );
    }

    #[test]
    fn policy_validate_requirements() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test")
            .with_rule(CapabilityRule::allow(SandboxCapability::ReadOwnData))
            .with_rule(CapabilityRule::prompt(SandboxCapability::NetworkLocalhost));

        let reqs = CapabilityRequirements::new()
            .with_required(SandboxCapability::ReadOwnData)
            .with_required(SandboxCapability::MutateBlocks)
            .with_optional(SandboxCapability::NetworkLocalhost);

        let validation = policy.validate_requirements(&reqs);

        assert!(!validation.is_allowed());
        assert!(validation.denied.contains(&SandboxCapability::MutateBlocks));
        assert!(
            validation
                .needs_prompt
                .contains(&SandboxCapability::NetworkLocalhost)
        );
    }

    #[test]
    fn policy_trusted_mods() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test")
            .with_trusted_mod("official_mod")
            .with_trusted_mod("verified_mod");

        assert!(policy.is_mod_trusted("official_mod"));
        assert!(policy.is_mod_trusted("verified_mod"));
        assert!(!policy.is_mod_trusted("unknown_mod"));
    }

    #[test]
    fn policy_serde_roundtrip() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test_policy")
            .with_description("Test policy")
            .with_rule(CapabilityRule::allow(SandboxCapability::ReadOwnData))
            .with_trusted_mod("trusted");

        let json = serde_json::to_string(&policy).unwrap();
        let restored: SandboxPolicy = serde_json::from_str(&json).unwrap();

        assert_eq!(policy.name, restored.name);
        assert_eq!(policy.rules.len(), restored.rules.len());
    }

    #[test]
    fn policy_bincode_roundtrip() {
        let policy = SandboxPolicy::new(PolicyId::new(1, 1), "test")
            .with_rule(CapabilityRule::deny(SandboxCapability::NativeFFI))
            .with_budget_limits(ResourceBudget::default());

        let bytes = bincode::serialize(&policy).unwrap();
        let restored: SandboxPolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(policy.id, restored.id);
        assert_eq!(policy.name, restored.name);
    }
}

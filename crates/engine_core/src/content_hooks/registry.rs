//! Content hook registry for managing registered hooks and their components.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{
    descriptor::{ActionDescriptor, ConditionDescriptor, ContentHookDescriptor, EventDescriptor},
    error::{ContentHookError, ContentHookResult},
    fingerprint::{ContentHookFingerprint, HookFingerprintBuilder},
    id::{ActionId, ConditionId, ContentHookId, EventId},
};

/// ID generator for deterministic hook ID allocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HookIdGenerator {
    seed: u32,
    next_hook: u32,
    next_event: u32,
    next_condition: u32,
    next_action: u32,
}

impl HookIdGenerator {
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            next_hook: 0,
            next_event: 0,
            next_condition: 0,
            next_action: 0,
        }
    }

    pub fn generate_hook_id(&mut self) -> ContentHookId {
        let id = ContentHookId::new(self.seed, self.next_hook);
        self.next_hook = self.next_hook.wrapping_add(1);
        id
    }

    pub fn generate_event_id(&mut self) -> EventId {
        let id = EventId::new(self.seed, self.next_event);
        self.next_event = self.next_event.wrapping_add(1);
        id
    }

    pub fn generate_condition_id(&mut self) -> ConditionId {
        let id = ConditionId::new(self.seed, self.next_condition);
        self.next_condition = self.next_condition.wrapping_add(1);
        id
    }

    pub fn generate_action_id(&mut self) -> ActionId {
        let id = ActionId::new(self.seed, self.next_action);
        self.next_action = self.next_action.wrapping_add(1);
        id
    }
}

/// Query parameters for searching hooks.
#[derive(Clone, Debug, Default)]
pub struct HookQuery {
    pub enabled_only: bool,
    pub event_name: Option<String>,
    pub has_tag: Option<String>,
    pub min_priority: Option<i32>,
}

impl HookQuery {
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
    pub fn for_event(mut self, event_name: impl Into<String>) -> Self {
        self.event_name = Some(event_name.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.has_tag = Some(tag.into());
        self
    }

    #[must_use]
    pub fn min_priority(mut self, priority: i32) -> Self {
        self.min_priority = Some(priority);
        self
    }
}

/// Validation report for content hooks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HookValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl HookValidationReport {
    #[must_use]
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.valid = false;
        self.errors.push(error.into());
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }
}

/// Activation status for a hook.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookActivationStatus {
    Ready,
    Disabled,
    MissingReferences { missing: Vec<String> },
    Invalid { reason: String },
}

impl HookActivationStatus {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Activation plan entry for a hook.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HookActivationPlan {
    pub hook_id: ContentHookId,
    pub hook_name: String,
    pub status: HookActivationStatus,
    pub execution_order: u32,
}

impl HookActivationPlan {
    #[must_use]
    pub fn ready(hook_id: ContentHookId, hook_name: String, order: u32) -> Self {
        Self {
            hook_id,
            hook_name,
            status: HookActivationStatus::Ready,
            execution_order: order,
        }
    }

    #[must_use]
    pub fn disabled(hook_id: ContentHookId, hook_name: String) -> Self {
        Self {
            hook_id,
            hook_name,
            status: HookActivationStatus::Disabled,
            execution_order: 0,
        }
    }

    #[must_use]
    pub fn missing_refs(hook_id: ContentHookId, hook_name: String, missing: Vec<String>) -> Self {
        Self {
            hook_id,
            hook_name,
            status: HookActivationStatus::MissingReferences { missing },
            execution_order: 0,
        }
    }
}

/// Main registry for content hooks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContentHookRegistry {
    hooks: BTreeMap<ContentHookId, ContentHookDescriptor>,
    events: BTreeMap<EventId, EventDescriptor>,
    conditions: BTreeMap<ConditionId, ConditionDescriptor>,
    actions: BTreeMap<ActionId, ActionDescriptor>,
    hook_name_index: HashMap<String, ContentHookId>,
    event_name_index: HashMap<String, EventId>,
    condition_name_index: HashMap<String, ConditionId>,
    action_name_index: HashMap<String, ActionId>,
    event_hooks_index: HashMap<String, Vec<ContentHookId>>,
    id_gen: HookIdGenerator,
}

impl ContentHookRegistry {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            hooks: BTreeMap::new(),
            events: BTreeMap::new(),
            conditions: BTreeMap::new(),
            actions: BTreeMap::new(),
            hook_name_index: HashMap::new(),
            event_name_index: HashMap::new(),
            condition_name_index: HashMap::new(),
            action_name_index: HashMap::new(),
            event_hooks_index: HashMap::new(),
            id_gen: HookIdGenerator::new(seed),
        }
    }

    pub fn generate_hook_id(&mut self) -> ContentHookId {
        self.id_gen.generate_hook_id()
    }

    pub fn generate_event_id(&mut self) -> EventId {
        self.id_gen.generate_event_id()
    }

    pub fn generate_condition_id(&mut self) -> ConditionId {
        self.id_gen.generate_condition_id()
    }

    pub fn generate_action_id(&mut self) -> ActionId {
        self.id_gen.generate_action_id()
    }

    /// Register an event descriptor.
    ///
    /// # Errors
    /// Returns an error if an event with the same ID or name already exists.
    pub fn register_event(&mut self, event: EventDescriptor) -> ContentHookResult<EventId> {
        if self.events.contains_key(&event.id) {
            return Err(ContentHookError::DuplicateEventId(event.id));
        }
        if self.event_name_index.contains_key(&event.name) {
            return Err(ContentHookError::DuplicateEventName(event.name.clone()));
        }

        let id = event.id;
        self.event_name_index.insert(event.name.clone(), id);
        self.events.insert(id, event);
        Ok(id)
    }

    /// Register a condition descriptor.
    ///
    /// # Errors
    /// Returns an error if a condition with the same ID or name already exists.
    pub fn register_condition(
        &mut self,
        condition: ConditionDescriptor,
    ) -> ContentHookResult<ConditionId> {
        if self.conditions.contains_key(&condition.id) {
            return Err(ContentHookError::DuplicateConditionId(condition.id));
        }
        if self.condition_name_index.contains_key(&condition.name) {
            return Err(ContentHookError::DuplicateConditionName(
                condition.name.clone(),
            ));
        }

        let id = condition.id;
        self.condition_name_index.insert(condition.name.clone(), id);
        self.conditions.insert(id, condition);
        Ok(id)
    }

    /// Register an action descriptor.
    ///
    /// # Errors
    /// Returns an error if an action with the same ID or name already exists.
    pub fn register_action(&mut self, action: ActionDescriptor) -> ContentHookResult<ActionId> {
        if self.actions.contains_key(&action.id) {
            return Err(ContentHookError::DuplicateActionId(action.id));
        }
        if self.action_name_index.contains_key(&action.name) {
            return Err(ContentHookError::DuplicateActionName(action.name.clone()));
        }

        let id = action.id;
        self.action_name_index.insert(action.name.clone(), id);
        self.actions.insert(id, action);
        Ok(id)
    }

    /// Register a content hook.
    ///
    /// # Errors
    /// Returns an error if a hook with the same ID or name already exists.
    pub fn register_hook(
        &mut self,
        hook: ContentHookDescriptor,
    ) -> ContentHookResult<ContentHookId> {
        if self.hooks.contains_key(&hook.id) {
            return Err(ContentHookError::DuplicateHookId(hook.id));
        }
        if self.hook_name_index.contains_key(&hook.name) {
            return Err(ContentHookError::DuplicateHookName(hook.name.clone()));
        }

        let id = hook.id;
        let event_ref = hook.event_ref.clone();

        self.hook_name_index.insert(hook.name.clone(), id);
        self.event_hooks_index
            .entry(event_ref)
            .or_default()
            .push(id);
        self.hooks.insert(id, hook);
        Ok(id)
    }

    /// Unregister an event.
    pub fn unregister_event(&mut self, id: EventId) -> Option<EventDescriptor> {
        if let Some(event) = self.events.remove(&id) {
            self.event_name_index.remove(&event.name);
            Some(event)
        } else {
            None
        }
    }

    /// Unregister a condition.
    pub fn unregister_condition(&mut self, id: ConditionId) -> Option<ConditionDescriptor> {
        if let Some(condition) = self.conditions.remove(&id) {
            self.condition_name_index.remove(&condition.name);
            Some(condition)
        } else {
            None
        }
    }

    /// Unregister an action.
    pub fn unregister_action(&mut self, id: ActionId) -> Option<ActionDescriptor> {
        if let Some(action) = self.actions.remove(&id) {
            self.action_name_index.remove(&action.name);
            Some(action)
        } else {
            None
        }
    }

    /// Unregister a hook.
    pub fn unregister_hook(&mut self, id: ContentHookId) -> Option<ContentHookDescriptor> {
        if let Some(hook) = self.hooks.remove(&id) {
            self.hook_name_index.remove(&hook.name);
            if let Some(hooks) = self.event_hooks_index.get_mut(&hook.event_ref) {
                hooks.retain(|&h| h != id);
            }
            Some(hook)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_hook(&self, id: ContentHookId) -> Option<&ContentHookDescriptor> {
        self.hooks.get(&id)
    }

    #[must_use]
    pub fn get_hook_by_name(&self, name: &str) -> Option<&ContentHookDescriptor> {
        self.hook_name_index
            .get(name)
            .and_then(|id| self.hooks.get(id))
    }

    #[must_use]
    pub fn get_event(&self, id: EventId) -> Option<&EventDescriptor> {
        self.events.get(&id)
    }

    #[must_use]
    pub fn get_event_by_name(&self, name: &str) -> Option<&EventDescriptor> {
        self.event_name_index
            .get(name)
            .and_then(|id| self.events.get(id))
    }

    #[must_use]
    pub fn get_condition(&self, id: ConditionId) -> Option<&ConditionDescriptor> {
        self.conditions.get(&id)
    }

    #[must_use]
    pub fn get_condition_by_name(&self, name: &str) -> Option<&ConditionDescriptor> {
        self.condition_name_index
            .get(name)
            .and_then(|id| self.conditions.get(id))
    }

    #[must_use]
    pub fn get_action(&self, id: ActionId) -> Option<&ActionDescriptor> {
        self.actions.get(&id)
    }

    #[must_use]
    pub fn get_action_by_name(&self, name: &str) -> Option<&ActionDescriptor> {
        self.action_name_index
            .get(name)
            .and_then(|id| self.actions.get(id))
    }

    #[must_use]
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    #[must_use]
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
            && self.events.is_empty()
            && self.conditions.is_empty()
            && self.actions.is_empty()
    }

    /// Iterate over all hooks in deterministic order.
    pub fn iter_hooks(&self) -> impl Iterator<Item = &ContentHookDescriptor> {
        self.hooks.values()
    }

    /// Iterate over all events in deterministic order.
    pub fn iter_events(&self) -> impl Iterator<Item = &EventDescriptor> {
        self.events.values()
    }

    /// Iterate over all conditions in deterministic order.
    pub fn iter_conditions(&self) -> impl Iterator<Item = &ConditionDescriptor> {
        self.conditions.values()
    }

    /// Iterate over all actions in deterministic order.
    pub fn iter_actions(&self) -> impl Iterator<Item = &ActionDescriptor> {
        self.actions.values()
    }

    /// Query hooks with filter.
    #[must_use]
    pub fn query_hooks(&self, query: &HookQuery) -> Vec<&ContentHookDescriptor> {
        let mut results: Vec<_> = self
            .hooks
            .values()
            .filter(|hook| {
                if query.enabled_only && !hook.enabled {
                    return false;
                }
                if let Some(ref event_name) = query.event_name
                    && &hook.event_ref != event_name
                {
                    return false;
                }
                if let Some(ref tag) = query.has_tag
                    && !hook.tags.contains(tag)
                {
                    return false;
                }
                if let Some(min_pri) = query.min_priority
                    && hook.priority < min_pri
                {
                    return false;
                }
                true
            })
            .collect();

        results.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.name.cmp(&b.name)));
        results
    }

    /// Get hooks registered for a specific event, sorted by priority.
    #[must_use]
    pub fn hooks_for_event(&self, event_name: &str) -> Vec<&ContentHookDescriptor> {
        self.event_hooks_index
            .get(event_name)
            .map(|ids| {
                let mut hooks: Vec<_> = ids.iter().filter_map(|id| self.hooks.get(id)).collect();
                hooks.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.name.cmp(&b.name)));
                hooks
            })
            .unwrap_or_default()
    }

    /// Validate all registered hooks and their references.
    #[must_use]
    pub fn validate(&self) -> HookValidationReport {
        let mut report = HookValidationReport::ok();

        for hook in self.hooks.values() {
            if !self.event_name_index.contains_key(&hook.event_ref) {
                report.add_error(format!(
                    "Hook '{}' references undefined event '{}'",
                    hook.name, hook.event_ref
                ));
            }

            for cond_ref in &hook.conditions {
                if !self.condition_name_index.contains_key(cond_ref) {
                    report.add_error(format!(
                        "Hook '{}' references undefined condition '{}'",
                        hook.name, cond_ref
                    ));
                }
            }

            for action_ref in &hook.actions {
                if !self.action_name_index.contains_key(action_ref) {
                    report.add_error(format!(
                        "Hook '{}' references undefined action '{}'",
                        hook.name, action_ref
                    ));
                }
            }

            if hook.actions.is_empty() {
                report.add_warning(format!("Hook '{}' has no actions defined", hook.name));
            }
        }

        for condition in self.conditions.values() {
            for sub_ref in &condition.sub_conditions {
                if !self.condition_name_index.contains_key(sub_ref) {
                    report.add_error(format!(
                        "Condition '{}' references undefined sub-condition '{}'",
                        condition.name, sub_ref
                    ));
                }
            }
        }

        for action in self.actions.values() {
            for sub_ref in &action.sub_actions {
                if !self.action_name_index.contains_key(sub_ref) {
                    report.add_error(format!(
                        "Action '{}' references undefined sub-action '{}'",
                        action.name, sub_ref
                    ));
                }
            }

            if let Some(ref cond_ref) = action.condition_ref
                && !self.condition_name_index.contains_key(cond_ref)
            {
                report.add_error(format!(
                    "Action '{}' references undefined condition '{}'",
                    action.name, cond_ref
                ));
            }
        }

        self.check_circular_references(&mut report);

        report
    }

    fn check_circular_references(&self, report: &mut HookValidationReport) {
        for condition in self.conditions.values() {
            let mut visited = HashSet::new();
            let mut path = vec![condition.name.clone()];

            if self.has_condition_cycle(&condition.name, &mut visited, &mut path) {
                report.add_error(format!(
                    "Circular condition reference: {}",
                    path.join(" -> ")
                ));
            }
        }

        for action in self.actions.values() {
            let mut visited = HashSet::new();
            let mut path = vec![action.name.clone()];

            if self.has_action_cycle(&action.name, &mut visited, &mut path) {
                report.add_error(format!("Circular action reference: {}", path.join(" -> ")));
            }
        }
    }

    fn has_condition_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if visited.contains(name) {
            return true;
        }

        if let Some(condition) = self.get_condition_by_name(name) {
            visited.insert(name.to_string());

            for sub in &condition.sub_conditions {
                path.push(sub.clone());
                if self.has_condition_cycle(sub, visited, path) {
                    return true;
                }
                path.pop();
            }

            visited.remove(name);
        }

        false
    }

    fn has_action_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if visited.contains(name) {
            return true;
        }

        if let Some(action) = self.get_action_by_name(name) {
            visited.insert(name.to_string());

            for sub in &action.sub_actions {
                path.push(sub.clone());
                if self.has_action_cycle(sub, visited, path) {
                    return true;
                }
                path.pop();
            }

            visited.remove(name);
        }

        false
    }

    /// Generate activation plans for all hooks.
    #[must_use]
    pub fn generate_activation_plan(&self) -> Vec<HookActivationPlan> {
        let mut plans = Vec::new();
        let validation = self.validate();

        let mut enabled_hooks: Vec<_> = self.hooks.values().filter(|h| h.enabled).collect();

        enabled_hooks.sort_by(|a, b| {
            a.event_ref
                .cmp(&b.event_ref)
                .then(b.priority.cmp(&a.priority))
                .then(a.name.cmp(&b.name))
        });

        for hook in self.hooks.values() {
            if !hook.enabled {
                plans.push(HookActivationPlan::disabled(hook.id, hook.name.clone()));
                continue;
            }

            let mut missing = Vec::new();

            if !self.event_name_index.contains_key(&hook.event_ref) {
                missing.push(format!("event:{}", hook.event_ref));
            }

            for cond_ref in &hook.conditions {
                if !self.condition_name_index.contains_key(cond_ref) {
                    missing.push(format!("condition:{cond_ref}"));
                }
            }

            for action_ref in &hook.actions {
                if !self.action_name_index.contains_key(action_ref) {
                    missing.push(format!("action:{action_ref}"));
                }
            }

            if !missing.is_empty() {
                plans.push(HookActivationPlan::missing_refs(
                    hook.id,
                    hook.name.clone(),
                    missing,
                ));
            } else if !validation.is_valid() {
                let relevant_errors: Vec<_> = validation
                    .errors
                    .iter()
                    .filter(|e| e.contains(&hook.name))
                    .cloned()
                    .collect();

                if !relevant_errors.is_empty() {
                    plans.push(HookActivationPlan {
                        hook_id: hook.id,
                        hook_name: hook.name.clone(),
                        status: HookActivationStatus::Invalid {
                            reason: relevant_errors.join("; "),
                        },
                        execution_order: 0,
                    });
                }
            }
        }

        let mut order = 0_u32;
        for hook in &enabled_hooks {
            if !plans.iter().any(|p| p.hook_id == hook.id) {
                plans.push(HookActivationPlan::ready(hook.id, hook.name.clone(), order));
                order = order.saturating_add(1);
            }
        }

        plans.sort_by(|a, b| {
            let status_order = |s: &HookActivationStatus| -> u8 {
                match s {
                    HookActivationStatus::Ready => 0,
                    HookActivationStatus::MissingReferences { .. } => 1,
                    HookActivationStatus::Invalid { .. } => 2,
                    HookActivationStatus::Disabled => 3,
                }
            };
            status_order(&a.status)
                .cmp(&status_order(&b.status))
                .then(a.execution_order.cmp(&b.execution_order))
        });

        plans
    }

    /// Compute a combined fingerprint for all content.
    #[must_use]
    pub fn combined_fingerprint(&self) -> ContentHookFingerprint {
        let mut builder = HookFingerprintBuilder::new();

        for event in self.events.values() {
            builder.add(&event.id.raw());
            builder.add(&event.name);
        }

        for condition in self.conditions.values() {
            builder.add(&condition.id.raw());
            builder.add(&condition.name);
        }

        for action in self.actions.values() {
            builder.add(&action.id.raw());
            builder.add(&action.name);
        }

        for hook in self.hooks.values() {
            builder.add(&hook.id.raw());
            builder.add(&hook.name);
            builder.add(&hook.event_ref);
        }

        builder.finish()
    }

    /// Set hook enabled state.
    pub fn set_hook_enabled(&mut self, id: ContentHookId, enabled: bool) -> bool {
        if let Some(hook) = self.hooks.get_mut(&id) {
            hook.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get enabled hooks in activation order.
    #[must_use]
    pub fn enabled_hooks_in_order(&self) -> Vec<&ContentHookDescriptor> {
        self.generate_activation_plan()
            .into_iter()
            .filter(|p| p.status.is_ready())
            .filter_map(|p| self.hooks.get(&p.hook_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_hooks::descriptor::{ActionType, ConditionType, EventTrigger};

    fn make_event(registry: &mut ContentHookRegistry, name: &str) -> EventDescriptor {
        EventDescriptor::new(registry.generate_event_id(), name)
    }

    fn make_condition(registry: &mut ContentHookRegistry, name: &str) -> ConditionDescriptor {
        ConditionDescriptor::new(registry.generate_condition_id(), name)
    }

    fn make_action(registry: &mut ContentHookRegistry, name: &str) -> ActionDescriptor {
        ActionDescriptor::new(registry.generate_action_id(), name)
    }

    fn make_hook(
        registry: &mut ContentHookRegistry,
        name: &str,
        event_ref: &str,
    ) -> ContentHookDescriptor {
        ContentHookDescriptor::new(registry.generate_hook_id(), name, event_ref)
    }

    #[test]
    fn register_and_get_event() {
        let mut registry = ContentHookRegistry::new(42);
        let event = make_event(&mut registry, "test_event");
        let id = event.id;

        registry.register_event(event).unwrap();

        assert!(registry.get_event(id).is_some());
        assert!(registry.get_event_by_name("test_event").is_some());
    }

    #[test]
    fn duplicate_event_rejected() {
        let mut registry = ContentHookRegistry::new(42);
        let event1 = make_event(&mut registry, "same_name");
        let event2 = make_event(&mut registry, "same_name");

        registry.register_event(event1).unwrap();
        let result = registry.register_event(event2);

        assert!(matches!(
            result,
            Err(ContentHookError::DuplicateEventName(_))
        ));
    }

    #[test]
    fn register_complete_hook() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "on_kill").with_trigger(EventTrigger::OnEntityDeath);
        registry.register_event(event).unwrap();

        let condition = make_condition(&mut registry, "is_boss").with_type(ConditionType::HasTag);
        registry.register_condition(condition).unwrap();

        let action = make_action(&mut registry, "give_loot").with_type(ActionType::GiveItem);
        registry.register_action(action).unwrap();

        let hook = make_hook(&mut registry, "boss_loot", "on_kill")
            .with_condition("is_boss")
            .with_action("give_loot");
        registry.register_hook(hook).unwrap();

        let report = registry.validate();
        assert!(report.is_valid());
    }

    #[test]
    fn validate_missing_event_ref() {
        let mut registry = ContentHookRegistry::new(42);

        let action = make_action(&mut registry, "test_action");
        registry.register_action(action).unwrap();

        let hook =
            make_hook(&mut registry, "test_hook", "nonexistent_event").with_action("test_action");
        registry.register_hook(hook).unwrap();

        let report = registry.validate();
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| e.contains("undefined event")));
    }

    #[test]
    fn validate_missing_condition_ref() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "test_event");
        registry.register_event(event).unwrap();

        let action = make_action(&mut registry, "test_action");
        registry.register_action(action).unwrap();

        let hook = make_hook(&mut registry, "test_hook", "test_event")
            .with_condition("nonexistent_condition")
            .with_action("test_action");
        registry.register_hook(hook).unwrap();

        let report = registry.validate();
        assert!(!report.is_valid());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("undefined condition"))
        );
    }

    #[test]
    fn validate_circular_condition_ref() {
        let mut registry = ContentHookRegistry::new(42);

        let cond_a = make_condition(&mut registry, "cond_a")
            .with_type(ConditionType::And)
            .with_sub_condition("cond_b");
        let cond_b = make_condition(&mut registry, "cond_b")
            .with_type(ConditionType::And)
            .with_sub_condition("cond_a");

        registry.register_condition(cond_a).unwrap();
        registry.register_condition(cond_b).unwrap();

        let report = registry.validate();
        assert!(!report.is_valid());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("Circular condition"))
        );
    }

    #[test]
    fn query_hooks() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "test_event");
        registry.register_event(event).unwrap();

        let action = make_action(&mut registry, "test_action");
        registry.register_action(action).unwrap();

        let hook1 = make_hook(&mut registry, "hook_a", "test_event")
            .with_priority(10)
            .with_action("test_action")
            .with_tag("combat");
        let hook2 = make_hook(&mut registry, "hook_b", "test_event")
            .with_priority(5)
            .with_action("test_action");
        let hook3 = make_hook(&mut registry, "hook_c", "test_event")
            .disabled()
            .with_action("test_action");

        registry.register_hook(hook1).unwrap();
        registry.register_hook(hook2).unwrap();
        registry.register_hook(hook3).unwrap();

        let query = HookQuery::new().enabled_only();
        let results = registry.query_hooks(&query);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "hook_a");

        let query = HookQuery::new().with_tag("combat");
        let results = registry.query_hooks(&query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn hooks_for_event_sorted() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "test_event");
        registry.register_event(event).unwrap();

        let action = make_action(&mut registry, "test_action");
        registry.register_action(action).unwrap();

        let low_priority_hook = make_hook(&mut registry, "low_priority", "test_event")
            .with_priority(1)
            .with_action("test_action");
        let high_priority_hook = make_hook(&mut registry, "high_priority", "test_event")
            .with_priority(100)
            .with_action("test_action");
        let med_priority_hook = make_hook(&mut registry, "medium_priority", "test_event")
            .with_priority(50)
            .with_action("test_action");

        registry.register_hook(low_priority_hook).unwrap();
        registry.register_hook(high_priority_hook).unwrap();
        registry.register_hook(med_priority_hook).unwrap();

        let hooks = registry.hooks_for_event("test_event");
        assert_eq!(hooks[0].name, "high_priority");
        assert_eq!(hooks[1].name, "medium_priority");
        assert_eq!(hooks[2].name, "low_priority");
    }

    #[test]
    fn activation_plan() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "test_event");
        registry.register_event(event).unwrap();

        let action = make_action(&mut registry, "test_action");
        registry.register_action(action).unwrap();

        let hook1 =
            make_hook(&mut registry, "enabled_hook", "test_event").with_action("test_action");
        let hook2 = make_hook(&mut registry, "disabled_hook", "test_event")
            .disabled()
            .with_action("test_action");
        let hook3 =
            make_hook(&mut registry, "missing_ref", "nonexistent").with_action("test_action");

        registry.register_hook(hook1).unwrap();
        registry.register_hook(hook2).unwrap();
        registry.register_hook(hook3).unwrap();

        let plans = registry.generate_activation_plan();

        let ready: Vec<_> = plans.iter().filter(|p| p.status.is_ready()).collect();
        let disabled: Vec<_> = plans
            .iter()
            .filter(|p| matches!(p.status, HookActivationStatus::Disabled))
            .collect();
        let missing: Vec<_> = plans
            .iter()
            .filter(|p| matches!(p.status, HookActivationStatus::MissingReferences { .. }))
            .collect();

        assert_eq!(ready.len(), 1);
        assert_eq!(disabled.len(), 1);
        assert_eq!(missing.len(), 1);
    }

    #[test]
    fn fingerprint_stability() {
        let mut registry1 = ContentHookRegistry::new(42);
        let event1 = make_event(&mut registry1, "event");
        registry1.register_event(event1).unwrap();

        let mut registry2 = ContentHookRegistry::new(42);
        let event2 = make_event(&mut registry2, "event");
        registry2.register_event(event2).unwrap();

        assert_eq!(
            registry1.combined_fingerprint(),
            registry2.combined_fingerprint()
        );
    }

    #[test]
    fn fingerprint_changes() {
        let mut registry1 = ContentHookRegistry::new(42);
        let event1 = make_event(&mut registry1, "event_a");
        registry1.register_event(event1).unwrap();

        let mut registry2 = ContentHookRegistry::new(42);
        let event2 = make_event(&mut registry2, "event_b");
        registry2.register_event(event2).unwrap();

        assert_ne!(
            registry1.combined_fingerprint(),
            registry2.combined_fingerprint()
        );
    }

    #[test]
    fn serde_roundtrip() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "test_event");
        registry.register_event(event).unwrap();

        let condition = make_condition(&mut registry, "test_condition");
        registry.register_condition(condition).unwrap();

        let action = make_action(&mut registry, "test_action");
        registry.register_action(action).unwrap();

        let hook = make_hook(&mut registry, "test_hook", "test_event")
            .with_condition("test_condition")
            .with_action("test_action");
        registry.register_hook(hook).unwrap();

        let bytes = bincode::serialize(&registry).unwrap();
        let restored: ContentHookRegistry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(registry.hook_count(), restored.hook_count());
        assert_eq!(registry.event_count(), restored.event_count());
        assert_eq!(
            registry.combined_fingerprint(),
            restored.combined_fingerprint()
        );
    }

    #[test]
    fn unregister_components() {
        let mut registry = ContentHookRegistry::new(42);

        let event = make_event(&mut registry, "test_event");
        let event_id = event.id;
        registry.register_event(event).unwrap();

        let condition = make_condition(&mut registry, "test_condition");
        let condition_id = condition.id;
        registry.register_condition(condition).unwrap();

        let action = make_action(&mut registry, "test_action");
        let action_id = action.id;
        registry.register_action(action).unwrap();

        assert!(registry.unregister_event(event_id).is_some());
        assert!(registry.unregister_condition(condition_id).is_some());
        assert!(registry.unregister_action(action_id).is_some());

        assert!(registry.get_event(event_id).is_none());
        assert!(registry.get_condition(condition_id).is_none());
        assert!(registry.get_action(action_id).is_none());
    }

    #[test]
    fn iter_deterministic() {
        let mut registry = ContentHookRegistry::new(42);

        for i in 0..5 {
            let event = make_event(&mut registry, &format!("event_{i}"));
            registry.register_event(event).unwrap();
        }

        let ids1: Vec<_> = registry.iter_events().map(|e| e.id).collect();
        let ids2: Vec<_> = registry.iter_events().map(|e| e.id).collect();

        assert_eq!(ids1, ids2);
    }
}

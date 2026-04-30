//! Task definitions and templates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::ids::{CapabilityId, TaskDefId};

/// Category for grouping task definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    Construction,
    Production,
    Hauling,
    Maintenance,
    Research,
    Medical,
    Combat,
    Social,
}

/// Defines how task priority is calculated.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PriorityMode {
    #[default]
    Fixed,
    Distance,
    Age,
    Custom,
}

/// Definition of a task type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskDef {
    pub id: TaskDefId,
    pub name: String,
    pub description: Option<String>,
    pub category: TaskCategory,
    pub required_capabilities: BTreeSet<CapabilityId>,
    pub base_work_amount: u32,
    pub base_priority: i32,
    pub priority_mode: PriorityMode,
    pub interruptible: bool,
    pub prerequisites: BTreeSet<TaskDefId>,
    pub max_workers: u32,
    pub auto_cancel_on_failure: bool,
}

impl TaskDef {
    #[must_use]
    pub fn new(id: TaskDefId, name: impl Into<String>, category: TaskCategory) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
            category,
            required_capabilities: BTreeSet::new(),
            base_work_amount: 100,
            base_priority: 0,
            priority_mode: PriorityMode::default(),
            interruptible: true,
            prerequisites: BTreeSet::new(),
            max_workers: 1,
            auto_cancel_on_failure: false,
        }
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    #[must_use]
    pub fn with_capability(mut self, cap: CapabilityId) -> Self {
        self.required_capabilities.insert(cap);
        self
    }

    #[must_use]
    pub fn with_capabilities(mut self, caps: impl IntoIterator<Item = CapabilityId>) -> Self {
        self.required_capabilities.extend(caps);
        self
    }

    #[must_use]
    pub fn with_work_amount(mut self, amount: u32) -> Self {
        self.base_work_amount = amount;
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.base_priority = priority;
        self
    }

    #[must_use]
    pub fn with_priority_mode(mut self, mode: PriorityMode) -> Self {
        self.priority_mode = mode;
        self
    }

    #[must_use]
    pub fn with_prerequisite(mut self, prereq: TaskDefId) -> Self {
        self.prerequisites.insert(prereq);
        self
    }

    #[must_use]
    pub fn with_max_workers(mut self, max: u32) -> Self {
        self.max_workers = max.max(1);
        self
    }

    #[must_use]
    pub fn non_interruptible(mut self) -> Self {
        self.interruptible = false;
        self
    }

    #[must_use]
    pub fn auto_cancel(mut self) -> Self {
        self.auto_cancel_on_failure = true;
        self
    }

    #[must_use]
    pub fn requires_capability(&self, cap: &CapabilityId) -> bool {
        self.required_capabilities.contains(cap)
    }

    #[must_use]
    pub fn has_prerequisites(&self) -> bool {
        !self.prerequisites.is_empty()
    }
}

/// Registry of task definitions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskDefRegistry {
    defs: std::collections::BTreeMap<TaskDefId, TaskDef>,
}

impl TaskDefRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: TaskDef) {
        self.defs.insert(def.id.clone(), def);
    }

    #[must_use]
    pub fn get(&self, id: &TaskDefId) -> Option<&TaskDef> {
        self.defs.get(id)
    }

    pub fn remove(&mut self, id: &TaskDefId) -> Option<TaskDef> {
        self.defs.remove(id)
    }

    #[must_use]
    pub fn contains(&self, id: &TaskDefId) -> bool {
        self.defs.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TaskDef> {
        self.defs.values()
    }

    #[must_use]
    pub fn by_category(&self, category: TaskCategory) -> Vec<&TaskDef> {
        self.defs
            .values()
            .filter(|d| d.category == category)
            .collect()
    }

    #[must_use]
    pub fn requiring_capability(&self, cap: &CapabilityId) -> Vec<&TaskDef> {
        self.defs
            .values()
            .filter(|d| d.requires_capability(cap))
            .collect()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.defs.len()
    }
}

/// Well-known task definition IDs.
pub mod presets {
    use super::{PriorityMode, TaskCategory, TaskDef, TaskDefId};
    use crate::settler::capability::presets as cap;

    pub fn mine_rock() -> TaskDefId {
        TaskDefId::new("mine_rock")
    }

    pub fn haul_item() -> TaskDefId {
        TaskDefId::new("haul_item")
    }

    pub fn construct_wall() -> TaskDefId {
        TaskDefId::new("construct_wall")
    }

    pub fn plant_crop() -> TaskDefId {
        TaskDefId::new("plant_crop")
    }

    pub fn harvest_crop() -> TaskDefId {
        TaskDefId::new("harvest_crop")
    }

    pub fn cook_meal() -> TaskDefId {
        TaskDefId::new("cook_meal")
    }

    pub fn treat_patient() -> TaskDefId {
        TaskDefId::new("treat_patient")
    }

    pub fn research_tech() -> TaskDefId {
        TaskDefId::new("research_tech")
    }

    pub fn clean_area() -> TaskDefId {
        TaskDefId::new("clean_area")
    }

    pub fn standard_task_defs() -> Vec<TaskDef> {
        vec![
            TaskDef::new(mine_rock(), "Mine Rock", TaskCategory::Production)
                .with_capability(cap::mining())
                .with_work_amount(150)
                .with_priority(5),
            TaskDef::new(haul_item(), "Haul Item", TaskCategory::Hauling)
                .with_capability(cap::hauling())
                .with_work_amount(50)
                .with_priority(3)
                .with_priority_mode(PriorityMode::Distance),
            TaskDef::new(
                construct_wall(),
                "Construct Wall",
                TaskCategory::Construction,
            )
            .with_capability(cap::construction())
            .with_work_amount(200)
            .with_priority(4)
            .with_max_workers(2),
            TaskDef::new(plant_crop(), "Plant Crop", TaskCategory::Production)
                .with_capability(cap::farming())
                .with_work_amount(30)
                .with_priority(6),
            TaskDef::new(harvest_crop(), "Harvest Crop", TaskCategory::Production)
                .with_capability(cap::farming())
                .with_work_amount(40)
                .with_priority(7),
            TaskDef::new(cook_meal(), "Cook Meal", TaskCategory::Production)
                .with_capability(cap::cooking())
                .with_work_amount(80)
                .with_priority(8),
            TaskDef::new(treat_patient(), "Treat Patient", TaskCategory::Medical)
                .with_capability(cap::medical())
                .with_work_amount(100)
                .with_priority(10)
                .non_interruptible(),
            TaskDef::new(
                research_tech(),
                "Research Technology",
                TaskCategory::Research,
            )
            .with_capability(cap::research())
            .with_work_amount(500)
            .with_priority(2),
            TaskDef::new(clean_area(), "Clean Area", TaskCategory::Maintenance)
                .with_capability(cap::cleaning())
                .with_work_amount(60)
                .with_priority(1),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskCategory, TaskDef, TaskDefRegistry, presets};
    use crate::settler::capability::presets as cap;

    #[test]
    fn test_task_def_creation() {
        let def = TaskDef::new(presets::mine_rock(), "Mine Rock", TaskCategory::Production)
            .with_capability(cap::mining())
            .with_work_amount(150);

        assert_eq!(def.name, "Mine Rock");
        assert_eq!(def.base_work_amount, 150);
        assert!(def.requires_capability(&cap::mining()));
        assert!(!def.requires_capability(&cap::hauling()));
    }

    #[test]
    fn test_task_def_prerequisites() {
        let def = TaskDef::new(
            presets::construct_wall(),
            "Build",
            TaskCategory::Construction,
        )
        .with_prerequisite(presets::haul_item());

        assert!(def.has_prerequisites());
        assert!(def.prerequisites.contains(&presets::haul_item()));
    }

    #[test]
    fn test_task_def_registry() {
        let mut registry = TaskDefRegistry::new();
        registry.register(TaskDef::new(
            presets::mine_rock(),
            "Mine",
            TaskCategory::Production,
        ));
        registry.register(TaskDef::new(
            presets::haul_item(),
            "Haul",
            TaskCategory::Hauling,
        ));

        assert_eq!(registry.count(), 2);
        assert!(registry.contains(&presets::mine_rock()));
        assert!(registry.get(&presets::mine_rock()).is_some());
    }

    #[test]
    fn test_registry_by_category() {
        let mut registry = TaskDefRegistry::new();
        for def in presets::standard_task_defs() {
            registry.register(def);
        }

        let production = registry.by_category(TaskCategory::Production);
        assert!(production.len() >= 2);
    }

    #[test]
    fn test_task_def_serde() {
        let def = TaskDef::new(presets::mine_rock(), "Mine Rock", TaskCategory::Production)
            .with_capability(cap::mining())
            .with_work_amount(150);

        let json = serde_json::to_string(&def).unwrap();
        let restored: TaskDef = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, def.name);
        assert_eq!(restored.base_work_amount, def.base_work_amount);
    }
}

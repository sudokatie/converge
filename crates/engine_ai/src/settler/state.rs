//! Settler/worker state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::capability::SkillSet;
use super::ids::{RegionId, SettlerId, TaskId};
use super::task::TaskPosition;

/// Current activity status of a settler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SettlerStatus {
    Idle,
    Working,
    Traveling,
    Resting,
    Incapacitated,
}

impl SettlerStatus {
    #[must_use]
    pub fn can_work(&self) -> bool {
        matches!(self, Self::Idle | Self::Working | Self::Traveling)
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

/// Work priority settings for a settler.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkPriorities {
    priorities: std::collections::BTreeMap<super::task_def::TaskCategory, i32>,
    forbidden_categories: BTreeSet<super::task_def::TaskCategory>,
}

impl WorkPriorities {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_priority(&mut self, category: super::task_def::TaskCategory, priority: i32) {
        self.priorities.insert(category, priority);
    }

    #[must_use]
    pub fn get_priority(&self, category: super::task_def::TaskCategory) -> i32 {
        self.priorities.get(&category).copied().unwrap_or(0)
    }

    pub fn forbid(&mut self, category: super::task_def::TaskCategory) {
        self.forbidden_categories.insert(category);
    }

    pub fn allow(&mut self, category: super::task_def::TaskCategory) {
        self.forbidden_categories.remove(&category);
    }

    #[must_use]
    pub fn is_forbidden(&self, category: super::task_def::TaskCategory) -> bool {
        self.forbidden_categories.contains(&category)
    }
}

/// A settler/worker entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settler {
    pub id: SettlerId,
    pub name: String,
    pub status: SettlerStatus,
    pub skills: SkillSet,
    pub work_priorities: WorkPriorities,
    pub region: Option<RegionId>,
    pub position: Option<TaskPosition>,
    pub current_task: Option<TaskId>,
    pub reserved_tasks: BTreeSet<TaskId>,
    pub work_speed_modifier: f32,
    pub created_tick: u64,
    pub total_work_done: u64,
    pub tasks_completed: u32,
}

impl Settler {
    #[must_use]
    pub fn new(id: SettlerId, name: impl Into<String>, created_tick: u64) -> Self {
        Self {
            id,
            name: name.into(),
            status: SettlerStatus::Idle,
            skills: SkillSet::new(),
            work_priorities: WorkPriorities::new(),
            region: None,
            position: None,
            current_task: None,
            reserved_tasks: BTreeSet::new(),
            work_speed_modifier: 1.0,
            created_tick,
            total_work_done: 0,
            tasks_completed: 0,
        }
    }

    #[must_use]
    pub fn with_region(mut self, region: RegionId) -> Self {
        self.region = Some(region);
        self
    }

    #[must_use]
    pub fn with_position(mut self, pos: TaskPosition) -> Self {
        self.position = Some(pos);
        self
    }

    #[must_use]
    pub fn with_skills(mut self, skills: SkillSet) -> Self {
        self.skills = skills;
        self
    }

    #[must_use]
    pub fn with_work_speed(mut self, modifier: f32) -> Self {
        self.work_speed_modifier = modifier.clamp(0.1, 5.0);
        self
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.status.is_available() && self.current_task.is_none()
    }

    #[must_use]
    pub fn can_work(&self) -> bool {
        self.status.can_work()
    }

    #[must_use]
    pub fn is_working(&self) -> bool {
        self.current_task.is_some()
    }

    pub fn assign_task(&mut self, task: TaskId) {
        self.current_task = Some(task);
        self.status = SettlerStatus::Working;
    }

    pub fn clear_task(&mut self) {
        self.current_task = None;
        if self.status == SettlerStatus::Working {
            self.status = SettlerStatus::Idle;
        }
    }

    pub fn reserve_task(&mut self, task: TaskId) {
        self.reserved_tasks.insert(task);
    }

    pub fn unreserve_task(&mut self, task: TaskId) -> bool {
        self.reserved_tasks.remove(&task)
    }

    #[must_use]
    pub fn has_reservation(&self, task: TaskId) -> bool {
        self.reserved_tasks.contains(&task)
    }

    pub fn record_work(&mut self, amount: u32) {
        self.total_work_done += u64::from(amount);
    }

    pub fn record_completion(&mut self) {
        self.tasks_completed += 1;
    }

    pub fn set_status(&mut self, status: SettlerStatus) {
        self.status = status;
        if !status.can_work() {
            self.current_task = None;
        }
    }

    pub fn incapacitate(&mut self) {
        self.set_status(SettlerStatus::Incapacitated);
        self.reserved_tasks.clear();
    }

    pub fn recover(&mut self) {
        if self.status == SettlerStatus::Incapacitated {
            self.status = SettlerStatus::Idle;
        }
    }

    #[must_use]
    pub fn distance_to(&self, pos: &TaskPosition) -> Option<f32> {
        self.position.as_ref().map(|p| p.distance(pos))
    }

    #[must_use]
    pub fn effective_work_speed(&self, capability: &super::ids::CapabilityId) -> f32 {
        let skill_modifier = self
            .skills
            .effective_level(capability)
            .map_or(0.5, super::capability::SkillLevel::speed_multiplier);
        self.work_speed_modifier * skill_modifier
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::settler::capability::{Skill, SkillLevel, presets as cap};

    #[test]
    fn test_settler_creation() {
        let settler = Settler::new(SettlerId::new(1), "Alice", 0);
        assert_eq!(settler.name, "Alice");
        assert_eq!(settler.status, SettlerStatus::Idle);
        assert!(settler.is_available());
    }

    #[test]
    fn test_settler_task_assignment() {
        let mut settler = Settler::new(SettlerId::new(1), "Bob", 0);
        let task = TaskId::new(1);

        settler.assign_task(task);
        assert!(!settler.is_available());
        assert!(settler.is_working());
        assert_eq!(settler.current_task, Some(task));
        assert_eq!(settler.status, SettlerStatus::Working);

        settler.clear_task();
        assert!(settler.is_available());
        assert!(settler.current_task.is_none());
    }

    #[test]
    fn test_settler_reservations() {
        let mut settler = Settler::new(SettlerId::new(1), "Carol", 0);
        let task1 = TaskId::new(1);
        let task2 = TaskId::new(2);

        settler.reserve_task(task1);
        settler.reserve_task(task2);

        assert!(settler.has_reservation(task1));
        assert!(settler.has_reservation(task2));
        assert!(!settler.has_reservation(TaskId::new(3)));

        settler.unreserve_task(task1);
        assert!(!settler.has_reservation(task1));
    }

    #[test]
    fn test_settler_incapacitation() {
        let mut settler = Settler::new(SettlerId::new(1), "Dave", 0);
        settler.assign_task(TaskId::new(1));
        settler.reserve_task(TaskId::new(2));

        settler.incapacitate();
        assert_eq!(settler.status, SettlerStatus::Incapacitated);
        assert!(settler.current_task.is_none());
        assert!(settler.reserved_tasks.is_empty());
        assert!(!settler.can_work());

        settler.recover();
        assert_eq!(settler.status, SettlerStatus::Idle);
    }

    #[test]
    fn test_settler_work_tracking() {
        let mut settler = Settler::new(SettlerId::new(1), "Eve", 0);
        settler.record_work(100);
        settler.record_work(50);
        settler.record_completion();

        assert_eq!(settler.total_work_done, 150);
        assert_eq!(settler.tasks_completed, 1);
    }

    #[test]
    fn test_settler_work_speed() {
        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(cap::mining()).with_level(SkillLevel::Expert));

        let settler = Settler::new(SettlerId::new(1), "Frank", 0).with_skills(skills);

        let speed = settler.effective_work_speed(&cap::mining());
        assert!(speed > 1.0);
    }

    #[test]
    fn test_work_priorities() {
        use super::super::task_def::TaskCategory;

        let mut priorities = WorkPriorities::new();
        priorities.set_priority(TaskCategory::Medical, 10);
        priorities.forbid(TaskCategory::Combat);

        assert_eq!(priorities.get_priority(TaskCategory::Medical), 10);
        assert_eq!(priorities.get_priority(TaskCategory::Production), 0);
        assert!(priorities.is_forbidden(TaskCategory::Combat));
        assert!(!priorities.is_forbidden(TaskCategory::Medical));
    }

    #[test]
    fn test_settler_distance() {
        let settler = Settler::new(SettlerId::new(1), "Grace", 0)
            .with_position(TaskPosition::new(0.0, 0.0, 0.0));

        let target = TaskPosition::new(3.0, 4.0, 0.0);
        let dist = settler.distance_to(&target).unwrap();
        assert!((dist - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_settler_serde() {
        let settler = Settler::new(SettlerId::new(42), "Henry", 100)
            .with_region(RegionId::new(1))
            .with_work_speed(1.2);

        let json = serde_json::to_string(&settler).unwrap();
        let restored: Settler = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, settler.id);
        assert_eq!(restored.name, "Henry");
        assert!((restored.work_speed_modifier - 1.2).abs() < f32::EPSILON);
    }
}

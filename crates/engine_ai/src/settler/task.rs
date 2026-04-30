//! Task instance state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::ids::{RegionId, SettlerId, TaskDefId, TaskId};

/// Position within a region (generic coordinates).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl TaskPosition {
    #[must_use]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn distance_squared(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    #[must_use]
    pub fn distance(&self, other: &Self) -> f32 {
        self.distance_squared(other).sqrt()
    }
}

/// Current status of a task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Claimed,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Claimed | Self::InProgress | Self::Paused)
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub fn can_be_claimed(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// Reason why a task failed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureReason {
    NoWorkers,
    MissingPrerequisite,
    ResourcesUnavailable,
    PathBlocked,
    WorkerIncapacitated,
    Timeout,
    External(String),
}

/// A task instance with state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub def_id: TaskDefId,
    pub status: TaskStatus,
    pub region: Option<RegionId>,
    pub position: Option<TaskPosition>,
    pub created_tick: u64,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    pub work_done: u32,
    pub work_required: u32,
    pub assigned_workers: BTreeSet<SettlerId>,
    pub priority_override: Option<i32>,
    pub failure_reason: Option<FailureReason>,
    pub parent_task: Option<TaskId>,
    pub child_tasks: BTreeSet<TaskId>,
    pub blocked_by: BTreeSet<TaskId>,
}

impl Task {
    #[must_use]
    pub fn new(id: TaskId, def_id: TaskDefId, work_required: u32, created_tick: u64) -> Self {
        Self {
            id,
            def_id,
            status: TaskStatus::Pending,
            region: None,
            position: None,
            created_tick,
            started_tick: None,
            completed_tick: None,
            work_done: 0,
            work_required,
            assigned_workers: BTreeSet::new(),
            priority_override: None,
            failure_reason: None,
            parent_task: None,
            child_tasks: BTreeSet::new(),
            blocked_by: BTreeSet::new(),
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
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority_override = Some(priority);
        self
    }

    #[must_use]
    pub fn with_parent(mut self, parent: TaskId) -> Self {
        self.parent_task = Some(parent);
        self
    }

    #[must_use]
    pub fn with_blocker(mut self, blocker: TaskId) -> Self {
        self.blocked_by.insert(blocker);
        self
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "work amounts bounded")]
    pub fn progress(&self) -> f32 {
        if self.work_required == 0 {
            return 1.0;
        }
        (self.work_done as f32) / (self.work_required as f32)
    }

    #[must_use]
    pub fn remaining_work(&self) -> u32 {
        self.work_required.saturating_sub(self.work_done)
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty()
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.status == TaskStatus::Completed
    }

    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.assigned_workers.len()
    }

    #[must_use]
    pub fn has_worker(&self, settler: SettlerId) -> bool {
        self.assigned_workers.contains(&settler)
    }

    pub fn assign_worker(&mut self, settler: SettlerId) {
        self.assigned_workers.insert(settler);
    }

    pub fn unassign_worker(&mut self, settler: SettlerId) -> bool {
        self.assigned_workers.remove(&settler)
    }

    pub fn add_work(&mut self, amount: u32) -> bool {
        self.work_done = self.work_done.saturating_add(amount);
        self.work_done >= self.work_required
    }

    pub fn claim(&mut self, settler: SettlerId, tick: u64) {
        self.status = TaskStatus::Claimed;
        self.assign_worker(settler);
        if self.started_tick.is_none() {
            self.started_tick = Some(tick);
        }
    }

    pub fn start(&mut self, tick: u64) {
        self.status = TaskStatus::InProgress;
        if self.started_tick.is_none() {
            self.started_tick = Some(tick);
        }
    }

    pub fn pause(&mut self) {
        if self.status.is_active() {
            self.status = TaskStatus::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.status == TaskStatus::Paused {
            self.status = TaskStatus::InProgress;
        }
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = TaskStatus::Completed;
        self.completed_tick = Some(tick);
        self.work_done = self.work_required;
    }

    pub fn fail(&mut self, reason: FailureReason, tick: u64) {
        self.status = TaskStatus::Failed;
        self.completed_tick = Some(tick);
        self.failure_reason = Some(reason);
    }

    pub fn cancel(&mut self, tick: u64) {
        self.status = TaskStatus::Cancelled;
        self.completed_tick = Some(tick);
    }

    pub fn release(&mut self) {
        self.status = TaskStatus::Pending;
        self.assigned_workers.clear();
    }

    pub fn remove_blocker(&mut self, blocker: TaskId) -> bool {
        self.blocked_by.remove(&blocker)
    }

    pub fn add_child(&mut self, child: TaskId) {
        self.child_tasks.insert(child);
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick)
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> Option<u64> {
        self.started_tick
            .map(|start| current_tick.saturating_sub(start))
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::settler::task_def::presets;

    #[test]
    fn test_task_creation() {
        let task = Task::new(TaskId::new(1), presets::mine_rock(), 100, 0);
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.work_required, 100);
        assert_eq!(task.work_done, 0);
        assert!((task.progress()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_task_progress() {
        let mut task = Task::new(TaskId::new(1), presets::mine_rock(), 100, 0);
        task.add_work(50);
        assert!((task.progress() - 0.5).abs() < f32::EPSILON);
        assert_eq!(task.remaining_work(), 50);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new(TaskId::new(1), presets::mine_rock(), 100, 0);
        let settler = SettlerId::new(1);

        task.claim(settler, 10);
        assert_eq!(task.status, TaskStatus::Claimed);
        assert!(task.has_worker(settler));
        assert_eq!(task.started_tick, Some(10));

        task.start(10);
        assert_eq!(task.status, TaskStatus::InProgress);

        task.add_work(100);
        task.complete(50);
        assert!(task.is_complete());
        assert_eq!(task.completed_tick, Some(50));
    }

    #[test]
    fn test_task_failure() {
        let mut task = Task::new(TaskId::new(1), presets::mine_rock(), 100, 0);
        task.fail(FailureReason::ResourcesUnavailable, 20);

        assert_eq!(task.status, TaskStatus::Failed);
        assert!(matches!(
            task.failure_reason,
            Some(FailureReason::ResourcesUnavailable)
        ));
    }

    #[test]
    fn test_task_blocking() {
        let mut task = Task::new(TaskId::new(1), presets::construct_wall(), 200, 0)
            .with_blocker(TaskId::new(2));

        assert!(task.is_blocked());
        task.remove_blocker(TaskId::new(2));
        assert!(!task.is_blocked());
    }

    #[test]
    fn test_task_position() {
        let pos1 = TaskPosition::new(0.0, 0.0, 0.0);
        let pos2 = TaskPosition::new(3.0, 4.0, 0.0);
        assert!((pos1.distance(&pos2) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_task_status_checks() {
        assert!(TaskStatus::Pending.is_available());
        assert!(TaskStatus::Pending.can_be_claimed());
        assert!(TaskStatus::InProgress.is_active());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
    }

    #[test]
    fn test_task_serde() {
        let task = Task::new(TaskId::new(42), presets::mine_rock(), 100, 0)
            .with_region(RegionId::new(1))
            .with_position(TaskPosition::new(10.0, 20.0, 0.0));

        let json = serde_json::to_string(&task).unwrap();
        let restored: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, task.id);
        assert_eq!(restored.region, task.region);
    }
}

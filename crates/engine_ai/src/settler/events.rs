//! Events emitted by the settler task AI system.

use serde::{Deserialize, Serialize};

use super::ids::{CapabilityId, RegionId, SettlerId, TaskDefId, TaskId};
use super::task::FailureReason;

/// Kind of settler event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SettlerEventKind {
    TaskCreated {
        task: TaskId,
        def: TaskDefId,
        region: Option<RegionId>,
    },
    TaskClaimed {
        task: TaskId,
        settler: SettlerId,
    },
    TaskStarted {
        task: TaskId,
        settler: SettlerId,
    },
    TaskProgress {
        task: TaskId,
        settler: SettlerId,
        work_done: u32,
        progress: f32,
    },
    TaskCompleted {
        task: TaskId,
        settler: SettlerId,
        duration_ticks: u64,
    },
    TaskFailed {
        task: TaskId,
        settler: Option<SettlerId>,
        reason: FailureReason,
    },
    TaskCancelled {
        task: TaskId,
        reason: String,
    },
    TaskReassigned {
        task: TaskId,
        from_settler: SettlerId,
        to_settler: SettlerId,
    },
    TaskReleased {
        task: TaskId,
        settler: SettlerId,
    },
    SettlerCreated {
        settler: SettlerId,
        name: String,
    },
    SettlerIncapacitated {
        settler: SettlerId,
        dropped_task: Option<TaskId>,
    },
    SettlerRecovered {
        settler: SettlerId,
    },
    SettlerIdle {
        settler: SettlerId,
    },
    SkillLevelUp {
        settler: SettlerId,
        capability: CapabilityId,
        new_level: String,
    },
    PrerequisiteMet {
        task: TaskId,
        prerequisite: TaskId,
    },
    TaskUnblocked {
        task: TaskId,
    },
}

/// An event emitted by the settler task AI system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettlerEvent {
    pub tick: u64,
    pub kind: SettlerEventKind,
}

impl SettlerEvent {
    #[must_use]
    pub fn new(tick: u64, kind: SettlerEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn task_created(tick: u64, task: TaskId, def: TaskDefId, region: Option<RegionId>) -> Self {
        Self::new(tick, SettlerEventKind::TaskCreated { task, def, region })
    }

    #[must_use]
    pub fn task_claimed(tick: u64, task: TaskId, settler: SettlerId) -> Self {
        Self::new(tick, SettlerEventKind::TaskClaimed { task, settler })
    }

    #[must_use]
    pub fn task_started(tick: u64, task: TaskId, settler: SettlerId) -> Self {
        Self::new(tick, SettlerEventKind::TaskStarted { task, settler })
    }

    #[must_use]
    pub fn task_progress(
        tick: u64,
        task: TaskId,
        settler: SettlerId,
        work_done: u32,
        progress: f32,
    ) -> Self {
        Self::new(
            tick,
            SettlerEventKind::TaskProgress {
                task,
                settler,
                work_done,
                progress,
            },
        )
    }

    #[must_use]
    pub fn task_completed(
        tick: u64,
        task: TaskId,
        settler: SettlerId,
        duration_ticks: u64,
    ) -> Self {
        Self::new(
            tick,
            SettlerEventKind::TaskCompleted {
                task,
                settler,
                duration_ticks,
            },
        )
    }

    #[must_use]
    pub fn task_failed(
        tick: u64,
        task: TaskId,
        settler: Option<SettlerId>,
        reason: FailureReason,
    ) -> Self {
        Self::new(
            tick,
            SettlerEventKind::TaskFailed {
                task,
                settler,
                reason,
            },
        )
    }

    #[must_use]
    pub fn task_cancelled(tick: u64, task: TaskId, reason: impl Into<String>) -> Self {
        Self::new(
            tick,
            SettlerEventKind::TaskCancelled {
                task,
                reason: reason.into(),
            },
        )
    }

    #[must_use]
    pub fn settler_created(tick: u64, settler: SettlerId, name: impl Into<String>) -> Self {
        Self::new(
            tick,
            SettlerEventKind::SettlerCreated {
                settler,
                name: name.into(),
            },
        )
    }

    #[must_use]
    pub fn settler_incapacitated(
        tick: u64,
        settler: SettlerId,
        dropped_task: Option<TaskId>,
    ) -> Self {
        Self::new(
            tick,
            SettlerEventKind::SettlerIncapacitated {
                settler,
                dropped_task,
            },
        )
    }

    #[must_use]
    pub fn skill_level_up(
        tick: u64,
        settler: SettlerId,
        capability: CapabilityId,
        new_level: impl Into<String>,
    ) -> Self {
        Self::new(
            tick,
            SettlerEventKind::SkillLevelUp {
                settler,
                capability,
                new_level: new_level.into(),
            },
        )
    }

    #[must_use]
    pub fn involves_task(&self, task: TaskId) -> bool {
        match &self.kind {
            SettlerEventKind::TaskCreated { task: t, .. }
            | SettlerEventKind::TaskClaimed { task: t, .. }
            | SettlerEventKind::TaskStarted { task: t, .. }
            | SettlerEventKind::TaskProgress { task: t, .. }
            | SettlerEventKind::TaskCompleted { task: t, .. }
            | SettlerEventKind::TaskFailed { task: t, .. }
            | SettlerEventKind::TaskCancelled { task: t, .. }
            | SettlerEventKind::TaskReleased { task: t, .. }
            | SettlerEventKind::PrerequisiteMet { task: t, .. }
            | SettlerEventKind::TaskUnblocked { task: t }
            | SettlerEventKind::TaskReassigned { task: t, .. } => *t == task,
            SettlerEventKind::SettlerIncapacitated { dropped_task, .. } => {
                *dropped_task == Some(task)
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn involves_settler(&self, settler: SettlerId) -> bool {
        match &self.kind {
            SettlerEventKind::TaskClaimed { settler: s, .. }
            | SettlerEventKind::TaskStarted { settler: s, .. }
            | SettlerEventKind::TaskProgress { settler: s, .. }
            | SettlerEventKind::TaskCompleted { settler: s, .. }
            | SettlerEventKind::TaskReleased { settler: s, .. }
            | SettlerEventKind::SettlerCreated { settler: s, .. }
            | SettlerEventKind::SettlerIncapacitated { settler: s, .. }
            | SettlerEventKind::SettlerRecovered { settler: s }
            | SettlerEventKind::SettlerIdle { settler: s }
            | SettlerEventKind::SkillLevelUp { settler: s, .. } => *s == settler,
            SettlerEventKind::TaskFailed { settler: s, .. } => *s == Some(settler),
            SettlerEventKind::TaskReassigned {
                from_settler,
                to_settler,
                ..
            } => *from_settler == settler || *to_settler == settler,
            _ => false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::settler::task_def::presets;

    #[test]
    fn test_event_creation() {
        let event = SettlerEvent::task_created(
            10,
            TaskId::new(1),
            presets::mine_rock(),
            Some(RegionId::new(1)),
        );
        assert_eq!(event.tick, 10);
        assert!(matches!(event.kind, SettlerEventKind::TaskCreated { .. }));
    }

    #[test]
    fn test_event_involves_task() {
        let task = TaskId::new(1);
        let settler = SettlerId::new(1);

        let event = SettlerEvent::task_claimed(0, task, settler);
        assert!(event.involves_task(task));
        assert!(!event.involves_task(TaskId::new(2)));
    }

    #[test]
    fn test_event_involves_settler() {
        let task = TaskId::new(1);
        let settler = SettlerId::new(1);

        let event = SettlerEvent::task_completed(100, task, settler, 50);
        assert!(event.involves_settler(settler));
        assert!(!event.involves_settler(SettlerId::new(2)));
    }

    #[test]
    fn test_event_serde() {
        let event = SettlerEvent::task_progress(50, TaskId::new(1), SettlerId::new(1), 25, 0.5);

        let json = serde_json::to_string(&event).unwrap();
        let restored: SettlerEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 50);
    }
}

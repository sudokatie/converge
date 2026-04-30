//! Settler task AI system for worker assignment and task management.
//!
//! Provides deterministic, data-driven simulation of settler work assignments:
//!
//! - Task definitions and instances with progress tracking
//! - Settler capabilities and skill progression
//! - Priority-based task assignment with distance/age/skill modifiers
//! - Reservation system for task claiming
//! - Event emission for task lifecycle
//! - Summaries and snapshots for state inspection
//! - Stable fingerprints for determinism verification

mod assignment;
mod capability;
mod events;
mod ids;
mod priority;
mod state;
mod task;
mod task_def;

pub use assignment::{AssignmentConfig, AssignmentEngine, AssignmentResult, ReservationTable};
pub use capability::{
    CapabilityCategory, CapabilityDef, Skill, SkillLevel, SkillSet, presets as capability_presets,
};
pub use events::{SettlerEvent, SettlerEventKind};
pub use ids::{CapabilityId, RegionId, SettlerId, TaskDefId, TaskId};
pub use priority::{AssignmentCandidate, PriorityConfig, PriorityScore, RegionPriority};
pub use state::{Settler, SettlerStatus, WorkPriorities};
pub use task::{FailureReason, Task, TaskPosition, TaskStatus};
pub use task_def::{
    PriorityMode, TaskCategory, TaskDef, TaskDefRegistry, presets as task_def_presets,
};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;

/// Configuration for the settler task manager.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettlerManagerConfig {
    pub assignment: AssignmentConfig,
    pub auto_reassign_on_incapacitation: bool,
    pub track_event_history: bool,
    pub max_event_history: usize,
    pub snapshot_interval: u64,
}

impl Default for SettlerManagerConfig {
    fn default() -> Self {
        Self {
            assignment: AssignmentConfig::default(),
            auto_reassign_on_incapacitation: true,
            track_event_history: true,
            max_event_history: 1000,
            snapshot_interval: 100,
        }
    }
}

impl SettlerManagerConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_assignment(mut self, config: AssignmentConfig) -> Self {
        self.assignment = config;
        self
    }

    #[must_use]
    pub fn with_max_event_history(mut self, max: usize) -> Self {
        self.max_event_history = max;
        self
    }
}

/// Result of a settler manager tick.
#[derive(Clone, Debug, Default)]
pub struct SettlerTickResult {
    pub events: Vec<SettlerEvent>,
    pub assignments_made: u32,
    pub tasks_completed: u32,
    pub tasks_failed: u32,
    pub tasks_cancelled: u32,
    pub work_performed: u32,
}

impl SettlerTickResult {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: Self) {
        self.events.extend(other.events);
        self.assignments_made += other.assignments_made;
        self.tasks_completed += other.tasks_completed;
        self.tasks_failed += other.tasks_failed;
        self.tasks_cancelled += other.tasks_cancelled;
        self.work_performed += other.work_performed;
    }
}

/// Snapshot of settler manager state at a point in time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SettlerSnapshot {
    pub tick: u64,
    pub total_settlers: u32,
    pub idle_settlers: u32,
    pub working_settlers: u32,
    pub incapacitated_settlers: u32,
    pub total_tasks: u32,
    pub pending_tasks: u32,
    pub in_progress_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub total_work_done: u64,
    pub tasks_by_category: BTreeMap<TaskCategory, u32>,
}

impl SettlerSnapshot {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded by game limits")]
    pub fn utilization(&self) -> f32 {
        if self.total_settlers == 0 {
            return 0.0;
        }
        self.working_settlers as f32 / self.total_settlers as f32
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded by game limits")]
    pub fn completion_rate(&self) -> f32 {
        let total_finished = self.completed_tasks + self.failed_tasks;
        if total_finished == 0 {
            return 0.0;
        }
        self.completed_tasks as f32 / total_finished as f32
    }
}

/// Summary of settler manager state for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SettlerSummary {
    pub tick: u64,
    pub settler_count: u32,
    pub task_count: u32,
    pub utilization: f32,
    pub completion_rate: f32,
}

impl From<&SettlerSnapshot> for SettlerSummary {
    fn from(snapshot: &SettlerSnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            settler_count: snapshot.total_settlers,
            task_count: snapshot.total_tasks,
            utilization: snapshot.utilization(),
            completion_rate: snapshot.completion_rate(),
        }
    }
}

/// Fingerprint for settler manager state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettlerFingerprint(pub u32);

impl SettlerFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for SettlerFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "settler:{:08x}", self.0)
    }
}

/// Projection of future settler manager state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettlerProjection {
    pub base_tick: u64,
    pub projected_tick: u64,
    pub estimated_completions: u32,
    pub estimated_idle_settlers: u32,
    pub estimated_pending_tasks: u32,
}

impl SettlerProjection {
    #[must_use]
    pub fn new(base_tick: u64, projected_tick: u64) -> Self {
        Self {
            base_tick,
            projected_tick,
            estimated_completions: 0,
            estimated_idle_settlers: 0,
            estimated_pending_tasks: 0,
        }
    }
}

/// Registry for settlers with deterministic iteration order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SettlerRegistry {
    settlers: BTreeMap<SettlerId, Settler>,
    next_id: u64,
}

impl SettlerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, settler: Settler) {
        let id = settler.id;
        self.settlers.insert(id, settler);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn create(&mut self, name: impl Into<String>, created_tick: u64) -> SettlerId {
        let id = SettlerId::new(self.next_id);
        self.next_id += 1;
        let settler = Settler::new(id, name, created_tick);
        self.settlers.insert(id, settler);
        id
    }

    pub fn remove(&mut self, id: SettlerId) -> Option<Settler> {
        self.settlers.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: SettlerId) -> Option<&Settler> {
        self.settlers.get(&id)
    }

    pub fn get_mut(&mut self, id: SettlerId) -> Option<&mut Settler> {
        self.settlers.get_mut(&id)
    }

    #[must_use]
    pub fn contains(&self, id: SettlerId) -> bool {
        self.settlers.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Settler> {
        self.settlers.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Settler> {
        self.settlers.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.settlers.len()
    }

    pub fn available(&self) -> impl Iterator<Item = &Settler> {
        self.settlers.values().filter(|s| s.is_available())
    }

    pub fn by_status(&self, status: SettlerStatus) -> impl Iterator<Item = &Settler> {
        self.settlers.values().filter(move |s| s.status == status)
    }

    pub fn with_capability(&self, cap: &CapabilityId) -> impl Iterator<Item = &Settler> {
        self.settlers
            .values()
            .filter(|s| s.skills.has_capability(cap))
    }
}

/// Registry for tasks with deterministic iteration order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaskRegistry {
    tasks: BTreeMap<TaskId, Task>,
    next_id: u64,
}

impl TaskRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, task: Task) {
        let id = task.id;
        self.tasks.insert(id, task);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn create(&mut self, def_id: TaskDefId, work_required: u32, created_tick: u64) -> TaskId {
        let id = TaskId::new(self.next_id);
        self.next_id += 1;
        let task = Task::new(id, def_id, work_required, created_tick);
        self.tasks.insert(id, task);
        id
    }

    pub fn remove(&mut self, id: TaskId) -> Option<Task> {
        self.tasks.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(&id)
    }

    #[must_use]
    pub fn contains(&self, id: TaskId) -> bool {
        self.tasks.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Task> {
        self.tasks.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.tasks.len()
    }

    pub fn pending(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values().filter(|t| t.status.is_available())
    }

    pub fn active(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values().filter(|t| t.status.is_active())
    }

    pub fn by_status(&self, status: TaskStatus) -> impl Iterator<Item = &Task> {
        self.tasks.values().filter(move |t| t.status == status)
    }

    pub fn by_def<'a>(&'a self, def_id: &'a TaskDefId) -> impl Iterator<Item = &'a Task> + 'a {
        self.tasks.values().filter(move |t| &t.def_id == def_id)
    }

    pub fn in_region(&self, region: RegionId) -> impl Iterator<Item = &Task> {
        self.tasks
            .values()
            .filter(move |t| t.region == Some(region))
    }
}

/// Manager for settler task AI simulation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SettlerManager {
    config: SettlerManagerConfig,
    task_defs: TaskDefRegistry,
    settlers: SettlerRegistry,
    tasks: TaskRegistry,
    reservations: ReservationTable,
    event_history: Vec<SettlerEvent>,
    current_tick: u64,
    total_work_done: u64,
    total_tasks_completed: u64,
    total_tasks_failed: u64,
}

impl SettlerManager {
    #[must_use]
    pub fn new(config: SettlerManagerConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn config(&self) -> &SettlerManagerConfig {
        &self.config
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn task_defs(&self) -> &TaskDefRegistry {
        &self.task_defs
    }

    pub fn task_defs_mut(&mut self) -> &mut TaskDefRegistry {
        &mut self.task_defs
    }

    #[must_use]
    pub fn settlers(&self) -> &SettlerRegistry {
        &self.settlers
    }

    pub fn settlers_mut(&mut self) -> &mut SettlerRegistry {
        &mut self.settlers
    }

    #[must_use]
    pub fn tasks(&self) -> &TaskRegistry {
        &self.tasks
    }

    pub fn tasks_mut(&mut self) -> &mut TaskRegistry {
        &mut self.tasks
    }

    #[must_use]
    pub fn reservations(&self) -> &ReservationTable {
        &self.reservations
    }

    pub fn reservations_mut(&mut self) -> &mut ReservationTable {
        &mut self.reservations
    }

    #[must_use]
    pub fn event_history(&self) -> &[SettlerEvent] {
        &self.event_history
    }

    pub fn register_task_def(&mut self, def: TaskDef) {
        self.task_defs.register(def);
    }

    pub fn create_settler(&mut self, name: impl Into<String>) -> SettlerId {
        let name_str: String = name.into();
        let id = self.settlers.create(name_str.clone(), self.current_tick);
        if self.config.track_event_history {
            self.record_event(SettlerEvent::settler_created(
                self.current_tick,
                id,
                name_str,
            ));
        }
        id
    }

    pub fn create_task(&mut self, def_id: TaskDefId, region: Option<RegionId>) -> Option<TaskId> {
        let def = self.task_defs.get(&def_id)?;
        let work_required = def.base_work_amount;
        let id = self
            .tasks
            .create(def_id.clone(), work_required, self.current_tick);

        if let Some(task) = self.tasks.get_mut(id) {
            task.region = region;
        }

        if self.config.track_event_history {
            self.record_event(SettlerEvent::task_created(
                self.current_tick,
                id,
                def_id,
                region,
            ));
        }
        Some(id)
    }

    pub fn reserve_task(&mut self, task: TaskId, settler: SettlerId) -> bool {
        if !self.tasks.contains(task) || !self.settlers.contains(settler) {
            return false;
        }
        self.reservations.reserve(task, settler);
        if let Some(s) = self.settlers.get_mut(settler) {
            s.reserve_task(task);
        }
        true
    }

    pub fn release_reservation(&mut self, task: TaskId, settler: SettlerId) {
        self.reservations.release(task, settler);
        if let Some(s) = self.settlers.get_mut(settler) {
            s.unreserve_task(task);
        }
    }

    pub fn cancel_task(&mut self, task_id: TaskId, reason: impl Into<String>) {
        let reason_str = reason.into();
        if let Some(task) = self.tasks.get_mut(task_id) {
            let assigned: Vec<SettlerId> = task.assigned_workers.iter().copied().collect();
            task.cancel(self.current_tick);

            for settler_id in assigned {
                if let Some(settler) = self.settlers.get_mut(settler_id) {
                    settler.clear_task();
                }
            }

            self.reservations.release_all_for_task(task_id);

            if self.config.track_event_history {
                self.record_event(SettlerEvent::task_cancelled(
                    self.current_tick,
                    task_id,
                    reason_str,
                ));
            }
            self.total_tasks_failed += 1;
        }
    }

    pub fn incapacitate_settler(&mut self, settler_id: SettlerId) {
        let dropped_task = if let Some(settler) = self.settlers.get_mut(settler_id) {
            let task = settler.current_task;
            settler.incapacitate();
            self.reservations.release_all_for_settler(settler_id);
            task
        } else {
            return;
        };

        if let Some(task_id) = dropped_task
            && let Some(task) = self.tasks.get_mut(task_id)
        {
            task.unassign_worker(settler_id);
            if task.assigned_workers.is_empty() {
                task.release();
            }
        }

        if self.config.track_event_history {
            self.record_event(SettlerEvent::settler_incapacitated(
                self.current_tick,
                settler_id,
                dropped_task,
            ));
        }
    }

    pub fn recover_settler(&mut self, settler_id: SettlerId) {
        if let Some(settler) = self.settlers.get_mut(settler_id) {
            settler.recover();
            if self.config.track_event_history {
                self.record_event(SettlerEvent::new(
                    self.current_tick,
                    SettlerEventKind::SettlerRecovered {
                        settler: settler_id,
                    },
                ));
            }
        }
    }

    pub fn add_dependency(&mut self, task: TaskId, depends_on: TaskId) {
        if let Some(t) = self.tasks.get_mut(task) {
            t.blocked_by.insert(depends_on);
        }
    }

    pub fn tick(&mut self) -> SettlerTickResult {
        self.current_tick += 1;
        let mut result = SettlerTickResult::new();

        self.resolve_blockers(&mut result);
        self.perform_assignments(&mut result);
        self.perform_work(&mut result);

        self.trim_event_history();

        result
    }

    fn resolve_blockers(&mut self, result: &mut SettlerTickResult) {
        let completed_tasks: BTreeSet<TaskId> = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .map(|t| t.id)
            .collect();

        for task in self.tasks.iter_mut() {
            if task.is_blocked() {
                let prev_blocked = task.blocked_by.len();
                task.blocked_by.retain(|id| !completed_tasks.contains(id));

                if task.blocked_by.is_empty() && prev_blocked > 0 && self.config.track_event_history
                {
                    result.events.push(SettlerEvent::new(
                        self.current_tick,
                        SettlerEventKind::TaskUnblocked { task: task.id },
                    ));
                }
            }
        }
    }

    fn perform_assignments(&mut self, result: &mut SettlerTickResult) {
        let engine = AssignmentEngine::new(&self.config.assignment, &self.task_defs);

        let mut tasks_map: BTreeMap<TaskId, Task> = self
            .tasks
            .iter()
            .filter(|t| t.status.can_be_claimed() && !t.is_blocked())
            .map(|t| (t.id, t.clone()))
            .collect();

        let mut settlers_map: BTreeMap<SettlerId, Settler> = self
            .settlers
            .iter()
            .filter(|s| s.is_available())
            .map(|s| (s.id, s.clone()))
            .collect();

        let assignments = engine.assign_greedy(
            &mut tasks_map,
            &mut settlers_map,
            &mut self.reservations,
            self.current_tick,
        );

        for assignment in &assignments {
            if let AssignmentResult::Assigned { task, settler } = assignment {
                if let Some(updated_task) = tasks_map.get(task)
                    && let Some(original) = self.tasks.get_mut(*task)
                {
                    *original = updated_task.clone();
                }
                if let Some(updated_settler) = settlers_map.get(settler)
                    && let Some(original) = self.settlers.get_mut(*settler)
                {
                    *original = updated_settler.clone();
                }

                if self.config.track_event_history {
                    result.events.push(SettlerEvent::task_claimed(
                        self.current_tick,
                        *task,
                        *settler,
                    ));
                }
                result.assignments_made += 1;
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn perform_work(&mut self, result: &mut SettlerTickResult) {
        let working_pairs: Vec<(SettlerId, TaskId)> = self
            .settlers
            .iter()
            .filter_map(|s| s.current_task.map(|t| (s.id, t)))
            .collect();

        for (settler_id, task_id) in working_pairs {
            let Some(task) = self.tasks.get(task_id) else {
                continue;
            };
            let Some(def) = self.task_defs.get(&task.def_id) else {
                continue;
            };
            let Some(settler) = self.settlers.get(settler_id) else {
                continue;
            };

            #[expect(
                clippy::cast_precision_loss,
                reason = "capability count is small and bounded"
            )]
            let work_speed = if def.required_capabilities.is_empty() {
                settler.work_speed_modifier
            } else {
                def.required_capabilities
                    .iter()
                    .map(|cap| settler.effective_work_speed(cap))
                    .sum::<f32>()
                    / def.required_capabilities.len() as f32
            };

            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "work is bounded"
            )]
            let work_amount = (10.0 * work_speed).max(1.0) as u32;

            let task = self.tasks.get_mut(task_id).unwrap();
            if task.status == TaskStatus::Claimed {
                task.start(self.current_tick);
                if self.config.track_event_history {
                    result.events.push(SettlerEvent::task_started(
                        self.current_tick,
                        task_id,
                        settler_id,
                    ));
                }
            }

            let completed = task.add_work(work_amount);
            self.total_work_done += u64::from(work_amount);
            result.work_performed += work_amount;

            let settler = self.settlers.get_mut(settler_id).unwrap();
            settler.record_work(work_amount);

            for cap in &def.required_capabilities {
                if let Some(skill) = settler.skills.get_skill_mut(cap) {
                    let exp_rate = self.task_defs.get(&def.id).map_or(1.0, |_d| {
                        capability_presets::standard_capability_defs()
                            .iter()
                            .find(|c| &c.id == cap)
                            .map_or(1.0, |c| c.base_experience_rate)
                    });
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "work_amount is small per-tick value"
                    )]
                    let leveled = skill.add_experience(work_amount as f32 * exp_rate * 0.1);
                    if leveled && self.config.track_event_history {
                        result.events.push(SettlerEvent::skill_level_up(
                            self.current_tick,
                            settler_id,
                            cap.clone(),
                            format!("{:?}", skill.level),
                        ));
                    }
                }
            }

            if self.config.track_event_history {
                let task = self.tasks.get(task_id).unwrap();
                result.events.push(SettlerEvent::task_progress(
                    self.current_tick,
                    task_id,
                    settler_id,
                    task.work_done,
                    task.progress(),
                ));
            }

            if completed {
                let task = self.tasks.get_mut(task_id).unwrap();
                let started = task.started_tick.unwrap_or(self.current_tick);
                let duration = self.current_tick.saturating_sub(started);
                task.complete(self.current_tick);

                let settler = self.settlers.get_mut(settler_id).unwrap();
                settler.record_completion();
                settler.clear_task();

                self.total_tasks_completed += 1;
                result.tasks_completed += 1;

                if self.config.track_event_history {
                    result.events.push(SettlerEvent::task_completed(
                        self.current_tick,
                        task_id,
                        settler_id,
                        duration,
                    ));
                }
            }
        }
    }

    fn record_event(&mut self, event: SettlerEvent) {
        self.event_history.push(event);
    }

    fn trim_event_history(&mut self) {
        if self.event_history.len() > self.config.max_event_history {
            let excess = self.event_history.len() - self.config.max_event_history;
            self.event_history.drain(0..excess);
        }
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "counts bounded by game limits"
    )]
    pub fn snapshot(&self) -> SettlerSnapshot {
        let mut snapshot = SettlerSnapshot::new(self.current_tick);

        snapshot.total_settlers = self.settlers.count() as u32;
        snapshot.idle_settlers = self.settlers.by_status(SettlerStatus::Idle).count() as u32;
        snapshot.working_settlers = self.settlers.by_status(SettlerStatus::Working).count() as u32;
        snapshot.incapacitated_settlers = self
            .settlers
            .by_status(SettlerStatus::Incapacitated)
            .count() as u32;

        snapshot.total_tasks = self.tasks.count() as u32;
        snapshot.pending_tasks = self.tasks.by_status(TaskStatus::Pending).count() as u32;
        snapshot.in_progress_tasks = self.tasks.by_status(TaskStatus::InProgress).count() as u32
            + self.tasks.by_status(TaskStatus::Claimed).count() as u32;
        snapshot.completed_tasks = self.tasks.by_status(TaskStatus::Completed).count() as u32;
        snapshot.failed_tasks = self.tasks.by_status(TaskStatus::Failed).count() as u32
            + self.tasks.by_status(TaskStatus::Cancelled).count() as u32;

        snapshot.total_work_done = self.total_work_done;

        for task in self.tasks.iter() {
            if let Some(def) = self.task_defs.get(&task.def_id) {
                *snapshot.tasks_by_category.entry(def.category).or_insert(0) += 1;
            }
        }

        snapshot
    }

    #[must_use]
    pub fn summary(&self) -> SettlerSummary {
        SettlerSummary::from(&self.snapshot())
    }

    #[must_use]
    pub fn fingerprint(&self) -> SettlerFingerprint {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        self.current_tick.hash(&mut hasher);
        self.total_work_done.hash(&mut hasher);
        self.total_tasks_completed.hash(&mut hasher);
        self.total_tasks_failed.hash(&mut hasher);

        for settler in self.settlers.iter() {
            settler.id.0.hash(&mut hasher);
            settler.status.hash(&mut hasher);
            settler.current_task.map(|t| t.0).hash(&mut hasher);
        }

        for task in self.tasks.iter() {
            task.id.0.hash(&mut hasher);
            task.status.hash(&mut hasher);
            task.work_done.hash(&mut hasher);
            task.work_required.hash(&mut hasher);
        }

        #[expect(clippy::cast_possible_truncation, reason = "fingerprint is u32")]
        let hash = std::hash::Hasher::finish(&hasher) as u32;
        SettlerFingerprint(hash)
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "projections are estimates with bounded values"
    )]
    pub fn project(&self, ticks_ahead: u64) -> SettlerProjection {
        let mut projection =
            SettlerProjection::new(self.current_tick, self.current_tick + ticks_ahead);

        let avg_work_per_tick: f32 = self
            .settlers
            .iter()
            .filter(|s| s.is_working())
            .map(|s| s.work_speed_modifier * 10.0)
            .sum();

        let total_remaining: u32 = self
            .tasks
            .iter()
            .filter(|t| t.status.is_active())
            .map(task::Task::remaining_work)
            .sum();

        if avg_work_per_tick > 0.0 {
            let ticks_to_complete = total_remaining as f32 / avg_work_per_tick;
            if (ticks_to_complete as u64) <= ticks_ahead {
                projection.estimated_completions = self.tasks.active().count() as u32;
            } else {
                projection.estimated_completions =
                    ((ticks_ahead as f32 * avg_work_per_tick) / 100.0) as u32;
            }
        }

        projection.estimated_idle_settlers = self.settlers.available().count() as u32;
        projection.estimated_pending_tasks = self.tasks.pending().count() as u32;

        projection
    }

    pub fn events_for_task(&self, task: TaskId) -> impl Iterator<Item = &SettlerEvent> {
        self.event_history
            .iter()
            .filter(move |e| e.involves_task(task))
    }

    pub fn events_for_settler(&self, settler: SettlerId) -> impl Iterator<Item = &SettlerEvent> {
        self.event_history
            .iter()
            .filter(move |e| e.involves_settler(settler))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RegionId, SettlerEventKind, SettlerFingerprint, SettlerId, SettlerManager,
        SettlerManagerConfig, SettlerProjection, SettlerSnapshot, SettlerStatus, SettlerSummary,
        Skill, SkillLevel, TaskStatus, capability_presets, task_def_presets,
    };
    use std::collections::BTreeMap;

    fn setup_manager() -> SettlerManager {
        let mut manager = SettlerManager::new(SettlerManagerConfig::default());
        for def in task_def_presets::standard_task_defs() {
            manager.register_task_def(def);
        }
        manager
    }

    fn create_miner(manager: &mut SettlerManager, name: &str) -> SettlerId {
        let id = manager.create_settler(name);
        let settler = manager.settlers_mut().get_mut(id).unwrap();
        settler
            .skills
            .add_skill(Skill::new(capability_presets::mining()));
        id
    }

    fn create_hauler(manager: &mut SettlerManager, name: &str) -> SettlerId {
        let id = manager.create_settler(name);
        let settler = manager.settlers_mut().get_mut(id).unwrap();
        settler
            .skills
            .add_skill(Skill::new(capability_presets::hauling()));
        id
    }

    #[test]
    fn test_manager_creation() {
        let manager = setup_manager();
        assert_eq!(manager.current_tick(), 0);
        assert_eq!(manager.settlers().count(), 0);
        assert_eq!(manager.tasks().count(), 0);
    }

    #[test]
    fn test_settler_creation_events() {
        let mut manager = setup_manager();
        let id = manager.create_settler("Alice");

        assert!(manager.settlers().contains(id));
        assert_eq!(manager.event_history().len(), 1);
        assert!(matches!(
            manager.event_history()[0].kind,
            SettlerEventKind::SettlerCreated { .. }
        ));
    }

    #[test]
    fn test_task_creation() {
        let mut manager = setup_manager();
        let task_id = manager
            .create_task(task_def_presets::mine_rock(), Some(RegionId::new(1)))
            .unwrap();

        let task = manager.tasks().get(task_id).unwrap();
        assert_eq!(task.region, Some(RegionId::new(1)));
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_assignment_by_capability() {
        let mut manager = setup_manager();

        let miner = create_miner(&mut manager, "Miner");
        let hauler = create_hauler(&mut manager, "Hauler");

        let mine_task = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();
        let haul_task = manager
            .create_task(task_def_presets::haul_item(), None)
            .unwrap();

        let result = manager.tick();

        assert_eq!(result.assignments_made, 2);

        let mine_task_state = manager.tasks().get(mine_task).unwrap();
        let haul_task_state = manager.tasks().get(haul_task).unwrap();

        assert!(mine_task_state.has_worker(miner));
        assert!(haul_task_state.has_worker(hauler));
    }

    #[test]
    fn test_task_completion() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        let mut completed = false;
        for _ in 0..100 {
            let result = manager.tick();
            if result.tasks_completed > 0 {
                completed = true;
                break;
            }
        }

        assert!(completed);
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.completed_tasks, 1);
    }

    #[test]
    fn test_task_failure_and_cancellation() {
        let mut manager = setup_manager();
        let task_id = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        manager.cancel_task(task_id, "Testing cancellation");

        let task = manager.tasks().get(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[test]
    fn test_settler_incapacitation() {
        let mut manager = setup_manager();
        let miner = create_miner(&mut manager, "Miner");
        let task_id = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        manager.tick();

        manager.incapacitate_settler(miner);

        let settler = manager.settlers().get(miner).unwrap();
        assert_eq!(settler.status, SettlerStatus::Incapacitated);
        assert!(settler.current_task.is_none());

        let task = manager.tasks().get(task_id).unwrap();
        assert!(!task.has_worker(miner));
    }

    #[test]
    fn test_settler_recovery() {
        let mut manager = setup_manager();
        let miner = create_miner(&mut manager, "Miner");

        manager.incapacitate_settler(miner);
        manager.recover_settler(miner);

        let settler = manager.settlers().get(miner).unwrap();
        assert_eq!(settler.status, SettlerStatus::Idle);
    }

    #[test]
    fn test_task_dependencies() {
        let mut manager = setup_manager();
        create_hauler(&mut manager, "Hauler");

        let prereq = manager
            .create_task(task_def_presets::haul_item(), None)
            .unwrap();
        let dependent = manager
            .create_task(task_def_presets::haul_item(), None)
            .unwrap();

        manager.add_dependency(dependent, prereq);

        manager.tick();

        let dep_task = manager.tasks().get(dependent).unwrap();
        assert!(dep_task.is_blocked());

        for _ in 0..50 {
            manager.tick();
        }

        let dep_task = manager.tasks().get(dependent).unwrap();
        assert!(!dep_task.is_blocked() || dep_task.status.is_active());
    }

    #[test]
    fn test_reservation_system() {
        let mut manager = setup_manager();
        let miner = create_miner(&mut manager, "Miner");
        let task_id = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        assert!(manager.reserve_task(task_id, miner));
        assert!(manager.reservations().has_reservation(task_id, miner));

        let settler = manager.settlers().get(miner).unwrap();
        assert!(settler.has_reservation(task_id));

        manager.release_reservation(task_id, miner);
        assert!(!manager.reservations().has_reservation(task_id, miner));
    }

    #[test]
    fn test_snapshot_accuracy() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner1");
        create_miner(&mut manager, "Miner2");
        create_hauler(&mut manager, "Hauler1");

        manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();
        manager
            .create_task(task_def_presets::haul_item(), None)
            .unwrap();

        manager.tick();

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.total_settlers, 3);
        assert_eq!(snapshot.total_tasks, 2);
        assert_eq!(snapshot.working_settlers, 2);
        assert_eq!(snapshot.idle_settlers, 1);
    }

    #[test]
    fn test_summary_from_snapshot() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();
        manager.tick();

        let summary = manager.summary();
        assert_eq!(summary.settler_count, 1);
        assert_eq!(summary.task_count, 1);
        assert!(summary.utilization > 0.0);
    }

    #[test]
    fn test_fingerprint_determinism() {
        let mut manager1 = setup_manager();
        let mut manager2 = setup_manager();

        let _m1_settler = create_miner(&mut manager1, "Miner");
        let _m2_settler = create_miner(&mut manager2, "Miner");

        manager1.create_task(task_def_presets::mine_rock(), None);
        manager2.create_task(task_def_presets::mine_rock(), None);

        manager1.tick();
        manager2.tick();

        let fp1 = manager1.fingerprint();
        let fp2 = manager2.fingerprint();

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_changes() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_task(task_def_presets::mine_rock(), None);

        let fp_before = manager.fingerprint();
        manager.tick();
        let fp_after = manager.fingerprint();

        assert_ne!(fp_before, fp_after);
    }

    #[test]
    fn test_projection() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_task(task_def_presets::mine_rock(), None);
        manager.tick();

        let projection = manager.project(100);
        assert_eq!(projection.base_tick, manager.current_tick());
        assert_eq!(projection.projected_tick, manager.current_tick() + 100);
    }

    #[test]
    fn test_progress_tracking() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        let task_id = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        manager.tick();
        manager.tick();

        let task = manager.tasks().get(task_id).unwrap();
        assert!(task.work_done > 0);
        assert!(task.progress() > 0.0);
    }

    #[test]
    fn test_settler_registry_queries() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner1");
        create_miner(&mut manager, "Miner2");
        create_hauler(&mut manager, "Hauler");

        let miners: Vec<_> = manager
            .settlers()
            .with_capability(&capability_presets::mining())
            .collect();
        assert_eq!(miners.len(), 2);

        let haulers: Vec<_> = manager
            .settlers()
            .with_capability(&capability_presets::hauling())
            .collect();
        assert_eq!(haulers.len(), 1);
    }

    #[test]
    fn test_task_registry_queries() {
        let mut manager = setup_manager();

        manager.create_task(task_def_presets::mine_rock(), Some(RegionId::new(1)));
        manager.create_task(task_def_presets::mine_rock(), Some(RegionId::new(1)));
        manager.create_task(task_def_presets::haul_item(), Some(RegionId::new(2)));

        let region1_tasks: Vec<_> = manager.tasks().in_region(RegionId::new(1)).collect();
        assert_eq!(region1_tasks.len(), 2);

        let mine_rock_id = task_def_presets::mine_rock();
        let mine_tasks: Vec<_> = manager.tasks().by_def(&mine_rock_id).collect();
        assert_eq!(mine_tasks.len(), 2);
    }

    #[test]
    fn test_event_filtering() {
        let mut manager = setup_manager();
        let miner = create_miner(&mut manager, "Miner");
        let task_id = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        for _ in 0..5 {
            manager.tick();
        }

        let task_events: Vec<_> = manager.events_for_task(task_id).collect();
        let settler_events: Vec<_> = manager.events_for_settler(miner).collect();

        assert!(!task_events.is_empty());
        assert!(!settler_events.is_empty());
    }

    #[test]
    fn test_reassignment_after_completion() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");

        manager.create_task(task_def_presets::mine_rock(), None);
        manager.create_task(task_def_presets::mine_rock(), None);

        let mut completions = 0;
        for _ in 0..200 {
            let result = manager.tick();
            completions += result.tasks_completed;
            if completions >= 2 {
                break;
            }
        }

        assert_eq!(completions, 2);
    }

    #[test]
    fn test_checksum_stability() {
        let snapshot = SettlerSnapshot {
            tick: 100,
            total_settlers: 5,
            idle_settlers: 2,
            working_settlers: 3,
            incapacitated_settlers: 0,
            total_tasks: 10,
            pending_tasks: 3,
            in_progress_tasks: 5,
            completed_tasks: 2,
            failed_tasks: 0,
            total_work_done: 5000,
            tasks_by_category: BTreeMap::new(),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: SettlerSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(snapshot, restored);
    }

    #[test]
    fn test_serde_coverage() {
        let config = SettlerManagerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: SettlerManagerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, restored);

        let summary = SettlerSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        let restored: SettlerSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary, restored);

        let fp = SettlerFingerprint(0x1234_5678);
        let json = serde_json::to_string(&fp).unwrap();
        let restored: SettlerFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(fp, restored);

        let projection = SettlerProjection::new(0, 100);
        let json = serde_json::to_string(&projection).unwrap();
        let restored: SettlerProjection = serde_json::from_str(&json).unwrap();
        assert_eq!(projection, restored);
    }

    #[test]
    fn test_priority_based_assignment() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");

        let low_priority = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();
        let high_priority = manager
            .create_task(task_def_presets::mine_rock(), None)
            .unwrap();

        if let Some(task) = manager.tasks_mut().get_mut(high_priority) {
            task.priority_override = Some(100);
        }

        manager.tick();

        let high_task = manager.tasks().get(high_priority).unwrap();
        let low_task = manager.tasks().get(low_priority).unwrap();

        assert!(high_task.status.is_active());
        assert!(low_task.status.is_available());
    }

    #[test]
    fn test_skill_level_affects_work_speed() {
        let mut manager = setup_manager();

        let novice_id = manager.create_settler("Novice");
        {
            let novice = manager.settlers_mut().get_mut(novice_id).unwrap();
            novice
                .skills
                .add_skill(Skill::new(capability_presets::mining()));
        }

        let expert_id = manager.create_settler("Expert");
        {
            let expert = manager.settlers_mut().get_mut(expert_id).unwrap();
            expert
                .skills
                .add_skill(Skill::new(capability_presets::mining()).with_level(SkillLevel::Expert));
        }

        let novice = manager.settlers().get(novice_id).unwrap();
        let expert = manager.settlers().get(expert_id).unwrap();

        let novice_speed = novice.effective_work_speed(&capability_presets::mining());
        let expert_speed = expert.effective_work_speed(&capability_presets::mining());

        assert!(expert_speed > novice_speed);
    }

    #[test]
    fn test_manager_serde_roundtrip() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_task(task_def_presets::mine_rock(), None);
        manager.tick();
        manager.tick();

        let json = serde_json::to_string(&manager).unwrap();
        let restored: SettlerManager = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.current_tick(), manager.current_tick());
        assert_eq!(restored.settlers().count(), manager.settlers().count());
        assert_eq!(restored.tasks().count(), manager.tasks().count());
        assert_eq!(restored.fingerprint(), manager.fingerprint());
    }
}

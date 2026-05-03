//! Colony job system with definitions, assignments, and lifecycle management.

use super::ids::{JobDefId, JobId, SkillId, WorkerId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Category of job for organization and prioritization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum JobCategory {
    Construction,
    Production,
    Hauling,
    Mining,
    Farming,
    Research,
    Medical,
    Maintenance,
    Emergency,
    Social,
}

impl JobCategory {
    #[must_use]
    pub fn base_priority(self) -> i32 {
        match self {
            Self::Emergency => 100,
            Self::Medical => 80,
            Self::Maintenance => 60,
            Self::Production => 50,
            Self::Construction => 45,
            Self::Mining | Self::Farming => 40,
            Self::Hauling => 30,
            Self::Research => 25,
            Self::Social => 20,
        }
    }
}

/// Definition of a job type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobDef {
    pub id: JobDefId,
    pub name: String,
    pub category: JobCategory,
    pub base_work_amount: u32,
    pub required_skills: Vec<SkillId>,
    pub min_workers: u32,
    pub max_workers: u32,
    pub can_interrupt: bool,
    pub prerequisites: Vec<JobDefId>,
    pub produces_resources: bool,
    pub consumes_resources: bool,
}

impl JobDef {
    #[must_use]
    pub fn new(id: impl Into<JobDefId>, name: impl Into<String>, category: JobCategory) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category,
            base_work_amount: 100,
            required_skills: Vec::new(),
            min_workers: 1,
            max_workers: 1,
            can_interrupt: true,
            prerequisites: Vec::new(),
            produces_resources: false,
            consumes_resources: false,
        }
    }

    #[must_use]
    pub fn with_work_amount(mut self, amount: u32) -> Self {
        self.base_work_amount = amount;
        self
    }

    #[must_use]
    pub fn with_required_skill(mut self, skill: impl Into<SkillId>) -> Self {
        self.required_skills.push(skill.into());
        self
    }

    #[must_use]
    pub fn with_worker_range(mut self, min: u32, max: u32) -> Self {
        self.min_workers = min;
        self.max_workers = max;
        self
    }

    #[must_use]
    pub fn with_prerequisite(mut self, prereq: impl Into<JobDefId>) -> Self {
        self.prerequisites.push(prereq.into());
        self
    }

    #[must_use]
    pub fn non_interruptible(mut self) -> Self {
        self.can_interrupt = false;
        self
    }
}

/// Registry of job definitions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JobDefRegistry {
    defs: BTreeMap<JobDefId, JobDef>,
}

impl JobDefRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: JobDef) {
        self.defs.insert(def.id.clone(), def);
    }

    #[must_use]
    pub fn get(&self, id: &JobDefId) -> Option<&JobDef> {
        self.defs.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &JobDef> {
        self.defs.values()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.defs.len()
    }

    pub fn by_category(&self, category: JobCategory) -> impl Iterator<Item = &JobDef> {
        self.defs.values().filter(move |d| d.category == category)
    }
}

/// Current status of a job instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Blocked,
    Claimed,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    #[must_use]
    pub fn is_available(self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Claimed | Self::InProgress | Self::Paused)
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    #[must_use]
    pub fn can_be_claimed(self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// Priority level for a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobPriority(pub i32);

impl JobPriority {
    pub const CRITICAL: Self = Self(100);
    pub const HIGH: Self = Self(75);
    pub const NORMAL: Self = Self(50);
    pub const LOW: Self = Self(25);
    pub const BACKGROUND: Self = Self(10);

    #[must_use]
    pub fn new(value: i32) -> Self {
        Self(value.clamp(0, 100))
    }

    #[must_use]
    pub fn raw(self) -> i32 {
        self.0
    }
}

impl Default for JobPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Reason for job failure.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobFailureReason {
    NoWorkers,
    MissingResources,
    BlockedDependency,
    WorkerIncapacitated,
    Timeout,
    Cancelled,
    ExternalFailure(String),
}

impl JobFailureReason {
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::NoWorkers | Self::MissingResources | Self::BlockedDependency
        )
    }
}

/// Instance of a job being executed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub def_id: JobDefId,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub work_required: u32,
    pub work_done: u32,
    pub assigned_workers: BTreeSet<WorkerId>,
    pub blocked_by: BTreeSet<JobId>,
    pub created_tick: u64,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    pub failure_reason: Option<JobFailureReason>,
    pub priority_override: Option<i32>,
    pub pause_count: u32,
    pub interrupt_count: u32,
}

impl Job {
    #[must_use]
    pub fn new(id: JobId, def_id: JobDefId, work_required: u32, created_tick: u64) -> Self {
        Self {
            id,
            def_id,
            status: JobStatus::Pending,
            priority: JobPriority::default(),
            work_required,
            work_done: 0,
            assigned_workers: BTreeSet::new(),
            blocked_by: BTreeSet::new(),
            created_tick,
            started_tick: None,
            completed_tick: None,
            failure_reason: None,
            priority_override: None,
            pause_count: 0,
            interrupt_count: 0,
        }
    }

    #[must_use]
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.work_required == 0 {
            return 1.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "work values bounded by game limits"
        )]
        {
            (self.work_done as f32 / self.work_required as f32).clamp(0.0, 1.0)
        }
    }

    #[must_use]
    pub fn remaining_work(&self) -> u32 {
        self.work_required.saturating_sub(self.work_done)
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty() || self.status == JobStatus::Blocked
    }

    #[must_use]
    pub fn has_worker(&self, worker: WorkerId) -> bool {
        self.assigned_workers.contains(&worker)
    }

    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.assigned_workers.len()
    }

    #[must_use]
    pub fn effective_priority(&self) -> i32 {
        self.priority_override.unwrap_or(self.priority.raw())
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.created_tick)
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> u64 {
        let start = self.started_tick.unwrap_or(current_tick);
        let end = self.completed_tick.unwrap_or(current_tick);
        end.saturating_sub(start)
    }

    pub fn assign_worker(&mut self, worker: WorkerId) {
        self.assigned_workers.insert(worker);
        if self.status == JobStatus::Pending {
            self.status = JobStatus::Claimed;
        }
    }

    pub fn unassign_worker(&mut self, worker: WorkerId) {
        self.assigned_workers.remove(&worker);
    }

    pub fn add_blocker(&mut self, job: JobId) {
        self.blocked_by.insert(job);
        if self.status == JobStatus::Pending {
            self.status = JobStatus::Blocked;
        }
    }

    pub fn remove_blocker(&mut self, job: JobId) {
        self.blocked_by.remove(&job);
        if self.blocked_by.is_empty() && self.status == JobStatus::Blocked {
            self.status = JobStatus::Pending;
        }
    }

    pub fn start(&mut self, tick: u64) {
        self.status = JobStatus::InProgress;
        if self.started_tick.is_none() {
            self.started_tick = Some(tick);
        }
    }

    pub fn add_work(&mut self, amount: u32) -> bool {
        self.work_done = self.work_done.saturating_add(amount);
        self.work_done >= self.work_required
    }

    pub fn pause(&mut self) {
        if self.status == JobStatus::InProgress {
            self.status = JobStatus::Paused;
            self.pause_count += 1;
        }
    }

    pub fn resume(&mut self) {
        if self.status == JobStatus::Paused {
            self.status = JobStatus::InProgress;
        }
    }

    pub fn interrupt(&mut self) {
        if self.status == JobStatus::InProgress {
            self.status = JobStatus::Pending;
            self.interrupt_count += 1;
            self.assigned_workers.clear();
        }
    }

    pub fn release(&mut self) {
        if self.status == JobStatus::Claimed {
            self.status = JobStatus::Pending;
            self.assigned_workers.clear();
        }
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = JobStatus::Completed;
        self.completed_tick = Some(tick);
    }

    pub fn fail(&mut self, tick: u64, reason: JobFailureReason) {
        self.status = JobStatus::Failed;
        self.completed_tick = Some(tick);
        self.failure_reason = Some(reason);
    }

    pub fn cancel(&mut self, tick: u64) {
        self.status = JobStatus::Cancelled;
        self.completed_tick = Some(tick);
        self.failure_reason = Some(JobFailureReason::Cancelled);
    }
}

/// Worker capability for job assignment matching.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerCapability {
    pub skill: SkillId,
    pub level: u32,
    pub experience: f32,
}

impl WorkerCapability {
    #[must_use]
    pub fn new(skill: impl Into<SkillId>) -> Self {
        Self {
            skill: skill.into(),
            level: 1,
            experience: 0.0,
        }
    }

    #[must_use]
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    #[must_use]
    pub fn effectiveness(&self) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "level is small")]
        {
            0.5 + (self.level as f32 * 0.1).min(1.5)
        }
    }

    pub fn add_experience(&mut self, amount: f32) -> bool {
        self.experience += amount;
        let threshold = self.level_up_threshold();
        if self.experience >= threshold {
            self.experience -= threshold;
            self.level += 1;
            true
        } else {
            false
        }
    }

    #[must_use]
    #[expect(clippy::cast_possible_wrap, reason = "level is small")]
    fn level_up_threshold(&self) -> f32 {
        100.0 * (1.5_f32).powi(self.level as i32)
    }
}

/// Set of worker capabilities.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkerSkillSet {
    skills: BTreeMap<SkillId, WorkerCapability>,
}

impl WorkerSkillSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, capability: WorkerCapability) {
        self.skills.insert(capability.skill.clone(), capability);
    }

    #[must_use]
    pub fn get(&self, skill: &SkillId) -> Option<&WorkerCapability> {
        self.skills.get(skill)
    }

    pub fn get_mut(&mut self, skill: &SkillId) -> Option<&mut WorkerCapability> {
        self.skills.get_mut(skill)
    }

    #[must_use]
    pub fn has(&self, skill: &SkillId) -> bool {
        self.skills.contains_key(skill)
    }

    #[must_use]
    pub fn has_all(&self, skills: &[SkillId]) -> bool {
        skills.iter().all(|s| self.has(s))
    }

    pub fn iter(&self) -> impl Iterator<Item = &WorkerCapability> {
        self.skills.values()
    }

    #[must_use]
    pub fn total_effectiveness(&self, required: &[SkillId]) -> f32 {
        if required.is_empty() {
            return 1.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "skill count is small and bounded"
        )]
        {
            let total: f32 = required
                .iter()
                .map(|s| self.get(s).map_or(0.5, WorkerCapability::effectiveness))
                .sum();
            total / required.len() as f32
        }
    }
}

/// Worker state for job assignment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Worker {
    pub id: WorkerId,
    pub name: String,
    pub skills: WorkerSkillSet,
    pub current_job: Option<JobId>,
    pub is_available: bool,
    pub is_incapacitated: bool,
    pub total_jobs_completed: u64,
    pub total_work_performed: u64,
    pub fatigue: f32,
}

impl Worker {
    #[must_use]
    pub fn new(id: WorkerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            skills: WorkerSkillSet::new(),
            current_job: None,
            is_available: true,
            is_incapacitated: false,
            total_jobs_completed: 0,
            total_work_performed: 0,
            fatigue: 0.0,
        }
    }

    #[must_use]
    pub fn can_work(&self) -> bool {
        self.is_available && !self.is_incapacitated && self.current_job.is_none()
    }

    #[must_use]
    pub fn can_perform(&self, required_skills: &[SkillId]) -> bool {
        self.skills.has_all(required_skills)
    }

    #[must_use]
    pub fn work_speed(&self, required_skills: &[SkillId]) -> f32 {
        let effectiveness = self.skills.total_effectiveness(required_skills);
        let fatigue_penalty = 1.0 - (self.fatigue * 0.5);
        effectiveness * fatigue_penalty.max(0.2)
    }

    pub fn assign_job(&mut self, job: JobId) {
        self.current_job = Some(job);
    }

    pub fn clear_job(&mut self) {
        self.current_job = None;
    }

    pub fn record_work(&mut self, amount: u32) {
        self.total_work_performed += u64::from(amount);
        self.fatigue = (self.fatigue + 0.001).min(1.0);
    }

    pub fn record_completion(&mut self) {
        self.total_jobs_completed += 1;
    }

    pub fn incapacitate(&mut self) {
        self.is_incapacitated = true;
        self.current_job = None;
    }

    pub fn recover(&mut self) {
        self.is_incapacitated = false;
    }

    pub fn rest(&mut self, amount: f32) {
        self.fatigue = (self.fatigue - amount).max(0.0);
    }
}

/// Registry of workers.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkerRegistry {
    workers: BTreeMap<WorkerId, Worker>,
    next_id: u64,
}

impl WorkerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, name: impl Into<String>) -> WorkerId {
        let id = WorkerId::new(self.next_id);
        self.next_id += 1;
        let worker = Worker::new(id, name);
        self.workers.insert(id, worker);
        id
    }

    pub fn register(&mut self, worker: Worker) {
        let id = worker.id;
        self.workers.insert(id, worker);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: WorkerId) -> Option<Worker> {
        self.workers.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: WorkerId) -> Option<&Worker> {
        self.workers.get(&id)
    }

    pub fn get_mut(&mut self, id: WorkerId) -> Option<&mut Worker> {
        self.workers.get_mut(&id)
    }

    #[must_use]
    pub fn contains(&self, id: WorkerId) -> bool {
        self.workers.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Worker> {
        self.workers.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Worker> {
        self.workers.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.workers.len()
    }

    pub fn available(&self) -> impl Iterator<Item = &Worker> {
        self.workers.values().filter(|w| w.can_work())
    }

    pub fn with_skill(&self, skill: &SkillId) -> impl Iterator<Item = &Worker> {
        self.workers.values().filter(|w| w.skills.has(skill))
    }
}

/// Registry of job instances.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JobRegistry {
    jobs: BTreeMap<JobId, Job>,
    next_id: u64,
}

impl JobRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, def_id: JobDefId, work_required: u32, created_tick: u64) -> JobId {
        let id = JobId::new(self.next_id);
        self.next_id += 1;
        let job = Job::new(id, def_id, work_required, created_tick);
        self.jobs.insert(id, job);
        id
    }

    pub fn register(&mut self, job: Job) {
        let id = job.id;
        self.jobs.insert(id, job);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: JobId) -> Option<Job> {
        self.jobs.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    pub fn get_mut(&mut self, id: JobId) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    #[must_use]
    pub fn contains(&self, id: JobId) -> bool {
        self.jobs.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Job> {
        self.jobs.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.jobs.len()
    }

    pub fn pending(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values().filter(|j| j.status.is_available())
    }

    pub fn active(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values().filter(|j| j.status.is_active())
    }

    pub fn by_status(&self, status: JobStatus) -> impl Iterator<Item = &Job> {
        self.jobs.values().filter(move |j| j.status == status)
    }

    pub fn by_def<'a>(&'a self, def_id: &'a JobDefId) -> impl Iterator<Item = &'a Job> + 'a {
        self.jobs.values().filter(move |j| &j.def_id == def_id)
    }

    pub fn blocked(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values().filter(|j| j.is_blocked())
    }

    pub fn priority_sorted(&self) -> Vec<&Job> {
        let mut jobs: Vec<_> = self.pending().collect();
        jobs.sort_by(|a, b| {
            b.effective_priority()
                .cmp(&a.effective_priority())
                .then_with(|| a.created_tick.cmp(&b.created_tick))
                .then_with(|| a.id.cmp(&b.id))
        });
        jobs
    }
}

/// Event types for job lifecycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobEvent {
    pub tick: u64,
    pub kind: JobEventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum JobEventKind {
    JobCreated {
        job: JobId,
        def_id: JobDefId,
    },
    JobClaimed {
        job: JobId,
        worker: WorkerId,
    },
    JobStarted {
        job: JobId,
        worker: WorkerId,
    },
    JobProgress {
        job: JobId,
        work_done: u32,
        progress: f32,
    },
    JobPaused {
        job: JobId,
    },
    JobResumed {
        job: JobId,
    },
    JobInterrupted {
        job: JobId,
    },
    JobCompleted {
        job: JobId,
        duration: u64,
    },
    JobFailed {
        job: JobId,
        reason: JobFailureReason,
    },
    JobCancelled {
        job: JobId,
        reason: String,
    },
    JobUnblocked {
        job: JobId,
    },
    WorkerAssigned {
        worker: WorkerId,
        job: JobId,
    },
    WorkerUnassigned {
        worker: WorkerId,
        job: JobId,
    },
    WorkerIncapacitated {
        worker: WorkerId,
        dropped_job: Option<JobId>,
    },
    WorkerRecovered {
        worker: WorkerId,
    },
    SkillLevelUp {
        worker: WorkerId,
        skill: SkillId,
        new_level: u32,
    },
}

impl JobEvent {
    #[must_use]
    pub fn new(tick: u64, kind: JobEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn job_created(tick: u64, job: JobId, def_id: JobDefId) -> Self {
        Self::new(tick, JobEventKind::JobCreated { job, def_id })
    }

    #[must_use]
    pub fn job_claimed(tick: u64, job: JobId, worker: WorkerId) -> Self {
        Self::new(tick, JobEventKind::JobClaimed { job, worker })
    }

    #[must_use]
    pub fn job_started(tick: u64, job: JobId, worker: WorkerId) -> Self {
        Self::new(tick, JobEventKind::JobStarted { job, worker })
    }

    #[must_use]
    pub fn job_progress(tick: u64, job: JobId, work_done: u32, progress: f32) -> Self {
        Self::new(
            tick,
            JobEventKind::JobProgress {
                job,
                work_done,
                progress,
            },
        )
    }

    #[must_use]
    pub fn job_completed(tick: u64, job: JobId, duration: u64) -> Self {
        Self::new(tick, JobEventKind::JobCompleted { job, duration })
    }

    #[must_use]
    pub fn job_failed(tick: u64, job: JobId, reason: JobFailureReason) -> Self {
        Self::new(tick, JobEventKind::JobFailed { job, reason })
    }

    #[must_use]
    pub fn job_cancelled(tick: u64, job: JobId, reason: impl Into<String>) -> Self {
        Self::new(
            tick,
            JobEventKind::JobCancelled {
                job,
                reason: reason.into(),
            },
        )
    }

    #[must_use]
    pub fn involves_job(&self, job: JobId) -> bool {
        match &self.kind {
            JobEventKind::JobCreated { job: j, .. }
            | JobEventKind::JobClaimed { job: j, .. }
            | JobEventKind::JobStarted { job: j, .. }
            | JobEventKind::JobProgress { job: j, .. }
            | JobEventKind::JobPaused { job: j }
            | JobEventKind::JobResumed { job: j }
            | JobEventKind::JobInterrupted { job: j }
            | JobEventKind::JobCompleted { job: j, .. }
            | JobEventKind::JobFailed { job: j, .. }
            | JobEventKind::JobCancelled { job: j, .. }
            | JobEventKind::JobUnblocked { job: j }
            | JobEventKind::WorkerAssigned { job: j, .. }
            | JobEventKind::WorkerUnassigned { job: j, .. } => *j == job,
            JobEventKind::WorkerIncapacitated { dropped_job, .. } => *dropped_job == Some(job),
            JobEventKind::WorkerRecovered { .. } | JobEventKind::SkillLevelUp { .. } => false,
        }
    }

    #[must_use]
    pub fn involves_worker(&self, worker: WorkerId) -> bool {
        match &self.kind {
            JobEventKind::JobClaimed { worker: w, .. }
            | JobEventKind::JobStarted { worker: w, .. }
            | JobEventKind::WorkerAssigned { worker: w, .. }
            | JobEventKind::WorkerUnassigned { worker: w, .. }
            | JobEventKind::WorkerIncapacitated { worker: w, .. }
            | JobEventKind::WorkerRecovered { worker: w }
            | JobEventKind::SkillLevelUp { worker: w, .. } => *w == worker,
            _ => false,
        }
    }
}

pub mod presets {
    use super::{JobCategory, JobDef, JobDefId, SkillId};

    #[must_use]
    pub fn mining() -> SkillId {
        SkillId::new("mining")
    }

    #[must_use]
    pub fn hauling() -> SkillId {
        SkillId::new("hauling")
    }

    #[must_use]
    pub fn construction() -> SkillId {
        SkillId::new("construction")
    }

    #[must_use]
    pub fn farming() -> SkillId {
        SkillId::new("farming")
    }

    #[must_use]
    pub fn medical() -> SkillId {
        SkillId::new("medical")
    }

    #[must_use]
    pub fn research() -> SkillId {
        SkillId::new("research")
    }

    #[must_use]
    pub fn mine_ore() -> JobDefId {
        JobDefId::new("mine_ore")
    }

    #[must_use]
    pub fn haul_item() -> JobDefId {
        JobDefId::new("haul_item")
    }

    #[must_use]
    pub fn build_structure() -> JobDefId {
        JobDefId::new("build_structure")
    }

    #[must_use]
    pub fn standard_job_defs() -> Vec<JobDef> {
        vec![
            JobDef::new(mine_ore(), "Mine Ore", JobCategory::Mining)
                .with_work_amount(100)
                .with_required_skill(mining()),
            JobDef::new(haul_item(), "Haul Item", JobCategory::Hauling)
                .with_work_amount(50)
                .with_required_skill(hauling()),
            JobDef::new(
                build_structure(),
                "Build Structure",
                JobCategory::Construction,
            )
            .with_work_amount(200)
            .with_required_skill(construction())
            .with_worker_range(1, 3),
            JobDef::new("plant_crops", "Plant Crops", JobCategory::Farming)
                .with_work_amount(80)
                .with_required_skill(farming()),
            JobDef::new("treat_patient", "Treat Patient", JobCategory::Medical)
                .with_work_amount(150)
                .with_required_skill(medical())
                .non_interruptible(),
            JobDef::new(
                "conduct_research",
                "Conduct Research",
                JobCategory::Research,
            )
            .with_work_amount(500)
            .with_required_skill(research()),
            JobDef::new(
                "repair_equipment",
                "Repair Equipment",
                JobCategory::Maintenance,
            )
            .with_work_amount(120)
            .with_required_skill(construction()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_registry() -> JobDefRegistry {
        let mut registry = JobDefRegistry::new();
        for def in presets::standard_job_defs() {
            registry.register(def);
        }
        registry
    }

    #[test]
    fn test_job_def_creation() {
        let def = JobDef::new("test", "Test Job", JobCategory::Production)
            .with_work_amount(150)
            .with_required_skill("testing")
            .with_worker_range(1, 2);

        assert_eq!(def.id.as_str(), "test");
        assert_eq!(def.base_work_amount, 150);
        assert_eq!(def.min_workers, 1);
        assert_eq!(def.max_workers, 2);
    }

    #[test]
    fn test_job_lifecycle() {
        let mut job = Job::new(JobId::new(1), JobDefId::new("test"), 100, 0);

        assert_eq!(job.status, JobStatus::Pending);
        assert!(job.status.is_available());

        job.assign_worker(WorkerId::new(1));
        assert_eq!(job.status, JobStatus::Claimed);
        assert!(job.has_worker(WorkerId::new(1)));

        job.start(10);
        assert_eq!(job.status, JobStatus::InProgress);
        assert_eq!(job.started_tick, Some(10));

        assert!(!job.add_work(50));
        assert!((job.progress() - 0.5).abs() < 0.001);

        assert!(job.add_work(50));
        job.complete(20);
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.completed_tick, Some(20));
    }

    #[test]
    fn test_job_blocking() {
        let mut job = Job::new(JobId::new(2), JobDefId::new("test"), 100, 0);

        job.add_blocker(JobId::new(1));
        assert!(job.is_blocked());
        assert_eq!(job.status, JobStatus::Blocked);

        job.remove_blocker(JobId::new(1));
        assert!(!job.is_blocked());
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_pause_resume() {
        let mut job = Job::new(JobId::new(1), JobDefId::new("test"), 100, 0);
        job.assign_worker(WorkerId::new(1));
        job.start(0);

        job.pause();
        assert_eq!(job.status, JobStatus::Paused);
        assert_eq!(job.pause_count, 1);

        job.resume();
        assert_eq!(job.status, JobStatus::InProgress);
    }

    #[test]
    fn test_job_interrupt() {
        let mut job = Job::new(JobId::new(1), JobDefId::new("test"), 100, 0);
        job.assign_worker(WorkerId::new(1));
        job.start(0);

        job.interrupt();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.interrupt_count, 1);
        assert!(job.assigned_workers.is_empty());
    }

    #[test]
    fn test_job_failure() {
        let mut job = Job::new(JobId::new(1), JobDefId::new("test"), 100, 0);

        job.fail(10, JobFailureReason::NoWorkers);
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.failure_reason, Some(JobFailureReason::NoWorkers));
    }

    #[test]
    fn test_worker_capability() {
        let mut cap = WorkerCapability::new("mining").with_level(3);

        assert_eq!(cap.level, 3);
        assert!(cap.effectiveness() > 0.5);

        let leveled = cap.add_experience(1000.0);
        assert!(leveled || cap.experience > 0.0);
    }

    #[test]
    fn test_worker_skill_set() {
        let mut skills = WorkerSkillSet::new();
        skills.add(WorkerCapability::new("mining").with_level(2));
        skills.add(WorkerCapability::new("hauling").with_level(1));

        assert!(skills.has(&SkillId::new("mining")));
        assert!(skills.has_all(&[SkillId::new("mining"), SkillId::new("hauling")]));
        assert!(!skills.has(&SkillId::new("research")));
    }

    #[test]
    fn test_worker_basics() {
        let mut worker = Worker::new(WorkerId::new(1), "Test Worker");

        assert!(worker.can_work());
        assert!(worker.can_perform(&[]));

        worker.assign_job(JobId::new(1));
        assert!(!worker.can_work());
        assert_eq!(worker.current_job, Some(JobId::new(1)));

        worker.clear_job();
        assert!(worker.can_work());
    }

    #[test]
    fn test_worker_incapacitation() {
        let mut worker = Worker::new(WorkerId::new(1), "Test Worker");
        worker.assign_job(JobId::new(1));

        worker.incapacitate();
        assert!(worker.is_incapacitated);
        assert!(!worker.can_work());
        assert!(worker.current_job.is_none());

        worker.recover();
        assert!(!worker.is_incapacitated);
    }

    #[test]
    fn test_worker_fatigue() {
        let mut worker = Worker::new(WorkerId::new(1), "Test Worker");

        let initial_speed = worker.work_speed(&[]);
        worker.fatigue = 0.5;
        let tired_speed = worker.work_speed(&[]);

        assert!(tired_speed < initial_speed);

        worker.rest(0.3);
        assert!((worker.fatigue - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_worker_registry() {
        let mut registry = WorkerRegistry::new();

        let id1 = registry.create("Worker 1");
        let id2 = registry.create("Worker 2");

        assert_eq!(registry.count(), 2);
        assert!(registry.contains(id1));
        assert!(registry.contains(id2));

        let worker = registry.get(id1).unwrap();
        assert_eq!(worker.name, "Worker 1");
    }

    #[test]
    fn test_job_registry() {
        let mut registry = JobRegistry::new();

        let id1 = registry.create(JobDefId::new("test1"), 100, 0);
        let _id2 = registry.create(JobDefId::new("test2"), 200, 0);

        assert_eq!(registry.count(), 2);
        assert!(registry.contains(id1));

        let job = registry.get(id1).unwrap();
        assert_eq!(job.work_required, 100);
    }

    #[test]
    fn test_job_registry_priority_sorted() {
        let mut registry = JobRegistry::new();

        let low = registry.create(JobDefId::new("low"), 100, 0);
        let high = registry.create(JobDefId::new("high"), 100, 1);
        let medium = registry.create(JobDefId::new("medium"), 100, 2);

        registry.get_mut(low).unwrap().priority = JobPriority::LOW;
        registry.get_mut(high).unwrap().priority = JobPriority::HIGH;
        registry.get_mut(medium).unwrap().priority = JobPriority::NORMAL;

        let sorted = registry.priority_sorted();
        assert_eq!(sorted[0].id, high);
        assert_eq!(sorted[1].id, medium);
        assert_eq!(sorted[2].id, low);
    }

    #[test]
    fn test_job_event_creation() {
        let event = JobEvent::job_created(100, JobId::new(1), JobDefId::new("test"));

        assert_eq!(event.tick, 100);
        assert!(event.involves_job(JobId::new(1)));
        assert!(!event.involves_job(JobId::new(2)));
    }

    #[test]
    fn test_job_event_worker_involvement() {
        let event = JobEvent::job_claimed(100, JobId::new(1), WorkerId::new(5));

        assert!(event.involves_job(JobId::new(1)));
        assert!(event.involves_worker(WorkerId::new(5)));
        assert!(!event.involves_worker(WorkerId::new(6)));
    }

    #[test]
    fn test_job_category_priority() {
        assert!(JobCategory::Emergency.base_priority() > JobCategory::Production.base_priority());
        assert!(JobCategory::Medical.base_priority() > JobCategory::Mining.base_priority());
        assert!(JobCategory::Hauling.base_priority() > JobCategory::Social.base_priority());
    }

    #[test]
    fn test_job_failure_reason_recoverable() {
        assert!(JobFailureReason::NoWorkers.is_recoverable());
        assert!(JobFailureReason::MissingResources.is_recoverable());
        assert!(!JobFailureReason::Timeout.is_recoverable());
        assert!(!JobFailureReason::Cancelled.is_recoverable());
    }

    #[test]
    fn test_serde_job() {
        let job = Job::new(JobId::new(42), JobDefId::new("test"), 100, 50)
            .with_priority(JobPriority::HIGH);

        let json = serde_json::to_string(&job).unwrap();
        let restored: Job = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, job.id);
        assert_eq!(restored.priority, job.priority);
    }

    #[test]
    fn test_serde_worker() {
        let mut worker = Worker::new(WorkerId::new(1), "Test");
        worker
            .skills
            .add(WorkerCapability::new("mining").with_level(3));

        let json = serde_json::to_string(&worker).unwrap();
        let restored: Worker = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, "Test");
        assert!(restored.skills.has(&SkillId::new("mining")));
    }

    #[test]
    fn test_bincode_job() {
        let job = Job::new(JobId::new(42), JobDefId::new("test"), 100, 50);

        let bytes = bincode::serialize(&job).unwrap();
        let restored: Job = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id, job.id);
        assert_eq!(restored.work_required, job.work_required);
    }

    #[test]
    fn test_bincode_worker() {
        let mut worker = Worker::new(WorkerId::new(1), "Test");
        worker.skills.add(WorkerCapability::new("mining"));

        let bytes = bincode::serialize(&worker).unwrap();
        let restored: Worker = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id, worker.id);
        assert_eq!(restored.name, worker.name);
    }

    #[test]
    fn test_bincode_event() {
        let event = JobEvent::job_completed(100, JobId::new(1), 50);

        let bytes = bincode::serialize(&event).unwrap();
        let restored: JobEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 100);
    }

    #[test]
    fn test_presets() {
        let registry = setup_registry();

        assert!(registry.get(&presets::mine_ore()).is_some());
        assert!(registry.get(&presets::haul_item()).is_some());
        assert!(registry.get(&presets::build_structure()).is_some());

        let mining_jobs: Vec<_> = registry.by_category(JobCategory::Mining).collect();
        assert!(!mining_jobs.is_empty());
    }
}

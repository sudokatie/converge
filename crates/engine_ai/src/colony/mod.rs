//! Colony simulation system for jobs, logistics, shelter, and failure cascades.
//!
//! Provides deterministic, data-driven simulation of colony operations:
//!
//! - Job system with definitions, assignments, priorities, dependencies,
//!   worker capability/skill fit, lifecycle states, and progress tracking
//! - Logistics with stockpiles, resource reservations, route capacities/costs,
//!   supply/demand balancing, and transfer planning
//! - Shelter value ratings based on capacity, safety, comfort, environmental
//!   factors, crowding, access, and hazard exposure
//! - Failure cascades with configurable triggers, severity escalation,
//!   bounded event logs, and mitigation helpers
//! - Snapshots, summaries, and projections for state inspection
//! - Stable fingerprints for determinism verification

mod failure;
pub mod ids;
mod job;
mod logistics;
mod shelter;

pub use failure::{
    CascadeConfig, Failure, FailureEvent, FailureEventKind, FailureEventLog, FailureFingerprint,
    FailureProjection, FailureRegistry, FailureSeverity, FailureStatus, FailureSummary,
    FailureTrigger, MitigationAction, suggest_mitigations,
};
pub use ids::{
    ColonyId, FailureId, JobDefId, JobId, ResourceId, RouteId, ShelterId, SkillId, StorageNodeId,
    TransferId, WorkerId,
};
pub use job::{
    Job, JobCategory, JobDef, JobDefRegistry, JobEvent, JobEventKind, JobFailureReason,
    JobPriority, JobRegistry, JobStatus, Worker, WorkerCapability, WorkerRegistry, WorkerSkillSet,
    presets as job_presets,
};
pub use logistics::{
    LogisticsEvent, LogisticsEventKind, LogisticsFingerprint, LogisticsProjection,
    LogisticsSummary, ResourceAmount, ResourceBalance, Route, RouteRegistry, StorageNode,
    StorageRegistry, Transfer, TransferRegistry, TransferStatus,
};
pub use shelter::{
    Rating, RatingCategory, Shelter, ShelterCoverage, ShelterFingerprint, ShelterRatings,
    ShelterRecommendation, ShelterRegistry, ShelterWeights, generate_recommendations,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuration for the colony manager.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColonyConfig {
    pub cascade_config: CascadeConfig,
    pub auto_assign_jobs: bool,
    pub auto_plan_transfers: bool,
    pub track_event_history: bool,
    pub max_event_history: usize,
    pub snapshot_interval: u64,
}

impl Default for ColonyConfig {
    fn default() -> Self {
        Self {
            cascade_config: CascadeConfig::default(),
            auto_assign_jobs: true,
            auto_plan_transfers: true,
            track_event_history: true,
            max_event_history: 1000,
            snapshot_interval: 100,
        }
    }
}

impl ColonyConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_cascade_config(mut self, config: CascadeConfig) -> Self {
        self.cascade_config = config;
        self
    }

    #[must_use]
    pub fn with_max_event_history(mut self, max: usize) -> Self {
        self.max_event_history = max;
        self
    }
}

/// Result of a colony manager tick.
#[derive(Clone, Debug, Default)]
pub struct ColonyTickResult {
    pub job_events: Vec<JobEvent>,
    pub logistics_events: Vec<LogisticsEvent>,
    pub failure_events: Vec<FailureEvent>,
    pub jobs_assigned: u32,
    pub jobs_completed: u32,
    pub jobs_failed: u32,
    pub transfers_completed: u32,
    pub failures_triggered: u32,
    pub failures_resolved: u32,
    pub failures_cascaded: u32,
}

impl ColonyTickResult {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: Self) {
        self.job_events.extend(other.job_events);
        self.logistics_events.extend(other.logistics_events);
        self.failure_events.extend(other.failure_events);
        self.jobs_assigned += other.jobs_assigned;
        self.jobs_completed += other.jobs_completed;
        self.jobs_failed += other.jobs_failed;
        self.transfers_completed += other.transfers_completed;
        self.failures_triggered += other.failures_triggered;
        self.failures_resolved += other.failures_resolved;
        self.failures_cascaded += other.failures_cascaded;
    }
}

/// Snapshot of colony state at a point in time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColonyStateSnapshot {
    pub tick: u64,
    pub population: u32,
    pub total_workers: u32,
    pub available_workers: u32,
    pub total_jobs: u32,
    pub active_jobs: u32,
    pub pending_jobs: u32,
    pub total_storage_nodes: u32,
    pub total_storage_capacity: u32,
    pub total_stored: u32,
    pub active_transfers: u32,
    pub total_shelters: u32,
    pub shelter_capacity: u32,
    pub shelter_occupancy: u32,
    pub shelter_coverage: f32,
    pub average_shelter_rating: f32,
    pub active_failures: u32,
    pub total_failures: u32,
    pub failure_severity_counts: BTreeMap<String, u32>,
    pub resource_balances: BTreeMap<String, i32>,
}

impl ColonyStateSnapshot {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded")]
    pub fn worker_utilization(&self) -> f32 {
        if self.total_workers == 0 {
            return 0.0;
        }
        1.0 - (self.available_workers as f32 / self.total_workers as f32)
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts bounded")]
    pub fn storage_utilization(&self) -> f32 {
        if self.total_storage_capacity == 0 {
            return 0.0;
        }
        self.total_stored as f32 / self.total_storage_capacity as f32
    }

    #[must_use]
    pub fn has_critical_issues(&self) -> bool {
        self.active_failures > 0 || self.shelter_coverage < 0.8
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.population.to_le_bytes());
        hasher.update(&self.total_workers.to_le_bytes());
        hasher.update(&self.total_jobs.to_le_bytes());
        hasher.update(&self.total_stored.to_le_bytes());
        hasher.update(&self.active_failures.to_le_bytes());
        hasher.update(&self.shelter_coverage.to_le_bytes());
        hasher.finalize()
    }
}

/// Summary of colony state for cheap transmission.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColonyStateSummary {
    pub tick: u64,
    pub population: u32,
    pub worker_utilization: f32,
    pub storage_utilization: f32,
    pub shelter_coverage: f32,
    pub active_failures: u32,
    pub stability_score: f32,
}

impl From<&ColonyStateSnapshot> for ColonyStateSummary {
    fn from(snapshot: &ColonyStateSnapshot) -> Self {
        let stability = if snapshot.active_failures == 0 {
            1.0
        } else {
            #[expect(clippy::cast_precision_loss, reason = "bounded")]
            {
                (1.0 - (snapshot.active_failures as f32 * 0.1)).max(0.0)
            }
        };

        Self {
            tick: snapshot.tick,
            population: snapshot.population,
            worker_utilization: snapshot.worker_utilization(),
            storage_utilization: snapshot.storage_utilization(),
            shelter_coverage: snapshot.shelter_coverage,
            active_failures: snapshot.active_failures,
            stability_score: stability,
        }
    }
}

/// Projection of future colony state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColonyProjection {
    pub base_tick: u64,
    pub projected_tick: u64,
    pub estimated_job_completions: u32,
    pub estimated_shortages: Vec<(String, u32)>,
    pub estimated_failures: u32,
    pub risk_score: f32,
    pub confidence: f32,
}

impl ColonyProjection {
    #[must_use]
    pub fn new(base_tick: u64, projected_tick: u64) -> Self {
        Self {
            base_tick,
            projected_tick,
            estimated_job_completions: 0,
            estimated_shortages: Vec::new(),
            estimated_failures: 0,
            risk_score: 0.0,
            confidence: 1.0,
        }
    }
}

/// Fingerprint for colony state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColonyFingerprint(pub u32);

impl ColonyFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Display for ColonyFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "colony:{:08x}", self.0)
    }
}

/// Manager for colony simulation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColonyManager {
    config: ColonyConfig,
    job_defs: JobDefRegistry,
    jobs: JobRegistry,
    workers: WorkerRegistry,
    storage: StorageRegistry,
    routes: RouteRegistry,
    transfers: TransferRegistry,
    shelters: ShelterRegistry,
    failures: FailureRegistry,
    failure_log: FailureEventLog,
    current_tick: u64,
    population: u32,
}

impl ColonyManager {
    #[must_use]
    pub fn new(config: ColonyConfig) -> Self {
        let log_capacity = config.cascade_config.event_log_capacity;
        Self {
            config,
            failure_log: FailureEventLog::new(log_capacity),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn config(&self) -> &ColonyConfig {
        &self.config
    }

    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    #[must_use]
    pub fn population(&self) -> u32 {
        self.population
    }

    pub fn set_population(&mut self, population: u32) {
        self.population = population;
    }

    #[must_use]
    pub fn job_defs(&self) -> &JobDefRegistry {
        &self.job_defs
    }

    pub fn job_defs_mut(&mut self) -> &mut JobDefRegistry {
        &mut self.job_defs
    }

    #[must_use]
    pub fn jobs(&self) -> &JobRegistry {
        &self.jobs
    }

    pub fn jobs_mut(&mut self) -> &mut JobRegistry {
        &mut self.jobs
    }

    #[must_use]
    pub fn workers(&self) -> &WorkerRegistry {
        &self.workers
    }

    pub fn workers_mut(&mut self) -> &mut WorkerRegistry {
        &mut self.workers
    }

    #[must_use]
    pub fn storage(&self) -> &StorageRegistry {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut StorageRegistry {
        &mut self.storage
    }

    #[must_use]
    pub fn routes(&self) -> &RouteRegistry {
        &self.routes
    }

    pub fn routes_mut(&mut self) -> &mut RouteRegistry {
        &mut self.routes
    }

    #[must_use]
    pub fn transfers(&self) -> &TransferRegistry {
        &self.transfers
    }

    pub fn transfers_mut(&mut self) -> &mut TransferRegistry {
        &mut self.transfers
    }

    #[must_use]
    pub fn shelters(&self) -> &ShelterRegistry {
        &self.shelters
    }

    pub fn shelters_mut(&mut self) -> &mut ShelterRegistry {
        &mut self.shelters
    }

    #[must_use]
    pub fn failures(&self) -> &FailureRegistry {
        &self.failures
    }

    pub fn failures_mut(&mut self) -> &mut FailureRegistry {
        &mut self.failures
    }

    #[must_use]
    pub fn failure_log(&self) -> &FailureEventLog {
        &self.failure_log
    }

    pub fn register_job_def(&mut self, def: JobDef) {
        self.job_defs.register(def);
    }

    pub fn create_worker(&mut self, name: impl Into<String>) -> WorkerId {
        self.workers.create(name)
    }

    pub fn create_job(&mut self, def_id: JobDefId) -> Option<JobId> {
        let def = self.job_defs.get(&def_id)?;
        let work_required = def.base_work_amount;
        Some(self.jobs.create(def_id, work_required, self.current_tick))
    }

    pub fn create_storage(&mut self, name: impl Into<String>, capacity: u32) -> StorageNodeId {
        self.storage.create(name, capacity)
    }

    pub fn create_route(&mut self, source: StorageNodeId, dest: StorageNodeId) -> RouteId {
        self.routes.create(source, dest)
    }

    pub fn create_shelter(&mut self, name: impl Into<String>, capacity: u32) -> ShelterId {
        self.shelters.create(name, capacity, self.current_tick)
    }

    pub fn trigger_failure(&mut self, trigger: FailureTrigger) -> FailureId {
        let id = self.failures.create(trigger, self.current_tick);
        let Some(failure) = self.failures.get(id) else {
            unreachable!("just created failure must exist")
        };
        let severity = failure.severity;

        let event = FailureEvent::failure_triggered(self.current_tick, id, severity);
        self.failure_log.push(event);

        id
    }

    pub fn tick(&mut self) -> ColonyTickResult {
        self.current_tick += 1;
        let mut result = ColonyTickResult::new();

        self.tick_jobs(&mut result);
        self.tick_transfers(&mut result);
        self.tick_failures(&mut result);
        self.routes.reset_loads();

        result
    }

    fn tick_jobs(&mut self, result: &mut ColonyTickResult) {
        let completed_jobs: Vec<JobId> = self
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Completed)
            .map(|j| j.id)
            .collect();

        for job in self.jobs.iter_mut() {
            if job.is_blocked() {
                job.blocked_by.retain(|id| !completed_jobs.contains(id));
                if job.blocked_by.is_empty() && job.status == JobStatus::Blocked {
                    job.status = JobStatus::Pending;
                    result.job_events.push(JobEvent::new(
                        self.current_tick,
                        JobEventKind::JobUnblocked { job: job.id },
                    ));
                }
            }
        }

        if self.config.auto_assign_jobs {
            self.assign_jobs(result);
        }

        let working_pairs: Vec<(WorkerId, JobId)> = self
            .workers
            .iter()
            .filter_map(|w| w.current_job.map(|j| (w.id, j)))
            .collect();

        for (worker_id, job_id) in working_pairs {
            let Some(job) = self.jobs.get(job_id) else {
                continue;
            };
            let Some(def) = self.job_defs.get(&job.def_id) else {
                continue;
            };
            let required_skills = def.required_skills.clone();
            let Some(worker) = self.workers.get(worker_id) else {
                continue;
            };

            let work_speed = worker.work_speed(&required_skills);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "work bounded"
            )]
            let work_amount = (10.0 * work_speed).max(1.0) as u32;

            let job = self.jobs.get_mut(job_id).unwrap();
            if job.status == JobStatus::Claimed {
                job.start(self.current_tick);
                result
                    .job_events
                    .push(JobEvent::job_started(self.current_tick, job_id, worker_id));
            }

            let completed = job.add_work(work_amount);

            let worker = self.workers.get_mut(worker_id).unwrap();
            worker.record_work(work_amount);

            if completed {
                let job = self.jobs.get_mut(job_id).unwrap();
                let duration = job.duration(self.current_tick);
                job.complete(self.current_tick);

                let worker = self.workers.get_mut(worker_id).unwrap();
                worker.record_completion();
                worker.clear_job();

                result.jobs_completed += 1;
                result.job_events.push(JobEvent::job_completed(
                    self.current_tick,
                    job_id,
                    duration,
                ));
            }
        }
    }

    fn assign_jobs(&mut self, result: &mut ColonyTickResult) {
        let pending: Vec<JobId> = self.jobs.priority_sorted().iter().map(|j| j.id).collect();

        for job_id in pending {
            let job = self.jobs.get(job_id).unwrap();
            if job.status != JobStatus::Pending || job.is_blocked() {
                continue;
            }

            let Some(def) = self.job_defs.get(&job.def_id) else {
                continue;
            };

            let required = &def.required_skills;
            let available: Vec<WorkerId> = self
                .workers
                .available()
                .filter(|w| w.can_perform(required))
                .map(|w| w.id)
                .collect();

            if let Some(worker_id) = available.first() {
                let worker = self.workers.get_mut(*worker_id).unwrap();
                worker.assign_job(job_id);

                let job = self.jobs.get_mut(job_id).unwrap();
                job.assign_worker(*worker_id);

                result.jobs_assigned += 1;
                result.job_events.push(JobEvent::job_claimed(
                    self.current_tick,
                    job_id,
                    *worker_id,
                ));
            }
        }
    }

    fn tick_transfers(&mut self, result: &mut ColonyTickResult) {
        let active_ids: Vec<TransferId> = self.transfers.active().map(|t| t.id).collect();

        for transfer_id in active_ids {
            let transfer = self.transfers.get(transfer_id).unwrap();
            let resource = transfer.resource.clone();
            let source_id = transfer.source;
            let dest_id = transfer.destination;
            let remaining = transfer.remaining();

            let can_provide = self
                .storage
                .get(source_id)
                .map_or(0, |n| n.available_quantity(&resource));
            let can_accept = self
                .storage
                .get(dest_id)
                .map_or(0, StorageNode::available_capacity);

            let transfer_amount = remaining.min(can_provide).min(can_accept).min(100);

            if transfer_amount > 0 {
                if let Some(source) = self.storage.get_mut(source_id) {
                    source.withdraw(&resource, transfer_amount);
                }
                if let Some(dest) = self.storage.get_mut(dest_id) {
                    dest.store(&resource, transfer_amount);
                }

                let transfer = self.transfers.get_mut(transfer_id).unwrap();
                transfer.apply(transfer_amount);

                if transfer.is_complete() {
                    transfer.complete(self.current_tick);
                    result.transfers_completed += 1;
                    result
                        .logistics_events
                        .push(LogisticsEvent::transfer_completed(
                            self.current_tick,
                            transfer_id,
                            transfer.quantity,
                        ));
                }
            }
        }
    }

    fn tick_failures(&mut self, result: &mut ColonyTickResult) {
        let active_ids: Vec<FailureId> = self.failures.active().map(|f| f.id).collect();

        for failure_id in active_ids {
            let failure = self.failures.get_mut(failure_id).unwrap();

            failure.add_escalation_pressure(5);

            if failure.should_escalate() {
                let from = failure.severity;
                failure.escalate(self.current_tick);
                let to = failure.severity;

                let event =
                    FailureEvent::failure_escalated(self.current_tick, failure_id, from, to);
                self.failure_log.push(event.clone());
                result.failure_events.push(event);
            }

            if self.config.cascade_config.auto_mitigation_enabled
                && failure.status == FailureStatus::Contained
            {
                let rate = self.config.cascade_config.mitigation_rate;
                if failure.add_mitigation(rate) {
                    failure.resolve(self.current_tick);
                    let duration = failure.duration(self.current_tick);
                    result.failures_resolved += 1;
                    let event =
                        FailureEvent::failure_resolved(self.current_tick, failure_id, duration);
                    self.failure_log.push(event.clone());
                    result.failure_events.push(event);
                }
            }
        }

        self.failures
            .cleanup_resolved(self.config.max_event_history);
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "counts bounded")]
    pub fn snapshot(&self) -> ColonyStateSnapshot {
        let shelter_coverage =
            ShelterCoverage::from_registry(&self.shelters, self.population, self.current_tick);

        let mut snapshot = ColonyStateSnapshot::new(self.current_tick);
        snapshot.population = self.population;
        snapshot.total_workers = self.workers.count() as u32;
        snapshot.available_workers = self.workers.available().count() as u32;
        snapshot.total_jobs = self.jobs.count() as u32;
        snapshot.active_jobs = self.jobs.active().count() as u32;
        snapshot.pending_jobs = self.jobs.pending().count() as u32;
        snapshot.total_storage_nodes = self.storage.count() as u32;
        snapshot.total_storage_capacity = self.storage.total_capacity();
        snapshot.total_stored = self.storage.total_stored();
        snapshot.active_transfers = self.transfers.active().count() as u32;
        snapshot.total_shelters = self.shelters.count() as u32;
        snapshot.shelter_capacity = self.shelters.total_capacity();
        snapshot.shelter_occupancy = self.shelters.total_occupancy();
        snapshot.shelter_coverage = shelter_coverage.coverage_ratio;
        snapshot.average_shelter_rating = shelter_coverage.average_rating;
        snapshot.active_failures = self.failures.active_count() as u32;
        snapshot.total_failures = self.failures.count() as u32;

        for failure in self.failures.iter() {
            *snapshot
                .failure_severity_counts
                .entry(format!("{:?}", failure.severity))
                .or_insert(0) += 1;
        }

        snapshot
    }

    #[must_use]
    pub fn summary(&self) -> ColonyStateSummary {
        ColonyStateSummary::from(&self.snapshot())
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "counts bounded by game limits"
    )]
    pub fn fingerprint(&self) -> ColonyFingerprint {
        let mut hasher = crc32fast::Hasher::new();

        hasher.update(&self.current_tick.to_le_bytes());
        hasher.update(&self.population.to_le_bytes());

        let job_fp = u64::from(self.jobs.count() as u32);
        hasher.update(&job_fp.to_le_bytes());

        let worker_fp = u64::from(self.workers.count() as u32);
        hasher.update(&worker_fp.to_le_bytes());

        let storage_fp = u64::from(self.storage.total_stored());
        hasher.update(&storage_fp.to_le_bytes());

        let shelter_fp = u64::from(self.shelters.total_occupancy());
        hasher.update(&shelter_fp.to_le_bytes());

        let failure_fp = u64::from(self.failures.active_count() as u32);
        hasher.update(&failure_fp.to_le_bytes());

        ColonyFingerprint(hasher.finalize())
    }

    #[must_use]
    pub fn project(&self, ticks_ahead: u64) -> ColonyProjection {
        let mut projection =
            ColonyProjection::new(self.current_tick, self.current_tick + ticks_ahead);

        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded non-negative values"
        )]
        {
            let active_jobs = self.jobs.active().count();
            let working_workers = self
                .workers
                .iter()
                .filter(|w| w.current_job.is_some())
                .count();

            if working_workers > 0 && active_jobs > 0 {
                projection.estimated_job_completions =
                    ((ticks_ahead as f32 * working_workers as f32) / 50.0).min(active_jobs as f32)
                        as u32;
            }

            let failure_projection = FailureProjection::from_registry(
                &self.failures,
                self.current_tick,
                self.current_tick + ticks_ahead,
            );
            projection.estimated_failures =
                failure_projection.estimated_escalations + failure_projection.estimated_cascades;
            projection.risk_score = failure_projection.risk_score;
            projection.confidence = if ticks_ahead > 500 { 0.5 } else { 0.8 };
        }

        projection
    }

    pub fn events_for_failure(&self, failure: FailureId) -> impl Iterator<Item = &FailureEvent> {
        self.failure_log.for_failure(failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_manager() -> ColonyManager {
        let mut manager = ColonyManager::new(ColonyConfig::default());

        for def in job_presets::standard_job_defs() {
            manager.register_job_def(def);
        }

        manager
    }

    fn create_miner(manager: &mut ColonyManager, name: &str) -> WorkerId {
        let id = manager.create_worker(name);
        let worker = manager.workers_mut().get_mut(id).unwrap();
        worker
            .skills
            .add(WorkerCapability::new(job_presets::mining()));
        id
    }

    #[test]
    fn test_colony_manager_creation() {
        let manager = setup_manager();
        assert_eq!(manager.current_tick(), 0);
        assert_eq!(manager.workers().count(), 0);
        assert_eq!(manager.jobs().count(), 0);
    }

    #[test]
    fn test_job_creation_and_assignment() {
        let mut manager = setup_manager();

        let worker_id = create_miner(&mut manager, "Miner");
        let job_id = manager.create_job(job_presets::mine_ore()).unwrap();

        let result = manager.tick();

        assert_eq!(result.jobs_assigned, 1);

        let job = manager.jobs().get(job_id).unwrap();
        assert!(job.has_worker(worker_id));
    }

    #[test]
    fn test_job_completion() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_job(job_presets::mine_ore());

        let mut completed = false;
        for _ in 0..50 {
            let result = manager.tick();
            if result.jobs_completed > 0 {
                completed = true;
                break;
            }
        }

        assert!(completed);
    }

    #[test]
    fn test_storage_operations() {
        let mut manager = setup_manager();

        let id = manager.create_storage("Warehouse", 1000);
        let node = manager.storage_mut().get_mut(id).unwrap();

        node.store(&ResourceId::new("iron"), 100);
        assert_eq!(node.total_stored(), 100);

        node.withdraw(&ResourceId::new("iron"), 30);
        assert_eq!(manager.storage().get(id).unwrap().total_stored(), 70);
    }

    #[test]
    fn test_shelter_operations() {
        let mut manager = setup_manager();
        manager.set_population(100);

        let id = manager.create_shelter("Hab 1", 150);

        let tick = manager.current_tick();
        let shelter = manager.shelters_mut().get_mut(id).unwrap();
        shelter.admit(50, tick);

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.shelter_capacity, 150);
        assert_eq!(snapshot.shelter_occupancy, 50);
        assert!(snapshot.shelter_coverage >= 1.0);
    }

    #[test]
    fn test_failure_triggering() {
        let mut manager = setup_manager();

        let trigger = FailureTrigger::ResourceShortage {
            resource: ResourceId::new("oxygen"),
            deficit: 100,
        };

        let id = manager.trigger_failure(trigger);

        assert!(manager.failures().get(id).is_some());
        assert_eq!(manager.failures().active_count(), 1);
    }

    #[test]
    fn test_snapshot() {
        let mut manager = setup_manager();
        manager.set_population(50);

        create_miner(&mut manager, "Miner1");
        create_miner(&mut manager, "Miner2");
        manager.create_job(job_presets::mine_ore());
        manager.create_storage("Storage", 500);
        manager.create_shelter("Hab", 100);

        manager.tick();

        let snapshot = manager.snapshot();

        assert_eq!(snapshot.total_workers, 2);
        assert_eq!(snapshot.total_jobs, 1);
        assert_eq!(snapshot.total_storage_nodes, 1);
        assert_eq!(snapshot.total_shelters, 1);
    }

    #[test]
    fn test_summary() {
        let mut manager = setup_manager();
        manager.set_population(100);
        manager.create_shelter("Hab", 200);

        let summary = manager.summary();

        assert_eq!(summary.population, 100);
        assert!(summary.shelter_coverage >= 1.0);
        assert_eq!(summary.active_failures, 0);
    }

    #[test]
    fn test_fingerprint_determinism() {
        let mut manager1 = setup_manager();
        let mut manager2 = setup_manager();

        create_miner(&mut manager1, "Miner");
        create_miner(&mut manager2, "Miner");

        manager1.create_job(job_presets::mine_ore());
        manager2.create_job(job_presets::mine_ore());

        manager1.tick();
        manager2.tick();

        let fp1 = manager1.fingerprint();
        let fp2 = manager2.fingerprint();

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_fingerprint_changes() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_job(job_presets::mine_ore());

        let fp_before = manager.fingerprint();
        manager.tick();
        let fp_after = manager.fingerprint();

        assert!(!fp_before.matches(&fp_after));
    }

    #[test]
    fn test_projection() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_job(job_presets::mine_ore());
        manager.tick();

        let projection = manager.project(100);

        assert_eq!(projection.base_tick, manager.current_tick());
        assert_eq!(projection.projected_tick, manager.current_tick() + 100);
        assert!(projection.confidence > 0.0);
    }

    #[test]
    fn test_failure_log() {
        let mut manager = setup_manager();

        manager.trigger_failure(FailureTrigger::MoraleCrisis { level: 50 });
        manager.trigger_failure(FailureTrigger::PanicPressure { level: 60 });

        assert_eq!(manager.failure_log().len(), 2);
    }

    #[test]
    fn test_tick_result() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_job(job_presets::mine_ore());

        let result = manager.tick();

        assert_eq!(result.jobs_assigned, 1);
    }

    #[test]
    fn test_serde_snapshot() {
        let snapshot = ColonyStateSnapshot {
            tick: 500,
            population: 100,
            total_workers: 20,
            available_workers: 5,
            total_jobs: 10,
            active_jobs: 5,
            pending_jobs: 3,
            total_storage_nodes: 3,
            total_storage_capacity: 3000,
            total_stored: 1500,
            active_transfers: 2,
            total_shelters: 2,
            shelter_capacity: 200,
            shelter_occupancy: 100,
            shelter_coverage: 1.0,
            average_shelter_rating: 0.75,
            active_failures: 1,
            total_failures: 5,
            failure_severity_counts: BTreeMap::new(),
            resource_balances: BTreeMap::new(),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: ColonyStateSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, snapshot);
    }

    #[test]
    fn test_serde_summary() {
        let summary = ColonyStateSummary {
            tick: 100,
            population: 50,
            worker_utilization: 0.8,
            storage_utilization: 0.5,
            shelter_coverage: 1.0,
            active_failures: 0,
            stability_score: 1.0,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let restored: ColonyStateSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, summary);
    }

    #[test]
    fn test_serde_projection() {
        let projection = ColonyProjection::new(100, 500);

        let json = serde_json::to_string(&projection).unwrap();
        let restored: ColonyProjection = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, projection);
    }

    #[test]
    fn test_serde_config() {
        let config = ColonyConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let restored: ColonyConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, config);
    }

    #[test]
    fn test_bincode_snapshot() {
        let snapshot = ColonyStateSnapshot {
            tick: 999,
            population: 200,
            total_workers: 50,
            ..Default::default()
        };

        let bytes = bincode::serialize(&snapshot).unwrap();
        let restored: ColonyStateSnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 999);
        assert_eq!(restored.population, 200);
    }

    #[test]
    fn test_bincode_summary() {
        let summary = ColonyStateSummary {
            tick: 500,
            population: 100,
            stability_score: 0.9,
            ..Default::default()
        };

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: ColonyStateSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 500);
    }

    #[test]
    fn test_bincode_fingerprint() {
        let fp = ColonyFingerprint(0xCAFE_BABE);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: ColonyFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.raw(), 0xCAFE_BABE);
    }

    #[test]
    fn test_bincode_manager() {
        let mut manager = setup_manager();
        create_miner(&mut manager, "Miner");
        manager.create_job(job_presets::mine_ore());
        manager.tick();

        let bytes = bincode::serialize(&manager).unwrap();
        let restored: ColonyManager = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.current_tick(), manager.current_tick());
        assert_eq!(restored.workers().count(), manager.workers().count());
        assert_eq!(restored.jobs().count(), manager.jobs().count());
        assert!(restored.fingerprint().matches(&manager.fingerprint()));
    }

    #[test]
    fn test_snapshot_checksum() {
        let snapshot1 = ColonyStateSnapshot {
            tick: 100,
            population: 50,
            total_workers: 10,
            ..Default::default()
        };

        let snapshot2 = snapshot1.clone();

        assert_eq!(snapshot1.checksum(), snapshot2.checksum());
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = ColonyFingerprint(0x1234_5678);
        assert_eq!(format!("{fp}"), "colony:12345678");
    }

    #[test]
    fn test_colony_config_builder() {
        let config = ColonyConfig::new()
            .with_max_event_history(500)
            .with_cascade_config(CascadeConfig::new().with_max_cascade_depth(3));

        assert_eq!(config.max_event_history, 500);
        assert_eq!(config.cascade_config.max_cascade_depth, 3);
    }
}

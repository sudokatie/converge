//! Task assignment and reservation logic.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::ids::{SettlerId, TaskId};
use super::priority::{AssignmentCandidate, PriorityConfig, PriorityScore};
use super::state::Settler;
use super::task::Task;
use super::task_def::TaskDefRegistry;

/// Result of an assignment attempt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AssignmentResult {
    Assigned { task: TaskId, settler: SettlerId },
    NoEligibleWorkers { task: TaskId },
    TaskUnavailable { task: TaskId, reason: String },
    SettlerUnavailable { settler: SettlerId, reason: String },
    AlreadyAssigned { task: TaskId, settler: SettlerId },
}

impl AssignmentResult {
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Assigned { .. })
    }
}

/// Reservation state for tasks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReservationTable {
    task_to_settlers: BTreeMap<TaskId, BTreeSet<SettlerId>>,
    settler_to_tasks: BTreeMap<SettlerId, BTreeSet<TaskId>>,
}

impl ReservationTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reserve(&mut self, task: TaskId, settler: SettlerId) {
        self.task_to_settlers
            .entry(task)
            .or_default()
            .insert(settler);
        self.settler_to_tasks
            .entry(settler)
            .or_default()
            .insert(task);
    }

    pub fn release(&mut self, task: TaskId, settler: SettlerId) {
        if let Some(settlers) = self.task_to_settlers.get_mut(&task) {
            settlers.remove(&settler);
            if settlers.is_empty() {
                self.task_to_settlers.remove(&task);
            }
        }
        if let Some(tasks) = self.settler_to_tasks.get_mut(&settler) {
            tasks.remove(&task);
            if tasks.is_empty() {
                self.settler_to_tasks.remove(&settler);
            }
        }
    }

    pub fn release_all_for_settler(&mut self, settler: SettlerId) {
        if let Some(tasks) = self.settler_to_tasks.remove(&settler) {
            for task in tasks {
                if let Some(settlers) = self.task_to_settlers.get_mut(&task) {
                    settlers.remove(&settler);
                    if settlers.is_empty() {
                        self.task_to_settlers.remove(&task);
                    }
                }
            }
        }
    }

    pub fn release_all_for_task(&mut self, task: TaskId) {
        if let Some(settlers) = self.task_to_settlers.remove(&task) {
            for settler in settlers {
                if let Some(tasks) = self.settler_to_tasks.get_mut(&settler) {
                    tasks.remove(&task);
                    if tasks.is_empty() {
                        self.settler_to_tasks.remove(&settler);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn has_reservation(&self, task: TaskId, settler: SettlerId) -> bool {
        self.task_to_settlers
            .get(&task)
            .is_some_and(|s| s.contains(&settler))
    }

    pub fn reservations_for_task(&self, task: TaskId) -> impl Iterator<Item = SettlerId> + '_ {
        self.task_to_settlers
            .get(&task)
            .into_iter()
            .flatten()
            .copied()
    }

    pub fn reservations_for_settler(
        &self,
        settler: SettlerId,
    ) -> impl Iterator<Item = TaskId> + '_ {
        self.settler_to_tasks
            .get(&settler)
            .into_iter()
            .flatten()
            .copied()
    }

    #[must_use]
    pub fn reservation_count_for_task(&self, task: TaskId) -> usize {
        self.task_to_settlers.get(&task).map_or(0, BTreeSet::len)
    }

    #[must_use]
    pub fn reservation_count_for_settler(&self, settler: SettlerId) -> usize {
        self.settler_to_tasks.get(&settler).map_or(0, BTreeSet::len)
    }
}

/// Configuration for the assignment algorithm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssignmentConfig {
    pub priority: PriorityConfig,
    pub max_reservations_per_settler: usize,
    pub prefer_reserved: bool,
    pub require_capability_match: bool,
}

impl Default for AssignmentConfig {
    fn default() -> Self {
        Self {
            priority: PriorityConfig::default(),
            max_reservations_per_settler: 3,
            prefer_reserved: true,
            require_capability_match: true,
        }
    }
}

/// Assignment engine for matching settlers to tasks.
pub struct AssignmentEngine<'a> {
    config: &'a AssignmentConfig,
    task_defs: &'a TaskDefRegistry,
}

impl<'a> AssignmentEngine<'a> {
    #[must_use]
    pub fn new(config: &'a AssignmentConfig, task_defs: &'a TaskDefRegistry) -> Self {
        Self { config, task_defs }
    }

    #[must_use]
    pub fn can_settler_perform_task(&self, settler: &Settler, task: &Task) -> bool {
        if !settler.can_work() {
            return false;
        }

        if self.config.require_capability_match
            && let Some(def) = self.task_defs.get(&task.def_id)
        {
            if !settler.skills.can_perform(
                &def.required_capabilities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            ) {
                return false;
            }
            if settler.work_priorities.is_forbidden(def.category) {
                return false;
            }
        }

        true
    }

    pub fn find_best_assignment(
        &self,
        available_tasks: &[&Task],
        available_settlers: &[&Settler],
        _reservations: &ReservationTable,
        current_tick: u64,
    ) -> Vec<AssignmentCandidate> {
        let mut candidates = Vec::new();

        for task in available_tasks {
            if !task.status.can_be_claimed() || task.is_blocked() {
                continue;
            }

            let Some(def) = self.task_defs.get(&task.def_id) else {
                continue;
            };

            let current_workers = task.worker_count();
            if current_workers >= def.max_workers as usize {
                continue;
            }

            for settler in available_settlers {
                if !self.can_settler_perform_task(settler, task) {
                    continue;
                }

                if settler.current_task.is_some() {
                    continue;
                }

                let score = PriorityScore::calculate(
                    task,
                    def,
                    settler,
                    current_tick,
                    &self.config.priority,
                );

                let candidate =
                    AssignmentCandidate::new(score, task, settler, &self.config.priority);
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|a, b| {
            let score_a = a.adjusted_score(&self.config.priority);
            let score_b = b.adjusted_score(&self.config.priority);

            if self.config.prefer_reserved && a.is_reserved != b.is_reserved {
                return if a.is_reserved {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                };
            }

            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }

    pub fn assign_greedy(
        &self,
        tasks: &mut BTreeMap<TaskId, Task>,
        settlers: &mut BTreeMap<SettlerId, Settler>,
        reservations: &mut ReservationTable,
        current_tick: u64,
    ) -> Vec<AssignmentResult> {
        let mut results = Vec::new();
        let mut assigned_settlers: BTreeSet<SettlerId> = BTreeSet::new();
        let mut assigned_tasks: BTreeSet<TaskId> = BTreeSet::new();

        let available_tasks: Vec<_> = tasks
            .values()
            .filter(|t| t.status.can_be_claimed() && !t.is_blocked())
            .collect();

        let available_settlers: Vec<_> = settlers.values().filter(|s| s.is_available()).collect();

        let candidates = self.find_best_assignment(
            &available_tasks,
            &available_settlers,
            reservations,
            current_tick,
        );

        for candidate in candidates {
            let task_id = candidate.score.task_id;
            let settler_id = candidate.score.settler_id;

            if assigned_tasks.contains(&task_id) || assigned_settlers.contains(&settler_id) {
                continue;
            }

            if let (Some(task), Some(settler)) =
                (tasks.get_mut(&task_id), settlers.get_mut(&settler_id))
            {
                let def = self.task_defs.get(&task.def_id);
                let max_workers = def.map_or(1, |d| d.max_workers as usize);

                if task.worker_count() >= max_workers {
                    continue;
                }

                task.claim(settler_id, current_tick);
                settler.assign_task(task_id);
                reservations.release(task_id, settler_id);

                assigned_tasks.insert(task_id);
                assigned_settlers.insert(settler_id);

                results.push(AssignmentResult::Assigned {
                    task: task_id,
                    settler: settler_id,
                });
            }
        }

        results
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::settler::capability::{Skill, SkillSet, presets as cap};
    use crate::settler::task_def::{TaskCategory, TaskDef, presets as task_presets};

    fn setup_registry() -> TaskDefRegistry {
        let mut registry = TaskDefRegistry::new();
        registry.register(
            TaskDef::new(task_presets::mine_rock(), "Mine", TaskCategory::Production)
                .with_capability(cap::mining()),
        );
        registry.register(
            TaskDef::new(task_presets::haul_item(), "Haul", TaskCategory::Hauling)
                .with_capability(cap::hauling()),
        );
        registry
    }

    #[test]
    fn test_reservation_table() {
        let mut table = ReservationTable::new();
        let task = TaskId::new(1);
        let settler = SettlerId::new(1);

        table.reserve(task, settler);
        assert!(table.has_reservation(task, settler));
        assert_eq!(table.reservation_count_for_task(task), 1);
        assert_eq!(table.reservation_count_for_settler(settler), 1);

        table.release(task, settler);
        assert!(!table.has_reservation(task, settler));
    }

    #[test]
    fn test_release_all_for_settler() {
        let mut table = ReservationTable::new();
        let settler = SettlerId::new(1);

        table.reserve(TaskId::new(1), settler);
        table.reserve(TaskId::new(2), settler);
        table.reserve(TaskId::new(3), settler);

        table.release_all_for_settler(settler);
        assert_eq!(table.reservation_count_for_settler(settler), 0);
    }

    #[test]
    fn test_can_settler_perform_task() {
        let registry = setup_registry();
        let config = AssignmentConfig::default();
        let engine = AssignmentEngine::new(&config, &registry);

        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(cap::mining()));

        let settler = Settler::new(SettlerId::new(1), "Miner", 0).with_skills(skills);
        let mine_task = Task::new(TaskId::new(1), task_presets::mine_rock(), 100, 0);
        let haul_task = Task::new(TaskId::new(2), task_presets::haul_item(), 50, 0);

        assert!(engine.can_settler_perform_task(&settler, &mine_task));
        assert!(!engine.can_settler_perform_task(&settler, &haul_task));
    }

    #[test]
    fn test_greedy_assignment() {
        let registry = setup_registry();
        let config = AssignmentConfig::default();
        let engine = AssignmentEngine::new(&config, &registry);

        let mut tasks = BTreeMap::new();
        tasks.insert(
            TaskId::new(1),
            Task::new(TaskId::new(1), task_presets::mine_rock(), 100, 0),
        );

        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(cap::mining()));

        let mut settlers = BTreeMap::new();
        settlers.insert(
            SettlerId::new(1),
            Settler::new(SettlerId::new(1), "Miner", 0).with_skills(skills),
        );

        let mut reservations = ReservationTable::new();

        let results = engine.assign_greedy(&mut tasks, &mut settlers, &mut reservations, 0);

        assert_eq!(results.len(), 1);
        assert!(results[0].is_success());
    }

    #[test]
    fn test_assignment_respects_max_workers() {
        let mut registry = TaskDefRegistry::new();
        registry.register(
            TaskDef::new(
                task_presets::construct_wall(),
                "Build",
                TaskCategory::Construction,
            )
            .with_capability(cap::construction())
            .with_max_workers(1),
        );

        let config = AssignmentConfig::default();
        let engine = AssignmentEngine::new(&config, &registry);

        let mut task = Task::new(TaskId::new(1), task_presets::construct_wall(), 200, 0);
        task.assign_worker(SettlerId::new(99));

        let mut skills = SkillSet::new();
        skills.add_skill(Skill::new(cap::construction()));

        let settler = Settler::new(SettlerId::new(1), "Builder", 0).with_skills(skills);

        let available_tasks = vec![&task];
        let available_settlers = vec![&settler];
        let reservations = ReservationTable::new();

        let candidates =
            engine.find_best_assignment(&available_tasks, &available_settlers, &reservations, 0);

        assert!(candidates.is_empty());
    }

    #[test]
    fn test_assignment_config_serde() {
        let config = AssignmentConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: AssignmentConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored.max_reservations_per_settler,
            config.max_reservations_per_settler
        );
    }
}

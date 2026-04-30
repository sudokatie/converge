//! Task priority calculation.

use serde::{Deserialize, Serialize};

use super::ids::{RegionId, SettlerId, TaskId};
use super::state::Settler;
use super::task::Task;
use super::task_def::{PriorityMode, TaskDef};

/// Computed priority score for a task-settler pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriorityScore {
    pub task_id: TaskId,
    pub settler_id: SettlerId,
    pub base_priority: i32,
    pub distance_modifier: f32,
    pub age_modifier: f32,
    pub skill_modifier: f32,
    pub settler_preference: i32,
    pub final_score: f32,
}

impl PriorityScore {
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "priority values bounded")]
    pub fn calculate(
        task: &Task,
        task_def: &TaskDef,
        settler: &Settler,
        current_tick: u64,
        config: &PriorityConfig,
    ) -> Self {
        let base_priority = task.priority_override.unwrap_or(task_def.base_priority);

        let distance_modifier = match task_def.priority_mode {
            PriorityMode::Distance => {
                if let (Some(task_pos), Some(settler_pos)) = (&task.position, &settler.position) {
                    let dist = settler_pos.distance(task_pos);
                    -(dist * config.distance_penalty)
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        let age_modifier = match task_def.priority_mode {
            PriorityMode::Age => {
                let age = task.age(current_tick);
                (age as f32) * config.age_bonus
            }
            _ => 0.0,
        };

        let skill_modifier = task_def
            .required_capabilities
            .iter()
            .filter_map(|cap| settler.skills.effective_level(cap))
            .map(super::capability::SkillLevel::speed_multiplier)
            .sum::<f32>()
            * config.skill_bonus;

        let settler_preference = settler.work_priorities.get_priority(task_def.category);

        let final_score = (base_priority as f32)
            + distance_modifier
            + age_modifier
            + skill_modifier
            + (settler_preference as f32 * config.preference_weight);

        Self {
            task_id: task.id,
            settler_id: settler.id,
            base_priority,
            distance_modifier,
            age_modifier,
            skill_modifier,
            settler_preference,
            final_score,
        }
    }
}

impl PartialOrd for PriorityScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.final_score.partial_cmp(&other.final_score)
    }
}

/// Configuration for priority calculation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriorityConfig {
    pub distance_penalty: f32,
    pub age_bonus: f32,
    pub skill_bonus: f32,
    pub preference_weight: f32,
    pub same_region_bonus: f32,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        Self {
            distance_penalty: 0.01,
            age_bonus: 0.001,
            skill_bonus: 2.0,
            preference_weight: 1.0,
            same_region_bonus: 5.0,
        }
    }
}

impl PriorityConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_distance_penalty(mut self, penalty: f32) -> Self {
        self.distance_penalty = penalty.max(0.0);
        self
    }

    #[must_use]
    pub fn with_age_bonus(mut self, bonus: f32) -> Self {
        self.age_bonus = bonus.max(0.0);
        self
    }

    #[must_use]
    pub fn with_skill_bonus(mut self, bonus: f32) -> Self {
        self.skill_bonus = bonus;
        self
    }
}

/// Ranked assignment candidate.
#[derive(Clone, Debug)]
pub struct AssignmentCandidate {
    pub score: PriorityScore,
    pub is_reserved: bool,
    pub same_region: bool,
}

impl AssignmentCandidate {
    #[must_use]
    pub fn new(
        score: PriorityScore,
        task: &Task,
        settler: &Settler,
        _config: &PriorityConfig,
    ) -> Self {
        let is_reserved = settler.has_reservation(score.task_id);
        let same_region = task.region.is_some() && task.region == settler.region;

        Self {
            score,
            is_reserved,
            same_region,
        }
    }

    #[must_use]
    pub fn adjusted_score(&self, config: &PriorityConfig) -> f32 {
        let mut score = self.score.final_score;
        if self.same_region {
            score += config.same_region_bonus;
        }
        score
    }
}

/// Region-specific priority adjustment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionPriority {
    pub region: RegionId,
    pub priority_boost: i32,
    pub urgent: bool,
}

impl RegionPriority {
    #[must_use]
    pub fn new(region: RegionId) -> Self {
        Self {
            region,
            priority_boost: 0,
            urgent: false,
        }
    }

    #[must_use]
    pub fn with_boost(mut self, boost: i32) -> Self {
        self.priority_boost = boost;
        self
    }

    #[must_use]
    pub fn urgent(mut self) -> Self {
        self.urgent = true;
        self
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;
    use crate::settler::capability::{Skill, SkillLevel, SkillSet, presets as cap};
    use crate::settler::task::TaskPosition;
    use crate::settler::task_def::{TaskCategory, TaskDef, presets as task_presets};

    fn make_task(id: u64) -> Task {
        Task::new(TaskId::new(id), task_presets::mine_rock(), 100, 0)
    }

    fn make_settler(id: u64) -> Settler {
        Settler::new(SettlerId::new(id), format!("Settler{id}"), 0)
    }

    fn make_task_def() -> TaskDef {
        TaskDef::new(task_presets::mine_rock(), "Mine", TaskCategory::Production)
            .with_capability(cap::mining())
            .with_priority(5)
    }

    #[test]
    fn test_basic_priority() {
        let task = make_task(1);
        let settler = make_settler(1);
        let def = make_task_def();
        let config = PriorityConfig::default();

        let score = PriorityScore::calculate(&task, &def, &settler, 0, &config);
        assert_eq!(score.base_priority, 5);
        assert!(score.final_score > 0.0);
    }

    #[test]
    fn test_distance_priority() {
        let task = Task::new(TaskId::new(1), task_presets::haul_item(), 50, 0)
            .with_position(TaskPosition::new(100.0, 0.0, 0.0));

        let settler = Settler::new(SettlerId::new(1), "Test", 0)
            .with_position(TaskPosition::new(0.0, 0.0, 0.0));

        let def = TaskDef::new(task_presets::haul_item(), "Haul", TaskCategory::Hauling)
            .with_priority_mode(PriorityMode::Distance);

        let config = PriorityConfig::default();
        let score = PriorityScore::calculate(&task, &def, &settler, 0, &config);

        assert!(score.distance_modifier < 0.0);
    }

    #[test]
    fn test_age_priority() {
        let task = Task::new(TaskId::new(1), task_presets::mine_rock(), 100, 0);
        let settler = make_settler(1);
        let def = TaskDef::new(task_presets::mine_rock(), "Mine", TaskCategory::Production)
            .with_priority_mode(PriorityMode::Age);

        let config = PriorityConfig::default();

        let score_new = PriorityScore::calculate(&task, &def, &settler, 0, &config);
        let score_old = PriorityScore::calculate(&task, &def, &settler, 1000, &config);

        assert!(score_old.age_modifier > score_new.age_modifier);
    }

    #[test]
    fn test_skill_priority() {
        let task = make_task(1);
        let def = make_task_def();
        let config = PriorityConfig::default();

        let novice = make_settler(1);

        let mut expert_skills = SkillSet::new();
        expert_skills.add_skill(Skill::new(cap::mining()).with_level(SkillLevel::Expert));
        let expert = Settler::new(SettlerId::new(2), "Expert", 0).with_skills(expert_skills);

        let novice_score = PriorityScore::calculate(&task, &def, &novice, 0, &config);
        let expert_score = PriorityScore::calculate(&task, &def, &expert, 0, &config);

        assert!(expert_score.skill_modifier > novice_score.skill_modifier);
    }

    #[test]
    fn test_assignment_candidate() {
        let task = make_task(1).with_region(RegionId::new(1));
        let settler = make_settler(1).with_region(RegionId::new(1));
        let def = make_task_def();
        let config = PriorityConfig::default();

        let score = PriorityScore::calculate(&task, &def, &settler, 0, &config);
        let candidate = AssignmentCandidate::new(score, &task, &settler, &config);

        assert!(candidate.same_region);
        assert!(candidate.adjusted_score(&config) > candidate.score.final_score);
    }

    #[test]
    fn test_priority_config_serde() {
        let config = PriorityConfig::default()
            .with_distance_penalty(0.02)
            .with_age_bonus(0.005);

        let json = serde_json::to_string(&config).unwrap();
        let restored: PriorityConfig = serde_json::from_str(&json).unwrap();

        assert!((restored.distance_penalty - 0.02).abs() < f32::EPSILON);
    }
}

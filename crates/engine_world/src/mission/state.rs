//! Mission runtime state tracking.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{MissionId, ObjectiveId};

/// State of a mission in its lifecycle.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum MissionState {
    /// Mission is available but not yet accepted.
    #[default]
    Available = 0,
    /// Mission has been accepted but not started.
    Accepted = 1,
    /// Mission is actively being worked on.
    Active = 2,
    /// Mission has been completed successfully.
    Completed = 3,
    /// Mission has failed.
    Failed = 4,
    /// Mission deadline has expired.
    Expired = 5,
    /// Mission was abandoned by the player.
    Abandoned = 6,
}

impl MissionState {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Accepted => "accepted",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Abandoned => "abandoned",
        }
    }

    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Available => "Available",
            Self::Accepted => "Accepted",
            Self::Active => "Active",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Expired => "Expired",
            Self::Abandoned => "Abandoned",
        }
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Expired | Self::Abandoned
        )
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Accepted | Self::Active)
    }

    #[must_use]
    pub const fn is_successful(&self) -> bool {
        matches!(self, Self::Completed)
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Available),
            1 => Some(Self::Accepted),
            2 => Some(Self::Active),
            3 => Some(Self::Completed),
            4 => Some(Self::Failed),
            5 => Some(Self::Expired),
            6 => Some(Self::Abandoned),
            _ => None,
        }
    }
}

/// State of a single objective.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum ObjectiveState {
    /// Objective is not yet started.
    #[default]
    Pending = 0,
    /// Objective is in progress.
    InProgress = 1,
    /// Objective has been completed.
    Completed = 2,
    /// Objective has failed.
    Failed = 3,
    /// Objective was skipped (optional only).
    Skipped = 4,
}

impl ObjectiveState {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }

    #[must_use]
    pub const fn is_successful(&self) -> bool {
        matches!(self, Self::Completed)
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Pending),
            1 => Some(Self::InProgress),
            2 => Some(Self::Completed),
            3 => Some(Self::Failed),
            4 => Some(Self::Skipped),
            _ => None,
        }
    }
}

/// Progress for a single objective.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveProgress {
    /// Objective identifier.
    pub id: ObjectiveId,

    /// Current state.
    pub state: ObjectiveState,

    /// Current progress count.
    pub current_count: u32,

    /// Target count for completion.
    pub target_count: u32,

    /// Elapsed ticks for timed objectives.
    pub elapsed_ticks: u64,

    /// Target duration for timed objectives.
    pub target_duration: u64,

    /// Tick when objective started.
    pub started_at: Option<u64>,

    /// Tick when objective completed/failed.
    pub ended_at: Option<u64>,

    /// Whether this objective is optional.
    pub optional: bool,

    /// Whether this objective is hidden.
    pub hidden: bool,

    /// Custom state data.
    pub state_data: BTreeMap<String, String>,
}

impl ObjectiveProgress {
    /// Create new objective progress.
    #[must_use]
    pub fn new(id: ObjectiveId, target_count: u32, target_duration: u64, optional: bool) -> Self {
        Self {
            id,
            state: ObjectiveState::Pending,
            current_count: 0,
            target_count,
            elapsed_ticks: 0,
            target_duration,
            started_at: None,
            ended_at: None,
            optional,
            hidden: false,
            state_data: BTreeMap::new(),
        }
    }

    /// Start the objective.
    pub fn start(&mut self, tick: u64) {
        if self.state == ObjectiveState::Pending {
            self.state = ObjectiveState::InProgress;
            self.started_at = Some(tick);
        }
    }

    /// Add progress.
    pub fn add_progress(&mut self, amount: u32, tick: u64) {
        if self.state == ObjectiveState::Pending {
            self.start(tick);
        }
        if self.state == ObjectiveState::InProgress {
            self.current_count = self.current_count.saturating_add(amount);
            if self.current_count >= self.target_count && self.target_count > 0 {
                self.complete(tick);
            }
        }
    }

    /// Add elapsed time.
    pub fn add_elapsed(&mut self, ticks: u64, current_tick: u64) {
        if self.state == ObjectiveState::Pending {
            self.start(current_tick);
        }
        if self.state == ObjectiveState::InProgress {
            self.elapsed_ticks = self.elapsed_ticks.saturating_add(ticks);
            if self.elapsed_ticks >= self.target_duration && self.target_duration > 0 {
                self.complete(current_tick);
            }
        }
    }

    /// Complete the objective.
    pub fn complete(&mut self, tick: u64) {
        if !self.state.is_terminal() {
            self.state = ObjectiveState::Completed;
            self.ended_at = Some(tick);
        }
    }

    /// Fail the objective.
    pub fn fail(&mut self, tick: u64) {
        if !self.state.is_terminal() {
            self.state = ObjectiveState::Failed;
            self.ended_at = Some(tick);
        }
    }

    /// Skip the objective (optional only).
    pub fn skip(&mut self, tick: u64) {
        if self.optional && !self.state.is_terminal() {
            self.state = ObjectiveState::Skipped;
            self.ended_at = Some(tick);
        }
    }

    /// Reveal a hidden objective.
    pub fn reveal(&mut self) {
        self.hidden = false;
    }

    /// Get progress percentage (0.0 to 1.0).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn progress_fraction(&self) -> f32 {
        if self.target_count > 0 {
            (self.current_count as f32 / self.target_count as f32).min(1.0)
        } else if self.target_duration > 0 {
            (self.elapsed_ticks as f32 / self.target_duration as f32).min(1.0)
        } else if self.state == ObjectiveState::Completed {
            1.0
        } else {
            0.0
        }
    }

    /// Get remaining count.
    #[must_use]
    pub fn remaining_count(&self) -> u32 {
        self.target_count.saturating_sub(self.current_count)
    }

    /// Get remaining duration.
    #[must_use]
    pub fn remaining_duration(&self) -> u64 {
        self.target_duration.saturating_sub(self.elapsed_ticks)
    }

    /// Set custom state.
    pub fn set_state_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.state_data.insert(key.into(), value.into());
    }

    /// Get custom state.
    #[must_use]
    pub fn get_state_data(&self, key: &str) -> Option<&str> {
        self.state_data.get(key).map(String::as_str)
    }
}

/// Runtime progress for an active mission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionProgress {
    /// Mission instance identifier.
    pub id: MissionId,

    /// Definition identifier.
    pub definition_id: String,

    /// Current state.
    pub state: MissionState,

    /// Objective progress, ordered by `ObjectiveId`.
    pub objectives: BTreeMap<ObjectiveId, ObjectiveProgress>,

    /// Tick when mission was accepted.
    pub accepted_at: u64,

    /// Tick when mission became active.
    pub started_at: Option<u64>,

    /// Tick when mission ended (completed/failed/expired/abandoned).
    pub ended_at: Option<u64>,

    /// Deadline tick.
    pub deadline: Option<u64>,

    /// Number of deadline extensions used.
    pub extensions_used: u32,

    /// Region where mission is active.
    pub active_region: Option<String>,

    /// Custom state data.
    pub state_data: BTreeMap<String, String>,
}

impl MissionProgress {
    /// Create new mission progress.
    #[must_use]
    pub fn new(id: MissionId, definition_id: impl Into<String>, accepted_at: u64) -> Self {
        Self {
            id,
            definition_id: definition_id.into(),
            state: MissionState::Accepted,
            objectives: BTreeMap::new(),
            accepted_at,
            started_at: None,
            ended_at: None,
            deadline: None,
            extensions_used: 0,
            active_region: None,
            state_data: BTreeMap::new(),
        }
    }

    /// Set deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Set active region.
    #[must_use]
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.active_region = Some(region.into());
        self
    }

    /// Add an objective.
    pub fn add_objective(&mut self, progress: ObjectiveProgress) {
        self.objectives.insert(progress.id, progress);
    }

    /// Start the mission.
    pub fn start(&mut self, tick: u64) {
        if self.state == MissionState::Accepted {
            self.state = MissionState::Active;
            self.started_at = Some(tick);
        }
    }

    /// Complete the mission.
    pub fn complete(&mut self, tick: u64) {
        if self.state.is_active() {
            self.state = MissionState::Completed;
            self.ended_at = Some(tick);
        }
    }

    /// Fail the mission.
    pub fn fail(&mut self, tick: u64) {
        if self.state.is_active() {
            self.state = MissionState::Failed;
            self.ended_at = Some(tick);
        }
    }

    /// Expire the mission.
    pub fn expire(&mut self, tick: u64) {
        if self.state.is_active() {
            self.state = MissionState::Expired;
            self.ended_at = Some(tick);
        }
    }

    /// Abandon the mission.
    pub fn abandon(&mut self, tick: u64) {
        if self.state.is_active() {
            self.state = MissionState::Abandoned;
            self.ended_at = Some(tick);
        }
    }

    /// Extend the deadline.
    pub fn extend_deadline(&mut self, additional_ticks: u64) -> bool {
        if let Some(deadline) = self.deadline.as_mut() {
            *deadline = deadline.saturating_add(additional_ticks);
            self.extensions_used += 1;
            true
        } else {
            false
        }
    }

    /// Check if deadline has passed.
    #[must_use]
    pub fn is_past_deadline(&self, tick: u64) -> bool {
        self.deadline.is_some_and(|d| tick >= d)
    }

    /// Get time until deadline.
    #[must_use]
    pub fn time_until_deadline(&self, tick: u64) -> Option<u64> {
        self.deadline.map(|d| d.saturating_sub(tick))
    }

    /// Get objective progress.
    #[must_use]
    pub fn objective(&self, id: ObjectiveId) -> Option<&ObjectiveProgress> {
        self.objectives.get(&id)
    }

    /// Get mutable objective progress.
    pub fn objective_mut(&mut self, id: ObjectiveId) -> Option<&mut ObjectiveProgress> {
        self.objectives.get_mut(&id)
    }

    /// Count of completed objectives.
    #[must_use]
    pub fn completed_objective_count(&self) -> usize {
        self.objectives
            .values()
            .filter(|o| o.state.is_successful())
            .count()
    }

    /// Count of required completed objectives.
    #[must_use]
    pub fn completed_required_count(&self) -> usize {
        self.objectives
            .values()
            .filter(|o| !o.optional && o.state.is_successful())
            .count()
    }

    /// Count of required objectives.
    #[must_use]
    pub fn required_objective_count(&self) -> usize {
        self.objectives.values().filter(|o| !o.optional).count()
    }

    /// Check if all required objectives are complete.
    #[must_use]
    pub fn all_required_complete(&self) -> bool {
        self.objectives
            .values()
            .filter(|o| !o.optional)
            .all(|o| o.state.is_successful())
    }

    /// Check if any required objective has failed.
    #[must_use]
    pub fn any_required_failed(&self) -> bool {
        self.objectives
            .values()
            .filter(|o| !o.optional)
            .any(|o| o.state == ObjectiveState::Failed)
    }

    /// Get overall progress fraction.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn overall_progress(&self) -> f32 {
        let required: Vec<_> = self.objectives.values().filter(|o| !o.optional).collect();
        if required.is_empty() {
            return 1.0;
        }
        let total: f32 = required.iter().map(|o| o.progress_fraction()).sum();
        total / required.len() as f32
    }

    /// Check if all optional objectives are complete.
    #[must_use]
    pub fn all_optional_complete(&self) -> bool {
        self.objectives
            .values()
            .filter(|o| o.optional)
            .all(|o| o.state.is_successful())
    }

    /// Set custom state.
    pub fn set_state_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.state_data.insert(key.into(), value.into());
    }

    /// Get custom state.
    #[must_use]
    pub fn get_state_data(&self, key: &str) -> Option<&str> {
        self.state_data.get(key).map(String::as_str)
    }

    /// Get elapsed time since start.
    #[must_use]
    pub fn elapsed_time(&self, current_tick: u64) -> u64 {
        self.started_at
            .map_or(0, |start| current_tick.saturating_sub(start))
    }

    /// Get summary for unloaded region projection.
    #[must_use]
    pub fn projection_summary(&self) -> ProjectionSummary {
        ProjectionSummary {
            mission_id: self.id,
            definition_id: self.definition_id.clone(),
            state: self.state,
            overall_progress: self.overall_progress(),
            required_complete: self.completed_required_count(),
            required_total: self.required_objective_count(),
            deadline: self.deadline,
            active_region: self.active_region.clone(),
        }
    }
}

/// Summary for projecting mission state in unloaded regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionSummary {
    /// Mission identifier.
    pub mission_id: MissionId,

    /// Definition identifier.
    pub definition_id: String,

    /// Current state.
    pub state: MissionState,

    /// Overall progress fraction.
    pub overall_progress: f32,

    /// Completed required objectives.
    pub required_complete: usize,

    /// Total required objectives.
    pub required_total: usize,

    /// Deadline tick if any.
    pub deadline: Option<u64>,

    /// Active region.
    pub active_region: Option<String>,
}

impl ProjectionSummary {
    /// Estimate completion tick based on current progress.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn estimated_completion(&self, current_tick: u64, start_tick: u64) -> Option<u64> {
        if self.overall_progress <= 0.0 || self.overall_progress >= 1.0 {
            return None;
        }
        let elapsed = current_tick.saturating_sub(start_tick);
        let rate = self.overall_progress / elapsed as f32;
        if rate <= 0.0 {
            return None;
        }
        let remaining = 1.0 - self.overall_progress;
        let estimated_remaining = (remaining / rate) as u64;
        Some(current_tick + estimated_remaining)
    }

    /// Check if on track to meet deadline.
    #[must_use]
    pub fn on_track(&self, current_tick: u64, start_tick: u64) -> Option<bool> {
        let estimated = self.estimated_completion(current_tick, start_tick)?;
        let deadline = self.deadline?;
        Some(estimated <= deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_state_properties() {
        assert!(MissionState::Completed.is_terminal());
        assert!(MissionState::Failed.is_terminal());
        assert!(MissionState::Expired.is_terminal());
        assert!(MissionState::Abandoned.is_terminal());
        assert!(!MissionState::Active.is_terminal());

        assert!(MissionState::Active.is_active());
        assert!(MissionState::Accepted.is_active());
        assert!(!MissionState::Completed.is_active());

        assert!(MissionState::Completed.is_successful());
        assert!(!MissionState::Failed.is_successful());
    }

    #[test]
    fn objective_state_properties() {
        assert!(ObjectiveState::Completed.is_terminal());
        assert!(ObjectiveState::Failed.is_terminal());
        assert!(ObjectiveState::Skipped.is_terminal());
        assert!(!ObjectiveState::InProgress.is_terminal());

        assert!(ObjectiveState::Completed.is_successful());
        assert!(!ObjectiveState::Failed.is_successful());
    }

    #[test]
    fn objective_progress_count() {
        let mut progress = ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false);

        assert_eq!(progress.current_count, 0);
        assert_eq!(progress.state, ObjectiveState::Pending);

        progress.add_progress(3, 100);
        assert_eq!(progress.current_count, 3);
        assert_eq!(progress.state, ObjectiveState::InProgress);
        assert_eq!(progress.started_at, Some(100));

        progress.add_progress(7, 200);
        assert_eq!(progress.current_count, 10);
        assert_eq!(progress.state, ObjectiveState::Completed);
        assert_eq!(progress.ended_at, Some(200));
    }

    #[test]
    fn objective_progress_timed() {
        let mut progress = ObjectiveProgress::new(ObjectiveId::new(0), 0, 1000, false);

        progress.add_elapsed(400, 100);
        assert_eq!(progress.elapsed_ticks, 400);
        assert_eq!(progress.state, ObjectiveState::InProgress);

        progress.add_elapsed(600, 200);
        assert_eq!(progress.elapsed_ticks, 1000);
        assert_eq!(progress.state, ObjectiveState::Completed);
    }

    #[test]
    fn objective_progress_fraction() {
        let mut progress = ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false);
        assert!((progress.progress_fraction() - 0.0).abs() < f32::EPSILON);

        progress.add_progress(5, 100);
        assert!((progress.progress_fraction() - 0.5).abs() < f32::EPSILON);

        progress.add_progress(5, 200);
        assert!((progress.progress_fraction() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn objective_progress_optional_skip() {
        let mut optional = ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, true);
        optional.skip(100);
        assert_eq!(optional.state, ObjectiveState::Skipped);

        let mut required = ObjectiveProgress::new(ObjectiveId::new(1), 10, 0, false);
        required.skip(100);
        assert_eq!(required.state, ObjectiveState::Pending);
    }

    #[test]
    fn mission_progress_lifecycle() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100).with_deadline(1000);

        assert_eq!(mission.state, MissionState::Accepted);

        mission.start(150);
        assert_eq!(mission.state, MissionState::Active);
        assert_eq!(mission.started_at, Some(150));

        mission.complete(500);
        assert_eq!(mission.state, MissionState::Completed);
        assert_eq!(mission.ended_at, Some(500));
    }

    #[test]
    fn mission_progress_deadline() {
        let mission = MissionProgress::new(MissionId::new(1), "test", 100).with_deadline(1000);

        assert!(!mission.is_past_deadline(500));
        assert!(mission.is_past_deadline(1000));
        assert_eq!(mission.time_until_deadline(500), Some(500));
    }

    #[test]
    fn mission_progress_extend_deadline() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100).with_deadline(1000);

        assert!(mission.extend_deadline(500));
        assert_eq!(mission.deadline, Some(1500));
        assert_eq!(mission.extensions_used, 1);
    }

    #[test]
    fn mission_progress_objectives() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100);
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false));
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(1), 5, 0, false));
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(2), 3, 0, true));

        assert_eq!(mission.required_objective_count(), 2);
        assert_eq!(mission.completed_required_count(), 0);
        assert!(!mission.all_required_complete());

        mission
            .objective_mut(ObjectiveId::new(0))
            .unwrap()
            .complete(200);
        mission
            .objective_mut(ObjectiveId::new(1))
            .unwrap()
            .complete(300);

        assert_eq!(mission.completed_required_count(), 2);
        assert!(mission.all_required_complete());
    }

    #[test]
    fn mission_progress_overall() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100);
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false));
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(1), 10, 0, false));

        mission
            .objective_mut(ObjectiveId::new(0))
            .unwrap()
            .add_progress(5, 100);
        assert!((mission.overall_progress() - 0.25).abs() < 0.01);

        mission
            .objective_mut(ObjectiveId::new(0))
            .unwrap()
            .add_progress(5, 200);
        assert!((mission.overall_progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn mission_progress_any_required_failed() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100);
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false));
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(1), 10, 0, false));

        assert!(!mission.any_required_failed());

        mission
            .objective_mut(ObjectiveId::new(0))
            .unwrap()
            .fail(200);
        assert!(mission.any_required_failed());
    }

    #[test]
    fn projection_summary() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100).with_deadline(1000);
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false));
        mission
            .objective_mut(ObjectiveId::new(0))
            .unwrap()
            .add_progress(5, 200);

        let summary = mission.projection_summary();
        assert_eq!(summary.mission_id, MissionId::new(1));
        assert!((summary.overall_progress - 0.5).abs() < 0.01);
        assert_eq!(summary.required_complete, 0);
        assert_eq!(summary.required_total, 1);
    }

    #[test]
    fn serde_mission_state() {
        let state = MissionState::Active;
        let json = serde_json::to_string(&state).unwrap();
        let recovered: MissionState = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, state);
    }

    #[test]
    fn serde_objective_progress() {
        let mut progress = ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false);
        progress.add_progress(5, 100);
        progress.set_state_data("custom", "value");

        let json = serde_json::to_string(&progress).unwrap();
        let recovered: ObjectiveProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, progress);
    }

    #[test]
    fn serde_mission_progress() {
        let mut mission = MissionProgress::new(MissionId::new(1), "test", 100)
            .with_deadline(1000)
            .with_region("north");
        mission.add_objective(ObjectiveProgress::new(ObjectiveId::new(0), 10, 0, false));

        let json = serde_json::to_string(&mission).unwrap();
        let recovered: MissionProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, mission);
    }
}

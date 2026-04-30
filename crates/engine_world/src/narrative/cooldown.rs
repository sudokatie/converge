//! Cooldown, repeat, and timed objective handling.

use serde::{Deserialize, Serialize};

/// How an event should repeat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepeatMode {
    /// Fire only once.
    #[default]
    Once,
    /// Repeat a fixed number of times.
    Count(u32),
    /// Repeat indefinitely.
    Forever,
    /// Repeat until a specific tick.
    UntilTick(u64),
}

impl RepeatMode {
    /// Check if more repeats are allowed given current count.
    #[must_use]
    pub fn allows_repeat(&self, current_count: u32, current_tick: u64) -> bool {
        match self {
            RepeatMode::Once => current_count == 0,
            RepeatMode::Count(max) => current_count < *max,
            RepeatMode::Forever => true,
            RepeatMode::UntilTick(end) => current_tick < *end,
        }
    }
}

/// Configuration for event cooldowns and repeating.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CooldownConfig {
    /// Minimum ticks between event firings.
    pub cooldown_ticks: u64,

    /// How the event should repeat.
    pub repeat_mode: RepeatMode,

    /// Jitter range added to cooldown (deterministic based on seed).
    pub jitter_ticks: u64,

    /// Whether cooldown is global (shared across instances) or per-trigger.
    pub global_cooldown: bool,
}

impl CooldownConfig {
    /// Create a one-shot config with no cooldown.
    #[must_use]
    pub const fn once() -> Self {
        Self {
            cooldown_ticks: 0,
            repeat_mode: RepeatMode::Once,
            jitter_ticks: 0,
            global_cooldown: false,
        }
    }

    /// Create a repeating config with cooldown.
    #[must_use]
    pub const fn repeating(cooldown_ticks: u64) -> Self {
        Self {
            cooldown_ticks,
            repeat_mode: RepeatMode::Forever,
            jitter_ticks: 0,
            global_cooldown: false,
        }
    }

    /// Create a config that repeats N times with cooldown.
    #[must_use]
    pub const fn repeat_count(count: u32, cooldown_ticks: u64) -> Self {
        Self {
            cooldown_ticks,
            repeat_mode: RepeatMode::Count(count),
            jitter_ticks: 0,
            global_cooldown: false,
        }
    }

    /// Set jitter range.
    #[must_use]
    pub const fn with_jitter(mut self, jitter_ticks: u64) -> Self {
        self.jitter_ticks = jitter_ticks;
        self
    }

    /// Set global cooldown.
    #[must_use]
    pub const fn with_global_cooldown(mut self, global: bool) -> Self {
        self.global_cooldown = global;
        self
    }

    /// Calculate effective cooldown with deterministic jitter.
    #[must_use]
    pub fn effective_cooldown(&self, seed: u64) -> u64 {
        if self.jitter_ticks == 0 {
            return self.cooldown_ticks;
        }
        let jitter_hash = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let jitter = jitter_hash % (self.jitter_ticks + 1);
        self.cooldown_ticks + jitter
    }
}

impl Default for CooldownConfig {
    fn default() -> Self {
        Self::once()
    }
}

/// Runtime state for a cooldown.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CooldownState {
    /// Last tick the event fired.
    pub last_fired_tick: Option<u64>,

    /// Number of times the event has fired.
    pub fire_count: u32,

    /// Next tick the event can fire (if on cooldown).
    pub ready_at_tick: u64,
}

impl CooldownState {
    /// Create a fresh cooldown state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_fired_tick: None,
            fire_count: 0,
            ready_at_tick: 0,
        }
    }

    /// Check if the cooldown is ready.
    #[must_use]
    pub const fn is_ready(&self, current_tick: u64) -> bool {
        current_tick >= self.ready_at_tick
    }

    /// Check if the event can fire given config and current state.
    #[must_use]
    pub fn can_fire(&self, config: &CooldownConfig, current_tick: u64) -> bool {
        self.is_ready(current_tick)
            && config
                .repeat_mode
                .allows_repeat(self.fire_count, current_tick)
    }

    /// Record a fire event and update cooldown.
    pub fn record_fire(&mut self, config: &CooldownConfig, current_tick: u64, seed: u64) {
        self.last_fired_tick = Some(current_tick);
        self.fire_count += 1;
        self.ready_at_tick = current_tick + config.effective_cooldown(seed);
    }

    /// Reset the cooldown state.
    pub fn reset(&mut self) {
        self.last_fired_tick = None;
        self.fire_count = 0;
        self.ready_at_tick = 0;
    }
}

/// Status of a timed objective.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectiveStatus {
    /// Objective not yet started.
    Pending,
    /// Objective in progress.
    Active,
    /// Objective completed successfully.
    Completed,
    /// Objective failed (deadline passed).
    Failed,
    /// Objective canceled.
    Canceled,
}

impl ObjectiveStatus {
    /// Check if the objective is still actionable.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, ObjectiveStatus::Pending | ObjectiveStatus::Active)
    }

    /// Check if the objective is resolved.
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        matches!(
            self,
            ObjectiveStatus::Completed | ObjectiveStatus::Failed | ObjectiveStatus::Canceled
        )
    }
}

/// A timed objective with deadline tracking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedObjective {
    /// Unique identifier for the objective.
    pub id: String,

    /// Display title.
    pub title: String,

    /// Optional description.
    pub description: Option<String>,

    /// Tick when the objective started.
    pub start_tick: u64,

    /// Tick when the objective expires (deadline).
    pub deadline_tick: u64,

    /// Current status.
    pub status: ObjectiveStatus,

    /// Progress value (0-100).
    pub progress: u8,

    /// Optional completion target (e.g., collect 10 items).
    pub target_count: Option<u32>,

    /// Current count toward target.
    pub current_count: u32,
}

impl TimedObjective {
    /// Create a new timed objective.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        start_tick: u64,
        duration: u64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            start_tick,
            deadline_tick: start_tick + duration,
            status: ObjectiveStatus::Pending,
            progress: 0,
            target_count: None,
            current_count: 0,
        }
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set target count.
    #[must_use]
    pub fn with_target(mut self, target: u32) -> Self {
        self.target_count = Some(target);
        self
    }

    /// Get remaining ticks until deadline.
    #[must_use]
    pub fn remaining_ticks(&self, current_tick: u64) -> u64 {
        self.deadline_tick.saturating_sub(current_tick)
    }

    /// Get elapsed ticks since start.
    #[must_use]
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.start_tick)
    }

    /// Get progress as percentage of time elapsed.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn time_progress(&self, current_tick: u64) -> f32 {
        let total = self.deadline_tick.saturating_sub(self.start_tick);
        if total == 0 {
            return 1.0;
        }
        let elapsed = self.elapsed_ticks(current_tick);
        (elapsed as f32 / total as f32).min(1.0)
    }

    /// Activate the objective.
    pub fn activate(&mut self) {
        if self.status == ObjectiveStatus::Pending {
            self.status = ObjectiveStatus::Active;
        }
    }

    /// Increment progress count.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn increment(&mut self, amount: u32) {
        self.current_count = self.current_count.saturating_add(amount);
        if let Some(target) = self.target_count {
            self.progress = ((self.current_count as f32 / target as f32) * 100.0).min(100.0) as u8;
            if self.current_count >= target {
                self.status = ObjectiveStatus::Completed;
            }
        }
    }

    /// Mark as completed.
    pub fn complete(&mut self) {
        self.status = ObjectiveStatus::Completed;
        self.progress = 100;
    }

    /// Mark as failed.
    pub fn fail(&mut self) {
        self.status = ObjectiveStatus::Failed;
    }

    /// Mark as canceled.
    pub fn cancel(&mut self) {
        self.status = ObjectiveStatus::Canceled;
    }

    /// Update based on current tick (check deadline).
    pub fn update(&mut self, current_tick: u64) {
        if self.status == ObjectiveStatus::Active && current_tick >= self.deadline_tick {
            self.status = ObjectiveStatus::Failed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_mode_once() {
        let mode = RepeatMode::Once;
        assert!(mode.allows_repeat(0, 0));
        assert!(!mode.allows_repeat(1, 0));
    }

    #[test]
    fn repeat_mode_count() {
        let mode = RepeatMode::Count(3);
        assert!(mode.allows_repeat(0, 0));
        assert!(mode.allows_repeat(2, 0));
        assert!(!mode.allows_repeat(3, 0));
    }

    #[test]
    fn repeat_mode_forever() {
        let mode = RepeatMode::Forever;
        assert!(mode.allows_repeat(0, 0));
        assert!(mode.allows_repeat(1000, 0));
    }

    #[test]
    fn repeat_mode_until_tick() {
        let mode = RepeatMode::UntilTick(100);
        assert!(mode.allows_repeat(0, 50));
        assert!(!mode.allows_repeat(0, 100));
    }

    #[test]
    fn cooldown_once() {
        let config = CooldownConfig::once();
        let mut state = CooldownState::new();

        assert!(state.can_fire(&config, 0));
        state.record_fire(&config, 0, 0);
        assert!(!state.can_fire(&config, 1));
    }

    #[test]
    fn cooldown_repeating() {
        let config = CooldownConfig::repeating(100);
        let mut state = CooldownState::new();

        assert!(state.can_fire(&config, 0));
        state.record_fire(&config, 0, 0);
        assert!(!state.can_fire(&config, 50));
        assert!(state.can_fire(&config, 100));
    }

    #[test]
    fn cooldown_jitter_deterministic() {
        let config = CooldownConfig::repeating(100).with_jitter(50);

        let eff1 = config.effective_cooldown(12345);
        let eff2 = config.effective_cooldown(12345);
        assert_eq!(eff1, eff2);

        let eff3 = config.effective_cooldown(54321);
        assert!((100..=150).contains(&eff1));
        assert!((100..=150).contains(&eff3));
    }

    #[test]
    fn cooldown_state_reset() {
        let config = CooldownConfig::once();
        let mut state = CooldownState::new();

        state.record_fire(&config, 0, 0);
        assert_eq!(state.fire_count, 1);

        state.reset();
        assert_eq!(state.fire_count, 0);
        assert!(state.can_fire(&config, 0));
    }

    #[test]
    fn objective_status_classification() {
        assert!(ObjectiveStatus::Pending.is_active());
        assert!(ObjectiveStatus::Active.is_active());
        assert!(!ObjectiveStatus::Completed.is_active());

        assert!(ObjectiveStatus::Completed.is_resolved());
        assert!(ObjectiveStatus::Failed.is_resolved());
        assert!(!ObjectiveStatus::Pending.is_resolved());
    }

    #[test]
    fn timed_objective_lifecycle() {
        let mut obj = TimedObjective::new("test", "Test Objective", 100, 500);

        assert_eq!(obj.status, ObjectiveStatus::Pending);
        assert_eq!(obj.remaining_ticks(100), 500);

        obj.activate();
        assert_eq!(obj.status, ObjectiveStatus::Active);

        obj.update(400);
        assert_eq!(obj.status, ObjectiveStatus::Active);

        obj.update(600);
        assert_eq!(obj.status, ObjectiveStatus::Failed);
    }

    #[test]
    fn timed_objective_with_target() {
        let mut obj = TimedObjective::new("collect", "Collect Items", 0, 1000).with_target(10);

        obj.activate();
        obj.increment(5);
        assert_eq!(obj.progress, 50);
        assert_eq!(obj.status, ObjectiveStatus::Active);

        obj.increment(5);
        assert_eq!(obj.progress, 100);
        assert_eq!(obj.status, ObjectiveStatus::Completed);
    }

    #[test]
    fn timed_objective_time_progress() {
        let obj = TimedObjective::new("test", "Test", 100, 100);
        assert!((obj.time_progress(100) - 0.0).abs() < 0.01);
        assert!((obj.time_progress(150) - 0.5).abs() < 0.01);
        assert!((obj.time_progress(200) - 1.0).abs() < 0.01);
    }

    #[test]
    fn serde_round_trip() {
        let config = CooldownConfig::repeating(100).with_jitter(50);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: CooldownConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, config);

        let obj = TimedObjective::new("test", "Test", 0, 100).with_target(5);
        let json = serde_json::to_string(&obj).unwrap();
        let recovered: TimedObjective = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, obj);
    }
}

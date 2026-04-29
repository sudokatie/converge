//! Per-region simulation state tracking.

use super::Fidelity;
use super::interest::{InterestConfig, InterestSummary, RegionInterest};

/// Tracking state for a single region in the scheduler.
///
/// Accumulates elapsed time since last simulation and tracks
/// the current fidelity assignment for priority ordering. Optionally
/// tracks interest-based relevance for field/hazard activity.
#[derive(Clone, Debug)]
pub struct RegionState {
    /// Current fidelity level based on observer distance.
    fidelity: Fidelity,
    /// Accumulated time since last simulation (seconds).
    accumulated: f32,
    /// Distance to nearest observer (Chebyshev, in chunks).
    distance: i32,
    /// Whether environmental field simulation is active.
    environment_active: bool,
    /// User-defined priority boost (added to base priority).
    priority_boost: i32,
    /// Interest-based relevance tracking (optional, lazily allocated).
    interest: Option<Box<RegionInterest>>,
}

impl RegionState {
    /// Create a new region state with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fidelity: Fidelity::Dormant,
            accumulated: 0.0,
            distance: i32::MAX,
            environment_active: false,
            priority_boost: 0,
            interest: None,
        }
    }

    /// Get the current fidelity level.
    #[must_use]
    pub fn fidelity(&self) -> Fidelity {
        self.fidelity
    }

    /// Get the accumulated time since last simulation.
    #[must_use]
    pub fn accumulated(&self) -> f32 {
        self.accumulated
    }

    /// Get the distance to nearest observer.
    #[must_use]
    pub fn distance(&self) -> i32 {
        self.distance
    }

    /// Check if environmental field simulation is active.
    #[must_use]
    pub fn environment_active(&self) -> bool {
        self.environment_active
    }

    /// Get the priority boost value.
    #[must_use]
    pub fn priority_boost(&self) -> i32 {
        self.priority_boost
    }

    /// Set the fidelity level.
    pub fn set_fidelity(&mut self, fidelity: Fidelity) {
        self.fidelity = fidelity;
    }

    /// Set the distance to nearest observer.
    pub fn set_distance(&mut self, distance: i32) {
        self.distance = distance;
    }

    /// Enable or disable environmental field simulation hooks.
    pub fn set_environment_active(&mut self, active: bool) {
        self.environment_active = active;
    }

    /// Set the priority boost (can be negative).
    pub fn set_priority_boost(&mut self, boost: i32) {
        self.priority_boost = boost;
    }

    /// Accumulate elapsed time.
    pub fn accumulate(&mut self, dt: f32) {
        self.accumulated += dt;
    }

    /// Check if accumulated time exceeds the given interval.
    #[must_use]
    pub fn is_ready(&self, interval: f32) -> bool {
        self.accumulated >= interval
    }

    /// Reset accumulated time after simulation.
    /// Returns the accumulated time that was consumed.
    pub fn consume(&mut self) -> f32 {
        let consumed = self.accumulated;
        self.accumulated = 0.0;
        consumed
    }

    /// Reset accumulated time, keeping overflow beyond the interval.
    /// Returns the consumed time (capped at interval).
    pub fn consume_interval(&mut self, interval: f32) -> f32 {
        if self.accumulated >= interval {
            self.accumulated -= interval;
            interval
        } else {
            let consumed = self.accumulated;
            self.accumulated = 0.0;
            consumed
        }
    }

    /// Compute the effective priority for job ordering.
    /// Higher values = higher priority.
    /// Combines fidelity priority, distance (inverted), and boost.
    #[must_use]
    pub fn effective_priority(&self) -> i64 {
        let fidelity_component = i64::from(self.fidelity.priority()) * 1_000_000;
        let distance_component = i64::from(i32::MAX - self.distance.max(0));
        let boost_component = i64::from(self.priority_boost) * 10_000;
        fidelity_component + distance_component + boost_component
    }

    /// Compute the effective priority including interest-based boost.
    #[must_use]
    pub fn effective_priority_with_interest(&self, config: &InterestConfig) -> i64 {
        let base = self.effective_priority();
        let interest_boost = self
            .interest_score()
            .map_or(0, |score| i64::from(config.priority_boost(score)));
        base + interest_boost
    }

    /// Get the interest tracking, if any.
    #[must_use]
    pub fn interest(&self) -> Option<&RegionInterest> {
        self.interest.as_deref()
    }

    /// Get mutable access to interest tracking, creating it if needed.
    pub fn interest_mut(&mut self) -> &mut RegionInterest {
        self.interest
            .get_or_insert_with(|| Box::new(RegionInterest::new()))
    }

    /// Get optional mutable access to interest tracking without creating.
    pub(crate) fn interest_option_mut(&mut self) -> Option<&mut RegionInterest> {
        self.interest.as_deref_mut()
    }

    /// Check if this region has any interest tracked.
    #[must_use]
    pub fn has_interest(&self) -> bool {
        self.interest.as_ref().is_some_and(|i| !i.is_empty())
    }

    /// Get the total interest score, or None if no interest is tracked.
    #[must_use]
    pub fn interest_score(&self) -> Option<f32> {
        self.interest.as_ref().and_then(|i| {
            let score = i.total_score();
            if score > 0.0 { Some(score) } else { None }
        })
    }

    /// Get a summary of interest state.
    #[must_use]
    pub fn interest_summary(&self) -> InterestSummary {
        self.interest
            .as_ref()
            .map_or_else(InterestSummary::default, |i| {
                InterestSummary::from_interest(i)
            })
    }

    /// Clear all interest tracking.
    pub fn clear_interest(&mut self) {
        if let Some(interest) = &mut self.interest {
            interest.clear();
        }
    }
}

impl Default for RegionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::HazardKind;
    use crate::scheduler::interest::InterestCategory;

    #[test]
    fn new_state_defaults() {
        let state = RegionState::new();
        assert_eq!(state.fidelity(), Fidelity::Dormant);
        assert!(state.accumulated().abs() < f32::EPSILON);
        assert_eq!(state.distance(), i32::MAX);
        assert!(!state.environment_active());
        assert_eq!(state.priority_boost(), 0);
        assert!(!state.has_interest());
    }

    #[test]
    fn accumulate_time() {
        let mut state = RegionState::new();
        state.accumulate(0.1);
        assert!((state.accumulated() - 0.1).abs() < 0.0001);
        state.accumulate(0.2);
        assert!((state.accumulated() - 0.3).abs() < 0.0001);
    }

    #[test]
    fn is_ready_check() {
        let mut state = RegionState::new();
        assert!(state.is_ready(0.0));
        assert!(!state.is_ready(0.1));

        state.accumulate(0.15);
        assert!(state.is_ready(0.1));
        assert!(!state.is_ready(0.2));
    }

    #[test]
    fn consume_resets() {
        let mut state = RegionState::new();
        state.accumulate(0.5);
        let consumed = state.consume();
        assert!((consumed - 0.5).abs() < 0.0001);
        assert!(state.accumulated().abs() < f32::EPSILON);
    }

    #[test]
    fn consume_interval_keeps_overflow() {
        let mut state = RegionState::new();
        state.accumulate(0.35);

        let consumed = state.consume_interval(0.1);
        assert!((consumed - 0.1).abs() < 0.0001);
        assert!((state.accumulated() - 0.25).abs() < 0.0001);

        let consumed = state.consume_interval(0.1);
        assert!((consumed - 0.1).abs() < 0.0001);
        assert!((state.accumulated() - 0.15).abs() < 0.0001);
    }

    #[test]
    fn consume_interval_partial() {
        let mut state = RegionState::new();
        state.accumulate(0.05);

        let consumed = state.consume_interval(0.1);
        assert!((consumed - 0.05).abs() < 0.0001);
        assert!(state.accumulated().abs() < f32::EPSILON);
    }

    #[test]
    fn effective_priority_fidelity_dominates() {
        let mut immediate = RegionState::new();
        immediate.set_fidelity(Fidelity::Immediate);
        immediate.set_distance(100);

        let mut dormant = RegionState::new();
        dormant.set_fidelity(Fidelity::Dormant);
        dormant.set_distance(1);

        assert!(immediate.effective_priority() > dormant.effective_priority());
    }

    #[test]
    fn effective_priority_distance_tiebreaker() {
        let mut close = RegionState::new();
        close.set_fidelity(Fidelity::Near);
        close.set_distance(3);

        let mut far = RegionState::new();
        far.set_fidelity(Fidelity::Near);
        far.set_distance(6);

        assert!(close.effective_priority() > far.effective_priority());
    }

    #[test]
    fn effective_priority_boost() {
        let mut normal = RegionState::new();
        normal.set_fidelity(Fidelity::Distant);
        normal.set_distance(10);

        let mut boosted = RegionState::new();
        boosted.set_fidelity(Fidelity::Distant);
        boosted.set_distance(10);
        boosted.set_priority_boost(50);

        assert!(boosted.effective_priority() > normal.effective_priority());
    }

    #[test]
    fn environment_active_toggle() {
        let mut state = RegionState::new();
        assert!(!state.environment_active());

        state.set_environment_active(true);
        assert!(state.environment_active());

        state.set_environment_active(false);
        assert!(!state.environment_active());
    }

    #[test]
    fn interest_initially_none() {
        let state = RegionState::new();
        assert!(state.interest().is_none());
        assert!(!state.has_interest());
        assert!(state.interest_score().is_none());
    }

    #[test]
    fn interest_mut_creates_tracking() {
        let mut state = RegionState::new();
        let interest = state.interest_mut();
        interest.set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);

        assert!(state.has_interest());
        assert!(state.interest().is_some());
    }

    #[test]
    fn interest_score_calculation() {
        let mut state = RegionState::new();
        state
            .interest_mut()
            .set(InterestCategory::Hazard(HazardKind::Fire), 0.5, 0);
        state
            .interest_mut()
            .set(InterestCategory::Structural, 0.3, 0);

        let score = state.interest_score().unwrap();
        assert!((score - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn interest_priority_boost() {
        let mut state = RegionState::new();
        state.set_fidelity(Fidelity::Distant);
        state.set_distance(50);
        state
            .interest_mut()
            .set(InterestCategory::Hazard(HazardKind::Fire), 1.0, 0);

        let config = InterestConfig::default();
        let base_priority = state.effective_priority();
        let with_interest = state.effective_priority_with_interest(&config);

        assert!(with_interest > base_priority);
        assert_eq!(with_interest - base_priority, 50_000);
    }

    #[test]
    fn interest_summary_reflects_state() {
        let mut state = RegionState::new();
        state
            .interest_mut()
            .set(InterestCategory::Hazard(HazardKind::Fire), 0.8, 0);

        let summary = state.interest_summary();
        assert!(summary.is_active());
        assert!(summary.has_hazards);
        assert!(!summary.has_scalar_fields);
    }

    #[test]
    fn clear_interest() {
        let mut state = RegionState::new();
        state
            .interest_mut()
            .set(InterestCategory::Structural, 0.5, 0);
        assert!(state.has_interest());

        state.clear_interest();
        assert!(!state.has_interest());
    }
}

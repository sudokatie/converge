//! Core simulation scheduler implementation.

use std::collections::HashMap;

use engine_core::coords::ChunkPos;

use super::{
    config::SchedulerConfig,
    fidelity::Fidelity,
    interest::{InterestCategory, InterestSummary, RegionInterest},
    job::EnvironmentHint,
    job::SimulationJob,
    state::RegionState,
};

/// Simulation scheduler managing region fidelity and tick timing.
///
/// The scheduler tracks regions, assigns fidelity levels based on
/// observer distance, accumulates time, and produces prioritized
/// batches of simulation jobs. Interest-based relevance can augment
/// distance-based fidelity for regions with active hazards or fields.
#[derive(Debug)]
pub struct SimulationScheduler {
    /// Configuration for intervals, thresholds, and budget.
    config: SchedulerConfig,
    /// Per-region tracking state.
    regions: HashMap<ChunkPos, RegionState>,
    /// Observer positions (typically player chunk positions).
    observers: Vec<ChunkPos>,
    /// Current tick counter for staleness tracking.
    current_tick: u64,
}

impl SimulationScheduler {
    /// Create a new scheduler with the given configuration.
    #[must_use]
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            regions: HashMap::new(),
            observers: Vec::new(),
            current_tick: 0,
        }
    }

    /// Get the current tick counter.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the current configuration.
    #[must_use]
    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut SchedulerConfig {
        &mut self.config
    }

    /// Get the number of tracked regions.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get the number of observers.
    #[must_use]
    pub fn observer_count(&self) -> usize {
        self.observers.len()
    }

    /// Add a region to the scheduler.
    /// Returns true if the region was newly added.
    pub fn add_region(&mut self, pos: ChunkPos) -> bool {
        if self.regions.contains_key(&pos) {
            return false;
        }

        let mut state = RegionState::new();
        self.update_region_fidelity(pos, &mut state);
        self.regions.insert(pos, state);
        true
    }

    /// Remove a region from the scheduler.
    /// Returns true if the region was present.
    pub fn remove_region(&mut self, pos: ChunkPos) -> bool {
        self.regions.remove(&pos).is_some()
    }

    /// Check if a region is tracked.
    #[must_use]
    pub fn has_region(&self, pos: ChunkPos) -> bool {
        self.regions.contains_key(&pos)
    }

    /// Get the state of a region.
    #[must_use]
    pub fn get_state(&self, pos: ChunkPos) -> Option<&RegionState> {
        self.regions.get(&pos)
    }

    /// Get mutable access to a region's state.
    pub fn get_state_mut(&mut self, pos: ChunkPos) -> Option<&mut RegionState> {
        self.regions.get_mut(&pos)
    }

    /// Set a single observer position.
    /// Clears any existing observers.
    pub fn set_observer(&mut self, pos: ChunkPos) {
        self.observers.clear();
        self.observers.push(pos);
        self.update_all_fidelities();
    }

    /// Set multiple observer positions.
    pub fn set_observers(&mut self, positions: impl IntoIterator<Item = ChunkPos>) {
        self.observers.clear();
        self.observers.extend(positions);
        self.update_all_fidelities();
    }

    /// Add an observer position.
    pub fn add_observer(&mut self, pos: ChunkPos) {
        self.observers.push(pos);
        self.update_all_fidelities();
    }

    /// Clear all observers.
    pub fn clear_observers(&mut self) {
        self.observers.clear();
        self.update_all_fidelities();
    }

    /// Get current observer positions.
    #[must_use]
    pub fn observers(&self) -> &[ChunkPos] {
        &self.observers
    }

    /// Mark a region as having active environmental fields.
    pub fn set_environment_active(&mut self, pos: ChunkPos, active: bool) {
        if let Some(state) = self.regions.get_mut(&pos) {
            state.set_environment_active(active);
        }
    }

    /// Set priority boost for a region.
    pub fn set_priority_boost(&mut self, pos: ChunkPos, boost: i32) {
        if let Some(state) = self.regions.get_mut(&pos) {
            state.set_priority_boost(boost);
        }
    }

    /// Advance time and return simulation jobs ready to execute.
    ///
    /// Accumulates `dt` seconds for all regions, applies interest decay,
    /// then returns up to `max_jobs_per_tick` jobs sorted by priority.
    /// Interest-based relevance affects priority ordering and environment hints.
    #[must_use]
    pub fn tick(&mut self, dt: f32) -> Vec<SimulationJob> {
        self.current_tick += 1;

        // Accumulate time for all regions
        for state in self.regions.values_mut() {
            state.accumulate(dt);
        }

        // Apply decay if configured
        if self.config.interest.decay_factor < 1.0 {
            for state in self.regions.values_mut() {
                if let Some(interest) = state.interest_option_mut() {
                    interest.decay_all(self.config.interest.decay_factor);
                }
            }
        }

        // Prune stale interest entries
        let max_staleness = self.config.interest.max_staleness_ticks;
        let current_tick = self.current_tick;
        for state in self.regions.values_mut() {
            if let Some(interest) = state.interest_option_mut() {
                interest.prune_stale(current_tick, max_staleness);
            }
        }

        // Collect ready jobs
        let mut jobs = Vec::new();
        for (&pos, state) in &self.regions {
            let effective_fidelity = self.effective_fidelity(state);
            let interval = self.config.intervals.get(effective_fidelity);
            if state.is_ready(interval) {
                let environment = self.compute_environment_hint(state, effective_fidelity);
                let priority = state.effective_priority_with_interest(&self.config.interest);

                jobs.push(SimulationJob::new(
                    pos,
                    effective_fidelity,
                    state.accumulated(),
                    state.distance(),
                    environment,
                    priority,
                ));
            }
        }

        // Sort by priority (highest first)
        jobs.sort();

        // Apply budget limit
        if jobs.len() > self.config.max_jobs_per_tick {
            jobs.truncate(self.config.max_jobs_per_tick);
        }

        // Consume time for returned jobs (use job's already-computed fidelity)
        for job in &jobs {
            if let Some(state) = self.regions.get_mut(&job.position()) {
                let interval = self.config.intervals.get(job.fidelity());
                state.consume_interval(interval);
            }
        }

        jobs
    }

    /// Compute effective fidelity, potentially promoted by interest.
    fn effective_fidelity(&self, state: &RegionState) -> Fidelity {
        if !self.config.interest.can_promote_dormant {
            return state.fidelity();
        }

        if state.fidelity() == Fidelity::Dormant && state.has_interest() {
            Fidelity::Distant
        } else {
            state.fidelity()
        }
    }

    /// Compute environment hints based on fidelity and interest.
    fn compute_environment_hint(&self, state: &RegionState, fidelity: Fidelity) -> EnvironmentHint {
        if !state.environment_active() {
            return EnvironmentHint::NONE;
        }

        let base = environment_hint_for_fidelity(fidelity);
        let summary = state.interest_summary();

        EnvironmentHint {
            scalar_fields: base.scalar_fields
                || (summary.has_scalar_fields
                    && summary.total_score >= self.config.interest.scalar_field_threshold),
            vector_fields: base.vector_fields
                || (summary.has_vector_fields
                    && summary.total_score >= self.config.interest.vector_field_threshold),
            hazard_spread: base.hazard_spread
                || (summary.has_hazards
                    && summary.total_score >= self.config.interest.hazard_threshold),
        }
    }

    /// Get the fidelity assigned to a region, if tracked.
    #[must_use]
    pub fn get_fidelity(&self, pos: ChunkPos) -> Option<Fidelity> {
        self.regions.get(&pos).map(RegionState::fidelity)
    }

    /// Calculate the minimum Chebyshev distance from any observer.
    fn distance_to_observers(&self, pos: ChunkPos) -> i32 {
        if self.observers.is_empty() {
            return i32::MAX;
        }

        self.observers
            .iter()
            .map(|obs| pos.chebyshev_distance(*obs))
            .min()
            .unwrap_or(i32::MAX)
    }

    /// Update fidelity for a single region based on observer distance.
    fn update_region_fidelity(&self, pos: ChunkPos, state: &mut RegionState) {
        let distance = self.distance_to_observers(pos);
        let fidelity = self.config.thresholds.fidelity_for_distance(distance);
        state.set_distance(distance);
        state.set_fidelity(fidelity);
    }

    /// Update fidelity for all regions.
    fn update_all_fidelities(&mut self) {
        let positions: Vec<ChunkPos> = self.regions.keys().copied().collect();
        for pos in positions {
            let distance = self.distance_to_observers(pos);
            let fidelity = self.config.thresholds.fidelity_for_distance(distance);
            if let Some(state) = self.regions.get_mut(&pos) {
                state.set_distance(distance);
                state.set_fidelity(fidelity);
            }
        }
    }

    /// Count regions at each fidelity level.
    #[must_use]
    pub fn fidelity_counts(&self) -> [usize; Fidelity::COUNT] {
        let mut counts = [0usize; Fidelity::COUNT];
        for state in self.regions.values() {
            counts[state.fidelity().as_index()] += 1;
        }
        counts
    }

    /// Iterate over all regions with their states.
    pub fn iter_regions(&self) -> impl Iterator<Item = (ChunkPos, &RegionState)> {
        self.regions.iter().map(|(&pos, state)| (pos, state))
    }

    // ==================== Interest Management ====================

    /// Set interest for a region with a specific category and weight.
    ///
    /// Creates interest tracking for the region if needed.
    /// Setting weight to 0 or negative removes that interest entry.
    pub fn set_interest(&mut self, pos: ChunkPos, category: InterestCategory, weight: f32) {
        if let Some(state) = self.regions.get_mut(&pos) {
            state
                .interest_mut()
                .set(category, weight, self.current_tick);
        }
    }

    /// Remove a specific interest category from a region.
    pub fn remove_interest(&mut self, pos: ChunkPos, category: InterestCategory) -> bool {
        if let Some(state) = self.regions.get_mut(&pos) {
            state.interest_mut().remove(category)
        } else {
            false
        }
    }

    /// Clear all interest for a region.
    pub fn clear_region_interest(&mut self, pos: ChunkPos) {
        if let Some(state) = self.regions.get_mut(&pos) {
            state.clear_interest();
        }
    }

    /// Clear all interest for all regions.
    pub fn clear_all_interest(&mut self) {
        for state in self.regions.values_mut() {
            state.clear_interest();
        }
    }

    /// Get read-only access to a region's interest tracking.
    #[must_use]
    pub fn get_interest(&self, pos: ChunkPos) -> Option<&RegionInterest> {
        self.regions.get(&pos).and_then(|s| s.interest())
    }

    /// Get an interest summary for a region.
    #[must_use]
    pub fn interest_summary(&self, pos: ChunkPos) -> InterestSummary {
        self.regions
            .get(&pos)
            .map_or_else(InterestSummary::default, RegionState::interest_summary)
    }

    /// Check if a region has any active interest.
    #[must_use]
    pub fn has_interest(&self, pos: ChunkPos) -> bool {
        self.regions
            .get(&pos)
            .is_some_and(RegionState::has_interest)
    }

    /// Get the interest score for a region, if any.
    #[must_use]
    pub fn interest_score(&self, pos: ChunkPos) -> Option<f32> {
        self.regions.get(&pos).and_then(RegionState::interest_score)
    }

    /// Query all regions with interest above a threshold.
    ///
    /// Returns positions sorted deterministically (by coordinates).
    #[must_use]
    pub fn regions_with_interest(&self, min_score: f32) -> Vec<ChunkPos> {
        let mut result: Vec<_> = self
            .regions
            .iter()
            .filter(|(_, state)| state.interest_score().is_some_and(|s| s >= min_score))
            .map(|(&pos, _)| pos)
            .collect();
        result.sort_by_key(|p| (p.x(), p.y(), p.z()));
        result
    }

    /// Count regions with active interest by category.
    #[must_use]
    pub fn interest_counts(&self) -> InterestCounts {
        let mut counts = InterestCounts::default();
        for state in self.regions.values() {
            let summary = state.interest_summary();
            if summary.is_active() {
                counts.total += 1;
                if summary.has_scalar_fields {
                    counts.with_scalar_fields += 1;
                }
                if summary.has_vector_fields {
                    counts.with_vector_fields += 1;
                }
                if summary.has_hazards {
                    counts.with_hazards += 1;
                }
            }
        }
        counts
    }

    /// Manually trigger interest decay for all regions.
    ///
    /// Useful when you want to decay without advancing the tick counter.
    pub fn decay_all_interest(&mut self, factor: f32) {
        for state in self.regions.values_mut() {
            if let Some(interest) = state.interest_option_mut() {
                interest.decay_all(factor);
            }
        }
    }

    /// Manually prune stale interest entries from all regions.
    ///
    /// Returns the total number of entries removed.
    pub fn prune_stale_interest(&mut self, max_staleness: u64) -> usize {
        let current_tick = self.current_tick;
        let mut total_removed = 0;
        for state in self.regions.values_mut() {
            if let Some(interest) = state.interest_option_mut() {
                total_removed += interest.prune_stale(current_tick, max_staleness);
            }
        }
        total_removed
    }
}

/// Summary counts of interest across all regions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InterestCounts {
    /// Total regions with active interest.
    pub total: usize,
    /// Regions with scalar field interest.
    pub with_scalar_fields: usize,
    /// Regions with vector field interest.
    pub with_vector_fields: usize,
    /// Regions with hazard interest.
    pub with_hazards: usize,
}

/// Determine environment hints based on fidelity level.
fn environment_hint_for_fidelity(fidelity: Fidelity) -> EnvironmentHint {
    match fidelity {
        Fidelity::Immediate | Fidelity::Near => EnvironmentHint::FULL,
        Fidelity::Distant => EnvironmentHint::SCALAR_ONLY,
        Fidelity::Dormant => EnvironmentHint::NONE,
    }
}

impl Default for SimulationScheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_scheduler() -> SimulationScheduler {
        SimulationScheduler::new(SchedulerConfig::default())
    }

    #[test]
    fn new_scheduler_empty() {
        let scheduler = default_scheduler();
        assert_eq!(scheduler.region_count(), 0);
        assert_eq!(scheduler.observer_count(), 0);
    }

    #[test]
    fn add_remove_regions() {
        let mut scheduler = default_scheduler();

        assert!(scheduler.add_region(ChunkPos::new(0, 0, 0)));
        assert!(!scheduler.add_region(ChunkPos::new(0, 0, 0))); // duplicate
        assert_eq!(scheduler.region_count(), 1);

        assert!(scheduler.add_region(ChunkPos::new(1, 0, 0)));
        assert_eq!(scheduler.region_count(), 2);

        assert!(scheduler.remove_region(ChunkPos::new(0, 0, 0)));
        assert!(!scheduler.remove_region(ChunkPos::new(0, 0, 0))); // already removed
        assert_eq!(scheduler.region_count(), 1);
    }

    #[test]
    fn has_region() {
        let mut scheduler = default_scheduler();
        let pos = ChunkPos::new(5, 5, 5);

        assert!(!scheduler.has_region(pos));
        scheduler.add_region(pos);
        assert!(scheduler.has_region(pos));
    }

    #[test]
    fn set_observer_updates_fidelity() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0));
        scheduler.add_region(ChunkPos::new(10, 0, 0));

        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        assert_eq!(
            scheduler.get_fidelity(ChunkPos::new(0, 0, 0)),
            Some(Fidelity::Immediate)
        );
        assert_eq!(
            scheduler.get_fidelity(ChunkPos::new(10, 0, 0)),
            Some(Fidelity::Distant)
        );
    }

    #[test]
    fn multiple_observers() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0));
        scheduler.add_region(ChunkPos::new(20, 0, 0));

        scheduler.set_observers([ChunkPos::new(0, 0, 0), ChunkPos::new(20, 0, 0)]);

        // Both should be immediate to their respective observer
        assert_eq!(
            scheduler.get_fidelity(ChunkPos::new(0, 0, 0)),
            Some(Fidelity::Immediate)
        );
        assert_eq!(
            scheduler.get_fidelity(ChunkPos::new(20, 0, 0)),
            Some(Fidelity::Immediate)
        );
    }

    #[test]
    fn tick_accumulates_time() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0));
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // Immediate fidelity has 0 interval, should tick immediately
        let jobs = scheduler.tick(0.016);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn tick_respects_intervals() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(5, 0, 0)); // Near fidelity (0.1s interval)
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // First tick, not enough time
        let jobs = scheduler.tick(0.05);
        assert_eq!(jobs.len(), 0);

        // Second tick, now ready
        let jobs = scheduler.tick(0.06);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn tick_budget_limit() {
        let config = SchedulerConfig {
            max_jobs_per_tick: 2,
            ..Default::default()
        };

        let mut scheduler = SimulationScheduler::new(config);
        for i in 0..5 {
            scheduler.add_region(ChunkPos::new(i, 0, 0));
        }
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // All regions should be ready, but limited to 2
        let jobs = scheduler.tick(0.1);
        assert!(jobs.len() <= 2);
    }

    #[test]
    fn tick_priority_ordering() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0)); // Immediate
        scheduler.add_region(ChunkPos::new(5, 0, 0)); // Near
        scheduler.add_region(ChunkPos::new(10, 0, 0)); // Distant
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        let jobs = scheduler.tick(10.0); // Ensure all are ready

        // Should be ordered by fidelity priority
        assert!(jobs.len() >= 2);
        assert!(jobs[0].fidelity().is_at_least(jobs[1].fidelity()));
    }

    #[test]
    fn dormant_regions_tick_infrequently() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(100, 0, 0)); // Far away = Dormant
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // Default dormant interval is 2.0s
        let jobs = scheduler.tick(1.0);
        assert_eq!(jobs.len(), 0);

        let jobs = scheduler.tick(1.5);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn observer_movement_changes_fidelity() {
        let mut scheduler = default_scheduler();
        let region = ChunkPos::new(5, 0, 0);
        scheduler.add_region(region);

        // Start with observer far away
        scheduler.set_observer(ChunkPos::new(100, 0, 0));
        assert_eq!(scheduler.get_fidelity(region), Some(Fidelity::Dormant));

        // Move observer close
        scheduler.set_observer(ChunkPos::new(5, 0, 0));
        assert_eq!(scheduler.get_fidelity(region), Some(Fidelity::Immediate));

        // Move observer to medium distance
        scheduler.set_observer(ChunkPos::new(10, 0, 0));
        assert_eq!(scheduler.get_fidelity(region), Some(Fidelity::Near));
    }

    #[test]
    fn deterministic_job_ordering() {
        let mut scheduler = default_scheduler();

        // Add regions in random order
        scheduler.add_region(ChunkPos::new(3, 0, 0));
        scheduler.add_region(ChunkPos::new(1, 0, 0));
        scheduler.add_region(ChunkPos::new(2, 0, 0));
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        let jobs1 = scheduler.tick(10.0);
        let _ = scheduler.tick(10.0); // Reset accumulated time

        // Run again
        let mut scheduler2 = default_scheduler();
        scheduler2.add_region(ChunkPos::new(2, 0, 0));
        scheduler2.add_region(ChunkPos::new(3, 0, 0));
        scheduler2.add_region(ChunkPos::new(1, 0, 0));
        scheduler2.set_observer(ChunkPos::new(0, 0, 0));

        let jobs2 = scheduler2.tick(10.0);

        // Order should be deterministic
        assert_eq!(jobs1.len(), jobs2.len());
        for (j1, j2) in jobs1.iter().zip(jobs2.iter()) {
            assert_eq!(j1.position(), j2.position());
        }
    }

    #[test]
    fn fidelity_counts() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0)); // Immediate
        scheduler.add_region(ChunkPos::new(1, 0, 0)); // Immediate
        scheduler.add_region(ChunkPos::new(5, 0, 0)); // Near
        scheduler.add_region(ChunkPos::new(10, 0, 0)); // Distant
        scheduler.add_region(ChunkPos::new(100, 0, 0)); // Dormant
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        let counts = scheduler.fidelity_counts();
        assert_eq!(counts[Fidelity::Immediate.as_index()], 2);
        assert_eq!(counts[Fidelity::Near.as_index()], 1);
        assert_eq!(counts[Fidelity::Distant.as_index()], 1);
        assert_eq!(counts[Fidelity::Dormant.as_index()], 1);
    }

    #[test]
    fn environment_active_flag() {
        let mut scheduler = default_scheduler();
        let pos = ChunkPos::new(0, 0, 0);
        scheduler.add_region(pos);
        scheduler.set_observer(pos);

        // Default: environment not active
        let jobs = scheduler.tick(0.1);
        assert!(!jobs[0].environment().any_active());

        // Enable environment
        scheduler.set_environment_active(pos, true);
        let jobs = scheduler.tick(0.1);
        assert!(jobs[0].environment().any_active());
    }

    #[test]
    fn priority_boost() {
        let mut scheduler = default_scheduler();
        let pos1 = ChunkPos::new(0, 0, 0);
        let pos2 = ChunkPos::new(1, 0, 0);
        scheduler.add_region(pos1);
        scheduler.add_region(pos2);
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // Boost pos2 priority
        scheduler.set_priority_boost(pos2, 1000);

        let jobs = scheduler.tick(0.1);

        // pos2 should come first despite being farther
        assert_eq!(jobs[0].position(), pos2);
    }

    #[test]
    fn iter_regions() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0));
        scheduler.add_region(ChunkPos::new(1, 0, 0));

        let positions: Vec<_> = scheduler.iter_regions().map(|(pos, _)| pos).collect();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn consume_interval_preserves_overflow() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(5, 0, 0)); // Near fidelity
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // Accumulate 0.25s (interval is 0.1s)
        let _ = scheduler.tick(0.25);

        // After consuming 0.1s, should have 0.15s left
        let state = scheduler.get_state(ChunkPos::new(5, 0, 0)).unwrap();
        assert!((state.accumulated() - 0.15).abs() < 0.001);

        // Next tick with 0 dt should still be ready
        let jobs = scheduler.tick(0.0);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn no_observers_all_dormant() {
        let mut scheduler = default_scheduler();
        scheduler.add_region(ChunkPos::new(0, 0, 0));
        scheduler.add_region(ChunkPos::new(1, 0, 0));

        // No observers set
        assert_eq!(
            scheduler.get_fidelity(ChunkPos::new(0, 0, 0)),
            Some(Fidelity::Dormant)
        );
        assert_eq!(
            scheduler.get_fidelity(ChunkPos::new(1, 0, 0)),
            Some(Fidelity::Dormant)
        );
    }

    #[test]
    fn clear_observers() {
        let mut scheduler = default_scheduler();
        let pos = ChunkPos::new(0, 0, 0);
        scheduler.add_region(pos);

        scheduler.set_observer(pos);
        assert_eq!(scheduler.get_fidelity(pos), Some(Fidelity::Immediate));

        scheduler.clear_observers();
        assert_eq!(scheduler.get_fidelity(pos), Some(Fidelity::Dormant));
    }

    #[test]
    fn environment_hint_per_fidelity() {
        let mut scheduler = default_scheduler();

        for i in 0..20 {
            scheduler.add_region(ChunkPos::new(i, 0, 0));
        }
        scheduler.set_observer(ChunkPos::new(0, 0, 0));

        // Enable environment for all regions
        for i in 0..20 {
            scheduler.set_environment_active(ChunkPos::new(i, 0, 0), true);
        }

        let jobs = scheduler.tick(10.0);

        for job in jobs {
            match job.fidelity() {
                Fidelity::Immediate | Fidelity::Near => {
                    assert!(job.environment().scalar_fields);
                    assert!(job.environment().vector_fields);
                }
                Fidelity::Distant => {
                    assert!(job.environment().scalar_fields);
                    assert!(!job.environment().vector_fields);
                }
                Fidelity::Dormant => {
                    assert!(!job.environment().any_active());
                }
            }
        }
    }

    // ==================== Interest Management Tests ====================

    mod interest_tests {
        use super::*;
        use crate::environment::{FieldChannel, HazardKind, VectorFieldChannel};

        fn scheduler_with_interest_config() -> SimulationScheduler {
            let mut config = SchedulerConfig::default();
            config.interest.decay_factor = 1.0; // No decay by default in tests
            SimulationScheduler::new(config)
        }

        #[test]
        fn set_and_get_interest() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);

            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 0.8);

            assert!(scheduler.has_interest(pos));
            let score = scheduler.interest_score(pos).unwrap();
            assert!((score - 0.8).abs() < f32::EPSILON);
        }

        #[test]
        fn set_interest_on_untracked_region() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);

            scheduler.set_interest(pos, InterestCategory::Structural, 0.5);

            assert!(!scheduler.has_interest(pos));
        }

        #[test]
        fn remove_interest() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);

            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 0.8);
            assert!(scheduler.has_interest(pos));

            assert!(scheduler.remove_interest(pos, InterestCategory::Hazard(HazardKind::Fire)));
            assert!(!scheduler.has_interest(pos));
        }

        #[test]
        fn clear_region_interest() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);

            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 0.8);
            scheduler.set_interest(pos, InterestCategory::Structural, 0.5);

            scheduler.clear_region_interest(pos);
            assert!(!scheduler.has_interest(pos));
        }

        #[test]
        fn clear_all_interest() {
            let mut scheduler = scheduler_with_interest_config();
            scheduler.add_region(ChunkPos::new(0, 0, 0));
            scheduler.add_region(ChunkPos::new(1, 0, 0));

            scheduler.set_interest(
                ChunkPos::new(0, 0, 0),
                InterestCategory::Hazard(HazardKind::Fire),
                0.8,
            );
            scheduler.set_interest(ChunkPos::new(1, 0, 0), InterestCategory::Structural, 0.5);

            scheduler.clear_all_interest();

            assert!(!scheduler.has_interest(ChunkPos::new(0, 0, 0)));
            assert!(!scheduler.has_interest(ChunkPos::new(1, 0, 0)));
        }

        #[test]
        fn interest_summary() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);

            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 0.8);
            scheduler.set_interest(
                pos,
                InterestCategory::ScalarField(FieldChannel::Temperature),
                0.3,
            );

            let summary = scheduler.interest_summary(pos);
            assert!(summary.is_active());
            assert!(summary.has_hazards);
            assert!(summary.has_scalar_fields);
            assert!(!summary.has_vector_fields);
            assert!((summary.total_score - 1.1).abs() < f32::EPSILON);
        }

        #[test]
        fn regions_with_interest_deterministic() {
            let mut scheduler = scheduler_with_interest_config();

            // Add regions in random order
            scheduler.add_region(ChunkPos::new(5, 0, 0));
            scheduler.add_region(ChunkPos::new(1, 0, 0));
            scheduler.add_region(ChunkPos::new(3, 0, 0));
            scheduler.add_region(ChunkPos::new(2, 0, 0));

            scheduler.set_interest(
                ChunkPos::new(5, 0, 0),
                InterestCategory::Hazard(HazardKind::Fire),
                0.5,
            );
            scheduler.set_interest(ChunkPos::new(1, 0, 0), InterestCategory::Structural, 0.5);
            scheduler.set_interest(ChunkPos::new(3, 0, 0), InterestCategory::Fluid, 0.5);

            let regions = scheduler.regions_with_interest(0.1);

            assert_eq!(regions.len(), 3);
            assert_eq!(regions[0], ChunkPos::new(1, 0, 0));
            assert_eq!(regions[1], ChunkPos::new(3, 0, 0));
            assert_eq!(regions[2], ChunkPos::new(5, 0, 0));
        }

        #[test]
        fn interest_counts() {
            let mut scheduler = scheduler_with_interest_config();
            scheduler.add_region(ChunkPos::new(0, 0, 0));
            scheduler.add_region(ChunkPos::new(1, 0, 0));
            scheduler.add_region(ChunkPos::new(2, 0, 0));

            scheduler.set_interest(
                ChunkPos::new(0, 0, 0),
                InterestCategory::Hazard(HazardKind::Fire),
                0.5,
            );
            scheduler.set_interest(
                ChunkPos::new(1, 0, 0),
                InterestCategory::ScalarField(FieldChannel::Temperature),
                0.5,
            );
            scheduler.set_interest(
                ChunkPos::new(2, 0, 0),
                InterestCategory::VectorField(VectorFieldChannel::Wind),
                0.5,
            );

            let counts = scheduler.interest_counts();
            assert_eq!(counts.total, 3);
            assert_eq!(counts.with_hazards, 1);
            assert_eq!(counts.with_scalar_fields, 1);
            assert_eq!(counts.with_vector_fields, 1);
        }

        #[test]
        fn interest_boosts_priority() {
            let mut scheduler = scheduler_with_interest_config();
            // Two regions at same fidelity level (both Near)
            let near1 = ChunkPos::new(3, 0, 0);
            let near2 = ChunkPos::new(5, 0, 0);

            scheduler.add_region(near1);
            scheduler.add_region(near2);
            scheduler.set_observer(ChunkPos::new(0, 0, 0));
            scheduler.set_environment_active(near1, true);
            scheduler.set_environment_active(near2, true);

            // Give near2 (farther) high interest - should now come before near1
            scheduler.set_interest(near2, InterestCategory::Hazard(HazardKind::Fire), 2.0);

            let jobs = scheduler.tick(10.0);

            // near2 should appear before near1 due to interest boost overcoming distance
            let near1_idx = jobs.iter().position(|j| j.position() == near1).unwrap();
            let near2_idx = jobs.iter().position(|j| j.position() == near2).unwrap();
            assert!(near2_idx < near1_idx);
        }

        #[test]
        fn interest_promotes_dormant_to_distant() {
            let mut config = SchedulerConfig::default();
            config.interest.can_promote_dormant = true;
            let mut scheduler = SimulationScheduler::new(config);

            let far = ChunkPos::new(100, 0, 0); // Way beyond distant threshold
            scheduler.add_region(far);
            scheduler.set_observer(ChunkPos::new(0, 0, 0));
            scheduler.set_environment_active(far, true);

            // Without interest, should be dormant (2.0s interval)
            assert_eq!(scheduler.get_fidelity(far), Some(Fidelity::Dormant));

            // Add interest
            scheduler.set_interest(far, InterestCategory::Hazard(HazardKind::Fire), 0.5);

            // Should tick at distant interval (0.5s) due to promotion
            let jobs = scheduler.tick(0.6);
            assert_eq!(jobs.len(), 1);
            assert_eq!(jobs[0].fidelity(), Fidelity::Distant);
        }

        #[test]
        fn interest_enables_hazard_hint_on_distant() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(10, 0, 0); // Distant fidelity
            scheduler.add_region(pos);
            scheduler.set_observer(ChunkPos::new(0, 0, 0));
            scheduler.set_environment_active(pos, true);

            // Add hazard interest
            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 0.5);

            let jobs = scheduler.tick(10.0);
            let job = jobs.iter().find(|j| j.position() == pos).unwrap();

            // Distant normally wouldn't have hazard_spread, but interest enables it
            assert!(job.environment().hazard_spread);
        }

        #[test]
        fn interest_enables_vector_field_hint() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(10, 0, 0); // Distant fidelity
            scheduler.add_region(pos);
            scheduler.set_observer(ChunkPos::new(0, 0, 0));
            scheduler.set_environment_active(pos, true);

            // Distant normally has scalar only, no vector
            let jobs = scheduler.tick(10.0);
            let job = jobs.iter().find(|j| j.position() == pos).unwrap();
            assert!(!job.environment().vector_fields);

            // Add vector field interest
            scheduler.set_interest(
                pos,
                InterestCategory::VectorField(VectorFieldChannel::Wind),
                0.5,
            );

            let jobs = scheduler.tick(10.0);
            let job = jobs.iter().find(|j| j.position() == pos).unwrap();
            assert!(job.environment().vector_fields);
        }

        #[test]
        fn decay_reduces_interest() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);

            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 1.0);

            scheduler.decay_all_interest(0.5);
            let score = scheduler.interest_score(pos).unwrap();
            assert!((score - 0.5).abs() < f32::EPSILON);

            scheduler.decay_all_interest(0.5);
            let score = scheduler.interest_score(pos).unwrap();
            assert!((score - 0.25).abs() < f32::EPSILON);
        }

        #[test]
        fn tick_applies_configured_decay() {
            let mut config = SchedulerConfig::default();
            config.interest.decay_factor = 0.9;
            let mut scheduler = SimulationScheduler::new(config);

            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);
            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 1.0);

            let _ = scheduler.tick(0.1);
            let score = scheduler.interest_score(pos).unwrap();
            assert!((score - 0.9).abs() < f32::EPSILON);
        }

        #[test]
        fn prune_stale_interest() {
            let mut scheduler = scheduler_with_interest_config();
            let pos = ChunkPos::new(0, 0, 0);
            scheduler.add_region(pos);

            scheduler.set_interest(pos, InterestCategory::Hazard(HazardKind::Fire), 0.5);

            // Advance many ticks
            for _ in 0..100 {
                let _ = scheduler.tick(0.0);
            }

            // Now add fresh interest
            scheduler.set_interest(pos, InterestCategory::Structural, 0.5);

            // Prune with short staleness threshold
            let removed = scheduler.prune_stale_interest(50);
            assert_eq!(removed, 1);

            // Fire should be gone, structural should remain
            let summary = scheduler.interest_summary(pos);
            assert!(!summary.has_hazards);
            assert_eq!(summary.entry_count, 1);
        }

        #[test]
        fn distance_only_behavior_unchanged() {
            let mut scheduler = default_scheduler();
            scheduler.add_region(ChunkPos::new(0, 0, 0));
            scheduler.add_region(ChunkPos::new(5, 0, 0));
            scheduler.add_region(ChunkPos::new(10, 0, 0));
            scheduler.add_region(ChunkPos::new(50, 0, 0));
            scheduler.set_observer(ChunkPos::new(0, 0, 0));

            // Without any interest set, behavior should match distance-based
            assert_eq!(
                scheduler.get_fidelity(ChunkPos::new(0, 0, 0)),
                Some(Fidelity::Immediate)
            );
            assert_eq!(
                scheduler.get_fidelity(ChunkPos::new(5, 0, 0)),
                Some(Fidelity::Near)
            );
            assert_eq!(
                scheduler.get_fidelity(ChunkPos::new(10, 0, 0)),
                Some(Fidelity::Distant)
            );
            assert_eq!(
                scheduler.get_fidelity(ChunkPos::new(50, 0, 0)),
                Some(Fidelity::Dormant)
            );

            let jobs = scheduler.tick(10.0);

            // Ordering should be by fidelity (higher first), then distance
            let positions: Vec<_> = jobs.iter().map(SimulationJob::position).collect();
            assert_eq!(positions[0], ChunkPos::new(0, 0, 0)); // Immediate, dist 0
        }

        #[test]
        fn budget_with_interest_respected() {
            let config = SchedulerConfig {
                max_jobs_per_tick: 3,
                ..Default::default()
            };
            let mut scheduler = SimulationScheduler::new(config);

            for i in 0..10 {
                scheduler.add_region(ChunkPos::new(i, 0, 0));
                scheduler.set_environment_active(ChunkPos::new(i, 0, 0), true);
            }
            scheduler.set_observer(ChunkPos::new(0, 0, 0));

            // Give high interest to distant regions
            for i in 5..10 {
                scheduler.set_interest(
                    ChunkPos::new(i, 0, 0),
                    InterestCategory::Hazard(HazardKind::Fire),
                    10.0,
                );
            }

            let jobs = scheduler.tick(10.0);
            assert_eq!(jobs.len(), 3);
        }

        #[test]
        fn deterministic_ordering_with_interest() {
            let mut scheduler1 = scheduler_with_interest_config();
            let mut scheduler2 = scheduler_with_interest_config();

            // Add regions in different orders
            for i in [3, 1, 4, 1, 5] {
                scheduler1.add_region(ChunkPos::new(i, 0, 0));
            }
            for i in [5, 4, 1, 3, 1] {
                scheduler2.add_region(ChunkPos::new(i, 0, 0));
            }

            // Same observer
            scheduler1.set_observer(ChunkPos::new(0, 0, 0));
            scheduler2.set_observer(ChunkPos::new(0, 0, 0));

            // Same interests
            for scheduler in [&mut scheduler1, &mut scheduler2] {
                scheduler.set_interest(
                    ChunkPos::new(5, 0, 0),
                    InterestCategory::Hazard(HazardKind::Fire),
                    0.5,
                );
                scheduler.set_interest(ChunkPos::new(3, 0, 0), InterestCategory::Structural, 0.3);
            }

            let jobs1 = scheduler1.tick(10.0);
            let jobs2 = scheduler2.tick(10.0);

            assert_eq!(jobs1.len(), jobs2.len());
            for (j1, j2) in jobs1.iter().zip(jobs2.iter()) {
                assert_eq!(j1.position(), j2.position());
            }
        }

        #[test]
        fn current_tick_increments() {
            let mut scheduler = scheduler_with_interest_config();
            scheduler.add_region(ChunkPos::new(0, 0, 0));

            assert_eq!(scheduler.current_tick(), 0);
            let _ = scheduler.tick(0.1);
            assert_eq!(scheduler.current_tick(), 1);
            let _ = scheduler.tick(0.1);
            assert_eq!(scheduler.current_tick(), 2);
        }
    }
}

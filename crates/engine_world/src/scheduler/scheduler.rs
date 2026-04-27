//! Core simulation scheduler implementation.

use std::collections::HashMap;

use engine_core::coords::ChunkPos;

use super::{
    config::SchedulerConfig, fidelity::Fidelity, job::EnvironmentHint, job::SimulationJob,
    state::RegionState,
};

/// Simulation scheduler managing region fidelity and tick timing.
///
/// The scheduler tracks regions, assigns fidelity levels based on
/// observer distance, accumulates time, and produces prioritized
/// batches of simulation jobs.
#[derive(Debug)]
pub struct SimulationScheduler {
    /// Configuration for intervals, thresholds, and budget.
    config: SchedulerConfig,
    /// Per-region tracking state.
    regions: HashMap<ChunkPos, RegionState>,
    /// Observer positions (typically player chunk positions).
    observers: Vec<ChunkPos>,
}

impl SimulationScheduler {
    /// Create a new scheduler with the given configuration.
    #[must_use]
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            regions: HashMap::new(),
            observers: Vec::new(),
        }
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
    /// Accumulates `dt` seconds for all regions, then returns up to
    /// `max_jobs_per_tick` jobs sorted by priority.
    #[must_use]
    pub fn tick(&mut self, dt: f32) -> Vec<SimulationJob> {
        // Accumulate time for all regions
        for state in self.regions.values_mut() {
            state.accumulate(dt);
        }

        // Collect ready jobs
        let mut jobs = Vec::new();
        for (&pos, state) in &self.regions {
            let interval = self.config.intervals.get(state.fidelity());
            if state.is_ready(interval) {
                let environment = if state.environment_active() {
                    environment_hint_for_fidelity(state.fidelity())
                } else {
                    EnvironmentHint::NONE
                };

                jobs.push(SimulationJob::new(
                    pos,
                    state.fidelity(),
                    state.accumulated(),
                    state.distance(),
                    environment,
                    state.effective_priority(),
                ));
            }
        }

        // Sort by priority (highest first)
        jobs.sort();

        // Apply budget limit
        if jobs.len() > self.config.max_jobs_per_tick {
            jobs.truncate(self.config.max_jobs_per_tick);
        }

        // Consume time for returned jobs
        for job in &jobs {
            if let Some(state) = self.regions.get_mut(&job.position()) {
                let interval = self.config.intervals.get(state.fidelity());
                state.consume_interval(interval);
            }
        }

        jobs
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
}

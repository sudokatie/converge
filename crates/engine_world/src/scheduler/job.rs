//! Simulation job types.

use engine_core::coords::ChunkPos;

use super::Fidelity;

/// Hint flags for environmental simulation hooks.
///
/// These provide metadata about what environmental systems should
/// be simulated without coupling to specific implementations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnvironmentHint {
    /// Whether scalar field simulation should run.
    pub scalar_fields: bool,
    /// Whether vector field simulation should run.
    pub vector_fields: bool,
    /// Whether hazard spread simulation should run.
    pub hazard_spread: bool,
}

impl EnvironmentHint {
    /// No environmental simulation.
    pub const NONE: Self = Self {
        scalar_fields: false,
        vector_fields: false,
        hazard_spread: false,
    };

    /// Full environmental simulation.
    pub const FULL: Self = Self {
        scalar_fields: true,
        vector_fields: true,
        hazard_spread: true,
    };

    /// Scalar fields only.
    pub const SCALAR_ONLY: Self = Self {
        scalar_fields: true,
        vector_fields: false,
        hazard_spread: false,
    };

    /// Check if any environmental simulation is requested.
    #[must_use]
    pub const fn any_active(&self) -> bool {
        self.scalar_fields || self.vector_fields || self.hazard_spread
    }
}

/// A simulation job ready to execute.
///
/// Represents a region that has accumulated enough time to warrant
/// simulation at its assigned fidelity level.
#[derive(Clone, Debug)]
pub struct SimulationJob {
    /// The chunk/region position.
    position: ChunkPos,
    /// Assigned fidelity level.
    fidelity: Fidelity,
    /// Elapsed time to simulate (seconds).
    delta_time: f32,
    /// Distance to nearest observer (Chebyshev, in chunks).
    distance: i32,
    /// Environmental simulation hints.
    environment: EnvironmentHint,
    /// Effective priority used for ordering.
    priority: i64,
}

impl SimulationJob {
    /// Create a new simulation job.
    #[must_use]
    pub fn new(
        position: ChunkPos,
        fidelity: Fidelity,
        delta_time: f32,
        distance: i32,
        environment: EnvironmentHint,
        priority: i64,
    ) -> Self {
        Self {
            position,
            fidelity,
            delta_time,
            distance,
            environment,
            priority,
        }
    }

    /// Get the chunk/region position.
    #[must_use]
    pub fn position(&self) -> ChunkPos {
        self.position
    }

    /// Get the fidelity level.
    #[must_use]
    pub fn fidelity(&self) -> Fidelity {
        self.fidelity
    }

    /// Get the elapsed time to simulate.
    #[must_use]
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Get the distance to nearest observer.
    #[must_use]
    pub fn distance(&self) -> i32 {
        self.distance
    }

    /// Get the environmental simulation hints.
    #[must_use]
    pub fn environment(&self) -> EnvironmentHint {
        self.environment
    }

    /// Get the priority used for ordering.
    #[must_use]
    pub fn priority(&self) -> i64 {
        self.priority
    }
}

/// Ordering for simulation jobs (highest priority first).
impl PartialEq for SimulationJob {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.position == other.position
    }
}

impl Eq for SimulationJob {}

impl PartialOrd for SimulationJob {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimulationJob {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first
        match other.priority.cmp(&self.priority) {
            std::cmp::Ordering::Equal => {
                // Deterministic tiebreaker: position tuple ordering
                let self_tuple = (self.position.x(), self.position.y(), self.position.z());
                let other_tuple = (other.position.x(), other.position.y(), other.position.z());
                self_tuple.cmp(&other_tuple)
            }
            ord => ord,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job(x: i32, fidelity: Fidelity, priority: i64) -> SimulationJob {
        SimulationJob::new(
            ChunkPos::new(x, 0, 0),
            fidelity,
            0.1,
            x.unsigned_abs() as i32,
            EnvironmentHint::NONE,
            priority,
        )
    }

    #[test]
    fn job_accessors() {
        let job = SimulationJob::new(
            ChunkPos::new(5, 10, 15),
            Fidelity::Near,
            0.25,
            7,
            EnvironmentHint::FULL,
            12345,
        );

        assert_eq!(job.position(), ChunkPos::new(5, 10, 15));
        assert_eq!(job.fidelity(), Fidelity::Near);
        assert!((job.delta_time() - 0.25).abs() < 0.0001);
        assert_eq!(job.distance(), 7);
        assert!(job.environment().any_active());
        assert_eq!(job.priority(), 12345);
    }

    #[test]
    fn job_ordering_by_priority() {
        let high = make_job(0, Fidelity::Immediate, 1000);
        let low = make_job(1, Fidelity::Dormant, 100);

        assert!(high < low); // high priority sorts first (lower in ordering)
    }

    #[test]
    fn job_ordering_deterministic_tiebreaker() {
        let a = make_job(5, Fidelity::Near, 500);
        let b = make_job(10, Fidelity::Near, 500);

        // Same priority, lower x should come first
        assert!(a < b);
    }

    #[test]
    fn job_ordering_stability() {
        let mut jobs = vec![
            make_job(10, Fidelity::Distant, 100),
            make_job(5, Fidelity::Immediate, 1000),
            make_job(3, Fidelity::Near, 500),
            make_job(7, Fidelity::Near, 500),
        ];

        jobs.sort();

        // Should be ordered: highest priority first, then by position
        assert_eq!(jobs[0].position().x(), 5); // Immediate, highest priority
        assert_eq!(jobs[1].position().x(), 3); // Near, lower x
        assert_eq!(jobs[2].position().x(), 7); // Near, higher x
        assert_eq!(jobs[3].position().x(), 10); // Distant, lowest priority
    }

    #[test]
    fn environment_hint_any_active() {
        assert!(!EnvironmentHint::NONE.any_active());
        assert!(EnvironmentHint::FULL.any_active());
        assert!(EnvironmentHint::SCALAR_ONLY.any_active());

        let custom = EnvironmentHint {
            scalar_fields: false,
            vector_fields: true,
            hazard_spread: false,
        };
        assert!(custom.any_active());
    }
}

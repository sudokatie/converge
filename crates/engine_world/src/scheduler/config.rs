//! Scheduler configuration types.

use serde::{Deserialize, Serialize};

use super::Fidelity;
use super::interest::InterestConfig;

/// Tick intervals for each fidelity level.
///
/// The interval is the minimum time (in seconds) between simulation
/// updates for regions at that fidelity level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickIntervals {
    /// Interval for Immediate fidelity regions (default: 0.0, every tick).
    pub immediate: f32,
    /// Interval for Near fidelity regions (default: 0.1s).
    pub near: f32,
    /// Interval for Distant fidelity regions (default: 0.5s).
    pub distant: f32,
    /// Interval for Dormant fidelity regions (default: 2.0s).
    pub dormant: f32,
}

impl TickIntervals {
    /// Get the interval for a specific fidelity level.
    #[must_use]
    pub fn get(&self, fidelity: Fidelity) -> f32 {
        match fidelity {
            Fidelity::Immediate => self.immediate,
            Fidelity::Near => self.near,
            Fidelity::Distant => self.distant,
            Fidelity::Dormant => self.dormant,
        }
    }

    /// Set the interval for a specific fidelity level.
    pub fn set(&mut self, fidelity: Fidelity, interval: f32) {
        let target = match fidelity {
            Fidelity::Immediate => &mut self.immediate,
            Fidelity::Near => &mut self.near,
            Fidelity::Distant => &mut self.distant,
            Fidelity::Dormant => &mut self.dormant,
        };
        *target = interval.max(0.0);
    }
}

impl Default for TickIntervals {
    fn default() -> Self {
        Self {
            immediate: 0.0,
            near: 0.1,
            distant: 0.5,
            dormant: 2.0,
        }
    }
}

/// Distance thresholds for fidelity assignment.
///
/// Uses Chebyshev (chessboard) distance in chunk coordinates.
/// A region is assigned the highest fidelity whose threshold it satisfies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FidelityThresholds {
    /// Maximum distance for Immediate fidelity (inclusive).
    pub immediate_radius: i32,
    /// Maximum distance for Near fidelity (inclusive).
    pub near_radius: i32,
    /// Maximum distance for Distant fidelity (inclusive).
    pub distant_radius: i32,
    // Beyond distant_radius is Dormant
}

impl FidelityThresholds {
    /// Determine fidelity level based on Chebyshev distance from nearest observer.
    #[must_use]
    pub fn fidelity_for_distance(&self, distance: i32) -> Fidelity {
        if distance <= self.immediate_radius {
            Fidelity::Immediate
        } else if distance <= self.near_radius {
            Fidelity::Near
        } else if distance <= self.distant_radius {
            Fidelity::Distant
        } else {
            Fidelity::Dormant
        }
    }

    /// Get the maximum radius for a fidelity level.
    #[must_use]
    pub fn radius_for(&self, fidelity: Fidelity) -> i32 {
        match fidelity {
            Fidelity::Immediate => self.immediate_radius,
            Fidelity::Near => self.near_radius,
            Fidelity::Distant => self.distant_radius,
            Fidelity::Dormant => i32::MAX,
        }
    }
}

impl Default for FidelityThresholds {
    fn default() -> Self {
        Self {
            immediate_radius: 2,
            near_radius: 6,
            distant_radius: 12,
        }
    }
}

/// Complete scheduler configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Tick intervals per fidelity level.
    pub intervals: TickIntervals,
    /// Distance thresholds for fidelity assignment.
    pub thresholds: FidelityThresholds,
    /// Maximum simulation jobs to return per tick.
    pub max_jobs_per_tick: usize,
    /// Interest-based relevance configuration.
    pub interest: InterestConfig,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            intervals: TickIntervals::default(),
            thresholds: FidelityThresholds::default(),
            max_jobs_per_tick: 64,
            interest: InterestConfig::default(),
        }
    }
}

impl SchedulerConfig {
    /// Create a configuration optimized for dense simulation.
    /// Smaller radii, more frequent updates, higher budget.
    #[must_use]
    pub fn dense() -> Self {
        Self {
            intervals: TickIntervals {
                immediate: 0.0,
                near: 0.05,
                distant: 0.2,
                dormant: 1.0,
            },
            thresholds: FidelityThresholds {
                immediate_radius: 3,
                near_radius: 8,
                distant_radius: 16,
            },
            max_jobs_per_tick: 128,
            interest: InterestConfig::default(),
        }
    }

    /// Create a configuration optimized for sparse simulation.
    /// Larger radii, less frequent updates, lower budget.
    #[must_use]
    pub fn sparse() -> Self {
        Self {
            intervals: TickIntervals {
                immediate: 0.0,
                near: 0.2,
                distant: 1.0,
                dormant: 5.0,
            },
            thresholds: FidelityThresholds {
                immediate_radius: 1,
                near_radius: 4,
                distant_radius: 8,
            },
            max_jobs_per_tick: 32,
            interest: InterestConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_intervals_get_set() {
        let mut intervals = TickIntervals::default();
        assert!(intervals.get(Fidelity::Immediate).abs() < f32::EPSILON);
        assert!((intervals.get(Fidelity::Near) - 0.1).abs() < f32::EPSILON);

        intervals.set(Fidelity::Near, 0.25);
        assert!((intervals.get(Fidelity::Near) - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn tick_intervals_clamps_negative() {
        let mut intervals = TickIntervals::default();
        intervals.set(Fidelity::Near, -1.0);
        assert!(intervals.get(Fidelity::Near).abs() < f32::EPSILON);
    }

    #[test]
    fn fidelity_thresholds_immediate() {
        let thresholds = FidelityThresholds::default();
        assert_eq!(thresholds.fidelity_for_distance(0), Fidelity::Immediate);
        assert_eq!(thresholds.fidelity_for_distance(1), Fidelity::Immediate);
        assert_eq!(thresholds.fidelity_for_distance(2), Fidelity::Immediate);
    }

    #[test]
    fn fidelity_thresholds_near() {
        let thresholds = FidelityThresholds::default();
        assert_eq!(thresholds.fidelity_for_distance(3), Fidelity::Near);
        assert_eq!(thresholds.fidelity_for_distance(6), Fidelity::Near);
    }

    #[test]
    fn fidelity_thresholds_distant() {
        let thresholds = FidelityThresholds::default();
        assert_eq!(thresholds.fidelity_for_distance(7), Fidelity::Distant);
        assert_eq!(thresholds.fidelity_for_distance(12), Fidelity::Distant);
    }

    #[test]
    fn fidelity_thresholds_dormant() {
        let thresholds = FidelityThresholds::default();
        assert_eq!(thresholds.fidelity_for_distance(13), Fidelity::Dormant);
        assert_eq!(thresholds.fidelity_for_distance(100), Fidelity::Dormant);
    }

    #[test]
    fn fidelity_thresholds_radius_for() {
        let thresholds = FidelityThresholds::default();
        assert_eq!(thresholds.radius_for(Fidelity::Immediate), 2);
        assert_eq!(thresholds.radius_for(Fidelity::Near), 6);
        assert_eq!(thresholds.radius_for(Fidelity::Distant), 12);
        assert_eq!(thresholds.radius_for(Fidelity::Dormant), i32::MAX);
    }

    #[test]
    fn scheduler_config_presets() {
        let dense = SchedulerConfig::dense();
        let sparse = SchedulerConfig::sparse();

        // Dense has higher budget
        assert!(dense.max_jobs_per_tick > sparse.max_jobs_per_tick);

        // Dense has larger radii
        assert!(dense.thresholds.immediate_radius >= sparse.thresholds.immediate_radius);
    }
}

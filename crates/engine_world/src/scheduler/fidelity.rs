//! Simulation fidelity levels.

use serde::{Deserialize, Serialize};

/// Simulation fidelity level determining update frequency and detail.
///
/// Higher fidelity levels receive more frequent updates and may run
/// more detailed simulation logic. The scheduler assigns fidelity
/// based on distance from observers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Fidelity {
    /// Highest fidelity: immediate vicinity of observers.
    /// Updates every tick for full simulation detail.
    Immediate = 0,

    /// High fidelity: nearby regions visible to observers.
    /// Updates frequently but may skip some detail.
    Near = 1,

    /// Reduced fidelity: distant but active regions.
    /// Updates at a reduced rate with simplified simulation.
    Distant = 2,

    /// Minimal fidelity: far regions with minimal activity.
    /// Updates infrequently, primarily for persistence.
    Dormant = 3,
}

impl Fidelity {
    /// Total number of fidelity levels.
    pub const COUNT: usize = 4;

    /// All fidelity levels in priority order (highest first).
    pub const ALL: [Fidelity; Self::COUNT] = [
        Fidelity::Immediate,
        Fidelity::Near,
        Fidelity::Distant,
        Fidelity::Dormant,
    ];

    /// Get the fidelity level from an index (0 = Immediate, 3 = Dormant).
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Fidelity> {
        match index {
            0 => Some(Fidelity::Immediate),
            1 => Some(Fidelity::Near),
            2 => Some(Fidelity::Distant),
            3 => Some(Fidelity::Dormant),
            _ => None,
        }
    }

    /// Get the index of this fidelity level (0 = Immediate, 3 = Dormant).
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    /// Get the priority of this fidelity level (higher = more important).
    /// Immediate has highest priority (3), Dormant has lowest (0).
    #[must_use]
    pub const fn priority(self) -> u8 {
        match self {
            Fidelity::Immediate => 3,
            Fidelity::Near => 2,
            Fidelity::Distant => 1,
            Fidelity::Dormant => 0,
        }
    }

    /// Check if this fidelity is higher than another.
    #[must_use]
    pub const fn is_higher_than(self, other: Fidelity) -> bool {
        self.priority() > other.priority()
    }

    /// Check if this fidelity is at least as high as another.
    #[must_use]
    pub const fn is_at_least(self, other: Fidelity) -> bool {
        self.priority() >= other.priority()
    }
}

impl Default for Fidelity {
    fn default() -> Self {
        Fidelity::Immediate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fidelity_ordering() {
        assert!(Fidelity::Immediate.is_higher_than(Fidelity::Near));
        assert!(Fidelity::Near.is_higher_than(Fidelity::Distant));
        assert!(Fidelity::Distant.is_higher_than(Fidelity::Dormant));
        assert!(!Fidelity::Dormant.is_higher_than(Fidelity::Dormant));
    }

    #[test]
    fn fidelity_is_at_least() {
        assert!(Fidelity::Immediate.is_at_least(Fidelity::Immediate));
        assert!(Fidelity::Immediate.is_at_least(Fidelity::Dormant));
        assert!(!Fidelity::Dormant.is_at_least(Fidelity::Immediate));
    }

    #[test]
    fn fidelity_from_index() {
        assert_eq!(Fidelity::from_index(0), Some(Fidelity::Immediate));
        assert_eq!(Fidelity::from_index(1), Some(Fidelity::Near));
        assert_eq!(Fidelity::from_index(2), Some(Fidelity::Distant));
        assert_eq!(Fidelity::from_index(3), Some(Fidelity::Dormant));
        assert_eq!(Fidelity::from_index(4), None);
    }

    #[test]
    fn fidelity_as_index() {
        assert_eq!(Fidelity::Immediate.as_index(), 0);
        assert_eq!(Fidelity::Near.as_index(), 1);
        assert_eq!(Fidelity::Distant.as_index(), 2);
        assert_eq!(Fidelity::Dormant.as_index(), 3);
    }

    #[test]
    fn fidelity_priority() {
        assert_eq!(Fidelity::Immediate.priority(), 3);
        assert_eq!(Fidelity::Near.priority(), 2);
        assert_eq!(Fidelity::Distant.priority(), 1);
        assert_eq!(Fidelity::Dormant.priority(), 0);
    }

    #[test]
    fn fidelity_all_array_order() {
        for (i, f) in Fidelity::ALL.iter().enumerate() {
            assert_eq!(f.as_index(), i);
        }
    }
}

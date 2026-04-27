//! State identification for multi-state chunk persistence.
//!
//! Provides typed identifiers for alternate dimensions, time-loop snapshots,
//! and phased realities.

use serde::{Deserialize, Serialize};

/// Unique identifier for a reality state.
///
/// State 0 is always the primary/base reality. Higher values represent
/// alternate states that can coexist at the same spatial position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateId(pub u16);

impl StateId {
    /// The primary reality state (always exists).
    pub const PRIMARY: Self = Self(0);

    /// Create a new state ID.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn id(self) -> u16 {
        self.0
    }

    /// Check if this is the primary state.
    #[must_use]
    pub const fn is_primary(self) -> bool {
        self.0 == 0
    }
}

impl Default for StateId {
    fn default() -> Self {
        Self::PRIMARY
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "state:{}", self.0)
    }
}

/// Classification of reality states for semantic meaning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateKind {
    /// Base reality - the canonical world state.
    #[default]
    Primary,

    /// Alternate dimension with different rules/content.
    AlternateDimension {
        /// Unique dimension identifier (e.g., 1 = nether, 2 = end).
        dimension_id: u16,
    },

    /// Snapshot from a specific tick for time-loop mechanics.
    TimeSnapshot {
        /// The game tick when this snapshot was captured.
        tick: u64,
    },

    /// Phased reality for out-of-phase areas (e.g., ghost dimension).
    PhasedReality {
        /// Phase offset (0-255 for different phase planes).
        phase: u8,
    },
}

impl StateKind {
    /// Convert a state kind to its canonical state ID.
    ///
    /// This provides a deterministic mapping from semantic state kinds
    /// to numeric IDs for persistence.
    #[must_use]
    pub const fn to_state_id(self) -> StateId {
        match self {
            Self::Primary => StateId::PRIMARY,
            Self::AlternateDimension { dimension_id } => {
                // Dimensions: 1-999
                StateId::new(dimension_id.saturating_add(1))
            }
            Self::TimeSnapshot { tick } => {
                // Time snapshots: 1000-9999 (max 9000 snapshots)
                // tick % 9000 is at most 8999, which fits in u16
                let offset = (tick % 9000) as u16;
                StateId::new(1000 + offset)
            }
            Self::PhasedReality { phase } => {
                // Phases: 10000-10255
                StateId::new(10000 + phase as u16)
            }
        }
    }

    /// Attempt to classify a state ID back to its semantic kind.
    ///
    /// Returns `None` for IDs that don't map to a known kind.
    #[must_use]
    pub const fn from_state_id(id: StateId) -> Option<Self> {
        let raw = id.0;
        if raw == 0 {
            Some(Self::Primary)
        } else if raw < 1000 {
            Some(Self::AlternateDimension {
                dimension_id: raw - 1,
            })
        } else if raw < 10000 {
            Some(Self::TimeSnapshot {
                tick: (raw - 1000) as u64,
            })
        } else if raw <= 10255 {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "range check guarantees value fits in u8"
            )]
            Some(Self::PhasedReality {
                phase: (raw - 10000) as u8,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_id_primary() {
        assert!(StateId::PRIMARY.is_primary());
        assert_eq!(StateId::PRIMARY.id(), 0);
        assert!(!StateId::new(1).is_primary());
    }

    #[test]
    fn test_state_id_ordering() {
        let a = StateId::new(1);
        let b = StateId::new(2);
        let c = StateId::new(1);

        assert!(a < b);
        assert_eq!(a, c);
    }

    #[test]
    fn test_state_kind_primary_roundtrip() {
        let kind = StateKind::Primary;
        let id = kind.to_state_id();
        assert_eq!(id, StateId::PRIMARY);
        assert_eq!(StateKind::from_state_id(id), Some(StateKind::Primary));
    }

    #[test]
    fn test_state_kind_alternate_dimension() {
        let kind = StateKind::AlternateDimension { dimension_id: 42 };
        let id = kind.to_state_id();
        assert_eq!(id.id(), 43);
        assert_eq!(
            StateKind::from_state_id(id),
            Some(StateKind::AlternateDimension { dimension_id: 42 })
        );
    }

    #[test]
    fn test_state_kind_time_snapshot() {
        let kind = StateKind::TimeSnapshot { tick: 1234 };
        let id = kind.to_state_id();
        assert!(id.id() >= 1000 && id.id() < 10000);

        if let Some(StateKind::TimeSnapshot { tick }) = StateKind::from_state_id(id) {
            assert_eq!(tick, 1234);
        } else {
            panic!("expected TimeSnapshot");
        }
    }

    #[test]
    fn test_state_kind_time_snapshot_wraps() {
        let kind = StateKind::TimeSnapshot { tick: 12345 };
        let id = kind.to_state_id();
        assert!(id.id() >= 1000 && id.id() < 10000);
    }

    #[test]
    fn test_state_kind_phased_reality() {
        let kind = StateKind::PhasedReality { phase: 128 };
        let id = kind.to_state_id();
        assert_eq!(id.id(), 10128);
        assert_eq!(
            StateKind::from_state_id(id),
            Some(StateKind::PhasedReality { phase: 128 })
        );
    }

    #[test]
    fn test_state_kind_unknown_id() {
        let id = StateId::new(60000);
        assert_eq!(StateKind::from_state_id(id), None);
    }

    #[test]
    fn test_state_id_serde_roundtrip() {
        let original = StateId::new(42);
        let serialized = bincode::serialize(&original).unwrap();
        let deserialized: StateId = bincode::deserialize(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_state_kind_serde_roundtrip() {
        let kinds = [
            StateKind::Primary,
            StateKind::AlternateDimension { dimension_id: 5 },
            StateKind::TimeSnapshot { tick: 9999 },
            StateKind::PhasedReality { phase: 255 },
        ];

        for kind in kinds {
            let serialized = bincode::serialize(&kind).unwrap();
            let deserialized: StateKind = bincode::deserialize(&serialized).unwrap();
            assert_eq!(kind, deserialized);
        }
    }

    #[test]
    fn test_state_id_display() {
        assert_eq!(format!("{}", StateId::PRIMARY), "state:0");
        assert_eq!(format!("{}", StateId::new(42)), "state:42");
    }
}

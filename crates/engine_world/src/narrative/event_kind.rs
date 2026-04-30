//! Narrative event type definitions.

use serde::{Deserialize, Serialize};

/// Categories of narrative events that can occur in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NarrativeEventKind {
    /// Major environmental disasters: meteor strikes, earthquakes, volcanic eruptions.
    Disaster = 0,

    /// Radio communications: distress calls, broadcasts, intercepted transmissions.
    Radio = 1,

    /// Time-limited objectives: rescue missions, evacuation deadlines, resource collection.
    Objective = 2,

    /// Anomaly sightings: strange phenomena, unidentified signals, unusual readings.
    Anomaly = 3,
}

impl NarrativeEventKind {
    /// Total number of narrative event kinds.
    pub const COUNT: usize = 4;

    /// All event kinds in order.
    pub const ALL: [NarrativeEventKind; Self::COUNT] = [
        NarrativeEventKind::Disaster,
        NarrativeEventKind::Radio,
        NarrativeEventKind::Objective,
        NarrativeEventKind::Anomaly,
    ];

    /// Convert to array index.
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    /// Create from array index.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(NarrativeEventKind::Disaster),
            1 => Some(NarrativeEventKind::Radio),
            2 => Some(NarrativeEventKind::Objective),
            3 => Some(NarrativeEventKind::Anomaly),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            NarrativeEventKind::Disaster => "Disaster",
            NarrativeEventKind::Radio => "Radio",
            NarrativeEventKind::Objective => "Objective",
            NarrativeEventKind::Anomaly => "Anomaly",
        }
    }

    /// Whether this event type requires immediate attention.
    #[must_use]
    pub const fn is_urgent(self) -> bool {
        matches!(
            self,
            NarrativeEventKind::Disaster | NarrativeEventKind::Objective
        )
    }

    /// Whether this event type has a time limit.
    #[must_use]
    pub const fn is_timed(self) -> bool {
        matches!(self, NarrativeEventKind::Objective)
    }

    /// Whether this event type may repeat.
    #[must_use]
    pub const fn can_repeat(self) -> bool {
        matches!(
            self,
            NarrativeEventKind::Radio | NarrativeEventKind::Anomaly
        )
    }

    /// Whether this event type affects world state directly.
    #[must_use]
    pub const fn affects_world(self) -> bool {
        matches!(self, NarrativeEventKind::Disaster)
    }

    /// Whether this event type triggers audio/UI notifications.
    #[must_use]
    pub const fn triggers_notification(self) -> bool {
        true
    }

    /// Default duration in world ticks if not specified.
    #[must_use]
    pub const fn default_duration(self) -> u64 {
        match self {
            NarrativeEventKind::Disaster => 1800,
            NarrativeEventKind::Radio => 300,
            NarrativeEventKind::Objective => 6000,
            NarrativeEventKind::Anomaly => 900,
        }
    }

    /// Default priority for output ordering.
    #[must_use]
    pub const fn default_priority(self) -> u8 {
        match self {
            NarrativeEventKind::Disaster => 255,
            NarrativeEventKind::Objective => 200,
            NarrativeEventKind::Anomaly => 150,
            NarrativeEventKind::Radio => 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(NarrativeEventKind::ALL.len(), NarrativeEventKind::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for kind in NarrativeEventKind::ALL {
            let index = kind.as_index();
            let recovered = NarrativeEventKind::from_index(index);
            assert_eq!(recovered, Some(kind));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(NarrativeEventKind::from_index(4), None);
        assert_eq!(NarrativeEventKind::from_index(255), None);
    }

    #[test]
    fn names_not_empty() {
        for kind in NarrativeEventKind::ALL {
            assert!(!kind.name().is_empty());
        }
    }

    #[test]
    fn urgent_classification() {
        assert!(NarrativeEventKind::Disaster.is_urgent());
        assert!(NarrativeEventKind::Objective.is_urgent());
        assert!(!NarrativeEventKind::Radio.is_urgent());
        assert!(!NarrativeEventKind::Anomaly.is_urgent());
    }

    #[test]
    fn timed_classification() {
        assert!(NarrativeEventKind::Objective.is_timed());
        assert!(!NarrativeEventKind::Disaster.is_timed());
    }

    #[test]
    fn repeatable_classification() {
        assert!(NarrativeEventKind::Radio.can_repeat());
        assert!(NarrativeEventKind::Anomaly.can_repeat());
        assert!(!NarrativeEventKind::Disaster.can_repeat());
        assert!(!NarrativeEventKind::Objective.can_repeat());
    }

    #[test]
    fn default_durations_positive() {
        for kind in NarrativeEventKind::ALL {
            assert!(kind.default_duration() > 0);
        }
    }

    #[test]
    fn priorities_ordered() {
        assert!(
            NarrativeEventKind::Disaster.default_priority()
                > NarrativeEventKind::Objective.default_priority()
        );
        assert!(
            NarrativeEventKind::Objective.default_priority()
                > NarrativeEventKind::Anomaly.default_priority()
        );
        assert!(
            NarrativeEventKind::Anomaly.default_priority()
                > NarrativeEventKind::Radio.default_priority()
        );
    }

    #[test]
    fn serde_round_trip() {
        for kind in NarrativeEventKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: NarrativeEventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}

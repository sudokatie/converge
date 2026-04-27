//! World event type definitions.

use serde::{Deserialize, Serialize};

/// Types of global world events that affect the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WorldEventKind {
    /// Solar eclipse: reduces light, triggers nocturnal behavior, affects temperature.
    Eclipse = 0,

    /// Structural collapse: regional terrain instability, triggers cave-ins.
    Collapse = 1,

    /// Season shift: changes temperature baselines, affects biome behavior.
    SeasonShift = 2,

    /// Biome corruption: spreads corruption hazard, transforms terrain.
    BiomeCorruption = 3,

    /// Migration wave: triggers creature movement, changes spawn patterns.
    MigrationWave = 4,
}

impl WorldEventKind {
    /// Total number of world event kinds.
    pub const COUNT: usize = 5;

    /// All event kinds in order.
    pub const ALL: [WorldEventKind; Self::COUNT] = [
        WorldEventKind::Eclipse,
        WorldEventKind::Collapse,
        WorldEventKind::SeasonShift,
        WorldEventKind::BiomeCorruption,
        WorldEventKind::MigrationWave,
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
            0 => Some(WorldEventKind::Eclipse),
            1 => Some(WorldEventKind::Collapse),
            2 => Some(WorldEventKind::SeasonShift),
            3 => Some(WorldEventKind::BiomeCorruption),
            4 => Some(WorldEventKind::MigrationWave),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            WorldEventKind::Eclipse => "Eclipse",
            WorldEventKind::Collapse => "Collapse",
            WorldEventKind::SeasonShift => "Season Shift",
            WorldEventKind::BiomeCorruption => "Biome Corruption",
            WorldEventKind::MigrationWave => "Migration Wave",
        }
    }

    /// Whether this event affects lighting calculations.
    #[must_use]
    pub const fn affects_lighting(self) -> bool {
        matches!(self, WorldEventKind::Eclipse)
    }

    /// Whether this event affects temperature fields.
    #[must_use]
    pub const fn affects_temperature(self) -> bool {
        matches!(self, WorldEventKind::Eclipse | WorldEventKind::SeasonShift)
    }

    /// Whether this event affects structural stability.
    #[must_use]
    pub const fn affects_structure(self) -> bool {
        matches!(self, WorldEventKind::Collapse)
    }

    /// Whether this event affects hazard spread.
    #[must_use]
    pub const fn affects_hazards(self) -> bool {
        matches!(self, WorldEventKind::BiomeCorruption)
    }

    /// Whether this event affects entity spawning/movement.
    #[must_use]
    pub const fn affects_entities(self) -> bool {
        matches!(
            self,
            WorldEventKind::Eclipse | WorldEventKind::MigrationWave
        )
    }

    /// Whether this event has a defined spatial region (vs global).
    #[must_use]
    pub const fn is_regional(self) -> bool {
        matches!(
            self,
            WorldEventKind::Collapse | WorldEventKind::BiomeCorruption
        )
    }

    /// Default duration in world ticks if not specified.
    #[must_use]
    pub const fn default_duration(self) -> u64 {
        match self {
            WorldEventKind::Eclipse => 3600,
            WorldEventKind::Collapse => 600,
            WorldEventKind::SeasonShift => 0,
            WorldEventKind::BiomeCorruption => 7200,
            WorldEventKind::MigrationWave => 1800,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(WorldEventKind::ALL.len(), WorldEventKind::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for kind in WorldEventKind::ALL {
            let index = kind.as_index();
            let recovered = WorldEventKind::from_index(index);
            assert_eq!(recovered, Some(kind));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(WorldEventKind::from_index(5), None);
        assert_eq!(WorldEventKind::from_index(255), None);
    }

    #[test]
    fn names_not_empty() {
        for kind in WorldEventKind::ALL {
            assert!(!kind.name().is_empty());
        }
    }

    #[test]
    fn lighting_effects() {
        assert!(WorldEventKind::Eclipse.affects_lighting());
        assert!(!WorldEventKind::Collapse.affects_lighting());
    }

    #[test]
    fn temperature_effects() {
        assert!(WorldEventKind::Eclipse.affects_temperature());
        assert!(WorldEventKind::SeasonShift.affects_temperature());
        assert!(!WorldEventKind::MigrationWave.affects_temperature());
    }

    #[test]
    fn regional_vs_global() {
        assert!(WorldEventKind::Collapse.is_regional());
        assert!(WorldEventKind::BiomeCorruption.is_regional());
        assert!(!WorldEventKind::Eclipse.is_regional());
        assert!(!WorldEventKind::SeasonShift.is_regional());
    }

    #[test]
    fn default_durations_valid() {
        for kind in WorldEventKind::ALL {
            let duration = kind.default_duration();
            if kind != WorldEventKind::SeasonShift {
                assert!(duration > 0, "{kind:?} should have positive duration");
            }
        }
    }

    #[test]
    fn serde_round_trip() {
        for kind in WorldEventKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let recovered: WorldEventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, kind);
        }
    }
}

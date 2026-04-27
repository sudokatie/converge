//! Season definitions and transitions.

use serde::{Deserialize, Serialize};

/// World seasons affecting temperature and biome behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum Season {
    /// Warming temperatures, growth begins.
    #[default]
    Spring = 0,

    /// Peak temperatures, full growth.
    Summer = 1,

    /// Cooling temperatures, harvest.
    Autumn = 2,

    /// Cold temperatures, dormancy.
    Winter = 3,
}

impl Season {
    /// Total number of seasons.
    pub const COUNT: usize = 4;

    /// All seasons in order.
    pub const ALL: [Season; Self::COUNT] = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
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
            0 => Some(Season::Spring),
            1 => Some(Season::Summer),
            2 => Some(Season::Autumn),
            3 => Some(Season::Winter),
            _ => None,
        }
    }

    /// Get the display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Autumn => "Autumn",
            Season::Winter => "Winter",
        }
    }

    /// Temperature modifier (-1.0 to 1.0).
    #[must_use]
    pub const fn temperature_modifier(self) -> f32 {
        match self {
            Season::Spring => 0.0,
            Season::Summer => 0.3,
            Season::Autumn => -0.1,
            Season::Winter => -0.4,
        }
    }

    /// Growth rate modifier (0.0 to 1.0).
    #[must_use]
    pub const fn growth_modifier(self) -> f32 {
        match self {
            Season::Spring => 1.0,
            Season::Summer => 0.8,
            Season::Autumn => 0.4,
            Season::Winter => 0.0,
        }
    }

    /// Day length modifier (0.5 to 1.5).
    #[must_use]
    pub const fn daylight_modifier(self) -> f32 {
        match self {
            Season::Spring => 1.0,
            Season::Summer => 1.3,
            Season::Autumn => 0.9,
            Season::Winter => 0.6,
        }
    }

    /// Get the next season in the cycle.
    #[must_use]
    pub const fn next(self) -> Season {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }

    /// Get the previous season in the cycle.
    #[must_use]
    pub const fn prev(self) -> Season {
        match self {
            Season::Spring => Season::Winter,
            Season::Summer => Season::Spring,
            Season::Autumn => Season::Summer,
            Season::Winter => Season::Autumn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(Season::ALL.len(), Season::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for season in Season::ALL {
            let index = season.as_index();
            let recovered = Season::from_index(index);
            assert_eq!(recovered, Some(season));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(Season::from_index(4), None);
        assert_eq!(Season::from_index(255), None);
    }

    #[test]
    fn season_cycle_next() {
        assert_eq!(Season::Spring.next(), Season::Summer);
        assert_eq!(Season::Summer.next(), Season::Autumn);
        assert_eq!(Season::Autumn.next(), Season::Winter);
        assert_eq!(Season::Winter.next(), Season::Spring);
    }

    #[test]
    fn season_cycle_prev() {
        assert_eq!(Season::Spring.prev(), Season::Winter);
        assert_eq!(Season::Summer.prev(), Season::Spring);
        assert_eq!(Season::Autumn.prev(), Season::Summer);
        assert_eq!(Season::Winter.prev(), Season::Autumn);
    }

    #[test]
    fn next_prev_inverse() {
        for season in Season::ALL {
            assert_eq!(season.next().prev(), season);
            assert_eq!(season.prev().next(), season);
        }
    }

    #[test]
    fn temperature_modifiers_in_range() {
        for season in Season::ALL {
            let modifier = season.temperature_modifier();
            assert!(
                (-1.0..=1.0).contains(&modifier),
                "{season:?} temperature modifier out of range: {modifier}"
            );
        }
    }

    #[test]
    fn growth_modifiers_in_range() {
        for season in Season::ALL {
            let modifier = season.growth_modifier();
            assert!(
                (0.0..=1.0).contains(&modifier),
                "{season:?} growth modifier out of range: {modifier}"
            );
        }
    }

    #[test]
    fn daylight_modifiers_in_range() {
        for season in Season::ALL {
            let modifier = season.daylight_modifier();
            assert!(
                (0.5..=1.5).contains(&modifier),
                "{season:?} daylight modifier out of range: {modifier}"
            );
        }
    }

    #[test]
    fn default_is_spring() {
        assert_eq!(Season::default(), Season::Spring);
    }

    #[test]
    fn serde_round_trip() {
        for season in Season::ALL {
            let json = serde_json::to_string(&season).unwrap();
            let recovered: Season = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, season);
        }
    }
}

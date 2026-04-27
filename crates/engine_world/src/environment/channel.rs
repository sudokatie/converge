//! Environmental field channel types.

use serde::{Deserialize, Serialize};

/// Typed environmental field channels.
///
/// Each channel represents a distinct scalar field that can vary per-cell
/// within chunks. Fields can be used for gameplay mechanics, biome effects,
/// and environmental hazards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum FieldChannel {
    /// Temperature in arbitrary units. Affects player comfort, crop growth.
    Temperature = 0,

    /// Oxygen level (0.0 = vacuum, 1.0 = breathable). Affects respiration.
    Oxygen = 1,

    /// Atmospheric pressure. Affects movement speed, sound propagation.
    Pressure = 2,

    /// Radiation level. Causes damage over time when elevated.
    Radiation = 3,

    /// Toxicity/poison level. Causes damage and debuffs.
    Toxicity = 4,

    /// Humidity level. Affects fire spread, plant growth.
    Humidity = 5,

    /// Corruption level. Spreads and transforms terrain.
    Corruption = 6,

    /// Spore density. Affects visibility and causes infection.
    SporeDensity = 7,
}

impl FieldChannel {
    /// Total number of field channels.
    pub const COUNT: usize = 8;

    /// Get all channels in order.
    pub const ALL: [FieldChannel; Self::COUNT] = [
        FieldChannel::Temperature,
        FieldChannel::Oxygen,
        FieldChannel::Pressure,
        FieldChannel::Radiation,
        FieldChannel::Toxicity,
        FieldChannel::Humidity,
        FieldChannel::Corruption,
        FieldChannel::SporeDensity,
    ];

    /// Convert to array index.
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }

    /// Create from array index.
    ///
    /// Returns None if index is out of range.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(FieldChannel::Temperature),
            1 => Some(FieldChannel::Oxygen),
            2 => Some(FieldChannel::Pressure),
            3 => Some(FieldChannel::Radiation),
            4 => Some(FieldChannel::Toxicity),
            5 => Some(FieldChannel::Humidity),
            6 => Some(FieldChannel::Corruption),
            7 => Some(FieldChannel::SporeDensity),
            _ => None,
        }
    }

    /// Get the default value for this channel.
    ///
    /// Default values represent neutral/safe environmental conditions.
    #[must_use]
    #[expect(
        clippy::match_same_arms,
        reason = "values documented separately for each channel for clarity"
    )]
    pub const fn default_value(self) -> f32 {
        match self {
            FieldChannel::Temperature => 20.0, // Room temperature (Celsius-ish)
            FieldChannel::Oxygen => 1.0,       // Full breathable atmosphere
            FieldChannel::Pressure => 1.0,     // Standard atmospheric pressure
            FieldChannel::Radiation => 0.0,    // No radiation
            FieldChannel::Toxicity => 0.0,     // No toxins
            FieldChannel::Humidity => 0.5,     // Moderate humidity
            FieldChannel::Corruption => 0.0,   // No corruption
            FieldChannel::SporeDensity => 0.0, // No spores
        }
    }

    /// Get the minimum valid value for this channel.
    #[must_use]
    pub const fn min_value(self) -> f32 {
        match self {
            FieldChannel::Temperature => -273.15, // Absolute zero
            _ => 0.0,
        }
    }

    /// Get the maximum valid value for this channel.
    ///
    /// Returns None if unbounded.
    #[must_use]
    #[expect(
        clippy::match_same_arms,
        reason = "values documented separately for each channel for clarity"
    )]
    pub const fn max_value(self) -> Option<f32> {
        match self {
            FieldChannel::Temperature => None, // Unbounded upper
            FieldChannel::Oxygen => Some(1.0),
            FieldChannel::Pressure => None,
            FieldChannel::Radiation => None,
            FieldChannel::Toxicity => Some(1.0),
            FieldChannel::Humidity => Some(1.0),
            FieldChannel::Corruption => Some(1.0),
            FieldChannel::SporeDensity => Some(1.0),
        }
    }

    /// Get the display name for this channel.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            FieldChannel::Temperature => "Temperature",
            FieldChannel::Oxygen => "Oxygen",
            FieldChannel::Pressure => "Pressure",
            FieldChannel::Radiation => "Radiation",
            FieldChannel::Toxicity => "Toxicity",
            FieldChannel::Humidity => "Humidity",
            FieldChannel::Corruption => "Corruption",
            FieldChannel::SporeDensity => "Spore Density",
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::uninlined_format_args,
    reason = "tests check exact values; format args clearer with explicit args"
)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_count() {
        assert_eq!(FieldChannel::ALL.len(), FieldChannel::COUNT);
    }

    #[test]
    fn test_as_index_round_trip() {
        for channel in FieldChannel::ALL {
            let index = channel.as_index();
            let recovered = FieldChannel::from_index(index);
            assert_eq!(recovered, Some(channel));
        }
    }

    #[test]
    fn test_from_index_out_of_range() {
        assert_eq!(FieldChannel::from_index(8), None);
        assert_eq!(FieldChannel::from_index(255), None);
    }

    #[test]
    fn test_default_values_within_bounds() {
        for channel in FieldChannel::ALL {
            let default = channel.default_value();
            let min = channel.min_value();
            assert!(default >= min, "{:?} default below minimum", channel);

            if let Some(max) = channel.max_value() {
                assert!(default <= max, "{:?} default above maximum", channel);
            }
        }
    }

    #[test]
    fn test_oxygen_defaults_breathable() {
        assert_eq!(FieldChannel::Oxygen.default_value(), 1.0);
    }

    #[test]
    fn test_radiation_defaults_safe() {
        assert_eq!(FieldChannel::Radiation.default_value(), 0.0);
    }
}

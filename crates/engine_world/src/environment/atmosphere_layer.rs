//! Atmosphere layer type definitions.

use serde::{Deserialize, Serialize};

/// Classification of atmospheric environment for a cell.
///
/// Layers represent fundamentally different atmospheric conditions that
/// affect environmental simulation, hazard propagation, and entity behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum AtmosphereLayer {
    /// Sealed indoor environment (buildings, underground, pressurized).
    ///
    /// Characteristics:
    /// - Atmosphere isolated from external conditions
    /// - Temperature regulated (or at least buffered)
    /// - Protected from weather and radiation
    Indoor = 0,

    /// Open outdoor environment (surface, open terrain).
    ///
    /// Characteristics:
    /// - Exposed to weather effects
    /// - Natural atmospheric conditions
    /// - Direct sunlight/radiation exposure
    #[default]
    Outdoor = 1,

    /// Partially exposed space (covered but not sealed).
    ///
    /// Examples: awnings, ruins, caves with openings, porches.
    /// Characteristics:
    /// - Partial protection from weather
    /// - Air exchange with outdoors
    /// - Some radiation shielding
    Exposed = 2,

    /// Vacuum or near-vacuum (space, decompressed areas).
    ///
    /// Characteristics:
    /// - No breathable atmosphere
    /// - Extreme temperature variation
    /// - Full radiation exposure
    Vacuum = 3,
}

impl AtmosphereLayer {
    /// Total number of atmosphere layers.
    pub const COUNT: usize = 4;

    /// All atmosphere layers in order.
    pub const ALL: [AtmosphereLayer; Self::COUNT] = [
        AtmosphereLayer::Indoor,
        AtmosphereLayer::Outdoor,
        AtmosphereLayer::Exposed,
        AtmosphereLayer::Vacuum,
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
            0 => Some(AtmosphereLayer::Indoor),
            1 => Some(AtmosphereLayer::Outdoor),
            2 => Some(AtmosphereLayer::Exposed),
            3 => Some(AtmosphereLayer::Vacuum),
            _ => None,
        }
    }

    /// Get the display name for this layer.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            AtmosphereLayer::Indoor => "Indoor",
            AtmosphereLayer::Outdoor => "Outdoor",
            AtmosphereLayer::Exposed => "Exposed",
            AtmosphereLayer::Vacuum => "Vacuum",
        }
    }

    /// Whether this layer provides shelter from weather.
    #[must_use]
    pub const fn sheltered(self) -> bool {
        matches!(self, AtmosphereLayer::Indoor)
    }

    /// Whether this layer is affected by external weather.
    #[must_use]
    pub const fn weather_affected(self) -> bool {
        matches!(self, AtmosphereLayer::Outdoor | AtmosphereLayer::Exposed)
    }

    /// Whether this layer has breathable atmosphere by default.
    #[must_use]
    pub const fn breathable(self) -> bool {
        !matches!(self, AtmosphereLayer::Vacuum)
    }

    /// Whether this layer is sealed from adjacent atmosphere.
    #[must_use]
    pub const fn sealed(self) -> bool {
        matches!(self, AtmosphereLayer::Indoor)
    }

    /// Radiation exposure factor (0.0 = fully shielded, 1.0 = fully exposed).
    #[must_use]
    pub const fn radiation_exposure(self) -> f32 {
        match self {
            AtmosphereLayer::Indoor => 0.0,
            AtmosphereLayer::Outdoor => 0.7,
            AtmosphereLayer::Exposed => 0.3,
            AtmosphereLayer::Vacuum => 1.0,
        }
    }

    /// Temperature regulation factor (0.0 = unregulated, 1.0 = fully regulated).
    #[must_use]
    pub const fn temperature_regulation(self) -> f32 {
        match self {
            AtmosphereLayer::Indoor => 1.0,
            AtmosphereLayer::Outdoor => 0.0,
            AtmosphereLayer::Exposed => 0.3,
            AtmosphereLayer::Vacuum => 0.0,
        }
    }

    /// Default oxygen level for this layer (0.0-1.0).
    #[must_use]
    pub const fn default_oxygen(self) -> f32 {
        match self {
            AtmosphereLayer::Indoor => 1.0,
            AtmosphereLayer::Outdoor => 1.0,
            AtmosphereLayer::Exposed => 1.0,
            AtmosphereLayer::Vacuum => 0.0,
        }
    }

    /// Default pressure level for this layer (normalized).
    #[must_use]
    pub const fn default_pressure(self) -> f32 {
        match self {
            AtmosphereLayer::Indoor => 1.0,
            AtmosphereLayer::Outdoor => 1.0,
            AtmosphereLayer::Exposed => 1.0,
            AtmosphereLayer::Vacuum => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_all() {
        assert_eq!(AtmosphereLayer::ALL.len(), AtmosphereLayer::COUNT);
    }

    #[test]
    fn index_round_trip() {
        for layer in AtmosphereLayer::ALL {
            let index = layer.as_index();
            let recovered = AtmosphereLayer::from_index(index);
            assert_eq!(recovered, Some(layer));
        }
    }

    #[test]
    fn from_index_out_of_range() {
        assert_eq!(AtmosphereLayer::from_index(4), None);
        assert_eq!(AtmosphereLayer::from_index(255), None);
    }

    #[test]
    fn default_is_outdoor() {
        assert_eq!(AtmosphereLayer::default(), AtmosphereLayer::Outdoor);
    }

    #[test]
    fn shelter_properties() {
        assert!(AtmosphereLayer::Indoor.sheltered());
        assert!(!AtmosphereLayer::Outdoor.sheltered());
        assert!(!AtmosphereLayer::Exposed.sheltered());
        assert!(!AtmosphereLayer::Vacuum.sheltered());
    }

    #[test]
    fn weather_affected_properties() {
        assert!(!AtmosphereLayer::Indoor.weather_affected());
        assert!(AtmosphereLayer::Outdoor.weather_affected());
        assert!(AtmosphereLayer::Exposed.weather_affected());
        assert!(!AtmosphereLayer::Vacuum.weather_affected());
    }

    #[test]
    fn breathable_properties() {
        assert!(AtmosphereLayer::Indoor.breathable());
        assert!(AtmosphereLayer::Outdoor.breathable());
        assert!(AtmosphereLayer::Exposed.breathable());
        assert!(!AtmosphereLayer::Vacuum.breathable());
    }

    #[test]
    fn sealed_properties() {
        assert!(AtmosphereLayer::Indoor.sealed());
        assert!(!AtmosphereLayer::Outdoor.sealed());
        assert!(!AtmosphereLayer::Exposed.sealed());
        assert!(!AtmosphereLayer::Vacuum.sealed());
    }

    #[test]
    fn radiation_exposure_range() {
        for layer in AtmosphereLayer::ALL {
            let exposure = layer.radiation_exposure();
            assert!(
                (0.0..=1.0).contains(&exposure),
                "{:?} radiation exposure out of range: {}",
                layer,
                exposure
            );
        }
    }

    #[test]
    fn radiation_exposure_ordering() {
        assert!(
            AtmosphereLayer::Indoor.radiation_exposure()
                < AtmosphereLayer::Exposed.radiation_exposure()
        );
        assert!(
            AtmosphereLayer::Exposed.radiation_exposure()
                < AtmosphereLayer::Outdoor.radiation_exposure()
        );
        assert!(
            AtmosphereLayer::Outdoor.radiation_exposure()
                < AtmosphereLayer::Vacuum.radiation_exposure()
        );
    }

    #[test]
    fn temperature_regulation_range() {
        for layer in AtmosphereLayer::ALL {
            let reg = layer.temperature_regulation();
            assert!(
                (0.0..=1.0).contains(&reg),
                "{:?} temp regulation out of range: {}",
                layer,
                reg
            );
        }
    }

    #[test]
    fn default_oxygen_range() {
        for layer in AtmosphereLayer::ALL {
            let oxygen = layer.default_oxygen();
            assert!(
                (0.0..=1.0).contains(&oxygen),
                "{:?} default oxygen out of range: {}",
                layer,
                oxygen
            );
        }
    }

    #[test]
    fn default_pressure_range() {
        for layer in AtmosphereLayer::ALL {
            let pressure = layer.default_pressure();
            assert!(
                (0.0..=1.0).contains(&pressure),
                "{:?} default pressure out of range: {}",
                layer,
                pressure
            );
        }
    }

    #[test]
    fn vacuum_has_no_atmosphere() {
        assert_eq!(AtmosphereLayer::Vacuum.default_oxygen(), 0.0);
        assert_eq!(AtmosphereLayer::Vacuum.default_pressure(), 0.0);
    }

    #[test]
    fn serde_round_trip() {
        for layer in AtmosphereLayer::ALL {
            let json = serde_json::to_string(&layer).unwrap();
            let recovered: AtmosphereLayer = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, layer);
        }
    }
}

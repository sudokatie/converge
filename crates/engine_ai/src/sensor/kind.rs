//! Sensor kinds/channels for perception.

use serde::{Deserialize, Serialize};

/// The type of sensory channel used for perception.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SensorKind {
    /// Visual perception (light, movement, color).
    #[default]
    Sight,
    /// Auditory perception (sounds, vibrations in air/medium).
    Sound,
    /// Tactile/substrate vibration detection (seismic, surface waves).
    Vibration,
    /// Olfactory perception (chemical traces, scent trails).
    Smell,
    /// Thermal perception (infrared, temperature gradients).
    Heat,
    /// Pressure/touch perception (barometric, water pressure, contact).
    Pressure,
    /// Electrical field detection (electroreception, bioelectricity).
    ElectricalField,
}

impl SensorKind {
    /// All sensor kinds in deterministic order.
    pub const ALL: &'static [SensorKind] = &[
        SensorKind::Sight,
        SensorKind::Sound,
        SensorKind::Vibration,
        SensorKind::Smell,
        SensorKind::Heat,
        SensorKind::Pressure,
        SensorKind::ElectricalField,
    ];

    /// Get a human-readable name for this sensor kind.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Sight => "sight",
            Self::Sound => "sound",
            Self::Vibration => "vibration",
            Self::Smell => "smell",
            Self::Heat => "heat",
            Self::Pressure => "pressure",
            Self::ElectricalField => "electrical_field",
        }
    }

    /// Whether this sensor typically requires line-of-sight.
    #[must_use]
    pub fn requires_line_of_sight(self) -> bool {
        matches!(self, Self::Sight)
    }

    /// Whether this sensor can detect through solid obstacles (attenuated).
    #[must_use]
    pub fn can_penetrate_solids(self) -> bool {
        matches!(
            self,
            Self::Sound | Self::Vibration | Self::Heat | Self::ElectricalField
        )
    }

    /// Whether this sensor is directional by default.
    #[must_use]
    pub fn is_directional(self) -> bool {
        matches!(self, Self::Sight | Self::Sound | Self::Smell)
    }

    /// Default base range for this sensor kind (arbitrary units).
    #[must_use]
    pub fn default_range(self) -> f32 {
        match self {
            Self::Sight => 50.0,
            Self::Sound => 30.0,
            Self::Vibration => 20.0,
            Self::Smell => 25.0,
            Self::Heat => 10.0,
            Self::Pressure => 5.0,
            Self::ElectricalField => 8.0,
        }
    }

    /// Default attenuation exponent (inverse-square = 2.0).
    #[must_use]
    pub fn default_attenuation_exponent(self) -> f32 {
        match self {
            Self::Sight => 0.5,
            Self::Sound | Self::Heat => 2.0,
            Self::Vibration => 2.5,
            Self::Smell => 1.5,
            Self::Pressure => 1.0,
            Self::ElectricalField => 3.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_kind_all() {
        assert_eq!(SensorKind::ALL.len(), 7);
        assert_eq!(SensorKind::ALL[0], SensorKind::Sight);
        assert_eq!(SensorKind::ALL[6], SensorKind::ElectricalField);
    }

    #[test]
    fn test_sensor_kind_name() {
        assert_eq!(SensorKind::Sight.name(), "sight");
        assert_eq!(SensorKind::ElectricalField.name(), "electrical_field");
    }

    #[test]
    fn test_sensor_kind_line_of_sight() {
        assert!(SensorKind::Sight.requires_line_of_sight());
        assert!(!SensorKind::Sound.requires_line_of_sight());
        assert!(!SensorKind::Smell.requires_line_of_sight());
    }

    #[test]
    fn test_sensor_kind_penetrate_solids() {
        assert!(!SensorKind::Sight.can_penetrate_solids());
        assert!(SensorKind::Sound.can_penetrate_solids());
        assert!(SensorKind::Vibration.can_penetrate_solids());
        assert!(!SensorKind::Smell.can_penetrate_solids());
    }

    #[test]
    fn test_sensor_kind_directional() {
        assert!(SensorKind::Sight.is_directional());
        assert!(SensorKind::Sound.is_directional());
        assert!(!SensorKind::Vibration.is_directional());
        assert!(!SensorKind::Heat.is_directional());
    }

    #[test]
    fn test_sensor_kind_default_range() {
        assert!((SensorKind::Sight.default_range() - 50.0).abs() < f32::EPSILON);
        assert!((SensorKind::Heat.default_range() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sensor_kind_ordering() {
        assert!(SensorKind::Sight < SensorKind::Sound);
        assert!(SensorKind::Sound < SensorKind::ElectricalField);
    }

    #[test]
    fn test_sensor_kind_serde() {
        let kind = SensorKind::Vibration;
        let json = serde_json::to_string(&kind).unwrap();
        let restored: SensorKind = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, SensorKind::Vibration);
    }
}

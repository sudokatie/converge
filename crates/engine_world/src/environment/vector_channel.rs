//! Vector environmental field channel types.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Typed vector environmental field channels.
///
/// Each channel represents a distinct vector field that can vary per-cell
/// within chunks. Vector fields store direction and magnitude for flow,
/// forces, and directional effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum VectorFieldChannel {
    /// Wind velocity. Affects projectiles, particles, sound propagation.
    Wind = 0,

    /// Water current. Affects swimming, floating objects, sediment transport.
    WaterCurrent = 1,

    /// Pressure gradient. Points toward lower pressure regions.
    PressureGradient = 2,

    /// Gravity override. Local gravity direction/magnitude for anti-gravity zones.
    GravityOverride = 3,

    /// Hazard spread direction. Direction of corruption/toxin/spore spread.
    HazardSpread = 4,
}

impl VectorFieldChannel {
    /// Total number of vector field channels.
    pub const COUNT: usize = 5;

    /// Get all channels in order.
    pub const ALL: [VectorFieldChannel; Self::COUNT] = [
        VectorFieldChannel::Wind,
        VectorFieldChannel::WaterCurrent,
        VectorFieldChannel::PressureGradient,
        VectorFieldChannel::GravityOverride,
        VectorFieldChannel::HazardSpread,
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
            0 => Some(VectorFieldChannel::Wind),
            1 => Some(VectorFieldChannel::WaterCurrent),
            2 => Some(VectorFieldChannel::PressureGradient),
            3 => Some(VectorFieldChannel::GravityOverride),
            4 => Some(VectorFieldChannel::HazardSpread),
            _ => None,
        }
    }

    /// Get the default value for this channel.
    ///
    /// Default values represent neutral environmental conditions.
    #[must_use]
    pub fn default_value(self) -> Vec3 {
        match self {
            VectorFieldChannel::Wind
            | VectorFieldChannel::WaterCurrent
            | VectorFieldChannel::PressureGradient
            | VectorFieldChannel::HazardSpread => Vec3::ZERO,
            VectorFieldChannel::GravityOverride => Vec3::new(0.0, -9.81, 0.0),
        }
    }

    /// Get the maximum magnitude for this channel.
    ///
    /// Returns None if unbounded.
    #[must_use]
    pub const fn max_magnitude(self) -> Option<f32> {
        match self {
            VectorFieldChannel::Wind => Some(100.0),
            VectorFieldChannel::WaterCurrent => Some(20.0),
            VectorFieldChannel::PressureGradient | VectorFieldChannel::GravityOverride => None,
            VectorFieldChannel::HazardSpread => Some(1.0),
        }
    }

    /// Get the display name for this channel.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            VectorFieldChannel::Wind => "Wind",
            VectorFieldChannel::WaterCurrent => "Water Current",
            VectorFieldChannel::PressureGradient => "Pressure Gradient",
            VectorFieldChannel::GravityOverride => "Gravity Override",
            VectorFieldChannel::HazardSpread => "Hazard Spread",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_count() {
        assert_eq!(VectorFieldChannel::ALL.len(), VectorFieldChannel::COUNT);
    }

    #[test]
    fn test_as_index_round_trip() {
        for channel in VectorFieldChannel::ALL {
            let index = channel.as_index();
            let recovered = VectorFieldChannel::from_index(index);
            assert_eq!(recovered, Some(channel));
        }
    }

    #[test]
    fn test_from_index_out_of_range() {
        assert_eq!(VectorFieldChannel::from_index(5), None);
        assert_eq!(VectorFieldChannel::from_index(255), None);
    }

    #[test]
    fn test_default_values() {
        assert_eq!(VectorFieldChannel::Wind.default_value(), Vec3::ZERO);
        assert_eq!(
            VectorFieldChannel::GravityOverride.default_value(),
            Vec3::new(0.0, -9.81, 0.0)
        );
    }

    #[test]
    fn test_max_magnitude_bounded() {
        assert!(VectorFieldChannel::Wind.max_magnitude().is_some());
        assert!(
            VectorFieldChannel::PressureGradient
                .max_magnitude()
                .is_none()
        );
    }
}

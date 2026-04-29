//! Distortion effect definitions.
//!
//! Each effect kind has distinct visual properties: distortion pattern,
//! animation behavior, and intensity characteristics.

use glam::Vec3;
use std::f32::consts::TAU;

/// Kind of distortion effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DistortionKind {
    /// Heat shimmer rising from hot surfaces.
    HeatShimmer = 0,
    /// Expanding pressure wave from explosions.
    PressureWave = 1,
    /// Radiation-induced visual warping.
    RadiationWarp = 2,
    /// Reality fracture/dimensional tear effects.
    FractureEvent = 3,
    /// User-defined custom distortion.
    Custom = 4,
}

impl DistortionKind {
    /// All distortion kinds in order.
    pub const ALL: [Self; 5] = [
        Self::HeatShimmer,
        Self::PressureWave,
        Self::RadiationWarp,
        Self::FractureEvent,
        Self::Custom,
    ];

    /// Get the default animation speed for this kind.
    #[must_use]
    pub fn default_animation_speed(self) -> f32 {
        match self {
            Self::HeatShimmer | Self::Custom => 1.0,
            Self::PressureWave => 3.0,
            Self::RadiationWarp => 0.5,
            Self::FractureEvent => 2.0,
        }
    }

    /// Whether this effect type typically uses radial distortion.
    #[must_use]
    pub fn is_radial(self) -> bool {
        matches!(self, Self::PressureWave | Self::FractureEvent)
    }
}

/// Configuration for a distortion effect.
#[derive(Debug, Clone, Copy)]
pub struct DistortionEffect {
    /// Effect type.
    pub kind: DistortionKind,
    /// Distortion strength (0.0 = none, 1.0 = full effect).
    pub strength: f32,
    /// Spatial frequency of distortion pattern.
    pub frequency: f32,
    /// Animation speed multiplier.
    pub animation_speed: f32,
    /// Phase offset for temporal variation (0.0 to TAU).
    pub phase: f32,
    /// Primary flow direction for directional effects.
    pub flow_direction: Vec3,
    /// Secondary distortion amplitude (for multi-layer effects).
    pub secondary_amplitude: f32,
    /// Whether the effect is currently active.
    pub active: bool,
}

impl DistortionEffect {
    /// Create an effect with default parameters for the given kind.
    #[must_use]
    pub fn from_kind(kind: DistortionKind) -> Self {
        match kind {
            DistortionKind::HeatShimmer => Self::heat_shimmer(),
            DistortionKind::PressureWave => Self::pressure_wave(),
            DistortionKind::RadiationWarp => Self::radiation_warp(),
            DistortionKind::FractureEvent => Self::fracture_event(),
            DistortionKind::Custom => Self::custom(),
        }
    }

    /// Heat shimmer effect for hot surfaces.
    #[must_use]
    pub fn heat_shimmer() -> Self {
        Self {
            kind: DistortionKind::HeatShimmer,
            strength: 0.3,
            frequency: 8.0,
            animation_speed: 1.0,
            phase: 0.0,
            flow_direction: Vec3::Y,
            secondary_amplitude: 0.1,
            active: true,
        }
    }

    /// Pressure wave from explosions.
    #[must_use]
    pub fn pressure_wave() -> Self {
        Self {
            kind: DistortionKind::PressureWave,
            strength: 0.8,
            frequency: 4.0,
            animation_speed: 3.0,
            phase: 0.0,
            flow_direction: Vec3::ZERO,
            secondary_amplitude: 0.0,
            active: true,
        }
    }

    /// Radiation warping effect.
    #[must_use]
    pub fn radiation_warp() -> Self {
        Self {
            kind: DistortionKind::RadiationWarp,
            strength: 0.5,
            frequency: 2.0,
            animation_speed: 0.5,
            phase: 0.0,
            flow_direction: Vec3::ZERO,
            secondary_amplitude: 0.3,
            active: true,
        }
    }

    /// Fracture/reality tear effect.
    #[must_use]
    pub fn fracture_event() -> Self {
        Self {
            kind: DistortionKind::FractureEvent,
            strength: 1.0,
            frequency: 6.0,
            animation_speed: 2.0,
            phase: 0.0,
            flow_direction: Vec3::ZERO,
            secondary_amplitude: 0.5,
            active: true,
        }
    }

    /// Custom effect with neutral defaults.
    #[must_use]
    pub fn custom() -> Self {
        Self {
            kind: DistortionKind::Custom,
            strength: 0.5,
            frequency: 4.0,
            animation_speed: 1.0,
            phase: 0.0,
            flow_direction: Vec3::ZERO,
            secondary_amplitude: 0.0,
            active: true,
        }
    }

    /// Set distortion strength.
    #[must_use]
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 2.0);
        self
    }

    /// Set spatial frequency.
    #[must_use]
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.max(0.1);
        self
    }

    /// Set animation speed.
    #[must_use]
    pub fn with_animation_speed(mut self, speed: f32) -> Self {
        self.animation_speed = speed.max(0.0);
        self
    }

    /// Set phase offset.
    #[must_use]
    pub fn with_phase(mut self, phase: f32) -> Self {
        self.phase = phase % TAU;
        self
    }

    /// Set flow direction (will be normalized).
    #[must_use]
    pub fn with_flow_direction(mut self, direction: Vec3) -> Self {
        self.flow_direction = if direction.length_squared() > 0.0001 {
            direction.normalize()
        } else {
            Vec3::ZERO
        };
        self
    }

    /// Set active state.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Interpolate between two effects.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            kind: if t < 0.5 { self.kind } else { other.kind },
            strength: self.strength + (other.strength - self.strength) * t,
            frequency: self.frequency + (other.frequency - self.frequency) * t,
            animation_speed: self.animation_speed
                + (other.animation_speed - self.animation_speed) * t,
            phase: self.phase + (other.phase - self.phase) * t,
            flow_direction: self.flow_direction.lerp(other.flow_direction, t),
            secondary_amplitude: self.secondary_amplitude
                + (other.secondary_amplitude - self.secondary_amplitude) * t,
            active: if t < 0.5 { self.active } else { other.active },
        }
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.strength = self.strength.clamp(0.0, 2.0);
        self.frequency = self.frequency.clamp(0.1, 100.0);
        self.animation_speed = self.animation_speed.clamp(0.0, 10.0);
        self.phase = self.phase.rem_euclid(TAU);
        self.secondary_amplitude = self.secondary_amplitude.clamp(0.0, 1.0);
        self
    }

    /// Check if values are within valid ranges.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.strength >= 0.0
            && self.strength <= 2.0
            && self.frequency >= 0.1
            && self.frequency <= 100.0
            && self.animation_speed >= 0.0
            && self.animation_speed <= 10.0
            && self.secondary_amplitude >= 0.0
            && self.secondary_amplitude <= 1.0
    }

    /// Compute effective strength at a given time.
    #[must_use]
    pub fn strength_at_time(&self, time: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let phase_offset = (time * self.animation_speed + self.phase).sin();
        let modulation = 0.8 + 0.2 * phase_offset;
        self.strength * modulation
    }
}

impl Default for DistortionEffect {
    fn default() -> Self {
        Self::heat_shimmer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_all_kinds_have_constructors() {
        for kind in DistortionKind::ALL {
            let effect = DistortionEffect::from_kind(kind);
            assert_eq!(effect.kind, kind);
            assert!(effect.is_valid());
        }
    }

    #[test]
    fn test_heat_shimmer_flows_up() {
        let effect = DistortionEffect::heat_shimmer();
        assert!(
            effect.flow_direction.y > 0.0,
            "heat shimmer should flow upward"
        );
    }

    #[test]
    fn test_pressure_wave_is_radial() {
        assert!(DistortionKind::PressureWave.is_radial());
        assert!(!DistortionKind::HeatShimmer.is_radial());
    }

    #[test]
    fn test_fracture_high_strength() {
        let effect = DistortionEffect::fracture_event();
        assert!(
            effect.strength >= 1.0,
            "fracture event should have high strength"
        );
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = DistortionEffect::heat_shimmer();
        let b = DistortionEffect::pressure_wave();

        let at_a = a.lerp(b, 0.0);
        assert_relative_eq!(at_a.strength, a.strength, epsilon = 0.001);

        let at_b = a.lerp(b, 1.0);
        assert_relative_eq!(at_b.strength, b.strength, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = DistortionEffect::heat_shimmer();
        let b = DistortionEffect::radiation_warp();

        let mid = a.lerp(b, 0.5);
        let expected = f32::midpoint(a.strength, b.strength);
        assert_relative_eq!(mid.strength, expected, epsilon = 0.001);
    }

    #[test]
    fn test_clamping() {
        let effect = DistortionEffect::custom()
            .with_strength(5.0)
            .with_frequency(0.01)
            .clamped();

        assert!(effect.is_valid());
        assert_relative_eq!(effect.strength, 2.0, epsilon = 0.001);
        assert_relative_eq!(effect.frequency, 0.1, epsilon = 0.001);
    }

    #[test]
    fn test_strength_at_time_inactive() {
        let effect = DistortionEffect::heat_shimmer().with_active(false);
        assert_relative_eq!(effect.strength_at_time(1.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_strength_at_time_modulation() {
        let effect = DistortionEffect::heat_shimmer();
        let s1 = effect.strength_at_time(0.0);
        let s2 = effect.strength_at_time(1.0);

        assert!(s1 > 0.0);
        assert!(s2 > 0.0);
        assert!(
            (s1 - s2).abs() < effect.strength,
            "modulation should be subtle"
        );
    }

    #[test]
    fn test_with_flow_direction_normalizes() {
        let effect = DistortionEffect::custom().with_flow_direction(Vec3::new(3.0, 4.0, 0.0));
        assert_relative_eq!(effect.flow_direction.length(), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_with_flow_direction_zero_safe() {
        let effect = DistortionEffect::custom().with_flow_direction(Vec3::ZERO);
        assert_eq!(effect.flow_direction, Vec3::ZERO);
    }

    #[test]
    fn test_phase_wrapping() {
        let effect = DistortionEffect::custom().with_phase(TAU * 2.5);
        assert!(effect.phase >= 0.0 && effect.phase < TAU);
    }
}

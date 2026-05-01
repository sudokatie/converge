//! Weather effect definitions.
//!
//! Each effect kind has distinct visual properties: particle behavior,
//! environmental modifiers, and rendering characteristics.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Kind of weather effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum WeatherKind {
    /// Rain/drizzle precipitation.
    #[default]
    Rain = 0,
    /// Snow/blizzard precipitation.
    Snow = 1,
    /// Underwater particles (bubbles, sediment).
    Underwater = 2,
    /// Dust/sand particles.
    Dust = 3,
    /// Spore/pollen particles.
    Spores = 4,
    /// Ash/ember particles.
    Ash = 5,
    /// Vacuum/zero-g debris.
    Vacuum = 6,
    /// Fog/mist particles.
    Fog = 7,
    /// User-defined custom weather.
    Custom = 8,
}

impl WeatherKind {
    /// All weather kinds in order.
    pub const ALL: [Self; 9] = [
        Self::Rain,
        Self::Snow,
        Self::Underwater,
        Self::Dust,
        Self::Spores,
        Self::Ash,
        Self::Vacuum,
        Self::Fog,
        Self::Custom,
    ];

    /// Get the default gravity multiplier for this kind.
    #[must_use]
    pub fn default_gravity(self) -> f32 {
        match self {
            Self::Rain => 1.0,
            Self::Snow => 0.3,
            Self::Underwater => -0.1,
            Self::Dust => 0.05,
            Self::Spores => 0.02,
            Self::Ash => 0.15,
            Self::Vacuum | Self::Fog => 0.0,
            Self::Custom => 0.5,
        }
    }

    /// Get the default particle lifetime range in seconds.
    #[must_use]
    pub fn default_lifetime(self) -> (f32, f32) {
        match self {
            Self::Rain => (0.5, 1.5),
            Self::Snow | Self::Fog => (3.0, 8.0),
            Self::Underwater => (2.0, 5.0),
            Self::Dust => (4.0, 10.0),
            Self::Spores => (5.0, 12.0),
            Self::Ash => (2.0, 6.0),
            Self::Vacuum => (8.0, 20.0),
            Self::Custom => (1.0, 5.0),
        }
    }

    /// Whether this kind typically uses collision detection.
    #[must_use]
    pub fn uses_collision(self) -> bool {
        matches!(self, Self::Rain | Self::Snow | Self::Ash)
    }

    /// Whether this kind typically has wind influence.
    #[must_use]
    pub fn has_wind_influence(self) -> bool {
        !matches!(self, Self::Underwater | Self::Vacuum)
    }
}

/// Configuration for a weather effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherEffect {
    /// Effect type.
    pub kind: WeatherKind,
    /// Intensity (0.0 = none, 1.0 = full effect).
    pub intensity: f32,
    /// Wind direction and strength (magnitude = speed in units/sec).
    pub wind: Vec3,
    /// Gravity multiplier (negative = upward).
    pub gravity: f32,
    /// Turbulence strength (0.0 = none, 1.0 = chaotic).
    pub turbulence: f32,
    /// Turbulence frequency (spatial scale).
    pub turbulence_frequency: f32,
    /// Color tint for particles (RGB).
    pub color: Vec3,
    /// Alpha/opacity multiplier.
    pub opacity: f32,
    /// Base particle size.
    pub particle_size: f32,
    /// Size variation (+/- this amount).
    pub size_variation: f32,
    /// Whether the effect is currently active.
    pub active: bool,
}

impl WeatherEffect {
    /// Create an effect with default parameters for the given kind.
    #[must_use]
    pub fn from_kind(kind: WeatherKind) -> Self {
        match kind {
            WeatherKind::Rain => Self::rain(),
            WeatherKind::Snow => Self::snow(),
            WeatherKind::Underwater => Self::underwater(),
            WeatherKind::Dust => Self::dust(),
            WeatherKind::Spores => Self::spores(),
            WeatherKind::Ash => Self::ash(),
            WeatherKind::Vacuum => Self::vacuum(),
            WeatherKind::Fog => Self::fog(),
            WeatherKind::Custom => Self::custom(),
        }
    }

    /// Rain effect preset.
    #[must_use]
    pub fn rain() -> Self {
        Self {
            kind: WeatherKind::Rain,
            intensity: 0.7,
            wind: Vec3::new(0.5, 0.0, 0.0),
            gravity: 1.0,
            turbulence: 0.1,
            turbulence_frequency: 2.0,
            color: Vec3::new(0.7, 0.8, 0.9),
            opacity: 0.6,
            particle_size: 0.02,
            size_variation: 0.005,
            active: true,
        }
    }

    /// Snow/blizzard effect preset.
    #[must_use]
    pub fn snow() -> Self {
        Self {
            kind: WeatherKind::Snow,
            intensity: 0.5,
            wind: Vec3::new(1.0, 0.0, 0.3),
            gravity: 0.3,
            turbulence: 0.4,
            turbulence_frequency: 1.5,
            color: Vec3::ONE,
            opacity: 0.8,
            particle_size: 0.03,
            size_variation: 0.015,
            active: true,
        }
    }

    /// Underwater particle effect preset.
    #[must_use]
    pub fn underwater() -> Self {
        Self {
            kind: WeatherKind::Underwater,
            intensity: 0.3,
            wind: Vec3::new(0.0, 0.2, 0.0),
            gravity: -0.1,
            turbulence: 0.2,
            turbulence_frequency: 0.8,
            color: Vec3::new(0.6, 0.8, 1.0),
            opacity: 0.4,
            particle_size: 0.01,
            size_variation: 0.008,
            active: true,
        }
    }

    /// Dust/sand particle effect preset.
    #[must_use]
    pub fn dust() -> Self {
        Self {
            kind: WeatherKind::Dust,
            intensity: 0.4,
            wind: Vec3::new(2.0, 0.0, 0.5),
            gravity: 0.05,
            turbulence: 0.6,
            turbulence_frequency: 1.0,
            color: Vec3::new(0.8, 0.7, 0.5),
            opacity: 0.5,
            particle_size: 0.008,
            size_variation: 0.004,
            active: true,
        }
    }

    /// Spore/pollen effect preset.
    #[must_use]
    pub fn spores() -> Self {
        Self {
            kind: WeatherKind::Spores,
            intensity: 0.2,
            wind: Vec3::new(0.3, 0.1, 0.2),
            gravity: 0.02,
            turbulence: 0.7,
            turbulence_frequency: 0.5,
            color: Vec3::new(0.9, 1.0, 0.6),
            opacity: 0.6,
            particle_size: 0.005,
            size_variation: 0.002,
            active: true,
        }
    }

    /// Ash/ember effect preset.
    #[must_use]
    pub fn ash() -> Self {
        Self {
            kind: WeatherKind::Ash,
            intensity: 0.5,
            wind: Vec3::new(0.8, 0.3, 0.2),
            gravity: 0.15,
            turbulence: 0.5,
            turbulence_frequency: 1.2,
            color: Vec3::new(0.3, 0.3, 0.3),
            opacity: 0.7,
            particle_size: 0.015,
            size_variation: 0.01,
            active: true,
        }
    }

    /// Vacuum/zero-g debris effect preset.
    #[must_use]
    pub fn vacuum() -> Self {
        Self {
            kind: WeatherKind::Vacuum,
            intensity: 0.15,
            wind: Vec3::ZERO,
            gravity: 0.0,
            turbulence: 0.1,
            turbulence_frequency: 0.3,
            color: Vec3::new(0.5, 0.5, 0.6),
            opacity: 0.5,
            particle_size: 0.02,
            size_variation: 0.015,
            active: true,
        }
    }

    /// Fog/mist effect preset.
    #[must_use]
    pub fn fog() -> Self {
        Self {
            kind: WeatherKind::Fog,
            intensity: 0.6,
            wind: Vec3::new(0.2, 0.0, 0.1),
            gravity: 0.0,
            turbulence: 0.3,
            turbulence_frequency: 0.4,
            color: Vec3::new(0.9, 0.9, 0.95),
            opacity: 0.3,
            particle_size: 0.1,
            size_variation: 0.05,
            active: true,
        }
    }

    /// Custom effect with neutral defaults.
    #[must_use]
    pub fn custom() -> Self {
        Self {
            kind: WeatherKind::Custom,
            intensity: 0.5,
            wind: Vec3::ZERO,
            gravity: 0.5,
            turbulence: 0.3,
            turbulence_frequency: 1.0,
            color: Vec3::ONE,
            opacity: 0.5,
            particle_size: 0.02,
            size_variation: 0.01,
            active: true,
        }
    }

    /// Set intensity.
    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set wind.
    #[must_use]
    pub fn with_wind(mut self, wind: Vec3) -> Self {
        self.wind = wind;
        self
    }

    /// Set gravity multiplier.
    #[must_use]
    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity.clamp(-2.0, 2.0);
        self
    }

    /// Set turbulence strength.
    #[must_use]
    pub fn with_turbulence(mut self, turbulence: f32) -> Self {
        self.turbulence = turbulence.clamp(0.0, 2.0);
        self
    }

    /// Set color tint.
    #[must_use]
    pub fn with_color(mut self, color: Vec3) -> Self {
        self.color = color.clamp(Vec3::ZERO, Vec3::ONE);
        self
    }

    /// Set opacity.
    #[must_use]
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set particle size.
    #[must_use]
    pub fn with_particle_size(mut self, size: f32) -> Self {
        self.particle_size = size.max(0.001);
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
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            kind: if t < 0.5 { self.kind } else { other.kind },
            intensity: self.intensity + (other.intensity - self.intensity) * t,
            wind: self.wind.lerp(other.wind, t),
            gravity: self.gravity + (other.gravity - self.gravity) * t,
            turbulence: self.turbulence + (other.turbulence - self.turbulence) * t,
            turbulence_frequency: self.turbulence_frequency
                + (other.turbulence_frequency - self.turbulence_frequency) * t,
            color: self.color.lerp(other.color, t),
            opacity: self.opacity + (other.opacity - self.opacity) * t,
            particle_size: self.particle_size + (other.particle_size - self.particle_size) * t,
            size_variation: self.size_variation + (other.size_variation - self.size_variation) * t,
            active: if t < 0.5 { self.active } else { other.active },
        }
    }

    /// Clamp all values to valid ranges.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.intensity = self.intensity.clamp(0.0, 1.0);
        self.gravity = self.gravity.clamp(-2.0, 2.0);
        self.turbulence = self.turbulence.clamp(0.0, 2.0);
        self.turbulence_frequency = self.turbulence_frequency.clamp(0.1, 10.0);
        self.color = self.color.clamp(Vec3::ZERO, Vec3::ONE);
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.particle_size = self.particle_size.clamp(0.001, 1.0);
        self.size_variation = self.size_variation.clamp(0.0, self.particle_size);
        self
    }

    /// Check if values are within valid ranges.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.intensity >= 0.0
            && self.intensity <= 1.0
            && self.gravity >= -2.0
            && self.gravity <= 2.0
            && self.turbulence >= 0.0
            && self.turbulence <= 2.0
            && self.turbulence_frequency >= 0.1
            && self.opacity >= 0.0
            && self.opacity <= 1.0
            && self.particle_size >= 0.001
    }

    /// Get spawn rate (particles per second) based on intensity.
    #[must_use]
    pub fn spawn_rate(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let base_rate = match self.kind {
            WeatherKind::Rain => 500.0,
            WeatherKind::Snow => 200.0,
            WeatherKind::Dust => 150.0,
            WeatherKind::Underwater | WeatherKind::Ash | WeatherKind::Custom => 100.0,
            WeatherKind::Spores => 50.0,
            WeatherKind::Vacuum => 20.0,
            WeatherKind::Fog => 80.0,
        };
        base_rate * self.intensity
    }
}

impl Default for WeatherEffect {
    fn default() -> Self {
        Self::rain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_all_kinds_have_constructors() {
        for kind in WeatherKind::ALL {
            let effect = WeatherEffect::from_kind(kind);
            assert_eq!(effect.kind, kind);
            assert!(effect.is_valid());
        }
    }

    #[test]
    fn test_rain_gravity_positive() {
        let effect = WeatherEffect::rain();
        assert!(effect.gravity > 0.0, "rain should fall down");
    }

    #[test]
    fn test_underwater_gravity_negative() {
        let effect = WeatherEffect::underwater();
        assert!(effect.gravity < 0.0, "underwater particles should rise");
    }

    #[test]
    fn test_vacuum_no_gravity() {
        let effect = WeatherEffect::vacuum();
        assert_relative_eq!(effect.gravity, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = WeatherEffect::rain();
        let b = WeatherEffect::snow();

        let at_a = a.clone().lerp(&b, 0.0);
        assert_relative_eq!(at_a.intensity, a.intensity, epsilon = 0.001);

        let at_b = a.lerp(&b, 1.0);
        assert_relative_eq!(at_b.intensity, b.intensity, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = WeatherEffect::rain();
        let b = WeatherEffect::snow();

        let mid = a.clone().lerp(&b, 0.5);
        let expected = f32::midpoint(a.intensity, b.intensity);
        assert_relative_eq!(mid.intensity, expected, epsilon = 0.001);
    }

    #[test]
    fn test_clamping() {
        let effect = WeatherEffect::custom()
            .with_intensity(2.0)
            .with_gravity(5.0)
            .with_turbulence(10.0)
            .clamped();

        assert!(effect.is_valid());
        assert_relative_eq!(effect.intensity, 1.0, epsilon = 0.001);
        assert_relative_eq!(effect.gravity, 2.0, epsilon = 0.001);
        assert_relative_eq!(effect.turbulence, 2.0, epsilon = 0.001);
    }

    #[test]
    fn test_spawn_rate_inactive() {
        let effect = WeatherEffect::rain().with_active(false);
        assert_relative_eq!(effect.spawn_rate(), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_spawn_rate_scales_with_intensity() {
        let low = WeatherEffect::rain().with_intensity(0.2);
        let high = WeatherEffect::rain().with_intensity(0.8);

        assert!(high.spawn_rate() > low.spawn_rate());
    }

    #[test]
    fn test_wind_influence() {
        assert!(WeatherKind::Rain.has_wind_influence());
        assert!(!WeatherKind::Underwater.has_wind_influence());
        assert!(!WeatherKind::Vacuum.has_wind_influence());
    }

    #[test]
    fn test_uses_collision() {
        assert!(WeatherKind::Rain.uses_collision());
        assert!(WeatherKind::Snow.uses_collision());
        assert!(!WeatherKind::Spores.uses_collision());
    }

    #[test]
    fn test_default_lifetime_ranges() {
        for kind in WeatherKind::ALL {
            let (min, max) = kind.default_lifetime();
            assert!(min > 0.0);
            assert!(max > min);
        }
    }
}

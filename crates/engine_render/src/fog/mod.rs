//! Distance-based fog system for hiding chunk pop-in.
//!
//! Implements spec 3.5.3: distance-based fog that matches the sky at horizon,
//! with density that increases at night.

use bytemuck::{Pod, Zeroable};

/// GPU-friendly fog uniform data.
///
/// Matches the expected layout in WGSL:
/// ```wgsl
/// struct FogUniform {
///     color: vec4<f32>,
///     density: f32,
///     start_distance: f32,
///     end_distance: f32,
///     enabled: u32,
/// }
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FogUniform {
    /// Fog color (RGBA, alpha unused).
    pub color: [f32; 4],
    /// Fog density (0.0 = no fog, 1.0 = max fog).
    pub density: f32,
    /// Distance where fog starts.
    pub start_distance: f32,
    /// Distance where fog is fully opaque.
    pub end_distance: f32,
    /// Whether fog is enabled (1 = enabled, 0 = disabled).
    pub enabled: u32,
}

impl Default for FogUniform {
    fn default() -> Self {
        Self {
            color: [0.7, 0.75, 0.85, 1.0], // Horizon sky blue
            density: 0.5,
            start_distance: 100.0,
            end_distance: 200.0,
            enabled: 1,
        }
    }
}

impl FogUniform {
    /// Create fog config from time of day.
    ///
    /// Daytime: light fog for chunk pop-in hiding.
    /// Nighttime: heavier, darker fog.
    pub fn from_time_of_day(time_of_day: f32, view_distance: i32) -> Self {
        let sun_elevation = Self::sun_elevation(time_of_day);
        let sun_up = sun_elevation.max(0.0);

        // Fog color matches sky at horizon
        let color = if sun_up > 0.4 {
            // Day: sky blue at horizon
            [0.7, 0.75, 0.85, 1.0]
        } else if sun_up > 0.0 {
            // Sunset/sunrise: warm orange-pink
            let t = sun_up / 0.4;
            [
                0.9 - 0.2 * t,
                0.55 + 0.2 * t,
                0.4 + 0.45 * t,
                1.0,
            ]
        } else {
            // Night: dark blue-grey
            [0.08, 0.08, 0.15, 1.0]
        };

        // Density increases at night
        let density = if sun_up > 0.4 {
            0.3 // Light daytime fog
        } else if sun_up > 0.0 {
            0.3 + 0.4 * (1.0 - sun_up / 0.4) // Transition
        } else {
            0.7 // Heavy nighttime fog
        };

        // Scale distances to view distance
        let max_dist = (view_distance * 16) as f32; // chunks to blocks
        let start_distance = max_dist * 0.5;
        let end_distance = max_dist * 0.85;

        Self {
            color,
            density,
            start_distance,
            end_distance,
            enabled: 1,
        }
    }

    /// Calculate sun elevation from time of day (0.0-1.0).
    fn sun_elevation(time_of_day: f32) -> f32 {
        // Sinusoidal: 0.25 = sunrise, 0.5 = noon, 0.75 = sunset
        let angle = (time_of_day - 0.25) * std::f32::consts::TAU;
        angle.sin()
    }

    /// Disable fog.
    pub fn disabled() -> Self {
        Self {
            enabled: 0,
            ..Default::default()
        }
    }
}

/// Fog configuration for runtime control.
#[derive(Debug, Clone)]
pub struct FogConfig {
    /// Whether fog is enabled.
    pub enabled: bool,
    /// Fog density multiplier (0.0-2.0, default 1.0).
    pub density_multiplier: f32,
    /// Fog start distance multiplier (default 1.0).
    pub start_multiplier: f32,
    /// Fog end distance multiplier (default 1.0).
    pub end_multiplier: f32,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            density_multiplier: 1.0,
            start_multiplier: 1.0,
            end_multiplier: 1.0,
        }
    }
}

impl FogConfig {
    /// Create a new fog config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply config multipliers to a fog uniform.
    #[must_use]
    pub fn apply(&self, uniform: FogUniform) -> FogUniform {
        FogUniform {
            density: uniform.density * self.density_multiplier,
            start_distance: uniform.start_distance * self.start_multiplier,
            end_distance: uniform.end_distance * self.end_multiplier,
            enabled: if self.enabled { 1 } else { 0 },
            ..uniform
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fog() {
        let fog = FogUniform::default();
        assert_eq!(fog.enabled, 1);
        assert!(fog.density > 0.0);
        assert!(fog.start_distance < fog.end_distance);
    }

    #[test]
    fn test_daytime_fog() {
        let fog = FogUniform::from_time_of_day(0.5, 12); // Noon
        assert_eq!(fog.enabled, 1);
        assert!(fog.density < 0.5, "Daytime fog should be light");
        // Daytime color should be blueish
        assert!(fog.color[2] > fog.color[0], "Daytime fog should be blueish");
    }

    #[test]
    fn test_nighttime_fog() {
        let fog = FogUniform::from_time_of_day(0.0, 12); // Midnight
        assert_eq!(fog.enabled, 1);
        assert!(fog.density > 0.5, "Nighttime fog should be heavy");
        // Nighttime color should be dark
        assert!(fog.color[0] < 0.2, "Nighttime fog should be dark");
    }

    #[test]
    fn test_sunset_fog() {
        // Use time well into sunset for visible warm color (sun_up ~0.06)
        let fog = FogUniform::from_time_of_day(0.74, 12);
        assert_eq!(fog.enabled, 1);
        // Sunset color should be warm (red > blue)
        assert!(fog.color[0] > fog.color[2], "Sunset fog should be warm, got R={} B={}", fog.color[0], fog.color[2]);
    }

    #[test]
    fn test_disabled_fog() {
        let fog = FogUniform::disabled();
        assert_eq!(fog.enabled, 0);
    }

    #[test]
    fn test_view_distance_scaling() {
        let fog_near = FogUniform::from_time_of_day(0.5, 8);
        let fog_far = FogUniform::from_time_of_day(0.5, 16);

        assert!(
            fog_far.end_distance > fog_near.end_distance,
            "Farther view distance should have farther fog"
        );
        assert!(
            fog_far.start_distance > fog_near.start_distance,
            "Farther view distance should have farther fog start"
        );
    }

    #[test]
    fn test_fog_config_apply() {
        let config = FogConfig {
            enabled: false,
            density_multiplier: 2.0,
            start_multiplier: 0.5,
            end_multiplier: 1.5,
            ..Default::default()
        };

        let base = FogUniform::from_time_of_day(0.5, 12);
        let applied = config.apply(base);

        assert_eq!(applied.enabled, 0, "Config disabled should disable fog");
        assert!(
            (applied.density - base.density * 2.0).abs() < 0.001,
            "Density should be doubled"
        );
        assert!(
            (applied.start_distance - base.start_distance * 0.5).abs() < 0.001,
            "Start should be halved"
        );
        assert!(
            (applied.end_distance - base.end_distance * 1.5).abs() < 0.001,
            "End should be 1.5x"
        );
    }

    #[test]
    fn test_sun_elevation() {
        // Noon: maximum elevation
        assert!(
            FogUniform::sun_elevation(0.5) > 0.9,
            "Noon should have high sun"
        );
        // Midnight: below horizon
        assert!(
            FogUniform::sun_elevation(0.0) < -0.9,
            "Midnight should have sun below horizon"
        );
        // Sunrise/sunset: near zero
        assert!(
            FogUniform::sun_elevation(0.25).abs() < 0.1,
            "Sunrise should be near horizon"
        );
        assert!(
            FogUniform::sun_elevation(0.75).abs() < 0.1,
            "Sunset should be near horizon"
        );
    }

    #[test]
    fn test_fog_start_before_end() {
        for time in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for vd in [4, 8, 12, 16, 24] {
                let fog = FogUniform::from_time_of_day(time, vd);
                assert!(
                    fog.start_distance < fog.end_distance,
                    "Fog start must be before end for time={time}, vd={vd}"
                );
            }
        }
    }
}

//! Directional light (sun) implementation.
//!
//! Calculates sun position based on time of day and provides color temperature
//! shifts for sunrise, midday, and sunset.

use glam::{Mat4, Vec3};

/// Directional light representing the sun.
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    /// Normalized direction from which light comes.
    pub direction: Vec3,
    /// Light color (linear RGB, pre-multiplied by intensity).
    pub color: Vec3,
    /// Shadow map view-projection matrix.
    pub light_view_proj: Mat4,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.3, 1.0, 0.2).normalize(),
            color: Vec3::splat(1.0),
            light_view_proj: Mat4::IDENTITY,
        }
    }
}

impl DirectionalLight {
    /// Create a new directional light.
    #[must_use]
    pub fn new(direction: Vec3, color: Vec3) -> Self {
        Self {
            direction: direction.normalize(),
            color,
            light_view_proj: Mat4::IDENTITY,
        }
    }

    /// Update sun position and color based on time of day.
    ///
    /// `time_normalized` is in range [0, 1] where:
    /// - 0.0 = midnight
    /// - 0.25 = sunrise (6 AM)
    /// - 0.5 = noon
    /// - 0.75 = sunset (6 PM)
    /// - 1.0 = midnight
    pub fn update_from_time(&mut self, time_normalized: f32) {
        // Sun angle (0 at sunrise, PI at sunset)
        let sun_angle = (time_normalized - 0.25) * std::f32::consts::TAU;

        // Sun position on circular path
        // Y is up, sun moves in XY plane
        let sun_height = sun_angle.sin();
        let sun_horizontal = sun_angle.cos();

        // Sun is below horizon at night
        self.direction = Vec3::new(sun_horizontal * 0.3, sun_height, 0.5).normalize();

        // Color temperature based on sun elevation
        self.color = Self::calculate_color_temperature(sun_height);
    }

    /// Calculate color temperature based on sun elevation.
    fn calculate_color_temperature(sun_height: f32) -> Vec3 {
        // Warm colors near horizon, cool at zenith
        if sun_height <= 0.0 {
            // Night - very dim bluish light (moonlight)
            return Vec3::new(0.05, 0.05, 0.1);
        }

        // Interpolate between sunrise/sunset warmth and midday coolness
        let t = sun_height.clamp(0.0, 1.0);

        // Sunrise/sunset: warm orange (color temp ~2000K)
        let warm = Vec3::new(1.0, 0.6, 0.3);

        // Midday: neutral white with slight blue (color temp ~6500K)
        let cool = Vec3::new(1.0, 0.98, 0.95);

        // Ease in-out interpolation for smoother transition
        let t_smooth = t * t * (3.0 - 2.0 * t);

        // Intensity also varies with sun height
        let intensity = 0.3 + 0.7 * t;

        warm.lerp(cool, t_smooth) * intensity
    }

    /// Update shadow map view-projection matrix.
    pub fn update_shadow_matrix(&mut self, focus_point: Vec3, shadow_distance: f32) {
        // Shadow camera looks from sun direction toward focus point
        let light_pos = focus_point - self.direction * shadow_distance;

        let view = Mat4::look_at_rh(light_pos, focus_point, Vec3::Y);

        // Orthographic projection for directional light shadows
        let half_size = shadow_distance * 0.5;
        let proj = Mat4::orthographic_rh(
            -half_size,
            half_size,
            -half_size,
            half_size,
            0.1,
            shadow_distance * 2.0,
        );

        self.light_view_proj = proj * view;
    }

    /// Check if the sun is above the horizon.
    #[must_use]
    pub fn is_daytime(&self) -> bool {
        self.direction.y > 0.0
    }

    /// Get sun elevation angle in radians.
    #[must_use]
    pub fn elevation(&self) -> f32 {
        self.direction.y.asin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_direction_normalized() {
        let light = DirectionalLight::default();
        assert_relative_eq!(light.direction.length(), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_noon_sun_high() {
        let mut light = DirectionalLight::default();
        light.update_from_time(0.5); // Noon
        assert!(light.direction.y > 0.8, "Sun should be high at noon");
        assert!(light.is_daytime());
    }

    #[test]
    fn test_midnight_sun_low() {
        let mut light = DirectionalLight::default();
        light.update_from_time(0.0); // Midnight
        assert!(light.direction.y < 0.0, "Sun should be below horizon at midnight");
        assert!(!light.is_daytime());
    }

    #[test]
    fn test_color_temperature_shifts() {
        let mut light = DirectionalLight::default();

        // Morning (slightly after sunrise) - warm
        light.update_from_time(0.28);
        assert!(light.color.x > light.color.z, "Morning should be warm (more red than blue)");

        // Noon - neutral/cool
        light.update_from_time(0.5);
        let noon_ratio = light.color.x / light.color.z;

        // Evening (slightly before sunset) - warm
        light.update_from_time(0.72);
        assert!(light.color.x > light.color.z, "Evening should be warm");

        // Noon should be closer to neutral than morning/evening
        assert!(noon_ratio < 1.5, "Noon should be more neutral");
    }

    #[test]
    fn test_shadow_matrix_valid() {
        let mut light = DirectionalLight::new(
            Vec3::new(0.5, 1.0, 0.3).normalize(),
            Vec3::ONE,
        );
        light.update_shadow_matrix(Vec3::ZERO, 100.0);

        // Matrix should not be identity (was updated)
        assert!(
            light.light_view_proj != Mat4::IDENTITY,
            "Shadow matrix should be updated from default"
        );

        // Orthographic projection should produce a valid matrix
        let col0 = light.light_view_proj.x_axis;
        assert!(
            col0.length() > 0.0,
            "Shadow matrix should have non-zero columns"
        );
    }
}

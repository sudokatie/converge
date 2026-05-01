//! Photo mode settings and camera configuration.
//!
//! Defines the core settings available in photo mode including
//! exposure, aperture, focus, field of view, and roll.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Photo mode camera settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhotoSettings {
    /// Exposure compensation in EV stops (-5.0 to +5.0).
    pub exposure: f32,
    /// Aperture f-stop for depth of field (1.4 to 22.0).
    pub aperture: f32,
    /// Focus distance in world units.
    pub focus_distance: f32,
    /// Vertical field of view in degrees (10.0 to 120.0).
    pub fov: f32,
    /// Camera roll in degrees (-180.0 to 180.0).
    pub roll: f32,
    /// Time freeze state.
    pub time_frozen: bool,
    /// UI visibility.
    pub ui_visible: bool,
    /// Active filter.
    pub filter: PhotoFilter,
    /// Camera position override.
    pub position: Option<Vec3>,
    /// Camera rotation override.
    pub rotation: Option<Quat>,
}

impl Default for PhotoSettings {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            aperture: 5.6,
            focus_distance: 10.0,
            fov: 70.0,
            roll: 0.0,
            time_frozen: false,
            ui_visible: false,
            filter: PhotoFilter::None,
            position: None,
            rotation: None,
        }
    }
}

impl PhotoSettings {
    /// Create new default photo settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set exposure compensation.
    #[must_use]
    pub fn with_exposure(mut self, ev: f32) -> Self {
        self.exposure = ev.clamp(-5.0, 5.0);
        self
    }

    /// Set aperture f-stop.
    #[must_use]
    pub fn with_aperture(mut self, fstop: f32) -> Self {
        self.aperture = fstop.clamp(1.4, 22.0);
        self
    }

    /// Set focus distance.
    #[must_use]
    pub fn with_focus_distance(mut self, distance: f32) -> Self {
        self.focus_distance = distance.max(0.1);
        self
    }

    /// Set field of view.
    #[must_use]
    pub fn with_fov(mut self, fov_degrees: f32) -> Self {
        self.fov = fov_degrees.clamp(10.0, 120.0);
        self
    }

    /// Set camera roll.
    #[must_use]
    pub fn with_roll(mut self, roll_degrees: f32) -> Self {
        self.roll = roll_degrees.clamp(-180.0, 180.0);
        self
    }

    /// Set time frozen state.
    #[must_use]
    pub fn with_time_frozen(mut self, frozen: bool) -> Self {
        self.time_frozen = frozen;
        self
    }

    /// Set UI visibility.
    #[must_use]
    pub fn with_ui_visible(mut self, visible: bool) -> Self {
        self.ui_visible = visible;
        self
    }

    /// Set active filter.
    #[must_use]
    pub fn with_filter(mut self, filter: PhotoFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set camera position override.
    #[must_use]
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = Some(position);
        self
    }

    /// Set camera rotation override.
    #[must_use]
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = Some(rotation);
        self
    }

    /// Clear position override.
    #[must_use]
    pub fn without_position_override(mut self) -> Self {
        self.position = None;
        self
    }

    /// Clear rotation override.
    #[must_use]
    pub fn without_rotation_override(mut self) -> Self {
        self.rotation = None;
        self
    }

    /// Calculate depth of field blur radius from aperture.
    #[must_use]
    pub fn blur_radius(&self) -> f32 {
        (22.0 - self.aperture) / 20.6
    }

    /// Calculate exposure multiplier from EV compensation.
    #[must_use]
    pub fn exposure_multiplier(&self) -> f32 {
        2.0_f32.powf(self.exposure)
    }

    /// Validate settings are within acceptable ranges.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (-5.0..=5.0).contains(&self.exposure)
            && (1.4..=22.0).contains(&self.aperture)
            && self.focus_distance >= 0.1
            && (10.0..=120.0).contains(&self.fov)
            && (-180.0..=180.0).contains(&self.roll)
    }

    /// Normalize settings to valid ranges.
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self {
            exposure: self.exposure.clamp(-5.0, 5.0),
            aperture: self.aperture.clamp(1.4, 22.0),
            focus_distance: self.focus_distance.max(0.1),
            fov: self.fov.clamp(10.0, 120.0),
            roll: self.roll.clamp(-180.0, 180.0),
            time_frozen: self.time_frozen,
            ui_visible: self.ui_visible,
            filter: self.filter,
            position: self.position,
            rotation: self.rotation,
        }
    }
}

/// Photo mode visual filters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PhotoFilter {
    /// No filter applied.
    #[default]
    None = 0,
    /// Black and white conversion.
    BlackAndWhite = 1,
    /// Sepia tone.
    Sepia = 2,
    /// Vintage film look.
    Vintage = 3,
    /// High contrast.
    HighContrast = 4,
    /// Cool temperature shift.
    Cool = 5,
    /// Warm temperature shift.
    Warm = 6,
    /// Cinematic color grading.
    Cinematic = 7,
    /// Neon/cyberpunk style.
    Neon = 8,
    /// Desaturated bleach bypass.
    BleachBypass = 9,
    /// Film noir style.
    Noir = 10,
    /// Dreamy soft focus.
    Dream = 11,
}

impl PhotoFilter {
    /// All available filters.
    pub const ALL: [PhotoFilter; 12] = [
        PhotoFilter::None,
        PhotoFilter::BlackAndWhite,
        PhotoFilter::Sepia,
        PhotoFilter::Vintage,
        PhotoFilter::HighContrast,
        PhotoFilter::Cool,
        PhotoFilter::Warm,
        PhotoFilter::Cinematic,
        PhotoFilter::Neon,
        PhotoFilter::BleachBypass,
        PhotoFilter::Noir,
        PhotoFilter::Dream,
    ];

    /// Get filter name for display.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            PhotoFilter::None => "None",
            PhotoFilter::BlackAndWhite => "Black & White",
            PhotoFilter::Sepia => "Sepia",
            PhotoFilter::Vintage => "Vintage",
            PhotoFilter::HighContrast => "High Contrast",
            PhotoFilter::Cool => "Cool",
            PhotoFilter::Warm => "Warm",
            PhotoFilter::Cinematic => "Cinematic",
            PhotoFilter::Neon => "Neon",
            PhotoFilter::BleachBypass => "Bleach Bypass",
            PhotoFilter::Noir => "Noir",
            PhotoFilter::Dream => "Dream",
        }
    }

    /// Whether this filter applies color grading.
    #[must_use]
    pub const fn applies_color_grading(&self) -> bool {
        !matches!(self, PhotoFilter::None)
    }

    /// Whether this filter desaturates the image.
    #[must_use]
    pub const fn is_desaturated(&self) -> bool {
        matches!(
            self,
            PhotoFilter::BlackAndWhite | PhotoFilter::Noir | PhotoFilter::BleachBypass
        )
    }
}

/// Time control settings for photo mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimeControl {
    /// Whether time is frozen.
    pub frozen: bool,
    /// Time scale when not frozen (0.0 to 2.0).
    pub time_scale: f32,
    /// Game time override (if any).
    pub time_override: Option<f32>,
}

impl TimeControl {
    /// Create default time control.
    #[must_use]
    pub fn new() -> Self {
        Self {
            frozen: false,
            time_scale: 1.0,
            time_override: None,
        }
    }

    /// Create frozen time control.
    #[must_use]
    pub fn frozen() -> Self {
        Self {
            frozen: true,
            time_scale: 0.0,
            time_override: None,
        }
    }

    /// Set frozen state.
    #[must_use]
    pub fn with_frozen(mut self, frozen: bool) -> Self {
        self.frozen = frozen;
        self
    }

    /// Set time scale.
    #[must_use]
    pub fn with_time_scale(mut self, scale: f32) -> Self {
        self.time_scale = scale.clamp(0.0, 2.0);
        self
    }

    /// Set time override.
    #[must_use]
    pub fn with_time_override(mut self, time: f32) -> Self {
        self.time_override = Some(time);
        self
    }

    /// Get effective time scale.
    #[must_use]
    pub fn effective_scale(&self) -> f32 {
        if self.frozen { 0.0 } else { self.time_scale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_settings() {
        let settings = PhotoSettings::default();
        assert_relative_eq!(settings.exposure, 0.0);
        assert_relative_eq!(settings.aperture, 5.6);
        assert_relative_eq!(settings.fov, 70.0);
        assert!(!settings.time_frozen);
        assert!(!settings.ui_visible);
        assert_eq!(settings.filter, PhotoFilter::None);
    }

    #[test]
    fn test_settings_validation() {
        let valid = PhotoSettings::default();
        assert!(valid.is_valid());

        let invalid = PhotoSettings {
            exposure: 10.0,
            ..Default::default()
        };
        assert!(!invalid.is_valid());

        let normalized = invalid.normalized();
        assert!(normalized.is_valid());
        assert_relative_eq!(normalized.exposure, 5.0);
    }

    #[test]
    fn test_exposure_multiplier() {
        let settings = PhotoSettings::default().with_exposure(0.0);
        assert_relative_eq!(settings.exposure_multiplier(), 1.0, epsilon = 0.001);

        let bright = PhotoSettings::default().with_exposure(2.0);
        assert_relative_eq!(bright.exposure_multiplier(), 4.0, epsilon = 0.001);

        let dark = PhotoSettings::default().with_exposure(-2.0);
        assert_relative_eq!(dark.exposure_multiplier(), 0.25, epsilon = 0.001);
    }

    #[test]
    fn test_blur_radius() {
        let wide_open = PhotoSettings::default().with_aperture(1.4);
        let narrow = PhotoSettings::default().with_aperture(22.0);

        assert!(wide_open.blur_radius() > narrow.blur_radius());
        assert!(narrow.blur_radius() >= 0.0);
    }

    #[test]
    fn test_filter_variants() {
        assert_eq!(PhotoFilter::ALL.len(), 12);
        for filter in PhotoFilter::ALL {
            assert!(!filter.name().is_empty());
        }
    }

    #[test]
    fn test_filter_desaturation() {
        assert!(PhotoFilter::BlackAndWhite.is_desaturated());
        assert!(PhotoFilter::Noir.is_desaturated());
        assert!(!PhotoFilter::Sepia.is_desaturated());
        assert!(!PhotoFilter::None.is_desaturated());
    }

    #[test]
    fn test_time_control() {
        let frozen = TimeControl::frozen();
        assert_relative_eq!(frozen.effective_scale(), 0.0);

        let slow = TimeControl::new().with_time_scale(0.5);
        assert_relative_eq!(slow.effective_scale(), 0.5);
    }

    #[test]
    fn test_settings_builder_chain() {
        let settings = PhotoSettings::new()
            .with_exposure(1.5)
            .with_aperture(2.8)
            .with_focus_distance(5.0)
            .with_fov(85.0)
            .with_roll(15.0)
            .with_time_frozen(true)
            .with_filter(PhotoFilter::Cinematic);

        assert_relative_eq!(settings.exposure, 1.5);
        assert_relative_eq!(settings.aperture, 2.8);
        assert_relative_eq!(settings.focus_distance, 5.0);
        assert_relative_eq!(settings.fov, 85.0);
        assert_relative_eq!(settings.roll, 15.0);
        assert!(settings.time_frozen);
        assert_eq!(settings.filter, PhotoFilter::Cinematic);
    }

    #[test]
    fn test_position_override() {
        let settings = PhotoSettings::new().with_position(Vec3::new(1.0, 2.0, 3.0));
        assert!(settings.position.is_some());

        let cleared = settings.without_position_override();
        assert!(cleared.position.is_none());
    }

    #[test]
    fn test_serde_roundtrip() {
        let settings = PhotoSettings::new()
            .with_exposure(2.0)
            .with_filter(PhotoFilter::Sepia);

        let bytes = bincode::serialize(&settings).expect("serialize");
        let restored: PhotoSettings = bincode::deserialize(&bytes).expect("deserialize");

        assert_relative_eq!(settings.exposure, restored.exposure);
        assert_eq!(settings.filter, restored.filter);
    }
}
